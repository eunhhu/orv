use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use orv_hir::{HirBlock, HirExpr, HirExprKind, HirStmt, HirStringSegment};

use crate::interp::RuntimeError;

use super::RATE_LIMIT_WINDOW;

mod bucket;
mod static_value;

pub(super) use bucket::rate_limit_bucket_key;
use static_value::{
    static_bool, static_positive_usize, static_rate_limit_key, static_rate_limit_window,
};

#[derive(Clone, Default)]
pub(super) struct RateLimitState {
    buckets: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl RateLimitState {
    pub(super) fn check(&self, key: &str, limit: usize, window: Duration) -> bool {
        let now = Instant::now();
        let cutoff = now.checked_sub(window).unwrap_or(now);
        let Ok(mut buckets) = self.buckets.lock() else {
            return true;
        };
        let bucket = buckets.entry(key.to_string()).or_default();
        bucket.retain(|instant| *instant >= cutoff);
        if bucket.len() >= limit {
            return false;
        }
        bucket.push(now);
        true
    }
}

#[derive(Clone)]
pub(super) struct RateLimitPolicy {
    pub(super) limit: usize,
    pub(super) window: Duration,
    pub(super) key: Option<String>,
}

enum RouteRateLimitPolicy {
    Apply(RateLimitPolicy),
    Exempt,
}

fn default_rate_limit_policy(method: &str, path: &str) -> Option<RateLimitPolicy> {
    match (method, path) {
        ("POST", "/members/login" | "/checkout") => Some(RateLimitPolicy {
            limit: 10,
            window: RATE_LIMIT_WINDOW,
            key: None,
        }),
        ("POST", "/webhooks/stripe") => Some(RateLimitPolicy {
            limit: 60,
            window: RATE_LIMIT_WINDOW,
            key: None,
        }),
        _ => None,
    }
}

pub(super) fn route_rate_limit_policy(
    method: &str,
    path: &str,
    handler: &HirBlock,
) -> Result<Option<RateLimitPolicy>, RuntimeError> {
    match find_route_rate_limit_policy(handler)? {
        Some(RouteRateLimitPolicy::Apply(policy)) => Ok(Some(policy)),
        Some(RouteRateLimitPolicy::Exempt) => Ok(None),
        None => Ok(default_rate_limit_policy(method, path)),
    }
}

fn find_route_rate_limit_policy(
    block: &HirBlock,
) -> Result<Option<RouteRateLimitPolicy>, RuntimeError> {
    for stmt in &block.stmts {
        if let Some(policy) = find_stmt_rate_limit_policy(stmt)? {
            return Ok(Some(policy));
        }
    }
    Ok(None)
}

fn find_stmt_rate_limit_policy(
    stmt: &HirStmt,
) -> Result<Option<RouteRateLimitPolicy>, RuntimeError> {
    match stmt {
        HirStmt::Let(stmt) => find_expr_rate_limit_policy(&stmt.init),
        HirStmt::Const(stmt) => find_expr_rate_limit_policy(&stmt.init),
        HirStmt::Function(stmt) => match &stmt.body {
            orv_hir::HirFunctionBody::Block(block) => find_route_rate_limit_policy(block),
            orv_hir::HirFunctionBody::Expr(expr) => find_expr_rate_limit_policy(expr),
        },
        HirStmt::Return(stmt) => stmt
            .value
            .as_ref()
            .map_or(Ok(None), find_expr_rate_limit_policy),
        HirStmt::Expr(expr) => find_expr_rate_limit_policy(expr),
        HirStmt::Struct(_) | HirStmt::Enum(_) | HirStmt::TypeAlias(_) | HirStmt::Import(_) => {
            Ok(None)
        }
    }
}

#[allow(clippy::too_many_lines)]
fn find_expr_rate_limit_policy(
    expr: &HirExpr,
) -> Result<Option<RouteRateLimitPolicy>, RuntimeError> {
    match &expr.kind {
        HirExprKind::Domain { name, args, .. } if name == "rateLimit" => {
            parse_rate_limit_policy(args).map(Some)
        }
        HirExprKind::Domain { args, .. } => {
            for arg in args {
                if let Some(policy) = find_expr_rate_limit_policy(arg)? {
                    return Ok(Some(policy));
                }
            }
            Ok(None)
        }
        HirExprKind::Block(block) | HirExprKind::Html(block) => find_route_rate_limit_policy(block),
        HirExprKind::If {
            cond,
            then,
            else_branch,
        } => find_expr_rate_limit_policy(cond)?.map_or_else(
            || {
                find_route_rate_limit_policy(then)?.map_or_else(
                    || {
                        else_branch
                            .as_ref()
                            .map_or(Ok(None), |expr| find_expr_rate_limit_policy(expr))
                    },
                    |policy| Ok(Some(policy)),
                )
            },
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::When { scrutinee, arms } => {
            if let Some(policy) = find_expr_rate_limit_policy(scrutinee)? {
                return Ok(Some(policy));
            }
            for arm in arms {
                if let Some(policy) = find_expr_rate_limit_policy(&arm.body)? {
                    return Ok(Some(policy));
                }
            }
            Ok(None)
        }
        HirExprKind::For { iter, body, .. } => find_expr_rate_limit_policy(iter)?.map_or_else(
            || find_route_rate_limit_policy(body),
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::While { cond, body } => find_expr_rate_limit_policy(cond)?.map_or_else(
            || find_route_rate_limit_policy(body),
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::Try { try_block, catch } => find_route_rate_limit_policy(try_block)?
            .map_or_else(
                || {
                    catch
                        .as_ref()
                        .map_or(Ok(None), |catch| find_route_rate_limit_policy(&catch.body))
                },
                |policy| Ok(Some(policy)),
            ),
        HirExprKind::Paren(expr)
        | HirExprKind::Out(expr)
        | HirExprKind::Throw(expr)
        | HirExprKind::Await(expr)
        | HirExprKind::Cast { expr, .. }
        | HirExprKind::Unary { expr, .. } => find_expr_rate_limit_policy(expr),
        HirExprKind::Binary { lhs, rhs, .. } => find_expr_rate_limit_policy(lhs)?.map_or_else(
            || find_expr_rate_limit_policy(rhs),
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::Assign { value, .. } | HirExprKind::AssignField { value, .. } => {
            find_expr_rate_limit_policy(value)
        }
        HirExprKind::AssignIndex {
            object,
            index,
            value,
        } => find_expr_rate_limit_policy(object)?.map_or_else(
            || {
                find_expr_rate_limit_policy(index)?.map_or_else(
                    || find_expr_rate_limit_policy(value),
                    |policy| Ok(Some(policy)),
                )
            },
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::Call { callee, args } => find_expr_rate_limit_policy(callee)?.map_or_else(
            || {
                for arg in args {
                    if let Some(policy) = find_expr_rate_limit_policy(arg)? {
                        return Ok(Some(policy));
                    }
                }
                Ok(None)
            },
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::Array(items) | HirExprKind::Tuple(items) => {
            for item in items {
                if let Some(policy) = find_expr_rate_limit_policy(item)? {
                    return Ok(Some(policy));
                }
            }
            Ok(None)
        }
        HirExprKind::Object(fields) | HirExprKind::TypedObject { fields, .. } => {
            for field in fields {
                if let Some(policy) = find_expr_rate_limit_policy(&field.value)? {
                    return Ok(Some(policy));
                }
            }
            Ok(None)
        }
        HirExprKind::Index { target, index } => find_expr_rate_limit_policy(target)?.map_or_else(
            || find_expr_rate_limit_policy(index),
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::Slice { target, start, end } => {
            if let Some(policy) = find_expr_rate_limit_policy(target)? {
                return Ok(Some(policy));
            }
            if let Some(start) = start {
                if let Some(policy) = find_expr_rate_limit_policy(start)? {
                    return Ok(Some(policy));
                }
            }
            end.as_ref()
                .map_or(Ok(None), |end| find_expr_rate_limit_policy(end))
        }
        HirExprKind::Field { target, .. } | HirExprKind::OptionalField { target, .. } => {
            find_expr_rate_limit_policy(target)
        }
        HirExprKind::Lambda { body, .. } => match body.as_ref() {
            orv_hir::HirFunctionBody::Block(block) => find_route_rate_limit_policy(block),
            orv_hir::HirFunctionBody::Expr(expr) => find_expr_rate_limit_policy(expr),
        },
        HirExprKind::Range { start, end, .. } => find_expr_rate_limit_policy(start)?.map_or_else(
            || find_expr_rate_limit_policy(end),
            |policy| Ok(Some(policy)),
        ),
        HirExprKind::String(segments) => {
            for segment in segments {
                if let HirStringSegment::Interp(expr) = segment {
                    if let Some(policy) = find_expr_rate_limit_policy(expr)? {
                        return Ok(Some(policy));
                    }
                }
            }
            Ok(None)
        }
        HirExprKind::Server { routes, .. } => {
            for route in routes {
                if let Some(policy) = find_expr_rate_limit_policy(route)? {
                    return Ok(Some(policy));
                }
            }
            Ok(None)
        }
        HirExprKind::Route { handler, .. } => find_route_rate_limit_policy(handler),
        HirExprKind::Integer(_)
        | HirExprKind::Float(_)
        | HirExprKind::Regex { .. }
        | HirExprKind::True
        | HirExprKind::False
        | HirExprKind::Void
        | HirExprKind::TypeName(_)
        | HirExprKind::Ident(_)
        | HirExprKind::Respond { .. }
        | HirExprKind::Break
        | HirExprKind::Continue => Ok(None),
    }
}

fn parse_rate_limit_policy(args: &[HirExpr]) -> Result<RouteRateLimitPolicy, RuntimeError> {
    let mut key = None;
    let mut limit = None;
    let mut window = None;
    let mut exempt = false;
    for arg in args {
        match &arg.kind {
            HirExprKind::Ident(ident) if ident.name == "exempt" => exempt = true,
            HirExprKind::Assign { target, value } if target.name == "exempt" => {
                exempt = static_bool(value).ok_or_else(|| {
                    RuntimeError::native("`@rateLimit exempt` expects a static bool")
                })?;
            }
            HirExprKind::Assign { target, value }
                if matches!(target.name.as_str(), "limit" | "max") =>
            {
                limit = Some(static_positive_usize(value).ok_or_else(|| {
                    RuntimeError::native("`@rateLimit limit` expects a positive static integer")
                })?);
            }
            HirExprKind::Assign { target, value } if target.name == "window" => {
                window = Some(static_rate_limit_window(value).ok_or_else(|| {
                    RuntimeError::native(
                        "`@rateLimit window` expects positive seconds or a duration string",
                    )
                })?);
            }
            HirExprKind::Assign { target, value } if target.name == "key" => {
                key = Some(static_rate_limit_key(value).ok_or_else(|| {
                    RuntimeError::native("`@rateLimit key` expects a static request key expression")
                })?);
            }
            _ => {
                return Err(RuntimeError::native(
                    "`@rateLimit` expects `limit=<n> window=<seconds|duration>`, optional `key=...`, or `exempt`",
                ));
            }
        }
    }
    if exempt {
        return Ok(RouteRateLimitPolicy::Exempt);
    }
    Ok(RouteRateLimitPolicy::Apply(RateLimitPolicy {
        limit: limit.ok_or_else(|| RuntimeError::native("`@rateLimit` missing `limit`"))?,
        window: window.ok_or_else(|| RuntimeError::native("`@rateLimit` missing `window`"))?,
        key,
    }))
}
