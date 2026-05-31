use std::collections::{BTreeSet, HashMap, VecDeque};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use serde_json::Value;

const TEST_ONLY_PIPELINE_CRATES: [&str; 3] = ["orv-analyzer", "orv-resolve", "orv-syntax"];
const FORBIDDEN_NORMAL_CRATES: [&str; 7] = [
    "orv-runtime",
    "orv-core",
    "orv-project",
    "orv-syntax",
    "orv-resolve",
    "orv-analyzer",
    "wgpu",
];

static CARGO_METADATA: OnceLock<Value> = OnceLock::new();

#[test]
fn orv_compiler_direct_normal_dependencies_stay_minimal() {
    // Given: the compiler crate is a pure artifact builder over HIR.
    let metadata = cargo_metadata();

    // When: reading direct normal dependency edges from Cargo metadata.
    let normal_deps = direct_dependency_names(&metadata, DependencyKind::Normal);

    // Then: runtime, project loading, syntax, resolve, and analyzer stay out.
    assert_eq!(normal_deps, names(["orv-diagnostics", "orv-hir", "serde"]));
}

#[test]
fn orv_compiler_pipeline_crates_stay_test_only() {
    // Given: pipeline tests need parser, resolver, and analyzer crates.
    let metadata = cargo_metadata();

    // When: reading direct dev-dependency edges from Cargo metadata.
    let dev_deps = direct_dependency_names(&metadata, DependencyKind::Dev);

    // Then: pipeline crates remain test-only instead of normal graph inputs.
    for crate_name in TEST_ONLY_PIPELINE_CRATES {
        assert!(
            dev_deps.contains(crate_name),
            "{crate_name} should remain an orv-compiler dev-dependency"
        );
    }
}

#[test]
fn orv_compiler_normal_closure_excludes_runtime_and_heavy_provider_crates() {
    // Given: CLI and downstream build paths should not pay runtime graphics deps
    // just because they depend on orv-compiler.
    let metadata = cargo_metadata();

    // When: walking only normal dependency edges from orv-compiler.
    let normal_closure = normal_dependency_closure_names(&metadata);

    // Then: runtime, pipeline-only crates, project loading, and wgpu stay out.
    for crate_name in FORBIDDEN_NORMAL_CRATES {
        assert!(
            !normal_closure.contains(crate_name),
            "{crate_name} leaked into orv-compiler normal dependency closure"
        );
    }
}

#[derive(Clone, Copy)]
enum DependencyKind {
    Normal,
    Dev,
}

fn cargo_metadata() -> &'static Value {
    CARGO_METADATA.get_or_init(load_cargo_metadata)
}

fn load_cargo_metadata() -> Value {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata should run");

    assert!(
        output.status.success(),
        "cargo metadata failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("cargo metadata should be valid JSON")
}

fn direct_dependency_names(metadata: &Value, kind: DependencyKind) -> BTreeSet<String> {
    let package_names = package_names_by_id(metadata);
    let compiler_id = compiler_package_id(metadata);
    let compiler_node = resolve_node(metadata, &compiler_id);

    compiler_node["deps"]
        .as_array()
        .expect("resolve node deps should be an array")
        .iter()
        .filter(|dep| dep_has_kind(dep, kind))
        .map(|dep| {
            let package_id = dep["pkg"].as_str().expect("dep pkg should be a package id");
            package_names
                .get(package_id)
                .expect("dep package id should have package metadata")
                .clone()
        })
        .collect()
}

fn normal_dependency_closure_names(metadata: &Value) -> BTreeSet<String> {
    let package_names = package_names_by_id(metadata);
    let compiler_id = compiler_package_id(metadata);
    let nodes = resolve_nodes_by_id(metadata);
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::from([compiler_id.clone()]);

    while let Some(package_id) = queue.pop_front() {
        let Some(node) = nodes.get(&package_id) else {
            continue;
        };

        for dep in node["deps"]
            .as_array()
            .expect("resolve node deps should be an array")
        {
            if !dep_has_kind(dep, DependencyKind::Normal) {
                continue;
            }

            let dep_id = dep["pkg"]
                .as_str()
                .expect("dep pkg should be a package id")
                .to_owned();
            if visited.insert(dep_id.clone()) {
                queue.push_back(dep_id);
            }
        }
    }

    visited.remove(&compiler_id);
    visited
        .into_iter()
        .map(|package_id| {
            package_names
                .get(&package_id)
                .expect("closure package id should have package metadata")
                .clone()
        })
        .collect()
}

fn dep_has_kind(dep: &Value, kind: DependencyKind) -> bool {
    dep["dep_kinds"]
        .as_array()
        .expect("dep_kinds should be an array")
        .iter()
        .any(|dep_kind| match kind {
            DependencyKind::Normal => dep_kind.get("kind").is_some_and(Value::is_null),
            DependencyKind::Dev => dep_kind.get("kind").and_then(Value::as_str) == Some("dev"),
        })
}

fn compiler_package_id(metadata: &Value) -> String {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("Cargo.toml")
        .canonicalize()
        .expect("compiler manifest path should canonicalize");

    metadata["packages"]
        .as_array()
        .expect("packages should be an array")
        .iter()
        .find(|package| {
            package["name"].as_str() == Some("orv-compiler")
                && package["manifest_path"]
                    .as_str()
                    .and_then(|path| Path::new(path).canonicalize().ok())
                    .as_ref()
                    == Some(&manifest_path)
        })
        .and_then(|package| package["id"].as_str())
        .expect("orv-compiler package id should exist")
        .to_owned()
}

fn package_names_by_id(metadata: &Value) -> HashMap<String, String> {
    metadata["packages"]
        .as_array()
        .expect("packages should be an array")
        .iter()
        .map(|package| {
            (
                package["id"]
                    .as_str()
                    .expect("package id should be a string")
                    .to_owned(),
                package["name"]
                    .as_str()
                    .expect("package name should be a string")
                    .to_owned(),
            )
        })
        .collect()
}

fn resolve_nodes_by_id(metadata: &Value) -> HashMap<String, &Value> {
    metadata["resolve"]["nodes"]
        .as_array()
        .expect("resolve nodes should be an array")
        .iter()
        .map(|node| {
            (
                node["id"]
                    .as_str()
                    .expect("resolve node id should be a string")
                    .to_owned(),
                node,
            )
        })
        .collect()
}

fn resolve_node<'a>(metadata: &'a Value, package_id: &str) -> &'a Value {
    metadata["resolve"]["nodes"]
        .as_array()
        .expect("resolve nodes should be an array")
        .iter()
        .find(|node| node["id"].as_str() == Some(package_id))
        .expect("package should have a resolve node")
}

fn names<const N: usize>(items: [&str; N]) -> BTreeSet<String> {
    items.into_iter().map(str::to_owned).collect()
}
