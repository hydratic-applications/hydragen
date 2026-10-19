use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use hydragen::LayerManifestFormat;
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

const PROJECT_FILES: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"existing-project","author":{"name":"Package author canary"}}"#,
    ),
    (
        "Cargo.toml",
        "[package]\nname = 'existing-project'\nauthors = ['Cargo author canary <cargo@example.invalid>']\n",
    ),
    (
        "pyproject.toml",
        "[project]\nname = 'existing-project'\nauthors = [{name = 'Python author canary'}]\n",
    ),
    (
        "README.md",
        "Existing project documentation must remain unchanged.\n",
    ),
];

struct Fixture {
    sandbox: TempDir,
    target: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let sandbox = tempdir().unwrap();
        let target = sandbox.path().join("existing-project");
        fs::create_dir(&target).unwrap();
        for (filename, contents) in PROJECT_FILES {
            fs::write(target.join(filename), contents).unwrap();
        }
        fs::create_dir(sandbox.path().join("bin")).unwrap();
        fs::create_dir(sandbox.path().join("home")).unwrap();
        fs::create_dir(sandbox.path().join("gh")).unwrap();
        fs::write(
            sandbox.path().join("home/.gitconfig"),
            "[user]\nname = Git configuration canary\n",
        )
        .unwrap();
        fs::write(
            sandbox.path().join("gh/hosts.yml"),
            "example.invalid:\n  user: gh-account-canary\n  oauth_token: not-a-real-token\n",
        )
        .unwrap();
        fs::write(sandbox.path().join("invocations.log"), "").unwrap();
        #[cfg(unix)]
        install_source_canaries(&sandbox.path().join("bin"));
        Self { sandbox, target }
    }

    fn command(&self) -> Command {
        self.command_with_profile(
            Command::new(env!("CARGO_BIN_EXE_hydragen")),
            std::env::var_os("LLVM_PROFILE_FILE"),
        )
    }

    fn command_with_profile(&self, mut command: Command, profile: Option<OsString>) -> Command {
        // Only the child is isolated. Never inherit credentials or alter the test
        // runner's environment, which other integration tests may use in parallel.
        command
            .current_dir(&self.target)
            .env_clear()
            .env("PATH", self.sandbox.path().join("bin"))
            .env("HOME", self.sandbox.path().join("home"))
            .env("USERPROFILE", self.sandbox.path().join("home"))
            .env("XDG_CONFIG_HOME", self.sandbox.path())
            .env("GH_CONFIG_DIR", self.sandbox.path().join("gh"))
            .env("GH_HOST", "example.invalid")
            .env("GH_TOKEN", "not-a-real-token")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Environment author canary")
            .env("LOGNAME", "environment-login-canary")
            .env("USER", "environment-user-canary")
            .env("USERNAME", "environment-username-canary")
            .env(
                "AUTHOR_ROUTES_LOG",
                self.sandbox.path().join("invocations.log"),
            )
            .env("CI", "false")
            .env("TERM", "xterm-256color")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Forward only the profiler output location, preserving OS bytes and
        // LLVM placeholders while keeping credentials/config isolated.
        if let Some(profile) = profile {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        command
    }

    fn run(&self, arguments: &[&str]) -> Output {
        let mut command = self.command();
        command.args(arguments);
        captured_output(&mut command)
    }

    fn assert_no_source_processes(&self, output: &Output) {
        // On Unix these scripts directly count executable invocations, including
        // probes, authentication attempts and API calls. They do not instrument
        // platform account APIs or prove that arbitrary file reads never happened.
        assert_eq!(
            fs::read_to_string(self.sandbox.path().join("invocations.log")).unwrap(),
            "",
            "An excluded route invoked a source process: {output:?}"
        );
        for stream in [&output.stdout, &output.stderr] {
            let text = String::from_utf8_lossy(stream);
            // Action labels can appear in help prose; reject prompt/loading
            // presentation instead of treating documentation as an active prompt.
            for forbidden in ["canary", "Finding author suggestions", "Author (optional)"] {
                assert!(!text.contains(forbidden), "{text}");
            }
            assert!(!stream.contains(&b'\x1b'), "{output:?}");
        }
    }

    fn assert_created(&self, output: &Output, format: LayerManifestFormat, expected: Value) {
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        assert_eq!(
            output.stdout,
            format!(
                "Layer initialised, layer manifest created at: {}\n",
                format.filename()
            )
            .as_bytes()
        );
        self.assert_no_source_processes(output);
        let text = fs::read_to_string(self.target.join(format.filename())).unwrap();
        let actual: Value = match format {
            LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
                serde_saphyr::from_str(&text).unwrap()
            }
            LayerManifestFormat::Toml => {
                serde_json::to_value(toml::from_str::<toml::Table>(&text).unwrap()).unwrap()
            }
            LayerManifestFormat::Json | LayerManifestFormat::Jsonc => {
                serde_json::from_str(&text).unwrap()
            }
        };
        assert_eq!(actual, expected);
        let mut expected_files = project_snapshot();
        expected_files.insert(
            PathBuf::from(format.filename()),
            Entry::File(text.into_bytes()),
        );
        assert_eq!(snapshot(&self.target), expected_files);
    }
}

#[cfg(unix)]
#[test]
fn profile_forwarding_is_verbatim_and_keeps_route_identity_isolated() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new();
    for profile in [
        Some(OsString::from_vec(
            b"/profiles/raw-\xff-%p-%m.profraw".to_vec(),
        )),
        Some(OsString::new()),
        None,
    ] {
        let mut inherited = Command::new("unused");
        inherited.env("GH_TOKEN", "inherited-private-token");
        inherited.env("GITHUB_TOKEN", "inherited-private-token");
        inherited.env("GH_CONFIG_DIR", "/private/gh-config");
        inherited.env("GIT_CONFIG_GLOBAL", "/private/git-config");
        inherited.env("GIT_CONFIG_COUNT", "1");
        inherited.env("LLVM_PROFILE_FILE", "inherited-profile-must-be-cleared");
        let command = fixture.command_with_profile(inherited, profile.clone());
        let environment: BTreeMap<_, _> = command.get_envs().collect();
        assert_eq!(
            environment
                .get(OsStr::new("LLVM_PROFILE_FILE"))
                .copied()
                .flatten(),
            profile.as_deref()
        );
        assert_eq!(
            environment.get(OsStr::new("GH_TOKEN")),
            Some(&Some(OsStr::new("not-a-real-token")))
        );
        for key in ["GITHUB_TOKEN", "GIT_CONFIG_GLOBAL", "GIT_CONFIG_COUNT"] {
            assert!(!environment.contains_key(OsStr::new(key)), "{key}");
        }
        assert_eq!(
            environment.get(OsStr::new("GH_CONFIG_DIR")),
            Some(&Some(fixture.sandbox.path().join("gh").as_os_str()))
        );
        assert_eq!(
            environment.get(OsStr::new("PATH")),
            Some(&Some(fixture.sandbox.path().join("bin").as_os_str()))
        );
    }
}

#[cfg(unix)]
#[test]
fn instrumented_route_child_writes_forwarded_profile_outside_init_target() {
    let fixture = Fixture::new();
    let before = snapshot(&fixture.target);
    let profiles = fixture.sandbox.path().join("profiles");
    fs::create_dir(&profiles).unwrap();
    let source = fixture.sandbox.path().join("profile_probe.rs");
    let executable = fixture.sandbox.path().join("profile-probe");
    fs::write(
        &source,
        r#"fn main() {
    assert_eq!(std::env::var_os("LLVM_PROFILE_FILE"), std::env::args_os().nth(1));
    assert_eq!(std::env::var("GH_TOKEN").unwrap(), "not-a-real-token");
    assert!(std::env::var_os("GITHUB_TOKEN").is_none());
    assert!(std::env::var_os("GIT_CONFIG_GLOBAL").is_none());
    assert!(std::env::var_os("GIT_CONFIG_COUNT").is_none());
}"#,
    )
    .unwrap();
    let compiled = Command::new("rustc")
        .args(["-C", "instrument-coverage"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{compiled:?}");
    let profile = profiles.join("route-%p-%m.profraw").into_os_string();
    let mut inherited = Command::new(&executable);
    inherited.env("GH_TOKEN", "inherited-private-token");
    inherited.env("GITHUB_TOKEN", "inherited-private-token");
    inherited.env("GIT_CONFIG_GLOBAL", "/private/git-config");
    inherited.env("GIT_CONFIG_COUNT", "1");
    let mut command = fixture.command_with_profile(inherited, Some(profile.clone()));
    command.arg(&profile);
    let output = captured_output(&mut command);
    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
    let files: Vec<_> = fs::read_dir(&profiles).unwrap().collect();
    assert_eq!(files.len(), 1);
    let output = files[0].as_ref().unwrap();
    let filename = output.file_name().into_string().unwrap();
    assert!(filename.starts_with("route-") && filename.ends_with(".profraw"));
    assert!(!filename.contains('%'), "{filename}");
    let bytes = output.metadata().unwrap().len();
    assert!(bytes > 0);
    assert_eq!(snapshot(&fixture.target), before);
    println!(
        "Route forwarding proof: {bytes} profile bytes outside init target; no default profile in target"
    );
}

#[cfg(unix)]
fn install_source_canaries(directory: &Path) {
    use std::os::unix::fs::PermissionsExt;

    // /bin/sh and printf are local; no script delegates to a real tool or network.
    // Include common account/network/project-tool commands as additional traps.
    for executable in [
        "git", "gh", "id", "whoami", "getent", "dscl", "curl", "wget", "npm", "cargo", "python",
        "python3",
    ] {
        let path = directory.join(executable);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nprintf '%s' '{executable}' >> \"$AUTHOR_ROUTES_LOG\"\nprintf ' <%s>' \"$@\" >> \"$AUTHOR_ROUTES_LOG\"\nprintf '\\n' >> \"$AUTHOR_ROUTES_LOG\"\nexit 93\n"
            ),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn captured_output(command: &mut Command) -> Output {
    let mut child = command.spawn().unwrap();
    // Drain both streams while waiting so even help output cannot fill a pipe.
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break (status, false);
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            break (child.wait().unwrap(), true);
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let output = Output {
        status,
        stdout: stdout_reader.join().unwrap(),
        stderr: stderr_reader.join().unwrap(),
    };
    assert!(
        !timed_out,
        "Excluded route waited for input or discovery: {output:?}"
    );
    output
}

#[derive(Debug, PartialEq)]
enum Entry {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
}

fn snapshot(directory: &Path) -> BTreeMap<PathBuf, Entry> {
    fn visit(root: &Path, directory: &Path, entries: &mut BTreeMap<PathBuf, Entry>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_symlink() {
                entries.insert(relative, Entry::Symlink(fs::read_link(path).unwrap()));
            } else if kind.is_dir() {
                entries.insert(relative, Entry::Directory);
                visit(root, &path, entries);
            } else {
                assert!(kind.is_file(), "Unexpected fixture type: {path:?}");
                entries.insert(relative, Entry::File(fs::read(path).unwrap()));
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(directory, directory, &mut entries);
    entries
}

fn project_snapshot() -> BTreeMap<PathBuf, Entry> {
    PROJECT_FILES
        .iter()
        .map(|(name, text)| (PathBuf::from(name), Entry::File(text.as_bytes().to_vec())))
        .collect()
}

fn assert_failure(output: &Output, message: &str) {
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "{output:?}"
    );
}

#[test]
fn each_input_flag_keeps_project_and_identity_authors_out_of_flag_only_init() {
    for (arguments, expected) in [
        (
            vec!["init", "--name", "explicit"],
            json!({"name": "explicit"}),
        ),
        (
            vec!["init", "--author", "Explicit team"],
            json!({"name": "existing-project", "author": "Explicit team"}),
        ),
        (
            vec!["init", "--description", "Existing layer"],
            json!({"name": "existing-project", "description": "Existing layer"}),
        ),
    ] {
        let fixture = Fixture::new();
        let output = fixture.run(&arguments);
        fixture.assert_created(&output, LayerManifestFormat::Yaml, expected);
    }
    // Explicit yaml must select the same flag-only route as every other format;
    // its value matching the inferred default must not make it look unsupplied.
    for format in LayerManifestFormat::ALL {
        let fixture = Fixture::new();
        let output = fixture.run(&["init", "--format", format.extension()]);
        fixture.assert_created(&output, format, json!({"name": "existing-project"}));
    }
}

#[test]
fn explicit_non_interactive_before_or_after_init_does_not_discover_authors() {
    for arguments in [["--non-interactive", "init"], ["init", "--non-interactive"]] {
        let fixture = Fixture::new();
        let output = fixture.run(&arguments);
        fixture.assert_created(
            &output,
            LayerManifestFormat::Yaml,
            json!({"name": "existing-project"}),
        );
    }
}

#[test]
fn ci_without_input_flags_does_not_discover_authors() {
    for ci in ["1", "true", "TrUe", "build"] {
        let fixture = Fixture::new();
        let output = captured_output(fixture.command().env("CI", ci).arg("init"));
        fixture.assert_created(
            &output,
            LayerManifestFormat::Yaml,
            json!({"name": "existing-project"}),
        );
    }
}

#[test]
fn captured_streams_with_null_or_live_piped_stdin_do_not_start_discovery() {
    // Both result streams are captured in every case. This tests the captured
    // route, not that stdout/stderr redirection independently disables a PTY.
    for live_pipe in [false, true] {
        let fixture = Fixture::new();
        let mut command = fixture.command();
        if live_pipe {
            // Retain an open writer until the child exits: an accidental prompt
            // or stdin read cannot succeed merely because it sees EOF.
            command.stdin(Stdio::piped());
        }
        let output = captured_output(command.arg("init"));
        fixture.assert_created(
            &output,
            LayerManifestFormat::Yaml,
            json!({"name": "existing-project"}),
        );
    }
}

#[test]
fn redirected_answer_file_is_not_read_and_does_not_supply_an_author() {
    let fixture = Fixture::new();
    let answers = fixture.sandbox.path().join("answers.txt");
    fs::write(
        &answers,
        "Injected name\nInjected author\nInjected description\njson\n",
    )
    .unwrap();
    let mut input = File::open(&answers).unwrap();
    // Cloned file handles share the position, giving direct evidence that this
    // redirected standard input was not consumed. This is not a project-read counter.
    let output = captured_output(
        fixture
            .command()
            .stdin(input.try_clone().unwrap())
            .arg("init"),
    );
    assert_eq!(input.stream_position().unwrap(), 0);
    fixture.assert_created(
        &output,
        LayerManifestFormat::Yaml,
        json!({"name": "existing-project"}),
    );
}

#[test]
fn explicit_author_is_preserved_without_discovery_and_blank_values_still_fail() {
    let fixture = Fixture::new();
    let author = "  \"quoted\" \\path\n配置 🦀  ";
    let output = fixture.run(&["init", "--author", author]);
    fixture.assert_created(
        &output,
        LayerManifestFormat::Yaml,
        json!({"name": "existing-project", "author": author}),
    );

    for author in ["", " \t\n", "\u{00a0}\u{2003}", "\u{85}"] {
        let fixture = Fixture::new();
        let before = snapshot(&fixture.target);
        let output = fixture.run(&["init", "--author", author]);
        assert_failure(&output, "--author");
        fixture.assert_no_source_processes(&output);
        assert_eq!(snapshot(&fixture.target), before);
    }
}

#[test]
fn only_the_first_creation_succeeds_without_changing_project_attribution() {
    let fixture = Fixture::new();
    let output = fixture.run(&["init", "--author", "Explicit team"]);
    fixture.assert_created(
        &output,
        LayerManifestFormat::Yaml,
        json!({"name": "existing-project", "author": "Explicit team"}),
    );
    let before = snapshot(fixture.sandbox.path());
    let output = fixture.run(&["init", "--author", "Replacement team"]);
    assert_failure(&output, "already has a layer manifest");
    fixture.assert_no_source_processes(&output);
    assert_eq!(snapshot(fixture.sandbox.path()), before);
}

#[test]
fn help_version_and_usage_errors_do_not_invoke_sources_or_change_files() {
    for (arguments, success) in [
        (vec!["--help"], true),
        (vec!["init", "--help"], true),
        (vec!["help", "init"], true),
        (vec!["--version"], true),
        (vec!["-V"], true),
        (vec!["init", "--unknown"], false),
        (vec!["init", "--name"], false),
        (vec!["init", "--author"], false),
        (vec!["init", "--description"], false),
        (vec!["init", "--format"], false),
        (vec!["init", "--format", "invalid"], false),
    ] {
        let fixture = Fixture::new();
        // Safe, inspectable invalid UTF-8 fixtures make accidental content
        // validation visible, but do not claim to count all attempted reads.
        fs::write(fixture.target.join("layer.yaml"), b"not parsed \xff").unwrap();
        fs::write(fixture.target.join("layer.json"), b"not parsed \xff").unwrap();
        let before = snapshot(&fixture.target);
        let output = fixture.run(&arguments);
        assert_eq!(
            output.status.success(),
            success,
            "{arguments:?}: {output:?}"
        );
        fixture.assert_no_source_processes(&output);
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Warning:"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("already has a layer manifest"));
        if success {
            assert!(output.stderr.is_empty(), "{output:?}");
            assert!(!output.stdout.is_empty(), "{output:?}");
        } else {
            assert_failure(&output, "error:");
        }
        assert_eq!(snapshot(&fixture.target), before);
    }
}

#[test]
fn occupied_supported_paths_fail_preflight_without_source_processes_or_changes() {
    for format in LayerManifestFormat::ALL {
        for directory in [false, true] {
            let fixture = Fixture::new();
            let occupied = fixture.target.join(format.filename());
            if directory {
                fs::create_dir(&occupied).unwrap();
                fs::write(occupied.join("keep"), "keep").unwrap();
            } else {
                fs::write(&occupied, b"not parsed \xff").unwrap();
            }
            let before = snapshot(&fixture.target);
            let output = fixture.run(&["init"]);
            assert_failure(&output, "already has a layer manifest");
            assert!(String::from_utf8_lossy(&output.stderr).contains(format.filename()));
            fixture.assert_no_source_processes(&output);
            assert_eq!(snapshot(&fixture.target), before);
        }
    }
}

#[cfg(unix)]
#[test]
fn dangling_supported_paths_fail_preflight_without_source_processes_or_changes() {
    use std::os::unix::fs::symlink;

    for format in LayerManifestFormat::ALL {
        let fixture = Fixture::new();
        let missing = fixture.sandbox.path().join("missing-target");
        symlink(&missing, fixture.target.join(format.filename())).unwrap();
        let before = snapshot(&fixture.target);
        let output = fixture.run(&["init"]);
        assert_failure(&output, "already has a layer manifest");
        fixture.assert_no_source_processes(&output);
        assert_eq!(snapshot(&fixture.target), before);
        assert!(!missing.exists());
    }
}

#[test]
fn nested_git_locations_fail_preflight_without_source_processes_or_changes() {
    for git_directory in [false, true] {
        let fixture = Fixture::new();
        let marker = fixture.sandbox.path().join(".git");
        if git_directory {
            fs::create_dir(&marker).unwrap();
            fs::write(marker.join("config"), "repository metadata canary\n").unwrap();
        } else {
            fs::write(&marker, "gitdir: missing-worktree\n").unwrap();
        }
        let before = snapshot(fixture.sandbox.path());
        let output = fixture.run(&["init"]);
        assert_failure(&output, "repository root");
        fixture.assert_no_source_processes(&output);
        assert_eq!(snapshot(fixture.sandbox.path()), before);
    }
}

#[cfg(unix)]
#[test]
fn repository_marker_inspection_failure_does_not_start_discovery() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    symlink(".git", fixture.target.join(".git")).unwrap();
    let before = snapshot(&fixture.target);
    let output = fixture.run(&["init"]);
    assert_failure(&output, "inspect repository marker");
    fixture.assert_no_source_processes(&output);
    assert_eq!(snapshot(&fixture.target), before);
}

#[cfg(unix)]
#[test]
fn fake_source_processes_record_arguments_and_fail_without_using_real_tools() {
    let fixture = Fixture::new();
    for executable in ["git", "gh"] {
        let output = Command::new(fixture.sandbox.path().join("bin").join(executable))
            .env_clear()
            .env(
                "AUTHOR_ROUTES_LOG",
                fixture.sandbox.path().join("invocations.log"),
            )
            .args(["canary-probe", "argument with spaces"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(93));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    assert_eq!(
        fs::read_to_string(fixture.sandbox.path().join("invocations.log")).unwrap(),
        "git <canary-probe> <argument with spaces>\ngh <canary-probe> <argument with spaces>\n"
    );
    assert_eq!(snapshot(&fixture.target), project_snapshot());
}
