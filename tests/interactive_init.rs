#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::os::fd::BorrowedFd;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use hydragen::LayerManifestFormat;
use portable_pty::{
    Child, CommandBuilder, ExitStatus, MasterPty, PtyPair, PtySize, native_pty_system,
};
use serde_json::{Value, json};
use tempfile::tempdir;

#[path = "support/terminal_identity.rs"]
mod terminal_identity;
use terminal_identity::{DEFAULT_AUTHOR, IdentityFixture};

const TIMEOUT: Duration = Duration::from_secs(10);
// Bound terminal allocation independently of the test runner's thread count.
// Some macOS environments reject overlapping openpty calls.
static TERMINAL_SESSION: Mutex<()> = Mutex::new(());

enum Event {
    Data(bool, Vec<u8>),
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputMode {
    Canonical,
    Noncanonical,
}

fn input_mode(input: &dyn MasterPty) -> Option<InputMode> {
    input.get_termios().map(|settings| {
        // Use the flag's supported name instead of platform-specific bit values
        // or a direct dependency on portable-pty's termios implementation.
        if settings
            .local_flags
            .iter_names()
            .any(|(name, _)| name == "ICANON")
        {
            InputMode::Canonical
        } else {
            InputMode::Noncanonical
        }
    })
}

fn noncanonical_input_ready(
    mode: Option<InputMode>,
    child_status: Option<&ExitStatus>,
    before_deadline: bool,
) -> Result<bool, String> {
    let mode = mode.ok_or_else(|| {
        "Could not observe dedicated input PTY termios: get_termios returned None".to_owned()
    })?;
    if let Some(status) = child_status {
        return Err(format!(
            "Terminal child exited before dedicated input PTY readiness: {status:?}"
        ));
    }
    if !before_deadline {
        return Err(format!(
            "Timed out waiting for dedicated input PTY to become noncanonical; last mode: {mode:?}"
        ));
    }
    Ok(mode == InputMode::Noncanonical)
}

#[test]
fn noncanonical_input_readiness_distinguishes_pending_ready_and_expired_states() {
    assert_eq!(
        noncanonical_input_ready(Some(InputMode::Canonical), None, true),
        Ok(false)
    );
    assert_eq!(
        noncanonical_input_ready(Some(InputMode::Noncanonical), None, true),
        Ok(true)
    );
    for mode in [InputMode::Canonical, InputMode::Noncanonical] {
        let error = noncanonical_input_ready(Some(mode), None, false).unwrap_err();
        assert!(
            error.contains("Timed out waiting for dedicated input PTY"),
            "{error}"
        );
        assert!(error.contains(&format!("last mode: {mode:?}")), "{error}");
    }
}

#[test]
fn noncanonical_input_readiness_rejects_missing_termios_and_exited_children() {
    let error = noncanonical_input_ready(None, None, true).unwrap_err();
    assert!(
        error.contains("Could not observe dedicated input PTY termios"),
        "{error}"
    );
    assert!(error.contains("get_termios returned None"), "{error}");

    let status = ExitStatus::with_exit_code(1);
    for mode in [InputMode::Canonical, InputMode::Noncanonical] {
        let error = noncanonical_input_ready(Some(mode), Some(&status), true).unwrap_err();
        assert!(
            error.contains("Terminal child exited before dedicated input PTY readiness"),
            "{error}"
        );
        assert!(error.contains(&format!("{status:?}")), "{error}");
    }
}

struct TerminalSettings {
    cols: u16,
    rows: u16,
    colour: Option<bool>,
    term: &'static str,
    no_color: Option<&'static str>,
    clicolor: Option<&'static str>,
    identity: Option<IdentityFixture>,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            cols: 120,
            rows: 24,
            colour: Some(true),
            term: "xterm-256color",
            no_color: None,
            clicolor: None,
            identity: None,
        }
    }
}

// Keep stdout and stderr on distinct terminals so stream assertions exercise
// actual cliclack output rather than relying on a combined terminal transcript.
struct Session {
    identity: IdentityFixture,
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
        Self::with_settings(
            directory,
            arguments,
            ci,
            redirect,
            TerminalSettings::default(),
        )
    }

    fn with_settings(
        directory: &Path,
        arguments: &[&str],
        ci: Option<&str>,
        redirect: Option<(&str, &Path)>,
        settings: TerminalSettings,
    ) -> Self {
        let terminal_session = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Self::spawn_with_lock(
            directory,
            arguments,
            ci,
            redirect,
            terminal_session,
            settings,
        )
    }

    fn spawn_with_lock(
        directory: &Path,
        arguments: &[&str],
        ci: Option<&str>,
        redirect: Option<(&str, &Path)>,
        terminal_session: MutexGuard<'static, ()>,
        settings: TerminalSettings,
    ) -> Self {
        let system = native_pty_system();
        let size = PtySize {
            rows: settings.rows,
            cols: settings.cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let terminal = system.openpty(size).unwrap();
        let stdout_terminal = system.openpty(size).unwrap();
        let mut command = CommandBuilder::new("/bin/sh");
        let identity = settings.identity.unwrap_or_else(IdentityFixture::new);
        identity.apply(&mut command);
        // console restores the previous input mode between key reads. Preserve
        // queued Backspace/Ctrl-C without changing output newline handling. Use
        // the system utility explicitly: PATH contains only fixture executables.
        // Run before redirection so a dedicated stdin PTY stays canonical until
        // console actually selects it for input.
        command.args([
            "-c",
            "/bin/stty -icanon -echo -isig min 1 time 0 || exit
case \"$REDIRECT\" in
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
        command.env("TERM", settings.term);
        command.env("RESULT_TTY", stdout_terminal.master.tty_name().unwrap());
        command.env_remove("CI");
        command.env_remove("REDIRECT");
        command.env_remove("COLUMNS");
        command.env_remove("CLICOLOR_FORCE");
        command.env_remove("NO_COLOR");
        command.env_remove("CLICOLOR");
        if let Some(colour) = settings.colour {
            command.env_remove("CLICOLOR_FORCE");
            command.env_remove("NO_COLOR");
            command.env_remove("CLICOLOR");
            if colour {
                command.env("CLICOLOR_FORCE", "1");
            } else {
                command.env("NO_COLOR", "1");
                command.env("CLICOLOR", "0");
            }
        }
        if let Some(value) = settings.no_color {
            command.env("NO_COLOR", value);
        }
        if let Some(value) = settings.clicolor {
            command.env("CLICOLOR", value);
        }
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
            identity,
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

    fn wait_for_noncanonical_input(&mut self, input: &dyn MasterPty) {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let mode = input_mode(input);
            let status = self.child.try_wait().expect(
                "Could not observe terminal child status while waiting for dedicated input PTY readiness",
            );
            match noncanonical_input_ready(mode, status.as_ref(), Instant::now() < deadline) {
                Ok(true) => return,
                Ok(false) => self.receive(
                    Duration::from_millis(10)
                        .min(deadline.saturating_duration_since(Instant::now())),
                ),
                Err(error) => panic!(
                    "{error}; closed readers: {}/{}; stderr: {}; stdout: {}",
                    self.closed,
                    self.readers.len(),
                    String::from_utf8_lossy(&self.stderr),
                    String::from_utf8_lossy(&self.stdout)
                ),
            }
        }
    }

    fn author_frame_since(&mut self, offset: usize) -> String {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let text = visible_text(&self.stderr[offset..]);
            if let Some((_, frame)) = text.rsplit_once("Author (optional)")
                && frame.lines().any(|line| line.trim_start().starts_with('└'))
            {
                return frame.to_owned();
            }
            assert!(
                Instant::now() < deadline,
                "Timed out waiting for a complete Author frame: {}",
                String::from_utf8_lossy(&self.stderr)
            );
            self.receive(Duration::from_millis(10));
        }
    }

    fn author_frame(&mut self) -> String {
        self.author_frame_since(0)
    }

    fn choose_author(&mut self, label: &str, action: bool) {
        let mut frame = self.author_frame();
        for _ in 0..128 {
            let selected = frame
                .lines()
                .find_map(|line| line.split_once('●').map(|(_, item)| item.trim()))
                .expect("Author must be a native selector");
            // Actions have no source hint. A candidate literally named Skip or
            // Enter another author has a hint and is a different typed value.
            if (action && selected == label)
                || (!action && selected.starts_with(&format!("{label} (")))
            {
                self.send(b"\r");
                return;
            }
            let offset = self.stderr.len();
            self.send(b"\x1b[B");
            frame = self.author_frame_since(offset);
        }
        panic!("Could not select author {label:?}: {frame}");
    }

    fn custom_author(&mut self) {
        self.choose_author("Enter another author", true);
        // The action is followed by another Author prompt, never editable
        // prefilling. Wait for its complete frame before sending text.
        let offset = self.stderr.len();
        let frame = self.author_frame_since(offset);
        assert!(!frame.contains('●'), "{frame}");
        assert!(!frame.contains(DEFAULT_AUTHOR), "{frame}");
    }

    fn skip_author(&mut self) {
        self.choose_author("Skip", true);
        self.wait_for("Description (optional");
    }

    fn choose_format(&mut self, directory: &Path, format: LayerManifestFormat) {
        self.wait_for("Layer manifest format");
        self.wait_for("jsonc");
        assert_no_layer_manifests(directory);
        for _ in 0..LayerManifestFormat::ALL
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
                "Terminal child did not finish (child status: {status:?}, closed readers: {}/{}): {}",
                self.closed,
                self.readers.len(),
                String::from_utf8_lossy(&self.stderr)
            );
            self.receive(Duration::from_millis(10));
        }
    }

    fn assert_no_prompts(&self) {
        let stderr = visible_text(&self.stderr);
        self.assert_no_header();
        assert!(!stderr.contains("Author (optional"), "{stderr}");
        assert!(!stderr.contains("Layer manifest format"), "{stderr}");
        for frame in ['┌', '│', '└', '◇', '◆'] {
            assert!(!stderr.contains(frame), "{stderr}");
        }
    }

    fn assert_no_header(&self) {
        for output in [&self.stderr, &self.stdout] {
            let text = visible_text(output);
            assert!(!text.contains("Hydragen"), "{text}");
            assert!(!text.lines().any(|line| line.trim() == "INIT"), "{text}");
            assert!(
                !text.chars().any(|c| ('\u{2800}'..='\u{28ff}').contains(&c)),
                "{text}"
            );
        }
    }

    fn assert_plain(&self) {
        let stderr = String::from_utf8_lossy(&self.stderr);
        assert!(!self.stderr.contains(&b'\x1b'), "{stderr}");
        assert!(!self.stdout.contains(&b'\x1b'));
        for output in [&self.stderr, &self.stdout] {
            let text = visible_text(output);
            for icon in ['✓', '▲', '✕'] {
                assert!(!text.contains(icon), "{text}");
            }
        }
    }

    fn assert_guided_success(&self, directory: &Path, format: LayerManifestFormat) {
        assert!(
            self.stdout.is_empty(),
            "{:?}",
            String::from_utf8_lossy(&self.stdout)
        );
        let text = visible_text(&self.stderr);
        let completion = format!(
            "Layer initialised, layer manifest created at: {}",
            format.filename()
        );
        assert_eq!(text.matches(&completion).count(), 1, "{text}");
        assert_eq!(text.matches("Layer initialised").count(), 1, "{text}");
        let completion_line = text
            .lines()
            .find(|line| line.contains(&completion))
            .unwrap();
        assert!(
            !completion_line.contains(&directory.display().to_string()),
            "{completion_line}"
        );
        assert!(!text.contains("Initialised layer:"), "{text}");
    }

    fn assert_compact_success(&self, format: LayerManifestFormat, styled: bool) {
        self.assert_no_prompts();
        assert!(self.stderr.is_empty());
        let expected = format!(
            "Layer initialised, layer manifest created at: {}\r\n",
            format.filename()
        );
        let text = visible_text(&self.stdout);
        assert_eq!(
            text,
            if styled {
                format!("✓ {expected}")
            } else {
                expected
            }
        );
        if styled {
            assert_style(&self.stdout, "✓", |style| style.foreground == Some(32));
            assert_style(
                &self.stdout,
                "Layer initialised, layer manifest created at:",
                |style| style.foreground == Some(32),
            );
            assert_style(&self.stdout, format.filename(), |style| {
                style.bold && style.foreground == Some(32)
            });
        } else {
            self.assert_plain();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.child.kill();
            let deadline = Instant::now() + Duration::from_millis(500);
            while Instant::now() < deadline {
                if !matches!(self.child.try_wait(), Ok(None)) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        self.writer.take();
        self.stdout_terminal.take();
        for reader in self.readers.drain(..) {
            // A descendant retaining the PTY must not hang the test runner.
            if reader.is_finished() {
                let _ = reader.join();
            }
        }
    }
}

// Match visible text even when cliclack styles parts of labels or defaults.
fn visible_text(bytes: &[u8]) -> String {
    console::strip_ansi_codes(&String::from_utf8_lossy(bytes)).into_owned()
}

#[derive(Clone, Copy, Debug, Default)]
struct TextStyle {
    foreground: Option<u16>,
    bold: bool,
    dim: bool,
    italic: bool,
}

// Inspect accepted SGR byte spans, not terminal pixels. Cursor movement used by
// cliclack's editor is deliberately independent from colour/style eligibility.
fn styled_characters(bytes: &[u8]) -> Vec<(char, TextStyle)> {
    let raw = String::from_utf8_lossy(bytes);
    let mut chars = raw.chars().peekable();
    let mut style = TextStyle::default();
    let mut result = Vec::new();
    while let Some(character) = chars.next() {
        if character == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            let mut parameters = String::new();
            for control in chars.by_ref() {
                if ('@'..='~').contains(&control) {
                    if control == 'm' {
                        for code in parameters
                            .split(';')
                            .map(|code| code.parse::<u16>().unwrap_or(0))
                        {
                            match code {
                                0 => style = TextStyle::default(),
                                1 => style.bold = true,
                                2 => style.dim = true,
                                3 => style.italic = true,
                                22 => {
                                    style.bold = false;
                                    style.dim = false;
                                }
                                23 => style.italic = false,
                                30..=37 | 90..=97 => style.foreground = Some(code),
                                39 => style.foreground = None,
                                _ => {}
                            }
                        }
                    }
                    break;
                }
                parameters.push(control);
            }
        } else {
            result.push((character, style));
        }
    }
    result
}

fn assert_style(bytes: &[u8], text: &str, matches: impl Fn(TextStyle) -> bool) {
    let characters = styled_characters(bytes);
    let needle: Vec<_> = text.chars().collect();
    let matching: Vec<_> = characters
        .windows(needle.len())
        .filter(|window| window.iter().map(|(c, _)| *c).eq(needle.iter().copied()))
        .collect();
    assert!(
        !matching.is_empty(),
        "Missing {text:?}: {}",
        String::from_utf8_lossy(bytes)
    );
    for window in matching {
        assert!(
            window.iter().all(|(_, style)| matches(*style)),
            "Incorrect style for {text:?}: {window:?}"
        );
    }
}

fn assert_no_sgr(bytes: &[u8]) {
    assert!(
        styled_characters(bytes).iter().all(|(_, style)| {
            style.foreground.is_none() && !style.bold && !style.dim && !style.italic
        }),
        "{}",
        String::from_utf8_lossy(bytes)
    );
    let raw = String::from_utf8_lossy(bytes);
    assert!(
        !raw.split("\x1b[").skip(1).any(|part| part
            .find('m')
            .is_some_and(|end| { part[..end].chars().all(|c| c.is_ascii_digit() || c == ';') })),
        "{raw}"
    );
}

fn selector_summary(transcript: &str) -> &str {
    // The subsequent completion legitimately names layer.<extension>; only the
    // completed selector line must keep its extension-only choice label.
    transcript
        .rsplit_once("Layer manifest format")
        .unwrap()
        .1
        .lines()
        .find(|line| {
            line.split_whitespace().any(|word| {
                LayerManifestFormat::ALL
                    .iter()
                    .any(|format| word == format.extension())
            })
        })
        .unwrap()
}

fn author_summary(transcript: &str) -> &str {
    transcript
        .split_once("Description (optional)")
        .unwrap()
        .0
        .rsplit_once("Author (optional)")
        .unwrap()
        .1
}

fn github_fixture(organisations: Value) -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.github(
        json!({"name": "Fixture GitHub person", "login": "fixture-person"}),
        json!({"data": {"viewer": {
            "login": "fixture-person",
            "organizations": {
                "nodes": organisations,
                "pageInfo": {"hasNextPage": false, "endCursor": null}
            }
        }}}),
    );
    fixture
}

fn finish_after_author(session: &mut Session, directory: &Path) {
    session.wait_for("Description (optional");
    assert_no_layer_manifests(directory);
    session.send(b"\r");
    session.choose_format(directory, LayerManifestFormat::Yaml);
    assert!(session.finish().success());
    session.assert_guided_success(directory, LayerManifestFormat::Yaml);
}

fn layer_manifest(directory: &Path) -> Value {
    layer_manifest_in_format(directory, LayerManifestFormat::Yaml)
}

// Decode independently of Hydragen's loader so matching reader/writer bugs cannot
// hide malformed output or a format field accidentally stored as metadata.
fn layer_manifest_in_format(directory: &Path, format: LayerManifestFormat) -> Value {
    let text = fs::read_to_string(directory.join(format.filename())).unwrap();
    assert!(text.ends_with('\n'));
    let document = match format {
        LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
            serde_saphyr::from_str(&text).unwrap()
        }
        LayerManifestFormat::Toml => {
            let table: toml::Table = toml::from_str(&text).unwrap();
            serde_json::to_value(table).unwrap()
        }
        LayerManifestFormat::Json | LayerManifestFormat::Jsonc => {
            serde_json::from_str(&text).unwrap()
        }
    };
    hydragen::layer_manifest::schema::validate(&document).unwrap();
    document
}

fn assert_no_layer_manifests(directory: &Path) {
    for format in LayerManifestFormat::ALL {
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

fn assert_occupied_feedback(stderr: &[u8]) {
    let text = visible_text(stderr);
    assert!(
        text.contains("This directory already has a layer manifest ("),
        "{text}"
    );
    let refusal = text.split_once("Error: ").unwrap().1;
    assert_eq!(refusal.lines().count(), 2, "{text}");
    let advice = refusal.lines().nth(1).unwrap().trim();
    assert_eq!(
        advice,
        "Initialisation creates a new layer manifest; it won’t overwrite an existing one."
    );
    for unsafe_advice in ["delete", "remove", "--force"] {
        assert!(!advice.contains(unsafe_advice), "{text}");
    }
}

// A black-box copy of the agreed character grid, independent of the runtime
// asset. These are character-cell assertions, not a promise about font pixels.
const MOTIF: [&str; 8] = [
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⢠⣶⣿⣿⣶⡄⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⣿⣿⡁⠈⠻⣿⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠘⠿⣿⣦⢀⣴⣄⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠙⠟⠁⠻⣿⣶⡄⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⣿⣦⡀⢈⣿⣿⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠘⠿⣿⣿⠿⠃⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
];

#[test]
fn guided_header_preserves_the_grid_hierarchy_and_once_per_session_opening() {
    let full_width = 17
        + format!("Hydragen  v{}", env!("CARGO_PKG_VERSION"))
            .len()
            .max("Initialise a layer".len());
    for (cols, term, full) in [
        (120, "xterm-256color", true),
        (full_width as u16, "xterm-256color", true),
        (full_width as u16 - 1, "xterm-256color", false),
        (120, "dumb", false),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::with_settings(
            directory.path(),
            &["init"],
            None,
            None,
            TerminalSettings {
                cols,
                term,
                ..Default::default()
            },
        );
        session.wait_for("Name");
        let opening = visible_text(&session.stderr).replace('\r', "");
        assert!(
            opening.contains(&format!("Hydragen  v{}", env!("CARGO_PKG_VERSION"))),
            "{opening}"
        );
        assert!(opening.contains("INIT"), "{opening}");
        assert!(opening.contains("Initialise a layer"), "{opening}");
        if full {
            let lines: Vec<_> = opening.lines().collect();
            let start = lines.iter().position(|line| *line == MOTIF[0]).unwrap();
            for (row, motif) in MOTIF.iter().enumerate() {
                assert_eq!(motif.chars().count(), 16);
                let suffix = match row {
                    2 => format!(" Hydragen  v{}", env!("CARGO_PKG_VERSION")),
                    4 => " INIT".to_owned(),
                    5 => " Initialise a layer".to_owned(),
                    _ => String::new(),
                };
                assert_eq!(lines[start + row], format!("{motif}{suffix}"));
            }
            assert_style(&session.stderr, "Hydragen", |style| {
                style.bold && style.foreground.is_none() && !style.dim
            });
            assert_style(
                &session.stderr,
                &format!("v{}", env!("CARGO_PKG_VERSION")),
                |style| style.dim,
            );
            assert_style(&session.stderr, "INIT", |style| {
                style.bold && style.foreground == Some(36)
            });
            assert_style(&session.stderr, "Initialise a layer", |style| {
                !style.bold && !style.dim && style.foreground.is_none()
            });
            assert_style(&session.stderr, "Name", |style| style.bold);
            assert_style(&session.stderr, "⣿", |style| style.foreground == Some(36));
        } else {
            assert!(
                !opening
                    .chars()
                    .any(|c| ('\u{2800}'..='\u{28ff}').contains(&c)),
                "{opening}"
            );
        }
        session.send(b"example\r");
        session.skip_author();
        session.wait_for("Description (optional");
        session.send(b"\r");
        session.choose_format(directory.path(), LayerManifestFormat::Yaml);
        assert!(session.finish().success());
        session.assert_guided_success(directory.path(), LayerManifestFormat::Yaml);
        let text = visible_text(&session.stderr);
        assert_eq!(text.matches("Hydragen").count(), 1, "{text}");
        assert_eq!(text.matches("Initialise a layer").count(), 1, "{text}");
        assert_eq!(layer_manifest(directory.path()), json!({"name": "example"}));
        assert_only_files(directory.path(), &["layer.yaml"]);
    }
}

#[test]
fn explicit_colour_disables_beat_force_without_disabling_guided_questions() {
    for settings in [
        TerminalSettings {
            no_color: Some("1"),
            ..Default::default()
        },
        TerminalSettings {
            clicolor: Some("0"),
            ..Default::default()
        },
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::with_settings(directory.path(), &["init"], None, None, settings);
        session.wait_for("Name");
        session.send(b" \r");
        session.wait_for("Enter a non-blank name.");
        assert_no_layer_manifests(directory.path());
        session.send(b"\x7fexample\r");
        session.skip_author();
        session.wait_for("Description (optional");
        session.send(b"\r");
        session.choose_format(directory.path(), LayerManifestFormat::Jsonc);
        assert!(session.finish().success());
        session.assert_guided_success(directory.path(), LayerManifestFormat::Jsonc);
        assert_no_sgr(&session.stderr);
        assert_eq!(
            layer_manifest_in_format(directory.path(), LayerManifestFormat::Jsonc),
            json!({"name": "example"})
        );
        assert_only_files(directory.path(), &["layer.jsonc"]);
    }
}

#[test]
fn compact_terminal_statuses_obey_disable_preferences_even_with_forced_colour() {
    for (settings, styled) in [
        (TerminalSettings::default(), true),
        (
            TerminalSettings {
                no_color: Some(""),
                ..Default::default()
            },
            true,
        ),
        (
            TerminalSettings {
                no_color: Some("1"),
                ..Default::default()
            },
            false,
        ),
        (
            TerminalSettings {
                clicolor: Some("0"),
                ..Default::default()
            },
            false,
        ),
        (
            TerminalSettings {
                term: "dumb",
                ..Default::default()
            },
            false,
        ),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::with_settings(
            directory.path(),
            &["--non-interactive", "init"],
            None,
            None,
            settings,
        );
        assert!(session.finish().success());
        session.assert_compact_success(LayerManifestFormat::Yaml, styled);
        assert_eq!(
            layer_manifest(directory.path()),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
        assert_only_files(directory.path(), &["layer.yaml"]);
    }
}

#[test]
fn colour_disabled_cancellation_is_reported_once_without_success_or_writes() {
    for settings in [
        TerminalSettings {
            no_color: Some("1"),
            ..Default::default()
        },
        TerminalSettings {
            clicolor: Some("0"),
            ..Default::default()
        },
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::with_settings(directory.path(), &["init"], None, None, settings);
        session.wait_for("Name");
        session.send(b"\x1b");
        assert!(!session.finish().success());
        let text = visible_text(&session.stderr);
        assert_eq!(text.matches("cancelled").count(), 1, "{text}");
        assert_eq!(
            text.to_lowercase().matches("no files were changed").count(),
            1,
            "{text}"
        );
        assert!(!text.contains("Error:"), "{text}");
        assert!(!text.contains("Layer initialised"), "{text}");
        assert!(session.stdout.is_empty());
        assert_no_sgr(&session.stderr);
        assert_no_layer_manifests(directory.path());
        assert_only_files(directory.path(), &[]);
    }
}

#[test]
fn warning_and_error_headlines_are_styled_only_in_eligible_terminals() {
    for (ci, redirect, no_color, clicolor, term, styled) in [
        (None, None, None, None, "xterm-256color", true),
        (Some("true"), None, None, None, "xterm-256color", false),
        (None, Some("stdin"), None, None, "xterm-256color", false),
        (None, Some("stdout"), None, None, "xterm-256color", false),
        (None, Some("stderr"), None, None, "xterm-256color", false),
        (None, None, Some("1"), None, "xterm-256color", false),
        (None, None, None, Some("0"), "xterm-256color", false),
        (None, None, None, None, "dumb", false),
    ] {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("layer.yaml"), "keep yaml").unwrap();
        fs::write(directory.path().join("layer.json"), "keep json").unwrap();
        let capture_directory = tempdir().unwrap();
        let capture = capture_directory.path().join("output");
        let mut session = Session::with_settings(
            directory.path(),
            &["init"],
            ci,
            redirect.map(|stream| (stream, capture.as_path())),
            TerminalSettings {
                no_color,
                clicolor,
                term,
                ..Default::default()
            },
        );
        assert!(!session.finish().success());
        session.assert_no_prompts();
        assert!(session.stdout.is_empty());
        let diagnostics = if redirect == Some("stderr") {
            fs::read(&capture).unwrap()
        } else {
            session.stderr.clone()
        };
        let text = visible_text(&diagnostics);
        assert_eq!(text.matches("Warning:").count(), 1, "{text}");
        assert_eq!(text.matches("Error:").count(), 1, "{text}");
        assert!(text.find("Warning:").unwrap() < text.find("Error:").unwrap());
        assert_occupied_feedback(&diagnostics);
        if styled {
            assert_style(&diagnostics, "▲ Warning:", |style| {
                style.foreground == Some(33)
            });
            assert_style(&diagnostics, "✕ Error:", |style| {
                style.foreground == Some(31)
            });
            assert_style(&diagnostics, "layer.yaml takes priority", |style| {
                style.foreground.is_none() && !style.dim
            });
            assert_style(&diagnostics, "Layer manifests are not merged", |style| {
                style.foreground.is_none() && !style.dim
            });
            assert_style(
                &diagnostics,
                "Initialisation creates a new layer manifest; it won’t overwrite an existing one.",
                |style| style.foreground.is_none() && !style.dim,
            );
        } else {
            session.assert_plain();
            assert!(!diagnostics.contains(&b'\x1b'), "{text}");
            for icon in ['▲', '✕'] {
                assert!(!text.contains(icon), "{text}");
            }
        }
        assert_eq!(
            fs::read_to_string(directory.path().join("layer.yaml")).unwrap(),
            "keep yaml"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("layer.json")).unwrap(),
            "keep json"
        );
        assert_only_files(directory.path(), &["layer.yaml", "layer.json"]);
    }
}

#[test]
fn help_adapts_to_terminal_width_without_prompts_styling_or_writes() {
    for cols in [40, 80, 100, 160] {
        for init in [false, true] {
            for flag in ["-h", "--help"] {
                for colour in [false, true] {
                    let directory = tempdir().unwrap();
                    let arguments = if init { vec!["init", flag] } else { vec![flag] };
                    let mut session = Session::with_settings(
                        directory.path(),
                        &arguments,
                        None,
                        None,
                        TerminalSettings {
                            cols,
                            colour: Some(colour),
                            ..Default::default()
                        },
                    );
                    assert!(session.finish().success());
                    session.assert_no_header();
                    assert!(session.stderr.is_empty());
                    assert!(!session.stdout.contains(&b'\x1b'));
                    let help = visible_text(&session.stdout);
                    assert!(help.contains("Usage:"));
                    for line in help.lines() {
                        assert!(
                            line.len() <= usize::from(cols.min(100)),
                            "{cols} columns: {line:?}"
                        );
                    }
                    if init {
                        assert!(help.contains("--name <NAME>\r\n"));
                        assert!(help.contains("hydragen init --name shared-rules\r\n"));
                        assert!(help.contains("--author \"Example team\" \\\r\n"));
                        let prose = help.split_whitespace().collect::<Vec<_>>().join(" ");
                        assert!(
                            prose.contains(
                                "Any input flag: flag-only input; no follow-up questions."
                            )
                        );
                        assert!(prose.contains(
                            "Input flags are --name, --author, --description and --format."
                        ));
                        assert!(prose.contains("Format defaults to yaml (layer.yaml)."));
                        for expected in [
                            "Guided Author offers project-declared authors first; other suggestions can come from Git, local accounts and optional GitHub lookups",
                            "with grouped source hints",
                            "Choose a name, Enter another author, or Skip",
                            "With no suggestions or after Enter another author, use an empty optional input; empty text and Skip omit author",
                            "Optional GitHub requests use only existing gh authentication, never login or extra permissions",
                            "Offline use and use without gh are supported",
                            "Flag-only input performs no author discovery",
                        ] {
                            assert!(prose.contains(expected), "{help}");
                        }
                        assert!(prose.contains("yaml, yml, toml, json, jsonc"));
                        assert!(help.contains("hydragen init --format json\r\n"));
                    }
                    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
                }
            }
        }
    }
}

#[test]
fn terminal_version_and_argument_errors_remain_plain_and_write_free() {
    for (arguments, success) in [
        (vec!["--version"], true),
        (vec!["-V"], true),
        (vec![], false),
        (vec!["init", "--bogus"], false),
        (vec!["init", "--name"], false),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::with_settings(
            directory.path(),
            &arguments,
            None,
            None,
            TerminalSettings {
                cols: 40,
                colour: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(session.finish().success(), success);
        session.assert_no_header();
        assert!(!session.stdout.contains(&b'\x1b'));
        assert!(!session.stderr.contains(&b'\x1b'));
        if success {
            assert!(session.stderr.is_empty());
            assert_eq!(
                visible_text(&session.stdout),
                format!("{}\r\n", env!("CARGO_PKG_VERSION"))
            );
        } else {
            assert!(session.stdout.is_empty());
            assert!(visible_text(&session.stderr).contains("help"));
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn guided_input_and_single_completion_use_stderr_without_a_stdout_result() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    let guidance = "Enter accepts the current selection or default; empty optional text skips.";
    assert!(visible_text(&session.stderr).contains(guidance));
    session.send("  配置 🦀  \r".as_bytes());
    session.custom_author();
    session.send(b"  Example team  \r");
    session.wait_for("Description (optional");
    session.send(b"Description\r");
    session.choose_format(directory.path(), LayerManifestFormat::Yaml);
    assert!(session.finish().success());
    assert_eq!(
        layer_manifest(directory.path()),
        json!({"name": "  配置 🦀  ", "author": "  Example team  ", "description": "Description"})
    );
    session.assert_guided_success(directory.path(), LayerManifestFormat::Yaml);
    let transcript = visible_text(&session.stderr);
    assert_eq!(transcript.matches(guidance).count(), 1);
    assert!(transcript.contains("Author (optional)"));
    assert!(transcript.contains("Description (optional)"));
    assert!(transcript.contains("Layer manifest format"));
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
    session.skip_author();
    session.wait_for("Description (optional");
    session.send(b"\r");
    session.choose_format(&directory, LayerManifestFormat::Yaml);
    assert!(session.finish().success());
    assert_eq!(layer_manifest(&directory), json!({"name": default_name}));
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
        session.custom_author();
        assert!(!visible_text(&session.stderr).contains("Skipped"));
        session.send(format!("{author}\r").as_bytes());
        session.wait_for("Description (optional");
        if author.is_empty() {
            assert!(visible_text(&session.stderr).contains("Skipped"));
        }
        session.send(format!("{description}\r").as_bytes());
        session.choose_format(directory.path(), LayerManifestFormat::Yaml);
        assert!(session.finish().success());
        assert_eq!(layer_manifest(directory.path()), expected);
        if description.is_empty() {
            assert!(visible_text(&session.stderr).contains("Skipped"));
        }
        if !author.is_empty() && !description.is_empty() {
            assert!(!visible_text(&session.stderr).contains("Skipped"));
        }
        let transcript = visible_text(&session.stderr);
        session.assert_guided_success(directory.path(), LayerManifestFormat::Yaml);
        if author.is_empty() || description.is_empty() {
            // A user can also type the literal metadata value "Skipped". Only
            // submitted empty answers get this treatment, not that ordinary text.
            let characters = styled_characters(&session.stderr);
            assert!(characters.windows("Skipped".len()).any(|window| {
                window.iter().map(|(c, _)| *c).eq("Skipped".chars())
                    && window.iter().all(|(_, style)| style.dim && style.italic)
            }));
        }
        let format_summary = selector_summary(&transcript);
        assert!(format_summary.contains("yaml"), "{format_summary}");
        assert!(!format_summary.contains("layer."), "{format_summary}");
        assert!(!format_summary.contains("Skipped"), "{format_summary}");
    }
}

#[test]
fn project_author_and_declared_organisation_are_default_without_changing_layer_name() {
    for author in ["Project author", "Fixture organisation"] {
        let directory = tempdir().unwrap();
        let package = json!({"name": "not-the-layer-name", "author": {"name": author}}).to_string();
        fs::write(directory.path().join("package.json"), &package).unwrap();
        let mut session = Session::with_settings(
            directory.path(),
            &["init"],
            None,
            None,
            TerminalSettings {
                identity: Some(github_fixture(json!([
                    {"name": "Fixture organisation", "login": "fixture-org"}
                ]))),
                ..Default::default()
            },
        );
        let layer_name = directory.path().file_name().unwrap().to_str().unwrap();
        session.wait_for("Name");
        session.wait_for(layer_name);
        session.send(b"\r");
        let frame = session.author_frame();
        assert!(
            frame.contains(&format!("● {author} (package.json author")),
            "{frame}"
        );
        assert!(frame.contains(DEFAULT_AUTHOR), "{frame}");
        session.send(b"\r");
        session.wait_for("Description (optional");
        let transcript = visible_text(&session.stderr);
        let summary = author_summary(&transcript);
        assert!(summary.contains(author), "{summary}");
        for hint in ["package.json", "GitHub", "GIT_AUTHOR_NAME", "fixture-org"] {
            assert!(!summary.contains(hint), "{summary}");
        }
        finish_after_author(&mut session, directory.path());
        assert_eq!(
            layer_manifest(directory.path()),
            json!({"name": layer_name, "author": author})
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("package.json")).unwrap(),
            package
        );
        assert_only_files(directory.path(), &["package.json", "layer.yaml"]);
    }
}

#[test]
fn project_sources_are_grouped_once_with_all_hints_before_identity_alternatives() {
    let directory = tempdir().unwrap();
    let files = [
        (
            "package.json",
            json!({"author": {"name": format!("  {DEFAULT_AUTHOR}  ")}}).to_string(),
        ),
        (
            "Cargo.toml",
            format!(
                "[package]\nauthors = [\"{DEFAULT_AUTHOR}\", \"Cargo alternative\", \"Cargo alternative\"]\n"
            ),
        ),
        (
            "pyproject.toml",
            format!(
                "[project]\nauthors = [{{name = \"{DEFAULT_AUTHOR}\"}}, {{name = \"Python alternative\"}}]\n"
            ),
        ),
    ];
    for (filename, contents) in &files {
        fs::write(directory.path().join(filename), contents).unwrap();
    }
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    session.send(b"example\r");
    let frame = session.author_frame();
    assert_eq!(frame.matches(DEFAULT_AUTHOR).count(), 1, "{frame}");
    let grouped = frame
        .lines()
        .find(|line| line.contains(DEFAULT_AUTHOR))
        .unwrap();
    for hint in [
        "package.json author",
        "Cargo.toml package.authors",
        "pyproject.toml project.authors",
        "GIT_AUTHOR_NAME",
    ] {
        assert_eq!(grouped.matches(hint).count(), 1, "{grouped}");
    }
    let first = frame.find(DEFAULT_AUTHOR).unwrap();
    let cargo = frame.find("Cargo alternative").unwrap();
    let python = frame.find("Python alternative").unwrap();
    assert!(first < cargo && cargo < python, "{frame}");
    assert_eq!(frame.matches("Cargo alternative").count(), 1, "{frame}");
    session.choose_author("Python alternative", false);
    finish_after_author(&mut session, directory.path());
    assert_eq!(
        layer_manifest(directory.path()),
        json!({"name": "example", "author": "Python alternative"})
    );
    for (filename, contents) in &files {
        assert_eq!(
            fs::read_to_string(directory.path().join(filename)).unwrap(),
            *contents
        );
    }
    assert_only_files(
        directory.path(),
        &["package.json", "Cargo.toml", "pyproject.toml", "layer.yaml"],
    );
}

#[test]
fn github_organisation_can_be_selected_without_storing_membership_or_leaking_credentials() {
    let directory = tempdir().unwrap();
    let mut session = Session::with_settings(
        directory.path(),
        &["init"],
        None,
        None,
        TerminalSettings {
            identity: Some(github_fixture(json!([
                {"name": "Fixture organisation", "login": "fixture-org"}
            ]))),
            ..Default::default()
        },
    );
    session.wait_for("Name");
    session.send(b"example\r");
    let frame = session.author_frame();
    let selected = frame
        .lines()
        .find_map(|line| line.split_once('●').map(|(_, item)| item.trim()))
        .expect("Author must have an initially selected candidate");
    let hints = selected
        .strip_prefix(&format!("{DEFAULT_AUTHOR} ("))
        .and_then(|hints| hints.strip_suffix(')'))
        .expect("The fixture author must be initially selected");
    assert!(
        hints.split(", ").any(|hint| hint == "GIT_AUTHOR_NAME"),
        "{frame}"
    );
    session.choose_author("Fixture organisation", false);
    session.wait_for("Description (optional");
    let text = visible_text(&session.stderr);
    let summary = author_summary(&text);
    assert!(summary.contains("Fixture organisation"), "{summary}");
    assert!(
        !summary.contains("fixture-org") && !summary.contains("GitHub"),
        "{summary}"
    );
    finish_after_author(&mut session, directory.path());
    assert_eq!(
        layer_manifest(directory.path()),
        json!({"name": "example", "author": "Fixture organisation"})
    );
    let root = session.identity.path();
    let environment = fs::read_to_string(root.join("gh-environment")).unwrap();
    for (key, value) in [
        ("HOME", root.join("home")),
        ("XDG_CONFIG_HOME", root.join("xdg")),
        ("GH_CONFIG_DIR", root.join("gh")),
        ("GIT_CONFIG_GLOBAL", root.join("gitconfig")),
        ("PATH", root.join("bin")),
    ] {
        assert!(
            environment
                .lines()
                .any(|line| line == format!("{key}={}", value.display())),
            "{environment}"
        );
    }
    for line in environment.lines() {
        let key = line.split_once('=').unwrap().0;
        assert!(
            !key.contains("TOKEN") && !key.contains("PASSWORD") && !key.contains("SECRET"),
            "Unexpected credential variable: {key}"
        );
        assert!(!matches!(
            key,
            "GIT_CONFIG_COUNT"
                | "GIT_CONFIG_SYSTEM"
                | "GIT_AUTHOR_EMAIL"
                | "GIT_COMMITTER_NAME"
                | "GIT_COMMITTER_EMAIL"
                | "USER"
                | "LOGNAME"
        ));
    }
    let calls = fs::read_to_string(root.join("gh-calls")).unwrap();
    assert_eq!(calls.lines().count(), 2, "{calls}");
    assert!(
        calls
            .lines()
            .all(|line| line.starts_with("api --hostname github.com ")),
        "{calls}"
    );
    assert!(
        !calls.contains("auth login") && !calls.contains("auth refresh"),
        "{calls}"
    );
    assert_only_files(directory.path(), &["layer.yaml"]);
}

#[test]
fn long_author_lists_scroll_natively_without_dropping_declared_names() {
    let directory = tempdir().unwrap();
    let names: Vec<_> = (0..36)
        .map(|index| format!("Project author {index:02}"))
        .collect();
    let cargo = format!(
        "[package]\nauthors = {}\n",
        serde_json::to_string(&names).unwrap()
    );
    fs::write(directory.path().join("Cargo.toml"), &cargo).unwrap();
    let mut session = Session::with_settings(
        directory.path(),
        &["init"],
        None,
        None,
        TerminalSettings {
            rows: 16,
            ..Default::default()
        },
    );
    session.wait_for("Name");
    session.send(b"example\r");
    let frame = session.author_frame();
    assert!(frame.contains("● Project author 00"), "{frame}");
    assert!(
        !frame.contains("Project author 35"),
        "Expected a scroll window, not every row: {frame}"
    );
    session.choose_author("Project author 35", false);
    finish_after_author(&mut session, directory.path());
    let transcript = visible_text(&session.stderr);
    for name in names {
        assert!(transcript.contains(&name), "Missing {name}: {transcript}");
    }
    assert_eq!(
        layer_manifest(directory.path()),
        json!({"name": "example", "author": "Project author 35"})
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("Cargo.toml")).unwrap(),
        cargo
    );
    assert_only_files(directory.path(), &["Cargo.toml", "layer.yaml"]);
}

#[test]
fn candidate_names_that_match_actions_remain_literal_author_metadata() {
    for name in ["Skip", "Enter another author"] {
        let directory = tempdir().unwrap();
        let package = json!({"author": {"name": name}}).to_string();
        fs::write(directory.path().join("package.json"), &package).unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        session.wait_for("Name");
        session.send(b"example\r");
        let frame = session.author_frame();
        assert!(
            frame.contains(&format!("● {name} (package.json author")),
            "{frame}"
        );
        session.choose_author(name, false);
        session.wait_for("Description (optional");
        let text = visible_text(&session.stderr);
        let summary = author_summary(&text);
        assert!(summary.contains(name), "{summary}");
        assert!(
            !summary.contains("Skipped") && !summary.contains("package.json"),
            "{summary}"
        );
        finish_after_author(&mut session, directory.path());
        assert_eq!(
            layer_manifest(directory.path()),
            json!({"name": "example", "author": name})
        );
        assert_only_files(directory.path(), &["package.json", "layer.yaml"]);
    }
}

#[test]
fn direct_skip_is_one_author_question_even_with_action_like_candidate_names() {
    let directory = tempdir().unwrap();
    fs::write(
        directory.path().join("Cargo.toml"),
        "[package]\nauthors = [\"Skip\", \"Enter another author\"]\n",
    )
    .unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    session.send(b"example\r");
    session.skip_author();
    let text = visible_text(&session.stderr);
    assert_eq!(
        text.lines()
            .filter(|line| line.contains('◇') && line.contains("Author (optional)"))
            .count(),
        1,
        "{text}"
    );
    let summary = author_summary(&text);
    assert!(summary.contains("Skipped"), "{summary}");
    assert!(
        !summary.contains("Enter another author") && !summary.contains("Cargo.toml"),
        "{summary}"
    );
    finish_after_author(&mut session, directory.path());
    assert_eq!(layer_manifest(directory.path()), json!({"name": "example"}));
    assert_only_files(directory.path(), &["Cargo.toml", "layer.yaml"]);
}

#[test]
fn author_choices_and_hints_remain_meaningful_without_colour_or_on_basic_terminals() {
    for settings in [
        TerminalSettings {
            no_color: Some("1"),
            ..Default::default()
        },
        TerminalSettings {
            term: "dumb",
            colour: Some(false),
            ..Default::default()
        },
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::with_settings(directory.path(), &["init"], None, None, settings);
        session.wait_for("Name");
        session.send(b"example\r");
        let frame = session.author_frame();
        for label in [
            DEFAULT_AUTHOR,
            "GIT_AUTHOR_NAME",
            "Enter another author",
            "Skip",
        ] {
            assert!(frame.contains(label), "{frame}");
        }
        session.send(b"\r");
        finish_after_author(&mut session, directory.path());
        assert_no_sgr(&session.stderr);
        assert_eq!(
            layer_manifest(directory.path()),
            json!({"name": "example", "author": DEFAULT_AUTHOR})
        );
        assert_only_files(directory.path(), &["layer.yaml"]);
    }
}

#[test]
fn missing_git_and_gh_still_allow_empty_custom_author_without_restoring_suggestions() {
    let directory = tempdir().unwrap();
    let fixture = IdentityFixture::new();
    fixture.without_tools();
    let mut session = Session::with_settings(
        directory.path(),
        &["init"],
        None,
        None,
        TerminalSettings {
            identity: Some(fixture),
            ..Default::default()
        },
    );
    session.wait_for("Name");
    session.send(b"example\r");
    session.custom_author();
    session.send(b"\r");
    session.wait_for("Description (optional");
    let text = visible_text(&session.stderr);
    assert!(author_summary(&text).contains("Skipped"), "{text}");
    assert!(
        !text.contains("Warning:") && !text.contains("Error:"),
        "{text}"
    );
    finish_after_author(&mut session, directory.path());
    assert_eq!(layer_manifest(directory.path()), json!({"name": "example"}));
    assert_only_files(directory.path(), &["layer.yaml"]);
}

#[test]
fn cancelling_custom_author_input_reports_once_and_creates_nothing() {
    for key in [b"\x1b".as_slice(), b"\x03"] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        session.wait_for("Name");
        session.send(b"example\r");
        session.custom_author();
        session.send(b"unsubmitted author");
        session.send(key);
        assert!(!session.finish().success());
        let text = visible_text(&session.stderr);
        assert_eq!(text.matches("cancelled").count(), 1, "{text}");
        assert_eq!(
            text.to_lowercase().matches("no files were changed").count(),
            1,
            "{text}"
        );
        assert!(
            !text.contains("Error:")
                && !text.contains("Description (optional")
                && !text.contains("Layer initialised"),
            "{text}"
        );
        assert!(session.stdout.is_empty());
        assert_no_layer_manifests(directory.path());
        assert_only_files(directory.path(), &[]);
    }
}

#[test]
fn invalid_answers_can_be_corrected_without_creating_files_early() {
    for colour in [false, true] {
        for supply_author in [false, true] {
            let directory = tempdir().unwrap();
            let mut session = Session::with_settings(
                directory.path(),
                &["init"],
                None,
                None,
                TerminalSettings {
                    colour: Some(colour),
                    ..Default::default()
                },
            );
            session.wait_for("Name");
            session.send(b" \r");
            session.wait_for("Enter a non-blank name.");
            assert_style(&session.stderr, "✕", |style| {
                !colour || style.foreground == Some(31)
            });
            assert!(!visible_text(&session.stderr).contains('▲'));
            if colour {
                assert_style(&session.stderr, "Enter a non-blank name.", |style| {
                    style.foreground == Some(31)
                });
                let characters = styled_characters(&session.stderr);
                let error = characters.iter().rposition(|(c, _)| *c == '✕').unwrap();
                assert!(characters[error..].iter().any(|(c, style)| {
                    matches!(c, '│' | '└') && style.foreground == Some(31)
                }));
            }
            assert_no_layer_manifests(directory.path());
            session.send("\x7f  配置 🦀  \r".as_bytes());
            session.custom_author();
            session.send(b" \r");
            session
                .wait_for("Enter a non-blank author, or clear the field and press Enter to skip.");
            assert_no_layer_manifests(directory.path());
            if supply_author {
                session.send("\x7f  Team 配置  \r".as_bytes());
            } else {
                session.send(b"\x7f\r");
            }
            session.wait_for("Description (optional");
            if !supply_author {
                assert!(visible_text(&session.stderr).contains("Skipped"));
            }
            session.send(b" \r");
            session.wait_for(
                "Enter a non-blank description, or clear the field and press Enter to skip.",
            );
            assert_no_layer_manifests(directory.path());
            if supply_author {
                session.send(b"\x7f\r");
            } else {
                session.send("\x7f  Description 配置  \r".as_bytes());
            }
            session.choose_format(directory.path(), LayerManifestFormat::Yaml);
            assert!(session.finish().success());
            let expected = if supply_author {
                json!({"name": "  配置 🦀  ", "author": "  Team 配置  "})
            } else {
                json!({"name": "  配置 🦀  ", "description": "  Description 配置  "})
            };
            assert_eq!(layer_manifest(directory.path()), expected);
            assert_only_files(directory.path(), &["layer.yaml"]);
            let transcript = visible_text(&session.stderr);
            assert!(!transcript.contains("--name"));
            assert!(!transcript.contains("--author"));
            assert!(!transcript.contains("--description"));
            assert!(transcript.contains("Skipped"));
            session.assert_guided_success(directory.path(), LayerManifestFormat::Yaml);
            let raw = String::from_utf8_lossy(&session.stderr);
            let has_style = raw.split("\x1b[").skip(1).any(|part| {
                part.find('m')
                    .is_some_and(|end| part[..end].chars().all(|c| c.is_ascii_digit() || c == ';'))
            });
            assert_eq!(has_style, colour);
        }
    }
}

#[test]
fn unicode_schema_answers_reject_next_line_whitespace_and_accept_bom_without_normalizing() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    session.send("\u{85}\r".as_bytes());
    session.wait_for("Enter a non-blank name.");
    assert_no_layer_manifests(directory.path());
    session.send("\x7f\u{feff}\r".as_bytes());
    session.custom_author();
    session.send("\u{85}\r".as_bytes());
    session.wait_for("Enter a non-blank author, or clear the field and press Enter to skip.");
    assert_no_layer_manifests(directory.path());
    session.send(b"\x7f\r");
    session.wait_for("Description (optional");
    session.send("\u{85}visible 配置 🦀\u{2003}\r".as_bytes());
    session.choose_format(directory.path(), LayerManifestFormat::Jsonc);
    assert!(session.finish().success());
    assert_eq!(
        layer_manifest_in_format(directory.path(), LayerManifestFormat::Jsonc),
        json!({"name": "\u{feff}", "description": "\u{85}visible 配置 🦀\u{2003}"})
    );
    assert_only_files(directory.path(), &["layer.jsonc"]);
    session.assert_guided_success(directory.path(), LayerManifestFormat::Jsonc);
}

#[test]
fn every_guided_format_choice_uses_a_fixed_filename_and_independent_content() {
    for format in LayerManifestFormat::ALL {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        session.wait_for("Name");
        assert_no_layer_manifests(directory.path());
        session.send(b"shared-agent-rules\r");
        session.custom_author();
        assert_no_layer_manifests(directory.path());
        session.send("  Équipe 🦀  \r".as_bytes());
        session.wait_for("Description (optional");
        assert_no_layer_manifests(directory.path());
        session.send(b"2026-10-16\r");
        session.wait_for("jsonc");
        let transcript = visible_text(&session.stderr);
        let choices = transcript.split_once("Layer manifest format").unwrap().1;
        assert!(!choices.contains("layer."), "{choices}");
        let raw = String::from_utf8_lossy(&session.stderr);
        let menu = raw
            .rsplit_once("Layer manifest format")
            .unwrap()
            .1
            .as_bytes();
        assert_style(menu, "yaml", |style| style.foreground == Some(36));
        for other in ["yml", "toml", "json", "jsonc"] {
            assert_style(menu, other, |style| style.foreground != Some(36));
        }
        let mut previous = 0;
        for choice in LayerManifestFormat::ALL {
            assert!(
                choices
                    .lines()
                    .any(|line| line.split_whitespace().last() == Some(choice.extension())),
                "{choices}"
            );
            let position = choices.find(choice.extension()).unwrap();
            assert!(position >= previous, "{choices}");
            previous = position;
        }
        session.choose_format(directory.path(), format);
        assert!(session.finish().success());
        assert_eq!(
            layer_manifest_in_format(directory.path(), format),
            json!({
                "name": "shared-agent-rules", "author": "  Équipe 🦀  ", "description": "2026-10-16"
            })
        );
        assert_only_files(directory.path(), &[format.filename()]);
        session.assert_guided_success(directory.path(), format);
        let transcript = visible_text(&session.stderr);
        assert!(!transcript.contains("Skipped"));
        let summary = selector_summary(&transcript);
        assert!(summary.contains(format.extension()), "{summary}");
        assert!(!summary.contains("layer."), "{summary}");
    }
}

#[test]
fn format_only_flags_never_prompt_in_a_terminal() {
    for format in LayerManifestFormat::ALL {
        let directory = tempdir().unwrap();
        let mut session = Session::new(
            directory.path(),
            &["init", "--format", format.extension()],
            None,
        );
        assert!(session.finish().success());
        session.assert_compact_success(format, true);
        assert_eq!(
            layer_manifest_in_format(directory.path(), format),
            json!({
                "name": directory.path().file_name().unwrap().to_str().unwrap()
            })
        );
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
        session.assert_compact_success(LayerManifestFormat::Yaml, true);
        let expected_name = if arguments.contains(&"--name") {
            "example"
        } else {
            directory.path().file_name().unwrap().to_str().unwrap()
        };
        assert_eq!(layer_manifest(directory.path())["name"], expected_name);
        assert_eq!(
            layer_manifest(directory.path()).get("author").is_some(),
            arguments.contains(&"--author")
        );
        assert_eq!(
            layer_manifest(directory.path())
                .get("description")
                .is_some(),
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
        if arguments.contains(&"--format") {
            session.assert_plain();
        } else {
            assert_style(&session.stderr, "✕ Error:", |style| {
                style.foreground == Some(31)
            });
        }
        assert_no_layer_manifests(directory.path());
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
                LayerManifestFormat::ALL
                    .into_iter()
                    .find(|format| format.extension() == arguments[index + 1])
                    .unwrap()
            })
            .unwrap_or_default();
        assert_eq!(
            layer_manifest_in_format(directory.path(), format),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
        assert_only_files(directory.path(), &[format.filename()]);
        session.assert_compact_success(format, ci.is_none());
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
                session.author_frame();
            }
            if answered >= 2 {
                session.custom_author();
                session.send(b"Team\r");
                session.wait_for("Description (optional");
            }
            if answered == 3 {
                session.send(b"Description\r");
                session.wait_for("Layer manifest format");
                session.wait_for("jsonc");
            }
            assert_no_layer_manifests(directory.path());
            session.send(key);
            assert!(!session.finish().success());
            let text = visible_text(&session.stderr);
            assert_eq!(text.matches("cancelled").count(), 1, "{text}");
            assert_eq!(
                text.to_lowercase().matches("no files were changed").count(),
                1,
                "{text}"
            );
            assert!(!text.contains("Error:"), "{text}");
            assert!(!text.contains("Layer initialised"), "{text}");
            assert_style(&session.stderr, "■", |style| style.foreground == Some(31));
            assert!(session.stdout.is_empty());
            assert_no_layer_manifests(directory.path());
            assert_only_files(directory.path(), &[]);
        }
    }
}

#[test]
fn every_occupied_layer_manifest_path_is_reported_before_any_questions() {
    use std::os::unix::fs::symlink;

    for (format, kind) in LayerManifestFormat::ALL
        .into_iter()
        .flat_map(|format| ["file", "directory", "symlink", "dangling"].map(|kind| (format, kind)))
    {
        let directory = tempdir().unwrap();
        let path = directory.path().join(format.filename());
        let target = directory.path().join("target");
        if kind == "file" {
            fs::write(&path, "not valid layer manifest content").unwrap();
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
        assert_occupied_feedback(&session.stderr);
        assert!(String::from_utf8_lossy(&session.stderr).contains(format.filename()));
        assert!(visible_text(&session.stderr).contains(&format!(
            "Error: This directory already has a layer manifest ({}).",
            format.filename()
        )));
        assert!(!visible_text(&session.stderr).contains(&directory.path().display().to_string()));
        assert!(!visible_text(&session.stderr).contains("Warning:"));
        if kind == "file" {
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                "not valid layer manifest content"
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
    for format in LayerManifestFormat::ALL {
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
    for format in LayerManifestFormat::ALL {
        assert!(stderr.contains(format.filename()), "{stderr}");
        assert_eq!(
            fs::read_to_string(directory.path().join(format.filename())).unwrap(),
            "not valid content"
        );
    }
    assert_only_files(
        directory.path(),
        &LayerManifestFormat::ALL.map(|format| format.filename()),
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
    assert_no_layer_manifests(&nested);
}

#[test]
fn the_exact_default_target_created_during_questions_is_not_overwritten() {
    let directory = tempdir().unwrap();
    let mut session = Session::new(directory.path(), &["init"], None);
    session.wait_for("Name");
    fs::write(directory.path().join("layer.yaml"), "another process").unwrap();
    session.send(b"example\r");
    session.skip_author();
    session.wait_for("Description (optional");
    session.send(b"\r");
    session.wait_for("Layer manifest format");
    session.wait_for("jsonc");
    session.send(b"\r");
    assert!(!session.finish().success());
    assert!(session.stdout.is_empty());
    assert_occupied_feedback(&session.stderr);
    assert_eq!(
        fs::read_to_string(directory.path().join("layer.yaml")).unwrap(),
        "another process"
    );
    assert!(visible_text(&session.stderr).contains("already has a layer manifest"));
    assert_only_files(directory.path(), &["layer.yaml"]);
}

#[test]
fn alternate_formats_appearing_during_questions_allow_creation_in_both_priority_directions() {
    for (appeared, chosen, governing) in [
        (
            LayerManifestFormat::Json,
            LayerManifestFormat::Yaml,
            LayerManifestFormat::Yaml,
        ),
        (
            LayerManifestFormat::Yaml,
            LayerManifestFormat::Jsonc,
            LayerManifestFormat::Yaml,
        ),
    ] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        session.wait_for("Name");
        assert_no_layer_manifests(directory.path());
        fs::write(
            directory.path().join(appeared.filename()),
            "another process",
        )
        .unwrap();
        session.send(b"example\r");
        session.skip_author();
        session.wait_for("Description (optional");
        session.send(b"\r");
        session.wait_for("Layer manifest format");
        session.wait_for("jsonc");
        // The output is chosen only after the competing path has appeared.
        for _ in 0..LayerManifestFormat::ALL
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
            layer_manifest_in_format(directory.path(), chosen),
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
        session.assert_guided_success(directory.path(), chosen);
        assert_only_files(directory.path(), &[appeared.filename(), chosen.filename()]);
    }
}

#[test]
fn redirecting_any_single_stream_disables_guided_input() {
    for (stream, format) in ["stdin", "stdout", "stderr"]
        .into_iter()
        .flat_map(|stream| {
            [LayerManifestFormat::Yaml, LayerManifestFormat::Jsonc].map(|format| (stream, format))
        })
    {
        let directory = tempdir().unwrap();
        let capture_directory = tempdir().unwrap();
        let capture = capture_directory.path().join("output");
        let arguments = if format == LayerManifestFormat::Yaml {
            vec!["init"]
        } else {
            vec!["init", "--format", format.extension()]
        };
        let mut session =
            Session::spawn(directory.path(), &arguments, None, Some((stream, &capture)));
        assert!(session.finish().success());
        session.assert_no_prompts();
        session.assert_plain();
        assert!(session.stderr.is_empty());
        let result = if stream == "stdout" {
            fs::read(&capture).unwrap()
        } else {
            session.stdout.clone()
        };
        assert_eq!(
            String::from_utf8_lossy(&result).replace("\r\n", "\n"),
            format!(
                "Layer initialised, layer manifest created at: {}\n",
                format.filename()
            )
        );
        if stream == "stderr" {
            assert!(fs::read(&capture).unwrap().is_empty());
        }
        assert_eq!(
            layer_manifest_in_format(directory.path(), format),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
        assert_only_files(directory.path(), &[format.filename()]);
    }
}

#[test]
fn absolute_stty_preamble_preserves_queued_keys_with_fixture_only_path() {
    for cancel in [false, true] {
        let directory = tempdir().unwrap();
        let mut session = Session::new(directory.path(), &["init"], None);
        assert!(!session.identity.path().join("bin/stty").exists());
        assert!(Path::new("/bin/stty").is_file());
        session.wait_for("Name");
        // Queue editing keys together, rather than timing each byte to a read.
        session.send(b"temp\x7f\x7f\x7f\x7fexample\r");
        session.custom_author();
        if cancel {
            session.send(b"queued\x7f\x03");
            assert!(!session.finish().success());
            let text = visible_text(&session.stderr);
            assert_eq!(text.matches("cancelled").count(), 1, "{text}");
            assert_eq!(
                text.to_lowercase().matches("no files were changed").count(),
                1,
                "{text}"
            );
            assert!(!text.contains("Error:") && !text.contains("Description (optional"));
            assert!(session.stdout.is_empty());
            assert_no_layer_manifests(directory.path());
            assert_only_files(directory.path(), &[]);
        } else {
            session.send(b"temp\x7f\x7f\x7f\x7fChosen author\r");
            finish_after_author(&mut session, directory.path());
            assert_eq!(
                layer_manifest(directory.path()),
                json!({"name": "example", "author": "Chosen author"})
            );
            assert_only_files(directory.path(), &["layer.yaml"]);
        }
    }
}

#[test]
fn losing_terminal_input_reports_failure_without_creating_a_layer_manifest() {
    for stop_at in ["name", "author-selector", "author-text", "format"] {
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
        assert_eq!(
            input_mode(input.master.as_ref()),
            Some(InputMode::Canonical),
            "Dedicated input PTY must have a canonical baseline before child spawn"
        );
        let input_path = input.master.tty_name().unwrap();
        // portable-pty's writer sends a newline and EOT on drop. A plain
        // duplicate closes silently, so dropping it tests disconnection instead
        // of accidentally submitting the current prompt.
        // SAFETY: input.master owns this descriptor and outlives the borrow;
        // try_clone_to_owned creates an independently owned descriptor.
        let input_fd = unsafe { BorrowedFd::borrow_raw(input.master.as_raw_fd().unwrap()) };
        let mut input_writer: Box<dyn Write + Send> =
            Box::new(fs::File::from(input_fd.try_clone_to_owned().unwrap()));
        let mut session = Session::spawn_with_lock(
            directory.path(),
            &["init"],
            None,
            Some(("stdin-eof", &input_path)),
            terminal_session,
            TerminalSettings::default(),
        );
        session.wait_for("Name");
        if stop_at != "name" {
            input_writer.write_all(b"example\r").unwrap();
            input_writer.flush().unwrap();
            session.author_frame();
        }
        if matches!(stop_at, "author-text" | "format") {
            let original_writer = session.writer.replace(input_writer);
            session.custom_author();
            input_writer = session.writer.take().unwrap();
            session.writer = original_writer;
        }
        if stop_at == "format" {
            for (answer, next_prompt) in [
                ("Team\r", "Description (optional"),
                ("Description\r", "Layer manifest format"),
            ] {
                input_writer.write_all(answer.as_bytes()).unwrap();
                input_writer.flush().unwrap();
                session.wait_for(next_prompt);
            }
            session.wait_for("jsonc");
        }
        assert_no_layer_manifests(directory.path());
        // Frames are flushed before console selects its input. With no answer
        // bytes left queued, this PTY's canonical-to-noncanonical transition
        // proves stdin has been selected, not that a read syscall is blocked.
        session.wait_for_noncanonical_input(input.master.as_ref());
        // Closing every master handle disconnects stdin while stderr remains usable.
        // This is actual terminal input loss, not Ctrl-D (a raw-mode keystroke).
        drop(input_writer);
        drop(input);
        let status = session.finish();
        assert!(
            !status.success(),
            "Input loss at {stop_at} returned {status:?}; stderr: {}; stdout: {}",
            String::from_utf8_lossy(&session.stderr),
            String::from_utf8_lossy(&session.stdout)
        );
        assert!(
            String::from_utf8_lossy(&session.stderr)
                .contains("Could not collect initialisation input")
        );
        let text = visible_text(&session.stderr);
        assert_eq!(text.matches("Error:").count(), 1, "{text}");
        assert!(!text.contains("cancelled"), "{text}");
        assert!(!text.contains("Layer initialised"), "{text}");
        assert_style(&session.stderr, "✕ Error:", |style| {
            style.foreground == Some(31)
        });
        assert!(session.stdout.is_empty());
        assert_no_layer_manifests(directory.path());
        assert_only_files(directory.path(), &[]);
    }
}
