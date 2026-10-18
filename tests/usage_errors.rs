#[cfg(unix)]
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime};

const FILENAMES: [&str; 5] = [
    "layer.yaml",
    "layer.yml",
    "layer.toml",
    "layer.json",
    "layer.jsonc",
];

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn command(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hydragen"));
    command.current_dir(directory);
    for key in [
        "CI",
        "NO_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
    ] {
        command.env_remove(key);
    }
    command.env("TERM", "xterm-256color");
    // Captured streams must stay plain even when colour is forced.
    command.env("CLICOLOR_FORCE", "1");
    command.env("FORCE_COLOR", "1");
    command
}

fn capture(mut command: Command) -> Output {
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let stderr = tempfile::NamedTempFile::new().unwrap();
    let mut child = Running(
        command
            .stdin(Stdio::piped())
            .stdout(stdout.reopen().unwrap())
            .stderr(stderr.reopen().unwrap())
            .spawn()
            .unwrap(),
    );
    // Keep an empty input pipe open: attempting a prompt/read cannot reach EOF
    // and must fail the bounded wait rather than hang the test process.
    let _live_input = child.0.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "usage/help child awaited stdin");
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: fs::read(stdout.path()).unwrap(),
        stderr: fs::read(stderr.path()).unwrap(),
    }
}

fn run(directory: &Path, arguments: &[&str]) -> Output {
    let mut command = command(directory);
    command.args(arguments);
    capture(command)
}

fn safe_text(bytes: &[u8]) -> String {
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(
        !text.chars().any(|character| {
            (character.is_control() && character != '\n')
                || matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        }),
        "raw diagnostic controls: {text:?}"
    );
    text
}

fn usage_failure(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = safe_text(&output.stderr);
    assert!(stderr.starts_with("Error:"), "{stderr:?}");
    assert_eq!(stderr.matches("Error:").count(), 1, "{stderr:?}");
    assert!(!stderr.contains('✕'));
    assert_no_operation(&stderr);
    stderr
}

fn assert_no_operation(text: &str) {
    for forbidden in [
        "Warning:",
        "Could not add dependencies",
        "Layer initialised",
        "Hydragen",
        "Author (optional",
        "No files were changed",
    ] {
        assert!(!text.contains(forbidden), "{forbidden:?}: {text:?}");
    }
    for frame in ['┌', '│', '└', '◇', '◆', '⠀'] {
        assert!(!text.contains(frame), "{text:?}");
    }
}

#[derive(Debug, PartialEq)]
struct Entry {
    path: PathBuf,
    modified: SystemTime,
    length: u64,
    readonly: bool,
    bytes: Option<Vec<u8>>,
    link: Option<PathBuf>,
    #[cfg(unix)]
    identity: (u64, u64, u32),
}

fn snapshot(directory: &Path) -> Vec<Entry> {
    fn visit(path: &Path, entries: &mut Vec<Entry>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        entries.push(Entry {
            path: path.to_owned(),
            modified: metadata.modified().unwrap(),
            length: metadata.len(),
            readonly: metadata.permissions().readonly(),
            bytes: metadata.is_file().then(|| fs::read(path).unwrap()),
            link: metadata
                .file_type()
                .is_symlink()
                .then(|| fs::read_link(path).unwrap()),
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino(), metadata.mode()),
        });
        if metadata.is_dir() {
            let mut children = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                visit(&child, entries);
            }
        }
    }
    let mut entries = Vec::new();
    visit(directory, &mut entries);
    entries
}

#[test]
fn missing_root_command_is_an_error_not_successful_explicit_help() {
    let directory = tempfile::tempdir().unwrap();
    for arguments in [vec![], vec!["--non-interactive"]] {
        let stderr = usage_failure(&run(directory.path(), &arguments));
        assert!(stderr.to_lowercase().contains("command"), "{stderr}");
        assert!(stderr.contains("Usage: hydragen"), "{stderr}");
        assert!(stderr.contains("init"));
        assert!(stderr.contains("add"));
        if arguments.is_empty() {
            assert!(
                stderr.contains("Commands:"),
                "root pseudo-help lost its layout: {stderr}"
            );
        }
    }
    assert!(fs::read_dir(directory.path()).unwrap().next().is_none());
}

#[test]
fn missing_add_source_has_one_actionable_example_not_full_help() {
    let directory = tempfile::tempdir().unwrap();
    for arguments in [
        vec!["add"],
        vec!["--non-interactive", "add"],
        vec!["add", "--non-interactive"],
    ] {
        let stderr = usage_failure(&run(directory.path(), &arguments));
        assert!(
            stderr.starts_with("Error: Add requires at least one source.\n"),
            "{stderr}"
        );
        assert!(stderr.contains("Usage: hydragen add"));
        assert!(stderr.contains("<SOURCE>..."));
        assert_eq!(stderr.matches("hydragen add ../shared-config").count(), 1);
        assert_eq!(stderr.matches("Example").count(), 1);
        assert!(stderr.contains("hydragen add --help"));
        for full_help_section in ["Sources:", "Behaviour:", "Options:", "Examples:"] {
            assert!(!stderr.contains(full_help_section), "{stderr}");
        }
        assert!(!stderr.contains("layer manifest"));
    }
}

#[test]
fn parser_failures_across_root_init_and_add_retain_clap_context() {
    let directory = tempfile::tempdir().unwrap();
    for (arguments, expected) in [
        (vec!["--bogus"], vec!["--bogus", "Usage:", "--help"]),
        (vec!["inti"], vec!["inti", "init", "similar", "Usage:"]),
        (
            vec!["init", "--nme", "rules"],
            vec!["--nme", "--name", "similar", "Usage:"],
        ),
        (
            vec!["add", "--non-interactiv"],
            vec!["--non-interactiv", "--non-interactive", "similar", "Usage:"],
        ),
        (
            vec!["add", "--path", "../rules"],
            vec!["--path", "Usage:", "--help"],
        ),
        (
            vec!["init", "--format", "yam"],
            vec![
                "--format", "yam", "yaml", "yml", "toml", "json", "jsonc", "similar",
            ],
        ),
        (
            vec!["init", "--format", "xml"],
            vec!["--format", "xml", "yaml", "yml", "toml", "json", "jsonc"],
        ),
        (
            vec!["init", "--name"],
            vec!["--name", "<NAME>", "value", "--help"],
        ),
        (
            vec!["init", "--author"],
            vec!["--author", "<AUTHOR>", "value", "--help"],
        ),
        (
            vec!["init", "--description"],
            vec!["--description", "<DESCRIPTION>", "value", "--help"],
        ),
        (
            vec!["init", "--format"],
            vec!["--format", "value", "yaml", "jsonc"],
        ),
        (
            vec!["init", "--name", "first", "--name", "second"],
            vec!["--name", "multiple times", "Usage:"],
        ),
    ] {
        let stderr = usage_failure(&run(directory.path(), &arguments));
        for phrase in expected {
            assert!(
                stderr.contains(phrase),
                "{arguments:?}: expected {phrase:?}: {stderr}"
            );
        }
    }
    assert!(fs::read_dir(directory.path()).unwrap().next().is_none());
}

fn exercise_bypass(directory: &Path) {
    let before = snapshot(directory);
    for arguments in [
        vec![],
        vec!["--non-interactive"],
        vec!["--bogus"],
        vec!["inti"],
        vec!["init", "--name"],
        vec!["init", "--format", "yam"],
        vec!["init", "--name", "one", "--name", "two"],
        vec!["add"],
        vec!["--non-interactive", "add"],
        vec!["add", "--non-interactive"],
        vec!["add", "--path", "../rules"],
    ] {
        usage_failure(&run(directory, &arguments));
        assert_eq!(snapshot(directory), before, "{arguments:?} changed files");
    }
    for arguments in explicit_help_and_version() {
        let output = run(directory, &arguments);
        assert_eq!(output.status.code(), Some(0), "{arguments:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{arguments:?}: {output:?}");
        let stdout = safe_text(&output.stdout);
        assert!(!stdout.is_empty());
        assert!(!stdout.contains("Error:"));
        assert_no_operation(&stdout);
        assert_eq!(snapshot(directory), before, "{arguments:?} changed files");
    }
}

fn explicit_help_and_version() -> Vec<Vec<&'static str>> {
    vec![
        vec!["-h"],
        vec!["--help"],
        vec!["help"],
        vec!["help", "init"],
        vec!["help", "add"],
        vec!["init", "-h"],
        vec!["init", "--help"],
        vec!["add", "-h"],
        vec!["add", "--help"],
        vec!["--non-interactive", "--help"],
        vec!["init", "--non-interactive", "--help"],
        vec!["add", "--non-interactive", "--help"],
        vec!["-V"],
        vec!["--version"],
        vec!["--non-interactive", "--version"],
    ]
}

#[test]
fn parsing_and_explicit_help_bypass_every_occupied_layer_manifest_filename() {
    for filename in FILENAMES {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            directory.path().join(filename),
            "invalid layer manifest\0[\n",
        )
        .unwrap();
        exercise_bypass(directory.path());
    }
}

#[test]
fn parsing_and_explicit_help_bypass_invalid_duplicates_and_directories() {
    let directory = tempfile::tempdir().unwrap();
    for filename in FILENAMES {
        fs::write(directory.path().join(filename), "[invalid\n").unwrap();
    }
    exercise_bypass(directory.path());
    let directories = tempfile::tempdir().unwrap();
    for filename in FILENAMES {
        fs::create_dir(directories.path().join(filename)).unwrap();
    }
    exercise_bypass(directories.path());
}

#[cfg(unix)]
#[test]
fn parsing_and_explicit_help_bypass_selected_and_dangling_symlinks() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("app");
    fs::create_dir(&directory).unwrap();
    let target = root.path().join("target");
    fs::write(&target, "name: target\n").unwrap();
    for filename in FILENAMES {
        symlink(
            if filename == "layer.yaml" {
                &target
            } else {
                Path::new("missing")
            },
            directory.join(filename),
        )
        .unwrap();
    }
    let before = snapshot(root.path());
    exercise_bypass(&directory);
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn valid_add_syntax_alone_reaches_missing_current_layer_discovery() {
    let directory = tempfile::tempdir().unwrap();
    let before = snapshot(directory.path());
    let invalid = usage_failure(&run(directory.path(), &["add"]));
    assert!(!invalid.contains("layer manifest"));
    let output = run(directory.path(), &["add", "../shared-config"]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = safe_text(&output.stderr);
    assert!(stderr.starts_with("Error:"));
    assert!(stderr.contains("layer manifest"), "{stderr}");
    assert!(!stderr.contains("Usage:"));
    assert!(!stderr.contains("requires at least one source"));
    assert!(output.stdout.is_empty());
    assert_eq!(snapshot(directory.path()), before);
}

#[test]
fn untrusted_argument_values_are_escaped_inside_headlines_and_suggestion_tips() {
    let directory = tempfile::tempdir().unwrap();
    let raw = "bad\u{1b}[2J\r\n\t\u{7f}\u{85}\u{9b}\u{202e}\u{2066}\u{2028}\u{2029}";
    let escaped = r"bad\x1b[2J\r\n\t\x7f\u{85}\u{9b}\u{202e}\u{2066}\u{2028}\u{2029}";
    for arguments in [
        vec![raw.to_owned()],
        vec![format!("--{raw}")],
        vec!["init".to_owned(), format!("--{raw}")],
        vec!["init".to_owned(), "--format".to_owned(), raw.to_owned()],
        vec!["add".to_owned(), format!("--{raw}")],
    ] {
        let mut command = command(directory.path());
        command.args(&arguments);
        let stderr = usage_failure(&capture(command));
        assert!(stderr.contains(escaped), "{arguments:?}: {stderr:?}");
        assert!(!stderr.contains(r"\\x1b"), "double escape: {stderr:?}");
        assert!(
            stderr.lines().count() > 1,
            "generated layout disappeared: {stderr:?}"
        );
        if arguments.first().is_some_and(|argument| argument == "add") {
            // clap copies this raw unknown option into its generated -- tip.
            assert!(stderr.contains("tip:"), "{stderr:?}");
            assert!(
                stderr.matches(escaped).count() >= 2,
                "suggestion lost raw leaf: {stderr:?}"
            );
        }
    }
}

#[test]
fn ordinary_unicode_and_literal_backslashes_are_not_rewritten() {
    let directory = tempfile::tempdir().unwrap();
    let value = r"配置🦀\n\x1b[2J";
    let stderr = usage_failure(&run(directory.path(), &["init", "--format", value]));
    assert!(stderr.contains(value), "{stderr:?}");
    assert!(!stderr.contains(r"\\n"));
    assert!(!stderr.contains(r"\\x1b"));
}

#[cfg(unix)]
#[test]
fn spoofed_argv_zero_never_supplies_help_or_usage_binary_name() {
    use std::os::unix::process::CommandExt;
    let directory = tempfile::tempdir().unwrap();
    let spoofed = "forged\u{1b}[2J\r\n\t\u{202e} executable";
    for arguments in [
        vec![],
        vec!["--bogus"],
        vec!["add"],
        vec!["--help"],
        vec!["init", "--help"],
        vec!["add", "--help"],
    ] {
        let mut command = command(directory.path());
        command.arg0(spoofed).args(&arguments);
        let output = capture(command);
        let text = if arguments.contains(&"--help") {
            assert!(output.status.success());
            assert!(output.stderr.is_empty());
            safe_text(&output.stdout)
        } else {
            usage_failure(&output)
        };
        assert!(text.contains("Usage: hydragen"), "{text:?}");
        assert!(!text.contains("forged"), "{text:?}");
    }
}

#[cfg(unix)]
#[test]
fn invalid_utf8_argument_remains_a_safe_usage_failure_not_a_process_panic() {
    use std::os::unix::ffi::OsStringExt;
    let directory = tempfile::tempdir().unwrap();
    let invalid = OsString::from_vec(vec![b'.', b'/', 0xff]);
    for prefix in [
        vec!["add"],
        vec!["init", "--name"],
        vec!["init", "--format"],
    ] {
        let mut command = command(directory.path());
        command.args(&prefix).arg(&invalid);
        let stderr = usage_failure(&capture(command));
        assert!(stderr.contains("UTF-8"), "{prefix:?}: {stderr}");
        assert!(!stderr.contains("panicked"));
    }
}

#[cfg(unix)]
#[test]
fn valid_control_containing_path_passes_parser_and_is_stored_byte_for_byte() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app");
    fs::create_dir(&app).unwrap();
    fs::write(app.join("layer.json"), r#"{"name":"app"}"#).unwrap();
    let reference = "../配置\\literal\u{1b}[2J\r\n\t\u{85}\u{202e}\u{2028}";
    let target = app.join(reference);
    fs::create_dir(&target).unwrap();
    fs::write(target.join("layer.yaml"), "name: rules\n").unwrap();
    let mut command = command(&app);
    command.args([OsStr::new("add"), OsStr::new(reference)]);
    let output = capture(command);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    safe_text(&output.stdout);
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(app.join("layer.json")).unwrap()).unwrap();
    assert_eq!(
        document["dependencies"][0].as_str().unwrap().as_bytes(),
        reference.as_bytes()
    );
}
