#[cfg(unix)]
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime};

mod usage_body_support;
use usage_body_support::{CASES, Scope, assert_body, assert_case};

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
    assert_body(&stderr, None);
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
        assert!(stderr.contains("Commands:"), "{stderr}");
        assert!(
            !stderr.contains("Options:"),
            "missing command must not dump full help: {stderr}"
        );
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
    for case in CASES {
        let stderr = usage_failure(&run(directory.path(), case.arguments));
        assert_case(&stderr, case);
    }
    assert!(fs::read_dir(directory.path()).unwrap().next().is_none());
}

#[test]
fn family_contract_accepts_approved_blocks_and_rejects_the_previous_mixed_templates() {
    let add = "Error: Add requires at least one source.\n\n  Usage: hydragen add [OPTIONS] <SOURCE>...\n  Example: hydragen add ../shared-config\n\n  For more information, try 'hydragen add --help'.\n";
    let init = "Error: Invalid value 'yam' for '--format <FORMAT>'.\n\n  Possible values: yaml, yml, toml, json, jsonc\n  Tip: Did you mean 'yaml'?\n\n  Usage: hydragen init [OPTIONS]\n  Example: hydragen init --format yaml\n\n  For more information, try 'hydragen init --help'.\n";
    assert_body(add, Some(Scope::Add));
    assert_body(init, Some(Scope::Init));
    for invalid in [
        add.replace("source.\n", "source\n"),
        add.replace("\n  Example:", "\n\n  Example:"),
        add.replace("\n\n", "\n\n\n"),
        add.replace("  Usage:", " Usage:"),
        add.replace("'hydragen add --help'", "'--help'"),
        add.replace("Example: hydragen add", "Example: hydragen init"),
        init.replace("Tip:", "tip:"),
        init.replacen("Error: Invalid", "Error: invalid", 1),
        init.replace(
            "  Possible values: yaml, yml, toml, json, jsonc\n  Tip: Did you mean 'yaml'?",
            "  Tip: Did you mean 'yaml'?\n  Possible values: yaml, yml, toml, json, jsonc",
        ),
        init.replace(
            "Possible values: yaml, yml, toml, json, jsonc",
            "[possible values: yaml, yml, toml, json, jsonc]",
        ),
        init.replace(
            "Possible values: yaml, yml, toml, json, jsonc",
            "Possible values:",
        ),
        init.replace("'hydragen init --help'", "'hydragen --help'"),
    ] {
        assert!(
            std::panic::catch_unwind(|| assert_body(&invalid, None)).is_err(),
            "family assertion accepted a mixed/invalid template: {invalid:?}"
        );
    }
}

#[test]
fn help_and_usage_scope_follow_the_error_not_later_commands_or_argument_values() {
    let directory = tempfile::tempdir().unwrap();
    let before = snapshot(directory.path());
    for (arguments, scope, facts) in [
        (vec!["--bogus", "init"], Scope::Root, vec!["--bogus"]),
        (vec!["--nme", "init"], Scope::Root, vec!["--nme"]),
        (
            vec!["--non-interactive", "--bogus", "add"],
            Scope::Root,
            vec!["--bogus"],
        ),
        (vec!["init", "--bogus", "add"], Scope::Init, vec!["--bogus"]),
        (
            vec!["init", "--format", "add"],
            Scope::Init,
            vec!["'add'", "--format", "Possible values:"],
        ),
        (
            vec!["--non-interactive", "init", "--format", "add"],
            Scope::Init,
            vec!["'add'", "--format"],
        ),
        (
            vec!["init", "--non-interactive", "--format", "add"],
            Scope::Init,
            vec!["'add'", "--format"],
        ),
        (
            vec!["init", "--format", "add", "--non-interactive"],
            Scope::Init,
            vec!["'add'", "--format"],
        ),
        (
            vec!["--non-interactive", "init", "--name"],
            Scope::Init,
            vec!["--name"],
        ),
        (
            vec!["init", "--name", "init", "--name", "add"],
            Scope::Init,
            vec!["--name"],
        ),
        (vec!["add", "--bogus", "init"], Scope::Add, vec!["--bogus"]),
        (
            vec!["--non-interactive", "add"],
            Scope::Add,
            vec!["at least one source"],
        ),
        (
            vec!["add", "--non-interactive"],
            Scope::Add,
            vec!["at least one source"],
        ),
    ] {
        let text = usage_failure(&run(directory.path(), &arguments));
        assert_body(&text, Some(scope));
        for fact in facts {
            assert!(text.contains(fact), "{arguments:?}: {text:?}");
        }
        assert_eq!(snapshot(directory.path()), before);
    }
}

#[test]
fn command_words_and_option_separators_still_reach_unchanged_operational_values() {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app");
    fs::create_dir(&app).unwrap();
    let output = run(
        &app,
        &[
            "init",
            "--name",
            "app",
            "--author",
            "init",
            "--description",
            "add",
            "--format",
            "json",
        ],
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let initial: serde_json::Value =
        serde_json::from_slice(&fs::read(app.join("layer.json")).unwrap()).unwrap();
    assert_eq!(initial["author"], "init");
    assert_eq!(initial["description"], "add");
    for command_word in ["init", "add", "help"] {
        let target = app.join(command_word);
        fs::create_dir(&target).unwrap();
        fs::write(target.join("layer.yaml"), format!("name: {command_word}\n")).unwrap();
    }
    let output = run(&app, &["add", "--", "./init", "./add", "./help"]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let updated: serde_json::Value =
        serde_json::from_slice(&fs::read(app.join("layer.json")).unwrap()).unwrap();
    assert_eq!(
        updated["dependencies"],
        serde_json::json!(["./init", "./add", "./help"])
    );
    let before = snapshot(&app);
    for word in ["init", "add", "help", "--format"] {
        let output = run(&app, &["add", "--", word]);
        assert_eq!(output.status.code(), Some(1), "{word}: {output:?}");
        let text = safe_text(&output.stderr);
        assert!(text.contains(word), "argument was not preserved: {text:?}");
        assert!(
            text.contains("source argument"),
            "must be an operational source failure: {text:?}"
        );
        assert!(!text.contains("Usage:"));
        assert!(output.stdout.is_empty());
        assert_eq!(snapshot(&app), before);
    }
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
    for arguments in [
        vec!["add", "../shared-config"],
        vec!["add", "test"],
        vec!["--non-interactive", "add", "test"],
        vec!["add", "test", "--non-interactive"],
    ] {
        let output = run(directory.path(), &arguments);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let stderr = safe_text(&output.stderr);
        assert_eq!(
            stderr,
            concat!(
                "Error: No layer manifest found in this directory or its parents.\n  \n",
                "  Tip: Run this command from an existing layer directory, or run\n",
                "  'hydragen init' in the directory you want to use as a layer.\n  \n",
                "  No files were changed.\n"
            )
        );
        assert!(!stderr.contains(&directory.path().display().to_string()));
        assert!(output.stdout.is_empty());
        assert_eq!(snapshot(directory.path()), before);
    }
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
            assert!(stderr.contains("Tip:"), "{stderr:?}");
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
