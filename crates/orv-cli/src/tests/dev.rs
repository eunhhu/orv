use super::*;

#[test]
fn dev_subcommand_is_accepted() {
    let parsed =
        Cli::try_parse_from(["orv", "dev", "src/main.orv", "--out", "target/orv-dev-test"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn dev_hmr_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "dev", "src/main.orv", "--hmr"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn dev_watch_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from(["orv", "dev", "src/main.orv", "--watch"]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn dev_watch_loop_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "dev",
        "src/main.orv",
        "--watch-loop",
        "--watch-iterations",
        "1",
    ]);
    if let Err(err) = parsed {
        panic!("{}", err.render());
    }
}

#[test]
fn dev_hmr_serve_subcommand_is_accepted() {
    let parsed = Cli::try_parse_from([
        "orv",
        "dev",
        "src/main.orv",
        "--hmr",
        "--serve",
        "--serve-port",
        "0",
        "--watch-iterations",
        "1",
    ])
    .unwrap_or_else(|err| panic!("{}", err.render()));
    let Command::Dev {
        serve, serve_port, ..
    } = parsed.command
    else {
        panic!("expected dev command");
    };
    assert!(serve);
    assert_eq!(serve_port, 0);
}

#[test]
fn dev_builds_verifies_and_runs_static_page() {
    let out = temp_output_dir("dev-static");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(&entry, r#"@out @html { @body { @h1 "Dev" } }"#).expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_with_writer(&entry, &build_out, &mut stdout).expect("dev");

    assert!(build_out.join("build-manifest.json").is_file());
    assert!(build_out.join("bundle-plan.json").is_file());
    assert_eq!(
        String::from_utf8(stdout).expect("stdout utf-8"),
        "<html><body><h1>Dev</h1></body></html>"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn dev_hmr_reference_server_serves_session_and_event_stream() {
    let out = temp_output_dir("dev-hmr-server");
    std::fs::create_dir_all(&out).expect("create temp root");
    let entry = out.join("page.orv");
    std::fs::write(
        &entry,
        "let sig count: int = 0\n@out @html { @body { @p count } }",
    )
    .expect("write entry");
    let build_out = out.join("dist");
    let mut stdout = Vec::new();

    dev_with_writer_with_options(&entry, &build_out, true, true, &mut stdout)
        .expect("dev hmr watch");
    write_dev_watch_events(
        &build_out,
        true,
        1,
        &[dev_watch_loop_event(
            1,
            "initial",
            "build-verify-run",
            "ok",
            Some("sig"),
        )],
    )
    .expect("write hmr events");
    let server = spawn_dev_hmr_server(&build_out, 0).expect("spawn hmr server");
    let address = server.addr().to_string();

    let manifest =
        read_json_value(&build_out.join("dev").join("server.json")).expect("server manifest");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["mode"], "hmr-server");
    assert_eq!(manifest["address"], address);
    assert_eq!(manifest["endpoints"]["session"], "/__orv/hmr/session");
    assert_eq!(manifest["endpoints"]["events"], "/__orv/hmr/events");

    let session_response = send_raw_http(&address, "/__orv/hmr/session");
    assert!(session_response.starts_with("HTTP/1.1 200 OK"));
    assert!(session_response.contains("Content-Type: application/json"));
    assert!(session_response.contains("\"mode\": \"hmr\""));

    let events_response = send_raw_http(&address, "/__orv/hmr/events");
    assert!(events_response.starts_with("HTTP/1.1 200 OK"));
    assert!(events_response.contains("Content-Type: text/event-stream"));
    assert!(events_response.contains("event: message"));
    assert!(events_response.contains("event: orv:reload"));
    assert!(events_response.contains("\"action\":\"build-verify-run\""));

    let missing_response = send_raw_http(&address, "/missing");
    assert!(missing_response.starts_with("HTTP/1.1 404 Not Found"));

    cmd_verify_build(&build_out).expect("verify dev hmr server build");
    drop(server);
    let _ = std::fs::remove_dir_all(&out);
}
