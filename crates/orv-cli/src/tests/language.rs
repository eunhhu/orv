use super::*;

#[test]
fn plan_pressure_fixtures_declare_contract_badges() {
    let mut fixtures = orv_files_under(&["fixtures", "plan"]);
    fixtures.push(workspace_path(&["fixtures", "default-syntax.orv"]));
    fixtures.push(workspace_path(&["fixtures", "e2e", "domains.orv"]));
    fixtures.sort();

    for path in fixtures {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let header = source.lines().take(12).collect::<Vec<_>>().join("\n");
        for badge in ["status:", "contract:", "milestone:", "purpose:"] {
            assert!(
                header.contains(badge),
                "{} missing plan fixture badge `{badge}` in first 12 lines",
                path.display()
            );
        }
    }
}

#[test]
fn client_wasm_i32_const_uses_signed_leb128_boundaries() {
    let mut body = Vec::new();
    push_wasm_const_i32_function(&mut body, 64);
    assert_eq!(body, [0x05, 0x00, 0x41, 0xc0, 0x00, 0x0b]);

    body.clear();
    push_wasm_const_i32_function(&mut body, 127);
    assert_eq!(body, [0x05, 0x00, 0x41, 0xff, 0x00, 0x0b]);

    body.clear();
    push_wasm_const_i32_function(&mut body, 128);
    assert_eq!(body, [0x05, 0x00, 0x41, 0x80, 0x01, 0x0b]);
}

#[test]
fn rendered_diagnostics_use_span_file_source() {
    let files = vec![
        orv_project::SourceFile {
            id: FileId(0),
            path: PathBuf::from("main.orv"),
            source: "import models.user.User\nlet u: User = { name: \"ok\" }\n".to_string(),
        },
        orv_project::SourceFile {
            id: FileId(1),
            path: PathBuf::from("models/user.orv"),
            source: "pub struct User { name: string }\nlet bad: int = \"wrong\"\n".to_string(),
        },
    ];
    let start = u32::try_from(files[1].source.find("\"wrong\"").unwrap()).expect("offset fits u32");
    let len = u32::try_from("\"wrong\"".len()).expect("length fits u32");
    let diag = orv_diagnostics::Diagnostic::error(
        "type mismatch: `bad` annotated as `int` but value has type `string`",
    )
    .with_primary(
        orv_diagnostics::Span::new(
            FileId(1),
            orv_diagnostics::ByteRange::new(start, start + len),
        ),
        "value has type `string`",
    );

    let rendered = render_diagnostics_for_test(&[diag], &files);
    assert!(rendered.contains("models/user.orv"), "{rendered}");
    assert!(rendered.contains("let bad: int = \"wrong\""), "{rendered}");
    assert!(
        !rendered.contains("let u: User = { name: \"ok\" }"),
        "{rendered}"
    );
}

#[test]
fn rendered_diagnostics_use_secondary_span_file_source() {
    let files = vec![
        orv_project::SourceFile {
            id: FileId(0),
            path: PathBuf::from("main.orv"),
            source: "import models.user.User\nlet user: User = make_user()\n".to_string(),
        },
        orv_project::SourceFile {
            id: FileId(1),
            path: PathBuf::from("models/user.orv"),
            source:
                "pub struct User { id: int }\nfunction make_user(): User -> { id: \"wrong\" }\n"
                    .to_string(),
        },
    ];
    let primary_start =
        u32::try_from(files[0].source.find("make_user").unwrap()).expect("primary offset fits u32");
    let secondary_start = u32::try_from(files[1].source.find("\"wrong\"").unwrap())
        .expect("secondary offset fits u32");
    let string_literal_len = u32::try_from("\"wrong\"".len()).expect("length fits u32");
    let diag = orv_diagnostics::Diagnostic::error("type mismatch across imported constructor")
        .with_primary(
            orv_diagnostics::Span::new(
                FileId(0),
                orv_diagnostics::ByteRange::new(primary_start, primary_start + 9),
            ),
            "constructor call",
        )
        .with_secondary(
            orv_diagnostics::Span::new(
                FileId(1),
                orv_diagnostics::ByteRange::new(
                    secondary_start,
                    secondary_start + string_literal_len,
                ),
            ),
            "field value has type `string`",
        );

    let rendered = render_diagnostics_for_test(&[diag], &files);
    assert!(rendered.contains("main.orv"), "{rendered}");
    assert!(
        rendered.contains("let user: User = make_user()"),
        "{rendered}"
    );
    assert!(rendered.contains("models/user.orv"), "{rendered}");
    assert!(
        rendered.contains("function make_user(): User -> { id: \"wrong\" }"),
        "{rendered}"
    );
    assert!(
        rendered.contains("field value has type `string`"),
        "{rendered}"
    );
}

#[test]
fn project_diagnostics_render_imported_file_source() {
    let dir = temp_output_dir("imported-diagnostic-source");
    let models = dir.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let entry = dir.join("main.orv");
    let imported = models.join("user.orv");
    std::fs::write(&entry, "import models.user.User\nlet ok: int = 1\n").expect("write entry");
    std::fs::write(
        &imported,
        "pub struct User { id: int }\nlet bad: int = \"wrong\"\n",
    )
    .expect("write imported");
    let loaded = orv_project::load_project(&entry).expect("load project");
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let mut diagnostics = Vec::new();
    diagnostics.extend(loaded.diagnostics.clone());
    diagnostics.extend(resolved.diagnostics);
    diagnostics.extend(lowered.diagnostics);

    let rendered = render_diagnostics_for_test(&diagnostics, &loaded.files);

    assert!(rendered.contains("models/user.orv"), "{rendered}");
    assert!(rendered.contains("let bad: int = \"wrong\""), "{rendered}");
    assert!(!rendered.contains("let ok: int = 1"), "{rendered}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn project_diagnostics_report_unknown_route_param_source() {
    let dir = temp_output_dir("unknown-route-param-diagnostic-source");
    std::fs::create_dir_all(&dir).expect("create source dir");
    let entry = dir.join("main.orv");
    std::fs::write(
        &entry,
        r#"@server {
  @listen 8080
  @route GET /users/:id {
    @respond 200 { name: @param.name }
  }
}
"#,
    )
    .expect("write entry");
    let loaded = orv_project::load_project(&entry).expect("load project");
    let resolved = orv_resolve::resolve(&loaded.program);
    let lowered = orv_analyzer::lower_with_diagnostics(&loaded.program, &resolved);
    let rendered = render_diagnostics_for_test(&lowered.diagnostics, &loaded.files);

    assert!(
        rendered.contains("unknown route param `name`"),
        "{rendered}"
    );
    assert!(rendered.contains("declared route params: id"), "{rendered}");
    assert!(
        rendered.contains("@respond 200 { name: @param.name }"),
        "{rendered}"
    );
    let _ = std::fs::remove_dir_all(dir);
}
