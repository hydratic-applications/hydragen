//! Controlled PTYs exercise the production collector with injected source lanes.
//! This module, including its environment protocol, does not exist in release builds.
use super::*;
use crate::author_discovery::{ContactField, ContactInput, ProjectFile, Source, Status, testing};
use portable_pty::{Child, CommandBuilder, ExitStatus, PtySize, native_pty_system};
use std::fs;
use std::io::{Read, Write};
use std::sync::MutexGuard;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(6);
static TERMINAL: Mutex<()> = Mutex::new(());

// Only an ignored test in the binary unit executable reads this protocol. The
// release main function cannot select these sources, clocks, or output files.
#[test]
#[ignore = "child-only controlled author terminal driver"]
fn terminal_driver() {
    let root = std::env::var_os("HYDRAGEN_AUTHOR_DRIVER_ROOT").expect("driver root");
    let root = std::path::PathBuf::from(root);
    let scenario = std::env::var("HYDRAGEN_AUTHOR_DRIVER_CASE").expect("driver case");
    let clock_root = root.clone();
    let clock = Arc::new(move || {
        let offset = fs::read_to_string(clock_root.join("clock"))
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        Instant::now() + Duration::from_millis(offset)
    });
    let source_root = root.clone();
    let source_case = scenario.clone();
    let (process_done, process_finished) = mpsc::channel();
    let local: testing::Lane = Box::new(move |publisher| {
        fs::write(source_root.join("started"), b"1").unwrap();
        match source_case.as_str() {
            "empty" => {}
            "custom" => {
                publisher.publish("Fixture author", false)?;
            }
            "contacts-one" | "contacts-multi" | "contacts-literal" => {
                let name = if source_case == "contacts-literal" {
                    "Fixture <literal@example.com> (https://literal.example)"
                } else {
                    "Fixture author"
                };
                publisher.publish_contacts(
                    name,
                    Source::Project {
                        file: ProjectFile::PackageJson,
                        index: 0,
                    },
                    &[
                        ContactInput {
                            field: ContactField::Email,
                            value: "work@example.com",
                        },
                        ContactInput {
                            field: ContactField::Url,
                            value: "https://work.example",
                        },
                    ],
                )?;
                if source_case == "contacts-multi" {
                    publisher.publish_contacts(
                        name,
                        Source::GithubProfile {
                            host: "github.com".into(),
                            login: "fixture".into(),
                        },
                        &[
                            ContactInput {
                                field: ContactField::Email,
                                value: "personal@example.com",
                            },
                            ContactInput {
                                field: ContactField::Website,
                                value: "https://personal.example",
                            },
                            ContactInput {
                                field: ContactField::ProfileUrl,
                                value: "https://github.com/fixture",
                            },
                        ],
                    )?;
                    publisher.publish_contacts(
                        "Other author",
                        Source::Project {
                            file: ProjectFile::PackageJson,
                            index: 1,
                        },
                        &[
                            ContactInput {
                                field: ContactField::Email,
                                value: "other@example.com",
                            },
                            ContactInput {
                                field: ContactField::Url,
                                value: "https://other.example",
                            },
                        ],
                    )?;
                }
            }
            "contacts-many" => {
                for index in 0..16 {
                    publisher.publish_contacts(
                        "Fixture author",
                        Source::Project {
                            file: ProjectFile::PackageJson,
                            index,
                        },
                        &[
                            ContactInput {
                                field: ContactField::Email,
                                value: &format!("contact-{index:02}@example.com"),
                            },
                            ContactInput {
                                field: ContactField::Url,
                                value: &format!("https://example.com/{index:02}"),
                            },
                        ],
                    )?;
                }
            }
            "late-contacts" => {
                publisher.publish_contacts(
                    "Fixture author",
                    Source::GithubProfile {
                        host: "github.com".into(),
                        login: "fixture".into(),
                    },
                    &[ContactInput {
                        field: ContactField::ProfileUrl,
                        value: "https://github.com/fixture",
                    }],
                )?;
                fs::write(source_root.join("ready-to-wait"), "1").unwrap();
                let until = Instant::now() + TIMEOUT;
                while !source_root.join("release").exists() && Instant::now() < until {
                    thread::sleep(Duration::from_millis(2));
                }
                let accepted = publisher.enrich_github_profile(
                    "github.com",
                    "fixture",
                    &[
                        ContactInput {
                            field: ContactField::Email,
                            value: "late@example.com",
                        },
                        ContactInput {
                            field: ContactField::Website,
                            value: "https://late.example",
                        },
                    ],
                )?;
                fs::write(
                    source_root.join("published"),
                    if accepted { "yes" } else { "no" },
                )
                .unwrap();
            }
            "membership" => {
                publisher.publish("Membership organisation", true)?;
            }
            "many-memberships" => {
                for index in 0..20 {
                    publisher.publish(&format!("Membership {index:02}"), true)?;
                }
            }
            "fatal" => {
                return Err(schema::Error::Setup {
                    message: "injected author failure".into(),
                    detail: None,
                });
            }
            "process" | "wait-error" | "wait-interrupted" => {
                let mut command = std::process::Command::new("/bin/sh");
                command
                    .args([
                        "-c",
                        "printf '%s' $$ > \"$1\"; exec /bin/sleep 60",
                        "source-child",
                    ])
                    .arg(source_root.join("pid"));
                publisher.run(command);
                fs::write(
                    source_root.join("closed-after-process"),
                    if publisher.active() { "no" } else { "yes" },
                )
                .unwrap();
                fs::write(source_root.join("process-finished"), b"1").unwrap();
                process_done.send(()).unwrap();
            }
            "pending" | "late" | "during-name" => {
                if source_case == "late" {
                    publisher.publish("Early author", false)?;
                }
                fs::write(source_root.join("ready-to-wait"), b"1").unwrap();
                let until = Instant::now() + TIMEOUT;
                while !source_root.join("release").exists()
                    && (publisher.active() || source_case == "late")
                    && Instant::now() < until
                {
                    thread::sleep(Duration::from_millis(2));
                }
                // Deliberately attempt publication even after closure to exercise
                // the real handle's cutoff, not just a cooperative fixture.
                let accepted = publisher.publish("Late author", false)?;
                fs::write(
                    source_root.join("published"),
                    if accepted { "yes" } else { "no" },
                )
                .unwrap();
            }
            other => panic!("unknown controlled source {other}"),
        }
        Ok(())
    });
    let preparation = testing::start(clock, [local, Box::new(|_| Ok(()))]).unwrap();
    if matches!(
        scenario.as_str(),
        "empty"
            | "custom"
            | "membership"
            | "many-memberships"
            | "fatal"
            | "contacts-one"
            | "contacts-multi"
            | "contacts-literal"
            | "contacts-many"
    ) {
        // These fixtures specifically enter Name with already-ready results.
        // The pending fixtures below never wait before calling the collector.
        let until = Instant::now() + TIMEOUT;
        while preparation.status() == Status::Pending {
            assert!(Instant::now() < until);
            thread::yield_now();
        }
    }
    let presentation = Presentation::detect();
    presentation.configure();
    let session = GuidedSession::install(presentation);
    let _poll = matches!(scenario.as_str(), "wait-error" | "wait-interrupted").then(|| {
        let error_root = root.clone();
        let interrupted = scenario == "wait-interrupted";
        author_wait::inject_poll_for_tests(Box::new(move |timeout| {
            if error_root.join("fail-poll").exists() {
                return Err(if interrupted {
                    io::ErrorKind::Interrupted.into()
                } else {
                    io::Error::from_raw_os_error(libc::EIO)
                });
            }
            crossterm::event::poll(timeout)
        }))
    });
    let before = terminal_flags();
    let result = collect_init_input("directory-default", &session, &preparation);
    let after = terminal_flags();
    fs::write(
        root.join("terminal-settings"),
        format!("before: {before:?}\nafter: {after:?}"),
    )
    .unwrap();
    fs::write(
        root.join("restored"),
        if before == after { "yes" } else { "no" },
    )
    .unwrap();
    drop(preparation);
    if matches!(
        scenario.as_str(),
        "process" | "wait-error" | "wait-interrupted"
    ) {
        // The runner releases its process reservation before fixture-only marker
        // writes. Acknowledge those writes before exiting the test process, rather
        // than race them against process::exit. Production cleanup is unchanged.
        process_finished
            .recv_timeout(Duration::from_secs(1))
            .expect("fixture process cleanup acknowledgement");
    }
    match result {
        Ok(input) => fs::write(
            root.join("result"),
            serde_json::to_vec(&serde_json::json!({
                "name": input.metadata.name, "author": input.metadata.author,
                "description": input.metadata.description, "format": input.format.extension(),
            }))
            .unwrap(),
        )
        .unwrap(),
        Err(error) => {
            fs::write(
                root.join("error"),
                serde_json::to_vec(&serde_json::json!({
                    "reported": is_reported_cancellation(&error), "message": error.to_string(),
                }))
                .unwrap(),
            )
            .unwrap();
            if !is_reported_cancellation(&error) {
                writeln!(
                    io::stderr().lock(),
                    "{}",
                    presentation.diagnostic(
                        crate::presentation::Severity::Error,
                        &crate::diagnostics::error_message(&error),
                    )
                )
                .unwrap();
            }
            drop(session);
            // Match the production failure outcome without a second libtest panic
            // printing the cancellation text again.
            std::process::exit(21);
        }
    }
}

fn terminal_flags() -> (
    libc::tcflag_t,
    libc::tcflag_t,
    libc::tcflag_t,
    libc::tcflag_t,
    Vec<libc::cc_t>,
) {
    let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
    // SAFETY: stdin is the fixture's live PTY; tcgetattr initializes the output
    // on success and we check that result before reading it.
    assert_eq!(
        unsafe { libc::tcgetattr(libc::STDIN_FILENO, settings.as_mut_ptr()) },
        0
    );
    let settings = unsafe { settings.assume_init() };
    (
        settings.c_iflag,
        settings.c_oflag,
        settings.c_cflag,
        // macOS marks queued input for reprocessing when console restores
        // canonical mode. PENDIN is kernel queue bookkeeping, not a changed
        // terminal setting; compare all configurable mode bits and controls.
        settings.c_lflag & !libc::PENDIN,
        settings.c_cc.to_vec(),
    )
}

enum Event {
    Data(Vec<u8>),
    Closed,
}

struct TerminalDriver {
    _lock: MutexGuard<'static, ()>,
    root: TempDir,
    child: Box<dyn Child + Send + Sync>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Option<Box<dyn Write + Send>>,
    events: Receiver<Event>,
    reader: Option<JoinHandle<()>>,
    output: Vec<u8>,
    closed: bool,
    exited: bool,
}

impl TerminalDriver {
    fn new(scenario: &str) -> Self {
        Self::with_term(scenario, "xterm-256color")
    }

    fn with_term(scenario: &str, term: &str) -> Self {
        let lock = TERMINAL.lock().unwrap_or_else(|error| error.into_inner());
        let root = tempfile::tempdir().unwrap();
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 110,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut command = CommandBuilder::new("/bin/sh");
        // Preserve queued editor/cancellation keys between console reads without
        // changing output newline handling. PATH contains no system utilities;
        // run absolute stty before stdout is redirected and before the driver
        // snapshots the exact terminal settings it must later restore.
        command.args([
            "-c",
            "/bin/stty -icanon -echo -isig min 1 time 0 || exit\nexec \"$@\" >\"$DRIVER_STDOUT\"",
            "author-driver",
        ]);
        command.arg(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "interaction::author_tests::terminal_driver",
            "--ignored",
            "--nocapture",
        ]);
        command.cwd(root.path());
        command.env("HYDRAGEN_AUTHOR_DRIVER_ROOT", root.path());
        command.env("HYDRAGEN_AUTHOR_DRIVER_CASE", scenario);
        command.env("DRIVER_STDOUT", root.path().join("stdout"));
        command.env("TERM", term);
        command.env("NO_COLOR", "1");
        command.env("CLICOLOR", "0");
        command.env_remove("CLICOLOR_FORCE");
        command.env_remove("CI");
        // No real discovery function is reachable from the injected lanes. Also
        // isolate child environment so fixture subprocesses cannot inherit auth.
        command.env("HOME", root.path());
        command.env("PATH", root.path());
        for name in [
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "GH_ENTERPRISE_TOKEN",
            "GITHUB_ENTERPRISE_TOKEN",
            "GH_HOST",
        ] {
            command.env_remove(name);
        }
        let writer = pair.master.take_writer().unwrap();
        let mut reader = pair.master.try_clone_reader().unwrap();
        let (send, events) = mpsc::channel();
        let reader_thread = thread::spawn(move || {
            let mut bytes = [0; 4096];
            while let Ok(count) = reader.read(&mut bytes) {
                if count == 0 {
                    break;
                }
                if send.send(Event::Data(bytes[..count].to_vec())).is_err() {
                    return;
                }
            }
            let _ = send.send(Event::Closed);
        });
        let child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        Self {
            _lock: lock,
            root,
            child,
            master: pair.master,
            writer: Some(writer),
            events,
            reader: Some(reader_thread),
            output: Vec::new(),
            closed: false,
            exited: false,
        }
    }

    fn receive(&mut self) {
        match self.events.recv_timeout(Duration::from_millis(10)) {
            Ok(Event::Data(bytes)) => self.output.extend(bytes),
            Ok(Event::Closed) => self.closed = true,
            Err(_) => {}
        }
    }

    fn text(&self) -> String {
        console::strip_ansi_codes(&String::from_utf8_lossy(&self.output)).replace('\r', "")
    }

    fn wait(&mut self, text: &str) {
        let until = Instant::now() + TIMEOUT;
        while !self.text().contains(text) {
            assert!(
                Instant::now() < until,
                "waiting for {text:?}: {}",
                self.text()
            );
            self.receive();
        }
    }

    fn file(&mut self, name: &str) -> String {
        let until = Instant::now() + TIMEOUT;
        loop {
            if let Ok(value) = fs::read_to_string(self.root.path().join(name))
                && !value.is_empty()
            {
                return value;
            }
            assert!(
                Instant::now() < until,
                "waiting for file {name}: {}",
                self.text()
            );
            self.receive();
        }
    }

    fn signal(&self, name: &str, value: &str) {
        let staging = self.root.path().join(format!("{name}.tmp"));
        fs::write(&staging, value).unwrap();
        fs::rename(staging, self.root.path().join(name)).unwrap();
    }

    fn send(&mut self, bytes: &[u8]) {
        let writer = self.writer.as_mut().unwrap();
        writer.write_all(bytes).unwrap();
        writer.flush().unwrap();
    }

    fn enter_name(&mut self) {
        self.wait("directory-default");
        assert!(!self.text().contains("Finding author suggestions"));
        self.send(b"\r");
    }

    fn custom(&mut self) {
        self.wait("Enter another author");
        self.send(b"\x1b[B");
        self.wait("● ✏️  Enter another author");
        let offset = self.output.len();
        self.send(b"\r");
        self.wait_text_author(offset);
    }

    fn wait_text_author(&mut self, offset: usize) {
        let until = Instant::now() + TIMEOUT;
        while !console::strip_ansi_codes(&String::from_utf8_lossy(&self.output[offset..]))
            .replace('\r', "")
            .contains("◆  Author (optional)\n│   \n")
        {
            assert!(
                Instant::now() < until,
                "waiting for fresh Author text prompt: {}",
                self.text()
            );
            self.receive();
        }
    }

    fn frame_since(&mut self, offset: usize, field: &str) -> String {
        let until = Instant::now() + TIMEOUT;
        loop {
            let fresh = console::strip_ansi_codes(&String::from_utf8_lossy(&self.output[offset..]))
                .replace('\r', "");
            if let Some((_, frame)) = fresh.rsplit_once(&format!("◆  {field} (optional)\n"))
                && frame.lines().any(|line| line.starts_with("└  "))
            {
                return frame.into();
            }
            assert!(Instant::now() < until, "fresh {field}: {}", self.text());
            self.receive();
        }
    }

    fn submit_to(&mut self, answer: &[u8], next: &str) -> String {
        let offset = self.output.len();
        self.send(answer);
        self.frame_since(offset, next)
    }

    fn contact_action(&mut self, field: &str, label: &str) -> usize {
        let mut frame = self.frame_since(0, field);
        for _ in 0..32 {
            let selected = frame
                .lines()
                .find(|line| line.starts_with("│  ● "))
                .expect("contact selector");
            if selected.ends_with(&format!("  {label}")) {
                let offset = self.output.len();
                self.send(b"\r");
                return offset;
            }
            let offset = self.output.len();
            self.send(b"\x1b[B");
            frame = self.frame_since(offset, field);
        }
        panic!("missing {label}: {frame}");
    }

    fn finish(&mut self) -> ExitStatus {
        let until = Instant::now() + TIMEOUT;
        let status = loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                break status;
            }
            assert!(
                Instant::now() < until,
                "driver exit timeout: {}",
                self.text()
            );
            self.receive();
        };
        self.exited = true;
        self.writer.take();
        while !self.closed && Instant::now() < until {
            self.receive();
        }
        if self.reader.as_ref().is_some_and(JoinHandle::is_finished) {
            self.reader.take().unwrap().join().unwrap();
        }
        assert_eq!(
            self.file("restored"),
            "yes",
            "terminal modes changed: {}",
            self.file("terminal-settings")
        );
        for format in LayerManifestFormat::ALL {
            assert!(!self.root.path().join(format.filename()).exists());
        }
        status
    }

    fn complete(&mut self) -> serde_json::Value {
        self.omit_contacts();
        self.send(b"\r");
        self.wait("jsonc");
        self.send(b"\r");
        assert!(self.finish().success(), "{}", self.text());
        serde_json::from_str(&self.file("result")).unwrap()
    }

    fn omit_contacts(&mut self) {
        let until = Instant::now() + TIMEOUT;
        let mut offset = 0;
        loop {
            let fresh = console::strip_ansi_codes(&String::from_utf8_lossy(&self.output[offset..]))
                .replace('\r', "");
            if let Some((_, frame)) = fresh.rsplit_once("◆  ") {
                let field = frame.lines().next().unwrap_or("");
                if field == "Description (optional)" {
                    return;
                }
                if matches!(field, "Author email (optional)" | "Author URL (optional)")
                    && frame.lines().any(|line| line == "└  ")
                {
                    offset = self.output.len();
                    let selected = frame.lines().find(|line| line.starts_with("│  ● "));
                    self.send(if selected.is_some_and(|line| !line.ends_with("  Skip")) {
                        b"\x1b[B"
                    } else {
                        b"\r"
                    });
                    continue;
                }
            }
            assert!(Instant::now() < until, "contact omission: {}", self.text());
            self.receive();
        }
    }

    fn cancellation(&mut self, key: &[u8]) {
        // The preamble's baseline preserves Ctrl-C even between console reads.
        // Check the expected input flags; this does not prove a read is blocked.
        let until = Instant::now() + TIMEOUT;
        loop {
            let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
            let fd = self.master.as_raw_fd().unwrap();
            assert_eq!(unsafe { libc::tcgetattr(fd, settings.as_mut_ptr()) }, 0);
            let settings = unsafe { settings.assume_init() };
            if settings.c_lflag & (libc::ICANON | libc::ISIG) == 0 {
                break;
            }
            assert!(
                Instant::now() < until,
                "terminal never acquired key-preserving input flags"
            );
            self.receive();
        }
        self.send(key);
        assert!(!self.finish().success());
        assert!(!self.root.path().join("result").exists());
        let error: serde_json::Value = serde_json::from_str(&self.file("error")).unwrap();
        assert_eq!(error["reported"], true, "{error}");
        assert_eq!(
            self.text()
                .matches("Layer initialisation cancelled. No files were changed.")
                .count(),
            1,
            "{}",
            self.text()
        );
        assert!(!self.text().contains("Description"));
    }
}

impl Drop for TerminalDriver {
    fn drop(&mut self) {
        if !self.exited {
            let _ = self.child.kill();
            let until = Instant::now() + Duration::from_secs(1);
            while Instant::now() < until {
                if self.child.try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(5));
            }
        }
        // A failed assertion must not strand the fixture's fake source process.
        if !self.root.path().join("process-finished").exists()
            && let Ok(pid) = fs::read_to_string(self.root.path().join("pid"))
            && let Ok(pid) = pid.parse::<i32>()
        {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
        self.writer.take();
        if self.reader.as_ref().is_some_and(JoinHandle::is_finished) {
            let _ = self.reader.take().unwrap().join();
        }
    }
}

#[test]
fn absolute_stty_preamble_preserves_queued_keys_without_path_utilities() {
    for cancel in [false, true] {
        let mut driver = TerminalDriver::new("custom");
        assert!(!driver.root.path().join("stty").exists());
        assert!(std::path::Path::new("/bin/stty").is_file());
        driver.wait("directory-default");
        driver.send(b"temp\x7f\x7f\x7f\x7fqueued-name\r");
        driver.custom();
        if cancel {
            // Include queued text/editor input before Ctrl-C, exercising the
            // restored baseline between reads rather than only the next key.
            driver.cancellation(b"queued\x7f\x03");
        } else {
            driver.send(b"temp\x7f\x7f\x7f\x7fQueued author\r");
            let result = driver.complete();
            assert_eq!(result["name"], "queued-name");
            assert_eq!(
                result["author"],
                serde_json::json!({"name": "Queued author"})
            );
        }
    }
}

#[test]
fn shorthand_bypasses_only_captured_fields_and_preserves_the_original_author_answer() {
    for scenario in ["empty", "custom"] {
        for (answer, email, url) in [
            ("Alex", None, None),
            ("Alex <alex@example.com>", Some("alex@example.com"), None),
            (
                "Alex (mailto:alex@example.com)",
                None,
                Some("mailto:alex@example.com"),
            ),
            (
                "Alex <alex@example.com> (https://alex.example)",
                Some("alex@example.com"),
                Some("https://alex.example"),
            ),
        ] {
            let mut driver = TerminalDriver::new(scenario);
            driver.enter_name();
            if scenario == "custom" {
                driver.custom();
            } else {
                driver.frame_since(0, "Author");
            }
            let offset = driver.output.len();
            driver.send(format!("{answer}\r").as_bytes());
            if email.is_none() {
                let frame = driver.frame_since(offset, "Author email");
                assert!(!frame.contains('●') && !frame.contains("Enter another email"));
                driver.send(b"\r");
            }
            if url.is_none() {
                let frame = driver.frame_since(offset, "Author URL");
                assert!(!frame.contains('●') && !frame.contains("Enter another URL"));
                driver.send(b"\r");
            }
            driver.wait("Description (optional)");
            let summary =
                console::strip_ansi_codes(&String::from_utf8_lossy(&driver.output[offset..]))
                    .replace('\r', "");
            assert!(summary.contains(&format!("│  {answer}\n")), "{summary}");
            assert_eq!(summary.contains("Author email (optional)"), email.is_none());
            assert_eq!(summary.contains("Author URL (optional)"), url.is_none());
            let result = driver.complete();
            let mut expected = serde_json::json!({"name":"Alex"});
            if let Some(value) = email {
                expected["email"] = value.into();
            }
            if let Some(value) = url {
                expected["url"] = value.into();
            }
            assert_eq!(result["author"], expected);
            assert!(!driver.text().contains("Finding author suggestions"));
        }
    }
}

#[test]
fn author_omission_bypasses_both_contact_steps_without_synthetic_summaries() {
    for scenario in ["empty", "custom", "contacts-one"] {
        let mut driver = TerminalDriver::new(scenario);
        driver.enter_name();
        if scenario == "custom" {
            driver.custom();
        } else if scenario == "contacts-one" {
            driver.wait("Enter another author");
            driver.send(b"\x1b[B\x1b[B");
            driver.wait("● ⏭   Skip");
        } else {
            driver.frame_since(0, "Author");
        }
        let offset = driver.output.len();
        driver.send(b"\r");
        driver.frame_since(offset, "Description");
        assert!(
            !driver.text().contains("Author email (optional)")
                && !driver.text().contains("Author URL (optional)")
        );
        assert!(driver.complete()["author"].is_null());
    }
}

#[test]
fn selected_contacts_are_independent_and_use_only_the_selected_groups_actual_sources() {
    for email_action in ["accept", "Skip", "Enter another email", "custom-empty"] {
        let mut driver = TerminalDriver::new("contacts-multi");
        driver.enter_name();
        driver.wait("Enter another author");
        let email = driver.submit_to(b"\r", "Author email");
        assert!(
            email.contains("● ◇   work@example.com (package.json author.email)"),
            "{email}"
        );
        assert!(email.contains("personal@example.com"));
        assert!(!email.contains("other@example.com"));
        assert!(!email.contains("GitHub profile fixture email"));
        let offset = driver.output.len();
        if email_action == "accept" {
            driver.send(b"\r");
        } else {
            let custom_offset = driver.contact_action(
                "Author email",
                if email_action == "custom-empty" {
                    "Enter another email"
                } else {
                    email_action
                },
            );
            if matches!(email_action, "Enter another email" | "custom-empty") {
                let custom = driver.frame_since(custom_offset, "Author email");
                assert!(!custom.contains('●'), "{custom}");
                driver.send(if email_action == "custom-empty" {
                    b"\r"
                } else {
                    b"custom@example.com\r"
                });
            }
        }
        let url = driver.frame_since(offset, "Author URL");
        for value in [
            "https://work.example",
            "https://personal.example",
            "https://github.com/fixture",
        ] {
            assert!(url.contains(value), "{url}");
        }
        assert!(!url.contains("https://other.example"));
        assert!(!url.contains("Finding author suggestions"));
        let first = driver.output.len();
        driver.send(b"\x1b[B");
        let focused = driver.frame_since(first, "Author URL");
        assert!(
            focused.contains("https://personal.example (GitHub profile fixture website)"),
            "{focused}"
        );
        let offset = driver.output.len();
        driver.send(b"\r");
        driver.frame_since(offset, "Description");
        let result = driver.complete();
        let mut expected =
            serde_json::json!({"name":"Fixture author", "url":"https://personal.example"});
        match email_action {
            "accept" => expected["email"] = "work@example.com".into(),
            "Enter another email" => expected["email"] = "custom@example.com".into(),
            _ => {}
        }
        assert_eq!(result["author"], expected);
    }
}

#[test]
fn one_contact_still_requires_selection_and_selected_name_labels_are_never_shorthand() {
    for scenario in ["contacts-one", "contacts-literal"] {
        let mut driver = TerminalDriver::new(scenario);
        driver.enter_name();
        driver.wait("Enter another author");
        let email = driver.submit_to(b"\r", "Author email");
        for value in ["work@example.com", "Enter another email", "Skip"] {
            assert!(email.contains(value));
        }
        let url = driver.submit_to(b"\r", "Author URL");
        for value in ["https://work.example", "Enter another URL", "Skip"] {
            assert!(url.contains(value));
        }
        driver.submit_to(b"\r", "Description");
        let result = driver.complete();
        assert_eq!(result["author"]["email"], "work@example.com");
        assert_eq!(result["author"]["url"], "https://work.example");
        assert_eq!(
            result["author"]["name"],
            if scenario == "contacts-literal" {
                "Fixture <literal@example.com> (https://literal.example)"
            } else {
                "Fixture author"
            }
        );
        assert!(!driver.text().contains("Finding author suggestions"));
    }
}

#[test]
fn custom_equal_name_never_recovers_suggestions_and_contact_fields_can_retry_clear_or_cancel() {
    for field in ["Author email", "Author URL"] {
        for cancel in [None, Some(b"\x1b".as_slice()), Some(b"\x03".as_slice())] {
            let mut driver = TerminalDriver::new("contacts-one");
            driver.enter_name();
            driver.custom();
            let email = driver.submit_to(b"Fixture author\r", "Author email");
            assert!(!email.contains("work@example.com") && !email.contains('●'));
            if field == "Author URL" {
                driver.submit_to(b"\r", "Author URL");
            }
            if let Some(key) = cancel {
                driver.cancellation(key);
                continue;
            }
            driver.send(b" \r");
            driver.wait("clear the field and press Enter to skip");
            let offset = driver.output.len();
            driver.send(b"\x7fnot-valid\r");
            let until = Instant::now() + TIMEOUT;
            while !console::strip_ansi_codes(&String::from_utf8_lossy(&driver.output[offset..]))
                .contains("Enter a valid")
            {
                assert!(Instant::now() < until, "{}", driver.text());
                driver.receive();
            }
            assert!(!driver.text().contains("--author"));
            driver.send(b"\x7f\x7f\x7f\x7f\x7f\x7f\x7f\x7f\x7f\r");
            if field == "Author email" {
                driver.frame_since(offset, "Author URL");
                driver.send(b"\r");
            }
            driver.frame_since(offset, "Description");
            assert_eq!(
                driver.complete()["author"],
                serde_json::json!({"name":"Fixture author"})
            );
        }
    }
}

#[test]
fn contact_selectors_and_custom_contact_inputs_cancel_without_files_or_theme_leaks() {
    for field in ["Author email", "Author URL"] {
        for custom in [false, true] {
            for key in [b"\x1b".as_slice(), b"\x03".as_slice()] {
                let mut driver = TerminalDriver::new("contacts-one");
                driver.enter_name();
                driver.wait("Enter another author");
                driver.submit_to(b"\r", "Author email");
                if field == "Author URL" {
                    driver.submit_to(b"\r", "Author URL");
                }
                if custom {
                    let offset = driver.contact_action(
                        field,
                        if field == "Author email" {
                            "Enter another email"
                        } else {
                            "Enter another URL"
                        },
                    );
                    let frame = driver.frame_since(offset, field);
                    assert!(!frame.contains('●') && !frame.contains("package.json"));
                }
                driver.cancellation(key);
            }
        }
    }
}

#[test]
fn late_contacts_freeze_with_names_without_contact_step_wait_or_spinner() {
    let mut driver = TerminalDriver::new("late-contacts");
    driver.wait("directory-default");
    driver.file("ready-to-wait");
    driver.signal("clock", "15000");
    driver.send(b"\r");
    driver.wait("Enter another author");
    driver.signal("release", "1");
    assert_eq!(driver.file("published"), "no");
    let offset = driver.output.len();
    let before = Instant::now();
    let email = driver.submit_to(b"\r", "Author email");
    assert!(before.elapsed() < Duration::from_millis(800));
    assert!(!email.contains('●') && !email.contains("late@example.com"));
    let before = Instant::now();
    let url = driver.submit_to(b"\r", "Author URL");
    assert!(before.elapsed() < Duration::from_millis(800));
    assert!(url.contains("https://github.com/fixture") && !url.contains("https://late.example"));
    driver.submit_to(b"\r", "Description");
    assert!(
        !String::from_utf8_lossy(&driver.output[offset..]).contains("Finding author suggestions")
    );
    assert_eq!(
        driver.complete()["author"],
        serde_json::json!({"name":"Fixture author", "url":"https://github.com/fixture"})
    );
}

#[test]
fn invalid_captured_contacts_retry_author_without_silent_literal_fallback_or_duplicate_rows() {
    for (answer, component) in [
        ("Alex <bad>", "author email"),
        ("Alex (https://bad host)", "author URL"),
    ] {
        let mut driver = TerminalDriver::new("empty");
        driver.enter_name();
        driver.frame_since(0, "Author");
        driver.send(format!("{answer}\r").as_bytes());
        driver.wait(&format!("Enter a valid {component}"));
        assert!(
            driver
                .text()
                .contains("clear the field and press Enter to skip")
        );
        assert!(!driver.text().contains("Description") && !driver.text().contains("--author"));
        driver.send(&vec![0x7f; answer.len()]);
        let offset = driver.output.len();
        driver.send(b"Alex <alex@example.com> (https://alex.example)\r");
        driver.frame_since(offset, "Description");
        let fresh = String::from_utf8_lossy(&driver.output[offset..]);
        assert!(
            !fresh.contains("Author email (optional)") && !fresh.contains("Author URL (optional)")
        );
        assert_eq!(
            driver.complete()["author"],
            serde_json::json!({"name":"Alex", "email":"alex@example.com", "url":"https://alex.example"})
        );
    }
}

#[test]
fn long_contact_lists_scroll_natively_and_keep_last_candidates_and_actions() {
    let mut driver = TerminalDriver::new("contacts-many");
    driver.enter_name();
    driver.wait("Enter another author");
    driver.submit_to(b"\r", "Author email");
    let selected = driver.contact_action(
        "Author email",
        "contact-15@example.com (package.json author.email)",
    );
    let url = driver.frame_since(selected, "Author URL");
    assert!(url.contains("https://example.com/00"));
    let selected = driver.contact_action(
        "Author URL",
        "https://example.com/15 (package.json author.url)",
    );
    driver.frame_since(selected, "Description");
    assert_eq!(
        driver.complete()["author"],
        serde_json::json!({"name":"Fixture author", "email":"contact-15@example.com", "url":"https://example.com/15"})
    );
}

#[test]
fn shared_optional_author_text_handles_values_empty_retry_clear_and_cancel() {
    for scenario in ["empty", "custom"] {
        for answer in [Some("  Custom author 🦀  "), None] {
            let mut driver = TerminalDriver::new(scenario);
            driver.enter_name();
            if scenario == "custom" {
                driver.custom();
            } else {
                driver.wait("Author (optional)");
                assert!(!driver.text().contains("Enter another author"));
            }
            if let Some(value) = answer {
                driver.send(value.as_bytes());
            }
            driver.send(b"\r");
            let result = driver.complete();
            assert_eq!(
                result["author"],
                answer.map_or(
                    serde_json::Value::Null,
                    |value| serde_json::json!({"name": value})
                )
            );
            assert_eq!(result["name"], "directory-default");
            assert!(!driver.text().contains("Finding author suggestions"));
        }
        let mut driver = TerminalDriver::new(scenario);
        driver.enter_name();
        if scenario == "custom" {
            driver.custom();
        } else {
            driver.wait("Author (optional)");
        }
        driver.send(b"   \r");
        driver.wait("clear the field and press Enter to skip");
        driver.send(b"\x7f\x7f\x7f\r");
        assert!(driver.complete()["author"].is_null());
        assert!(driver.text().contains("Skipped"));
        drop(driver);
        for key in [b"\x1b".as_slice(), b"\x03".as_slice()] {
            let mut driver = TerminalDriver::new(scenario);
            driver.enter_name();
            if scenario == "custom" {
                driver.custom();
            } else {
                driver.wait("Author (optional)");
            }
            driver.cancellation(key);
        }
    }
}

#[test]
fn membership_only_defaults_to_empty_custom_input() {
    let mut driver = TerminalDriver::new("membership");
    driver.enter_name();
    driver.wait("Membership organisation");
    driver.wait("● ✏️  Enter another author");
    let offset = driver.output.len();
    driver.send(b"\r");
    // The action's submitted frame is suppressed; the next Author is text.
    driver.wait_text_author(offset);
    driver.send(b"\r");
    assert!(driver.complete()["author"].is_null());
}

#[test]
#[ignore = "upstream cliclack #115: initial custom selection is hidden https://github.com/fadeevab/cliclack/issues/115"]
fn long_membership_only_selector_visibly_selects_custom_and_enter_opens_empty_input() {
    let mut driver = TerminalDriver::new("many-memberships");
    driver.enter_name();
    // Regression: cliclack 0.5.6 puts this initial cursor below its viewport.
    // This must remain a real, failing assertion until the dependency is fixed.
    driver.wait("Author (optional)");
    let offset = driver.output.len();
    driver.send(b"\r");
    driver.wait_text_author(offset);
    let initially_visible = driver.text().contains("● ✏️  Enter another author");
    driver.send(b"\r");
    let result = driver.complete();
    assert!(result["author"].is_null());
    assert!(
        initially_visible,
        "initial custom selection was not visible; empty Author input still returned {result}; transcript:\n{}",
        driver.text()
    );
}

#[test]
#[ignore = "upstream cliclack #115: Up-selected name and summary are hidden https://github.com/fadeevab/cliclack/issues/115"]
fn moving_up_from_long_membership_custom_visibly_selects_and_summarises_the_name() {
    let mut driver = TerminalDriver::new("many-memberships");
    driver.enter_name();
    driver.wait("Author (optional)");
    let offset = driver.output.len();
    // Queue both native keys: the selector must render the Up-selected name
    // before reading Enter, even when both keys are already available.
    driver.send(b"\x1b[A\r");
    let result = driver.complete();
    assert_eq!(result["author"]["name"], "Membership 19");
    let transcript = console::strip_ansi_codes(&String::from_utf8_lossy(&driver.output[offset..]))
        .replace('\r', "");
    let submit = transcript.find("◇  Author (optional)");
    let description = transcript
        .find("◆  Description")
        .expect("next Description prompt");
    let visible = transcript[..submit.unwrap_or(description)].contains("● 🏢  Membership 19");
    let summary =
        submit.is_some_and(|submit| transcript[submit..description].contains("│  Membership 19\n"));
    assert!(
        visible && summary,
        "Up-selected name visible: {visible}; submitted name visible: {summary}; accepted result: {result}; transcript:\n{transcript}"
    );
}

#[test]
fn delayed_sources_leave_name_responsive_and_ready_results_need_no_wait() {
    let mut driver = TerminalDriver::new("during-name");
    let before_name = Instant::now();
    driver.wait("directory-default");
    assert!(
        before_name.elapsed() < Duration::from_millis(800),
        "optional sources delayed the first prompt"
    );
    driver.file("started");
    assert!(!driver.root.path().join("published").exists());
    driver.send(b"   \r");
    driver.wait("Enter a non-blank name.");
    assert!(!driver.text().contains("Finding author suggestions"));
    driver.signal("release", "1");
    assert_eq!(driver.file("published"), "yes");
    driver.send(b"\x7f\x7f\x7fresponsive-name\r");
    driver.wait("Late author");
    assert!(!driver.text().contains("Finding author suggestions"));
    driver.send(b"\r");
    let result = driver.complete();
    assert_eq!(result["name"], "responsive-name");
    assert_eq!(result["author"]["name"], "Late author");
}

#[test]
fn fast_name_gets_fresh_author_wait_and_late_results_are_frozen() {
    let mut driver = TerminalDriver::new("late");
    driver.wait("directory-default");
    driver.file("ready-to-wait");
    // Advance only the background clock. Author must still get a fresh 1.5s.
    driver.signal("clock", "3000");
    let before = Instant::now();
    driver.send(b"\r");
    driver.wait("Finding author suggestions…");
    driver.wait("Enter another author");
    let elapsed = before.elapsed();
    assert!(
        elapsed >= Duration::from_millis(1400),
        "visible wait too short: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(2300),
        "visible wait too long: {elapsed:?}"
    );
    driver.signal("release", "1");
    assert_eq!(driver.file("published"), "no");
    assert!(!driver.text().contains("Late author"));
    driver.send(b"\r");
    assert_eq!(driver.complete()["author"]["name"], "Early author");
    assert!(!driver.text().contains("Warning"));
}

#[test]
fn preparation_expired_during_name_has_no_new_wait_or_spinner() {
    let mut driver = TerminalDriver::new("late");
    driver.wait("directory-default");
    driver.file("ready-to-wait");
    driver.signal("clock", "15000");
    let before = Instant::now();
    driver.send(b"\r");
    driver.wait("Enter another author");
    assert!(before.elapsed() < Duration::from_millis(800));
    assert!(!driver.text().contains("Finding author suggestions"));
    driver.signal("release", "1");
    assert_eq!(driver.file("published"), "no");
    driver.send(b"\r");
    assert_eq!(driver.complete()["author"]["name"], "Early author");
}

#[test]
fn name_and_wait_cancellation_close_and_reap_actual_preparation_children() {
    for at_wait in [false, true] {
        for key in [b"\x1b".as_slice(), b"\x03".as_slice()] {
            let mut driver = TerminalDriver::new("process");
            driver.wait("directory-default");
            let pid: i32 = driver.file("pid").parse().unwrap();
            if at_wait {
                driver.send(b"\r");
                driver.wait("Finding author suggestions…");
            }
            driver.cancellation(key);
            assert!(!driver.text().contains("Author (optional)"));
            assert_eq!(driver.file("process-finished"), "1");
            // The source runner, not the outer harness's Drop fallback, reaped it.
            assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "source process survived");
        }
    }
}

#[test]
fn completing_a_pending_source_clears_wait_before_the_next_prompt() {
    let mut driver = TerminalDriver::new("pending");
    driver.enter_name();
    driver.wait("Finding author suggestions…");
    let before = Instant::now();
    driver.signal("release", "1");
    driver.wait("Enter another author");
    assert!(
        before.elapsed() < Duration::from_millis(800),
        "ready results were artificially delayed"
    );
    assert_eq!(driver.file("published"), "yes");
    driver.send(b"\r");
    assert_eq!(driver.complete()["author"]["name"], "Late author");
    let text = driver.text();
    let author = text.find("◆  Author (optional)").unwrap();
    assert!(!text[author..].contains("Finding author suggestions"));
    assert!(!text.contains("Warning"));
}

#[test]
fn visible_wait_timeout_cleans_up_the_child_while_author_remains_usable() {
    let mut driver = TerminalDriver::new("process");
    driver.wait("directory-default");
    let pid: i32 = driver.file("pid").parse().unwrap();
    driver.send(b"\r");
    driver.wait("Finding author suggestions…");
    driver.wait("Author (optional)");
    assert_eq!(driver.file("process-finished"), "1");
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert!(!driver.text().contains("Enter another author"));
    driver.send(b"Still usable\r");
    assert_eq!(driver.complete()["author"]["name"], "Still usable");
}

#[test]
fn poll_failure_during_real_terminal_wait_clears_restores_and_cleans_up_without_cancelling() {
    for (scenario, term) in [
        ("wait-error", "xterm-256color"),
        ("wait-interrupted", "dumb"),
    ] {
        let mut driver = TerminalDriver::with_term(scenario, term);
        driver.wait("directory-default");
        let pid: i32 = driver.file("pid").parse().unwrap();
        driver.send(b"\r");
        driver.wait("Finding author suggestions…");
        let before_failure = Instant::now();
        driver.signal("fail-poll", "1");
        assert!(!driver.finish().success());
        assert!(
            before_failure.elapsed() < Duration::from_millis(800),
            "wait failure waited for the suggestion deadline"
        );
        let error: serde_json::Value = serde_json::from_str(&driver.file("error")).unwrap();
        assert_eq!(error["reported"], false, "{error}");
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("Could not collect initialisation input")
        );
        assert_eq!(driver.file("closed-after-process"), "yes");
        assert_eq!(driver.file("process-finished"), "1");
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            -1,
            "source process survived poll failure"
        );
        assert!(!driver.root.path().join("result").exists());
        let text = driver.text();
        assert!(
            text.contains("Error: Could not collect initialisation input"),
            "{text}"
        );
        assert_eq!(text.matches("Error:").count(), 1, "{text}");
        assert!(text.contains("No files were changed."));
        assert!(!text.contains("Layer initialisation cancelled"));
        assert!(!text.contains("Author (optional)"));
        assert!(!text.contains("Description"));
        let raw = String::from_utf8_lossy(&driver.output);
        let indicator_end =
            raw.rfind("Finding author suggestions…").unwrap() + "Finding author suggestions…".len();
        let error_start = raw[indicator_end..].find("Error:").unwrap() + indicator_end;
        assert!(
            raw[indicator_end..error_start].contains("\x1b[2K"),
            "indicator was not cleared before the error: {raw:?}"
        );
    }
}

#[test]
fn wait_without_colour_and_basic_terminal_keeps_meaning_and_cancellation() {
    for term in ["xterm-256color", "dumb"] {
        let mut driver = TerminalDriver::with_term("pending", term);
        driver.enter_name();
        driver.wait("Finding author suggestions…");
        driver.cancellation(b"\x1b");
        assert!(!String::from_utf8_lossy(&driver.output).contains("\x1b[36m"));
    }
}

#[test]
fn fatal_preparation_does_not_become_an_empty_author_prompt() {
    let mut driver = TerminalDriver::new("fatal");
    driver.enter_name();
    assert!(!driver.finish().success());
    let error: serde_json::Value = serde_json::from_str(&driver.file("error")).unwrap();
    assert_eq!(error["reported"], false);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("injected author failure")
    );
    assert!(!driver.text().contains("Author (optional)"));
    assert!(!driver.text().contains("Finding author suggestions"));
}
