#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use hydragen::ManifestFormat;
use portable_pty::{Child, CommandBuilder, ExitStatus, PtyPair, PtySize, native_pty_system};
use serde_json::{Value, json};
use tempfile::tempdir;

const TIMEOUT: Duration = Duration::from_secs(10);
// Bound terminal allocation independently of the test runner's thread count.
// Some macOS environments reject overlapping openpty calls.
static TERMINAL_SESSION: Mutex<()> = Mutex::new(());

enum Event {
    Data(bool, Vec<u8>),
    Closed,
}

// Keep stdout and stderr on distinct terminals so stream assertions exercise
// actual cliclack output rather than relying on a combined terminal transcript.
struct Session {
    _terminal_session: MutexGuard<'static, ()>,
    child: Box<dyn Child + Send + Sync>,
    writer: Option<Box<dyn Write + Send>>,
    stdout_terminal: Option<PtyPair>,
    events: Receiver<Event>,
    readers: Vec<JoinHandle<()>>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    closed: usize,
    finished: bool,
}

impl Session {
    fn new(directory: &Path, arguments: &[&str], ci: Option<&str>) -> Self {
        Self::spawn(directory, arguments, ci, None)
    }

    fn spawn(
        directory: &Path,
        arguments: &[&str],
        ci: Option<&str>,
        redirect: Option<(&str, &Path)>,
    ) -> Self {
        let terminal_session = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Self::spawn_with_lock(directory, arguments, ci, redirect, terminal_session)
    }

    fn spawn_with_lock(
        directory: &Path,
        arguments: &[&str],
        ci: Option<&str>,
        redirect: Option<(&str, &Path)>,
        terminal_session: MutexGuard<'static, ()>,
    ) -> Self {
        let system = native_pty_system();
        let size = PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };
        let terminal = system.openpty(size).unwrap();
        let stdout_terminal = system.openpty(size).unwrap();
        let mut command = CommandBuilder::new("/bin/sh");
        command.args([
            "-c",
            "case \"$REDIRECT\" in
stdin) exec \"$@\" </dev/null >\"$RESULT_TTY\";;
stdin-eof) exec \"$@\" <\"$CAPTURE\" >\"$RESULT_TTY\";;
stdout) exec \"$@\" >\"$CAPTURE\";;
stderr) exec \"$@\" >\"$RESULT_TTY\" 2>\"$CAPTURE\";;
*) exec \"$@\" >\"$RESULT_TTY\";;
esac",
            "terminal-test",
            env!("CARGO_BIN_EXE_hydragen"),
        ]);
        command.args(arguments);
        command.cwd(directory);
        command.env("TERM", "xterm-256color");
        command.env("RESULT_TTY", stdout_terminal.master.tty_name().unwrap());
        command.env_remove("CI");
        command.env_remove("REDIRECT");
        if let Some(ci) = ci {
            command.env("CI", ci);
        }
        if let Some((stream, capture)) = redirect {
            command.env("REDIRECT", stream);
            command.env("CAPTURE", capture);
        }
        let writer = Some(terminal.master.take_writer().unwrap());
        let (sender, events) = mpsc::channel();
        let readers = [
            (false, terminal.master.try_clone_reader().unwrap()),
            (true, stdout_terminal.master.try_clone_reader().unwrap()),
        ]
        .into_iter()
        .map(|(stdout, mut reader)| {
            let sender = sender.clone();
            std::thread::spawn(move || {
                let mut buffer = [0; 4096];
                while let Ok(count) = reader.read(&mut buffer) {
                    if count == 0 {
                        break;
                    }
                    if sender
                        .send(Event::Data(stdout, buffer[..count].to_vec()))
                        .is_err()
                    {
                        return;
                    }
                }
                let _ = sender.send(Event::Closed);
            })
        })
        .collect();
        let child = terminal.slave.spawn_command(command).unwrap();
        drop(terminal.slave);
        Self {
            _terminal_session: terminal_session,
            child,
            writer,
            stdout_terminal: Some(stdout_terminal),
            events,
            readers,
            stdout: Vec::new(),
            stderr: Vec::new(),
            closed: 0,
            finished: false,
        }
    }

    fn receive(&mut self, wait: Duration) {
        match self.events.recv_timeout(wait) {
            Ok(Event::Data(true, data)) => self.stdout.extend(data),
            Ok(Event::Data(false, data)) => self.stderr.extend(data),
            Ok(Event::Closed) => self.closed += 1,
            Err(_) => {}
        }
    }

    fn wait_for(&mut self, message: &str) {
        let deadline = Instant::now() + TIMEOUT;
        while !visible_text(&self.stderr).contains(message) {
            assert!(
                Instant::now() < deadline,
                "Timed out waiting for {message:?}: {}",
                String::from_utf8_lossy(&self.stderr)
            );
            self.receive(Duration::from_millis(10));
        }
    }

    fn send(&mut self, input: &[u8]) {
        let writer = self.writer.as_mut().unwrap();
        writer.write_all(input).unwrap();
        writer.flush().unwrap();
    }

    fn choose_format(&mut self, directory: &Path, format: ManifestFormat) {
        self.wait_for("Manifest format");
        self.wait_for("layer.jsonc");
        assert_no_manifests(directory);
        for _ in 0..ManifestFormat::ALL
            .iter()
            .position(|item| *item == format)
            .unwrap()
        {
            self.send(b"\x1b[B");
        }
        self.send(b"\r");
    }

    fn finish(&mut self) -> ExitStatus {
        let deadline = Instant::now() + TIMEOUT;
        let mut status = None;
        loop {
            if status.is_none() {
                status = self.child.try_wait().unwrap();
            }
            if let Some(status) = &status {
                self.writer.take();
                self.stdout_terminal.take();
                if self.closed == self.readers.len() {
                    self.finished = true;
                    return status.clone();
                }
            }
            assert!(
                Instant::now() < deadline,
                "Terminal child did not finish: {}",
                String::from_utf8_lossy(&self.stderr)
            );
            self.receive(Duration::from_millis(10));
        }
    }

    fn assert_no_prompts(&self) {
        let stderr = String::from_utf8_lossy(&self.stderr);
        assert!(!stderr.contains("Initialise a layer"), "{stderr}");
        assert!(!stderr.contains("Author (optional"), "{stderr}");
        assert!(!stderr.contains("Manifest format"), "{stderr}");
        assert!(!self.stderr.contains(&b'\x1b'), "{stderr}");
        assert!(!self.stdout.contains(&b'\x1b'));
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        self.writer.take();
        self.stdout_terminal.take();
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

// Match visible text even when cliclack styles parts of labels or defaults.
fn visible_text(bytes: &[u8]) -> String {
    console::strip_ansi_codes(&String::from_utf8_lossy(bytes)).into_owned()
}

fn manifest(directory: &Path) -> Value {
    manifest_in_format(directory, ManifestFormat::Yaml)
}

// Decode independently of Hydragen's loader so matching reader/writer bugs cannot
// hide malformed output or a format field accidentally stored as metadata.
fn manifest_in_format(directory: &Path, format: ManifestFormat) -> Value {
    let text = fs::read_to_string(directory.join(format.filename())).unwrap();
    assert!(text.ends_with('\n'));
    match format {
        ManifestFormat::Yaml | ManifestFormat::Yml => serde_saphyr::from_str(&text).unwrap(),
        ManifestFormat::Toml => {
            let table: toml::Table = toml::from_str(&text).unwrap();
            serde_json::to_value(table).unwrap()
        }
        ManifestFormat::Json | ManifestFormat::Jsonc => serde_json::from_str(&text).unwrap(),
    }
}

fn assert_no_manifests(directory: &Path) {
    for format in ManifestFormat::ALL {
        assert!(
            fs::symlink_metadata(directory.join(format.filename())).is_err(),
            "{} appeared early",
            format.filename()
        );
    }
}

fn assert_only_files(directory: &Path, expected: &[&str]) {
    let mut actual: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    actual.sort();
    let mut expected = expected.to_vec();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn guided_input_uses_cliclack_on_stderr_and_keeps_the_result_on_stdout() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    let guidance = "Enter accepts the default or skips an optional field.";
    assert!(visible_text(&session.stderr).contains(guidance));
    session.send("  配置 🦀  \r".as_bytes());
    session.wait_for("Author (optional");
    session.send(b"  Example team  \r");
    session.wait_for("Description (optional");
    session.send(b"Description\r");
    session.choose_format(directory.path(), ManifestFormat::Yaml);
    assert!(session.finish().success());
    assert_eq!(
        manifest(directory.path()),
        json!({"name": "  配置 🦀  ", "author": "  Example team  ", "description": "Description"})
    );
    let stdout = String::from_utf8_lossy(&session.stdout);
    assert!(stdout.starts_with("Initialised layer:"), "{stdout}");
    assert!(stdout.contains("layer.yaml"), "{stdout}");
    assert!(!stdout.contains("Author (optional"));
    assert!(!stdout.contains(guidance));
    assert!(String::from_utf8_lossy(&session.stderr).contains("Layer initialised"));
    let transcript = visible_text(&session.stderr);
    assert_eq!(transcript.matches(guidance).count(), 1);
    assert!(transcript.contains("Author (optional)"));
    assert!(transcript.contains("Description (optional)"));
    assert!(transcript.contains("Manifest format"));
    assert!(!transcript.contains("Enter to skip"));
}

#[test]
fn guided_name_shows_the_directory_default_and_enter_accepts_it() {
    let parent = tempdir().unwrap();
    let directory = parent.path().join(" My Project 配置 🦀 ");
    fs::create_dir(&directory).unwrap();
    let default_name = directory.file_name().unwrap().to_str().unwrap();
    let mut session = Session::new(&directory, &["init"], None);
    session.wait_for("Name");
    session.wait_for(default_name);
    session.send(b"\r");
    session.wait_for("Author (optional");
    session.send(b"\r");
    session.wait_for("Description (optional");
    session.send(b"\r");
    session.choose_format(&directory, ManifestFormat::Yaml);
    assert!(session.finish().success());
    assert_eq!(manifest(&directory), json!({"name": default_name}));
}

#[test]
fn optional_fields_can_be_skipped_independently() {
    for (author, description, expected) in [
        ("", "", json!({"name": "example"})),
        ("Team", "", json!({"name": "example", "author": "Team"})),
        (
            "Skipped",
            "",
            json!({"name": "example", "author": "Skipped"}),
        ),
        (
            "Team",
            "Layer",
            json!({"name": "example", "author": "Team", "description": "Layer"}),
        ),
        (
            "",
            "Layer",
            json!({"name": "example", "description": "Layer"}),
        ),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        session.wait_for("Name");
        session.send(b"example\r");
        session.wait_for("Author (optional");
        assert!(!visible_text(&session.stderr).contains("Skipped"));
        session.send(format!("{author}\r").as_bytes());
        session.wait_for("Description (optional");
        if author.is_empty() {
            assert!(visible_text(&session.stderr).contains("Skipped"));
        }
        session.send(format!("{description}\r").as_bytes());
        session.choose_format(directory.path(), ManifestFormat::Yaml);
        assert!(session.finish().success());
        assert_eq!(manifest(directory.path()), expected);
        if description.is_empty() {
            assert!(visible_text(&session.stderr).contains("Skipped"));
        }
        if !author.is_empty() && !description.is_empty() {
            assert!(!visible_text(&session.stderr).contains("Skipped"));
        }
        let transcript = visible_text(&session.stderr);
        let format_summary = transcript.rsplit_once("Manifest format").unwrap().1;
        assert!(format_summary.contains("layer.yaml"), "{format_summary}");
        assert!(!format_summary.contains("Skipped"), "{format_summary}");
    }
}

#[test]
fn invalid_answers_can_be_corrected_without_creating_files_early() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    session.send(b" \r");
    session.wait_for("non-whitespace");
    assert_no_manifests(directory.path());
    session.send(b"\x7fexample\r");
    session.wait_for("Author (optional");
    session.send(b" \r");
    session.wait_for("--author must contain");
    assert_no_manifests(directory.path());
    session.send(b"\x7f\r");
    session.wait_for("Description (optional");
    session.send(b"\r");
    session.choose_format(directory.path(), ManifestFormat::Yaml);
    assert!(session.finish().success());
    assert_eq!(manifest(directory.path()), json!({"name": "example"}));
}

#[test]
fn every_guided_format_choice_uses_a_fixed_filename_and_independent_content() {
    for format in ManifestFormat::ALL {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        session.wait_for("Name");
        assert_no_manifests(directory.path());
        session.send(b"shared-agent-rules\r");
        session.wait_for("Author (optional");
        assert_no_manifests(directory.path());
        session.send("  Équipe 🦀  \r".as_bytes());
        session.wait_for("Description (optional");
        assert_no_manifests(directory.path());
        session.send(b"2026-10-16\r");
        session.wait_for("layer.jsonc");
        let transcript = visible_text(&session.stderr);
        let choices = transcript.split_once("Manifest format").unwrap().1;
        let mut previous = 0;
        for choice in ManifestFormat::ALL {
            let position = choices.find(choice.filename()).unwrap();
            assert!(position >= previous, "{choices}");
            previous = position;
        }
        session.choose_format(directory.path(), format);
        assert!(session.finish().success());
        assert_eq!(
            manifest_in_format(directory.path(), format),
            json!({
                "name": "shared-agent-rules", "author": "  Équipe 🦀  ", "description": "2026-10-16"
            })
        );
        assert_only_files(directory.path(), &[format.filename()]);
        let stdout = visible_text(&session.stdout);
        assert!(
            stdout.contains(
                &directory
                    .path()
                    .join(format.filename())
                    .display()
                    .to_string()
            ),
            "{stdout}"
        );
        assert!(!stdout.contains("Manifest format"));
        let transcript = visible_text(&session.stderr);
        assert!(!transcript.contains("Skipped"));
        let summary = transcript.rsplit_once("Manifest format").unwrap().1;
        assert!(summary.contains(format.filename()), "{summary}");
    }
}

#[test]
fn format_only_flags_never_prompt_in_a_terminal() {
    for format in ManifestFormat::ALL {
        let directory = tempdir().unwrap();
        let mut session = Session::new(
            directory.path(),
            &["init", "--format", format.extension()],
            None,
        );
        assert!(session.finish().success());
        session.assert_no_prompts();
        assert!(session.stderr.is_empty());
        assert_eq!(
            manifest_in_format(directory.path(), format),
            json!({
                "name": directory.path().file_name().unwrap().to_str().unwrap()
            })
        );
        assert!(visible_text(&session.stdout).contains(format.filename()));
        assert_only_files(directory.path(), &[format.filename()]);
    }
}

#[test]
fn metadata_flags_never_start_a_guided_session_in_a_terminal() {
    for arguments in [
        vec!["init", "--author", "Team"],
        vec!["init", "--description", "Layer"],
        vec!["init", "--name", "example"],
        vec!["init", "--name", "example", "--author", "Team"],
        vec![
            "init",
            "--name",
            "example",
            "--author",
            "Team",
            "--description",
            "Layer",
        ],
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &arguments, None);
        assert!(session.finish().success());
        session.assert_no_prompts();
        let expected_name = if arguments.contains(&"--name") {
            "example"
        } else {
            directory.path().file_name().unwrap().to_str().unwrap()
        };
        assert_eq!(manifest(directory.path())["name"], expected_name);
        assert_eq!(
            manifest(directory.path()).get("author").is_some(),
            arguments.contains(&"--author")
        );
        assert_eq!(
            manifest(directory.path()).get("description").is_some(),
            arguments.contains(&"--description")
        );
    }
}

#[test]
fn invalid_flags_error_instead_of_asking_for_replacements() {
    for arguments in [
        vec!["init", "--author", " "],
        vec!["init", "--name", ""],
        vec!["init", "--name", "example", "--author", " "],
        vec!["init", "--format", "invalid"],
        vec!["init", "--format"],
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &arguments, None);
        assert!(!session.finish().success());
        session.assert_no_prompts();
        assert!(session.stdout.is_empty());
        assert_no_manifests(directory.path());
        assert_only_files(directory.path(), &[]);
    }
}

#[test]
fn explicit_override_and_ci_use_defaults_without_prompts_in_terminals() {
    for (arguments, ci) in [
        (vec!["--non-interactive", "init"], None),
        (vec!["init", "--non-interactive"], None),
        (vec!["init"], Some("true")),
        (vec!["init"], Some("1")),
        (vec!["--non-interactive", "init", "--format", "toml"], None),
        (vec!["init", "--format", "jsonc", "--non-interactive"], None),
        (vec!["init", "--format", "json"], Some("true")),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &arguments, ci);
        assert!(session.finish().success());
        session.assert_no_prompts();
        assert!(session.stderr.is_empty());
        let format = arguments
            .iter()
            .position(|argument| *argument == "--format")
            .map(|index| {
                ManifestFormat::ALL
                    .into_iter()
                    .find(|format| format.extension() == arguments[index + 1])
                    .unwrap()
            })
            .unwrap_or_default();
        assert_eq!(
            manifest_in_format(directory.path(), format),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
        assert_only_files(directory.path(), &[format.filename()]);
    }
}

#[test]
fn escape_and_ctrl_c_cancel_before_or_after_metadata_and_at_format() {
    for key in [b"\x1b".as_slice(), b"\x03"] {
        for answered in 0..=3 {
            let directory = tempdir().unwrap();
            let mut session = Session::new(directory.path(), &["init"], None);
            session.wait_for("Name");
            if answered >= 1 {
                session.send(b"example\r");
                session.wait_for("Author (optional");
            }
            if answered >= 2 {
                session.send(b"Team\r");
                session.wait_for("Description (optional");
            }
            if answered == 3 {
                session.send(b"Description\r");
                session.wait_for("Manifest format");
                session.wait_for("layer.jsonc");
            }
            assert_no_manifests(directory.path());
            session.send(key);
            assert!(!session.finish().success());
            assert!(String::from_utf8_lossy(&session.stderr).contains("cancelled"));
            assert!(session.stdout.is_empty());
            assert_no_manifests(directory.path());
            assert_only_files(directory.path(), &[]);
        }
    }
}

#[test]
fn every_occupied_manifest_path_is_reported_before_any_questions() {
    use std::os::unix::fs::symlink;

    for (format, kind) in ManifestFormat::ALL
        .into_iter()
        .flat_map(|format| ["file", "directory", "symlink", "dangling"].map(|kind| (format, kind)))
    {
        let directory = tempdir().unwrap();
        let path = directory.path().join(format.filename());
        let target = directory.path().join("target");
        if kind == "file" {
            fs::write(&path, "not valid manifest content").unwrap();
        } else if kind == "directory" {
            fs::create_dir(&path).unwrap();
            fs::write(path.join("keep"), "keep").unwrap();
        } else {
            if kind == "symlink" {
                fs::write(&target, "keep").unwrap();
            }
            symlink(&target, &path).unwrap();
        }
        let mut session = Session::new(directory.path(), &["init"], None);
        assert!(!session.finish().success());
        session.assert_no_prompts();
        assert!(session.stdout.is_empty());
        assert!(String::from_utf8_lossy(&session.stderr).contains("already exists"));
        assert!(String::from_utf8_lossy(&session.stderr).contains(format.filename()));
        assert!(!visible_text(&session.stderr).contains("Warning:"));
        if kind == "file" {
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                "not valid manifest content"
            );
        } else if kind == "directory" {
            assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "keep");
        } else {
            assert_eq!(fs::read_link(&path).unwrap(), target);
            if kind == "symlink" {
                assert_eq!(fs::read_to_string(&target).unwrap(), "keep");
            } else {
                assert!(!target.exists());
            }
        }
        let expected = if kind == "symlink" {
            vec![format.filename(), "target"]
        } else {
            vec![format.filename()]
        };
        assert_only_files(directory.path(), &expected);
    }
}

#[test]
fn initial_duplicates_warn_once_before_refusal_without_questions() {
    let directory = tempdir().unwrap();
    for format in ManifestFormat::ALL {
        fs::write(
            directory.path().join(format.filename()),
            "not valid content",
        )
        .unwrap();
    }
    let mut session = Session::new(directory.path(), &["init"], None);
    assert!(!session.finish().success());
    session.assert_no_prompts();
    assert!(session.stdout.is_empty());
    let stderr = visible_text(&session.stderr);
    assert_eq!(stderr.matches("Warning:").count(), 1, "{stderr}");
    assert!(stderr.find("Warning:").unwrap() < stderr.find("Error:").unwrap());
    assert!(stderr.contains("layer.yaml takes priority"), "{stderr}");
    for format in ManifestFormat::ALL {
        assert!(stderr.contains(format.filename()), "{stderr}");
        assert_eq!(
            fs::read_to_string(directory.path().join(format.filename())).unwrap(),
            "not valid content"
        );
    }
    assert_only_files(
        directory.path(),
        &ManifestFormat::ALL.map(|format| format.filename()),
    );
}

#[test]
fn repository_location_errors_happen_before_questions() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join(".git")).unwrap();
    let nested = directory.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let mut session = Session::new(&nested, &["init"], None);
    assert!(!session.finish().success());
    session.assert_no_prompts();
    assert!(String::from_utf8_lossy(&session.stderr).contains("repository root"));
    assert_no_manifests(&nested);
}

#[test]
fn the_exact_default_target_created_during_questions_is_not_overwritten() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    fs::write(directory.path().join("layer.yaml"), "another process").unwrap();
    session.send(b"example\r");
    session.wait_for("Author (optional");
    session.send(b"\r");
    session.wait_for("Description (optional");
    session.send(b"\r");
    session.wait_for("Manifest format");
    session.wait_for("layer.jsonc");
    session.send(b"\r");
    assert!(!session.finish().success());
    assert!(session.stdout.is_empty());
    assert_eq!(
        fs::read_to_string(directory.path().join("layer.yaml")).unwrap(),
        "another process"
    );
    assert!(visible_text(&session.stderr).contains("already exists"));
    assert_only_files(directory.path(), &["layer.yaml"]);
}

#[test]
fn alternate_formats_appearing_during_questions_allow_creation_in_both_priority_directions() {
    for (appeared, chosen, governing) in [
        (
            ManifestFormat::Json,
            ManifestFormat::Yaml,
            ManifestFormat::Yaml,
        ),
        (
            ManifestFormat::Yaml,
            ManifestFormat::Jsonc,
            ManifestFormat::Yaml,
        ),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        session.wait_for("Name");
        assert_no_manifests(directory.path());
        fs::write(
            directory.path().join(appeared.filename()),
            "another process",
        )
        .unwrap();
        session.send(b"example\r");
        session.wait_for("Author (optional");
        session.send(b"\r");
        session.wait_for("Description (optional");
        session.send(b"\r");
        session.wait_for("Manifest format");
        session.wait_for("layer.jsonc");
        // The output is chosen only after the competing path has appeared.
        for _ in 0..ManifestFormat::ALL
            .iter()
            .position(|format| *format == chosen)
            .unwrap()
        {
            session.send(b"\x1b[B");
        }
        assert!(fs::symlink_metadata(directory.path().join(chosen.filename())).is_err());
        session.send(b"\r");
        assert!(session.finish().success());
        assert_eq!(
            manifest_in_format(directory.path(), chosen),
            json!({"name": "example"})
        );
        assert_eq!(
            fs::read_to_string(directory.path().join(appeared.filename())).unwrap(),
            "another process"
        );
        let stderr = visible_text(&session.stderr);
        assert_eq!(stderr.matches("Warning:").count(), 1, "{stderr}");
        assert!(
            stderr.contains(&format!("{} takes priority", governing.filename())),
            "{stderr}"
        );
        assert!(stderr.contains(appeared.filename()));
        assert!(stderr.contains(chosen.filename()));
        let stdout = visible_text(&session.stdout);
        assert!(
            stdout.contains(
                &directory
                    .path()
                    .join(chosen.filename())
                    .display()
                    .to_string()
            ),
            "{stdout}"
        );
        assert!(!stdout.contains("Warning:"));
        assert_only_files(directory.path(), &[appeared.filename(), chosen.filename()]);
    }
}

#[test]
fn redirecting_any_single_stream_disables_guided_input() {
    for (stream, format) in ["stdin", "stdout", "stderr"]
        .into_iter()
        .flat_map(|stream| {
            [ManifestFormat::Yaml, ManifestFormat::Jsonc].map(|format| (stream, format))
        })
    {
        let directory = tempdir().unwrap();
        let capture_directory = tempdir().unwrap();
        let capture = capture_directory.path().join("output");
        let arguments = if format == ManifestFormat::Yaml {
            vec!["init"]
        } else {
            vec!["init", "--format", format.extension()]
        };
        let mut session =
            Session::spawn(directory.path(), &arguments, None, Some((stream, &capture)));
        assert!(session.finish().success());
        session.assert_no_prompts();
        assert!(session.stderr.is_empty());
        let result = if stream == "stdout" {
            fs::read(&capture).unwrap()
        } else {
            session.stdout.clone()
        };
        assert!(String::from_utf8_lossy(&result).contains("Initialised layer:"));
        if stream == "stderr" {
            assert!(fs::read(&capture).unwrap().is_empty());
        }
        assert_eq!(
            manifest_in_format(directory.path(), format),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
        assert_only_files(directory.path(), &[format.filename()]);
    }
}

#[test]
fn losing_terminal_input_reports_failure_without_creating_a_manifest() {
    for at_selector in [false, true] {
        let directory = tempdir().unwrap();
        let terminal_session = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let input = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let input_path = input.master.tty_name().unwrap();
        let mut input_writer = input.master.take_writer().unwrap();
        let mut session = Session::spawn_with_lock(
            directory.path(),
            &["init"],
            None,
            Some(("stdin-eof", &input_path)),
            terminal_session,
        );
        session.wait_for("Name");
        if at_selector {
            for (answer, next_prompt) in [
                ("example\r", "Author (optional"),
                ("Team\r", "Description (optional"),
                ("Description\r", "Manifest format"),
            ] {
                input_writer.write_all(answer.as_bytes()).unwrap();
                input_writer.flush().unwrap();
                session.wait_for(next_prompt);
            }
            session.wait_for("layer.jsonc");
        }
        assert_no_manifests(directory.path());
        // Closing every master handle disconnects stdin while stderr remains usable.
        // This is actual terminal input loss, not Ctrl-D (a raw-mode keystroke).
        drop(input_writer);
        drop(input);
        assert!(!session.finish().success());
        assert!(
            String::from_utf8_lossy(&session.stderr)
                .contains("Could not collect initialisation input")
        );
        assert!(session.stdout.is_empty());
        assert_no_manifests(directory.path());
        assert_only_files(directory.path(), &[]);
    }
}
