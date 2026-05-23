use std::collections::HashMap;

use crate::interp::{Value, ORV_SESSION_COOKIE_NAME, ORV_SESSION_ROLE_COOKIE_NAME};

pub(in crate::server) fn rate_limit_bucket_key(
    method: &str,
    path: &str,
    policy_key: Option<&str>,
    client_ip: &str,
    headers: &HashMap<String, String>,
    query: &HashMap<String, String>,
    body: &Value,
) -> String {
    let discriminator = policy_key
        .and_then(|key| rate_limit_key_value(key, headers, query, body))
        .unwrap_or_else(|| format!("ip:{client_ip}"));
    format!("{method}:{path}:{discriminator}")
}

fn cookie_value_from_headers(
    headers: &HashMap<String, String>,
    cookie_name: &str,
) -> Option<String> {
    let cookie_header = header_value_case_insensitive(headers, "cookie")?;
    cookie_header.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        let value = value.trim();
        (name.trim() == cookie_name && !value.is_empty()).then(|| value.to_string())
    })
}

fn header_value_case_insensitive(headers: &HashMap<String, String>, name: &str) -> Option<String> {
    headers.iter().find_map(|(header, value)| {
        (header.eq_ignore_ascii_case(name) && !value.is_empty()).then(|| value.clone())
    })
}

fn value_object_field_string(value: &Value, name: &str) -> Option<String> {
    let Value::Object(fields) = value else {
        return None;
    };
    fields.iter().find_map(|(field, value)| {
        if field != name {
            return None;
        }
        match value {
            Value::Str(value) if !value.is_empty() => Some(value.clone()),
            Value::Int(value) => Some(value.to_string()),
            Value::Float(value) if value.is_finite() => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        }
    })
}

fn rate_limit_key_value(
    key: &str,
    headers: &HashMap<String, String>,
    query: &HashMap<String, String>,
    body: &Value,
) -> Option<String> {
    match key {
        "@session.id" | "@session.userId" => {
            cookie_value_from_headers(headers, ORV_SESSION_COOKIE_NAME)
                .map(|value| format!("session:{value}"))
        }
        "@session.role" => cookie_value_from_headers(headers, ORV_SESSION_ROLE_COOKIE_NAME)
            .map(|value| format!("session-role:{value}")),
        _ => key
            .strip_prefix("@body.")
            .and_then(|field| value_object_field_string(body, field))
            .map(|value| format!("body:{value}"))
            .or_else(|| {
                key.strip_prefix("@query.")
                    .and_then(|field| query.get(field).filter(|value| !value.is_empty()).cloned())
                    .map(|value| format!("query:{value}"))
            })
            .or_else(|| {
                key.strip_prefix("@header.")
                    .and_then(|field| header_value_case_insensitive(headers, field))
                    .map(|value| format!("header:{value}"))
            })
            .or_else(|| (!key.is_empty()).then(|| format!("static:{key}"))),
    }
}
