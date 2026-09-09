use super::*;

#[test]
fn verify_build_rejects_deploy_smoke_test_path_mismatch() {
    let (src_dir, path) = prod_server_source("deploy-smoke-path-source");
    let out = temp_output_dir("deploy-smoke-path-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let canonical_smoke_path = out.join("deploy").join("smoke-test.sh");
    let wrong_smoke_path = out.join("deploy").join("alternate-smoke.sh");
    std::fs::copy(&canonical_smoke_path, &wrong_smoke_path).expect("copy smoke test");
    let deploy_manifest_path = out.join("deploy").join("manifest.json");
    let mut deploy = read_json_value(&deploy_manifest_path).expect("deploy manifest");
    deploy["server"]["smoke_test"] = serde_json::json!("deploy/alternate-smoke.sh");
    write_json(&deploy_manifest_path, &deploy).expect("write corrupt deploy manifest");
    let runbook_path = out.join("deploy").join("README.md");
    let runbook = std::fs::read_to_string(&runbook_path).expect("deploy runbook");
    std::fs::write(
        &runbook_path,
        runbook.replace("deploy/smoke-test.sh", "deploy/alternate-smoke.sh"),
    )
    .expect("write corrupt deploy runbook");

    let err = cmd_verify_build(&out).expect_err("smoke test path mismatch");

    assert!(err
        .to_string()
        .contains("deploy server smoke_test must be deploy/smoke-test.sh"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_route_reveal_summary_count_mismatch() {
    let (src_dir, path) =
        multi_route_prod_server_source("deploy-smoke-route-reveal-summary-source");
    let out = temp_output_dir("deploy-smoke-route-reveal-summary");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    write_text(
        &smoke_path,
        &smoke.replace(
            r#"orv_smoke_reveal_contains "reveal GET /ping route summary" "$ORV_SMOKE_ORIGIN_GET_PING" '"route_target_count": 1'"#,
            r#"orv_smoke_reveal_contains "reveal GET /ping route summary" "$ORV_SMOKE_ORIGIN_GET_PING" '"route_target_count": 2'"#,
        ),
    )
    .expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("route reveal summary count mismatch");

    assert!(
        err.to_string()
            .contains("deploy smoke test must verify reveal production summary for GET /ping"),
        "{err:?}"
    );
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_origin_assignment_mismatch() {
    let (src_dir, path) = prod_server_source("deploy-smoke-origin-source");
    let out = temp_output_dir("deploy-smoke-origin-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let artifact = read_server_artifact(&out.join("server").join("app.orv-runtime.json"))
        .expect("server artifact");
    let route = artifact
        .routes
        .iter()
        .find(|route| route.method == "GET" && route.path == "/ping")
        .expect("GET /ping route");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    let expected = format!(r#"ORV_SMOKE_ORIGIN_GET_PING="{}""#, route.origin_id);
    let smoke = smoke.replace(&expected, r#"ORV_SMOKE_ORIGIN_GET_PING="ori_wrong""#);
    write_text(&smoke_path, &smoke).expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke origin mismatch");

    assert!(err
        .to_string()
        .contains("deploy smoke test must declare expected origin for GET /ping"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_build_rejects_deploy_smoke_response_origin_assignment_mismatch() {
    let (src_dir, path) = prod_server_source("deploy-smoke-response-origin-source");
    let out = temp_output_dir("deploy-smoke-response-origin-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let artifact = read_server_artifact(&out.join("server").join("app.orv-runtime.json"))
        .expect("server artifact");
    let route = artifact
        .routes
        .iter()
        .find(|route| route.method == "GET" && route.path == "/ping")
        .expect("GET /ping route");
    let response_origin = route
        .response_origin_ids
        .first()
        .expect("GET /ping response origin");
    let smoke_path = out.join("deploy").join("smoke-test.sh");
    let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
    let expected = format!(r#"ORV_SMOKE_RESPONSE_ORIGIN_GET_PING="{response_origin}""#);
    let smoke = smoke.replace(
        &expected,
        r#"ORV_SMOKE_RESPONSE_ORIGIN_GET_PING="ori_wrong""#,
    );
    write_text(&smoke_path, &smoke).expect("write corrupt smoke test");

    let err = cmd_verify_build(&out).expect_err("smoke response origin mismatch");

    assert!(err
        .to_string()
        .contains("deploy smoke test must declare expected response origin for GET /ping"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(out);
}

#[test]
fn verify_smoke_artifact_cases() {
    verify_artifact_cases(
        "verify_smoke_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            artifact_case("deploy_smoke_graph_contract_missing", |out| {
                let smoke_path = out.join("deploy").join("smoke-test.sh");
                let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
                write_text(
                    &smoke_path,
                    &smoke.replace("\norv_smoke_graph_contract\n", "\n"),
                )
                .expect("write corrupt smoke test");

                let err = cmd_verify_build(out).expect_err("smoke graph contract mismatch");

                assert!(err
                    .to_string()
                    .contains("deploy smoke test must verify the build graph contract"));
            }),
            artifact_case("deploy_smoke_reveal_marker_contract_missing", |out| {
                let smoke_path = out.join("deploy").join("smoke-test.sh");
                let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
                write_text(
        &smoke_path,
        &smoke.replace(
            r#"orv_smoke_reveal_contains "reveal smoke required markers" "$ORV_SMOKE_ORIGIN_GET_PING" '"smoke_test_required_markers": ['
"#,
            "",
        ),
    )
    .expect("write corrupt smoke test");

                let err = cmd_verify_build(out).expect_err("smoke reveal marker contract mismatch");

                assert!(
        err.to_string()
            .contains("deploy smoke test must verify smoke marker contract across reveal surfaces"),
        "{err:?}"
    );
            }),
            artifact_case("deploy_smoke_output_contract_missing", |out| {
                let smoke_path = out.join("deploy").join("smoke-test.sh");
                let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
                write_text(
                    &smoke_path,
                    &smoke.replace(r#"> "$ORV_SMOKE_OUTPUT""#, r#"> /dev/null"#),
                )
                .expect("write corrupt smoke test");

                let err = cmd_verify_build(out).expect_err("smoke output contract mismatch");

                assert!(
                    err.to_string()
                        .contains("deploy smoke test must write deploy smoke output artifact"),
                    "{err:?}"
                );
            }),
            artifact_case("deploy_smoke_trace_frame_wrapper_gate_missing", |out| {
                let smoke_path = out.join("deploy").join("smoke-test.sh");
                let smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
                for (from, to) in [
                    (
                        r#"'"kind":"orv.production.trace.frame"'"#,
                        r#"'"kind":"orv.production.trace"'"#,
                    ),
                    (r#"'"index":0'"#, r#"'"index":1'"#),
                    (r#"'"frame":{'"#, r#"'"request":{'"#),
                    (
                        r#"'"trace_frame_event_count":'"#,
                        r#"'"trace_event_count":'"#,
                    ),
                ] {
                    write_text(&smoke_path, &smoke.replace(from, to))
                        .expect("write corrupt smoke test");

                    let err = cmd_verify_build(out)
                        .expect_err("trace frame wrapper gate drift must fail");

                    assert!(
                        err.to_string()
                            .contains("deploy smoke test must optionally verify live trace stream"),
                        "{from} drift should fail trace stream verifier; got {err:?}"
                    );
                }
            }),
            artifact_case("deploy_runbook_smoke_marker_mismatch", |out| {
                let runbook_path = out.join("deploy").join("README.md");
                let mut runbook = std::fs::read_to_string(&runbook_path).expect("runbook");
                runbook = runbook.replace("- `dap_source_bundle`", "- `dap_source_bundle_missing`");
                write_text(&runbook_path, &runbook).expect("write corrupt runbook");

                let err = cmd_verify_build(out).expect_err("runbook smoke marker mismatch");

                assert!(err.to_string().contains(
                    "deploy runbook must document smoke output marker dap_source_bundle"
                ));
            }),
            json_case(
                "deploy_preflight_smoke_output_contract_mismatch",
                "deploy/preflight.json",
                "deploy preflight smoke_output_contract must match smoke output contract",
                |preflight| {
                    preflight["smoke_output_contract"]["required_markers"] =
                        serde_json::json!(["pass_marker", "build_dir"]);
                },
            ),
            json_case(
                "deploy_preflight_extra_smoke_output_contract_key",
                "deploy/preflight.json",
                "deploy preflight smoke_output_contract keys must match contract",
                |preflight| {
                    preflight["smoke_output_contract"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "deploy_preflight_smoke_command_mismatch",
                "deploy/preflight.json",
                "deploy preflight smoke_test command must be ./deploy/smoke-test.sh",
                |preflight| {
                    preflight["commands"]["smoke_test"] =
                        serde_json::json!("./deploy/other-smoke.sh");
                },
            ),
            json_case(
                "deploy_preflight_trace_stream_smoke_command_mismatch",
                "deploy/preflight.json",
                "deploy preflight trace_stream_smoke command",
                |preflight| {
                    preflight["commands"]["trace_stream_smoke"] =
                        serde_json::json!("./deploy/smoke-test.sh");
                },
            ),
        ],
    );
}

#[cfg(unix)]
#[test]
fn verify_smoke_permission_artifact_cases() {
    verify_artifact_cases(
        "verify_smoke_permission_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            artifact_case("non_executable_deploy_smoke_test", |out| {
                use std::os::unix::fs::PermissionsExt;
                let smoke_path = out.join("deploy").join("smoke-test.sh");
                let mut permissions = std::fs::metadata(&smoke_path)
                    .expect("smoke metadata")
                    .permissions();
                permissions.set_mode(0o644);
                std::fs::set_permissions(&smoke_path, permissions).expect("remove executable bit");

                let err = cmd_verify_build(out).expect_err("smoke test mode mismatch");

                assert!(err
                    .to_string()
                    .contains("deploy smoke test must be executable"));
            }),
            artifact_case("invalid_deploy_smoke_test_shell_syntax", |out| {
                let smoke_path = out.join("deploy").join("smoke-test.sh");
                let mut smoke = std::fs::read_to_string(&smoke_path).expect("smoke test");
                smoke.push_str("\nif\n");
                std::fs::write(&smoke_path, smoke).expect("write corrupt smoke script");

                let err = cmd_verify_build(out).expect_err("smoke shell syntax mismatch");

                assert!(err
                    .to_string()
                    .contains("deploy smoke test shell syntax invalid"));
            }),
        ],
    );
}
