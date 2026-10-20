//! A runner owns each child for its entire lifetime. The foreground only closes
//! the shared context; it never acquires a child-control lock or joins a worker.
use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use process_wrap::std::{ChildWrapper, CommandWrap};

use super::Context;
use super::preparation::CLEANUP;

const LIMIT: usize = 1024 * 1024;
const POLL: Duration = Duration::from_millis(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Failure {
    Unavailable,
    /// A capture or child did not acknowledge cleanup. Never start another
    /// command in this lane: even escaped descendants cannot accumulate helpers.
    Abandoned,
}

struct Reservation(Context);
impl Drop for Reservation {
    fn drop(&mut self) {
        self.0.release_process();
    }
}

struct Capture {
    thread: JoinHandle<io::Result<Vec<u8>>>,
    overflow: Arc<AtomicBool>,
}

fn capture(reader: impl Read + Send + 'static) -> io::Result<Capture> {
    let overflow = Arc::new(AtomicBool::new(false));
    let signal = overflow.clone();
    let thread = thread::Builder::new()
        .name("author-capture".into())
        .spawn(move || {
            // Exactly LIMIT bytes plus one overflow-detection byte, never an
            // unbounded read_to_end allocation, including under simultaneous floods.
            let mut bytes = vec![0; LIMIT + 1];
            let mut count = 0;
            let mut reader = reader;
            while count < bytes.len() {
                match reader.read(&mut bytes[count..]) {
                    Ok(0) => break,
                    Ok(read) => count += read,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }
            if count > LIMIT {
                signal.store(true, Ordering::Release);
            }
            bytes.truncate(count);
            Ok(bytes)
        })?;
    Ok(Capture { thread, overflow })
}

pub(super) fn run(
    command: Command,
    context: &Context,
    source_deadline: Option<Instant>,
) -> Result<Vec<u8>, Failure> {
    run_with_spawn(command, context, source_deadline, |mut command| {
        command.spawn()
    })
}

fn run_with_spawn(
    command: Command,
    context: &Context,
    source_deadline: Option<Instant>,
    spawn: impl FnOnce(CommandWrap) -> io::Result<Box<dyn ChildWrapper>>,
) -> Result<Vec<u8>, Failure> {
    run_owned(command, context, source_deadline, spawn, capture)
}

fn run_owned(
    mut command: Command,
    context: &Context,
    source_deadline: Option<Instant>,
    spawn: impl FnOnce(CommandWrap) -> io::Result<Box<dyn ChildWrapper>>,
    mut capture_reader: impl FnMut(Box<dyn Read + Send>) -> io::Result<Capture>,
) -> Result<Vec<u8>, Failure> {
    if source_deadline.is_some_and(|deadline| context.now() >= deadline)
        || !context.reserve_process()
    {
        return Err(Failure::Unavailable);
    }
    let _reservation = Reservation(context.clone());
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut command = CommandWrap::from(command);
    #[cfg(unix)]
    command.wrap(process_wrap::std::ProcessGroup::leader());
    #[cfg(windows)]
    command.wrap(process_wrap::std::JobObject);
    // OS spawn and every child operation are deliberately outside the state lock.
    let mut child = spawn(command).map_err(|_| Failure::Unavailable)?;
    let mut captures = Vec::with_capacity(2);
    let mut failed =
        !context.active() || source_deadline.is_some_and(|deadline| context.now() >= deadline);
    if !failed {
        for reader in [
            child
                .stdout()
                .take()
                .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>),
            child
                .stderr()
                .take()
                .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>),
        ] {
            match reader.map(&mut capture_reader) {
                Some(Ok(capture)) => captures.push(capture),
                _ => {
                    failed = true;
                    break;
                }
            }
        }
    }
    let mut status = None;
    while !failed {
        if !context.active()
            || source_deadline.is_some_and(|deadline| context.now() >= deadline)
            || captures
                .iter()
                .any(|capture| capture.overflow.load(Ordering::Acquire))
        {
            failed = true;
            break;
        }
        match child.try_wait() {
            Ok(Some(exited)) => {
                status = Some(exited);
                break;
            }
            Ok(None) => thread::sleep(POLL),
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    // Also kill the group/job after a successful root exit. Descendants may
    // still hold inherited pipes, even though the root itself has been reaped.
    let _ = child.start_kill();
    let until = Instant::now() + CLEANUP;
    loop {
        if status.is_none() {
            status = child.try_wait().ok().flatten();
        }
        if status.is_some() && captures.iter().all(|capture| capture.thread.is_finished()) {
            break;
        }
        if Instant::now() >= until {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let mut outputs = Vec::with_capacity(2);
    let mut abandoned = status.is_none();
    for capture in captures {
        if capture.thread.is_finished() {
            match capture.thread.join() {
                Ok(Ok(output)) if output.len() <= LIMIT => outputs.push(output),
                _ => failed = true,
            }
        } else {
            // Detach only this lane's bounded reader; caller must abort the lane.
            abandoned = true;
        }
    }
    if status.is_none() {
        // A root stuck in an uninterruptible kernel operation cannot be made
        // reapable by a userspace timeout. Keep ownership in this already
        // isolated lane, rather than dropping an unreaped child or spawning an
        // accumulating helper. Foreground cancel only awaits its bounded 100ms
        // acknowledgement; this lane performs no more source calls or captures.
        // Ordinary capture draining ended above at the fixed cleanup cutoff.
        drop(outputs);
        while !matches!(child.try_wait(), Ok(Some(_))) {
            thread::sleep(POLL);
        }
        return Err(Failure::Abandoned);
    }
    if abandoned {
        return Err(Failure::Abandoned);
    }
    if failed || !status.is_some_and(|status| status.success()) || outputs.len() != 2 {
        return Err(Failure::Unavailable);
    }
    Ok(outputs.remove(0))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn shell(script: &str) -> Command {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn captures_both_streams_and_disconnects_stdin() {
        let context = Context::for_tests();
        assert_eq!(
            run(
                shell("read answer || :; printf hello; printf hidden >&2"),
                &context,
                None
            )
            .unwrap(),
            b"hello"
        );
    }

    #[test]
    fn missing_failing_and_flooding_processes_are_silent_failures() {
        let context = Context::for_tests();
        assert_eq!(
            run(
                Command::new("/nonexistent/hydragen-fake-tool"),
                &context,
                None
            ),
            Err(Failure::Unavailable)
        );
        assert_eq!(
            run(shell("printf secret >&2; exit 7"), &context, None),
            Err(Failure::Unavailable)
        );
        for script in [
            "/usr/bin/yes flood",
            "/usr/bin/yes flood >&2",
            "/usr/bin/yes out & /usr/bin/yes err >&2 & wait",
        ] {
            assert_eq!(
                run(
                    shell(script),
                    &context,
                    Some(Instant::now() + Duration::from_secs(2))
                ),
                Err(Failure::Unavailable)
            );
        }
    }

    #[test]
    fn stalled_process_is_killed_and_reaped_at_source_deadline() {
        let context = Context::for_tests();
        let start = Instant::now();
        assert_eq!(
            run(
                shell("/bin/sleep 30"),
                &context,
                Some(start + Duration::from_millis(30))
            ),
            Err(Failure::Unavailable)
        );
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(context.reserve_process());
        context.release_process();
    }

    #[test]
    fn root_exit_with_descendant_inherited_pipes_does_not_hang_capture() {
        let context = Context::for_tests();
        let start = Instant::now();
        assert_eq!(
            run(
                shell("/bin/sleep 30 & printf complete; exit 0"),
                &context,
                Some(start + Duration::from_secs(2))
            )
            .unwrap(),
            b"complete"
        );
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn capture_buffer_has_a_single_detection_byte() {
        let capture = capture(io::repeat(b'x')).unwrap();
        while !capture.thread.is_finished() {
            thread::yield_now();
        }
        assert!(capture.overflow.load(Ordering::Acquire));
        assert_eq!(capture.thread.join().unwrap().unwrap().len(), LIMIT + 1);
    }

    #[test]
    fn closure_during_reserved_spawn_cannot_block_foreground_or_start_capture() {
        use std::sync::mpsc;
        let context = Context::for_tests();
        let worker_context = context.clone();
        let (reserved, reservation) = mpsc::channel();
        let (spawn, continue_spawn) = mpsc::channel();
        let (done, completion) = mpsc::channel();
        let worker = thread::spawn(move || {
            done.send(run_with_spawn(
                shell("/bin/sleep 30"),
                &worker_context,
                None,
                |mut command| {
                    reserved.send(()).unwrap();
                    continue_spawn.recv_timeout(Duration::from_secs(2)).unwrap();
                    command.spawn()
                },
            ))
            .unwrap();
        });
        reservation.recv_timeout(Duration::from_secs(2)).unwrap();
        context.close_for_tests();
        assert!(!context.reserve_process());
        spawn.send(()).unwrap();
        assert_eq!(
            completion.recv_timeout(Duration::from_secs(2)).unwrap(),
            Err(Failure::Unavailable)
        );
        while !worker.is_finished() {
            thread::yield_now();
        }
        worker.join().unwrap();
    }

    #[test]
    fn incomplete_capture_aborts_lane_after_bounded_drain_without_joining() {
        use std::sync::mpsc;
        struct HeldPipe(mpsc::Receiver<()>, mpsc::Sender<()>);
        impl Read for HeldPipe {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                self.0
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(io::Error::other)?;
                self.1.send(()).unwrap();
                Ok(0)
            }
        }
        let (release, held) = mpsc::channel();
        let (finished, completion) = mpsc::channel();
        let mut held = Some(HeldPipe(held, finished));
        let mut captures = 0;
        let context = Context::for_tests();
        let start = Instant::now();
        let result = run_owned(
            shell("printf complete"),
            &context,
            None,
            |mut command| command.spawn(),
            |reader| {
                captures += 1;
                if captures == 2 {
                    capture(held.take().unwrap())
                } else {
                    capture(reader)
                }
            },
        );
        assert_eq!(result, Err(Failure::Abandoned));
        assert_eq!(captures, 2);
        assert!(start.elapsed() < Duration::from_secs(1));
        release.send(()).unwrap();
        completion.recv_timeout(Duration::from_secs(2)).unwrap();
    }

    #[test]
    fn timed_out_direct_child_is_reaped_not_left_a_zombie() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pid");
        let context = Context::for_tests();
        let mut command = shell("printf '%s' \"$$\" > \"$PID_FILE\"; exec /bin/sleep 30");
        command.env("PID_FILE", &path);
        assert_eq!(
            run(
                command,
                &context,
                Some(Instant::now() + Duration::from_millis(100))
            ),
            Err(Failure::Unavailable)
        );
        let pid: libc::pid_t = std::fs::read_to_string(path).unwrap().parse().unwrap();
        let mut status = 0;
        // SAFETY: pid is the fixture's child PID; status is a valid writable int.
        assert_eq!(
            unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
        // SAFETY: signal zero only probes existence; it sends no signal.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    }
}
