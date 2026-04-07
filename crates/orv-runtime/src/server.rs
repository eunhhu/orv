//! Minimal HTTP server for `orv run` with path parameter matching.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

/// A parsed HTTP request from the TCP stream.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub path_params: HashMap<String, String>,
    pub body: String,
}

/// An HTTP response to write back.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl HttpResponse {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "application/json".to_owned(),
            body: body.to_owned(),
            headers: HashMap::new(),
        }
    }

    pub fn html(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "text/html; charset=utf-8".to_owned(),
            body: body.to_owned(),
            headers: HashMap::new(),
        }
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_owned(),
            body: body.to_owned(),
            headers: HashMap::new(),
        }
    }

    pub fn not_found() -> Self {
        Self::json(404, r#"{"error":"not found"}"#)
    }

    pub fn internal_error(msg: &str) -> Self {
        eprintln!("orv server error: {msg}");
        Self::json(500, r#"{"error":"internal server error"}"#)
    }

    fn status_text(&self) -> &str {
        match self.status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            422 => "Unprocessable Entity",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "OK",
        }
    }

    fn to_http_bytes(&self) -> Vec<u8> {
        let mut response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n",
            self.status,
            self.status_text(),
            self.content_type,
            self.body.len()
        );
        // Security headers
        response.push_str("X-Content-Type-Options: nosniff\r\n");
        response.push_str("X-Frame-Options: DENY\r\n");
        // Sanitize custom headers to prevent CRLF injection
        for (key, value) in &self.headers {
            let safe_key = key.replace(['\r', '\n'], "");
            let safe_value = value.replace(['\r', '\n'], "");
            response.push_str(&format!("{safe_key}: {safe_value}\r\n"));
        }
        response.push_str("\r\n");
        response.push_str(&self.body);
        response.into_bytes()
    }
}

/// A compiled route pattern that supports path parameters like `/users/:id`.
#[derive(Debug, Clone)]
pub struct RoutePattern {
    pub method: String,
    segments: Vec<PathSegment>,
    pub raw_path: String,
}

#[derive(Debug, Clone)]
enum PathSegment {
    Literal(String),
    Param(String),
    /// Matches any remaining path segments (the `*` glob).
    Wildcard,
}

impl RoutePattern {
    pub fn new(method: &str, path: &str) -> Self {
        let segments = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s == "*" {
                    PathSegment::Wildcard
                } else if let Some(name) = s.strip_prefix(':') {
                    PathSegment::Param(name.to_owned())
                } else {
                    PathSegment::Literal(s.to_owned())
                }
            })
            .collect();

        Self {
            method: method.to_uppercase(),
            segments,
            raw_path: path.to_owned(),
        }
    }

    /// Attempts to match a request path and extract parameters.
    pub fn match_path(&self, method: &str, path: &str) -> Option<HashMap<String, String>> {
        if self.method != method.to_uppercase() {
            return None;
        }

        let request_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // A single Wildcard segment matches any path (including empty).
        if self.segments.len() == 1 && matches!(self.segments[0], PathSegment::Wildcard) {
            return Some(HashMap::new());
        }

        if request_segments.len() != self.segments.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (pattern, actual) in self.segments.iter().zip(request_segments.iter()) {
            match pattern {
                PathSegment::Literal(expected) => {
                    if expected != actual {
                        return None;
                    }
                }
                PathSegment::Param(name) => {
                    params.insert(name.clone(), (*actual).to_owned());
                }
                PathSegment::Wildcard => {
                    // Wildcard matches any single segment
                }
            }
        }

        Some(params)
    }
}

/// Parse query string from URL path.
fn parse_query_string(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// Parse an HTTP request from a TCP stream.
fn parse_request(reader: &mut BufReader<&std::net::TcpStream>) -> Option<HttpRequest> {
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let method = parts[0].to_owned();
    let full_path = parts[1];

    // Split path and query string
    let (path, query_string) = if let Some(idx) = full_path.find('?') {
        (&full_path[..idx], &full_path[idx + 1..])
    } else {
        (full_path, "")
    };

    let path = path.to_owned();
    let query_params = parse_query_string(query_string);

    // Parse headers
    let mut headers = HashMap::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some(idx) = line.find(':') {
            let key = line[..idx].trim().to_lowercase();
            let value = line[idx + 1..].trim().to_owned();
            headers.insert(key, value);
        }
    }

    // Parse body if Content-Length is present (max 8 MiB to prevent OOM)
    const MAX_BODY_SIZE: usize = 8 * 1024 * 1024;
    let body = if let Some(len_str) = headers.get("content-length") {
        if let Ok(len) = len_str.parse::<usize>() {
            if len > MAX_BODY_SIZE {
                return None; // reject oversized payloads
            }
            let mut buf = vec![0u8; len];
            std::io::Read::read_exact(reader, &mut buf).ok()?;
            String::from_utf8(buf).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    Some(HttpRequest {
        method,
        path,
        headers,
        query_params,
        path_params: HashMap::new(),
        body,
    })
}

/// Route handler callback type.
pub type RouteHandler = Box<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync>;

/// A registered route with its pattern and handler.
struct RegisteredRoute {
    pattern: RoutePattern,
    handler: RouteHandler,
}

/// The HTTP server that dispatches requests to registered routes.
pub struct HttpServer {
    routes: Vec<RegisteredRoute>,
}

impl Default for HttpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpServer {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Register a route handler.
    pub fn route(&mut self, method: &str, path: &str, handler: RouteHandler) {
        self.routes.push(RegisteredRoute {
            pattern: RoutePattern::new(method, path),
            handler,
        });
    }

    /// Start listening and serving requests. Blocks forever.
    pub fn listen(self, port: u16) -> std::io::Result<()> {
        let addr = format!("127.0.0.1:{port}");
        let listener = TcpListener::bind(&addr)?;
        eprintln!("orv server listening on http://{addr}");

        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("connection error: {e}");
                    continue;
                }
            };

            // Prevent slowloris DoS by setting connection timeouts
            let timeout = Some(Duration::from_secs(30));
            let _ = stream.set_read_timeout(timeout);
            let _ = stream.set_write_timeout(timeout);

            let mut reader = BufReader::new(&stream);
            let Some(mut request) = parse_request(&mut reader) else {
                continue;
            };

            let response = self.dispatch(&mut request);

            if let Err(e) = (&stream).write_all(&response.to_http_bytes()) {
                eprintln!("write error: {e}");
            }
        }

        Ok(())
    }

    fn dispatch(&self, request: &mut HttpRequest) -> HttpResponse {
        for registered in &self.routes {
            if let Some(params) = registered
                .pattern
                .match_path(&request.method, &request.path)
            {
                request.path_params = params;
                return (registered.handler)(request);
            }
        }

        HttpResponse::not_found()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_pattern_exact_match() {
        let pattern = RoutePattern::new("GET", "/api/users");
        let result = pattern.match_path("GET", "/api/users");
        assert!(result.is_some());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn route_pattern_method_mismatch() {
        let pattern = RoutePattern::new("GET", "/api/users");
        assert!(pattern.match_path("POST", "/api/users").is_none());
    }

    #[test]
    fn route_pattern_path_param() {
        let pattern = RoutePattern::new("GET", "/users/:id");
        let result = pattern.match_path("GET", "/users/42");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("id").unwrap(), "42");
    }

    #[test]
    fn route_pattern_multiple_params() {
        let pattern = RoutePattern::new("GET", "/users/:user_id/posts/:post_id");
        let result = pattern.match_path("GET", "/users/1/posts/42");
        assert!(result.is_some());
        let params = result.unwrap();
        assert_eq!(params.get("user_id").unwrap(), "1");
        assert_eq!(params.get("post_id").unwrap(), "42");
    }

    #[test]
    fn route_pattern_length_mismatch() {
        let pattern = RoutePattern::new("GET", "/users/:id");
        assert!(pattern.match_path("GET", "/users").is_none());
        assert!(pattern.match_path("GET", "/users/1/extra").is_none());
    }

    #[test]
    fn route_pattern_literal_mismatch() {
        let pattern = RoutePattern::new("GET", "/api/users");
        assert!(pattern.match_path("GET", "/api/posts").is_none());
    }

    #[test]
    fn parse_query_params() {
        let params = parse_query_string("name=test&page=2&empty=");
        assert_eq!(params.get("name").unwrap(), "test");
        assert_eq!(params.get("page").unwrap(), "2");
        assert_eq!(params.get("empty").unwrap(), "");
    }

    #[test]
    fn parse_empty_query() {
        let params = parse_query_string("");
        assert!(params.is_empty());
    }

    #[test]
    fn http_response_json() {
        let resp = HttpResponse::json(200, r#"{"ok":true}"#);
        let bytes = resp.to_http_bytes();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: application/json"));
        assert!(text.contains(r#"{"ok":true}"#));
    }

    #[test]
    fn http_response_html() {
        let resp = HttpResponse::html(200, "<h1>Hello</h1>");
        let bytes = resp.to_http_bytes();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("text/html"));
        assert!(text.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn http_response_not_found() {
        let resp = HttpResponse::not_found();
        assert_eq!(resp.status, 404);
    }
}
