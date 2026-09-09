use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeployNativeServerSummaryCounts {
    pub(crate) targets: usize,
    pub(crate) routes: usize,
}

pub(crate) fn deploy_native_server_summary_counts(
    dir: &Path,
) -> anyhow::Result<DeployNativeServerSummaryCounts> {
    let targets = editor_production_native_server_targets(dir)?;
    Ok(DeployNativeServerSummaryCounts {
        targets: targets.len(),
        routes: production_native_server_route_count(&targets),
    })
}
pub(crate) const NATIVE_SERVER_PLAN_PATH: &str = "server/native-server.json";
pub(crate) const NATIVE_RUNTIME_IMAGE_PLAN_PATH: &str = "server/runtime-image.json";
pub(crate) const NATIVE_RUNTIME_IMAGE_DOCKERFILE_PATH: &str = "server/native/Dockerfile";
pub(crate) const NATIVE_SERVER_SOURCE_PATH: &str = "server/native/main.rs";
pub(crate) const NATIVE_SERVER_ROUTES_SOURCE_PATH: &str = "server/native/routes.rs";
pub(crate) const NATIVE_SERVER_ROUTER_SOURCE_PATH: &str = "server/native/router.rs";
pub(crate) const NATIVE_SERVER_HANDLERS_SOURCE_PATH: &str = "server/native/handlers.rs";
pub(crate) const NATIVE_SERVER_PACKAGE_PATH: &str = "server/native/Cargo.toml";
pub(crate) const NATIVE_SERVER_BINARY_PATH: &str = "server/app";
pub(crate) const NATIVE_SERVER_LAUNCHER_BINARY_PATH: &str =
    "./server/native/target/release/orv-native-server";
pub(crate) const NATIVE_RUNTIME_IMAGE_NAME: &str = "orv-native-server:latest";
pub(crate) const NATIVE_RUNTIME_IMAGE_DOCKERFILE_SOURCE: &str = r#"FROM rust:1-bookworm AS build
WORKDIR /work
COPY server/native /work/server/native
RUN cargo build --manifest-path /work/server/native/Cargo.toml --release

FROM debian:bookworm-slim
WORKDIR /app
COPY . /app
COPY --from=build /work/server/native/target/release/orv-native-server /app/server/app
ENV ORV_BUILD_DIR=/app
ENV ORV_HOST=0.0.0.0
ENTRYPOINT ["/app/server/app"]
"#;

pub(crate) struct NativeServerPlanPaths<'a> {
    pub(crate) plan: &'a str,
    pub(crate) artifact: &'a str,
    pub(crate) launcher: &'a str,
    pub(crate) source: &'a str,
    pub(crate) routes_source: &'a str,
    pub(crate) router_source: &'a str,
    pub(crate) handlers_source: &'a str,
    pub(crate) package: &'a str,
    pub(crate) runtime_image_plan: &'a str,
}

pub(crate) fn write_native_server_plan_artifact(
    out: &Path,
    paths: &NativeServerPlanPaths<'_>,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let direct_http = orv_compiler::native_server_direct_http_capable(server_artifact);
    let plan = orv_compiler::NativeServerPlanArtifact {
        schema_version: orv_compiler::NATIVE_SERVER_PLAN_ARTIFACT_VERSION,
        kind: "native_server_plan".to_string(),
        status: native_server_plan_status(direct_http).to_string(),
        runtime: server_artifact.runtime.clone(),
        runtime_features: server_artifact.runtime_features.clone(),
        artifact: paths.artifact.to_string(),
        launcher: paths.launcher.to_string(),
        source: paths.source.to_string(),
        routes_source: paths.routes_source.to_string(),
        router_source: paths.router_source.to_string(),
        handlers_source: paths.handlers_source.to_string(),
        package: paths.package.to_string(),
        runtime_image_plan: paths.runtime_image_plan.to_string(),
        target: orv_compiler::NativeServerTargetArtifact {
            kind: "server_binary".to_string(),
            path: NATIVE_SERVER_BINARY_PATH.to_string(),
            protocol: "http1".to_string(),
        },
        commands: orv_compiler::NativeServerCommands {
            build: vec![
                "cargo".to_string(),
                "build".to_string(),
                "--manifest-path".to_string(),
                paths.package.to_string(),
                "--release".to_string(),
            ],
            run: orv_compiler::NativeServerRunCommand {
                env: HashMap::from([("ORV_BUILD_DIR".to_string(), ".".to_string())]),
                command: vec![NATIVE_SERVER_LAUNCHER_BINARY_PATH.to_string()],
            },
        },
        blocked_by: native_server_plan_blockers(direct_http),
        listen: server_artifact.listen.clone(),
        routes: server_artifact.routes.clone(),
    };
    write_json(&out.join(paths.plan), &serde_json::to_value(plan)?)
}

pub(crate) fn write_native_runtime_image_plan_artifact(
    out: &Path,
    path: &str,
    dockerfile_path: &str,
    server_artifact_path: &str,
    native_server_plan_path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let direct_http = orv_compiler::native_server_direct_http_capable(server_artifact);
    let plan = orv_compiler::NativeRuntimeImagePlanArtifact {
        schema_version: orv_compiler::NATIVE_RUNTIME_IMAGE_PLAN_ARTIFACT_VERSION,
        kind: "native_runtime_image_plan".to_string(),
        status: native_runtime_image_plan_status(direct_http).to_string(),
        runtime: server_artifact.runtime.clone(),
        runtime_features: server_artifact.runtime_features.clone(),
        artifact: server_artifact_path.to_string(),
        native_plan: native_server_plan_path.to_string(),
        reference_image: ORV_REFERENCE_RUNTIME_IMAGE.to_string(),
        target: orv_compiler::NativeRuntimeImageTargetArtifact {
            kind: "oci_image".to_string(),
            image: NATIVE_RUNTIME_IMAGE_NAME.to_string(),
            binary: NATIVE_SERVER_BINARY_PATH.to_string(),
            protocol: "http1".to_string(),
        },
        dockerfile: dockerfile_path.to_string(),
        commands: orv_compiler::NativeRuntimeImageCommands {
            build: vec![
                "docker".to_string(),
                "build".to_string(),
                "-f".to_string(),
                dockerfile_path.to_string(),
                "-t".to_string(),
                NATIVE_RUNTIME_IMAGE_NAME.to_string(),
                ".".to_string(),
            ],
        },
        blocked_by: native_runtime_image_plan_blockers(direct_http),
        listen: server_artifact.listen.clone(),
        routes: server_artifact.routes.clone(),
    };
    write_json(&out.join(path), &serde_json::to_value(plan)?)
}

pub(crate) fn native_server_plan_status(direct_http: bool) -> &'static str {
    if direct_http {
        "direct_http"
    } else {
        "planned"
    }
}

pub(crate) fn native_runtime_image_plan_status(direct_http: bool) -> &'static str {
    if direct_http {
        "image_planned"
    } else {
        "planned"
    }
}

pub(crate) fn native_server_plan_blockers(direct_http: bool) -> Vec<String> {
    if direct_http {
        Vec::new()
    } else {
        vec![
            "native-codegen".to_string(),
            "native-runtime-image".to_string(),
        ]
    }
}

pub(crate) fn native_runtime_image_plan_blockers(direct_http: bool) -> Vec<String> {
    if direct_http {
        Vec::new()
    } else {
        vec![
            "native-codegen".to_string(),
            "native-runtime-image".to_string(),
        ]
    }
}

pub(crate) fn write_native_runtime_image_dockerfile(out: &Path, path: &str) -> anyhow::Result<()> {
    write_text(&out.join(path), NATIVE_RUNTIME_IMAGE_DOCKERFILE_SOURCE)
}

pub(crate) fn write_native_server_launcher_source(
    out: &Path,
    path: &str,
    server_artifact_path: &str,
    native_server_plan_path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let source = orv_compiler::native_server_launcher_source(
        server_artifact_path,
        native_server_plan_path,
        server_artifact,
    );
    write_text(&out.join(path), &source)
}

pub(crate) fn write_native_server_routes_source(
    out: &Path,
    path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let source = orv_compiler::native_server_routes_source(server_artifact);
    write_text(&out.join(path), &source)
}

pub(crate) fn write_native_server_router_source(out: &Path, path: &str) -> anyhow::Result<()> {
    let source = orv_compiler::native_server_router_source();
    write_text(&out.join(path), &source)
}

pub(crate) fn write_native_server_handlers_source(
    out: &Path,
    path: &str,
    server_artifact: &orv_compiler::ServerRuntimeArtifact,
) -> anyhow::Result<()> {
    let source = orv_compiler::native_server_handlers_source(server_artifact);
    write_text(&out.join(path), &source)
}

pub(crate) fn write_native_server_launcher_package(out: &Path, path: &str) -> anyhow::Result<()> {
    let manifest = r#"[package]
name = "orv-native-server"
version = "0.1.0"
edition = "2021"
publish = false

[[bin]]
name = "orv-native-server"
path = "main.rs"
"#;
    write_text(&out.join(path), manifest)
}
