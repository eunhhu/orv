use bytes::Bytes;
use hyper::{Response, StatusCode};

use crate::interp::{
    ResponseCtx, Value, ORV_CSRF_COOKIE_NAME, ORV_REFERENCE_CSRF_TOKEN, ORV_SESSION_COOKIE_NAME,
    ORV_SESSION_ROLE_COOKIE_NAME,
};

use super::{
    value_to_json, RuntimeBody, ServerResponse, ORV_ORIGIN_ID_HEADER, ORV_RESPONSE_ORIGIN_ID_HEADER,
};

pub(super) fn response_from_respond(
    resp: ResponseCtx,
    origin_id: Option<&str>,
    extra_headers: &[(String, String)],
) -> ServerResponse {
    let response_origin_id = resp.origin_id.clone();
    let status = u16::try_from(resp.status)
        .ok()
        .and_then(|s| StatusCode::from_u16(s).ok())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // SPEC §11.9: `@redirect` 가 기록한 Location 이 있으면 body 없이
    // `Location:` 헤더 + 상태로 응답한다. payload/raw_body 는 무시.
    if let Some(loc) = resp.location {
        let builder = response_builder(status, origin_id, response_origin_id.as_deref())
            .header("location", loc);
        return apply_extra_response_headers(builder, extra_headers)
            .body(RuntimeBody::full(Bytes::new()))
            .expect("valid response");
    }

    // A5a: `@serve` 가 기록한 raw body 는 JSON 경로를 우회하고 그대로 나간다.
    // body 금지 상태(204/304/1xx)에서도 파일은 있을 수 없는 조합이라 일반
    // 경로보다 먼저 잡는다.
    if let Some(raw) = resp.raw_body {
        let builder = response_builder(status, origin_id, response_origin_id.as_deref())
            .header("content-type", raw.content_type);
        return apply_extra_response_headers(builder, extra_headers)
            .body(RuntimeBody::full(Bytes::from(raw.bytes)))
            .expect("valid response");
    }

    // RFC 상 body 가 허용되지 않는 상태(204/304/1xx)와 Void payload 는 항상
    // 빈 body 로 보낸다. SPEC 도 `@respond 204 {}` 에서 body 인코더 제거를
    // 기대하므로, payload 값과 무관하게 no-body 경로를 우선한다.
    if status_disallows_body(status) || matches!(resp.payload, Value::Void) {
        return apply_extra_response_headers(
            response_builder(status, origin_id, response_origin_id.as_deref()),
            extra_headers,
        )
        .body(RuntimeBody::full(Bytes::new()))
        .expect("valid response");
    }
    let json = value_to_json(&resp.payload);
    let body = serde_json::to_vec(&json).unwrap_or_else(|_| b"null".to_vec());
    let builder = response_builder(status, origin_id, response_origin_id.as_deref())
        .header("content-type", "application/json");
    apply_extra_response_headers(builder, extra_headers)
        .body(RuntimeBody::full(Bytes::from(body)))
        .expect("valid response")
}

pub(super) fn response_extra_headers(
    method: &str,
    path: &str,
    resp: &ResponseCtx,
) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    if let Some(cookie) = login_session_cookie(method, path, resp) {
        headers.push(("set-cookie".to_string(), cookie));
    }
    if let Some(cookie) = login_session_role_cookie(method, path, resp) {
        headers.push(("set-cookie".to_string(), cookie));
    }
    if let Some(cookie) = csrf_cookie(method, resp) {
        headers.push(("set-cookie".to_string(), cookie));
    }
    headers
}

pub(super) fn login_session_cookie(method: &str, path: &str, resp: &ResponseCtx) -> Option<String> {
    if method != "POST" || path != "/members/login" || !(200..300).contains(&resp.status) {
        return None;
    }
    let session = object_field_value(&resp.payload, "session")?;
    let session_id = object_field_value(session, "id")?;
    let cookie_value = cookie_scalar_value(session_id)?;
    Some(format!(
        "{ORV_SESSION_COOKIE_NAME}={cookie_value}; Path=/; Max-Age=86400; HttpOnly; SameSite=Lax; Secure"
    ))
}

fn login_session_role_cookie(method: &str, path: &str, resp: &ResponseCtx) -> Option<String> {
    if method != "POST" || path != "/members/login" || !(200..300).contains(&resp.status) {
        return None;
    }
    let session = object_field_value(&resp.payload, "session")?;
    let role = object_field_value(session, "role")?;
    let cookie_value = cookie_scalar_value(role)?;
    Some(format!(
        "{ORV_SESSION_ROLE_COOKIE_NAME}={cookie_value}; Path=/; Max-Age=86400; HttpOnly; SameSite=Lax; Secure"
    ))
}

fn cookie_scalar_value(value: &Value) -> Option<String> {
    let value = match value {
        Value::Int(id) => id.to_string(),
        Value::Str(id) if !id.is_empty() => id.clone(),
        _ => return None,
    };
    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~'))
        .then_some(value)
}

fn csrf_cookie(method: &str, resp: &ResponseCtx) -> Option<String> {
    if method != "GET" || !(200..300).contains(&resp.status) {
        return None;
    }
    let raw = resp.raw_body.as_ref()?;
    if !raw.content_type.starts_with("text/html") {
        return None;
    }
    Some(format!(
        "{ORV_CSRF_COOKIE_NAME}={ORV_REFERENCE_CSRF_TOKEN}; Path=/; Max-Age=86400; SameSite=Lax"
    ))
}

fn object_field_value<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    let Value::Object(fields) = value else {
        return None;
    };
    fields
        .iter()
        .rev()
        .find_map(|(field, value)| (field == name).then_some(value))
}

fn apply_extra_response_headers(
    mut builder: hyper::http::response::Builder,
    headers: &[(String, String)],
) -> hyper::http::response::Builder {
    for (name, value) in headers {
        builder = builder.header(name.as_str(), value.as_str());
    }
    builder
}

fn status_disallows_body(status: StatusCode) -> bool {
    status.is_informational()
        || status == StatusCode::NO_CONTENT
        || status == StatusCode::NOT_MODIFIED
}

pub(super) fn default_response(value: &Value, origin_id: Option<&str>) -> ServerResponse {
    // handler 가 `@respond` 없이 값으로 끝나면 그 값을 JSON 으로 200 응답.
    // Void 는 빈 200. 이렇게 하면 `@route GET /health { "ok" }` 같은 간단한
    // 핸들러가 그대로 동작한다.
    if matches!(value, Value::Void) {
        return response_builder(StatusCode::OK, origin_id, None)
            .body(RuntimeBody::full(Bytes::new()))
            .expect("valid response");
    }
    let json = value_to_json(value);
    let body = serde_json::to_vec(&json).unwrap_or_else(|_| b"null".to_vec());
    response_builder(StatusCode::OK, origin_id, None)
        .header("content-type", "application/json")
        .body(RuntimeBody::full(Bytes::from(body)))
        .expect("valid response")
}

fn response_builder(
    status: StatusCode,
    origin_id: Option<&str>,
    response_origin_id: Option<&str>,
) -> hyper::http::response::Builder {
    let mut builder = Response::builder().status(status);
    if let Some(origin_id) = origin_id {
        builder = builder.header(ORV_ORIGIN_ID_HEADER, origin_id);
    }
    if let Some(response_origin_id) = response_origin_id {
        builder = builder.header(ORV_RESPONSE_ORIGIN_ID_HEADER, response_origin_id);
    }
    builder
}

pub(super) fn plain_response(status: StatusCode, body: String) -> ServerResponse {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(RuntimeBody::full(Bytes::from(body)))
        .expect("valid response")
}
