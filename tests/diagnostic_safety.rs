use std::fs;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use hydragen::layer_manifest::{LayerManifestError, LayerManifestFormat, edit::EditError, schema};
use serde_json::json;

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run(directory: &Path, arguments: &[&str]) -> Output {
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let stderr = tempfile::NamedTempFile::new().unwrap();
    let mut child = Running(
        Command::new(env!("CARGO_BIN_EXE_hydragen"))
            .current_dir(directory)
            .args(arguments)
            .env("CI", "1")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(stdout.reopen().unwrap())
            .stderr(stderr.reopen().unwrap())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "diagnostic safety child timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: fs::read(stdout.path()).unwrap(),
        stderr: fs::read(stderr.path()).unwrap(),
    }
}

fn safe_text(bytes: &[u8]) -> String {
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        !text
            .chars()
            .any(|character| character.is_control() && character != '\n'),
        "unsafe diagnostic: {text:?}"
    );
    for character in ['\u{2028}', '\u{2029}', '\u{202e}', '\u{2066}'] {
        assert!(!text.contains(character), "unsafe diagnostic: {text:?}");
    }
    text
}

fn layer(directory: &Path, name: &str) {
    fs::create_dir_all(directory).unwrap();
    fs::write(
        directory.join("layer.json"),
        json!({"name": name}).to_string(),
    )
    .unwrap();
}

#[test]
fn stored_missing_reference_escapes_heading_and_path_without_losing_layout() {
    let root = tempfile::tempdir().unwrap();
    let reference = "./missing\u{1b}[2J\u{1b}[Hforged-success\r\n\t\u{7f}\u{85}\u{202e}\u{2028}";
    let original = json!({"name": "app", "dependencies": [reference]}).to_string();
    fs::write(root.path().join("layer.json"), &original).unwrap();
    let output = run(root.path(), &["add", "../new"]);
    assert!(!output.status.success());
    let stderr = safe_text(&output.stderr);
    let escaped = r"./missing\x1b[2J\x1b[Hforged-success\r\n\t\x7f\u{85}\u{202e}\u{2028}";
    assert!(
        stderr.contains(&format!(
            "\n  Existing dependency '{escaped}'\n    Could not"
        )),
        "{stderr:?}"
    );
    assert_eq!(stderr.matches(escaped).count(), 2, "{stderr:?}");
    assert!(stderr.contains("\n  Source '../new'\n"));
    assert!(stderr.ends_with("\n  No files were changed.\n"));
    assert!(output.stdout.is_empty());
    assert_eq!(
        fs::read_to_string(root.path().join("layer.json")).unwrap(),
        original
    );
}

#[cfg(unix)]
#[test]
fn control_containing_native_target_resolves_and_is_stored_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app");
    layer(&app, "app");
    let reference = "../real\u{1b}[2J\r\n\t\u{7f}\u{85}\u{202e}\u{2028}";
    layer(&app.join(reference), "rules");
    let output = run(&app, &["add", reference]);
    assert!(
        output.status.success(),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(safe_text(&output.stdout), "Added rules to app.\n");
    assert!(safe_text(&output.stderr).is_empty());
    let stored = fs::read_to_string(app.join("layer.json")).unwrap();
    let document: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(
        document["dependencies"][0].as_str().unwrap().as_bytes(),
        reference.as_bytes()
    );
    let repeat = run(&app, &["add", reference]);
    assert!(
        repeat.status.success(),
        "{:?}",
        String::from_utf8_lossy(&repeat.stderr)
    );
    assert_eq!(
        safe_text(&repeat.stdout),
        "No changes needed: rules is already a dependency of app.\n"
    );
    assert!(safe_text(&repeat.stderr).is_empty());
    assert_eq!(fs::read_to_string(app.join("layer.json")).unwrap(), stored);
    layer(&root.path().join("second"), "second");
    let appended = run(&app, &["add", "../second"]);
    assert!(
        appended.status.success(),
        "{:?}",
        String::from_utf8_lossy(&appended.stderr)
    );
    safe_text(&appended.stdout);
    safe_text(&appended.stderr);
    let updated = fs::read_to_string(app.join("layer.json")).unwrap();
    assert!(
        updated.contains(&serde_json::to_string(reference).unwrap()),
        "{updated:?}"
    );
    let document: serde_json::Value = serde_json::from_str(&updated).unwrap();
    assert_eq!(document["dependencies"], json!([reference, "../second"]));
}

#[cfg(unix)]
#[test]
fn current_directory_duplicate_warnings_and_missing_paths_are_safe() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app\u{1b}[H\r\n\t");
    layer(&app, "app");
    fs::write(app.join("layer.yaml"), "name: app\n").unwrap();
    let output = run(&app, &["add", "./missing\u{1b}[2J\n"]);
    assert!(!output.status.success());
    let stderr = safe_text(&output.stderr);
    assert!(stderr.contains(r"app\x1b[H\r\n\t"), "{stderr:?}");
    assert!(stderr.contains(r"./missing\x1b[2J\n"), "{stderr:?}");
    assert_eq!(stderr.matches("Warning:").count(), 1);
    assert!(stderr.find("Warning:").unwrap() < stderr.find("Error:").unwrap());
    assert!(stderr.contains("layer.yaml, layer.json.\n\n  layer.yaml takes priority"));
    assert!(stderr.contains("Tip: Layer manifests are not merged"));
    assert!(
        !stderr.contains("Paths are relative"),
        "same-directory base is redundant: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn conflict_origins_and_both_directories_are_safe() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app");
    layer(&app, "app");
    let first = "../first\u{1b}[H\n";
    let second = "../second\r\t\u{9b}2J";
    layer(&app.join(first), "shared");
    layer(&app.join(second), "shared");
    let output = run(&app, &["add", first, second]);
    assert!(!output.status.success());
    let stderr = safe_text(&output.stderr);
    assert!(
        stderr.contains("\n  Conflicting source: Source '../first\\x1b[H\\n'\n"),
        "{stderr:?}"
    );
    assert!(stderr.contains("\n  Earlier source path:"));
    assert!(stderr.contains("\n  Requested source path:"));
    assert!(stderr.contains(r"second\r\t\u{9b}2J"));
}

#[cfg(unix)]
#[test]
fn edit_refusal_under_control_containing_directory_keeps_path_safe() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app\u{1b}[2J\n\t");
    fs::create_dir(&app).unwrap();
    let original = "name: app\ndependencies: &deps []\ncustom: *deps\n";
    fs::write(app.join("layer.yaml"), original).unwrap();
    layer(&root.path().join("rules"), "rules");
    let output = run(&app, &["add", "../rules"]);
    assert!(!output.status.success());
    let stderr = safe_text(&output.stderr);
    assert!(stderr.contains("Could not safely edit yaml layer manifest"));
    assert!(stderr.contains(r"app\x1b[2J\n\t/layer.yaml"), "{stderr:?}");
    assert!(stderr.contains("\n  No files were changed."));
    assert_eq!(
        fs::read_to_string(app.join("layer.yaml")).unwrap(),
        original
    );
}

#[test]
fn parser_errors_from_all_formats_cannot_create_terminal_commands_or_lines() {
    let root = tempfile::tempdir().unwrap();
    for (filename, source) in [
        ("layer.yaml", "name: app\ncustom: \"bad\u{1b}[2J\"\n"),
        ("layer.yml", "name: app\ncustom: \"bad\u{1b}[2J\"\n"),
        ("layer.toml", "name = 'app'\ncustom = \"bad\u{1b}[2J\"\n"),
        (
            "layer.json",
            "{\"name\":\"app\",\"custom\":\"bad\u{1b}[2J\"}",
        ),
        (
            "layer.jsonc",
            "{\"name\":\"app\",\"custom\":\"bad\u{1b}[2J\"}",
        ),
    ] {
        fs::write(root.path().join(filename), source).unwrap();
        let output = run(root.path(), &["add", "./missing"]);
        assert!(!output.status.success());
        let stderr = safe_text(&output.stderr);
        assert!(
            stderr.contains("Could not decode"),
            "{filename}: {stderr:?}"
        );
        assert!(!stderr.contains("\ncustom"), "{filename}: {stderr:?}");
        assert!(
            !stderr
                .lines()
                .any(|line| line.trim_start().starts_with("2 |")),
            "parser excerpt became an output line: {filename}: {stderr:?}"
        );
        fs::remove_file(root.path().join(filename)).unwrap();
    }
}

#[test]
fn nested_schema_parser_and_edit_errors_escape_only_raw_data_once() {
    let raw = "bad\u{1b}[2J\r\n\t\u{7f}\u{9b}\u{2029}";
    let escaped = r"bad\x1b[2J\r\n\t\x7f\u{9b}\u{2029}";
    let violation = schema::Violation {
        instance_path: raw.into(),
        schema_path: raw.into(),
        keyword: raw.into(),
        message: raw.into(),
        kind: schema::ViolationKind::Other,
    };
    let detail = schema::Error::Setup {
        message: raw.into(),
        detail: Some(Box::new(violation)),
    };
    let decode = LayerManifestError::Decode {
        path: raw.into(),
        format: LayerManifestFormat::Toml,
        message: raw.into(),
    };
    for error in [
        EditError::InvalidCandidate {
            path: raw.into(),
            format: LayerManifestFormat::Toml,
            source: decode,
        },
        EditError::InvalidCandidate {
            path: raw.into(),
            format: LayerManifestFormat::Json,
            source: LayerManifestError::Metadata {
                path: raw.into(),
                source: hydragen::MetadataError::Schema(detail),
            },
        },
        EditError::UnsupportedEdit {
            path: raw.into(),
            format: LayerManifestFormat::Yaml,
            reason: raw.into(),
        },
        EditError::UnintendedChange {
            path: raw.into(),
            format: LayerManifestFormat::Jsonc,
            reason: raw.into(),
        },
    ] {
        let message = safe_text(error.to_string().as_bytes());
        assert!(message.contains(escaped), "{message:?}");
        assert!(!message.contains("\\\\x1b"), "{message:?}");
        assert!(!message.contains('\n'), "{message:?}");
    }
}

#[cfg(unix)]
#[test]
fn init_directory_default_errors_are_safe_without_changing_name_validation() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("bad\u{1b}[H\r\n\t");
    fs::create_dir(&directory).unwrap();
    let output = run(&directory, &["init", "--format", "json"]);
    assert!(!output.status.success());
    let stderr = safe_text(&output.stderr);
    assert!(stderr.contains(r"bad\x1b[H\r\n\t"), "{stderr:?}");
    assert!(stderr.contains("Supply --name explicitly with a valid layer name"));
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
    let output = run(&directory, &["init", "--name", "valid", "--format", "json"]);
    assert!(
        output.status.success(),
        "{:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(safe_text(&output.stdout).contains("Layer initialised"));
    assert!(safe_text(&output.stderr).is_empty());
}

#[test]
fn operational_error_variants_escape_paths_and_external_io_details_without_io() {
    use hydragen::dependencies::{DependencyError, SourceOrigin};
    use hydragen::{EncodeError, InitError};
    use std::fmt::Display;
    use std::io;

    let raw = "path\u{1b}[H\r\n\t\u{7f}\u{9b}\u{202e}";
    let escaped = r"path\x1b[H\r\n\t\x7f\u{9b}\u{202e}";
    let errors: Vec<Box<dyn Display>> = vec![
        Box::new(SourceOrigin::Existing {
            index: 0,
            reference: raw.into(),
        }),
        Box::new(SourceOrigin::Argument {
            index: 0,
            reference: raw.into(),
        }),
        Box::new(DependencyError::Io {
            action: "inspect directory",
            path: raw.into(),
            source: io::Error::other(raw.to_owned()),
        }),
        Box::new(DependencyError::NotDirectory { path: raw.into() }),
        Box::new(DependencyError::MissingCurrent {
            invocation: raw.into(),
        }),
        Box::new(DependencyError::CurrentSymlink { path: raw.into() }),
        Box::new(DependencyError::UnsupportedPath {
            reference: raw.into(),
            reason: "not native",
        }),
        Box::new(DependencyError::NameConflict {
            name: "shared".into(),
            previous: SourceOrigin::Existing {
                index: 0,
                reference: raw.into(),
            },
            previous_directory: Some(raw.into()),
            directory: Some(raw.into()),
        }),
        Box::new(LayerManifestError::Missing {
            directory: raw.into(),
        }),
        Box::new(LayerManifestError::NotRegular { path: raw.into() }),
        Box::new(LayerManifestError::Io {
            action: "read layer manifest",
            path: raw.into(),
            source: io::Error::other(raw.to_owned()),
        }),
        Box::new(EncodeError::Serialization {
            format: LayerManifestFormat::Json,
            message: raw.into(),
        }),
        Box::new(InitError::NestedRepository { root: raw.into() }),
        Box::new(InitError::ExistingLayerManifest { path: raw.into() }),
        Box::new(InitError::Io {
            action: "inspect repository marker",
            path: raw.into(),
            source: io::Error::other(raw.to_owned()),
        }),
    ];
    for error in errors {
        let message = safe_text(error.to_string().as_bytes());
        assert!(message.contains(escaped), "{message:?}");
        assert!(!message.contains("\\\\x1b"), "{message:?}");
    }
}
