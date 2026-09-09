use super::*;

#[test]
fn verify_build_rejects_native_server_plan_command_mismatch() {
    let (src_dir, path) = prod_server_source("native-server-plan-command-source");
    let out = temp_output_dir("native-server-plan-command-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let native_plan_path = out.join("server").join("native-server.json");
    let mut native_plan = read_json_value(&native_plan_path).expect("native server plan");
    native_plan["commands"] = serde_json::json!({
        "build": [
            "wrong-cargo",
            "build",
            "--manifest-path",
            "server/native/Cargo.toml",
            "--release"
        ],
        "run": {
            "env": {
                "ORV_BUILD_DIR": "."
            },
            "command": [
                "./server/native/target/release/orv-native-server"
            ]
        }
    });
    write_json(&native_plan_path, &native_plan).expect("write corrupt native server plan");

    let err = cmd_verify_build(&out).expect_err("native server plan command mismatch");

    assert!(err
        .to_string()
        .contains("native server plan build command must match generated launcher package"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_native_server_routes_source_mismatch() {
    let (src_dir, path) = prod_server_source("native-server-routes-source");
    let out = temp_output_dir("native-server-routes-source-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let routes_path = out.join("server").join("native").join("routes.rs");
    let mut source = std::fs::read_to_string(&routes_path).expect("native routes source");
    source = source.replace("path: \"/ping\"", "path: \"/wrong\"");
    write_text(&routes_path, &source).expect("write corrupt native routes source");

    let err = cmd_verify_build(&out).expect_err("native routes source mismatch");

    assert!(err
        .to_string()
        .contains("native server routes source must match server runtime artifact"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_native_server_launcher_package_mismatch() {
    let (src_dir, path) = prod_server_source("native-server-package-source");
    let out = temp_output_dir("native-server-package-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let package_path = out.join("server").join("native").join("Cargo.toml");
    let mut package = std::fs::read_to_string(&package_path).expect("native package");
    package = package.replace("path = \"main.rs\"", "path = \"wrong.rs\"");
    write_text(&package_path, &package).expect("write corrupt native package");

    let err = cmd_verify_build(&out).expect_err("native server package mismatch");

    assert!(err
        .to_string()
        .contains("native server launcher package bin path must be main.rs"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_native_artifact_cases() {
    verify_artifact_cases(
        "verify_native_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            json_case(
                "native_server_plan_mismatch",
                "server/native-server.json",
                "native server plan artifact must be server/app.orv-runtime.json",
                |native_plan| {
                    native_plan["artifact"] = serde_json::json!("server/wrong.orv-runtime.json");
                },
            ),
            json_case(
                "native_server_plan_extra_root_key",
                "server/native-server.json",
                "native server plan keys must match contract",
                |native_plan| {
                    native_plan["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "native_server_plan_extra_target_key",
                "server/native-server.json",
                "native server plan target keys must match contract",
                |native_plan| {
                    native_plan["target"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "native_server_plan_extra_run_key",
                "server/native-server.json",
                "native server plan run command keys must match contract",
                |native_plan| {
                    native_plan["commands"]["run"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "native_server_plan_extra_commands_key",
                "server/native-server.json",
                "native server plan commands keys must match contract",
                |native_plan| {
                    native_plan["commands"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "native_server_plan_extra_run_env_key",
                "server/native-server.json",
                "native server plan run env keys must match contract",
                |native_plan| {
                    native_plan["commands"]["run"]["env"]["UNEXPECTED"] =
                        serde_json::json!("drift");
                },
            ),
            json_case(
                "native_server_plan_extra_route_key",
                "server/native-server.json",
                "native server plan routes[0] keys must match contract",
                |native_plan| {
                    native_plan["routes"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "native_server_plan_extra_response_key",
                "server/native-server.json",
                "native server plan routes[0].responses[0] keys must match contract",
                |native_plan| {
                    native_plan["routes"][0]["responses"][0]["unexpected"] =
                        serde_json::json!("drift");
                },
            ),
            json_case(
                "native_runtime_image_plan_extra_root_key",
                "server/runtime-image.json",
                "native runtime image plan keys must match contract",
                |image_plan| {
                    image_plan["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "native_runtime_image_plan_extra_target_key",
                "server/runtime-image.json",
                "native runtime image plan target keys must match contract",
                |image_plan| {
                    image_plan["target"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "native_runtime_image_plan_extra_commands_key",
                "server/runtime-image.json",
                "native runtime image plan commands keys must match contract",
                |image_plan| {
                    image_plan["commands"]["unexpected"] = serde_json::json!("drift");
                },
            ),
            artifact_case("native_runtime_image_dockerfile_mismatch", |out| {
                let dockerfile_path = out.join(NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH);
                let mut dockerfile = std::fs::read_to_string(&dockerfile_path).expect("Dockerfile");
                dockerfile.push_str("RUN echo drift\n");
                write_text(&dockerfile_path, &dockerfile).expect("write drifted Dockerfile");

                let err = cmd_verify_build(out).expect_err("Dockerfile drift must fail");

                assert!(err
                    .to_string()
                    .contains("native runtime image Dockerfile must match generated Dockerfile"));
            }),
            artifact_case("native_server_launcher_source_mismatch", |out| {
                let source_path = out.join("server").join("native").join("main.rs");
                let mut source = std::fs::read_to_string(&source_path).expect("native source");
                source = source.replace(
                    "router::orv_native_dispatch_with_request(",
                    "router::orv_native_dispatch(\"GET\", \"/wrong\")",
                );
                write_text(&source_path, &source).expect("write corrupt native source");

                let err = cmd_verify_build(out).expect_err("native server source mismatch");

                assert!(err.to_string().contains(
                    "native server launcher source must dispatch through generated router"
                ));
            }),
            artifact_case("native_server_launcher_compile_error", |out| {
                let source_path = out.join("server").join("native").join("main.rs");
                let mut source = std::fs::read_to_string(&source_path).expect("native source");
                source.push_str("\nfn __orv_compile_error( {\n");
                write_text(&source_path, &source).expect("write corrupt native source");

                let err = cmd_verify_build(out).expect_err("native server source compile mismatch");

                assert!(err
                    .to_string()
                    .contains("native server launcher source must match generated source"));
            }),
            artifact_case("native_server_router_source_mismatch", |out| {
                let router_path = out.join("server").join("native").join("router.rs");
                let mut source =
                    std::fs::read_to_string(&router_path).expect("native router source");
                source = source.replace(
                    "handlers::orv_native_handle_route(&route_match)",
                    "handlers::orv_native_handle_missing_route(&route_match)",
                );
                write_text(&router_path, &source).expect("write corrupt native router source");

                let err = cmd_verify_build(out).expect_err("native router source mismatch");

                assert!(err
                    .to_string()
                    .contains("native server router source must match generated source"));
            }),
        ],
    );
}
