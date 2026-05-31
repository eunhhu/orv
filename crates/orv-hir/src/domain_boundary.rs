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

/// Domain boundary metadata emitted beside stable artifact/schema surface strings.
///
/// The descriptor keeps current domain classification behavior while adding the
/// package that owns the domain surface, so later plugin registry work can read
/// ownership without re-encoding string tables.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainBoundaryDescriptor<'a> {
    /// Bare domain name without `@`.
    pub domain: &'a str,
    /// Platform-boundary surface used in stable artifact/schema fields.
    pub surface: DomainSurface,
    /// Package or extension namespace that owns the domain surface.
    pub owner_package: &'static str,
}

impl<'a> DomainBoundaryDescriptor<'a> {
    const fn new(domain: &'a str, surface: DomainSurface, owner_package: &'static str) -> Self {
        Self {
            domain,
            surface,
            owner_package,
        }
    }
}

/// Return full boundary metadata for a bare domain name without `@`.
#[must_use]
pub fn domain_boundary_descriptor(name: &str) -> DomainBoundaryDescriptor<'_> {
    match name {
        "out" => DomainBoundaryDescriptor::new(name, DomainSurface::CoreIntrinsic, "orv-core"),
        "body" | "form" | "header" | "html" | "listen" | "param" | "query" | "request"
        | "respond" | "route" | "serve" | "server" => {
            DomainBoundaryDescriptor::new(name, DomainSurface::FirstPartyCompilerPlugin, "orv-web")
        }
        "db" => {
            DomainBoundaryDescriptor::new(name, DomainSurface::FirstPartyCompilerPlugin, "orv-data")
        }
        "Auth" | "csrf" | "rateLimit" | "session" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::FirstPartyCompilerPlugin,
            "orv-security",
        ),
        "design" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::FirstPartyCompilerPlugin,
            "orv-design",
        ),
        "cron" => {
            DomainBoundaryDescriptor::new(name, DomainSurface::FirstPartyCompilerPlugin, "orv-jobs")
        }
        "payment" | "shipping" => DomainBoundaryDescriptor::new(
            name,
            DomainSurface::LibraryProviderPackage,
            "orv-commerce",
        ),
        _ => DomainBoundaryDescriptor::new(name, DomainSurface::Extension, "extension"),
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
mod tests {
    use super::{
        domain_boundary_descriptor, domain_surface, origin_call_boundary_descriptor,
        origin_call_domain_method, origin_call_surface, DomainBoundaryDescriptor, DomainSurface,
    };

    #[test]
    fn domain_surface_separates_core_plugin_and_library_provider_boundaries() {
        assert_eq!(domain_surface("out"), DomainSurface::CoreIntrinsic);
        assert_eq!(
            domain_surface("server"),
            DomainSurface::FirstPartyCompilerPlugin
        );
        assert_eq!(
            domain_surface("db"),
            DomainSurface::FirstPartyCompilerPlugin
        );
        assert_eq!(
            domain_surface("payment"),
            DomainSurface::LibraryProviderPackage
        );
        assert_eq!(
            domain_surface("shipping"),
            DomainSurface::LibraryProviderPackage
        );
        assert_eq!(domain_surface("custom"), DomainSurface::Extension);
        assert_eq!(
            domain_surface("payment").as_contract_str(),
            "library_provider_package"
        );
    }

    #[test]
    fn domain_boundary_descriptor_attaches_owner_packages() {
        assert_eq!(
            domain_boundary_descriptor("out"),
            DomainBoundaryDescriptor {
                domain: "out",
                surface: DomainSurface::CoreIntrinsic,
                owner_package: "orv-core",
            }
        );
        assert_eq!(
            domain_boundary_descriptor("server").owner_package,
            "orv-web"
        );
        assert_eq!(domain_boundary_descriptor("db").owner_package, "orv-data");
        assert_eq!(
            domain_boundary_descriptor("Auth").owner_package,
            "orv-security"
        );
        assert_eq!(
            domain_boundary_descriptor("design").owner_package,
            "orv-design"
        );
        assert_eq!(domain_boundary_descriptor("cron").owner_package, "orv-jobs");
        assert_eq!(
            domain_boundary_descriptor("payment").owner_package,
            "orv-commerce"
        );
        assert_eq!(
            domain_boundary_descriptor("custom").owner_package,
            "extension"
        );
    }

    #[test]
    fn origin_call_domain_method_parses_domain_method_display_names() {
        assert_eq!(
            origin_call_domain_method("@db.connect"),
            Some(("db", "connect"))
        );
        assert_eq!(
            origin_call_domain_method("@payment.capture"),
            Some(("payment", "capture"))
        );
        assert_eq!(origin_call_domain_method("db.connect"), None);
        assert_eq!(origin_call_domain_method("@db"), None);
        assert_eq!(origin_call_domain_method("@.connect"), None);
        assert_eq!(origin_call_domain_method("@db."), None);
    }

    #[test]
    fn origin_call_surface_reuses_bare_domain_classification() {
        assert_eq!(
            origin_call_surface("@db.connect"),
            Some(DomainSurface::FirstPartyCompilerPlugin)
        );
        assert_eq!(
            origin_call_surface("@payment.connect"),
            Some(DomainSurface::LibraryProviderPackage)
        );
        assert_eq!(
            origin_call_surface("@custom.run"),
            Some(DomainSurface::Extension)
        );
    }

    #[test]
    fn origin_call_boundary_descriptor_reuses_bare_domain_descriptor() {
        assert_eq!(
            origin_call_boundary_descriptor("@payment.capture"),
            Some(DomainBoundaryDescriptor {
                domain: "payment",
                surface: DomainSurface::LibraryProviderPackage,
                owner_package: "orv-commerce",
            })
        );
        assert_eq!(
            origin_call_boundary_descriptor("@custom.run"),
            Some(DomainBoundaryDescriptor {
                domain: "custom",
                surface: DomainSurface::Extension,
                owner_package: "extension",
            })
        );
        assert_eq!(origin_call_boundary_descriptor("payment.capture"), None);
    }
}
