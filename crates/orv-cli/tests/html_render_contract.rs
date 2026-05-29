use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

const HTML_RENDER_GOLDEN: &str = include_str!("../../../docs/samples/html-render-v1.golden.json");

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("orv-{name}-{}-{nonce}", std::process::id()))
}

const fn orv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_orv")
}

fn run_orv(args: &[&str]) -> Output {
    Command::new(orv_bin())
        .args(args)
        .output()
        .expect("run orv")
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read json")).expect("json")
}

#[test]
fn html_render_v1_freezes_static_build_and_run_build_contract() {
    let root = temp_dir("html-render-contract");
    std::fs::create_dir_all(&root).expect("create temp root");
    let entry = root.join("page.orv");
    std::fs::write(
        &entry,
        r#"@out @html { @body { @h1 "Home" @p "<script>alert(1)</script>&" @a title="<img src=x onerror=\"alert(1)\" data-note='x'>&" "safe" } }"#,
    )
    .expect("write entry");
    let dist = root.join("dist");
    let entry_arg = entry.display().to_string();
    let dist_arg = dist.display().to_string();

    let build = run_orv(&["build", &entry_arg, "--out", &dist_arg]);
    assert_success(&build, "orv build");

    let html =
        std::fs::read_to_string(dist.join("pages").join("index.html")).expect("static page html");
    assert_eq!(
        html,
        "<html><body><h1>Home</h1><p>&lt;script&gt;alert(1)&lt;/script&gt;&amp;</p><a title=\"&lt;img src=x onerror=&quot;alert(1)&quot; data-note=&#39;x&#39;&gt;&amp;\">safe</a></body></html>"
    );

    let plan = read_json(&dist.join("bundle-plan.json"));
    let bundles = plan["bundles"].as_array().expect("bundle array");
    let static_page = bundles
        .iter()
        .find(|bundle| bundle["kind"] == "static_page")
        .expect("static page bundle");
    assert_eq!(static_page["path"], serde_json::json!("pages/index.html"));
    assert_eq!(
        static_page["runtime_features"]
            .as_array()
            .expect("runtime features")
            .len(),
        0
    );
    assert!(
        !bundles
            .iter()
            .any(|bundle| bundle["kind"] == "server_runtime"),
        "zero-runtime static page must not produce a server runtime bundle"
    );

    let run_build = run_orv(&["run-build", &dist_arg]);
    assert_success(&run_build, "orv run-build");
    assert_eq!(String::from_utf8_lossy(&run_build.stdout), html);
    assert!(run_build.stderr.is_empty());
    assert_eq!(
        html_render_inventory(&html, &plan, &run_build),
        html_render_golden(),
        "HTML Render v1 golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

fn html_render_golden() -> Value {
    serde_json::from_str(HTML_RENDER_GOLDEN).expect("HTML render golden")
}

fn html_render_inventory(html: &str, plan: &Value, run_build: &Output) -> Value {
    let bundles = plan["bundles"].as_array().expect("bundle array");
    let static_page = bundles
        .iter()
        .find(|bundle| bundle["kind"] == "static_page")
        .expect("static page bundle");
    serde_json::json!({
        "schema_version": 1,
        "kind": "orv.html_render.inventory",
        "static_html": html,
        "bundle_plan": {
            "static_page": {
                "path": static_page["path"],
                "runtime_feature_count": static_page["runtime_features"]
                    .as_array()
                    .expect("static runtime features")
                    .len(),
            },
            "server_runtime_present": bundles
                .iter()
                .any(|bundle| bundle["kind"] == "server_runtime"),
        },
        "run_build": {
            "exit_success": run_build.status.success(),
            "stdout": String::from_utf8_lossy(&run_build.stdout),
            "stderr_empty": run_build.stderr.is_empty(),
        },
    })
}
