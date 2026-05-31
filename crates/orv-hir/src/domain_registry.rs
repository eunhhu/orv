use crate::domain_boundary::DomainSurface;

const CORE_CAPABILITIES: &[&str] = &["core.stdout"];
const CORE_EFFECTS: &[&str] = &["io.write"];
const CORE_HOOKS: &[&str] = &["hir.lower", "origin.emit"];
const CORE_DOMAINS: &[&str] = &["out"];
const WEB_CAPABILITIES: &[&str] = &["http.route", "http.request", "http.response", "html.render"];
const WEB_EFFECTS: &[&str] = &["network.listen", "http.respond"];
const WEB_DOMAINS: &[&str] = &[
    "body", "form", "header", "html", "listen", "param", "query", "request", "respond", "route",
    "serve", "server",
];
const DB_CAPABILITIES: &[&str] = &["db.operation", "db.transaction", "adapter.bridge"];
const DB_EFFECTS: &[&str] = &["storage.read", "storage.write"];
const DB_DOMAINS: &[&str] = &["db"];
const SECURITY_CAPABILITIES: &[&str] = &["security.policy", "secret.env", "cookie.session"];
const SECURITY_EFFECTS: &[&str] = &["auth.decision", "cookie.issue"];
const SECURITY_DOMAINS: &[&str] = &["Auth", "csrf", "rateLimit", "session"];
const DESIGN_CAPABILITIES: &[&str] = &["design.token", "style.artifact"];
const DESIGN_EFFECTS: &[&str] = &["artifact.emit"];
const DESIGN_DOMAINS: &[&str] = &["design"];
const CRON_CAPABILITIES: &[&str] = &["job.schedule"];
const CRON_EFFECTS: &[&str] = &["time.schedule", "background.run"];
const CRON_DOMAINS: &[&str] = &["cron"];
const COMMERCE_CAPABILITIES: &[&str] = &[
    "adapter.bridge",
    "secret.env",
    "idempotency.key",
    "webhook.verify",
];
const COMMERCE_EFFECTS: &[&str] = &["external.call", "secret.read"];
const COMMERCE_DOMAINS: &[&str] = &["payment", "shipping"];
const PLUGIN_HOOKS: &[&str] = &["type.check", "hir.lower", "origin.emit", "artifact.emit"];

const DOMAIN_PLUGIN_REGISTRY: &[DomainPluginRegistration] = &[
    DomainPluginRegistration::new(
        DomainSurface::CoreIntrinsic,
        "orv-core",
        CORE_DOMAINS,
        CORE_CAPABILITIES,
        CORE_EFFECTS,
        CORE_HOOKS,
    ),
    DomainPluginRegistration::new(
        DomainSurface::FirstPartyCompilerPlugin,
        "orv-web",
        WEB_DOMAINS,
        WEB_CAPABILITIES,
        WEB_EFFECTS,
        PLUGIN_HOOKS,
    ),
    DomainPluginRegistration::new(
        DomainSurface::FirstPartyCompilerPlugin,
        "orv-data",
        DB_DOMAINS,
        DB_CAPABILITIES,
        DB_EFFECTS,
        PLUGIN_HOOKS,
    ),
    DomainPluginRegistration::new(
        DomainSurface::FirstPartyCompilerPlugin,
        "orv-security",
        SECURITY_DOMAINS,
        SECURITY_CAPABILITIES,
        SECURITY_EFFECTS,
        PLUGIN_HOOKS,
    ),
    DomainPluginRegistration::new(
        DomainSurface::FirstPartyCompilerPlugin,
        "orv-design",
        DESIGN_DOMAINS,
        DESIGN_CAPABILITIES,
        DESIGN_EFFECTS,
        PLUGIN_HOOKS,
    ),
    DomainPluginRegistration::new(
        DomainSurface::FirstPartyCompilerPlugin,
        "orv-jobs",
        CRON_DOMAINS,
        CRON_CAPABILITIES,
        CRON_EFFECTS,
        PLUGIN_HOOKS,
    ),
    DomainPluginRegistration::new(
        DomainSurface::LibraryProviderPackage,
        "orv-commerce",
        COMMERCE_DOMAINS,
        COMMERCE_CAPABILITIES,
        COMMERCE_EFFECTS,
        PLUGIN_HOOKS,
    ),
];

/// Static registry entry for a current compiler/plugin/library boundary owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainPluginRegistration {
    /// Boundary surface owned by this registration.
    pub surface: DomainSurface,
    /// Package or namespace that owns these domains.
    pub owner_package: &'static str,
    /// Bare domain names registered to this owner.
    pub domains: &'static [&'static str],
    /// Generic capability labels required by this owner.
    pub capabilities: &'static [&'static str],
    /// Generic side-effect labels exposed by this owner.
    pub effects: &'static [&'static str],
    /// Generic compiler/runtime hook labels used by this owner.
    pub hooks: &'static [&'static str],
}

impl DomainPluginRegistration {
    const fn new(
        surface: DomainSurface,
        owner_package: &'static str,
        domains: &'static [&'static str],
        capabilities: &'static [&'static str],
        effects: &'static [&'static str],
        hooks: &'static [&'static str],
    ) -> Self {
        Self {
            surface,
            owner_package,
            domains,
            capabilities,
            effects,
            hooks,
        }
    }
}

/// Return the static domain boundary registry scaffold.
#[must_use]
pub const fn domain_plugin_registry() -> &'static [DomainPluginRegistration] {
    DOMAIN_PLUGIN_REGISTRY
}

/// Return the registry entry that owns a bare domain name, if it is registered.
#[must_use]
pub fn domain_plugin_registration(domain: &str) -> Option<&'static DomainPluginRegistration> {
    domain_plugin_registry()
        .iter()
        .find(|registration| registration.domains.contains(&domain))
}
