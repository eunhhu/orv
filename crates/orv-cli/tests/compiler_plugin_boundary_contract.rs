use serde_json::{json, Value};

#[path = "compiler_plugin_boundary_contract/drift_guard.rs"]
mod drift_guard;
#[path = "compiler_plugin_boundary_contract/inventory.rs"]
mod inventory;

use drift_guard::{
    assert_inventory_rejection_contains, domain_descriptor_index, origin_call_descriptor_index,
    validate_compiler_plugin_boundary_inventory,
};
use inventory::{compiler_plugin_boundary_inventory, domain_descriptor, origin_call_descriptor};

const COMPILER_PLUGIN_BOUNDARY_GOLDEN: &str =
    include_str!("../../../docs/samples/compiler-plugin-boundary-v1.golden.json");

#[test]
fn compiler_plugin_boundary_v1_freezes_domain_descriptor_inventory() {
    let inventory = compiler_plugin_boundary_inventory();
    let golden: Value =
        serde_json::from_str(COMPILER_PLUGIN_BOUNDARY_GOLDEN).expect("boundary golden");

    assert_eq!(
        inventory, golden,
        "Compiler Plugin Boundary v1 golden drift"
    );
    let registry = inventory["plugin_registry"]
        .as_array()
        .expect("plugin registry");
    assert_eq!(registry.len(), 7);
    assert_eq!(registry[1]["owner_package"], json!("orv-web"));
    assert_eq!(registry[1]["surface"], json!("first_party_compiler_plugin"));
    assert_eq!(registry[6]["surface"], json!("library_provider_package"));
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
        origin_call_descriptor("@Stripe.capture")["surface"],
        json!("extension")
    );
    assert_eq!(
        origin_call_descriptor("@carrier.connect")["surface"],
        json!("extension")
    );
    assert_eq!(
        origin_call_descriptor("@carrier.book")["surface"],
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

    let mut missing_registry = baseline.clone();
    missing_registry
        .as_object_mut()
        .expect("inventory object")
        .remove("plugin_registry");
    assert_inventory_rejection_contains(&missing_registry, "plugin_registry missing");

    let mut wrong_schema_version = baseline.clone();
    wrong_schema_version["schema_version"] = json!(2);
    assert_inventory_rejection_contains(&wrong_schema_version, "schema_version expected 1, got 2");

    let mut wrong_kind = baseline.clone();
    wrong_kind["kind"] = json!("orv.compiler_plugin_boundary.v2");
    assert_inventory_rejection_contains(
        &wrong_kind,
        "kind expected `orv.compiler_plugin_boundary.v1`, got `orv.compiler_plugin_boundary.v2`",
    );

    let mut extra_root_key = baseline.clone();
    extra_root_key["provider_surface"] = json!("first_party_compiler_plugin");
    assert_inventory_rejection_contains(&extra_root_key, "provider_surface unexpected root key");

    let mut missing_origin_calls = baseline.clone();
    missing_origin_calls
        .as_object_mut()
        .expect("inventory object")
        .remove("origin_call_descriptors");
    assert_inventory_rejection_contains(&missing_origin_calls, "origin_call_descriptors missing");

    let mut duplicate_registry_domain = baseline.clone();
    duplicate_registry_domain["plugin_registry"][6]["domains"]
        .as_array_mut()
        .expect("commerce registry domains")
        .push(json!("server"));
    assert_inventory_rejection_contains(
        &duplicate_registry_domain,
        "plugin_registry[6].domains[2] duplicate domain `server`",
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

    let provider_call_index =
        origin_call_descriptor_index(&compiler_plugin_boundary_inventory(), "@Stripe.connect");
    let mut provider_call_surface_drift = compiler_plugin_boundary_inventory();
    provider_call_surface_drift["origin_call_descriptors"][provider_call_index]["surface"] =
        json!("first_party_compiler_plugin");
    assert_inventory_rejection_contains(
        &provider_call_surface_drift,
        &format!(
            "origin_call_descriptors[{provider_call_index}].surface expected `extension`, got `first_party_compiler_plugin`"
        ),
    );

    let mut provider_call_owner_drift = compiler_plugin_boundary_inventory();
    provider_call_owner_drift["origin_call_descriptors"][provider_call_index]["owner_package"] =
        json!("orv-commerce");
    assert_inventory_rejection_contains(
        &provider_call_owner_drift,
        &format!(
            "origin_call_descriptors[{provider_call_index}].owner_package expected `extension`, got `orv-commerce`"
        ),
    );
}
