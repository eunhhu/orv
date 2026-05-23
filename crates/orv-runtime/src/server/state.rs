use std::collections::HashMap;
use std::rc::Rc;

use orv_hir::{HirExpr, NameId};

use crate::interp::{RuntimeTypeRegistry, Value};

use super::RateLimitPolicy;

/// `@server` 가 수집한 단일 라우트 — handler HIR 의 스냅샷.
///
/// HIR 은 `Clone` 이므로 서버 기동 시점에 한번 복제해 두고 요청마다 또 한 번
/// clone 해서 handler 평가에 넘긴다. 이중 clone 이 비효율적으로 보이지만 MVP
/// 에서는 라우트 수와 handler 크기가 작고, 이 구조 덕에 Interp 가 HIR 에 대한
/// 참조 수명을 가질 필요가 없어 전체 설계가 단순해진다.
#[derive(Clone)]
pub(super) struct RouteEntry {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) handler: HirExpr,
    pub(super) origin_id: String,
    pub(super) rate_limit: Option<RateLimitPolicy>,
}

/// Route table shared only inside a tokio current-thread server loop.
///
/// `RouteEntry` contains HIR values backed by non-thread-safe runtime data, so
/// this type deliberately uses `Rc` instead of `Arc`. If the server execution
/// model moves to multi-threaded request handling, the compiler will force this
/// boundary to be redesigned instead of silently cloning non-Send state across
/// tasks.
#[derive(Clone)]
pub(super) struct LocalRoutes(Rc<Vec<RouteEntry>>);

impl LocalRoutes {
    pub(super) fn new(routes: Vec<RouteEntry>) -> Self {
        Self(Rc::new(routes))
    }

    pub(super) fn iter(&self) -> std::slice::Iter<'_, RouteEntry> {
        self.0.iter()
    }
}

/// Captured server environment shared only by local request evaluation.
///
/// The values can contain `Rc`-backed runtime data. Keeping the state behind an
/// explicit local wrapper makes the current-thread invariant visible at the
/// function boundary.
#[derive(Clone)]
pub(super) struct CapturedRuntimeState {
    pub(super) env: HashMap<NameId, Value>,
    pub(super) types: RuntimeTypeRegistry,
}

impl CapturedRuntimeState {
    #[allow(clippy::missing_const_for_fn)]
    pub(super) fn new(env: HashMap<NameId, Value>, types: RuntimeTypeRegistry) -> Self {
        Self { env, types }
    }
}

#[derive(Clone)]
pub(super) struct LocalCapturedEnv {
    env: Rc<HashMap<NameId, Value>>,
    types: Rc<RuntimeTypeRegistry>,
}

impl LocalCapturedEnv {
    pub(super) fn new(captured: CapturedRuntimeState) -> Self {
        Self {
            env: Rc::new(captured.env),
            types: Rc::new(captured.types),
        }
    }

    pub(super) fn snapshot(&self) -> HashMap<NameId, Value> {
        self.env.as_ref().clone()
    }

    pub(super) fn type_registry(&self) -> RuntimeTypeRegistry {
        self.types.as_ref().clone()
    }
}
