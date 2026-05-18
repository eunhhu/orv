use super::*;

#[derive(Clone)]
pub(in crate::server) struct TraceState {
    frames: Arc<Mutex<Vec<ServerRequestFrame>>>,
    subscribers: Arc<Mutex<Vec<tokio_mpsc::UnboundedSender<Bytes>>>>,
}

impl TraceState {
    pub(super) fn new() -> Self {
        Self {
            frames: Arc::new(Mutex::new(Vec::new())),
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(super) fn frames_handle(&self) -> Arc<Mutex<Vec<ServerRequestFrame>>> {
        Arc::clone(&self.frames)
    }

    pub(super) fn frames(&self) -> Vec<ServerRequestFrame> {
        self.frames
            .lock()
            .map_or_else(|_| Vec::new(), |frames| frames.clone())
    }

    fn subscribe(&self) -> tokio_mpsc::UnboundedReceiver<Bytes> {
        let (tx, rx) = tokio_mpsc::unbounded_channel();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(tx);
        }
        rx
    }

    fn record(&self, frame: ServerRequestFrame) {
        let event = match self.frames.lock() {
            Ok(mut frames) => {
                let index = frames.len();
                let event = Bytes::from(request_trace_frame_event_body(index, &frame));
                frames.push(frame);
                event
            }
            Err(_) => Bytes::from(request_trace_frame_event_body(0, &frame)),
        };
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|subscriber| subscriber.send(event.clone()).is_ok());
        }
    }
}

/// Captured metadata for one HTTP request handled by an attached runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerRequestFrame {
    /// Request method after HTTP parsing.
    pub method: String,
    /// Normalized request path, without query string.
    pub path: String,
    /// Route method that matched the request.
    pub route_method: Option<String>,
    /// Route path pattern that matched the request.
    pub route_path: Option<String>,
    /// Origin id for the matched route.
    pub route_origin_id: Option<String>,
    /// Origin id for the response-producing source node, when known.
    pub response_origin_id: Option<String>,
    /// HTTP response status returned to the client.
    pub status: u16,
    /// Captured path parameters.
    pub params: HashMap<String, String>,
    /// Captured query parameters.
    pub query: HashMap<String, String>,
    /// Compact request body display.
    pub body: String,
}

pub(in crate::server) fn record_request_frame(
    trace_state: Option<&TraceState>,
    frame: ServerRequestFrame,
) {
    if let Some(trace_state) = trace_state {
        trace_state.record(frame);
    }
}

pub(in crate::server) fn request_trace_events_response(trace_state: &TraceState) -> ServerResponse {
    let frames = trace_state.frames();
    let body = request_trace_events_body(&frames);
    let rx = trace_state.subscribe();
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(RuntimeBody::trace(body, rx))
        .expect("valid trace event stream response")
}

fn request_trace_events_body(frames: &[ServerRequestFrame]) -> String {
    let mut body = String::new();
    let payload = serde_json::to_string(&request_trace_json(frames)).unwrap_or_default();
    body.push_str("event: orv:trace\n");
    body.push_str("data: ");
    body.push_str(&payload);
    body.push_str("\n\n");
    for (index, frame) in frames.iter().enumerate() {
        body.push_str(&request_trace_frame_event_body(index, frame));
    }
    body
}

fn request_trace_frame_event_body(index: usize, frame: &ServerRequestFrame) -> String {
    let payload =
        serde_json::to_string(&request_trace_frame_event_json(index, frame)).unwrap_or_default();
    format!("event: orv:trace.frame\ndata: {payload}\n\n")
}

/// Build the shared production request trace JSON document.
#[must_use]
pub fn request_trace_json(frames: &[ServerRequestFrame]) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace",
        "frame_count": frames.len(),
        "frames": frames
            .iter()
            .map(request_frame_json)
            .collect::<Vec<_>>(),
    })
}

fn request_trace_frame_event_json(index: usize, frame: &ServerRequestFrame) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.production.trace.frame",
        "index": index,
        "frame": request_frame_json(frame),
    })
}

/// Write captured request frames as an `orv.production.trace` JSON file.
///
/// # Errors
/// Returns a runtime error if a parent directory cannot be created or if the
/// JSON file cannot be serialized/written.
pub fn write_request_trace_file(
    path: &std::path::Path,
    frames: &[ServerRequestFrame],
) -> Result<(), RuntimeError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            RuntimeError::native(format!(
                "failed to create request trace directory {}: {e}",
                parent.display()
            ))
        })?;
    }
    let bytes = serde_json::to_vec_pretty(&request_trace_json(frames))
        .map_err(|e| RuntimeError::native(format!("failed to encode request trace JSON: {e}")))?;
    std::fs::write(path, bytes).map_err(|e| {
        RuntimeError::native(format!(
            "failed to write request trace file {}: {e}",
            path.display()
        ))
    })
}

fn request_frame_json(frame: &ServerRequestFrame) -> serde_json::Value {
    serde_json::json!({
        "method": &frame.method,
        "path": &frame.path,
        "status": frame.status,
        "route_method": frame.route_method.as_deref(),
        "route_path": frame.route_path.as_deref(),
        "route_origin_id": frame.route_origin_id.as_deref(),
        "response_origin_id": frame.response_origin_id.as_deref(),
        "params": &frame.params,
        "query": &frame.query,
        "body": &frame.body,
    })
}
