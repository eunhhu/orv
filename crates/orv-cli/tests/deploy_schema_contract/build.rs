use crate::common::assert_keys;

pub(crate) fn assert_build_manifest_contract(build_manifest: &serde_json::Value) {
    assert_keys(
        build_manifest,
        &[
            "schema_version",
            "entry",
            "runtime",
            "artifacts",
            "capabilities",
        ],
        "build manifest",
    );
    assert_eq!(build_manifest["schema_version"], serde_json::json!(1));
    assert!(build_manifest["artifacts"].is_array());
    assert!(build_manifest["capabilities"].is_object());
}

pub(crate) fn assert_source_bundle_contract(source_bundle: &serde_json::Value) {
    assert_keys(
        source_bundle,
        &["schema_version", "entry", "files"],
        "source bundle",
    );
    assert_eq!(source_bundle["schema_version"], serde_json::json!(1));
    assert!(source_bundle["files"].is_array());
}

pub(crate) fn assert_bundle_plan_contract(bundle_plan: &serde_json::Value) {
    assert_keys(bundle_plan, &["schema_version", "bundles"], "bundle plan");
    assert_eq!(bundle_plan["schema_version"], serde_json::json!(1));
    assert!(bundle_plan["bundles"].is_array());
}
