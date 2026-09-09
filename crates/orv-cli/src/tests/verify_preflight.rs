use super::*;

#[test]
fn verify_build_rejects_deploy_preflight_runtime_mirror_mismatches() {
    let (src_dir, path) = prod_server_source("deploy-preflight-runtime-mirror-source");
    let out = temp_output_dir("deploy-preflight-runtime-mirror-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let preflight_path = out.join("deploy").join("preflight.json");
    let original = read_json_value(&preflight_path).expect("preflight");

    for (pointer, value, expected) in [
        (
            "/runtime",
            serde_json::json!("other-runtime"),
            "deploy preflight runtime does not match runtime artifact",
        ),
        (
            "/security_features",
            serde_json::json!(["unexpected"]),
            "deploy preflight security_features do not match runtime artifact",
        ),
        (
            "/listen/port",
            serde_json::json!(9090),
            "deploy preflight listen does not match runtime artifact",
        ),
        (
            "/routes/0/path",
            serde_json::json!("/wrong"),
            "deploy preflight routes do not match runtime artifact",
        ),
        (
            "/persistence/db_paths",
            serde_json::json!(["wrong.db"]),
            "deploy preflight persistence does not match runtime artifact",
        ),
        (
            "/required_env",
            serde_json::json!(["ORV_REQUIRED_DRIFT"]),
            "deploy preflight required_env does not match runtime artifact",
        ),
        (
            "/optional_env",
            serde_json::json!(["ORV_OPTIONAL_DRIFT"]),
            "deploy preflight optional_env does not match runtime artifact",
        ),
        (
            "/client",
            serde_json::json!({"enabled": true}),
            "deploy preflight client does not match deploy manifest",
        ),
    ] {
        let mut preflight = original.clone();
        *preflight
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("preflight pointer {pointer} must exist")) = value;
        write_json(&preflight_path, &preflight).expect("write corrupt preflight");

        let err = cmd_verify_build(&out).expect_err("preflight mirror mismatch must fail");

        assert!(
            err.to_string().contains(expected),
            "{pointer} drift should fail with {expected}; got {err}"
        );
    }

    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_preflight_artifact_cases() {
    verify_artifact_cases(
        "verify_preflight_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            artifact_case("deploy_env_example_extra_drift", |out| {
                let env_example_path = out.join("deploy").join("env.example");
                let mut env_example =
                    std::fs::read_to_string(&env_example_path).expect("env example");
                env_example.push_str("EXTRA_DEPLOY_DRIFT=1\n");
                write_text(&env_example_path, &env_example).expect("write corrupt env example");

                let err = cmd_verify_build(out).expect_err("env example extra drift");

                assert!(err
                    .to_string()
                    .contains("deploy env example must match generated artifact"));
            }),
            json_case(
                "deploy_preflight_runtime_feature_mismatch",
                "deploy/preflight.json",
                "deploy preflight runtime_features do not match runtime artifact",
                |preflight| {
                    preflight["runtime_features"] = serde_json::json!(["http_server"]);
                },
            ),
            json_case(
                "deploy_preflight_extra_root_key",
                "deploy/preflight.json",
                "deploy preflight keys must match contract",
                |preflight| {
                    preflight["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "deploy_preflight_root_artifact_mismatch",
                "deploy/preflight.json",
                "deploy preflight artifact",
                |preflight| {
                    preflight["artifact"] = serde_json::json!("server/other.orv-runtime.json");
                },
            ),
            json_case(
                "deploy_preflight_extra_command_key",
                "deploy/preflight.json",
                "deploy preflight commands keys must match contract",
                |preflight| {
                    preflight["commands"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "deploy_preflight_extra_artifact_key",
                "deploy/preflight.json",
                "deploy preflight artifacts keys must match contract",
                |preflight| {
                    preflight["artifacts"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            artifact_case("deploy_preflight_remaining_command_mismatches", |out| {
                let preflight_path = out.join("deploy").join("preflight.json");
                let original = read_json_value(&preflight_path).expect("preflight");

                for (key, value, expected) in [
                    (
                        "verify_build",
                        "orv verify-build other",
                        "deploy preflight verify_build command",
                    ),
                    (
                        "env_check",
                        "orv deploy-env-check other",
                        "deploy preflight env_check command",
                    ),
                    (
                        "benchmark_prepare",
                        "orv benchmark-prepare .",
                        "deploy preflight benchmark_prepare command",
                    ),
                    (
                        "benchmark_report_require_pass",
                        "orv benchmark-report .",
                        "deploy preflight benchmark_report_require_pass command",
                    ),
                    (
                        "compose_up",
                        "docker compose up -d",
                        "deploy preflight compose_up command",
                    ),
                    (
                        "trace",
                        "./deploy/server.sh --trace other.json",
                        "deploy preflight trace command",
                    ),
                    (
                        "editor_trace",
                        "orv editor trace . --trace other.json",
                        "deploy preflight editor_trace command",
                    ),
                ] {
                    let mut preflight = original.clone();
                    preflight["commands"][key] = serde_json::json!(value);
                    write_json(&preflight_path, &preflight).expect("write corrupt preflight");

                    let err =
                        cmd_verify_build(out).expect_err("preflight command mismatch must fail");

                    assert!(
                        err.to_string().contains(expected),
                        "{key} drift should fail with {expected}; got {err}"
                    );
                }
            }),
            json_case(
                "deploy_preflight_run_build_command_mismatch",
                "deploy/preflight.json",
                "deploy preflight run_build command",
                |preflight| {
                    preflight["commands"]["run_build"] = serde_json::json!("orv run-build other");
                },
            ),
            json_case(
                "deploy_preflight_trace_run_build_command_mismatch",
                "deploy/preflight.json",
                "deploy preflight trace_run_build command",
                |preflight| {
                    preflight["commands"]["trace_run_build"] =
                        serde_json::json!("orv run-build . --trace other.json");
                },
            ),
            json_case(
                "deploy_preflight_graph_artifact_mismatch",
                "deploy/preflight.json",
                "deploy preflight artifact origin_map must be origin-map.json",
                |preflight| {
                    preflight["artifacts"]["origin_map"] =
                        serde_json::json!("wrong-origin-map.json");
                },
            ),
            artifact_case("deploy_preflight_remaining_artifact_mismatches", |out| {
                let preflight_path = out.join("deploy").join("preflight.json");
                let original = read_json_value(&preflight_path).expect("preflight");

                for (key, value, expected) in [
                    (
                        "server",
                        "server/other.orv-runtime.json",
                        "deploy preflight artifact server",
                    ),
                    (
                        "routes",
                        "deploy/other-routes.json",
                        "deploy preflight artifact routes",
                    ),
                    (
                        "source_bundle",
                        "wrong-source-bundle.json",
                        "deploy preflight artifact source_bundle",
                    ),
                    (
                        "project_graph",
                        "wrong-project-graph.json",
                        "deploy preflight artifact project_graph",
                    ),
                    (
                        "build_manifest",
                        "wrong-build-manifest.json",
                        "deploy preflight artifact build_manifest",
                    ),
                    (
                        "bundle_plan",
                        "wrong-bundle-plan.json",
                        "deploy preflight artifact bundle_plan",
                    ),
                    (
                        "env_example",
                        "deploy/wrong-env.example",
                        "deploy preflight artifact env_example",
                    ),
                    (
                        "db_adapters",
                        "deploy/wrong-db-adapters.json",
                        "deploy preflight artifact db_adapters",
                    ),
                    (
                        "commerce_adapters",
                        "deploy/wrong-commerce-adapters.json",
                        "deploy preflight artifact commerce_adapters",
                    ),
                    (
                        "smoke_test",
                        "deploy/wrong-smoke-test.sh",
                        "deploy preflight artifact smoke_test",
                    ),
                    (
                        "smoke_output",
                        "deploy/wrong-smoke-output.txt",
                        "deploy preflight artifact smoke_output",
                    ),
                    (
                        "preflight",
                        "deploy/wrong-preflight.json",
                        "deploy preflight artifact preflight",
                    ),
                    (
                        "benchmark_evidence",
                        "deploy/wrong-benchmark-evidence.json",
                        "deploy preflight artifact benchmark_evidence",
                    ),
                    (
                        "runbook",
                        "deploy/wrong-readme.md",
                        "deploy preflight artifact runbook",
                    ),
                ] {
                    let mut preflight = original.clone();
                    preflight["artifacts"][key] = serde_json::json!(value);
                    write_json(&preflight_path, &preflight).expect("write corrupt preflight");

                    let err =
                        cmd_verify_build(out).expect_err("preflight artifact mismatch must fail");

                    assert!(
                        err.to_string().contains(expected),
                        "{key} drift should fail with {expected}; got {err}"
                    );
                }
            }),
        ],
    );
}
