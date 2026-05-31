use serde_json::{json, Value};

const COMPILER_PLUGIN_BOUNDARY_GOLDEN: &str =
    include_str!("../../../docs/samples/compiler-plugin-boundary-v1.golden.json");

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

const ORIGIN_CALLS: [&str; 9] = [
    "@db.connect",
    "@payment.capture",
    "@shipping.book",
    "@server.listen",
    "@custom.run",
    "db.connect",
    "@db",
    "@.connect",
    "@db.",
];

#[test]
fn compiler_plugin_boundary_v1_freezes_domain_descriptor_inventory() {
    let inventory = compiler_plugin_boundary_inventory();
    let golden: Value =
        serde_json::from_str(COMPILER_PLUGIN_BOUNDARY_GOLDEN).expect("boundary golden");

    assert_eq!(
        inventory, golden,
        "Compiler Plugin Boundary v1 golden drift"
    );
    assert_eq!(
        domain_descriptor("payment")["surface"],
        json!("library_provider_package")
    );
    assert_eq!(
        domain_descriptor("payment")["owner_package"],
        json!("orv-commerce")
    );
    assert_eq!(
        domain_descriptor("server")["surface"],
        json!("first_party_compiler_plugin")
    );
    assert_eq!(domain_descriptor("custom")["surface"], json!("extension"));
}

fn compiler_plugin_boundary_inventory() -> Value {
    json!({
        "schema_version": 1,
        "kind": "orv.compiler_plugin_boundary.v1",
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

fn domain_descriptor(domain: &str) -> Value {
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

fn origin_call_descriptor(call: &str) -> Value {
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
