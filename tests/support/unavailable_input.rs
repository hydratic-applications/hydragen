//! Direct terminal launch and bounded exceptional cleanup, only for Unix tests.
//! Existing shell-based sessions retain their original output readers.

use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use portable_pty::{Child, ExitStatus, MasterPty, PtyPair, PtySize, native_pty_system};

use super::{Event, TIMEOUT};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum OutputEnd {
    Eof,
    Shutdown,
    Error(String),
}

impl OutputEnd {
    pub(super) fn require_eof(&self) -> Result<(), String> {
        match self {
            Self::Eof => Ok(()),
            Self::Shutdown => Err("Output reader shut down instead of reaching EOF".to_owned()),
            Self::Error(error) => Err(format!("Output reader failed: {error}")),
        }
    }
}

// Keep OS signalling separate from status observation so exceptional paths can
// be tested without deliberately leaking a real child or depending on permissions.
trait Process: Send + Sync {
    fn pid(&self) -> Option<u32>;
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>>;
    fn signal(&mut self, signal: libc::c_int) -> io::Result<()>;
}

struct PtyChild(Box<dyn Child + Send + Sync>);

impl Process for PtyChild {
    fn pid(&self) -> Option<u32> {
        self.0.process_id()
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.0.try_wait()
    }

    fn signal(&mut self, signal: libc::c_int) -> io::Result<()> {
        let pid = self
            .pid()
            .ok_or_else(|| io::Error::other("Missing child PID"))?;
        // SAFETY: this is our owned, not-known-reaped child, never a process
        // group. Only the owning guard observes/reaps it and sends signals.
        if unsafe { libc::kill(pid as libc::pid_t, signal) } == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

struct OutputWorker {
    stdout: bool,
    shutdown: Option<UnixStream>,
    thread: Option<JoinHandle<OutputEnd>>,
    end: Option<OutputEnd>,
}

impl OutputWorker {
    fn join_finished(&mut self) {
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished) {
            self.end = Some(match self.thread.take().unwrap().join() {
                Ok(end) => end,
                Err(_) => OutputEnd::Error("Output reader panicked".to_owned()),
            });
        }
    }
}

/// Owns the child immediately after spawn, including during reader setup.
/// Reaped status is independent of output EOF and remains cached for Drop.
pub(super) struct OwnedChild {
    process: Box<dyn Process>,
    status: Option<ExitStatus>,
    workers: Vec<OutputWorker>,
    cleanup_attempted: bool,
}

impl OwnedChild {
    pub(super) fn new(child: Box<dyn Child + Send + Sync>) -> Self {
        Self {
            process: Box::new(PtyChild(child)),
            status: None,
            workers: Vec::new(),
            cleanup_attempted: false,
        }
    }

    pub(super) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if self.status.is_none() {
            self.status = self.process.try_wait()?;
        }
        Ok(self.status.clone())
    }

    pub(super) fn reader_count(&self) -> usize {
        self.workers.len()
    }

    pub(super) fn readers_stopped(&self) -> bool {
        self.workers.iter().all(|worker| worker.thread.is_none())
    }

    pub(super) fn diagnostic(&self) -> String {
        let readers: Vec<_> = self
            .workers
            .iter()
            .map(|worker| {
                let stream = if worker.stdout { "stdout" } else { "stderr" };
                format!(
                    "{stream}: {:?}, finished={}",
                    worker.end,
                    worker.thread.as_ref().is_none_or(JoinHandle::is_finished)
                )
            })
            .collect();
        format!(
            "PID {:?}, last observed status: {:?}, readers: [{}]",
            self.process.pid(),
            self.status,
            readers.join(", ")
        )
    }

    pub(super) fn cleanup(&mut self) -> Result<(), String> {
        self.cleanup_until(cleanup_deadline(Instant::now()))
    }

    fn cleanup_until(&mut self, deadline: Instant) -> Result<(), String> {
        self.cleanup_attempted = true;
        // Each worker owns the only other endpoint. Closing *all* supervisor
        // endpoints wakes poll even if a slave elsewhere prevents output EOF.
        // Do this before observing child termination, not after its deadline.
        for worker in &mut self.workers {
            worker.shutdown.take();
        }
        let mut observation_error = None;
        let mut signal_error = None;
        if let Err(error) = self.try_wait() {
            observation_error = Some(error.to_string());
        }
        if self.status.is_none() {
            if let Err(error) = self.process.signal(libc::SIGHUP) {
                signal_error = Some(format!("SIGHUP: {error}"));
            }
            if let Err(error) = self.try_wait() {
                observation_error = Some(error.to_string());
            }
            if self.status.is_none() {
                // Do not use portable-pty's cloned killer: it sends only HUP.
                // No blocking wait follows either successful or failed signals.
                if let Err(error) = self.process.signal(libc::SIGKILL) {
                    signal_error = Some(format!("SIGKILL: {error}"));
                }
            }
        }
        loop {
            if let Err(error) = self.try_wait() {
                observation_error = Some(error.to_string());
            }
            for worker in &mut self.workers {
                worker.join_finished();
            }
            let complete = self.status.is_some() && self.readers_stopped();
            let errors = format!(
                "status observation error: {observation_error:?}; termination error: {signal_error:?}"
            );
            if complete {
                if observation_error.is_some() || signal_error.is_some() {
                    return Err(format!(
                        "Terminal cleanup encountered errors; {}; {errors}",
                        self.diagnostic()
                    ));
                }
                return Ok(());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(format!(
                    "Incomplete terminal cleanup; {}; {errors}",
                    self.diagnostic()
                ));
            }
            // Bounded status/worker observation, not an input-readiness sleep.
            thread::park_timeout(Duration::from_millis(10).min(remaining));
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.cleanup_attempted
            && let Err(error) = self.cleanup()
        {
            report_cleanup_failure(error);
        }
    }
}

pub(super) fn report_cleanup_failure(error: String) {
    // Never double-panic during a test failure, but retain the diagnostic. A
    // live worker/unknown child is explicitly incomplete, not a successful detach.
    if thread::panicking() {
        eprintln!("{error}");
    } else {
        panic!("{error}");
    }
}

fn cleanup_deadline(start: Instant) -> Instant {
    start + TIMEOUT
}

struct PreparedReader {
    output: File,
    supervisor: UnixStream,
    notification: UnixStream,
    stdout: bool,
}

impl PreparedReader {
    fn new(terminal: &PtyPair, stdout: bool) -> io::Result<Self> {
        let output = duplicate_master(terminal.master.as_ref())?;
        // SAFETY: output owns a valid descriptor. These fcntl operations only
        // inspect/update its file status flags. Only new, output-only PTYs use it.
        let flags = unsafe { libc::fcntl(output.as_raw_fd(), libc::F_GETFL) };
        if flags == -1
            || unsafe { libc::fcntl(output.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                == -1
        {
            return Err(io::Error::last_os_error());
        }
        let (supervisor, notification) = UnixStream::pair()?;
        notification.set_nonblocking(true)?;
        Ok(Self {
            output,
            supervisor,
            notification,
            stdout,
        })
    }

    fn start(self, child: &mut OwnedChild, sender: Sender<Event>) -> io::Result<()> {
        let Self {
            output,
            supervisor,
            notification,
            stdout,
        } = self;
        let thread = thread::Builder::new()
            .name(
                if stdout {
                    "terminal-stdout"
                } else {
                    "terminal-stderr"
                }
                .to_owned(),
            )
            .spawn(move || {
                let end = read_output(output, notification, stdout, &sender);
                // read_output releases both descriptors before reporting completion.
                let _ = sender.send(Event::ReaderEnd(end.clone()));
                end
            })?;
        child.workers.push(OutputWorker {
            stdout,
            shutdown: Some(supervisor),
            thread: Some(thread),
            end: None,
        });
        Ok(())
    }
}

fn read_output(
    mut output: File,
    mut notification: UnixStream,
    stdout: bool,
    sender: &Sender<Event>,
) -> OutputEnd {
    let mut buffer = [0; 4096];
    loop {
        let mut fds = [
            libc::pollfd {
                fd: notification.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: output.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: both descriptors and the array are live for this poll. Shutdown
        // wakes an otherwise indefinite readiness wait by closing the peer socket.
        if unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) } == -1 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return OutputEnd::Error(format!("Output readiness: {error}"));
        }
        // Prioritise shutdown even if output is continuously readable.
        if fds[0].revents != 0 {
            match notification.read(&mut [0]) {
                Ok(0) => return OutputEnd::Shutdown,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                    ) =>
                {
                    continue;
                }
                result => {
                    return OutputEnd::Error(format!("Unexpected reader notification: {result:?}"));
                }
            }
        }
        if fds[1].revents & libc::POLLNVAL != 0 {
            return OutputEnd::Error("Invalid output descriptor".to_owned());
        }
        if fds[1].revents != 0 {
            match output.read(&mut buffer) {
                Ok(0) => return OutputEnd::Eof,
                Ok(count) => {
                    if sender
                        .send(Event::Data(stdout, buffer[..count].to_vec()))
                        .is_err()
                    {
                        return OutputEnd::Error("Output receiver disconnected".to_owned());
                    }
                }
                // Linux PTY masters report EIO once the last slave closes;
                // macOS also permits a zero-length read. Neither is cancellation.
                Err(error) if error.raw_os_error() == Some(libc::EIO) => return OutputEnd::Eof,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
                    ) => {}
                Err(error) => return OutputEnd::Error(format!("Output read: {error}")),
            }
        }
    }
}

fn duplicate_master(master: &dyn MasterPty) -> io::Result<File> {
    let fd = master
        .as_raw_fd()
        .ok_or_else(|| io::Error::other("PTY has no raw descriptor"))?;
    // SAFETY: master outlives the borrow; the returned File owns a new descriptor.
    let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
    Ok(File::from(borrowed.try_clone_to_owned()?))
}

fn open_slave(terminal: &PtyPair) -> io::Result<File> {
    let path = terminal
        .master
        .tty_name()
        .ok_or_else(|| io::Error::other("PTY has no slave path"))?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY | libc::O_CLOEXEC)
        .open(path)
}

// This function runs after fork in a multithreaded process. No allocations,
// formatting, locks, unwinding or other non-async-signal-safe operations belong here.
unsafe fn establish_session(tty_probe: &'static CStr) -> io::Result<()> {
    // SAFETY: setsid/open/close are async-signal-safe; the C string is static.
    if unsafe { libc::setsid() } == -1 {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe {
        libc::open(
            tty_probe.as_ptr(),
            libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
        )
    };
    if fd >= 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(io::Error::from_raw_os_error(libc::ENOTTY));
    }
    // Capture errno before any subsequent OS call can change it. Constructing
    // the raw OS error is allocation-free; add context only back in the parent.
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ENXIO) {
        Ok(())
    } else {
        Err(error)
    }
}

pub(super) struct DirectLaunch {
    pub child: OwnedChild,
    pub input_master: Box<dyn MasterPty + Send>,
    pub input_writer: File,
}

pub(super) fn spawn(
    directory: &Path,
    arguments: &[&str],
    sender: Sender<Event>,
) -> io::Result<DirectLaunch> {
    spawn_command(guided_command(directory, arguments), sender, c"/dev/tty")
}

fn guided_command(directory: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hydragen"));
    command
        .args(arguments)
        .current_dir(directory)
        .env("TERM", "xterm-256color")
        .env("CLICOLOR_FORCE", "1");
    for variable in ["CI", "COLUMNS", "NO_COLOR", "CLICOLOR"] {
        command.env_remove(variable);
    }
    command
}

fn spawn_command(
    mut command: Command,
    sender: Sender<Event>,
    tty_probe: &'static CStr,
) -> io::Result<DirectLaunch> {
    let system = native_pty_system();
    let size = PtySize {
        rows: 24,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    };
    let terminal = || system.openpty(size).map_err(io::Error::other);
    let input = terminal()?;
    let stdout = terminal()?;
    let stderr = terminal()?;
    let input_writer = duplicate_master(input.master.as_ref())?;
    let outputs = [
        PreparedReader::new(&stdout, true)?,
        PreparedReader::new(&stderr, false)?,
    ];
    command
        .stdin(open_slave(&input)?)
        .stdout(open_slave(&stdout)?)
        .stderr(open_slave(&stderr)?);
    // SAFETY: callback is limited to the async-signal-safe setup above, using a
    // static C string. All terminal opens and reader allocation happen in parent.
    unsafe {
        command.pre_exec(move || establish_session(tty_probe));
    }
    let mut child = OwnedChild::new(Box::new(command.spawn().map_err(|error| {
        io::Error::new(error.kind(), format!("Direct terminal launch (setsid and no-controlling-terminal verification at {tty_probe:?}): {error}"))
    })?));
    // The guard exists before any new reader thread can be started. Neither
    // Command's Stdio nor portable-pty's original slaves may retain output slaves.
    drop(command);
    drop(stdout);
    drop(stderr);
    drop(input.slave);
    for output in outputs {
        output.start(&mut child, sender.clone())?;
    }
    Ok(DirectLaunch {
        child,
        input_master: input.master,
        input_writer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, mpsc};

    use crate::{TERMINAL_SESSION, support::tempdir, visible_text};

    #[derive(Default)]
    struct Trace {
        signals: Vec<libc::c_int>,
        reaped: bool,
    }

    struct TrackedProcess {
        process: Box<dyn Process>,
        trace: Arc<Mutex<Trace>>,
    }

    impl Process for TrackedProcess {
        fn pid(&self) -> Option<u32> {
            self.process.pid()
        }

        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            let status = self.process.try_wait()?;
            if status.is_some() {
                self.trace.lock().unwrap().reaped = true;
            }
            Ok(status)
        }

        fn signal(&mut self, signal: libc::c_int) -> io::Result<()> {
            self.trace.lock().unwrap().signals.push(signal);
            self.process.signal(signal)
        }
    }

    fn track(child: &mut OwnedChild) -> Arc<Mutex<Trace>> {
        let trace = Arc::new(Mutex::new(Trace::default()));
        // Replace via swap rather than move a field out of a Drop type.
        let mut process: Box<dyn Process> = Box::new(FailedProcess);
        std::mem::swap(&mut child.process, &mut process);
        child.process = Box::new(TrackedProcess {
            process,
            trace: trace.clone(),
        });
        trace
    }

    fn wait_for_name(child: &mut OwnedChild, events: &mpsc::Receiver<Event>) {
        let deadline = Instant::now() + TIMEOUT;
        let mut stderr = Vec::new();
        while !visible_text(&stderr).contains("Name") {
            assert!(
                child.try_wait().unwrap().is_none(),
                "{}",
                child.diagnostic()
            );
            match events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap()
            {
                Event::Data(false, bytes) => stderr.extend(bytes),
                Event::Data(true, bytes) => assert!(bytes.is_empty()),
                Event::ReaderEnd(end) => panic!("Unexpected output end before Name: {end:?}"),
                Event::Closed => panic!("Unexpected legacy output reader"),
            }
        }
    }

    #[test]
    fn verification_rejects_an_openable_probe_and_preserves_unexpected_errno() {
        let _lock = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = tempdir().unwrap();
        for (probe, errno) in [
            (c"/dev/null", libc::ENOTTY),
            (c"/dev/tty/hydragen-impossible-child", libc::ENOTDIR),
        ] {
            let (sender, events) = mpsc::channel();
            let error =
                match spawn_command(guided_command(directory.path(), &["init"]), sender, probe) {
                    Ok(_) => panic!("Unexpected successful verification with {probe:?}"),
                    Err(error) => error.to_string(),
                };
            assert!(
                error.contains("no-controlling-terminal verification"),
                "{error}"
            );
            assert!(error.contains(&format!("os error {errno}")), "{error}");
            assert!(matches!(
                events.try_recv(),
                Err(mpsc::TryRecvError::Disconnected)
            ));
            assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        }
    }

    #[test]
    fn cleanup_forcefully_reaps_a_child_that_ignores_hup_and_stops_its_readers() {
        let _lock = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = tempdir().unwrap();
        let (sender, events) = mpsc::channel();
        let mut command = guided_command(directory.path(), &["init"]);
        // SAFETY: signal is async-signal-safe. Ignored HUP survives exec; this
        // affects only this test child, with no production environment switch.
        unsafe {
            command.pre_exec(|| {
                if libc::signal(libc::SIGHUP, libc::SIG_IGN) == libc::SIG_ERR {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(())
                }
            });
        }
        let mut launch = spawn_command(command, sender, c"/dev/tty").unwrap();
        let trace = track(&mut launch.child);
        wait_for_name(&mut launch.child, &events);
        launch.child.cleanup().unwrap();
        assert_eq!(trace.lock().unwrap().signals, [libc::SIGHUP, libc::SIGKILL]);
        assert!(trace.lock().unwrap().reaped);
        assert!(!launch.child.status.as_ref().unwrap().success());
        assert!(
            launch
                .child
                .workers
                .iter()
                .all(|worker| worker.thread.is_none())
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn dropping_a_guarded_launch_reaps_the_child_and_stops_started_readers() {
        let _lock = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let directory = tempdir().unwrap();
        let (sender, events) = mpsc::channel();
        let mut launch = spawn_command(
            guided_command(directory.path(), &["init"]),
            sender,
            c"/dev/tty",
        )
        .unwrap();
        let trace = track(&mut launch.child);
        wait_for_name(&mut launch.child, &events);
        let setup_result: io::Result<()> = {
            let _guarded_launch = launch;
            Err(io::Error::other(
                "controlled setup failure after reader startup",
            ))
        };
        assert!(
            setup_result
                .unwrap_err()
                .to_string()
                .contains("controlled setup failure")
        );
        assert!(trace.lock().unwrap().reaped);
        let mut ends = 0;
        while let Ok(event) = events.try_recv() {
            if let Event::ReaderEnd(_) = event {
                ends += 1;
            }
        }
        assert_eq!(ends, 2, "setup guard did not stop both output workers");
    }

    #[test]
    fn reaped_child_with_a_retained_output_slave_is_not_signalled_and_reader_stops() {
        let _lock = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let terminal = native_pty_system().openpty(PtySize::default()).unwrap();
        let reader = PreparedReader::new(&terminal, true).unwrap();
        let (sender, events) = mpsc::channel();
        let mut command = Command::new(env!("CARGO_BIN_EXE_hydragen"));
        command
            .arg("--version")
            .stdout(open_slave(&terminal).unwrap());
        let mut child = OwnedChild::new(Box::new(command.spawn().unwrap()));
        drop(command);
        let trace = track(&mut child);
        reader.start(&mut child, sender).unwrap();
        let deadline = Instant::now() + TIMEOUT;
        let mut output = Vec::new();
        while child.try_wait().unwrap().is_none() || output.is_empty() {
            assert!(Instant::now() < deadline, "{}", child.diagnostic());
            if let Ok(event) = events.recv_timeout(Duration::from_millis(10)) {
                match event {
                    Event::Data(true, bytes) => output.extend(bytes),
                    Event::ReaderEnd(end) => {
                        panic!("Retained slave unexpectedly completed: {end:?}")
                    }
                    _ => panic!("Unexpected output event"),
                }
            }
        }
        assert!(visible_text(&output).contains(env!("CARGO_PKG_VERSION")));
        assert!(!child.workers[0].thread.as_ref().unwrap().is_finished());
        child.cleanup().unwrap();
        assert!(trace.lock().unwrap().signals.is_empty());
        assert!(trace.lock().unwrap().reaped);
        assert_eq!(child.workers[0].end, Some(OutputEnd::Shutdown));
        assert!(child.workers[0].thread.is_none());
        // Cancellation cannot satisfy Session's normal EOF completion rule.
        assert!(
            child.workers[0]
                .end
                .as_ref()
                .unwrap()
                .require_eof()
                .is_err()
        );
        // Releasing the terminal allocation is not what woke the worker.
        drop(terminal);
    }

    struct FailedProcess;

    impl Process for FailedProcess {
        fn pid(&self) -> Option<u32> {
            Some(424242)
        }
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            Err(io::Error::other("controlled status observation failure"))
        }
        fn signal(&mut self, _: libc::c_int) -> io::Result<()> {
            Err(io::Error::other("controlled termination failure"))
        }
    }

    #[test]
    fn failed_observation_and_termination_share_a_fresh_ten_second_cleanup_deadline() {
        let _lock = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let terminal = native_pty_system().openpty(PtySize::default()).unwrap();
        let reader = PreparedReader::new(&terminal, false).unwrap();
        let mut child = OwnedChild {
            process: Box::new(FailedProcess),
            status: None,
            workers: Vec::new(),
            cleanup_attempted: false,
        };
        let (sender, events) = mpsc::channel();
        reader.start(&mut child, sender).unwrap();
        let expired_normal_deadline = Instant::now() - TIMEOUT;
        let start = Instant::now();
        let deadline = cleanup_deadline(start);
        assert_eq!(deadline.duration_since(start), Duration::from_secs(10));
        assert!(deadline > expired_normal_deadline + TIMEOUT);
        let error = child.cleanup_until(deadline).unwrap_err();
        assert!(Instant::now() >= deadline);
        assert!(
            start.elapsed() < TIMEOUT * 2,
            "cleanup used serial child/reader deadlines"
        );
        for detail in [
            "Incomplete terminal cleanup",
            "424242",
            "last observed status: None",
            "readers:",
            "stderr:",
            "Shutdown",
            "controlled status observation failure",
            "controlled termination failure",
        ] {
            assert!(error.contains(detail), "Missing {detail:?}: {error}");
        }
        assert!(
            child.status.is_none(),
            "observation error is not termination"
        );
        assert!(child.workers[0].thread.is_none());
        assert_eq!(child.workers[0].end, Some(OutputEnd::Shutdown));
        assert!(matches!(
            events.try_recv(),
            Ok(Event::ReaderEnd(OutputEnd::Shutdown))
        ));
        // This is a test double: no actual PID was signalled or abandoned.
        drop(terminal);
    }

    #[test]
    fn incomplete_worker_cleanup_does_not_join_or_report_success() {
        let (gate, mut worker_gate) = UnixStream::pair().unwrap();
        let worker = thread::spawn(move || {
            // Deliberately ignore the normal shutdown channel. Dropping the
            // test-owned gate always releases this worker, including on panic.
            worker_gate.read_to_end(&mut Vec::new()).unwrap();
            OutputEnd::Shutdown
        });
        let mut child = OwnedChild {
            process: Box::new(FailedProcess),
            status: Some(ExitStatus::with_exit_code(0)),
            workers: vec![OutputWorker {
                stdout: true,
                shutdown: None,
                thread: Some(worker),
                end: None,
            }],
            cleanup_attempted: false,
        };
        // An expired controlled deadline must not enter an unconditional join.
        let error = child.cleanup_until(Instant::now()).unwrap_err();
        for detail in [
            "Incomplete terminal cleanup",
            "424242",
            "last observed status: Some",
            "stdout:",
            "finished=false",
        ] {
            assert!(error.contains(detail), "Missing {detail:?}: {error}");
        }
        assert!(child.workers[0].thread.is_some());
        drop(gate);
        // Recover the deliberately delayed test worker without detaching it.
        let deadline = Instant::now() + TIMEOUT;
        while child.workers[0].thread.is_some() {
            child.workers[0].join_finished();
            assert!(Instant::now() < deadline, "{}", child.diagnostic());
            thread::park_timeout(Duration::from_millis(1));
        }
        assert_eq!(child.workers[0].end, Some(OutputEnd::Shutdown));
    }

    #[test]
    fn only_genuine_eof_is_accepted_as_normal_output_completion() {
        assert!(OutputEnd::Eof.require_eof().is_ok());
        assert!(
            OutputEnd::Shutdown
                .require_eof()
                .unwrap_err()
                .contains("instead of reaching EOF")
        );
        assert!(
            OutputEnd::Error("controlled read failure".into())
                .require_eof()
                .unwrap_err()
                .contains("controlled read failure")
        );
    }

    #[test]
    fn cancelled_output_cannot_make_a_reaped_session_finish_successfully() {
        let lock = TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let (sender, events) = mpsc::channel();
        let mut session = crate::Session {
            _terminal_session: lock,
            child: OwnedChild {
                process: Box::new(FailedProcess),
                status: Some(ExitStatus::with_exit_code(0)),
                workers: vec![OutputWorker {
                    stdout: true,
                    shutdown: None,
                    thread: None,
                    end: Some(OutputEnd::Shutdown),
                }],
                cleanup_attempted: false,
            },
            writer: None,
            prompt_terminal: None,
            stdout_terminal: None,
            events,
            readers: Vec::new(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            closed: 0,
            finished: false,
        };
        sender.send(Event::ReaderEnd(OutputEnd::Shutdown)).unwrap();
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| session.finish()))
            .expect_err("cancelled output falsely satisfied normal completion");
        let message = failure.downcast_ref::<String>().unwrap();
        assert!(message.contains("instead of reaching EOF"), "{message}");
        assert_eq!(session.closed, 0);
        assert!(!session.finished);
    }
}
