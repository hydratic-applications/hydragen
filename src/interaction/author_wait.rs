//! The foreground alone reads keys between native cliclack prompts.
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::author_discovery::{Choice, Preparation, Status};

pub(super) fn snapshot(
    preparation: &Preparation,
    cancelled: &AtomicBool,
    basic: bool,
) -> io::Result<Vec<Choice>> {
    preparation.enter_author();
    if preparation.status() == Status::Pending
        && let Err(error) = wait(preparation, cancelled, basic)
    {
        // Close publication immediately; the owning handle performs bounded
        // process cleanup on exit. Never add its cleanup budget to this wait loop.
        let _ = preparation.close_and_snapshot();
        return Err(error);
    }
    // Also surface Fatal and any fatal error racing the last status check.
    preparation.close_and_snapshot()
}

fn wait(preparation: &Preparation, cancelled: &AtomicBool, basic: bool) -> io::Result<()> {
    let mut raw = RawMode::enter()?;
    let result = (|| {
        let mut spinner = Spinner::new(preparation.status() == Status::Pending, basic)?;
        let result = (|| {
            while preparation.status() == Status::Pending {
                if poll_events(Duration::from_millis(20))?
                    && let Event::Key(key) = event::read()?
                    && key.kind != KeyEventKind::Release
                    && (key.code == KeyCode::Esc
                        || (matches!(key.code, KeyCode::Char('c' | 'C'))
                            && key.modifiers.contains(KeyModifiers::CONTROL)))
                {
                    cancelled.store(true, Ordering::Relaxed);
                    return Err(io::ErrorKind::Interrupted.into());
                }
            }
            Ok(())
        })();
        spinner
            .clear()
            .map_err(|error| wait_failure("clear the author loading indicator", error))?;
        result
    })();
    // A restoration error is an operational error, even after an observed key.
    // Explicit restoration is fallible; Drop is only a best-effort backup.
    raw.restore()
        .map_err(|error| wait_failure("restore the terminal after author waiting", error))?;
    result
}

fn poll_events(timeout: Duration) -> io::Result<bool> {
    #[cfg(all(test, unix))]
    if let Some(result) =
        TEST_POLL.with(|poll| poll.borrow_mut().as_mut().map(|poll| poll(timeout)))
    {
        return result;
    }
    event::poll(timeout)
}

// The terminal driver retains its real PTY so cleanup and restoration remain
// observable. Only the foreground poll result is injectable, only in unit builds;
// no release environment switch or competing terminal reader is introduced.
#[cfg(all(test, unix))]
type TestPoll = Box<dyn FnMut(Duration) -> io::Result<bool>>;

#[cfg(all(test, unix))]
thread_local! {
    static TEST_POLL: std::cell::RefCell<Option<TestPoll>> = const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, unix))]
pub(super) struct TestPollScope;

#[cfg(all(test, unix))]
pub(super) fn inject_poll_for_tests(poll: TestPoll) -> TestPollScope {
    TEST_POLL.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(poll);
    });
    TestPollScope
}

#[cfg(all(test, unix))]
impl Drop for TestPollScope {
    fn drop(&mut self) {
        TEST_POLL.with(|slot| *slot.borrow_mut() = None);
    }
}

fn wait_failure(action: &str, error: io::Error) -> io::Error {
    // Keep an operational Interrupted distinguishable from the bare Interrupted
    // returned for an observed key, even if clearing/restoring failed after it.
    io::Error::new(error.kind(), format!("Could not {action}: {error}"))
}

struct Spinner {
    native: Option<cliclack::ProgressBar>,
    plain_line: bool,
}

impl Spinner {
    fn new(pending: bool, basic: bool) -> io::Result<Self> {
        let mut guard = Self {
            native: pending.then(|| {
                let spinner = cliclack::spinner();
                spinner.start("Finding author suggestions…");
                spinner
            }),
            plain_line: pending && basic,
        };
        if guard.plain_line {
            // Indicatif hides its native spinner for TERM=dumb. Keep the same
            // meaningful message as a temporary plain line in that case.
            if let Err(error) = console::Term::stderr().write_line("│  Finding author suggestions…")
            {
                guard.clear()?;
                return Err(error);
            }
        }
        Ok(guard)
    }

    fn clear(&mut self) -> io::Result<()> {
        if let Some(spinner) = &self.native {
            spinner.clear();
        }
        if self.plain_line {
            console::Term::stderr().clear_last_lines(1)?;
            self.plain_line = false;
        }
        Ok(())
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}

#[cfg(not(windows))]
struct RawMode {
    restore: bool,
}

#[cfg(not(windows))]
impl RawMode {
    fn enter() -> io::Result<Self> {
        let mut guard = Self {
            restore: !crossterm::terminal::is_raw_mode_enabled()?,
        };
        if guard.restore {
            // Crossterm retains the complete previous Unix termios, not merely
            // an approximation of cooked mode.
            if let Err(error) = crossterm::terminal::enable_raw_mode() {
                guard.restore()?;
                return Err(error);
            }
        }
        Ok(guard)
    }

    fn restore(&mut self) -> io::Result<()> {
        if self.restore {
            crossterm::terminal::disable_raw_mode()?;
            self.restore = false;
        }
        Ok(())
    }
}

#[cfg(windows)]
struct RawMode {
    handle: windows_sys::Win32::Foundation::HANDLE,
    mode: u32,
    restore: bool,
}

#[cfg(windows)]
impl RawMode {
    fn enter() -> io::Result<Self> {
        use windows_sys::Win32::System::Console::{GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE};
        // SAFETY: GetStdHandle returns a borrowed process handle; the output
        // pointer is valid and no ownership is transferred.
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
        let mut mode = 0;
        if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut guard = Self {
            handle,
            mode,
            restore: true,
        };
        if let Err(error) = crossterm::terminal::enable_raw_mode() {
            guard.restore()?;
            return Err(error);
        }
        Ok(guard)
    }

    fn restore(&mut self) -> io::Result<()> {
        use windows_sys::Win32::System::Console::SetConsoleMode;
        if self.restore {
            // SAFETY: the borrowed console handle remains valid for this scope;
            // restore the exact mode read before entering raw mode.
            if unsafe { SetConsoleMode(self.handle, self.mode) } == 0 {
                return Err(io::Error::last_os_error());
            }
            self.restore = false;
        }
        Ok(())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_interruption_after_an_observed_key_is_not_reported_as_deliberate_cancellation() {
        for action in [
            "clear the author loading indicator",
            "restore the terminal after author waiting",
        ] {
            let failure = wait_failure(action, io::ErrorKind::Interrupted.into());
            assert!(failure.get_ref().is_some());
            let error = super::super::finish_input::<()>(Err(failure), true, || {
                panic!("an operational terminal failure must not become a cancellation report")
            })
            .unwrap_err();
            assert!(!super::super::is_reported_cancellation(&error));
            assert!(error.to_string().contains(action));
        }
    }
}
