use super::*;

/// `?a=1&b=hello` 형태 쿼리 문자열을 맵으로.
///
/// SPEC §11.3 은 쿼리 디코딩 규칙을 깊게 정의하지 않는다. 적용 순서:
/// 1. `+` → space (application/x-www-form-urlencoded 관습. value 에만 적용해
///    key 의 literal `+` 는 그대로 두는 게 안전하지만, 키에 `+` 가 등장할 일
///    자체가 드물어 양쪽 모두 치환한다).
/// 2. percent-decoding — RFC 3986 `%HH` 두 자리 hex. 잘못된 시퀀스(`%ZZ`,
///    `%2`) 는 raw 로 보존해 요청을 거부하지 않는다 (best-effort 파싱).
/// 3. UTF-8 검증 — 디코딩 결과가 UTF-8 이 아니면 raw 문자열로 폴백.
pub(crate) fn parse_query(raw: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut it = pair.splitn(2, '=');
        let k = percent_decode_form(it.next().unwrap_or(""));
        let v = percent_decode_form(it.next().unwrap_or(""));
        out.insert(k, v);
    }
    out
}

/// application/x-www-form-urlencoded 규칙으로 한 토큰을 디코딩한다.
///
/// `+` → space → `%HH` → UTF-8 조립. `%HH` 가 잘못되면 해당 `%` 는 literal
/// 로 남기고 다음 문자부터 계속 스캔한다. 결과 바이트가 UTF-8 이 아니면
/// 입력을 그대로 반환한다.
fn percent_decode_form(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = hex_value(bytes[i + 1]);
                let lo = hex_value(bytes[i + 2]);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

const fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// 요청 경로의 trailing `/` 를 제거한다 (단 `/` 자체는 그대로 유지).
///
/// hyper 는 경로를 원문 그대로 전달해 `/users/42` 와 `/users/42/` 가 다른
/// 값이 된다. 대부분의 사용자는 두 형태를 동치로 기대하므로 여기서 정규화해
/// 라우트 매처가 동일하게 처리하도록 돕는다.
pub(crate) fn normalize_path(path: &str) -> String {
    if path == "/" {
        return path.to_string();
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

/// 라우트 패턴(`/users/:id`) 과 실제 경로(`/users/42`) 를 segment 단위로 비교.
///
/// 매칭되면 `:param` 자리의 값을 맵으로 반환. 빈 segment(`//` 연속)는 분할
/// 그대로 보존한다.
///
/// 특수 패턴:
/// - `*` (catchall) — 패턴 전체가 단일 `"*"` 면 어떤 경로든 매치. SPEC §11.2
///   의 `@route GET * { @respond 404 ... }` 구문을 지원하기 위한 규칙.
///   params 는 비어 있다. 세그먼트 수준 wildcard(`/a/*`)는 이번 범위 밖.
pub(crate) fn match_route(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    if pattern == "*" {
        return Some(HashMap::new());
    }
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let actual_segments: Vec<&str> = path.split('/').collect();

    // A2b: named wildcard suffix `:NAME*` — 패턴 마지막 세그먼트가 이 형태면
    // 앞쪽은 정확 매치, 그 이후의 모든 세그먼트는 `/` 로 join 해 `NAME` 에
    // 캡처. rest 는 최소 1개 세그먼트를 요구 (0 segments 는 일반 prefix 매치와
    // 모호해지므로 거부).
    if let Some(last) = pattern_segments.last() {
        if let Some(name) = last.strip_prefix(':').and_then(|n| n.strip_suffix('*')) {
            // 앞쪽 세그먼트 수가 path 의 세그먼트 수보다 작아야 rest 가
            // 최소 1개 존재한다. `:rest*` 는 필수 캡처이므로 같거나 적으면 실패.
            let prefix_len = pattern_segments.len() - 1;
            if actual_segments.len() <= prefix_len {
                return None;
            }
            let mut params = HashMap::new();
            for (pp, ap) in pattern_segments
                .iter()
                .take(prefix_len)
                .zip(actual_segments.iter())
            {
                if !match_route_segment(pp, ap, &mut params) {
                    return None;
                }
            }
            let rest = actual_segments[prefix_len..].join("/");
            params.insert(name.to_string(), rest);
            return Some(params);
        }
    }

    if pattern_segments.len() != actual_segments.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (pp, ap) in pattern_segments.iter().zip(actual_segments.iter()) {
        if !match_route_segment(pp, ap, &mut params) {
            return None;
        }
    }
    Some(params)
}

fn match_route_segment(
    pattern_segment: &str,
    path_segment: &str,
    params: &mut HashMap<String, String>,
) -> bool {
    let Some((name, suffix)) = route_param_segment(pattern_segment) else {
        return pattern_segment == path_segment;
    };
    let Some(value) = path_segment.strip_suffix(suffix) else {
        return false;
    };
    params.insert(name.to_string(), value.to_string());
    true
}

fn route_param_segment(segment: &str) -> Option<(&str, &str)> {
    let body = segment.strip_prefix(':')?;
    let end = body
        .char_indices()
        .find_map(|(index, ch)| (!route_param_name_char(ch)).then_some(index))
        .unwrap_or(body.len());
    (end > 0).then_some((&body[..end], &body[end..]))
}

const fn route_param_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// orv [`Value`] → `serde_json::Value`.
///
/// 변환 규칙 (MVP):
/// - Int/Float/Bool/Str → scalar JSON.
/// - Void → `null` (SPEC §11.4 가 Void payload 를 "빈 body" 로 규정하지만
///   직렬화 경로에 들어올 일이 없도록 상위에서 분기. 안전망으로 null.).
/// - Array → JSON array (재귀).
/// - Object → JSON object (필드 순서 보존은 `serde_json::Map` 이 기본 `BTreeMap`
///   이 아니라 `preserve_order` feature 가 꺼져 있으면 알파벳 순이 될 수
///   있다. 테스트가 순서에 의존하지 않도록 값만 비교).
/// - Function/Lambda/BoundMethod → 문자열로 표시 (SPEC 은 직렬화 불가를
///   규정하지만 panic 대신 문자열로 떨어뜨려 진단이 쉽다).
pub(crate) fn value_to_json(v: &Value) -> serde_json::Value {
    use serde_json::Value as J;
    match v {
        Value::Int(n) => J::from(*n),
        Value::Float(f) => serde_json::Number::from_f64(*f).map_or(J::Null, J::Number),
        Value::Bool(b) => J::Bool(*b),
        Value::Str(s) => J::String(s.clone()),
        Value::Regex { pattern, flags } => J::String(format!("r\"{pattern}\"{flags}")),
        Value::Void => J::Null,
        Value::Array(items) => J::Array(items.iter().map(value_to_json).collect()),
        Value::Tuple(elems) => J::Array(elems.iter().map(value_to_json).collect()),
        Value::Object(fields) => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), value_to_json(v));
            }
            J::Object(map)
        }
        Value::Function(f) => J::String(format!("<function {}>", f.name.name)),
        Value::Lambda(_) => J::String("<lambda>".into()),
        Value::BoundMethod { method, .. } => J::String(format!("<method {method}>")),
        Value::Db(_) => J::String("<db>".into()),
        Value::TypeName(n) => J::String(format!("<type {n}>")),
        Value::Builtin(n) => J::String(format!("<builtin {n}>")),
    }
}

/// `serde_json::Value` → orv [`Value`]. 요청 body JSON 파싱 경로에서만 사용.
///
/// 숫자 매핑 규칙:
/// - `i64` 범위면 `Value::Int`.
/// - `f64` 로 표현 가능한 부동소수점이면 `Value::Float`.
/// - `u64::MAX` 쪽으로 i64 상한을 넘는 큰 정수는 **precision 손실을 피하려고
///   원문 문자열을 `Value::Str`** 로 보존한다. 사용자가 명시적으로 처리하도록
///   미는 선택 — 조용히 f64 로 몰아서 `9999999999999999999` → `1e19` 가 되는
///   경우를 막는다.
pub(super) fn json_to_value(j: serde_json::Value) -> Value {
    use serde_json::Value as J;
    match j {
        J::Null => Value::Void,
        J::Bool(b) => Value::Bool(b),
        J::Number(n) => n.as_i64().map_or_else(
            || {
                if n.is_f64() {
                    // 명시적으로 소수점이 있는 표기면 float 로 받는다.
                    n.as_f64().map_or(Value::Void, Value::Float)
                } else {
                    // i64 를 넘는 정수(u64 상단)는 원문을 보존.
                    Value::Str(n.to_string())
                }
            },
            Value::Int,
        ),
        J::String(s) => Value::Str(s),
        J::Array(items) => Value::Array(items.into_iter().map(json_to_value).collect()),
        J::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, json_to_value(v)))
                .collect(),
        ),
    }
}
