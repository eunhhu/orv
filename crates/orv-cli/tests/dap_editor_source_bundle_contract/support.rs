//! Shared fixture/build/json helpers used by both the source-bundle contract
//! target and the summary-parity contract target. DAP stdio helpers that only
//! the source-bundle contract target consumes live in `dap_support.rs`.

pub use crate::support::{assert_success, orv_bin, read_json, run_orv_json, temp_dir};

use crate::support::run_orv;

use std::path::{Path, PathBuf};

use serde_json::Value;

pub const APP_SOURCE: &str = r"import models.user.user_id

let total: int = user_id()
@out total
";

pub const IMPORTED_SOURCE: &str = r"pub function user_id(): int -> 7
";

fn write_fixture(root: &Path) -> (PathBuf, PathBuf) {
    let models = root.join("models");
    std::fs::create_dir_all(&models).expect("create models dir");
    let app = root.join("app.orv");
    let imported = models.join("user.orv");
    std::fs::write(&app, APP_SOURCE).expect("write app source");
    std::fs::write(&imported, IMPORTED_SOURCE).expect("write imported source");
    (app, imported)
}

pub fn build_fixture(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (app, imported) = write_fixture(root);
    let out = root.join("dist");
    let app_arg = app.display().to_string();
    let out_arg = out.display().to_string();
    run_orv(&["build", &app_arg, "--out", &out_arg, "--prod"]);
    (app, imported, out)
}

pub fn assert_source_bundle_files(bundle: &Value) {
    let files = bundle["files"].as_array().expect("source-bundle files");
    assert_eq!(files.len(), 2, "source-bundle file count");
    assert!(files.iter().any(|file| file["path"]
        .as_str()
        .is_some_and(|path| path.ends_with("app.orv"))
        && file["source"] == APP_SOURCE));
    assert!(files.iter().any(|file| {
        file["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("models/user.orv"))
            && file["source"] == IMPORTED_SOURCE
    }));
}
