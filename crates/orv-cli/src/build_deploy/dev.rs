use super::*;

#[derive(Clone, Copy)]
pub(crate) struct DevOptions {
    pub(crate) hmr: bool,
    pub(crate) watch: bool,
    pub(crate) loop_mode: DevLoopMode,
    pub(crate) serve: Option<DevServeOptions>,
}

#[derive(Clone, Copy)]
pub(crate) struct DevServeOptions {
    pub(crate) port: u16,
    pub(crate) iterations: Option<u64>,
    pub(crate) interval_ms: u64,
}

#[derive(Clone, Copy)]
pub(crate) enum DevLoopMode {
    Once,
    WatchLoop {
        iterations: Option<u64>,
        interval_ms: u64,
    },
}

pub(crate) fn cmd_dev(path: &Path, out: &Path, options: DevOptions) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    if let Some(serve) = options.serve {
        return dev_hmr_serve_with_writer(
            path,
            out,
            serve.port,
            serve.iterations,
            serve.interval_ms,
            &mut stdout,
        );
    }
    if let DevLoopMode::WatchLoop {
        iterations,
        interval_ms,
    } = options.loop_mode
    {
        return dev_watch_loop_with_writer(
            path,
            out,
            options.hmr,
            iterations,
            interval_ms,
            &mut stdout,
        );
    }
    if options.hmr {
        dev_with_writer_with_options(path, out, true, options.watch, &mut stdout)
    } else if options.watch {
        dev_with_writer_with_options(path, out, false, true, &mut stdout)
    } else {
        dev_with_writer(path, out, &mut stdout)
    }
}

pub(crate) struct DevHmrServer {
    pub(crate) addr: SocketAddr,
    pub(crate) shutdown: Arc<AtomicBool>,
    pub(crate) handle: Option<JoinHandle<()>>,
}

impl DevHmrServer {
    pub(crate) const fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for DevHmrServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = std::net::TcpStream::connect(self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub(crate) fn dev_hmr_serve_with_writer<W: std::io::Write>(
    path: &Path,
    out: &Path,
    port: u16,
    iterations: Option<u64>,
    interval_ms: u64,
    writer: &mut W,
) -> anyhow::Result<()> {
    validate_dev_loop_options(iterations, interval_ms)?;
    let mut events = Vec::new();
    let mut previous_signature: Option<String> = None;
    let mut server: Option<DevHmrServer> = None;
    let mut iteration = 0_u64;

    loop {
        iteration = iteration.saturating_add(1);
        let reason = dev_watch_loop_reason(out, previous_signature.as_deref())?;
        if reason == "unchanged" {
            events.push(dev_watch_loop_event(iteration, reason, "skip", "ok", None));
        } else {
            dev_with_writer_with_options(path, out, true, true, writer)?;
            let signature = dev_watch_current_source_signature(out)?;
            events.push(dev_watch_loop_event(
                iteration,
                reason,
                "build-verify-run",
                "ok",
                Some(&signature),
            ));
            previous_signature = Some(signature);
        }
        write_dev_watch_events(out, true, interval_ms, &events)?;

        if server.is_none() {
            let spawned = spawn_dev_hmr_server(out, port)?;
            writeln!(writer, "\n[orv dev] hmr server http://{}", spawned.addr())?;
            server = Some(spawned);
        }
        if iterations.is_some_and(|limit| iteration >= limit) {
            break;
        }
        std::thread::sleep(Duration::from_millis(interval_ms));
    }
    drop(server);
    Ok(())
}

pub(crate) fn spawn_dev_hmr_server(out: &Path, port: u16) -> anyhow::Result<DevHmrServer> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| anyhow::anyhow!("failed to bind HMR dev server: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| anyhow::anyhow!("failed to configure HMR dev server: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| anyhow::anyhow!("failed to read HMR dev server address: {e}"))?;
    write_dev_hmr_server_manifest(out, addr)?;

    let root = out.to_path_buf();
    let shutdown = Arc::new(AtomicBool::new(false));
    let worker_shutdown = Arc::clone(&shutdown);
    let handle =
        std::thread::spawn(move || dev_hmr_server_loop(&listener, &root, &worker_shutdown));
    Ok(DevHmrServer {
        addr,
        shutdown,
        handle: Some(handle),
    })
}

pub(crate) fn write_dev_hmr_server_manifest(out: &Path, addr: SocketAddr) -> anyhow::Result<()> {
    let server = serde_json::json!({
        "schema_version": 1,
        "mode": "hmr-server",
        "protocol": "http1",
        "address": addr.to_string(),
        "source_bundle": "source-bundle.json",
        "session": "dev/session.json",
        "events": "dev/events.json",
        "endpoints": {
            "session": "/__orv/hmr/session",
            "events": "/__orv/hmr/events",
        },
    });
    write_json(&out.join("dev").join("server.json"), &server)
}

pub(crate) fn dev_hmr_server_loop(
    listener: &std::net::TcpListener,
    out: &Path,
    shutdown: &AtomicBool,
) {
    while !shutdown.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = handle_dev_hmr_connection(stream, out);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break,
        }
    }
}

pub(crate) fn handle_dev_hmr_connection(
    mut stream: std::net::TcpStream,
    out: &Path,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") && request.len() < 8192 {
        let read = std::io::Read::read(&mut stream, &mut buffer)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
    }
    let path = dev_hmr_request_path(&request).unwrap_or("/");
    let response = dev_hmr_http_response(out, path)
        .unwrap_or_else(|err| dev_hmr_text_response("500 Internal Server Error", &err.to_string()));
    std::io::Write::write_all(&mut stream, &response)
}

pub(crate) fn dev_hmr_request_path(request: &[u8]) -> Option<&str> {
    let request = std::str::from_utf8(request).ok()?;
    let line = request.lines().next()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let path = parts.next()?;
    (method == "GET").then_some(path)
}

pub(crate) fn dev_hmr_http_response(out: &Path, path: &str) -> anyhow::Result<Vec<u8>> {
    match path {
        "/__orv/hmr/session" => {
            let body = std::fs::read_to_string(out.join("dev").join("session.json"))?;
            Ok(dev_hmr_response(
                "200 OK",
                "application/json",
                "no-cache",
                &body,
            ))
        }
        "/__orv/hmr/events" => {
            let events = read_json_value(&out.join("dev").join("events.json"))?;
            let body = dev_hmr_sse_body(&events);
            Ok(dev_hmr_response(
                "200 OK",
                "text/event-stream",
                "no-cache",
                &body,
            ))
        }
        _ => Ok(dev_hmr_text_response("404 Not Found", "not found")),
    }
}

pub(crate) fn dev_hmr_sse_body(events: &serde_json::Value) -> String {
    let mut body = String::new();
    if let Some(items) = events.get("events").and_then(serde_json::Value::as_array) {
        for event in items {
            let data = serde_json::to_string(event).unwrap_or_else(|_| "{}".to_string());
            let _ = write!(body, "event: message\ndata: {data}\n\n");
            if event.get("action").and_then(serde_json::Value::as_str) == Some("build-verify-run") {
                let _ = write!(body, "event: orv:reload\ndata: {data}\n\n");
            }
        }
    }
    body
}

pub(crate) fn dev_hmr_text_response(status: &str, body: &str) -> Vec<u8> {
    dev_hmr_response(status, "text/plain; charset=utf-8", "no-cache", body)
}

pub(crate) fn dev_hmr_response(
    status: &str,
    content_type: &str,
    cache_control: &str,
    body: &str,
) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nCache-Control: {cache_control}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

pub(crate) fn dev_watch_loop_with_writer<W: std::io::Write>(
    path: &Path,
    out: &Path,
    hmr: bool,
    iterations: Option<u64>,
    interval_ms: u64,
    writer: &mut W,
) -> anyhow::Result<()> {
    validate_dev_loop_options(iterations, interval_ms)?;

    let mut events = Vec::new();
    let mut previous_signature: Option<String> = None;
    let mut iteration = 0_u64;
    loop {
        iteration = iteration.saturating_add(1);
        let reason = dev_watch_loop_reason(out, previous_signature.as_deref())?;
        if reason == "unchanged" {
            events.push(dev_watch_loop_event(iteration, reason, "skip", "ok", None));
        } else {
            dev_with_writer_with_options(path, out, hmr, true, writer)?;
            let signature = dev_watch_current_source_signature(out)?;
            events.push(dev_watch_loop_event(
                iteration,
                reason,
                "build-verify-run",
                "ok",
                Some(&signature),
            ));
            previous_signature = Some(signature);
        }
        write_dev_watch_events(out, hmr, interval_ms, &events)?;

        if iterations.is_some_and(|limit| iteration >= limit) {
            break;
        }
        std::thread::sleep(Duration::from_millis(interval_ms));
    }
    Ok(())
}

pub(crate) fn validate_dev_loop_options(
    iterations: Option<u64>,
    interval_ms: u64,
) -> anyhow::Result<()> {
    if interval_ms == 0 {
        anyhow::bail!("watch loop interval_ms must be positive");
    }
    if iterations == Some(0) {
        anyhow::bail!("watch loop iterations must be positive");
    }
    Ok(())
}

pub(crate) fn dev_watch_loop_reason(
    out: &Path,
    previous_signature: Option<&str>,
) -> anyhow::Result<&'static str> {
    let Some(signature) = previous_signature else {
        return Ok("initial");
    };
    let current = dev_watch_current_source_signature(out)?;
    if current == signature {
        Ok("unchanged")
    } else {
        Ok("changed")
    }
}

pub(crate) fn dev_watch_loop_event(
    iteration: u64,
    reason: &str,
    action: &str,
    status: &str,
    source_signature: Option<&str>,
) -> serde_json::Value {
    let mut event = serde_json::json!({
        "iteration": iteration,
        "reason": reason,
        "action": action,
        "status": status,
        "watch": "dev/watch.json",
    });
    if let Some(signature) = source_signature {
        event["source_signature"] = serde_json::json!(signature);
    }
    event
}

pub(crate) fn write_dev_watch_events(
    out: &Path,
    hmr: bool,
    interval_ms: u64,
    events: &[serde_json::Value],
) -> anyhow::Result<()> {
    let value = serde_json::json!({
        "schema_version": 1,
        "mode": "watch-loop",
        "source_bundle": "source-bundle.json",
        "loop": {
            "strategy": "poll",
            "interval_ms": interval_ms,
            "run": "build-verify-run",
            "hmr": hmr,
        },
        "transport": {
            "kind": "manifest",
            "path": "dev/events.json",
        },
        "events": events,
    });
    write_json(&out.join("dev").join("events.json"), &value)
}

pub(crate) fn dev_watch_current_source_signature(out: &Path) -> anyhow::Result<String> {
    let session = read_json_value(&out.join("dev").join("watch.json"))?;
    let sources = session
        .pointer("/watch/sources")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("dev watch session watch.sources must be an array"))?;
    let mut current = Vec::with_capacity(sources.len());
    for source in sources {
        let path = json_str(source, "path", "dev watch source")?;
        let bytes = std::fs::read(path)
            .map_err(|e| anyhow::anyhow!("failed to read watched source {path}: {e}"))?;
        current.push(serde_json::json!({
            "path": path,
            "content_hash": format!("fnv1a64:{:016x}", fnv1a64(&bytes)),
        }));
    }
    stable_json_hash(&serde_json::Value::Array(current))
}

pub(crate) fn dev_with_writer<W: std::io::Write>(
    path: &Path,
    out: &Path,
    writer: &mut W,
) -> anyhow::Result<()> {
    dev_with_writer_with_options(path, out, false, false, writer)
}

pub(crate) fn dev_with_writer_with_options<W: std::io::Write>(
    path: &Path,
    out: &Path,
    hmr: bool,
    watch: bool,
    writer: &mut W,
) -> anyhow::Result<()> {
    cmd_build(path, out)?;
    verify_build_dir(out)?;
    if hmr {
        write_dev_hmr_session(out)?;
        write_dev_hmr_transport(out)?;
    }
    if watch {
        write_dev_watch_session(out, hmr)?;
    }
    run_build_with_writer(out, writer)
}

pub(crate) fn write_dev_hmr_session(out: &Path) -> anyhow::Result<()> {
    let (sources, targets, has_client_target) = dev_session_inputs(out)?;
    let session = serde_json::json!({
        "schema_version": 1,
        "mode": "hmr",
        "source_bundle": "source-bundle.json",
        "watch": {
            "sources": sources,
            "targets": targets,
        },
        "reload": {
            "strategy": if has_client_target { "hot-reload" } else { "full-reload" },
            "fallback": "full-reload",
            "state": if has_client_target { "preserve-sig-state-when-compatible" } else { "stateless" },
        },
    });
    write_json(&out.join("dev").join("session.json"), &session)
}

pub(crate) fn write_dev_hmr_transport(out: &Path) -> anyhow::Result<()> {
    let transport = serde_json::json!({
        "schema_version": 1,
        "mode": "hmr-transport",
        "source_bundle": "source-bundle.json",
        "session": "dev/session.json",
        "browser": {
            "kind": "event-source",
            "client": "dev/hmr-client.js",
            "event_source": "/__orv/hmr/events",
            "session": "/__orv/hmr/session",
            "reload_event": "orv:reload",
        },
        "server": {
            "kind": "reference-dev",
            "events": "dev/events.json",
            "session": "dev/session.json",
        },
    });
    write_json(&out.join("dev").join("transport.json"), &transport)?;
    write_text(&out.join("dev").join("hmr-client.js"), DEV_HMR_CLIENT_JS)
}

pub(crate) const DEV_HMR_CLIENT_JS: &str = r"(function () {
  if (!('EventSource' in window)) {
    return;
  }
  var source = new EventSource('/__orv/hmr/events');
  source.addEventListener('message', function (event) {
    var payload = {};
    try {
      payload = JSON.parse(event.data || '{}');
    } catch (_) {
      payload = {};
    }
    window.dispatchEvent(new CustomEvent('orv:hmr', { detail: payload }));
    if (payload.action === 'build-verify-run' || payload.action === 'reload') {
      window.location.reload();
    }
  });
  source.addEventListener('orv:reload', function () {
    window.location.reload();
  });
}());
";

pub(crate) fn write_dev_watch_session(out: &Path, hmr: bool) -> anyhow::Result<()> {
    let (sources, targets, has_client_target) = dev_session_inputs(out)?;
    let session = serde_json::json!({
        "schema_version": 1,
        "mode": "watch",
        "source_bundle": "source-bundle.json",
        "watch": {
            "sources": sources,
            "targets": targets,
        },
        "loop": {
            "strategy": "poll",
            "interval_ms": 500,
            "run": "build-verify-run",
            "hmr": hmr,
        },
        "reload": {
            "strategy": if hmr && has_client_target { "hot-reload" } else { "full-reload" },
            "fallback": "full-reload",
            "state": if hmr && has_client_target { "preserve-sig-state-when-compatible" } else { "stateless" },
        },
        "transport": {
            "kind": "manifest",
            "path": "dev/watch.json",
        },
    });
    write_json(&out.join("dev").join("watch.json"), &session)
}

pub(crate) fn dev_session_inputs(
    out: &Path,
) -> anyhow::Result<(Vec<serde_json::Value>, Vec<serde_json::Value>, bool)> {
    let source_bundle = read_json_value(&out.join("source-bundle.json"))?;
    let bundle_plan = read_json_value(&out.join("bundle-plan.json"))?;
    let sources = source_bundle
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("source-bundle.json files must be an array"))?
        .iter()
        .map(|source| {
            Ok(serde_json::json!({
                "path": json_string_field(source, "path", "source bundle file")?,
                "content_hash": json_string_field(source, "content_hash", "source bundle file")?,
            }))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let targets = bundle_plan
        .get("bundles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("bundle-plan.json bundles must be an array"))?
        .iter()
        .map(|target| {
            let runtime_features = target
                .get("runtime_features")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    anyhow::anyhow!("bundle target runtime_features must be an array")
                })?;
            Ok(serde_json::json!({
                "kind": json_string_field(target, "kind", "bundle target")?,
                "path": json_string_field(target, "path", "bundle target")?,
                "runtime_features": runtime_features,
            }))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let has_client_target = targets.iter().any(|target| {
        target
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .is_some_and(is_client_bundle_kind)
    });
    Ok((sources, targets, has_client_target))
}
