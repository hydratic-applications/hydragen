//! Permission-denial prerequisites, not UID guesses. Probe before taking snapshots.
#![allow(dead_code)]

#[cfg(unix)]
use std::fs;
use std::io;
#[cfg(unix)]
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Setup<T> {
    Ready(T),
    Unsupported(String),
}

/// Only PermissionDenied establishes the intended precondition. Other failures
/// are fixture errors; successful access means the environment cannot test denial.
pub fn classify_denial(result: io::Result<()>, operation: &str) -> io::Result<Setup<()>> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => Ok(Setup::Ready(())),
        Err(error) => Err(error),
        Ok(()) => Ok(Setup::Unsupported(format!(
            "{operation} still succeeds with the restrictive permission bits"
        ))),
    }
}

#[cfg(unix)]
#[derive(Debug)]
pub struct PermissionGuard {
    path: PathBuf,
    original: Option<fs::Permissions>,
}

#[cfg(unix)]
impl PermissionGuard {
    fn restrict(path: &Path, mode: u32) -> io::Result<Self> {
        use std::os::unix::fs::PermissionsExt;
        let guard = Self {
            path: path.into(),
            original: Some(fs::metadata(path)?.permissions()),
        };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
        Ok(guard)
    }

    pub fn restore(&mut self) -> io::Result<()> {
        if let Some(original) = &self.original {
            fs::set_permissions(&self.path, original.clone())?;
            self.original = None;
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for PermissionGuard {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            if std::thread::panicking() {
                eprintln!(
                    "Permission fixture cleanup failed at {}: {error}",
                    self.path.display()
                );
            } else {
                panic!(
                    "Permission fixture cleanup failed at {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}

#[cfg(unix)]
fn finish_probe(
    mut guard: PermissionGuard,
    probe: io::Result<()>,
    operation: &str,
) -> io::Result<Setup<PermissionGuard>> {
    match classify_denial(probe, operation) {
        Ok(Setup::Ready(())) => Ok(Setup::Ready(guard)),
        Ok(Setup::Unsupported(reason)) => {
            guard.restore()?;
            Ok(Setup::Unsupported(reason))
        }
        Err(error) => {
            guard.restore()?;
            Err(error)
        }
    }
}

#[cfg(unix)]
pub fn deny_read(path: &Path) -> io::Result<Setup<PermissionGuard>> {
    deny_read_with_probe(path, || fs::File::open(path).map(drop))
}

/// Injectable only in test support, to exercise unsupported/setup-error cleanup
/// without claiming that a privileged runtime was actually executed.
#[cfg(unix)]
pub fn deny_read_with_probe(
    path: &Path,
    probe: impl FnOnce() -> io::Result<()>,
) -> io::Result<Setup<PermissionGuard>> {
    let guard = PermissionGuard::restrict(path, 0o000)?;
    finish_probe(guard, probe(), "Opening the unreadable layer manifest")
}

#[cfg(unix)]
pub fn deny_create(directory: &Path) -> io::Result<Setup<PermissionGuard>> {
    let mut guard = PermissionGuard::restrict(directory, 0o555)?;
    let probe = match tempfile::Builder::new()
        .prefix(".permission-probe-")
        .tempfile_in(directory)
    {
        Ok(file) => {
            // Cleanup errors are setup errors, never an unsupported capability.
            let probe_path = file.path().to_owned();
            if let Err(error) = file.close() {
                guard.restore()?;
                return Err(io::Error::new(
                    error.kind(),
                    format!(
                        "could not remove permission probe '{}': {error}",
                        probe_path.display()
                    ),
                ));
            }
            Ok(())
        }
        Err(error) => Err(error),
    };
    finish_probe(
        guard,
        probe,
        "Creating a file in the non-writable directory",
    )
}
