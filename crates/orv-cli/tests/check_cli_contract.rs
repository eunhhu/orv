#[path = "check_cli_contract/support.rs"]
mod support;

use serde_json::Value;

use support::{
    check_cli_entry_diagnostic_inventory, check_cli_golden,
    check_cli_imported_diagnostic_inventory, check_cli_success_inventory, content_hash, run_orv,
    temp_dir, write_source,
};

#[test]
fn check_cli_v1_freezes_success_envelope() {
    let root = temp_dir("check-cli-success");
    let entry = root.join("main.orv");
    write_source(&entry, "let ok: int = 1\n@out ok\n");
    let entry_arg = entry.display().to_string();

    let output = run_orv(&["check", &entry_arg]);

    assert!(
        output.status.success(),
        "orv check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("check: {} passed\n", entry.display())
    );
    assert!(output.stderr.is_empty());
    assert_eq!(
        check_cli_success_inventory(&output, &entry),
        check_cli_golden()["success"],
        "Check CLI v1 success golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn check_cli_v1_routes_imported_file_diagnostics_to_imported_source() {
    let root = temp_dir("check-cli-imported-diagnostic");
    let entry = root.join("main.orv");
    let imported = root.join("models").join("user.orv");
    let entry_lookalike_line = "let bad: string = \"wrong\"";
    write_source(
        &entry,
        &format!("import models.user.User\n{entry_lookalike_line}\n"),
    );
    write_source(
        &imported,
        "pub struct User { id: int }\nlet bad: int = \"wrong\"\n",
    );
    let entry_arg = entry.display().to_string();

    let output = run_orv(&["check", &entry_arg]);

    assert!(
        !output.status.success(),
        "invalid imported source must fail check"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&imported.display().to_string()), "{stderr}");
    assert!(!stderr.contains(&entry.display().to_string()), "{stderr}");
    assert!(stderr.contains("let bad: int = \"wrong\""), "{stderr}");
    assert!(!stderr.contains(entry_lookalike_line), "{stderr}");
    assert!(stderr.contains("value has type `string`"), "{stderr}");
    assert!(
        stderr.contains("error: aborting due to previous errors"),
        "{stderr}"
    );
    assert_eq!(
        check_cli_imported_diagnostic_inventory(&output, &entry, &imported, entry_lookalike_line),
        check_cli_golden()["imported_diagnostic"],
        "Check CLI v1 imported diagnostic golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn check_cli_v1_routes_entry_file_diagnostics_to_entry_source() {
    let root = temp_dir("check-cli-entry-diagnostic");
    let entry = root.join("main.orv");
    let entry_bad_line = "let bad: int = \"wrong\"";
    write_source(&entry, &format!("{entry_bad_line}\n"));
    let entry_arg = entry.display().to_string();

    let output = run_orv(&["check", &entry_arg]);

    assert!(
        !output.status.success(),
        "invalid entry source must fail check"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&entry.display().to_string()), "{stderr}");
    assert!(stderr.contains(entry_bad_line), "{stderr}");
    assert!(stderr.contains("value has type `string`"), "{stderr}");
    assert!(
        stderr.contains("error: aborting due to previous errors"),
        "{stderr}"
    );
    assert_eq!(
        check_cli_entry_diagnostic_inventory(&output, &entry, entry_bad_line),
        check_cli_golden()["entry_diagnostic"],
        "Check CLI v1 entry diagnostic golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn check_artifact_routes_imported_bundle_diagnostics_to_imported_source() {
    let root = temp_dir("check-artifact-imported-diagnostic");
    let entry = root.join("main.orv");
    let imported = root.join("models").join("user.orv");
    let out = root.join("dist");
    let entry_lookalike_line = "let bad: string = \"wrong\"";
    write_source(
        &entry,
        &format!(
            r"import models.user.User
{entry_lookalike_line}
@server {{
  @listen 8080
  @route GET / {{
    @respond 200 {{ ok: true }}
  }}
}}
"
        ),
    );
    write_source(&imported, "pub struct User { id: int }\nlet ok: int = 1\n");
    let entry_arg = entry.display().to_string();
    let out_arg = out.display().to_string();

    let build = run_orv(&["build", &entry_arg, "--prod", "--out", &out_arg]);
    assert!(
        build.status.success(),
        "orv build failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let artifact_path = out.join("server").join("app.orv-runtime.json");
    let mut artifact: Value =
        serde_json::from_str(&std::fs::read_to_string(&artifact_path).expect("read artifact"))
            .expect("parse artifact");
    let imported_source = "pub struct User { id: int }\nlet bad: int = \"wrong\"\n";
    let file = artifact["source_bundle"]["files"]
        .as_array_mut()
        .expect("artifact source bundle files")
        .iter_mut()
        .find(|file| {
            file["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("models/user.orv"))
        })
        .expect("imported source bundle file");
    file["source"] = serde_json::json!(imported_source);
    file["content_hash"] = serde_json::json!(content_hash(imported_source));
    std::fs::write(
        &artifact_path,
        serde_json::to_string_pretty(&artifact).expect("serialize artifact"),
    )
    .expect("write artifact");
    let artifact_arg = artifact_path.display().to_string();

    let output = run_orv(&["check-artifact", &artifact_arg]);

    assert!(
        !output.status.success(),
        "invalid imported artifact source must fail check-artifact"
    );
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&imported.display().to_string()), "{stderr}");
    assert!(!stderr.contains(&entry.display().to_string()), "{stderr}");
    assert!(stderr.contains("let bad: int = \"wrong\""), "{stderr}");
    assert!(stderr.contains("value has type `string`"), "{stderr}");
    assert!(!stderr.contains(entry_lookalike_line), "{stderr}");
    assert_eq!(
        check_cli_imported_diagnostic_inventory(&output, &entry, &imported, entry_lookalike_line),
        check_cli_golden()["check_artifact_imported_diagnostic"],
        "Check CLI v1 check-artifact imported diagnostic golden drift"
    );

    let _ = std::fs::remove_dir_all(root);
}
