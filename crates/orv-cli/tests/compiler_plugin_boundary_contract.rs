use serde_json::{json, Value};

#[path = "compiler_plugin_boundary_contract/drift_guard.rs"]
mod drift_guard;

use drift_guard::{
    assert_inventory_rejection_contains, domain_descriptor_index,
    validate_compiler_plugin_boundary_inventory,
};

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
    assert_eq!(
        origin_call_descriptor("@payment.connect")["surface"],
        json!("library_provider_package")
    );
    assert_eq!(
        origin_call_descriptor("@shipping.connect")["surface"],
        json!("library_provider_package")
    );
    assert_eq!(
        origin_call_descriptor("@Stripe.connect")["surface"],
        json!("extension")
    );
    assert_eq!(
        origin_call_descriptor("@carrier.connect")["surface"],
        json!("extension")
    );
    assert!(
        validate_compiler_plugin_boundary_inventory(&inventory).is_empty(),
        "inventory should satisfy local drift guard"
    );
}

#[test]
fn compiler_plugin_boundary_v1_rejects_descriptor_drift_shapes() {
    let baseline = compiler_plugin_boundary_inventory();
    assert!(
        validate_compiler_plugin_boundary_inventory(&baseline).is_empty(),
        "baseline inventory should pass local drift guard"
    );

    let plugin_index = domain_descriptor_index(&baseline, "server");

    let mut missing_surface = baseline.clone();
    missing_surface["domain_descriptors"][plugin_index]
        .as_object_mut()
        .expect("domain descriptor object")
        .remove("surface");
    assert_inventory_rejection_contains(
        &missing_surface,
        &format!("domain_descriptors[{plugin_index}].surface missing"),
    );

    let mut unknown_surface = baseline.clone();
    unknown_surface["domain_descriptors"][plugin_index]["surface"] = json!("first_party_plugin");
    assert_inventory_rejection_contains(
        &unknown_surface,
        &format!("domain_descriptors[{plugin_index}].surface unknown `first_party_plugin`"),
    );

    let mut missing_owner_package = baseline.clone();
    missing_owner_package["domain_descriptors"][plugin_index]
        .as_object_mut()
        .expect("domain descriptor object")
        .remove("owner_package");
    assert_inventory_rejection_contains(
        &missing_owner_package,
        &format!("domain_descriptors[{plugin_index}].owner_package missing"),
    );

    let mut empty_capabilities = baseline.clone();
    empty_capabilities["domain_descriptors"][plugin_index]["capabilities"] = json!([]);
    assert_inventory_rejection_contains(
        &empty_capabilities,
        &format!("domain_descriptors[{plugin_index}].capabilities empty"),
    );

    let mut unknown_metadata = baseline;
    unknown_metadata["domain_descriptors"][plugin_index]["capabilities"] =
        json!(["http.route", "capability.unknown"]);
    unknown_metadata["domain_descriptors"][plugin_index]["effects"] = json!(["effect.unknown"]);
    unknown_metadata["domain_descriptors"][plugin_index]["hooks"] = json!(["hook.unknown"]);
    assert_inventory_rejection_contains(
        &unknown_metadata,
        &format!("domain_descriptors[{plugin_index}].capabilities[1] unknown `capability.unknown`"),
    );
    assert_inventory_rejection_contains(
        &unknown_metadata,
        &format!("domain_descriptors[{plugin_index}].effects[0] unknown `effect.unknown`"),
    );
    assert_inventory_rejection_contains(
        &unknown_metadata,
        &format!("domain_descriptors[{plugin_index}].hooks[0] unknown `hook.unknown`"),
    );
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
