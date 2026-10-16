#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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
    serde_json::from_slice(&fs::read(directory.join("layer.json")).unwrap()).unwrap()
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
    assert!(session.finish().success());
    assert_eq!(
        manifest(directory.path()),
        json!({"name": "  配置 🦀  ", "author": "  Example team  ", "description": "Description"})
    );
    let stdout = String::from_utf8_lossy(&session.stdout);
    assert!(stdout.starts_with("Initialised layer:"), "{stdout}");
    assert!(!stdout.contains("Author (optional"));
    assert!(!stdout.contains(guidance));
    assert!(String::from_utf8_lossy(&session.stderr).contains("Layer initialised"));
    let transcript = visible_text(&session.stderr);
    assert_eq!(transcript.matches(guidance).count(), 1);
    assert!(transcript.contains("Author (optional)"));
    assert!(transcript.contains("Description (optional)"));
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
        assert!(session.finish().success());
        assert_eq!(manifest(directory.path()), expected);
        if description.is_empty() {
            assert!(visible_text(&session.stderr).contains("Skipped"));
        }
        if !author.is_empty() && !description.is_empty() {
            assert!(!visible_text(&session.stderr).contains("Skipped"));
        }
    }
}

#[test]
fn invalid_answers_can_be_corrected_without_creating_files_early() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    session.send(b" \r");
    session.wait_for("non-whitespace");
    assert!(!directory.path().join("layer.json").exists());
    session.send(b"\x7fexample\r");
    session.wait_for("Author (optional");
    session.send(b" \r");
    session.wait_for("--author must contain");
    assert!(!directory.path().join("layer.json").exists());
    session.send(b"\x7f\r");
    session.wait_for("Description (optional");
    session.send(b"\r");
    assert!(session.finish().success());
    assert_eq!(manifest(directory.path()), json!({"name": "example"}));
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
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &arguments, None);
        assert!(!session.finish().success());
        session.assert_no_prompts();
        assert!(session.stdout.is_empty());
        assert!(!directory.path().join("layer.json").exists());
    }
}

#[test]
fn explicit_override_and_ci_use_defaults_without_prompts_in_terminals() {
    for (arguments, ci) in [
        (vec!["--non-interactive", "init"], None),
        (vec!["init", "--non-interactive"], None),
        (vec!["init"], Some("true")),
        (vec!["init"], Some("1")),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &arguments, ci);
        assert!(session.finish().success());
        session.assert_no_prompts();
        assert!(session.stderr.is_empty());
        assert_eq!(
            manifest(directory.path()),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
    }
}

#[test]
fn escape_and_ctrl_c_cancel_before_or_after_some_answers() {
    for key in [b"\x1b".as_slice(), b"\x03"] {
        for answered_name in [false, true] {
            let directory = tempdir().unwrap();
            let mut session = Session::new(directory.path(), &["init"], None);
            session.wait_for("Name");
            if answered_name {
                session.send(b"example\r");
                session.wait_for("Author (optional");
            }
            session.send(key);
            assert!(!session.finish().success());
            assert!(String::from_utf8_lossy(&session.stderr).contains("cancelled"));
            assert!(session.stdout.is_empty());
            assert!(!directory.path().join("layer.json").exists());
        }
    }
}

#[test]
fn an_existing_manifest_is_reported_before_any_questions() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("layer.json"), "not valid JSON").unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    assert!(!session.finish().success());
    session.assert_no_prompts();
    assert!(String::from_utf8_lossy(&session.stderr).contains("already exists"));
    assert_eq!(
        fs::read_to_string(directory.path().join("layer.json")).unwrap(),
        "not valid JSON"
    );
}

#[test]
fn directories_and_symlinks_are_reported_before_any_questions() {
    use std::os::unix::fs::symlink;

    for kind in ["directory", "symlink", "dangling"] {
        let directory = tempdir().unwrap();
        let path = directory.path().join("layer.json");
        let target = directory.path().join("target");
        if kind == "directory" {
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
        assert!(String::from_utf8_lossy(&session.stderr).contains("already exists"));
        if kind == "directory" {
            assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "keep");
        } else {
            assert_eq!(fs::read_link(path).unwrap(), target);
            if kind == "symlink" {
                assert_eq!(fs::read_to_string(target).unwrap(), "keep");
            } else {
                assert!(!target.exists());
            }
        }
    }
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
    assert!(!nested.join("layer.json").exists());
}

#[test]
fn a_manifest_created_during_questions_is_not_overwritten() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    fs::write(directory.path().join("layer.json"), "another process").unwrap();
    session.send(b"example\r");
    session.wait_for("Author (optional");
    session.send(b"\r");
    session.wait_for("Description (optional");
    session.send(b"\r");
    assert!(!session.finish().success());
    assert!(session.stdout.is_empty());
    assert_eq!(
        fs::read_to_string(directory.path().join("layer.json")).unwrap(),
        "another process"
    );
}

#[test]
fn redirecting_any_single_stream_disables_guided_input() {
    for stream in ["stdin", "stdout", "stderr"] {
        let directory = tempdir().unwrap();
        let capture_directory = tempdir().unwrap();
        let capture = capture_directory.path().join("output");
        let mut session =
            Session::spawn(directory.path(), &["init"], None, Some((stream, &capture)));
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
            manifest(directory.path()),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
    }
}

#[test]
fn losing_terminal_input_reports_failure_without_creating_a_manifest() {
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
    let mut session = Session::spawn_with_lock(
        directory.path(),
        &["init"],
        None,
        Some(("stdin-eof", &input_path)),
        terminal_session,
    );
    session.wait_for("Name");
    // Closing every master handle disconnects stdin while stderr remains usable.
    // This is actual terminal input loss, not Ctrl-D (a raw-mode keystroke).
    drop(input);
    assert!(!session.finish().success());
    assert!(String::from_utf8_lossy(&session.stderr).contains("Could not collect metadata"));
    assert!(session.stdout.is_empty());
    assert!(!directory.path().join("layer.json").exists());
}
