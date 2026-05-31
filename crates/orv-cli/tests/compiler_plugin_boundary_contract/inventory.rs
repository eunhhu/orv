use serde_json::{json, Value};

const BARE_DOMAINS: [&str; 14] = [
    "out",
    "server",
    "route",
    "html",
    "db",
    "Auth",
    "session",
    "csrf",
    "rateLimit",
    "design",
    "cron",
    "payment",
    "shipping",
    "custom",
];

const ORIGIN_CALLS: [&str; 13] = [
    "@db.connect",
    "@payment.capture",
    "@payment.connect",
    "@shipping.book",
    "@shipping.connect",
    "@Stripe.connect",
    "@carrier.connect",
    "@server.listen",
    "@custom.run",
    "db.connect",
    "@db",
    "@.connect",
    "@db.",
];

pub fn compiler_plugin_boundary_inventory() -> Value {
    json!({
        "schema_version": 1,
        "kind": "orv.compiler_plugin_boundary.v1",
        "plugin_registry": orv_hir::domain_plugin_registry()
            .iter()
            .map(plugin_registration)
            .collect::<Vec<_>>(),
        "domain_descriptors": BARE_DOMAINS
            .iter()
            .map(|domain| domain_descriptor(domain))
            .collect::<Vec<_>>(),
        "origin_call_descriptors": ORIGIN_CALLS
            .iter()
            .map(|call| origin_call_descriptor(call))
            .collect::<Vec<_>>(),
    })
}

fn plugin_registration(registration: &orv_hir::DomainPluginRegistration) -> Value {
    json!({
        "surface": registration.surface.as_contract_str(),
        "owner_package": registration.owner_package,
        "domains": registration.domains,
        "capabilities": registration.capabilities,
        "effects": registration.effects,
        "hooks": registration.hooks,
    })
}

pub fn domain_descriptor(domain: &str) -> Value {
    let descriptor = orv_hir::domain_boundary_descriptor(domain);
    json!({
        "domain": descriptor.domain,
        "surface": descriptor.surface.as_contract_str(),
        "owner_package": descriptor.owner_package,
        "capabilities": descriptor.capabilities,
        "effects": descriptor.effects,
        "hooks": descriptor.hooks,
    })
}

pub fn origin_call_descriptor(call: &str) -> Value {
    match (
        orv_hir::origin_call_domain_method(call),
        orv_hir::origin_call_boundary_descriptor(call),
    ) {
        (Some((_, method)), Some(descriptor)) => json!({
            "call": call,
            "domain": descriptor.domain,
            "method": method,
            "surface": descriptor.surface.as_contract_str(),
            "owner_package": descriptor.owner_package,
            "capabilities": descriptor.capabilities,
            "effects": descriptor.effects,
            "hooks": descriptor.hooks,
        }),
        _ => json!({
            "call": call,
            "domain": null,
            "method": null,
            "surface": null,
            "owner_package": null,
            "capabilities": null,
            "effects": null,
            "hooks": null,
        }),
    }
}
