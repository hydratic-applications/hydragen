//! Shell-private FIFO fixtures. The multithreaded runner never owns a FIFO reader.
#![cfg(unix)]
#![allow(dead_code)]

use std::cell::RefCell;
use std::fs;
use std::io;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub enum Stream {
    Stdout,
    Stderr,
}
#[derive(Clone, Copy)]
pub enum Delivery {
    BrokenPipe,
    LiveReader,
    ClosedDescriptor,
}

pub struct Capture {
    pub output: Output,
    pub consumed: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    GroupSignal(i32),
    SignalsFinished,
    Reaped,
}
pub type LifecycleTrace = Rc<RefCell<Vec<LifecycleEvent>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnershipPhase {
    Anchored,
    SignalsFinished,
    Reaped,
}

pub struct GroupOwnership {
    phase: OwnershipPhase,
    trace: LifecycleTrace,
}

impl Default for GroupOwnership {
    fn default() -> Self {
        Self::new(LifecycleTrace::default())
    }
}

impl GroupOwnership {
    pub fn new(trace: LifecycleTrace) -> Self {
        Self {
            phase: OwnershipPhase::Anchored,
            trace,
        }
    }
    pub fn record_signal(&mut self, signal: i32) -> io::Result<()> {
        if self.phase != OwnershipPhase::Anchored {
            return Err(io::Error::other("group signaling is permanently closed"));
        }
        self.trace
            .borrow_mut()
            .push(LifecycleEvent::GroupSignal(signal));
        Ok(())
    }
    pub fn finish_signals(&mut self) {
        assert_eq!(
            self.phase,
            OwnershipPhase::Anchored,
            "signaling cannot reopen"
        );
        self.phase = OwnershipPhase::SignalsFinished;
        self.trace
            .borrow_mut()
            .push(LifecycleEvent::SignalsFinished);
    }
    pub fn record_reaped(&mut self) -> io::Result<()> {
        if self.phase != OwnershipPhase::SignalsFinished {
            return Err(io::Error::other(
                "cannot reap while group signaling remains available",
            ));
        }
        self.phase = OwnershipPhase::Reaped;
        self.trace.borrow_mut().push(LifecycleEvent::Reaped);
        Ok(())
    }
}

// POSIX test-process group cleanup only; no production descriptor checks or new
// runtime dependency. SIGTERM/SIGKILL are the Unix signals used by these fixtures.
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

pub fn process_exists(pid: u32) -> bool {
    unsafe { kill(pid as i32, 0) == 0 }
}

pub fn command_status(code: i32) -> io::Result<std::process::ExitStatus> {
    // A shell wait status at/above 128 may encode signal termination. Never
    // normalize it into an ordinary exit 1 or successful completion.
    if !(0..128).contains(&code) {
        return Err(io::Error::other(format!(
            "fixture command returned abnormal shell wait status {code}"
        )));
    }
    Ok(std::process::ExitStatus::from_raw(code << 8))
}

struct Running {
    child: Child,
    deadline: Instant,
    ownership: GroupOwnership,
    completion: std::path::PathBuf,
    reader_pid: std::path::PathBuf,
    status: Option<std::process::ExitStatus>,
}

impl Running {
    fn signal_group(&mut self, signal: i32) -> io::Result<()> {
        self.ownership.record_signal(signal)?;
        // No wait has occurred: even a dead leader remains our unreaped anchor,
        // preventing numeric ID reuse until ALL group-directed signals finish.
        unsafe {
            kill(-(self.child.id() as i32), signal);
        }
        Ok(())
    }

    fn final_signal(&mut self) -> io::Result<()> {
        if self.ownership.phase == OwnershipPhase::Anchored {
            self.signal_group(9)?;
            // Irreversible BEFORE the first try_wait, including all error/Drop paths.
            self.ownership.finish_signals();
        }
        Ok(())
    }

    fn completion_code(&self) -> io::Result<Option<i32>> {
        match fs::read_to_string(&self.completion) {
            Ok(text) if !text.ends_with('\n') => Ok(None),
            Ok(text) => text
                .trim_end_matches('\n')
                .parse::<i32>()
                .ok()
                .filter(|code| (0..=255).contains(code))
                .map(Some)
                .ok_or_else(|| {
                    io::Error::other("invalid private output-fixture completion record")
                }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn reap(&mut self) -> io::Result<std::process::ExitStatus> {
        if self.ownership.phase == OwnershipPhase::Anchored {
            return Err(io::Error::other(
                "attempted to wait before the final group signal",
            ));
        }
        if let Some(status) = self.status {
            return Ok(status);
        }
        loop {
            if let Some(status) = self.child.try_wait()? {
                if self.ownership.phase != OwnershipPhase::Reaped {
                    self.ownership.record_reaped()?;
                }
                self.status = Some(status);
                return Ok(status);
            }
            if Instant::now() >= self.deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "could not reap output-fixture group leader before cleanup deadline",
                ));
            }
            std::thread::yield_now();
        }
    }

    fn cleanup(&mut self) -> io::Result<()> {
        if self.ownership.phase == OwnershipPhase::Reaped {
            return Ok(());
        }
        if self.ownership.phase == OwnershipPhase::Anchored {
            self.signal_group(15)?;
            let soft = Instant::now() + self.deadline.saturating_duration_since(Instant::now()) / 2;
            // Only the shell's ready record is observed here. No early wait.
            while Instant::now() < soft {
                if matches!(self.completion_code(), Ok(Some(_))) {
                    break;
                }
                std::thread::yield_now();
            }
            self.final_signal()?;
        }
        // A killed supervisor cannot run its trap. Its remaining members were
        // killed while its unreaped PID still anchored ownership. Individual
        // reader observations are non-destructive, not group-ownership checks.
        let mut reader_pending = false;
        if matches!(self.completion_code(), Ok(None))
            && let Ok(text) = fs::read_to_string(&self.reader_pid)
            && let Ok(pid) = text.trim().parse::<u32>()
        {
            while process_exists(pid) && Instant::now() < self.deadline {
                std::thread::yield_now();
            }
            reader_pending = process_exists(pid);
        }
        self.reap()?;
        if reader_pending {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "private FIFO reader was not reaped within the fixture deadline",
            ));
        }
        Ok(())
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!("Output fixture cleanup failed: {error}");
        }
    }
}

fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\"'\"'"))
}

pub fn run(
    directory: &Path,
    arguments: &[&str],
    stream: Stream,
    delivery: Delivery,
) -> io::Result<Capture> {
    run_environment(directory, arguments, stream, delivery, &[])
}

pub fn run_environment(
    directory: &Path,
    arguments: &[&str],
    stream: Stream,
    delivery: Delivery,
    environment: &[(&str, &str)],
) -> io::Result<Capture> {
    run_inner(
        directory,
        arguments,
        stream,
        delivery,
        Duration::from_secs(10),
        None,
        (environment, LifecycleTrace::default()),
    )
}

/// An optional trusted setup tail lets the harness tests exercise timeout/error
/// cleanup after reader creation. It is never sent to Hydragen as a fault seam.
pub fn run_with_timeout(
    directory: &Path,
    arguments: &[&str],
    stream: Stream,
    delivery: Delivery,
    timeout: Duration,
    setup_tail: Option<&str>,
) -> io::Result<Capture> {
    run_inner(
        directory,
        arguments,
        stream,
        delivery,
        timeout,
        setup_tail,
        (&[], LifecycleTrace::default()),
    )
}

pub fn run_with_trace(
    directory: &Path,
    arguments: &[&str],
    stream: Stream,
    delivery: Delivery,
    timeout: Duration,
    setup_tail: Option<&str>,
    trace: LifecycleTrace,
) -> io::Result<Capture> {
    run_inner(
        directory,
        arguments,
        stream,
        delivery,
        timeout,
        setup_tail,
        (&[], trace),
    )
}

fn run_inner(
    directory: &Path,
    arguments: &[&str],
    stream: Stream,
    delivery: Delivery,
    timeout: Duration,
    setup_tail: Option<&str>,
    context: (&[(&str, &str)], LifecycleTrace),
) -> io::Result<Capture> {
    let (environment, trace) = context;
    let deadline = Instant::now() + timeout;
    let scratch = tempfile::tempdir()?;
    let fifo = scratch.path().join("stdout-fifo");
    let consumed = scratch.path().join("consumed");
    let completion = scratch.path().join("completion");
    let reader_pid = scratch.path().join("reader-pid");
    let stdout = tempfile::NamedTempFile::new()?;
    let stderr = tempfile::NamedTempFile::new()?;
    let fd = match stream {
        Stream::Stdout => 1,
        Stream::Stderr => 2,
    };
    let delivery_setup = if matches!(delivery, Delivery::ClosedDescriptor) {
        String::new()
    } else {
        let setup = format!(
            r#"
mkfifo {fifo} || exit 91
# Keep cleanup's TERM from racing the supervisor's own terminate-and-reap step.
(trap '' HUP INT TERM; exec cat {fifo} >{consumed}) &
reader=$!
printf '%s\n' "$reader" >{reader_pid}
{setup_tail}
exec 3>{fifo}
"#,
            fifo = quote(&fifo),
            consumed = quote(&consumed),
            reader_pid = quote(&reader_pid),
            setup_tail = setup_tail.unwrap_or("")
        );
        let stop_reader = if matches!(delivery, Delivery::BrokenPipe) {
            "waiting_reader=$reader\nreader=\nkill -KILL \"$waiting_reader\" || exit 92\nwait \"$waiting_reader\" 2>/dev/null\nwaiting_reader=\n"
        } else {
            ""
        };
        format!(
            "{setup}{stop_reader}rm {fifo} || exit 94",
            fifo = quote(&fifo)
        )
    };
    let redirect = if matches!(delivery, Delivery::ClosedDescriptor) {
        format!("exec {fd}>&-")
    } else {
        format!("exec {fd}>&3\nexec 3>&-")
    };
    // The direct child is an ownership anchor, not the measured CLI. It publishes
    // the CLI's shell-wait status only AFTER all children/readers are reaped, then
    // blocks on a private control stream. Rust sends its FINAL group signal while
    // this anchor is still unreaped; only afterward may Rust collect its status.
    let script = format!(
        r#"
reader=
app=
waiting_app=
waiting_reader=
cleanup() {{
  if [ -n "$app" ]; then waiting_app=$app; app=; fi
  if [ -n "$waiting_app" ]; then wait "$waiting_app" 2>/dev/null; waiting_app=; fi
  if [ -n "$reader" ]; then kill -KILL "$reader" 2>/dev/null; waiting_reader=$reader; reader=; fi
  if [ -n "$waiting_reader" ]; then wait "$waiting_reader" 2>/dev/null; waiting_reader=; fi
  exec 3>&-
  rm -f {fifo}
}}
finish() {{
  result=$?
  trap - EXIT HUP INT TERM
  cleanup
  printf '%s\n' "$result" >{completion}
  IFS= read -r release
  exit "$result"
}}
trap finish EXIT
trap 'exit 125' HUP INT TERM
{delivery_setup}
trap '' PIPE
({redirect}
exec "$@" </dev/null) &
app=$!
waiting_app=$app
app=
wait "$waiting_app"
result=$?
waiting_app=
waiting_reader=$reader
reader=
exec 3>&-
if [ -n "$waiting_reader" ]; then wait "$waiting_reader" || exit 93; waiting_reader=; fi
exit "$result"
"#,
        fifo = quote(&fifo),
        completion = quote(&completion)
    );
    let mut command = Command::new("/bin/sh");
    command
        .args([
            "-c",
            &script,
            "output-fixture",
            env!("CARGO_BIN_EXE_hydragen"),
        ])
        .args(arguments)
        .current_dir(directory)
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(stdout.reopen()?)
        .stderr(stderr.reopen()?);
    for key in [
        "CI",
        "NO_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "TERM",
    ] {
        command.env_remove(key);
    }
    command.envs(environment.iter().copied());
    let mut running = Running {
        child: command.spawn()?,
        deadline,
        ownership: GroupOwnership::new(trace),
        completion,
        reader_pid,
        status: None,
    };
    let reserve = Duration::from_millis(250).min(timeout / 2);
    let work_deadline = deadline - reserve;
    let code = loop {
        if Instant::now() >= work_deadline {
            running.cleanup()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "output fixture timed out during setup, command or reader collection",
            ));
        }
        if let Some(code) = running.completion_code()? {
            break code;
        }
        std::thread::yield_now();
    };
    running.final_signal()?;
    running.reap()?;
    let status = command_status(code)?;
    let capture = Capture {
        output: Output {
            status,
            stdout: fs::read(stdout.path())?,
            stderr: fs::read(stderr.path())?,
        },
        consumed: if consumed.exists() {
            fs::read(&consumed)?
        } else {
            Vec::new()
        },
    };
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "output capture exceeded fixture deadline",
        ));
    }
    Ok(capture)
}
