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
            capabilities: &["core.stdout"],
            effects: &["io.write"],
            hooks: &["hir.lower", "origin.emit"],
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
fn domain_boundary_descriptor_attaches_generic_capability_effect_hook_metadata() {
    let server = domain_boundary_descriptor("server");
    assert_eq!(
        server.capabilities,
        &["http.route", "http.request", "http.response", "html.render",]
    );
    assert_eq!(server.effects, &["network.listen", "http.respond"]);
    assert_eq!(
        server.hooks,
        &["type.check", "hir.lower", "origin.emit", "artifact.emit"]
    );

    let db = domain_boundary_descriptor("db");
    assert_eq!(
        db.capabilities,
        &["db.operation", "db.transaction", "adapter.bridge"]
    );
    assert_eq!(db.effects, &["storage.read", "storage.write"]);

    let payment = domain_boundary_descriptor("payment");
    assert_eq!(
        payment.capabilities,
        &[
            "adapter.bridge",
            "secret.env",
            "idempotency.key",
            "webhook.verify",
        ]
    );
    assert_eq!(payment.effects, &["external.call", "secret.read"]);

    let custom = domain_boundary_descriptor("custom");
    assert!(custom.capabilities.is_empty());
    assert!(custom.effects.is_empty());
    assert!(custom.hooks.is_empty());
}

#[test]
fn domain_boundary_metadata_does_not_leak_provider_names() {
    for domain in [
        "out", "server", "db", "Auth", "design", "cron", "payment", "shipping", "custom",
    ] {
        let descriptor = domain_boundary_descriptor(domain);
        for value in descriptor
            .capabilities
            .iter()
            .chain(descriptor.effects)
            .chain(descriptor.hooks)
        {
            let normalized = value.to_ascii_lowercase();
            assert!(!normalized.contains("stripe"), "{value}");
            assert!(!normalized.contains("carrier"), "{value}");
            assert!(!normalized.contains("shop"), "{value}");
        }
    }
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
            capabilities: &[
                "adapter.bridge",
                "secret.env",
                "idempotency.key",
                "webhook.verify",
            ],
            effects: &["external.call", "secret.read"],
            hooks: &["type.check", "hir.lower", "origin.emit", "artifact.emit"],
        })
    );
    assert_eq!(
        origin_call_boundary_descriptor("@custom.run"),
        Some(DomainBoundaryDescriptor {
            domain: "custom",
            surface: DomainSurface::Extension,
            owner_package: "extension",
            capabilities: &[],
            effects: &[],
            hooks: &[],
        })
    );
    assert_eq!(origin_call_boundary_descriptor("payment.capture"), None);
}
