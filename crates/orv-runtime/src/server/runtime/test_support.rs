use super::serve::{prepare_server_state, serve_loop, serve_loop_with_request_trace_file};
use std::collections::HashMap;
use std::net::SocketAddr;

use orv_hir::{HirExpr, NameId};
use tokio::net::TcpListener;

use crate::db::new_db_handle;
use crate::interp::{RuntimeError, RuntimeOptions, RuntimeTypeRegistry, Value};

use super::super::{CapturedRuntimeState, LocalCapturedEnv, LocalRoutes};

/// 테스트에서 임의의 포트에 바인딩하고 주소를 돌려받기 위한 진입점.
///
/// 운영 경로([`run_server`])와 다른 점:
/// - 포트 0 으로 바인딩해 OS 에 맡기고 실제 주소를 반환한다.
/// - accept 루프는 별도 tokio task 로 띄우고 즉시 `(addr, handle, boot)` 를
///   돌려준다.
/// - 호출자는 테스트 끝에 `handle.abort()` 로 서버를 정리한다.
///
/// `body_stmts` 는 `@server { @out "boot" @listen 0 ... }` 처럼 @server 블록
/// 최상단에 있던 non-route 문장들이다. [`run_server`] 는 이들을 accept 시작
/// 전에 **공용 stdout** 으로 흘린다. 테스트에서는 stdout 을 가로챌 수 없어
/// 같은 순서로 `Vec<u8>` writer 에 캡처해 돌려준다 — C5c 의 `body_stmts` 패치가
/// 실제로 런타임에 도달하는지 fixture 수준에서 증명하기 위함.
#[allow(clippy::future_not_send)]
pub(in crate::server) async fn spawn_for_test<S>(
    listen: Option<&HirExpr>,
    routes: &[HirExpr],
    body_stmts: &[orv_hir::HirStmt],
    captured_env: HashMap<NameId, Value>,
    shutdown: S,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>, Vec<u8>), RuntimeError>
where
    S: std::future::Future<Output = ()> + 'static,
{
    let mut boot_buf: Vec<u8> = Vec::new();
    let (port, entries, captured, db) = prepare_server_state(
        listen,
        routes,
        body_stmts,
        CapturedRuntimeState::new(captured_env, RuntimeTypeRegistry::default()),
        new_db_handle(),
        &mut boot_buf,
        true,
        RuntimeOptions::default(),
    )?;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| RuntimeError::native(format!("test bind failed: {e}")))?;
    let addr = listener
        .local_addr()
        .map_err(|e| RuntimeError::native(format!("local_addr failed: {e}")))?;
    let table = LocalRoutes::new(entries);
    let captured_env = LocalCapturedEnv::new(captured);
    let handle = tokio::task::spawn_local(async move {
        let _ = serve_loop(
            listener,
            table,
            captured_env,
            db,
            None,
            RuntimeOptions::default(),
            shutdown,
        )
        .await;
    });
    Ok((addr, handle, boot_buf))
}

#[allow(clippy::future_not_send)]
pub(in crate::server) async fn spawn_for_test_with_request_trace_file<S>(
    listen: Option<&HirExpr>,
    routes: &[HirExpr],
    body_stmts: &[orv_hir::HirStmt],
    captured_env: HashMap<NameId, Value>,
    request_trace_path: std::path::PathBuf,
    shutdown: S,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>, Vec<u8>), RuntimeError>
where
    S: std::future::Future<Output = ()> + 'static,
{
    let mut boot_buf: Vec<u8> = Vec::new();
    let (port, entries, captured, db) = prepare_server_state(
        listen,
        routes,
        body_stmts,
        CapturedRuntimeState::new(captured_env, RuntimeTypeRegistry::default()),
        new_db_handle(),
        &mut boot_buf,
        true,
        RuntimeOptions::default(),
    )?;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| RuntimeError::native(format!("test bind failed: {e}")))?;
    let addr = listener
        .local_addr()
        .map_err(|e| RuntimeError::native(format!("local_addr failed: {e}")))?;
    let table = LocalRoutes::new(entries);
    let captured_env = LocalCapturedEnv::new(captured);
    let handle = tokio::task::spawn_local(async move {
        let _ = serve_loop_with_request_trace_file(
            listener,
            table,
            captured_env,
            db,
            None,
            Some(request_trace_path),
            RuntimeOptions::default(),
            shutdown,
        )
        .await;
    });
    Ok((addr, handle, boot_buf))
}
