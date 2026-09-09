//! Build each fixture once; restore its artifacts between independent drift cases.

use super::{cmd_build_with_profile, cmd_verify_build, read_json_value, write_json, BuildProfile};
use std::path::{Path, PathBuf};

pub(super) struct ArtifactCase {
    name: &'static str,
    check: ArtifactCheck,
}

enum ArtifactCheck {
    Json {
        artifact: &'static str,
        expected: &'static str,
        mutate: fn(&mut serde_json::Value),
    },
    Custom(fn(&Path)),
}

pub(super) const fn json_case(
    name: &'static str,
    artifact: &'static str,
    expected: &'static str,
    mutate: fn(&mut serde_json::Value),
) -> ArtifactCase {
    ArtifactCase {
        name,
        check: ArtifactCheck::Json {
            artifact,
            expected,
            mutate,
        },
    }
}

pub(super) const fn artifact_case(name: &'static str, check: fn(&Path)) -> ArtifactCase {
    ArtifactCase {
        name,
        check: ArtifactCheck::Custom(check),
    }
}

struct FixtureDirectory(PathBuf);

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct ArtifactFile {
    path: PathBuf,
    bytes: Vec<u8>,
    permissions: std::fs::Permissions,
}

fn snapshot_files(directory: &Path, files: &mut Vec<ArtifactFile>) {
    for entry in std::fs::read_dir(directory).expect("read fixture directory") {
        let path = entry.expect("fixture entry").path();
        if path.is_dir() {
            snapshot_files(&path, files);
        } else {
            files.push(ArtifactFile {
                bytes: std::fs::read(&path).expect("snapshot fixture"),
                permissions: std::fs::metadata(&path)
                    .expect("fixture metadata")
                    .permissions(),
                path,
            });
        }
    }
}

pub(super) fn verify_artifact_cases(
    name: &str,
    source: impl FnOnce(&str) -> (PathBuf, PathBuf),
    profile: BuildProfile,
    cases: &[ArtifactCase],
) {
    let (directory, entry) = source(name);
    let directory = FixtureDirectory(directory);
    let out = directory.0.join("dist");
    cmd_build_with_profile(&entry, &out, profile).expect("build shared fixture");
    cmd_verify_build(&out).expect("unmodified fixture must pass");
    let mut files = Vec::new();
    snapshot_files(&out, &mut files);
    let mut failures = Vec::new();

    for case in cases {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match case.check {
            ArtifactCheck::Json {
                artifact,
                expected,
                mutate,
            } => reject_json_change(&out, artifact, expected, mutate),
            ArtifactCheck::Custom(check) => check(&out),
        }));
        // Always restore before running the next case, including when a check fails.
        for file in &files {
            if std::fs::read(&file.path).ok().as_deref() != Some(file.bytes.as_slice()) {
                std::fs::write(&file.path, &file.bytes).expect("restore fixture contents");
            }
            std::fs::set_permissions(&file.path, file.permissions.clone())
                .expect("restore fixture permissions");
        }
        if let Err(error) = result {
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("test panicked");
            failures.push(format!("{}: {message}", case.name));
        }
    }
    cmd_verify_build(&out).expect("fixture must remain valid after all cases");
    assert!(
        failures.is_empty(),
        "artifact cases failed:\n{}",
        failures.join("\n")
    );
}

fn reject_json_change(
    out: &Path,
    artifact: &str,
    expected: &str,
    mutate: impl FnOnce(&mut serde_json::Value),
) {
    let path = out.join(artifact);
    let mut value = read_json_value(&path).expect("fixture JSON");
    mutate(&mut value);
    write_json(&path, &value).expect("write changed JSON");
    let error = cmd_verify_build(out).expect_err("changed artifact must be rejected");
    assert!(
        error.to_string().contains(expected),
        "expected {expected}; got {error}"
    );
}

pub(super) fn source_fixture(name: &str, source: &str) -> (PathBuf, PathBuf) {
    let directory = super::temp_output_dir(name);
    std::fs::create_dir_all(&directory).expect("create source directory");
    let entry = directory.join("page.orv");
    std::fs::write(&entry, source).expect("write source");
    (directory, entry)
}
