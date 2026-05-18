use super::*;

/// 포트 번호와 라우트 테이블을 들고 hyper 서버를 기동한다.
///
/// # Errors
/// - `listen` 이 Int 가 아니거나 포트 범위를 벗어나면 `RuntimeError`.
/// - 바인딩 실패도 `RuntimeError`.
/// - accept/serve 루프의 I/O 에러는 로그로 흘려보내고 다음 연결로 넘어간다
///   (한 커넥션 실패로 서버 전체가 죽지 않도록).
pub(crate) fn run_server_with_options(
    listen: Option<&HirExpr>,
    routes: &[HirExpr],
    body_stmts: &[orv_hir::HirStmt],
    captured_env: HashMap<NameId, Value>,
    captured_types: RuntimeTypeRegistry,
    db: DbHandle,
    runtime_options: RuntimeOptions,
) -> Result<Value, RuntimeError> {
    let mut stdout = std::io::stdout().lock();
    let (port, entries, captured, db) = prepare_server_state(
        listen,
        routes,
        body_stmts,
        CapturedRuntimeState::new(captured_env, captured_types),
        db,
        &mut stdout,
        false,
        runtime_options.clone(),
    )?;

    // 4) tokio current_thread 런타임 생성. 전용 런타임이라 스레드 이동 제약이
    //    없고, `!Send` HIR 값(Rc 기반 Value)도 요청 핸들러 안에서 그대로 사용
    //    가능하다. hyper 1.x 는 `Send + Sync` handler 를 요구하지 않으므로 이
    //    조합이 자연스럽다.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| RuntimeError::native(format!("tokio runtime init failed: {e}")))?;

    let request_trace_path = runtime_options
        .request_trace_path
        .clone()
        .or_else(runtime_request_trace_path_from_env);
    let local = tokio::task::LocalSet::new();
    runtime.block_on(local.run_until(async move {
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| RuntimeError::native(format!("failed to bind {addr}: {e}")))?;
        // Graceful shutdown — SIGINT (ctrl_c) + SIGTERM (Unix).
        //
        // SIGTERM 은 컨테이너/systemd 가 기본으로 보내는 신호라 SIGINT 만으로는
        // 프로덕션 배포에서 graceful 이 안 먹는다. Windows 타깃은 SIGTERM
        // 개념이 없으므로 `#[cfg(unix)]` 로 갈라친다.
        serve_loop_with_request_trace_file(
            listener,
            LocalRoutes::new(entries),
            LocalCapturedEnv::new(captured),
            db,
            None,
            request_trace_path,
            runtime_options,
            shutdown_signal(),
        )
        .await
    }))?;

    Ok(Value::Void)
}

/// SIGINT + (Unix) SIGTERM 둘 중 하나가 오면 resolve 되는 Future.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to install SIGTERM handler: {e}");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// 서버 기동 전 상태 — `(포트, 라우트 테이블, 캡처 런타임 상태, 공유 DB)`.
pub(super) type PreparedServerState = (u16, Vec<RouteEntry>, CapturedRuntimeState, DbHandle);

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_server_state<W: std::io::Write>(
    listen: Option<&HirExpr>,
    routes: &[HirExpr],
    body_stmts: &[orv_hir::HirStmt],
    captured: CapturedRuntimeState,
    db: DbHandle,
    boot_writer: &mut W,
    allow_ephemeral_port: bool,
    runtime_options: RuntimeOptions,
) -> Result<PreparedServerState, RuntimeError> {
    // 1) body_stmts 평가 — `@out` 같은 부트 출력뿐 아니라 server-level
    //    let/const/function 선언도 여기서 캡처된 환경 위에 쌓아 handler 가
    //    볼 수 있게 만든다. `@listen port` 같은 표현식도 이 환경을 보게 하기
    //    위해 포트 결정보다 먼저 수행한다.
    let captured = if body_stmts.is_empty() {
        captured
    } else {
        let boot_program = HirProgram {
            items: body_stmts.to_vec(),
            span: body_stmts[0].span(),
        };
        let (env, types) = run_with_writer_in_env_and_types_with_db_and_options(
            &boot_program,
            captured.env,
            captured.types,
            db.clone(),
            boot_writer,
            runtime_options,
        )?;
        CapturedRuntimeState::new(env, types)
    };

    // 2) listen 포트 결정. 운영 경로는 @listen 없으면 에러, 테스트 경로는 `0`
    //    을 허용해 OS 임의 포트 바인딩을 사용할 수 있다.
    let port = resolve_listen_port(listen, &captured.env, allow_ephemeral_port)?;

    // 3) routes → RouteEntry 로 평평하게. analyzer 가 routes 벡터에 Route
    //    variant 만 넣기로 계약했으므로 그 외는 에러.
    let entries = collect_routes(routes)?;

    Ok((port, entries, captured, db))
}

fn resolve_listen_port(
    listen: Option<&HirExpr>,
    env: &HashMap<NameId, Value>,
    allow_ephemeral_port: bool,
) -> Result<u16, RuntimeError> {
    let Some(expr) = listen else {
        return Err(RuntimeError::native(
            "`@server` requires an `@listen PORT` declaration",
        ));
    };
    // `@listen` 은 이제 캡처 환경을 보는 식을 허용한다. top-level/server-level
    // 바인딩, 괄호식, 간단한 산술 등을 평가한 뒤 정수 포트로 해석한다.
    let mut sink = Vec::new();
    let value = eval_expr_in_env(expr, env, &mut sink)?;
    let n = match value {
        Value::Int(n) => n,
        other => {
            return Err(RuntimeError::native(format!(
                "`@listen` port expression must evaluate to int, got {other}"
            )));
        }
    };
    let valid = if allow_ephemeral_port {
        (0..=65535).contains(&n)
    } else {
        (1..=65535).contains(&n)
    };
    if !valid {
        let range = if allow_ephemeral_port {
            "0..=65535"
        } else {
            "1..=65535"
        };
        return Err(RuntimeError::native(format!(
            "@listen port out of range {range}: {n}"
        )));
    }
    u16::try_from(n).map_err(|_| {
        RuntimeError::native(format!(
            "@listen port out of range {}: {n}",
            if allow_ephemeral_port {
                "0..=65535"
            } else {
                "1..=65535"
            }
        ))
    })
}

fn collect_routes(routes: &[HirExpr]) -> Result<Vec<RouteEntry>, RuntimeError> {
    let mut out = Vec::with_capacity(routes.len());
    for expr in routes {
        let HirExprKind::Route {
            method,
            path,
            handler,
            ..
        } = &expr.kind
        else {
            return Err(RuntimeError::native(
                "internal: @server routes slot contains non-Route HIR (analyzer contract violated)",
            ));
        };
        // handler 는 HirBlock 이지만 Interp::eval 은 HirExpr 을 받는다. 요청
        // 시점에 HirExprKind::Block 으로 감싸 평가하기 쉽도록 미리 변환.
        let handler_expr = HirExpr {
            kind: HirExprKind::Block(handler.clone()),
            ty: orv_hir::Type::Unknown,
            span: expr.span,
        };
        out.push(RouteEntry {
            method: method.clone(),
            path: path.clone(),
            handler: handler_expr,
            origin_id: origin_id("route", &format!("{method} {path}"), expr.span),
            rate_limit: route_rate_limit_policy(method, path, handler)?,
        });
    }
    Ok(out)
}

#[allow(clippy::future_not_send)]
pub(super) async fn serve_loop<S>(
    listener: TcpListener,
    routes: LocalRoutes,
    captured_env: LocalCapturedEnv,
    db: DbHandle,
    trace_state: Option<TraceState>,
    runtime_options: RuntimeOptions,
    shutdown: S,
) -> Result<(), RuntimeError>
where
    S: std::future::Future<Output = ()>,
{
    // C_db: 서버 수명 동안 공유하는 DB handle. Server boot body가 `@db.wal`
    // 또는 `@db.load`로 구성한 persistence 설정도 같은 handle을 통해 route
    // handler에 전달된다.
    // shutdown 은 단일 해상도 이벤트라 `tokio::pin!` 로 고정해 `select!` 에서
    // `&mut` 참조로 폴링한다. 이렇게 해야 매 반복에서 future 를 소비하지 않고
    // 재진입이 가능하다.
    tokio::pin!(shutdown);
    let rate_limits = RateLimitState::default();
    loop {
        let (stream, peer) = tokio::select! {
            biased;
            // shutdown 우선. accept 가 동시에 준비되어도 먼저 빠져나간다.
            () = &mut shutdown => return Ok(()),
            accept_result = listener.accept() => match accept_result {
                Ok(pair) => pair,
                Err(e) => {
                    eprintln!("accept error: {e}");
                    continue;
                }
            }
        };
        let io = TokioIo::new(stream);
        let routes = routes.clone();
        let captured_env = captured_env.clone();
        let db = db.clone();
        let trace_state = trace_state.clone();
        let rate_limits = rate_limits.clone();
        let client_ip = peer.ip().to_string();
        let runtime_options = runtime_options.clone();
        let service = service_fn(move |req| {
            let routes = routes.clone();
            let captured_env = captured_env.clone();
            let db = db.clone();
            let trace_state = trace_state.clone();
            let rate_limits = rate_limits.clone();
            let client_ip = client_ip.clone();
            let runtime_options = runtime_options.clone();
            async move {
                Ok::<_, Infallible>(
                    handle_request(
                        req,
                        routes,
                        captured_env,
                        db,
                        client_ip,
                        trace_state,
                        rate_limits,
                        runtime_options,
                    )
                    .await,
                )
            }
        });
        tokio::task::spawn_local(async move {
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .keep_alive(false)
                .serve_connection(io, service)
                .await
            {
                eprintln!("connection error: {e}");
            }
        });
    }
}

#[allow(clippy::future_not_send)]
#[allow(clippy::too_many_arguments)]
pub(super) async fn serve_loop_with_request_trace_file<S>(
    listener: TcpListener,
    routes: LocalRoutes,
    captured_env: LocalCapturedEnv,
    db: DbHandle,
    trace_state: Option<TraceState>,
    request_trace_path: Option<std::path::PathBuf>,
    runtime_options: RuntimeOptions,
    shutdown: S,
) -> Result<(), RuntimeError>
where
    S: std::future::Future<Output = ()>,
{
    let trace_state =
        trace_state.or_else(|| request_trace_path.as_ref().map(|_| TraceState::new()));
    serve_loop(
        listener,
        routes,
        captured_env,
        db,
        trace_state.clone(),
        runtime_options,
        shutdown,
    )
    .await?;
    if let (Some(path), Some(trace_state)) = (request_trace_path, trace_state) {
        write_request_trace_file(&path, &trace_state.frames())?;
    }
    Ok(())
}
