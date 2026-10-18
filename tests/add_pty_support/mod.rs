//! Small one-shot terminal harness: no input or init prompt machinery.

use std::io::Read;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, ExitStatus, MasterPty, PtySize, native_pty_system};

const TIMEOUT: Duration = Duration::from_secs(10);
static TERMINAL: Mutex<()> = Mutex::new(());

pub struct Settings<'a> {
    pub width: u16,
    pub environment: &'a [(&'a str, &'a str)],
    pub redirect: Option<(&'a str, &'a Path)>,
}

impl Default for Settings<'_> {
    fn default() -> Self {
        Self {
            width: 80,
            environment: &[],
            redirect: None,
        }
    }
}

enum Event {
    Bytes(Vec<u8>),
    Closed,
}

pub struct Output {
    pub status: ExitStatus,
    pub raw: String,
    redirected: Option<String>,
}

impl Output {
    pub fn plain(&self) -> String {
        console::strip_ansi_codes(&self.raw).replace('\r', "")
    }

    pub fn evidence(&self, label: &str) {
        if let Some(directory) = std::env::var_os("HYDRAGEN_USAGE_EVIDENCE")
            .or_else(|| std::env::var_os("HYDRAGEN_ADD_EVIDENCE"))
        {
            let directory = Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            std::fs::write(directory.join(format!("{label}.ansi.txt")), &self.raw).unwrap();
            std::fs::write(directory.join(format!("{label}.plain.txt")), self.plain()).unwrap();
            if let Some(redirected) = &self.redirected {
                std::fs::write(
                    directory.join(format!("{label}.redirected.txt")),
                    redirected,
                )
                .unwrap();
            }
        }
    }
}

struct Session {
    _terminal: MutexGuard<'static, ()>,
    child: Box<dyn Child + Send + Sync>,
    master: Option<Box<dyn MasterPty + Send>>,
    reader: Option<JoinHandle<()>>,
    events: Receiver<Event>,
    bytes: Vec<u8>,
}

impl Drop for Session {
    fn drop(&mut self) {
        // Every assertion/timeout path reaps the actual exec child. Never perform
        // an unbounded reader join when unwinding from a failed terminal test.
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.master.take();
        if let Some(reader) = self.reader.take().filter(|reader| reader.is_finished()) {
            let _ = reader.join();
        }
    }
}

pub fn run(directory: &Path, arguments: &[&str], settings: Settings<'_>) -> Output {
    let terminal = TERMINAL.lock().unwrap_or_else(|poison| poison.into_inner());
    let pty = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: settings.width,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut command = CommandBuilder::new("/bin/sh");
    command.args([
        "-c",
        "case \"$REDIRECT\" in
stdin) exec \"$@\" </dev/null;;
stdout) exec \"$@\" >\"$CAPTURE\";;
stderr) exec \"$@\" 2>\"$CAPTURE\";;
stdin,stdout) exec \"$@\" </dev/null >\"$CAPTURE\";;
stdin,stderr) exec \"$@\" </dev/null 2>\"$CAPTURE\";;
stdout,stderr) exec \"$@\" >\"$CAPTURE\" 2>&1;;
stdin,stdout,stderr) exec \"$@\" </dev/null >\"$CAPTURE\" 2>&1;;
*) exec \"$@\";;
esac",
        "add-terminal",
        env!("CARGO_BIN_EXE_hydragen"),
    ]);
    command.args(arguments);
    command.cwd(directory);
    for key in [
        "CI",
        "NO_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "REDIRECT",
        "CAPTURE",
        "COLUMNS",
    ] {
        command.env_remove(key);
    }
    command.env("TERM", "xterm-256color");
    command.env("CLICOLOR_FORCE", "1");
    for (key, value) in settings.environment {
        command.env(key, value);
    }
    if let Some((stream, capture)) = settings.redirect {
        command.env("REDIRECT", stream);
        command.env("CAPTURE", capture);
    }
    let mut input = pty.master.try_clone_reader().unwrap();
    let (sender, events) = mpsc::channel();
    let child = pty.slave.spawn_command(command).unwrap();
    drop(pty.slave);
    let reader = std::thread::spawn(move || {
        let mut buffer = [0; 4096];
        while let Ok(count) = input.read(&mut buffer) {
            if count == 0 {
                break;
            }
            if sender.send(Event::Bytes(buffer[..count].to_vec())).is_err() {
                return;
            }
        }
        let _ = sender.send(Event::Closed);
    });
    let mut session = Session {
        _terminal: terminal,
        child,
        master: Some(pty.master),
        reader: Some(reader),
        events,
        bytes: Vec::new(),
    };
    let deadline = Instant::now() + TIMEOUT;
    let mut status = None;
    let mut closed = false;
    loop {
        if status.is_none() {
            status = session.child.try_wait().unwrap();
        }
        if status.is_some() && closed {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "add terminal timed out: {}",
            String::from_utf8_lossy(&session.bytes)
        );
        match session.events.recv_timeout(Duration::from_millis(10)) {
            Ok(Event::Bytes(bytes)) => session.bytes.extend(bytes),
            Ok(Event::Closed) => closed = true,
            Err(mpsc::RecvTimeoutError::Disconnected) => closed = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    Output {
        status: status.unwrap(),
        raw: String::from_utf8(session.bytes.clone()).unwrap(),
        redirected: settings
            .redirect
            .and_then(|(_, capture)| std::fs::read_to_string(capture).ok()),
    }
}
