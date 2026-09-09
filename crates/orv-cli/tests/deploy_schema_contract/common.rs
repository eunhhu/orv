pub use crate::support::{assert_keys, read_json, run_orv, temp_dir as temp_output_dir};
use std::path::{Path, PathBuf};

pub fn write_prod_server_fixture(out: &Path) -> PathBuf {
    let fixture = out.join("app.orv");
    std::fs::write(
        &fixture,
        r#"@server {
  @listen 8080

  @route GET /ping {
    @respond 200 { ok: true, msg: "pong" }
  }
}
"#,
    )
    .expect("write fixture");
    fixture
}
