use super::*;

#[test]
fn verify_build_rejects_server_policy_origin_drift_from_origin_map() {
    let dir = temp_output_dir("server-policy-origin-source");
    std::fs::create_dir_all(&dir).expect("create policy origin source dir");
    let path = dir.join("app.orv");
    std::fs::write(
        &path,
        r#"@server {
  @listen 8080
  @route POST /checkout {
    @csrf
    @respond 201 { ok: true }
  }
}
"#,
    )
    .expect("write policy source");
    let out = temp_output_dir("server-policy-origin-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let artifact_path = out.join("server").join("app.orv-runtime.json");
    let mut artifact = read_json_value(&artifact_path).expect("server artifact");
    artifact["routes"][0]["policies"][1]["origin_id"] = serde_json::json!("ori_missing_policy");
    write_json(&artifact_path, &artifact).expect("write corrupt server artifact");

    let err = cmd_verify_build(&out).expect_err("policy origin mismatch");

    assert!(err.to_string().contains(
            "server route POST /checkout policy `csrf` origin_id `ori_missing_policy` not found in origin-map.json"
        ));
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_project_graph_stat_content_drift() {
    for key in [
        "file_count",
        "import_count",
        "declaration_count",
        "domain_count",
        "max_source_contains_depth",
        "max_semantic_contains_depth",
    ] {
        let fixture_name = format!("project-graph-stat-{}-source", key.replace('_', "-"));
        let out_name = format!("project-graph-stat-{}-drift", key.replace('_', "-"));
        let (src_dir, path) = prod_server_source(&fixture_name);
        let out = temp_output_dir(&out_name);

        cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
        let graph_path = out.join("project-graph.json");
        let mut graph = read_json_value(&graph_path).expect("project graph");
        let current = graph["stats"][key].as_u64().expect("stat value");
        graph["stats"][key] = serde_json::json!(current + 1);
        write_json(&graph_path, &graph).expect("write drifted project graph");

        let err = cmd_verify_build(&out).expect_err("project graph stat drift must fail");

        assert!(
            err.to_string().contains(&format!(
                "project-graph.json stats.{key} does not match graph content"
            )),
            "{key}: {err}"
        );
        let _ = std::fs::remove_dir_all(src_dir);
        let _ = std::fs::remove_dir_all(&out);
    }
}

#[test]
fn verify_build_rejects_project_graph_source_file_drift() {
    let (src_dir, path) = prod_server_source("project-graph-file-source");
    let out = temp_output_dir("project-graph-file-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let graph_path = out.join("project-graph.json");
    let mut graph = read_json_value(&graph_path).expect("project graph");
    graph["nodes"][0]["name"] = serde_json::json!("/tmp/wrong.orv");
    write_json(&graph_path, &graph).expect("write corrupt project graph");

    let err = cmd_verify_build(&out).expect_err("project graph source file drift");

    assert!(err
        .to_string()
        .contains("project graph file node name must match source-bundle file path"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_build_rejects_project_graph_file_node_source_bundle_index_drift() {
    let (src_dir, path) = imported_prod_server_source("project-graph-file-index-source");
    let out = temp_output_dir("project-graph-file-index-mismatch");

    cmd_build_with_profile(&path, &out, BuildProfile::Production).expect("prod build");
    let graph_path = out.join("project-graph.json");
    let mut graph = read_json_value(&graph_path).expect("project graph");
    let nodes = graph["nodes"].as_array_mut().expect("project graph nodes");
    let file_node_indexes = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (node["kind"] == "file").then_some(index))
        .collect::<Vec<_>>();
    assert!(
        file_node_indexes.len() >= 2,
        "expected imported build files"
    );
    let first = file_node_indexes[0];
    let second = file_node_indexes[1];
    let first_name = nodes[first]["name"].clone();
    nodes[first]["name"] = nodes[second]["name"].clone();
    nodes[second]["name"] = first_name;
    write_json(&graph_path, &graph).expect("write corrupt project graph");

    let err = cmd_verify_build(&out).expect_err("project graph file index drift");

    assert!(err
        .to_string()
        .contains("project graph file node name must match source-bundle file path"));
    let _ = std::fs::remove_dir_all(src_dir);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn verify_graph_artifact_cases() {
    verify_artifact_cases(
        "verify_graph_artifact_cases",
        prod_server_source,
        BuildProfile::Production,
        &[
            json_case(
                "origin_map_extra_root_key",
                "origin-map.json",
                "origin-map.json keys must match contract",
                |origin_map| {
                    origin_map["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "origin_map_extra_entry_key",
                "origin-map.json",
                "origin-map.json entries[0] keys must match contract",
                |origin_map| {
                    origin_map["entries"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "origin_map_entry_id_drift",
                "origin-map.json",
                "origin-map.json entry `ori_drift` id does not match fingerprint",
                |origin_map| {
                    origin_map["entries"][0]["id"] = serde_json::json!("ori_drift");
                },
            ),
            json_case(
                "origin_map_entry_fingerprint_drift",
                "origin-map.json",
                "fingerprint does not match span",
                |origin_map| {
                    origin_map["entries"][0]["fingerprint"] = serde_json::json!("0000000000000000");
                },
            ),
            json_case(
                "origin_map_extra_span_key",
                "origin-map.json",
                "origin-map.json entries[0].span keys must match contract",
                |origin_map| {
                    origin_map["entries"][0]["span"]["unexpected"] = serde_json::json!(1);
                },
            ),
            json_case(
                "origin_map_span_file_drift",
                "origin-map.json",
                "does not reference source-bundle file",
                |origin_map| {
                    origin_map["entries"][0]["span"]["file"] = serde_json::json!(99);
                    refresh_origin_map_entry_identity(origin_map, 0);
                },
            ),
            json_case(
                "origin_map_span_bounds_drift",
                "origin-map.json",
                "span.end exceeds source-bundle file length",
                |origin_map| {
                    origin_map["entries"][0]["span"]["end"] = serde_json::json!(10_000);
                    refresh_origin_map_entry_identity(origin_map, 0);
                },
            ),
            json_case(
                "origin_map_extra_edge_key",
                "origin-map.json",
                "origin-map.json edges[0] keys must match contract",
                |origin_map| {
                    origin_map["edges"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "origin_map_unsupported_edge_kind",
                "origin-map.json",
                "origin-map.json edge kind `drift` is not supported",
                |origin_map| {
                    origin_map["edges"][0]["kind"] = serde_json::json!("drift");
                },
            ),
            artifact_case("origin_map_duplicate_edge", |out| {
                let origin_map_path = out.join("origin-map.json");
                let mut origin_map = read_json_value(&origin_map_path).expect("origin map");
                let duplicate_edge = origin_map["edges"][0].clone();
                origin_map["edges"]
                    .as_array_mut()
                    .expect("origin map edges")
                    .push(duplicate_edge.clone());
                write_json(&origin_map_path, &origin_map).expect("write duplicate origin edge");

                let graph_path = out.join("project-graph.json");
                let mut graph = read_json_value(&graph_path).expect("project graph");
                graph["semantic"]["origin_map"] = origin_map;
                graph["semantic"]["origin_edges"]
                    .as_array_mut()
                    .expect("project graph origin edges")
                    .push(duplicate_edge);
                let semantic_edge_count = graph["stats"]["semantic_edge_count"]
                    .as_u64()
                    .expect("semantic edge count");
                graph["stats"]["semantic_edge_count"] = serde_json::json!(semantic_edge_count + 1);
                write_json(&graph_path, &graph).expect("write mirrored duplicate origin edge");

                let err = cmd_verify_build(out).expect_err("duplicate origin edge must fail");

                assert!(err
                    .to_string()
                    .contains("origin-map.json contains duplicate edge"));
            }),
            json_case(
                "origin_map_edge_from_missing_entry",
                "origin-map.json",
                "origin-map.json edge from `ori_missing_from` does not reference an entry",
                |origin_map| {
                    origin_map["edges"][0]["from"] = serde_json::json!("ori_missing_from");
                },
            ),
            json_case(
                "origin_map_edge_to_missing_entry",
                "origin-map.json",
                "origin-map.json edge to `ori_missing_to` does not reference an entry",
                |origin_map| {
                    origin_map["edges"][0]["to"] = serde_json::json!("ori_missing_to");
                },
            ),
            json_case(
                "server_listen_origin_missing_from_origin_map",
                "server/app.orv-runtime.json",
                "server listen origin_id `ori_missing_listen` not found in origin-map.json",
                |artifact| {
                    artifact["listen"]["origin_id"] = serde_json::json!("ori_missing_listen");
                },
            ),
            artifact_case("server_route_origin_missing_from_origin_map", |out| {
                let artifact_path = out.join("server").join("app.orv-runtime.json");
                let mut artifact = read_json_value(&artifact_path).expect("server artifact");
                artifact["routes"][0]["origin_id"] = serde_json::json!("ori_missing_route");
                write_json(&artifact_path, &artifact).expect("write corrupt server artifact");

                let err = cmd_verify_build(out).expect_err("route origin mismatch");

                assert!(err.to_string().contains(
        "server route GET /ping origin_id `ori_missing_route` not found in origin-map.json"
    ));
            }),
            artifact_case("server_response_origin_drift_from_origin_map", |out| {
                let artifact_path = out.join("server").join("app.orv-runtime.json");
                let mut artifact = read_json_value(&artifact_path).expect("server artifact");
                artifact["routes"][0]["response_origin_ids"][0] =
                    serde_json::json!("ori_missing_response");
                write_json(&artifact_path, &artifact).expect("write corrupt server artifact");

                let err = cmd_verify_build(out).expect_err("response origin mismatch");

                assert!(err.to_string().contains(
        "server route GET /ping response_origin_ids do not match origin-map contains edges"
    ));
            }),
            json_case(
                "project_graph_extra_root_key",
                "project-graph.json",
                "project-graph.json keys must match contract",
                |graph| {
                    graph["unexpected"] = serde_json::json!(true);
                },
            ),
            json_case(
                "project_graph_extra_stats_key",
                "project-graph.json",
                "project-graph.json stats keys must match contract",
                |graph| {
                    graph["stats"]["unexpected"] = serde_json::json!(1);
                },
            ),
            json_case(
                "project_graph_extra_node_key",
                "project-graph.json",
                "project graph node keys must match contract",
                |graph| {
                    graph["nodes"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "project_graph_extra_node_span_key",
                "project-graph.json",
                "project graph node span keys must match contract",
                |graph| {
                    graph["nodes"][0]["span"]["unexpected"] = serde_json::json!(1);
                },
            ),
            json_case(
                "project_graph_node_span_file_mismatch",
                "project-graph.json",
                "project graph node span.file must match node file",
                |graph| {
                    graph["nodes"][0]["span"]["file"] = serde_json::json!(99);
                },
            ),
            json_case(
                "project_graph_node_span_out_of_bounds",
                "project-graph.json",
                "project graph node span.end exceeds source-bundle file length",
                |graph| {
                    graph["nodes"][0]["span"]["end"] = serde_json::json!(u64::MAX);
                },
            ),
            json_case(
                "project_graph_extra_edge_key",
                "project-graph.json",
                "project graph edge keys must match contract",
                |graph| {
                    graph["edges"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "project_graph_extra_origin_link_key",
                "project-graph.json",
                "project graph origin link keys must match contract",
                |graph| {
                    graph["semantic"]["origin_links"][0]["unexpected"] = serde_json::json!("drift");
                },
            ),
            json_case(
                "project_graph_origin_link_kind_drift",
                "project-graph.json",
                "project graph origin link kind must be source_node",
                |graph| {
                    graph["semantic"]["origin_links"][0]["kind"] = serde_json::json!("wrong_kind");
                },
            ),
            artifact_case("project_graph_origin_link_missing_origin", |out| {
                let graph_path = out.join("project-graph.json");
                let mut graph = read_json_value(&graph_path).expect("project graph");
                graph["semantic"]["origin_links"][0]["origin_id"] =
                    serde_json::json!("ori_missing_link");
                write_json(&graph_path, &graph).expect("write corrupt project graph");

                let err =
                    cmd_verify_build(out).expect_err("project graph origin link missing origin");

                assert!(err.to_string().contains(
        "project-graph.json origin link `ori_missing_link` does not reference origin-map.json"
    ));
            }),
            json_case(
                "project_graph_origin_link_missing_node",
                "project-graph.json",
                "project-graph.json origin link node_id 999999 does not reference a node",
                |graph| {
                    graph["semantic"]["origin_links"][0]["node_id"] =
                        serde_json::json!(999_999_u64);
                },
            ),
            json_case(
                "project_graph_semantic_origin_drift",
                "project-graph.json",
                "project-graph.json semantic origin_map does not match origin-map.json",
                |graph| {
                    graph["semantic"]["origin_map"]["entries"][0]["id"] =
                        serde_json::json!("ori_wrong");
                },
            ),
            artifact_case("project_graph_origin_link_drift", |out| {
                let graph_path = out.join("project-graph.json");
                let mut graph = read_json_value(&graph_path).expect("project graph");
                graph["semantic"]["origin_links"] = serde_json::json!([]);
                write_json(&graph_path, &graph).expect("write corrupt project graph");

                let err = cmd_verify_build(out).expect_err("project graph origin link drift");

                assert!(err.to_string().contains(
        "project-graph.json semantic origin_links do not match graph nodes and origin-map.json"
    ));
            }),
        ],
    );
}
