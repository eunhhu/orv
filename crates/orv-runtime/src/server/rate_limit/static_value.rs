use super::*;

pub(super) fn static_positive_usize(expr: &HirExpr) -> Option<usize> {
    static_integer(expr).and_then(|value| usize::try_from(value).ok().filter(|value| *value > 0))
}

fn static_integer(expr: &HirExpr) -> Option<i64> {
    match &expr.kind {
        HirExprKind::Integer(value) => value.replace('_', "").parse::<i64>().ok(),
        HirExprKind::Unary {
            op: UnaryOp::Neg,
            expr,
        } => static_integer(expr).map(|value| -value),
        HirExprKind::Paren(expr) => static_integer(expr),
        _ => None,
    }
}

pub(super) fn static_bool(expr: &HirExpr) -> Option<bool> {
    match &expr.kind {
        HirExprKind::True => Some(true),
        HirExprKind::False => Some(false),
        HirExprKind::Paren(expr) => static_bool(expr),
        _ => None,
    }
}

fn static_string(expr: &HirExpr) -> Option<String> {
    match &expr.kind {
        HirExprKind::String(segments) => {
            let mut out = String::new();
            for segment in segments {
                match segment {
                    HirStringSegment::Str(value) => out.push_str(value),
                    HirStringSegment::Interp(_) => return None,
                }
            }
            Some(out)
        }
        HirExprKind::Paren(expr) => static_string(expr),
        _ => None,
    }
}

pub(super) fn static_rate_limit_window(expr: &HirExpr) -> Option<Duration> {
    static_positive_usize(expr)
        .map(|seconds| Duration::from_secs(seconds as u64))
        .or_else(|| static_string(expr).and_then(|value| parse_duration_literal(&value)))
}

fn parse_duration_literal(value: &str) -> Option<Duration> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let digit_len = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_')
        .map(char::len_utf8)
        .sum::<usize>();
    let (amount, unit) = trimmed.split_at(digit_len);
    let amount = amount.replace('_', "").parse::<u64>().ok()?;
    if amount == 0 {
        return None;
    }
    let multiplier = match unit.trim() {
        "" | "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 60 * 60,
        _ => return None,
    };
    amount.checked_mul(multiplier).map(Duration::from_secs)
}

pub(super) fn static_rate_limit_key(expr: &HirExpr) -> Option<String> {
    match &expr.kind {
        HirExprKind::String(_) => static_string(expr),
        HirExprKind::Ident(ident) => Some(ident.name.clone()),
        HirExprKind::Domain { name, args, .. } if args.is_empty() => Some(format!("@{name}")),
        HirExprKind::Field { target, field, .. } => {
            static_rate_limit_key(target).map(|target| format!("{target}.{field}"))
        }
        HirExprKind::OptionalField { target, field, .. } => {
            static_rate_limit_key(target).map(|target| format!("{target}?.{field}"))
        }
        HirExprKind::Paren(expr) => static_rate_limit_key(expr),
        _ => None,
    }
}
