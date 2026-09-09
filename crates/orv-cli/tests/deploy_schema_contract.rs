#[path = "deploy_schema_contract/build.rs"]
mod build;
#[path = "deploy_schema_contract/common.rs"]
mod common;
#[path = "deploy_schema_contract/deploy.rs"]
mod deploy;
#[path = "deploy_schema_contract/fixture.rs"]
mod fixture;

const DEPLOY_PREFLIGHT_GOLDEN: &str =
    include_str!("../../../docs/samples/deploy-preflight-v1.golden.json");
const DEPLOY_BENCHMARK_EVIDENCE_GOLDEN: &str =
    include_str!("../../../docs/samples/deploy-benchmark-evidence-v1.golden.json");

#[test]
fn prod_build_deploy_and_benchmark_json_contracts_freeze_public_shape() {
    let out = fixture::build_prod_contract_fixture();

    build::assert_build_manifest_contract(&common::read_json(&out.join("build-manifest.json")));
    build::assert_source_bundle_contract(&common::read_json(&out.join("source-bundle.json")));
    build::assert_bundle_plan_contract(&common::read_json(&out.join("bundle-plan.json")));
    let deploy = common::read_json(&out.join("deploy").join("manifest.json"));
    deploy::assert_deploy_manifest_contract(&deploy);
    deploy::assert_deploy_routes_contract(
        &common::read_json(&out.join("deploy").join("routes.json")),
        &deploy,
    );
    deploy::assert_deploy_container_contract(
        &common::read_json(&out.join("deploy").join("container.json")),
        &deploy,
    );
    let preflight = common::read_json(&out.join("deploy").join("preflight.json"));
    let preflight_golden: serde_json::Value =
        serde_json::from_str(DEPLOY_PREFLIGHT_GOLDEN).expect("deploy preflight golden");
    assert_eq!(preflight, preflight_golden, "deploy preflight golden drift");
    let evidence = common::read_json(&out.join("deploy").join("benchmark-evidence.json"));
    let evidence_golden: serde_json::Value = serde_json::from_str(DEPLOY_BENCHMARK_EVIDENCE_GOLDEN)
        .expect("deploy benchmark evidence golden");
    assert_eq!(
        evidence, evidence_golden,
        "deploy benchmark evidence golden drift"
    );

    let _ = std::fs::remove_dir_all(&out);
}
