//! Domain-call platform boundary descriptors.

use crate::domain_registry::domain_plugin_registration;

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
    if let Some(registration) = domain_plugin_registration(name) {
        return DomainBoundaryDescriptor::new(
            name,
            registration.surface,
            registration.owner_package,
            registration.capabilities,
            registration.effects,
            registration.hooks,
        );
    }
    DomainBoundaryDescriptor::new(
        name,
        DomainSurface::Extension,
        "extension",
        NO_METADATA,
        NO_METADATA,
        NO_METADATA,
    )
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
