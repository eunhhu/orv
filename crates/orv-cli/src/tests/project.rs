use super::*;

#[test]
fn check_accepts_orv_toml_project_entry() {
    let dir = temp_output_dir("project-manifest-check");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("create src dir");
    let entry = src.join("main.orv");
    std::fs::write(&entry, "@out \"manifest check\"\n").expect("write entry");
    let manifest = dir.join("orv.toml");
    std::fs::write(
        &manifest,
        r#"[project]
name = "manifest-demo"
entry = "src/main.orv"
"#,
    )
    .expect("write manifest");

    cmd_check(&manifest).expect("manifest check");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn init_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "init", "target/new-shop", "--name", "new-shop"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn test_summary_runs_only_matching_test_blocks() {
    let dir = temp_output_dir("test-runner-filter-isolation");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let source = dir.join("checkout_test.orv");
    std::fs::write(
        &source,
        r#"test "checkout only" {
  assert true
}

test "checkout excluded failure" {
  assert false
}
"#,
    )
    .expect("write test source");

    let summary = orv_test_summary(&dir, Some("only")).expect("test summary");

    assert_eq!(summary.selected, 1);
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.files, vec![source.clone()]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn init_writes_project_manifest_and_entry() {
    let dir = temp_output_dir("init-project");

    cmd_init(&dir, Some("starter-shop"), InitTemplate::Basic).expect("init project");

    let manifest = dir.join("orv.toml");
    let entry = dir.join("src").join("main.orv");
    assert!(manifest.is_file(), "missing {}", manifest.display());
    assert!(entry.is_file(), "missing {}", entry.display());
    let manifest_text = std::fs::read_to_string(&manifest).expect("manifest text");
    assert!(manifest_text.contains("name = \"starter-shop\""));
    assert!(manifest_text.contains("entry = \"src/main.orv\""));
    cmd_check(&manifest).expect("check manifest project");
    cmd_check(&dir).expect("check project directory");
    let out = dir.join("dist");
    cmd_build(&dir, &out).expect("build project directory");
    assert!(out.join("pages").join("index.html").is_file());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "lock", "demo", "--check"])
        .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Lock { dir, check } = parsed.command else {
        panic!("expected lock command");
    };
    assert_eq!(dir, PathBuf::from("demo"));
    assert!(check);
}

#[test]
fn fetch_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "fetch", "demo", "--out", "target/orv-deps"])
        .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Fetch { dir, out } = parsed.command else {
        panic!("expected fetch command");
    };
    assert_eq!(dir, PathBuf::from("demo"));
    assert_eq!(out, PathBuf::from("target/orv-deps"));
}

#[test]
fn add_and_remove_subcommands_are_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "add",
        "auth",
        "1.2.3",
        "--manifest",
        "demo",
        "--dev",
        "--registry",
        "https://registry.orv.dev",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Add {
        pkg,
        version,
        manifest,
        dev,
        path,
        registry,
    } = parsed.command
    else {
        panic!("expected add command");
    };
    assert_eq!(pkg, "auth");
    assert_eq!(version.as_deref(), Some("1.2.3"));
    assert_eq!(manifest, PathBuf::from("demo"));
    assert!(dev);
    assert!(path.is_none());
    assert_eq!(registry.as_deref(), Some("https://registry.orv.dev"));

    let parsed = Cli::try_parse_from(["orv", "remove", "auth", "--manifest", "demo"])
        .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Remove { pkg, manifest, dev } = parsed.command else {
        panic!("expected remove command");
    };
    assert_eq!(pkg, "auth");
    assert_eq!(manifest, PathBuf::from("demo"));
    assert!(!dev);
}

#[test]
fn workspace_new_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "workspace",
        "new",
        "apps/web",
        "--root",
        "demo",
        "--name",
        "web",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Workspace { command } = parsed.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::New {
        member,
        root,
        name,
        template,
    } = command
    else {
        panic!("expected workspace new command");
    };
    assert_eq!(member, PathBuf::from("apps/web"));
    assert_eq!(root, PathBuf::from("demo"));
    assert_eq!(name.as_deref(), Some("web"));
    assert_eq!(template, InitTemplate::Basic);
}

#[test]
fn workspace_graph_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "workspace",
        "graph",
        "demo",
        "--view",
        "--out",
        "target/orv-workspace-view",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Workspace { command } = parsed.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Graph { root, view, out } = command else {
        panic!("expected workspace graph command");
    };
    assert_eq!(root, PathBuf::from("demo"));
    assert!(view);
    assert_eq!(out, Some(PathBuf::from("target/orv-workspace-view")));
}

#[test]
fn workspace_lock_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "workspace",
        "lock",
        "demo",
        "--out",
        "target/orv-workspace-lock",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Workspace { command } = parsed.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Lock { root, out } = command else {
        panic!("expected workspace lock command");
    };
    assert_eq!(root, PathBuf::from("demo"));
    assert_eq!(out, PathBuf::from("target/orv-workspace-lock"));
}

#[test]
fn workspace_fetch_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "workspace",
        "fetch",
        "demo",
        "--out",
        "target/orv-workspace-deps",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Workspace { command } = parsed.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Fetch { root, out } = command else {
        panic!("expected workspace fetch command");
    };
    assert_eq!(root, PathBuf::from("demo"));
    assert_eq!(out, PathBuf::from("target/orv-workspace-deps"));
}

#[test]
fn workspace_build_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "workspace",
        "build",
        "demo",
        "--out",
        "target/orv-workspace-build",
        "--prod",
        "--incremental",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Workspace { command } = parsed.command else {
        panic!("expected workspace command");
    };
    let WorkspaceCommand::Build {
        root,
        out,
        prod,
        incremental,
    } = command
    else {
        panic!("expected workspace build command");
    };
    assert_eq!(root, PathBuf::from("demo"));
    assert_eq!(out, PathBuf::from("target/orv-workspace-build"));
    assert!(prod);
    assert!(incremental);
}

#[test]
fn lock_writes_and_checks_deterministic_project_lockfile() {
    let dir = temp_output_dir("project-lock");
    std::fs::create_dir_all(dir.join("src")).expect("create src");
    std::fs::write(dir.join("src").join("main.orv"), "@out \"lock\"\n").expect("write entry");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
zeta = "2.0.0"
auth = { version = "1.2.3", registry = "https://registry.orv.dev" }
ui = { version = "0.1.0", path = "libs/ui" }

[dev-dependencies]
mock-server = "0.2.0"
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["schema_version"], 1);
    assert_eq!(lock["kind"], "orv.lock");
    assert_eq!(lock["project"]["name"], "shop");
    assert_eq!(lock["project"]["version"], "0.1.0");
    assert_eq!(lock["project"]["entry"], "src/main.orv");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.2.3");
    assert_eq!(lock["dependencies"][0]["source"], "registry");
    assert_eq!(
        lock["dependencies"][0]["registry"],
        "https://registry.orv.dev"
    );
    assert!(lock["dependencies"][0]["checksum"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("fnv1a64:")));
    assert_eq!(lock["dependencies"][1]["name"], "ui");
    assert_eq!(lock["dependencies"][1]["source"], "path");
    assert_eq!(lock["dependencies"][1]["path"], "libs/ui");
    assert_eq!(lock["dependencies"][2]["name"], "zeta");
    assert_eq!(lock["dev_dependencies"][0]["name"], "mock-server");

    cmd_lock(&dir, true).expect("check lock");

    let mut stale = lock;
    stale["dependencies"][0]["version"] = serde_json::json!("9.9.9");
    write_json_atomic(&dir.join("orv.lock"), &stale).expect("write stale lock");
    let err = cmd_lock(&dir, true).expect_err("stale lock");
    assert!(err.to_string().contains("orv.lock is out of date"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn fetch_writes_dependency_source_bundles_from_lockfile() {
    let dir = temp_output_dir("project-fetch");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::create_dir_all(dir.join("libs/ui/src")).expect("create path dep src");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.3/src")).expect("create registry dep src");
    std::fs::write(dir.join("src/main.orv"), "@out \"fetch\"\n").expect("write entry");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = { version = "1.2.3", registry = "registry" }
ui = { version = "0.1.0", path = "libs/ui" }
"#,
    )
    .expect("write manifest");
    std::fs::write(
        dir.join("libs/ui/orv.toml"),
        r#"[project]
name = "ui"
version = "0.1.0"
entry = "src/main.orv"
"#,
    )
    .expect("write path dep manifest");
    std::fs::write(
        dir.join("libs/ui/src/main.orv"),
        r#"@out @html { @body { @p "UI" } }"#,
    )
    .expect("write path dep source");
    std::fs::write(
        dir.join("registry/auth/1.2.3/orv.toml"),
        r#"[project]
name = "auth"
version = "1.2.3"
entry = "src/main.orv"
"#,
    )
    .expect("write registry dep manifest");
    std::fs::write(
        dir.join("registry/auth/1.2.3/src/main.orv"),
        r#"@out @html { @body { @p "Auth" } }"#,
    )
    .expect("write registry dep source");
    cmd_lock(&dir, false).expect("write lock");

    let out = dir.join("target/orv-deps");
    cmd_fetch(&dir, &out).expect("fetch dependencies");

    assert!(out
        .join("packages/dependencies/auth/source-bundle.json")
        .is_file());
    assert!(out
        .join("packages/dependencies/ui/source-bundle.json")
        .is_file());
    let manifest = read_json_value(&out.join("deps-manifest.json")).expect("read manifest");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["kind"], "orv.dependencies");
    assert_eq!(manifest["lockfile"], "orv.lock");
    assert_eq!(manifest["stats"]["package_count"], 2);
    assert!(manifest["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .any(|package| package["name"] == "auth"
            && package["source"] == "registry"
            && package["source_bundle"] == "packages/dependencies/auth/source-bundle.json"
            && package["verified"] == true));
    assert!(manifest["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .any(|package| package["name"] == "ui"
            && package["source"] == "path"
            && package["source_bundle"] == "packages/dependencies/ui/source-bundle.json"
            && package["verified"] == true));
    read_source_bundle_artifact(&out.join("packages/dependencies/auth/source-bundle.json"))
        .expect("auth source bundle");
    read_source_bundle_artifact(&out.join("packages/dependencies/ui/source-bundle.json"))
        .expect("ui source bundle");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn fetch_downloads_dependency_source_bundle_from_http_registry() {
    let dir = temp_output_dir("project-fetch-http");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::write(dir.join("src/main.orv"), "@out \"fetch-http\"\n").expect("write entry");
    let bundle = orv_compiler::source_bundle_artifact(
        "registry/auth/1.2.3/src/main.orv",
        [(
            "registry/auth/1.2.3/src/main.orv",
            r#"@out @html { @body { @p "Auth" } }"#,
        )],
    );
    let body = serde_json::to_vec_pretty(&serde_json::to_value(&bundle).expect("bundle json"))
        .expect("bundle bytes");
    let (registry, handle) = spawn_one_shot_http_json("/auth/1.2.3/source-bundle.json", body);
    std::fs::write(
            dir.join("orv.toml"),
            format!(
                "[project]\nname = \"shop\"\nversion = \"0.1.0\"\nentry = \"src/main.orv\"\n\n[dependencies]\nauth = {{ version = \"1.2.3\", registry = \"{registry}\" }}\n"
            ),
        )
        .expect("write manifest");
    cmd_lock(&dir, false).expect("write lock");

    let out = dir.join("target/orv-deps");
    cmd_fetch(&dir, &out).expect("fetch dependencies");
    handle.join().expect("registry served request");

    let manifest = read_json_value(&out.join("deps-manifest.json")).expect("read manifest");
    assert!(manifest["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .any(|package| package["name"] == "auth"
            && package["source"] == "registry"
            && package["resolved_url"] == format!("{registry}/auth/1.2.3/source-bundle.json")
            && package["source_bundle"] == "packages/dependencies/auth/source-bundle.json"));
    let downloaded =
        read_source_bundle_artifact(&out.join("packages/dependencies/auth/source-bundle.json"))
            .expect("downloaded source bundle");
    assert_eq!(downloaded.entry, "registry/auth/1.2.3/src/main.orv");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn fetch_sends_bearer_token_for_authenticated_http_registry() {
    let dir = temp_output_dir("project-fetch-http-auth");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::write(dir.join("src/main.orv"), "@out \"fetch-http-auth\"\n").expect("write entry");
    let bundle = orv_compiler::source_bundle_artifact(
        "registry/auth/1.2.3/src/main.orv",
        [(
            "registry/auth/1.2.3/src/main.orv",
            r#"@out @html { @body { @p "Auth" } }"#,
        )],
    );
    let body = serde_json::to_vec_pretty(&serde_json::to_value(&bundle).expect("bundle json"))
        .expect("bundle bytes");
    let (registry, handle) = spawn_one_shot_http_json_with_auth(
        "/auth/1.2.3/source-bundle.json",
        body,
        "Bearer orv-test-token",
    );
    std::env::set_var("ORV_TEST_REGISTRY_TOKEN_AUTH_FETCH", "orv-test-token");
    std::fs::write(
            dir.join("orv.toml"),
            format!(
                "[project]\nname = \"shop\"\nversion = \"0.1.0\"\nentry = \"src/main.orv\"\n\n[dependencies]\nauth = {{ version = \"1.2.3\", registry = \"{registry}\", auth_token_env = \"ORV_TEST_REGISTRY_TOKEN_AUTH_FETCH\" }}\n"
            ),
        )
        .expect("write manifest");
    cmd_lock(&dir, false).expect("write lock");

    let out = dir.join("target/orv-deps");
    cmd_fetch(&dir, &out).expect("fetch dependencies");
    handle.join().expect("registry served request");
    std::env::remove_var("ORV_TEST_REGISTRY_TOKEN_AUTH_FETCH");

    let manifest = read_json_value(&out.join("deps-manifest.json")).expect("read manifest");
    assert!(manifest["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .any(|package| package["name"] == "auth"
            && package["source"] == "registry"
            && package["auth_token_env"] == "ORV_TEST_REGISTRY_TOKEN_AUTH_FETCH"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_resolves_caret_version_from_local_registry_index() {
    let dir = temp_output_dir("project-lock-registry-index");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.0/src")).expect("create 1.2.0");
    std::fs::create_dir_all(dir.join("registry/auth/1.3.0/src")).expect("create 1.3.0");
    std::fs::create_dir_all(dir.join("registry/auth/2.0.0/src")).expect("create 2.0.0");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-index\"\n").expect("write entry");
    std::fs::write(
        dir.join("registry/auth/index.json"),
        r#"{"versions":["1.2.0","1.3.0","2.0.0"]}"#,
    )
    .expect("write index");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = { version = "^1.2.0", registry = "registry" }
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.3.0");
    assert_eq!(lock["dependencies"][0]["requested_version"], "^1.2.0");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_sends_bearer_token_for_authenticated_http_registry_index() {
    let dir = temp_output_dir("project-lock-http-auth-index");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-http-auth\"\n").expect("write entry");
    let (registry, handle) = spawn_one_shot_http_json_with_auth(
        "/auth/index.json",
        br#"{"versions":["1.2.0","1.3.0"]}"#.to_vec(),
        "Bearer orv-index-token",
    );
    std::env::set_var("ORV_TEST_REGISTRY_TOKEN_AUTH_INDEX", "orv-index-token");
    std::fs::write(
            dir.join("orv.toml"),
            format!(
                "[project]\nname = \"shop\"\nversion = \"0.1.0\"\nentry = \"src/main.orv\"\n\n[dependencies]\nauth = {{ version = \"^1.2.0\", registry = \"{registry}\", auth_token_env = \"ORV_TEST_REGISTRY_TOKEN_AUTH_INDEX\" }}\n"
            ),
        )
        .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");
    handle.join().expect("registry served request");
    std::env::remove_var("ORV_TEST_REGISTRY_TOKEN_AUTH_INDEX");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.3.0");
    assert_eq!(
        lock["dependencies"][0]["auth_token_env"],
        "ORV_TEST_REGISTRY_TOKEN_AUTH_INDEX"
    );
    assert_eq!(lock["dependencies"][0]["requested_version"], "^1.2.0");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn registry_index_uses_https_transport_instead_of_roadmap_error() {
    let error = registry_index_versions(Path::new("."), "auth", "https://127.0.0.1:9", None)
        .expect_err("unreachable https registry");

    assert!(!error.to_string().contains("not implemented"), "{error}");
}

#[test]
fn registry_fetch_uses_https_transport_instead_of_roadmap_error() {
    let dependency = serde_json::json!({
        "name": "auth",
        "section": "dependencies",
        "source": "registry",
        "registry": "https://127.0.0.1:9",
        "version": "1.2.3",
        "checksum": "fnv1a64:test",
    });
    let Err(error) = registry_dependency_source(Path::new("."), &dependency) else {
        panic!("unreachable https registry unexpectedly succeeded");
    };

    assert!(!error.to_string().contains("not implemented"), "{error}");
}

#[test]
fn lock_resolves_tilde_version_from_local_registry_index() {
    let dir = temp_output_dir("project-lock-registry-tilde");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.0/src")).expect("create 1.2.0");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.9/src")).expect("create 1.2.9");
    std::fs::create_dir_all(dir.join("registry/auth/1.3.0/src")).expect("create 1.3.0");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-tilde\"\n").expect("write entry");
    std::fs::write(
        dir.join("registry/auth/index.json"),
        r#"{"versions":["1.2.0","1.2.9","1.3.0"]}"#,
    )
    .expect("write index");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = { version = "~1.2.0", registry = "registry" }
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.2.9");
    assert_eq!(lock["dependencies"][0]["requested_version"], "~1.2.0");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_resolves_segment_wildcard_versions_from_local_registry_index() {
    let dir = temp_output_dir("project-lock-registry-wildcard");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.0/src")).expect("create auth 1.2.0");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.9/src")).expect("create auth 1.2.9");
    std::fs::create_dir_all(dir.join("registry/auth/1.3.0/src")).expect("create auth 1.3.0");
    std::fs::create_dir_all(dir.join("registry/ui/1.0.0/src")).expect("create ui 1.0.0");
    std::fs::create_dir_all(dir.join("registry/ui/1.4.0/src")).expect("create ui 1.4.0");
    std::fs::create_dir_all(dir.join("registry/ui/2.0.0/src")).expect("create ui 2.0.0");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-wildcard\"\n").expect("write entry");
    std::fs::write(
        dir.join("registry/auth/index.json"),
        r#"{"versions":["1.2.0","1.2.9","1.3.0"]}"#,
    )
    .expect("write auth index");
    std::fs::write(
        dir.join("registry/ui/index.json"),
        r#"{"versions":["1.0.0","1.4.0","2.0.0"]}"#,
    )
    .expect("write ui index");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = { version = "1.2.*", registry = "registry" }
ui = { version = "1.*", registry = "registry" }
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.2.9");
    assert_eq!(lock["dependencies"][0]["requested_version"], "1.2.*");
    assert_eq!(lock["dependencies"][1]["name"], "ui");
    assert_eq!(lock["dependencies"][1]["version"], "1.4.0");
    assert_eq!(lock["dependencies"][1]["requested_version"], "1.*");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_resolves_compound_comparator_version_from_local_registry_index() {
    let dir = temp_output_dir("project-lock-registry-comparator");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.0/src")).expect("create 1.2.0");
    std::fs::create_dir_all(dir.join("registry/auth/1.9.0/src")).expect("create 1.9.0");
    std::fs::create_dir_all(dir.join("registry/auth/2.0.0/src")).expect("create 2.0.0");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-comparator\"\n").expect("write entry");
    std::fs::write(
        dir.join("registry/auth/index.json"),
        r#"{"versions":["1.2.0","1.9.0","2.0.0"]}"#,
    )
    .expect("write index");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = { version = ">=1.2.0 <2.0.0", registry = "registry" }
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.9.0");
    assert_eq!(
        lock["dependencies"][0]["requested_version"],
        ">=1.2.0 <2.0.0"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_preserves_exact_version_with_build_metadata() {
    let dir = temp_output_dir("project-lock-registry-build-metadata");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-build\"\n").expect("write entry");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = "1.2.3+build.7"
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.2.3+build.7");
    assert!(lock["dependencies"][0].get("requested_version").is_none());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_resolves_prerelease_comparator_version_from_local_registry_index() {
    let dir = temp_output_dir("project-lock-registry-prerelease");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.0-alpha.1/src")).expect("create alpha.1");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.0-alpha.2/src")).expect("create alpha.2");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.0/src")).expect("create 1.2.0");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-prerelease\"\n").expect("write entry");
    std::fs::write(
        dir.join("registry/auth/index.json"),
        r#"{"versions":["1.2.0-alpha.1","1.2.0-alpha.2","1.2.0"]}"#,
    )
    .expect("write index");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = { version = ">=1.2.0-alpha.1 <1.2.0", registry = "registry" }
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "1.2.0-alpha.2");
    assert_eq!(
        lock["dependencies"][0]["requested_version"],
        ">=1.2.0-alpha.1 <1.2.0"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lock_resolves_disjunction_version_from_local_registry_index() {
    let dir = temp_output_dir("project-lock-registry-disjunction");
    std::fs::create_dir_all(dir.join("src")).expect("create project src");
    std::fs::create_dir_all(dir.join("registry/auth/1.2.4/src")).expect("create 1.2.4");
    std::fs::create_dir_all(dir.join("registry/auth/1.3.0/src")).expect("create 1.3.0");
    std::fs::create_dir_all(dir.join("registry/auth/2.1.0/src")).expect("create 2.1.0");
    std::fs::create_dir_all(dir.join("registry/auth/3.0.0/src")).expect("create 3.0.0");
    std::fs::write(dir.join("src/main.orv"), "@out \"lock-disjunction\"\n").expect("write entry");
    std::fs::write(
        dir.join("registry/auth/index.json"),
        r#"{"versions":["1.2.4","1.3.0","2.1.0","3.0.0"]}"#,
    )
    .expect("write index");
    std::fs::write(
        dir.join("orv.toml"),
        r#"[project]
name = "shop"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
auth = { version = ">=1.2.0 <1.3.0 || >=2.0.0 <3.0.0", registry = "registry" }
"#,
    )
    .expect("write manifest");

    cmd_lock(&dir, false).expect("write lock");

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dependencies"][0]["version"], "2.1.0");
    assert_eq!(
        lock["dependencies"][0]["requested_version"],
        ">=1.2.0 <1.3.0 || >=2.0.0 <3.0.0"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn add_and_remove_update_manifest_and_lockfile() {
    let dir = temp_output_dir("project-add-remove");
    std::fs::create_dir_all(dir.join("src")).expect("create src");
    std::fs::write(dir.join("src").join("main.orv"), "@out \"deps\"\n").expect("write entry");
    std::fs::write(
        dir.join("orv.toml"),
        "[project]\nname = \"shop\"\nversion = \"0.1.0\"\nentry = \"src/main.orv\"\n",
    )
    .expect("write manifest");

    cmd_add_dependency(
        &dir,
        "auth",
        Some("1.2.3"),
        false,
        None,
        Some("https://registry.orv.dev"),
    )
    .expect("add registry dependency");
    cmd_add_dependency(
        &dir,
        "ui",
        Some("0.1.0"),
        true,
        Some(Path::new("libs/ui")),
        None,
    )
    .expect("add path dev dependency");

    let manifest = std::fs::read_to_string(dir.join("orv.toml")).expect("read manifest");
    let manifest = toml::from_str::<toml::Value>(&manifest).expect("parse manifest");
    assert_eq!(
        manifest["dependencies"]["auth"]["version"].as_str(),
        Some("1.2.3")
    );
    assert_eq!(
        manifest["dependencies"]["auth"]["registry"].as_str(),
        Some("https://registry.orv.dev")
    );
    assert_eq!(
        manifest["dev-dependencies"]["ui"]["path"].as_str(),
        Some("libs/ui")
    );

    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert_eq!(lock["dependencies"][0]["name"], "auth");
    assert_eq!(lock["dev_dependencies"][0]["name"], "ui");

    cmd_remove_dependency(&dir, "auth", false).expect("remove registry dependency");

    let manifest = std::fs::read_to_string(dir.join("orv.toml")).expect("read manifest");
    let manifest = toml::from_str::<toml::Value>(&manifest).expect("parse manifest");
    assert!(manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .is_none_or(toml::map::Map::is_empty));
    assert_eq!(
        manifest["dev-dependencies"]["ui"]["version"].as_str(),
        Some("0.1.0")
    );
    let lock = read_json_value(&dir.join("orv.lock")).expect("read lock");
    assert!(lock["dependencies"].as_array().is_some_and(Vec::is_empty));
    assert_eq!(lock["dev_dependencies"][0]["name"], "ui");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn workspace_new_updates_root_manifest_and_creates_member_project() {
    let root = temp_output_dir("workspace-new");
    std::fs::create_dir_all(&root).expect("create workspace root");

    cmd_workspace_new(
        &root,
        Path::new("apps/web"),
        Some("web"),
        InitTemplate::Basic,
    )
    .expect("workspace new");

    let root_manifest = std::fs::read_to_string(root.join("orv.toml")).expect("read root manifest");
    let root_manifest = toml::from_str::<toml::Value>(&root_manifest).expect("parse root");
    assert_eq!(root_manifest["workspace"]["resolver"].as_str(), Some("2"));
    assert_eq!(
        root_manifest["workspace"]["members"][0].as_str(),
        Some("apps/web")
    );

    let member_manifest =
        std::fs::read_to_string(root.join("apps/web/orv.toml")).expect("read member manifest");
    let member_manifest = toml::from_str::<toml::Value>(&member_manifest).expect("parse member");
    assert_eq!(member_manifest["project"]["name"].as_str(), Some("web"));
    assert_eq!(
        member_manifest["project"]["entry"].as_str(),
        Some("src/main.orv")
    );
    assert!(root.join("apps/web/src/main.orv").is_file());

    cmd_workspace_new(
        &root,
        Path::new("shared/models"),
        Some("models"),
        InitTemplate::Basic,
    )
    .expect("workspace new second member");
    let root_manifest = std::fs::read_to_string(root.join("orv.toml")).expect("read root manifest");
    let root_manifest = toml::from_str::<toml::Value>(&root_manifest).expect("parse root");
    let members = root_manifest["workspace"]["members"]
        .as_array()
        .expect("members");
    assert_eq!(members.len(), 2);
    assert!(members
        .iter()
        .any(|member| member.as_str() == Some("apps/web")));
    assert!(members
        .iter()
        .any(|member| member.as_str() == Some("shared/models")));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_graph_merges_member_graphs_and_path_dependency_edges() {
    let root = temp_output_dir("workspace-graph");
    std::fs::create_dir_all(root.join("apps/web/src")).expect("create web src");
    std::fs::create_dir_all(root.join("shared/models/src")).expect("create models src");
    std::fs::write(
        root.join("orv.toml"),
        r#"[workspace]
resolver = "2"
members = ["apps/web", "shared/models"]
"#,
    )
    .expect("write root manifest");
    std::fs::write(
        root.join("apps/web/orv.toml"),
        r#"[project]
name = "web"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
models = { path = "../../shared/models", version = "0.1.0" }
"#,
    )
    .expect("write web manifest");
    std::fs::write(
        root.join("shared/models/orv.toml"),
        r#"[project]
name = "models"
version = "0.1.0"
entry = "src/main.orv"
"#,
    )
    .expect("write models manifest");
    std::fs::write(
        root.join("apps/web/src/main.orv"),
        "@server { @route GET / { @respond 200 { ok: true } } }\n",
    )
    .expect("write web source");
    std::fs::write(
        root.join("shared/models/src/main.orv"),
        "pub struct User { id: int, name: string }\n",
    )
    .expect("write models source");

    let graph = workspace_graph_json(&root).expect("workspace graph");

    assert_eq!(graph["schema_version"], 1);
    assert_eq!(graph["kind"], "orv.workspace.graph");
    assert_eq!(graph["resolver"], "2");
    assert_eq!(graph["stats"]["member_count"], 2);
    let members = graph["members"].as_array().expect("members");
    assert!(members
        .iter()
        .any(|member| member["path"] == "apps/web" && member["name"] == "web"));
    assert!(members
        .iter()
        .any(|member| member["path"] == "shared/models"
            && member["graph"]["nodes"]
                .as_array()
                .expect("nodes")
                .iter()
                .any(|node| node["kind"] == "struct" && node["name"] == "User")));
    assert!(graph["edges"]
        .as_array()
        .expect("workspace edges")
        .iter()
        .any(|edge| edge["kind"] == "path_dependency"
            && edge["from"] == "apps/web"
            && edge["to"] == "shared/models"
            && edge["package"] == "models"
            && edge["requested_version"] == "0.1.0"
            && edge["target_name"] == "models"
            && edge["target_version"] == "0.1.0"
            && edge["version_match"] == true));

    let out = root.join("target/orv-workspace");
    cmd_workspace_graph(&root, Some(&out), false).expect("write workspace graph");
    assert!(out.join("workspace-graph.json").is_file());
    let written = read_json_value(&out.join("workspace-graph.json")).expect("read written");
    assert_eq!(written["stats"]["member_count"], 2);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_graph_view_writes_static_html_artifact() {
    let root = workspace_build_fixture("workspace-graph-view");
    let out = root.join("target/orv-workspace-view");

    cmd_workspace_graph(&root, Some(&out), true).expect("write workspace graph view");

    let graph = read_json_value(&out.join("workspace-graph.json")).expect("read graph");
    assert_eq!(graph["kind"], "orv.workspace.graph");
    let html = std::fs::read_to_string(out.join("index.html")).expect("workspace html");
    assert!(html.contains("ORV Workspace Graph"));
    assert!(html.contains("data-member-count=\"2\""));
    assert!(html.contains("workspace-graph.json"));
    assert!(html.contains("apps/web"));
    assert!(html.contains("shared/models"));
    assert!(html.contains("path_dependency"));
    assert!(html.contains("id=\"workspace-search\""));
    assert!(html.contains("data-workspace-member-row"));
    assert!(html.contains("data-workspace-edge-row"));
    assert!(html.contains("filterWorkspaceGraphRows"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_graph_rejects_member_path_dependency_version_mismatch() {
    let root = workspace_build_fixture("workspace-graph-version-mismatch");
    std::fs::write(
        root.join("apps/web/orv.toml"),
        r#"[project]
name = "web"
version = "0.1.0"
entry = "src/main.orv"

[dependencies]
models = { path = "../../shared/models", version = "2.0.0" }
"#,
    )
    .expect("write mismatched web manifest");

    let error = workspace_graph_json(&root).expect_err("version mismatch");
    assert!(error.to_string().contains(
            "workspace dependency apps/web -> shared/models requests `2.0.0` but target version is `0.1.0`"
        ));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_build_writes_member_builds_and_workspace_manifest() {
    let root = workspace_build_fixture("workspace-build");
    let out = root.join("target/orv-workspace-build");
    cmd_workspace_build(&root, &out, BuildProfile::Development, false).expect("workspace build");

    assert!(out.join("workspace-graph.json").is_file());
    assert!(out.join("members/apps/web/build-manifest.json").is_file());
    assert!(out
        .join("members/shared/models/build-manifest.json")
        .is_file());
    let manifest = read_json_value(&out.join("workspace-build.json")).expect("read manifest");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["kind"], "orv.workspace.build");
    assert_eq!(manifest["profile"], "dev");
    assert_eq!(manifest["stats"]["member_count"], 2);
    assert_eq!(manifest["workspace_graph"], "workspace-graph.json");
    assert_eq!(
        manifest["build_order"],
        serde_json::json!(["shared/models", "apps/web"])
    );
    let member_paths = manifest["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| member["path"].as_str().expect("member path"))
        .collect::<Vec<_>>();
    assert_eq!(member_paths, ["shared/models", "apps/web"]);
    assert!(manifest["members"]
        .as_array()
        .expect("members")
        .iter()
        .any(|member| member["path"] == "apps/web"
            && member["build_dir"] == "members/apps/web"
            && member["manifest"] == "members/apps/web/build-manifest.json"));
    assert!(manifest["dependency_edges"]
        .as_array()
        .expect("dependency edges")
        .iter()
        .any(|edge| edge["kind"] == "path_dependency"
            && edge["from"] == "apps/web"
            && edge["to"] == "shared/models"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_lock_writes_member_locks_and_workspace_manifest() {
    let root = workspace_build_fixture("workspace-lock");
    let out = root.join("target/orv-workspace-lock");
    cmd_workspace_lock(&root, &out).expect("workspace lock");

    assert!(out.join("workspace-graph.json").is_file());
    assert!(out.join("workspace-lock.json").is_file());
    assert!(out.join("members/shared/models/orv.lock").is_file());
    assert!(out.join("members/apps/web/orv.lock").is_file());
    let manifest = read_json_value(&out.join("workspace-lock.json")).expect("read lock");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["kind"], "orv.workspace.lock");
    assert_eq!(manifest["stats"]["member_count"], 2);
    assert_eq!(
        manifest["lock_order"],
        serde_json::json!(["shared/models", "apps/web"])
    );
    assert!(manifest["members"]
        .as_array()
        .expect("members")
        .iter()
        .any(|member| member["path"] == "apps/web"
            && member["lockfile"] == "members/apps/web/orv.lock"
            && member["dependencies"][0]["source"] == "path"
            && member["dependencies"][0]["path"] == "../../shared/models"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_fetch_writes_member_dependency_caches() {
    let root = workspace_build_fixture("workspace-fetch");
    let out = root.join("target/orv-workspace-fetch");
    cmd_workspace_fetch(&root, &out).expect("workspace fetch");

    assert!(out.join("workspace-graph.json").is_file());
    assert!(out.join("workspace-lock.json").is_file());
    assert!(out.join("workspace-fetch.json").is_file());
    assert!(out
        .join("members/apps/web/deps/deps-manifest.json")
        .is_file());
    assert!(out
        .join("members/apps/web/deps/packages/dependencies/models/source-bundle.json")
        .is_file());
    assert!(out
        .join("members/shared/models/deps/deps-manifest.json")
        .is_file());
    let manifest = read_json_value(&out.join("workspace-fetch.json")).expect("read fetch");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["kind"], "orv.workspace.dependencies");
    assert_eq!(manifest["stats"]["member_count"], 2);
    assert_eq!(manifest["stats"]["package_count"], 1);
    assert_eq!(
        manifest["fetch_order"],
        serde_json::json!(["shared/models", "apps/web"])
    );
    assert!(manifest["members"]
        .as_array()
        .expect("members")
        .iter()
        .any(|member| member["path"] == "apps/web"
            && member["deps_manifest"] == "members/apps/web/deps/deps-manifest.json"
            && member["package_count"] == 1));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn workspace_build_incremental_skips_unchanged_member_builds() {
    let root = workspace_build_fixture("workspace-build-incremental");
    let out = root.join("target/orv-workspace-build");
    cmd_workspace_build(&root, &out, BuildProfile::Development, false)
        .expect("initial workspace build");

    cmd_workspace_build(&root, &out, BuildProfile::Development, true)
        .expect("incremental workspace build");

    let manifest = read_json_value(&out.join("workspace-build.json")).expect("read manifest");
    assert_eq!(manifest["stats"]["built_count"], 0);
    assert_eq!(manifest["stats"]["skipped_count"], 2);
    assert!(manifest["members"]
        .as_array()
        .expect("members")
        .iter()
        .all(|member| member["status"] == "skipped"
            && member["input_hash"]
                .as_str()
                .is_some_and(|hash| hash.starts_with("fnv1a64:"))));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn dev_hmr_writes_session_manifest_for_client_page() {
    let out = temp_output_dir("dev-hmr-session");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();
    let canonical_entry = std::fs::canonicalize(&entry).expect("canonical entry");

    dev_with_writer_with_options(&entry, &build_out, true, false, &mut stdout).expect("dev hmr");

    let session =
        read_json_value(&build_out.join("dev").join("session.json")).expect("dev session");
    assert_eq!(session["schema_version"], 1);
    assert_eq!(session["mode"], "hmr");
    assert_eq!(session["source_bundle"], "source-bundle.json");
    assert_eq!(session["reload"]["strategy"], "hot-reload");
    assert_eq!(session["reload"]["fallback"], "full-reload");
    assert!(session["watch"]["sources"]
        .as_array()
        .expect("watch sources")
        .iter()
        .any(|source| {
            source["path"] == canonical_entry.display().to_string()
                && source["content_hash"]
                    .as_str()
                    .is_some_and(|hash| hash.starts_with("fnv1a64:"))
        }));
    assert!(session["watch"]["targets"]
        .as_array()
        .expect("watch targets")
        .iter()
        .any(|target| {
            target["kind"] == "client_wasm"
                && target["path"] == "client/app.wasm"
                && target["runtime_features"]
                    .as_array()
                    .expect("runtime features")
                    .iter()
                    .any(|feature| feature == "client_wasm")
        }));
    let transport =
        read_json_value(&build_out.join("dev").join("transport.json")).expect("hmr transport");
    assert_eq!(transport["schema_version"], 1);
    assert_eq!(transport["mode"], "hmr-transport");
    assert_eq!(transport["source_bundle"], "source-bundle.json");
    assert_eq!(transport["session"], "dev/session.json");
    assert_eq!(transport["browser"]["kind"], "event-source");
    assert_eq!(transport["browser"]["client"], "dev/hmr-client.js");
    assert_eq!(transport["browser"]["event_source"], "/__orv/hmr/events");
    assert_eq!(transport["browser"]["session"], "/__orv/hmr/session");
    assert_eq!(transport["server"]["kind"], "reference-dev");
    assert_eq!(transport["server"]["events"], "dev/events.json");
    let client =
        std::fs::read_to_string(build_out.join("dev").join("hmr-client.js")).expect("hmr client");
    assert!(client.contains("EventSource('/__orv/hmr/events')"));
    assert!(client.contains("window.location.reload()"));
    cmd_verify_build(&build_out).expect("verify dev hmr build");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn dev_watch_writes_watch_session_manifest() {
    let out = temp_output_dir("dev-watch-session");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, "@out @html { @body { @h1 \"Watch\" } }").expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();
    let canonical_entry = std::fs::canonicalize(&entry).expect("canonical entry");

    dev_with_writer_with_options(&entry, &build_out, false, true, &mut stdout).expect("dev watch");

    let watch = read_json_value(&build_out.join("dev").join("watch.json")).expect("watch session");
    assert_eq!(watch["schema_version"], 1);
    assert_eq!(watch["mode"], "watch");
    assert_eq!(watch["source_bundle"], "source-bundle.json");
    assert_eq!(watch["loop"]["strategy"], "poll");
    assert_eq!(watch["loop"]["run"], "build-verify-run");
    assert_eq!(watch["reload"]["strategy"], "full-reload");
    assert!(watch["watch"]["sources"]
        .as_array()
        .expect("watch sources")
        .iter()
        .any(|source| {
            source["path"] == canonical_entry.display().to_string()
                && source["content_hash"]
                    .as_str()
                    .is_some_and(|hash| hash.starts_with("fnv1a64:"))
        }));
    assert!(watch["watch"]["targets"]
        .as_array()
        .expect("watch targets")
        .iter()
        .any(|target| target["kind"] == "static_page" && target["path"] == "pages/index.html"));
    cmd_verify_build(&build_out).expect("verify dev watch build");
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn dev_watch_loop_writes_bounded_event_manifest() {
    let out = temp_output_dir("dev-watch-loop");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, "@out @html { @body { @h1 \"Loop\" } }").expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_watch_loop_with_writer(&entry, &build_out, false, Some(2), 1, &mut stdout)
        .expect("dev watch loop");

    let events = read_json_value(&build_out.join("dev").join("events.json")).expect("watch events");
    assert_eq!(events["schema_version"], 1);
    assert_eq!(events["mode"], "watch-loop");
    assert_eq!(events["loop"]["strategy"], "poll");
    assert_eq!(events["loop"]["run"], "build-verify-run");
    assert_eq!(events["loop"]["interval_ms"], 1);
    assert_eq!(events["transport"]["path"], "dev/events.json");
    assert_eq!(events["events"][0]["iteration"], 1);
    assert_eq!(events["events"][0]["reason"], "initial");
    assert_eq!(events["events"][0]["action"], "build-verify-run");
    assert_eq!(events["events"][0]["status"], "ok");
    assert!(events["events"][0]["source_signature"]
        .as_str()
        .is_some_and(|signature| !signature.is_empty()));
    assert_eq!(events["events"][1]["iteration"], 2);
    assert_eq!(events["events"][1]["reason"], "unchanged");
    assert_eq!(events["events"][1]["action"], "skip");
    assert_eq!(events["events"][1]["status"], "ok");
    assert!(events["events"][1]["source_signature"].is_null());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout utf-8"),
        "<html><body><h1>Loop</h1></body></html>"
    );
    cmd_verify_build(&build_out).expect("verify dev watch loop build");
    let _ = std::fs::remove_dir_all(&out);
}
