//! `@server` HTTP 런타임 (C5b, MVP).
//!
//! tokio 의 `current_thread` 런타임 위에서 hyper 1.x HTTP/1.1 서버를 기동한다.
//! 요청마다 매칭된 route 의 handler HIR 을 **복제**하고 새 [`crate::interp::Interp`]
//! 를 만들어 [`crate::interp::run_handler_with_request`] 로 평가한다. 이 구조의
//! 이점:
//!
//! - 인터프리터 자체는 여전히 순수 동기 — async 는 이 파일 안에만 갇힌다.
//! - 요청 간 상태 누수 없음. 각 요청이 새 env, 새 writer(버퍼), 새 response 슬롯
//!   을 갖는다.
//! - 기존 interp 구조 변경 최소. Server arm 이 이 모듈의 [`run_server`] 를
//!   부르기만 한다.
//!
//! MVP 범위 / 비범위
//! - HTTP/1.1 단일. SPEC §11 의 QUIC/HTTP3 기본값은 이후 마일스톤.
//! - JSON 직렬화는 [`value_to_json`] — object/array/스칼라/void 만.
//! - 경로 매처는 [`match_route`] — 선형 탐색, `:param` 추출, `*` wildcard segment
//!   미지원 (C5 범위 밖, §11.7 중첩 라우트와 함께 후속).

use std::collections::HashMap;
use std::time::Duration;

use orv_hir::{HirExpr, HirStmt, NameId};

use crate::db::DbHandle;
use crate::interp::{RuntimeError, RuntimeOptions, RuntimeTypeRegistry, Value};

/// MVP request body size limit (1MB). 초과 시 413 Payload Too Large.
///
/// hyper 자체는 body 크기 상한이 없어, 악의적 거대 POST 한 번에 메모리를 전부
/// 할당해 버리는 `DoS` 벡터가 된다. `http_body_util::Limited` 로 래핑해 수집
/// 단계에서 방지한다. 1MB 는 작은 JSON 페이로드/폼 입력을 통과시키면서
/// 멀티파트 파일 업로드는 막는 선. 파일 업로드는 SPEC §11 의 별도 경로로
/// 다룬다.
const MAX_BODY_BYTES: usize = 1024 * 1024;
const ORV_ORIGIN_ID_HEADER: &str = "x-orv-origin-id";
const ORV_RESPONSE_ORIGIN_ID_HEADER: &str = "x-orv-response-origin-id";
const ORV_RUNTIME_REQUEST_TRACE_PATH_ENV: &str = "ORV_RUNTIME_REQUEST_TRACE_PATH";
const ORV_TRACE_EVENTS_PATH: &str = "/__orv/trace/events";
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

mod body;
mod rate_limit;
mod request;
mod response;
mod routing;
mod runtime;
mod state;

use body::{runtime_request_trace_path_from_env, RuntimeBody, ServerResponse};
use rate_limit::{rate_limit_bucket_key, route_rate_limit_policy, RateLimitPolicy, RateLimitState};
use request::handle_request;
#[cfg(test)]
use response::login_session_cookie;
use response::{default_response, plain_response, response_extra_headers, response_from_respond};
use routing::{json_to_value, match_route, normalize_path, parse_query, value_to_json};
use runtime::{record_request_frame, request_trace_events_response, TraceState};
pub use runtime::{
    request_trace_json, spawn_attached_server, write_request_trace_file, AttachedServer,
    ServerRequestFrame,
};
use state::{CapturedRuntimeState, LocalCapturedEnv, LocalRoutes, RouteEntry};

pub(crate) fn run_server_with_options(
    listen: Option<&HirExpr>,
    routes: &[HirExpr],
    body_stmts: &[HirStmt],
    captured_env: HashMap<NameId, Value>,
    captured_types: RuntimeTypeRegistry,
    db: DbHandle,
    runtime_options: RuntimeOptions,
) -> Result<Value, RuntimeError> {
    runtime::run_server_with_options(
        listen,
        routes,
        body_stmts,
        captured_env,
        captured_types,
        db,
        runtime_options,
    )
}

#[cfg(test)]
mod tests;
