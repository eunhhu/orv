use std::collections::HashMap;

use http_body_util::{BodyExt, Limited};
use hyper::body::Incoming;
use hyper::{Request, StatusCode};

use crate::db::DbHandle;
use crate::interp::{
    run_handler_with_request_in_env_and_types_with_options, RequestCtx, RuntimeOptions, Value,
};

use super::{
    default_response, json_to_value, match_route, normalize_path, parse_query, plain_response,
    rate_limit_bucket_key, record_request_frame, request_trace_events_response,
    response_extra_headers, response_from_respond, value_to_json, LocalCapturedEnv, LocalRoutes,
    RateLimitState, RouteEntry, ServerRequestFrame, ServerResponse, TraceState, MAX_BODY_BYTES,
    ORV_TRACE_EVENTS_PATH,
};

#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::future_not_send)]
pub(super) async fn handle_request(
    req: Request<Incoming>,
    routes: LocalRoutes,
    captured_env: LocalCapturedEnv,
    db: DbHandle,
    client_ip: String,
    trace_state: Option<TraceState>,
    rate_limits: RateLimitState,
    runtime_options: RuntimeOptions,
) -> ServerResponse {
    let method = req.method().as_str().to_string();
    let uri = req.uri().clone();
    // hyper 는 요청 경로의 trailing `/` 를 그대로 보존한다. curl 사용자가 흔히
    // `/users/42/` 로 쳐도 `/users/:id` 매치 대상이 되도록 정규화한다. 루트
    // `/` 자체는 예외 — 빈 문자열이 되면 매칭 규칙이 무의미해진다.
    let path_raw = uri.path().to_string();
    let path = normalize_path(&path_raw);
    let query = uri.query().map(parse_query).unwrap_or_default();
    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let (body_value, raw_body) = match request_body_value(req, &headers).await {
        Ok(value) => value,
        Err(response) => return response,
    };

    if method == "GET" && path == ORV_TRACE_EVENTS_PATH {
        if let Some(trace_state) = trace_state.as_ref() {
            return request_trace_events_response(trace_state);
        }
    }

    // 라우트 매칭 — 선형 탐색. method 는 "*" wildcard 허용.
    let mut matched: Option<(RouteEntry, HashMap<String, String>)> = None;
    for entry in routes.iter() {
        if entry.method != "*" && entry.method != method {
            continue;
        }
        if let Some(params) = match_route(&entry.path, &path) {
            matched = Some((entry.clone(), params));
            break;
        }
    }

    let Some((entry, params)) = matched else {
        let response = plain_response(StatusCode::NOT_FOUND, "Not Found".into());
        record_request_frame(
            trace_state.as_ref(),
            ServerRequestFrame {
                method,
                path,
                route_method: None,
                route_path: None,
                route_origin_id: None,
                response_origin_id: None,
                status: response.status().as_u16(),
                params: HashMap::new(),
                query,
                body: request_body_display(&body_value),
            },
        );
        return response;
    };

    if let Some(policy) = &entry.rate_limit {
        let bucket = rate_limit_bucket_key(
            &entry.method,
            &entry.path,
            policy.key.as_deref(),
            &client_ip,
            &headers,
            &query,
            &body_value,
        );
        if !rate_limits.check(&bucket, policy.limit, policy.window) {
            let response = plain_response(
                StatusCode::TOO_MANY_REQUESTS,
                "Too Many Requests: route rate limit exceeded".into(),
            );
            record_request_frame(
                trace_state.as_ref(),
                ServerRequestFrame {
                    method,
                    path,
                    route_method: Some(entry.method),
                    route_path: Some(entry.path),
                    route_origin_id: Some(entry.origin_id),
                    response_origin_id: None,
                    status: response.status().as_u16(),
                    params,
                    query,
                    body: request_body_display(&body_value),
                },
            );
            return response;
        }
    }

    let frame_method = method.clone();
    let frame_path = path.clone();
    let frame_params = params.clone();
    let frame_query = query.clone();
    let frame_body = request_body_display(&body_value);
    let ctx = RequestCtx {
        method,
        path,
        ip: client_ip,
        params,
        query,
        query_value: None,
        headers,
        raw_body,
        body: body_value,
        form: None,
    };

    // handler 평가는 동기. stdout 은 버리는 버퍼로 흘려 — `@out` 은 서버
    // 콘솔이 아니라 요청 단위로 캡처해 반환 헤더에 싣는 편이 정석이지만
    // MVP 는 단순히 버린다.
    let mut sink = Vec::<u8>::new();
    let outcome = match run_handler_with_request_in_env_and_types_with_options(
        &entry.handler,
        ctx,
        captured_env.snapshot(),
        captured_env.type_registry(),
        db.clone(),
        &mut sink,
        runtime_options,
    ) {
        Ok(o) => o,
        Err(e) => {
            // 스택 트레이스나 내부 메시지 누출을 막기 위해 일반 메시지만.
            eprintln!("handler runtime error: {e}");
            return plain_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".into(),
            );
        }
    };

    // A3 하이브리드: server-level 바인딩 재할당 경고는 stderr 로 흘린다.
    // 프로덕션 로깅 레이어가 없는 MVP 이므로 단순 eprintln.
    for w in &outcome.warnings {
        eprintln!("{w}");
    }

    let (response, response_origin_id) = match outcome.response {
        Some(resp) => {
            let response_origin_id = resp.origin_id.clone();
            let extra_headers = response_extra_headers(&entry.method, &entry.path, &resp);
            (
                response_from_respond(resp, Some(&entry.origin_id), &extra_headers),
                response_origin_id,
            )
        }
        None => (
            default_response(&outcome.value, Some(&entry.origin_id)),
            None,
        ),
    };
    record_request_frame(
        trace_state.as_ref(),
        ServerRequestFrame {
            method: frame_method,
            path: frame_path,
            route_method: Some(entry.method),
            route_path: Some(entry.path),
            route_origin_id: Some(entry.origin_id),
            response_origin_id,
            status: response.status().as_u16(),
            params: frame_params,
            query: frame_query,
            body: frame_body,
        },
    );
    response
}

async fn request_body_value(
    req: Request<Incoming>,
    headers: &HashMap<String, String>,
) -> Result<(Value, String), ServerResponse> {
    // `Limited` 로 크기 상한을 걸어 거대 POST 의 메모리 폭주를 차단. 초과 시
    // 413 응답.
    let limited = Limited::new(req.into_body(), MAX_BODY_BYTES);
    let body_bytes = match limited.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            let msg = format!("{e}");
            if msg.contains("length limit exceeded") {
                return Err(plain_response(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    format!("request body exceeds {MAX_BODY_BYTES} bytes"),
                ));
            }
            return Err(plain_response(
                StatusCode::BAD_REQUEST,
                format!("failed to read request body: {msg}"),
            ));
        }
    };
    let content_type = headers
        .get("content-type")
        .map(|ct| ct.to_ascii_lowercase())
        .unwrap_or_default();
    let is_json = content_type.starts_with("application/json");
    let is_form_urlencoded = content_type.starts_with("application/x-www-form-urlencoded");
    let raw_body = String::from_utf8_lossy(&body_bytes).into_owned();
    if body_bytes.is_empty() {
        Ok((Value::Void, raw_body))
    } else if is_json {
        let body = serde_json::from_slice::<serde_json::Value>(&body_bytes)
            .map(json_to_value)
            .map_err(|e| {
                plain_response(StatusCode::BAD_REQUEST, format!("invalid JSON body: {e}"))
            })?;
        Ok((body, raw_body))
    } else if is_form_urlencoded {
        Ok((
            Value::Object(
                parse_query(&raw_body)
                    .into_iter()
                    .map(|(key, value)| (key, Value::Str(value)))
                    .collect(),
            ),
            raw_body,
        ))
    } else {
        Ok((Value::Str(raw_body.clone()), raw_body))
    }
}

fn request_body_display(value: &Value) -> String {
    if matches!(value, Value::Void) {
        return String::new();
    }
    serde_json::to_string(&value_to_json(value)).unwrap_or_default()
}
