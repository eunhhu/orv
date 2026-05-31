//! Domain-call platform boundary descriptors.

/// Compiler core가 도메인 호출을 어느 boundary로 취급해야 하는지 나타낸다.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainSurface {
    /// 언어/runtime spine이 직접 소유하는 최소 primitive.
    CoreIntrinsic,
    /// repo에 함께 배포될 수 있지만 core intrinsic은 아닌 first-party compiler plugin.
    FirstPartyCompilerPlugin,
    /// first-party library, template, provider package가 소유하는 surface.
    LibraryProviderPackage,
    /// third-party plugin 또는 아직 등록되지 않은 사용자 확장 surface.
    Extension,
}

impl DomainSurface {
    /// Stable artifact/schema spelling for this platform-boundary surface.
    #[must_use]
    pub const fn as_contract_str(self) -> &'static str {
        match self {
            Self::CoreIntrinsic => "core_intrinsic",
            Self::FirstPartyCompilerPlugin => "first_party_compiler_plugin",
            Self::LibraryProviderPackage => "library_provider_package",
            Self::Extension => "extension",
        }
    }

    /// Parse a stable artifact/schema surface string into a domain boundary surface.
    ///
    /// Returns `None` for unknown spellings so drift guards can reject descriptor
    /// artifacts that no longer match the compiler's supported contract values.
    #[must_use]
    pub fn from_contract_str(value: &str) -> Option<Self> {
        match value {
            "core_intrinsic" => Some(Self::CoreIntrinsic),
            "first_party_compiler_plugin" => Some(Self::FirstPartyCompilerPlugin),
            "library_provider_package" => Some(Self::LibraryProviderPackage),
            "extension" => Some(Self::Extension),
            _ => None,
        }
    }

    /// Core intrinsic이면 true.
    #[must_use]
    pub const fn is_core_intrinsic(self) -> bool {
        matches!(self, Self::CoreIntrinsic)
    }

    /// First-party compiler plugin surface이면 true.
    #[must_use]
    pub const fn is_first_party_compiler_plugin(self) -> bool {
        matches!(self, Self::FirstPartyCompilerPlugin)
    }

    /// Library/template/provider package surface이면 true.
    #[must_use]
    pub const fn is_library_provider_package(self) -> bool {
        matches!(self, Self::LibraryProviderPackage)
    }
}

const NO_METADATA: &[&str] = &[];
const CORE_CAPABILITIES: &[&str] = &["core.stdout"];
const CORE_EFFECTS: &[&str] = &["io.write"];
const CORE_HOOKS: &[&str] = &["hir.lower", "origin.emit"];
const WEB_CAPABILITIES: &[&str] = &["http.route", "http.request", "http.response", "html.render"];
const WEB_EFFECTS: &[&str] = &["network.listen", "http.respond"];
const DB_CAPABILITIES: &[&str] = &["db.operation", "db.transaction", "adapter.bridge"];
const DB_EFFECTS: &[&str] = &["storage.read", "storage.write"];
const SECURITY_CAPABILITIES: &[&str] = &["security.policy", "secret.env", "cookie.session"];
const SECURITY_EFFECTS: &[&str] = &["auth.decision", "cookie.issue"];
const DESIGN_CAPABILITIES: &[&str] = &["design.token", "style.artifact"];
const DESIGN_EFFECTS: &[&str] = &["artifact.emit"];
const CRON_CAPABILITIES: &[&str] = &["job.schedule"];
const CRON_EFFECTS: &[&str] = &["time.schedule", "background.run"];
const COMMERCE_CAPABILITIES: &[&str] = &[
    "adapter.bridge",
    "secret.env",
    "idempotency.key",
    "webhook.verify",
];
const COMMERCE_EFFECTS: &[&str] = &["external.call", "secret.read"];
const PLUGIN_HOOKS: &[&str] = &["type.check", "hir.lower", "origin.emit", "artifact.emit"];

/// Domain boundary metadata emitted beside stable artifact/schema surface strings.
///
/// The descriptor keeps current domain classification behavior while adding the
/// package that owns the domain surface plus generic plugin metadata, so later
/// plugin registry work can read ownership and required compiler/runtime
/// affordances without re-encoding string tables.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainBoundaryDescriptor<'a> {
    /// Bare domain name without `@`.
    pub domain: &'a str,
    /// Platform-boundary surface used in stable artifact/schema fields.
    pub surface: DomainSurface,
    /// Package or extension namespace that owns the domain surface.
    pub owner_package: &'static str,
    /// Generic capability labels required by this surface.
    pub capabilities: &'static [&'static str],
    /// Generic side-effect labels exposed by this surface.
    pub effects: &'static [&'static str],
    /// Generic compiler/runtime hook labels used by this surface.
    pub hooks: &'static [&'static str],
}

impl<'a> DomainBoundaryDescriptor<'a> {
    const fn new(
        domain: &'a str,
        surface: DomainSurface,
        owner_package: &'static str,
        capabilities: &'static [&'static str],
        effects: &'static [&'static str],
        hooks: &'static [&'static str],
    ) -> Self {
        Self {
            domain,
            surface,
            owner_package,
            capabilities,
            effects,
            hooks,
        }
    }
}

/// Return full boundary metadata for a bare domain name without `@`.
#[must_use]
pub fn domain_boundary_descriptor(name: &str) -> DomainBoundaryDescriptor<'_> {
    match name {
        "out" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::CoreIntrinsic,
            "orv-core",
            CORE_CAPABILITIES,
            CORE_EFFECTS,
            CORE_HOOKS,
        ),
        "body" | "form" | "header" | "html" | "listen" | "param" | "query" | "request"
        | "respond" | "route" | "serve" | "server" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::FirstPartyCompilerPlugin,
            "orv-web",
            WEB_CAPABILITIES,
            WEB_EFFECTS,
            PLUGIN_HOOKS,
        ),
        "db" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::FirstPartyCompilerPlugin,
            "orv-data",
            DB_CAPABILITIES,
            DB_EFFECTS,
            PLUGIN_HOOKS,
        ),
        "Auth" | "csrf" | "rateLimit" | "session" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::FirstPartyCompilerPlugin,
            "orv-security",
            SECURITY_CAPABILITIES,
            SECURITY_EFFECTS,
            PLUGIN_HOOKS,
        ),
        "design" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::FirstPartyCompilerPlugin,
            "orv-design",
            DESIGN_CAPABILITIES,
            DESIGN_EFFECTS,
            PLUGIN_HOOKS,
        ),
        "cron" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::FirstPartyCompilerPlugin,
            "orv-jobs",
            CRON_CAPABILITIES,
            CRON_EFFECTS,
            PLUGIN_HOOKS,
        ),
        "payment" | "shipping" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::LibraryProviderPackage,
            "orv-commerce",
            COMMERCE_CAPABILITIES,
            COMMERCE_EFFECTS,
            PLUGIN_HOOKS,
        ),
        _ => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::Extension,
            "extension",
            NO_METADATA,
            NO_METADATA,
            NO_METADATA,
        ),
    }
}

/// Return the platform-boundary surface for a bare domain name without `@`.
#[must_use]
pub fn domain_surface(name: &str) -> DomainSurface {
    domain_boundary_descriptor(name).surface
}

/// Parse an origin-map call display like `@db.connect` into `(domain, method)`.
#[must_use]
pub fn origin_call_domain_method(call_name: &str) -> Option<(&str, &str)> {
    let without_at = call_name.strip_prefix('@')?;
    let (domain, method) = without_at.split_once('.')?;
    if domain.is_empty() || method.is_empty() {
        return None;
    }
    Some((domain, method))
}

/// Return the domain surface for an origin-map call display like `@db.connect`.
#[must_use]
pub fn origin_call_surface(call_name: &str) -> Option<DomainSurface> {
    origin_call_boundary_descriptor(call_name).map(|descriptor| descriptor.surface)
}

/// Return full boundary metadata for an origin-map call display like `@db.connect`.
#[must_use]
pub fn origin_call_boundary_descriptor(call_name: &str) -> Option<DomainBoundaryDescriptor<'_>> {
    origin_call_domain_method(call_name).map(|(domain, _)| domain_boundary_descriptor(domain))
}

#[cfg(test)]
mod tests;
