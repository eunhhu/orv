use super::serve::{prepare_server_state, serve_loop};
use super::*;

/// Handle for an in-process attached `@server` runtime.
///
/// Dropping the handle sends a graceful shutdown signal and joins the runtime
/// thread before returning.
pub struct AttachedServer {
    addr: SocketAddr,
    boot_output: Vec<u8>,
    request_frames: Arc<Mutex<Vec<ServerRequestFrame>>>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

type AttachedStartup = Result<
    (
        SocketAddr,
        Vec<u8>,
        Arc<Mutex<Vec<ServerRequestFrame>>>,
        tokio::sync::oneshot::Sender<()>,
    ),
    String,
>;

impl AttachedServer {
    /// Bound socket address for the attached server.
    #[must_use]
    pub const fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Output produced while preparing the attached server.
    #[must_use]
    pub fn boot_output(&self) -> &[u8] {
        &self.boot_output
    }

    /// Request frames captured by this attached server so far.
    ///
    /// If the internal lock is poisoned, returns an empty vector. The debug
    /// surface treats request frames as best-effort telemetry.
    #[must_use]
    pub fn request_frames(&self) -> Vec<ServerRequestFrame> {
        self.request_frames
            .lock()
            .map_or_else(|_| Vec::new(), |frames| frames.clone())
    }
}

impl Drop for AttachedServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Start the first `@server` expression in `program` on a dedicated in-process
/// runtime thread.
///
/// This tooling entry point permits `@listen 0` so callers can request an
/// ephemeral local port.
///
/// # Errors
/// Returns a runtime error when the program does not contain an `@server`, when
/// the server cannot prepare its routes/listen port, or when the listener fails
/// to bind.
pub fn spawn_attached_server(program: HirProgram) -> Result<AttachedServer, RuntimeError> {
    let (startup_tx, startup_rx) = mpsc::sync_channel::<AttachedStartup>(1);
    let handle = thread::spawn(move || attached_server_thread(&program, &startup_tx));
    match startup_rx.recv() {
        Ok(Ok((addr, boot_output, request_frames, shutdown))) => Ok(AttachedServer {
            addr,
            boot_output,
            request_frames,
            shutdown: Some(shutdown),
            handle: Some(handle),
        }),
        Ok(Err(message)) => {
            let _ = handle.join();
            Err(RuntimeError::native(message))
        }
        Err(err) => {
            let _ = handle.join();
            Err(RuntimeError::native(format!(
                "attached server failed before startup: {err}"
            )))
        }
    }
}

fn attached_server_thread(program: &HirProgram, startup: &mpsc::SyncSender<AttachedStartup>) {
    if let Err(message) = run_attached_server_thread(program, startup) {
        let _ = startup.send(Err(message));
    }
}

fn run_attached_server_thread(
    program: &HirProgram,
    startup: &mpsc::SyncSender<AttachedStartup>,
) -> Result<(), String> {
    let mut boot_output = Vec::new();
    let (port, entries, captured, db) =
        attached_server_state(program, &mut boot_output).map_err(|e| e.to_string())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime init failed: {e}"))?;
    let local = tokio::task::LocalSet::new();
    runtime.block_on(local.run_until(async move {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|e| format!("attached server bind failed: {e}"))?;
        let addr = listener
            .local_addr()
            .map_err(|e| format!("attached server local_addr failed: {e}"))?;
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let trace_state = TraceState::new();
        let request_frames = trace_state.frames_handle();
        startup
            .send(Ok((
                addr,
                boot_output,
                Arc::clone(&request_frames),
                shutdown_tx,
            )))
            .map_err(|_| "attached server startup receiver dropped".to_string())?;
        serve_loop(
            listener,
            LocalRoutes::new(entries),
            LocalCapturedEnv::new(captured),
            db,
            Some(trace_state),
            RuntimeOptions::default(),
            async move {
                let _ = shutdown_rx.await;
            },
        )
        .await
        .map_err(|e| e.to_string())
    }))
}

fn attached_server_state<W: std::io::Write>(
    program: &HirProgram,
    boot_writer: &mut W,
) -> Result<super::serve::PreparedServerState, RuntimeError> {
    let db = new_db_handle();
    let server_idx = program
        .items
        .iter()
        .position(|stmt| {
            matches!(stmt, HirStmt::Expr(expr) if matches!(expr.kind, HirExprKind::Server { .. }))
        })
        .ok_or_else(|| RuntimeError::native("attached runtime requires an `@server` expression"))?;
    let (captured_env, captured_types) = if server_idx == 0 {
        (HashMap::new(), RuntimeTypeRegistry::default())
    } else {
        let prefix = HirProgram {
            items: program.items[..server_idx].to_vec(),
            span: program.items[0]
                .span()
                .join(program.items[server_idx - 1].span()),
        };
        run_with_writer_in_env_and_types_with_db(
            &prefix,
            HashMap::new(),
            RuntimeTypeRegistry::default(),
            db.clone(),
            boot_writer,
        )?
    };
    let HirStmt::Expr(expr) = &program.items[server_idx] else {
        return Err(RuntimeError::native("attached runtime expected expression"));
    };
    let HirExprKind::Server {
        listen,
        routes,
        body_stmts,
    } = &expr.kind
    else {
        return Err(RuntimeError::native("attached runtime expected server"));
    };
    prepare_server_state(
        listen.as_deref(),
        routes,
        body_stmts,
        CapturedRuntimeState::new(captured_env, captured_types),
        db,
        boot_writer,
        true,
        RuntimeOptions::default(),
    )
}
