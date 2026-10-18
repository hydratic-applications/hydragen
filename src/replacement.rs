//! Replace an existing layer manifest using prepared bytes and a hidden sibling.
//!
//! This module does not parse content or detect stale edits. Concurrent writers can
//! overwrite one another. Abrupt termination can leave a staging file; syncing the
//! file and renaming it does not promise unconditional power-loss durability or
//! preservation of ownership, ACLs, or extended attributes.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tempfile::{Builder, TempPath};

use crate::diagnostic_text::display;

/// The primary operation that failed before replacement completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplaceLayerManifestOperation {
    InspectTarget,
    CreateStaging,
    WriteStaging,
    PreservePermissions,
    SyncStaging,
    Rename,
}

impl fmt::Display for ReplaceLayerManifestOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InspectTarget => "inspect the selected layer manifest",
            Self::CreateStaging => "create a sibling staging file",
            Self::WriteStaging => "write the staged layer manifest",
            Self::PreservePermissions => "preserve layer manifest permissions on the staging file",
            Self::SyncStaging => "synchronise the staged layer manifest",
            Self::Rename => "replace the selected layer manifest",
        })
    }
}

/// Replacement did not complete. The primary cause is never replaced by a cleanup
/// error. This operation has not replaced the selected layer manifest, but a failed
/// cleanup may have left a staging file; callers must not claim no files changed.
/// This is not a guarantee against changes made by another process.
#[derive(Debug)]
pub struct ReplaceLayerManifestError {
    layer_manifest_path: PathBuf,
    operation: ReplaceLayerManifestOperation,
    source: io::Error,
    staging_path: Option<PathBuf>,
    cleanup_error: Option<io::Error>,
}

impl ReplaceLayerManifestError {
    pub fn operation(&self) -> ReplaceLayerManifestOperation {
        self.operation
    }

    /// The selected destination, in the spelling supplied by the caller.
    pub fn layer_manifest_path(&self) -> &Path {
        &self.layer_manifest_path
    }

    /// Path associated with the primary operation: the sibling directory for
    /// creation, the staging file for preparation, or the selected destination.
    pub fn path(&self) -> &Path {
        match self.operation {
            ReplaceLayerManifestOperation::CreateStaging => {
                sibling_directory(&self.layer_manifest_path)
            }
            ReplaceLayerManifestOperation::WriteStaging
            | ReplaceLayerManifestOperation::PreservePermissions
            | ReplaceLayerManifestOperation::SyncStaging => {
                self.staging_path().unwrap_or(&self.layer_manifest_path)
            }
            _ => &self.layer_manifest_path,
        }
    }

    pub fn io_error(&self) -> &io::Error {
        &self.source
    }

    pub fn cleanup_failed(&self) -> bool {
        self.cleanup_error.is_some()
    }

    pub fn cleanup_error(&self) -> Option<&io::Error> {
        self.cleanup_error.as_ref()
    }

    /// Present if staging was created, even when it was subsequently removed.
    /// Use `cleanup_failed` to distinguish a failed removal from normal cleanup.
    pub fn staging_path(&self) -> Option<&Path> {
        self.staging_path.as_deref()
    }
}

impl fmt::Display for ReplaceLayerManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Could not {} at '{}': {}. The selected layer manifest '{}' was not replaced.",
            self.operation,
            display(self.path().display()),
            display(&self.source),
            display(self.layer_manifest_path.display()),
        )?;
        if let (Some(path), Some(error)) = (self.staging_path(), self.cleanup_error()) {
            write!(
                formatter,
                " Could not remove staging file '{}': {}. A staging file may remain; inspect this path before removing it.",
                display(path.display()),
                display(error),
            )?;
        }
        Ok(())
    }
}

impl Error for ReplaceLayerManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Write already-prepared bytes beside an existing regular layer manifest, copy
/// its permissions, sync the staged file, close its handle, then use native rename.
/// Known symlinks and non-regular targets are refused before creating a sibling.
/// There is no in-place write, copy fallback, lock, content check, or merge.
///
/// Every `Err` is before successful rename. After rename succeeds, only infallible
/// cleanup disarming remains. A caller reporting completion must retain this
/// successful-update state if its own output subsequently fails. Callers must skip
/// this operation entirely for no-op plans.
pub fn replace_layer_manifest(
    path: &Path,
    contents: &[u8],
) -> Result<(), ReplaceLayerManifestError> {
    replace_with(path, contents, |_, _| Ok(()), TempPath::close)
}

fn sibling_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

// Small private checkpoints let tests fail real staging operations without a
// public filesystem abstraction. No checkpoint or fallible callback runs after
// successful rename. The cleanup callback consumes the closed owned path.
fn replace_with(
    path: &Path,
    contents: &[u8],
    mut before: impl FnMut(ReplaceLayerManifestOperation, &Path) -> io::Result<()>,
    cleanup: impl FnOnce(TempPath) -> io::Result<()>,
) -> Result<(), ReplaceLayerManifestError> {
    use ReplaceLayerManifestOperation as Operation;

    let without_staging = |operation, source| ReplaceLayerManifestError {
        layer_manifest_path: path.to_owned(),
        operation,
        source,
        staging_path: None,
        cleanup_error: None,
    };
    let metadata = before(Operation::InspectTarget, path)
        .and_then(|()| fs::symlink_metadata(path))
        .map_err(|error| without_staging(Operation::InspectTarget, error))?;
    if !metadata.file_type().is_file() {
        let reason = if metadata.file_type().is_symlink() {
            "the selected layer manifest is a symlink; editing through a symlink is not supported"
        } else {
            "the selected layer manifest must be an existing regular file"
        };
        return Err(without_staging(
            Operation::InspectTarget,
            io::Error::new(io::ErrorKind::InvalidInput, reason),
        ));
    }

    let directory = sibling_directory(path);
    let mut staged = before(Operation::CreateStaging, directory)
        .and_then(|()| {
            Builder::new()
                .prefix(".hydragen-layer-manifest-")
                .tempfile_in(directory)
        })
        .map_err(|error| without_staging(Operation::CreateStaging, error))?;
    let staging_path = staged.path().to_owned();
    let permissions = metadata.permissions();

    let prepared = (|| {
        attempt(Operation::WriteStaging, &staging_path, &mut before, || {
            staged.write_all(contents)
        })?;
        attempt(
            Operation::PreservePermissions,
            &staging_path,
            &mut before,
            || staged.as_file().set_permissions(permissions.clone()),
        )?;
        attempt(Operation::SyncStaging, &staging_path, &mut before, || {
            staged.as_file().sync_all()
        })
    })();

    // Conversion drops the file handle before either explicit cleanup or rename,
    // including after a partial write or permission/sync failure.
    let mut staged = staged.into_temp_path();
    let result = prepared.and_then(|()| {
        attempt(Operation::Rename, path, &mut before, || {
            fs::rename(&staged, path)
        })
    });
    if let Err((operation, source)) = result {
        // Explicit close reports removal failures (including read-only staging
        // files on Windows). Do not relax permissions or hide a cleanup error.
        let cleanup_error = cleanup(staged).err();
        return Err(ReplaceLayerManifestError {
            layer_manifest_path: path.to_owned(),
            operation,
            source,
            staging_path: Some(staging_path),
            cleanup_error,
        });
    }

    // The old staging pathname no longer belongs to us after rename. Infallible
    // disarm avoids deleting a newly created file there and cannot turn a completed
    // replacement into a reported storage failure.
    staged.disable_cleanup(true);
    Ok(())
}

fn attempt(
    operation: ReplaceLayerManifestOperation,
    path: &Path,
    before: &mut impl FnMut(ReplaceLayerManifestOperation, &Path) -> io::Result<()>,
    action: impl FnOnce() -> io::Result<()>,
) -> Result<(), (ReplaceLayerManifestOperation, io::Error)> {
    before(operation, path)
        .and_then(|()| action())
        .map_err(|error| (operation, error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ReplaceLayerManifestOperation as Operation;
    use std::cell::Cell;

    const ORIGINAL: &[u8] = b"name: original\n# retain on failure\n";
    const REPLACEMENT: &[u8] = b"name: original\ndependencies: [../shared]\n";

    #[test]
    fn every_operation_and_cleanup_display_escapes_raw_paths_and_io_causes() {
        let raw = "bad\u{1b}[2J\r\n\t\u{7f}\u{9b}\u{202e}";
        let escaped = r"bad\x1b[2J\r\n\t\x7f\u{9b}\u{202e}";
        for operation in [
            Operation::InspectTarget,
            Operation::CreateStaging,
            Operation::WriteStaging,
            Operation::PreservePermissions,
            Operation::SyncStaging,
            Operation::Rename,
        ] {
            for cleanup_failed in [false, true] {
                let error = ReplaceLayerManifestError {
                    layer_manifest_path: Path::new(raw).join("layer.json"),
                    operation,
                    source: io::Error::other(raw.to_owned()),
                    staging_path: Some(Path::new(raw).join(".staging")),
                    cleanup_error: cleanup_failed.then(|| io::Error::other(raw.to_owned())),
                };
                let message = error.to_string();
                assert!(!message.chars().any(char::is_control), "{message:?}");
                assert!(!message.contains('\u{202e}'), "{message:?}");
                assert!(message.contains(escaped), "{message:?}");
                assert!(!message.contains("\\\\x1b"), "{message:?}");
                assert!(message.contains("was not replaced."));
                assert_eq!(
                    message.contains("A staging file may remain"),
                    cleanup_failed
                );
                assert_eq!(error.cleanup_failed(), cleanup_failed);
                assert_eq!(error.io_error().to_string(), raw);
                if cleanup_failed {
                    assert!(
                        message
                            .contains(&format!("{escaped}{}.staging", std::path::MAIN_SEPARATOR)),
                        "{message:?}"
                    );
                }
            }
        }
    }

    fn fixture() -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("layer.yaml");
        fs::write(&path, ORIGINAL).unwrap();
        (directory, path)
    }

    fn entries(directory: &Path) -> Vec<PathBuf> {
        let mut paths: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        paths.sort();
        paths
    }

    fn injected(operation: Operation) -> io::Error {
        io::Error::other(format!("injected {operation:?} failure"))
    }

    #[test]
    fn success_replaces_complete_bytes_without_leaving_a_sibling() {
        let (directory, path) = fixture();
        replace_layer_manifest(&path, REPLACEMENT).unwrap();
        assert_eq!(fs::read(&path).unwrap(), REPLACEMENT);
        assert_eq!(entries(directory.path()), [path]);
    }

    #[test]
    fn only_prepared_bytes_are_used_including_empty_or_non_utf8_content() {
        for bytes in [b"".as_slice(), b"\xff\0not parsed\n"] {
            let (directory, path) = fixture();
            replace_layer_manifest(&path, bytes).unwrap();
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert_eq!(entries(directory.path()), [path]);
        }
    }

    #[test]
    fn staging_is_hidden_exclusive_and_beside_the_selected_target() {
        let (directory, path) = fixture();
        let untouched = directory.path().join(".hydragen-layer-manifest-existing");
        fs::write(&untouched, b"not ours").unwrap();
        let mut seen = None;
        replace_with(
            &path,
            REPLACEMENT,
            |operation, operation_path| {
                if operation == Operation::WriteStaging {
                    assert_eq!(operation_path.parent(), Some(directory.path()));
                    assert!(
                        operation_path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with(".hydragen-layer-manifest-")
                    );
                    assert_ne!(operation_path, untouched);
                    assert!(operation_path.is_file());
                    assert_eq!(fs::metadata(operation_path).unwrap().len(), 0);
                    assert_eq!(
                        fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(operation_path)
                            .unwrap_err()
                            .kind(),
                        io::ErrorKind::AlreadyExists
                    );
                    seen = Some(operation_path.to_owned());
                }
                Ok(())
            },
            |_| panic!("cleanup must not run after successful rename"),
        )
        .unwrap();
        assert!(!seen.unwrap().exists());
        assert_eq!(fs::read(&untouched).unwrap(), b"not ours");
        assert_eq!(entries(directory.path()).len(), 2);
    }

    #[test]
    fn every_precommit_checkpoint_keeps_original_and_explicitly_cleans_staging() {
        for failure in [
            Operation::InspectTarget,
            Operation::CreateStaging,
            Operation::WriteStaging,
            Operation::PreservePermissions,
            Operation::SyncStaging,
            Operation::Rename,
        ] {
            let (directory, path) = fixture();
            let cleanup_called = Cell::new(false);
            let error = replace_with(
                &path,
                REPLACEMENT,
                |operation, _| {
                    if operation == failure {
                        Err(injected(operation))
                    } else {
                        Ok(())
                    }
                },
                |staged| {
                    cleanup_called.set(true);
                    staged.close()
                },
            )
            .unwrap_err();
            assert_eq!(error.operation(), failure);
            assert_eq!(error.layer_manifest_path(), path);
            assert_eq!(error.io_error().to_string(), injected(failure).to_string());
            assert_eq!(
                error.source().unwrap().to_string(),
                injected(failure).to_string()
            );
            assert!(!error.cleanup_failed());
            assert!(error.cleanup_error().is_none());
            let created = !matches!(failure, Operation::InspectTarget | Operation::CreateStaging);
            assert_eq!(cleanup_called.get(), created);
            assert_eq!(error.staging_path().is_some(), created);
            if let Some(staged) = error.staging_path() {
                assert!(!staged.exists());
            }
            assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
            assert_eq!(entries(directory.path()), [path]);
        }
    }

    #[test]
    fn partial_staging_write_failure_does_not_truncate_original() {
        let (directory, path) = fixture();
        let error = replace_with(
            &path,
            REPLACEMENT,
            |operation, staged| {
                if operation == Operation::WriteStaging {
                    fs::write(staged, b"partial candidate")?;
                    return Err(io::Error::new(io::ErrorKind::StorageFull, "disk full"));
                }
                Ok(())
            },
            |staged| {
                assert_eq!(fs::read(&staged)?, b"partial candidate");
                staged.close()
            },
        )
        .unwrap_err();
        assert_eq!(error.operation(), Operation::WriteStaging);
        assert_eq!(error.io_error().kind(), io::ErrorKind::StorageFull);
        assert!(!error.cleanup_failed());
        assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
        assert_eq!(entries(directory.path()), [path]);
    }

    #[test]
    fn cleanup_failure_retains_primary_cause_path_and_leftover() {
        for failure in [
            Operation::WriteStaging,
            Operation::PreservePermissions,
            Operation::SyncStaging,
            Operation::Rename,
        ] {
            let (directory, path) = fixture();
            let error = replace_with(
                &path,
                REPLACEMENT,
                |operation, _| {
                    if operation == failure {
                        Err(injected(operation))
                    } else {
                        Ok(())
                    }
                },
                |mut staged| {
                    // Model a refused removal, without Drop silently retrying it.
                    staged.disable_cleanup(true);
                    Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "injected cleanup refusal",
                    ))
                },
            )
            .unwrap_err();
            assert_eq!(error.operation(), failure);
            assert_eq!(error.io_error().to_string(), injected(failure).to_string());
            assert_eq!(
                error.source().unwrap().to_string(),
                injected(failure).to_string()
            );
            assert!(error.cleanup_failed());
            assert_eq!(
                error.cleanup_error().unwrap().kind(),
                io::ErrorKind::PermissionDenied
            );
            let staged = error.staging_path().unwrap();
            assert!(staged.is_file());
            assert_eq!(staged.parent(), Some(directory.path()));
            assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
            assert_eq!(entries(directory.path()).len(), 2);
            let diagnostic = error.to_string();
            assert!(diagnostic.contains(&injected(failure).to_string()));
            assert!(diagnostic.contains("injected cleanup refusal"));
            assert!(diagnostic.contains(&staged.display().to_string()));
            assert!(diagnostic.contains("was not replaced"));
            assert!(!diagnostic.contains("no files changed"));
            fs::remove_file(staged).unwrap();
        }
    }

    #[test]
    fn actual_cleanup_failure_is_not_suppressed_by_drop() {
        let (directory, path) = fixture();
        let error = replace_with(
            &path,
            REPLACEMENT,
            |operation, _| {
                if operation == Operation::Rename {
                    Err(injected(operation))
                } else {
                    Ok(())
                }
            },
            |staged| {
                // A directory cannot be removed by TempPath::close's remove_file.
                fs::remove_file(&staged)?;
                fs::create_dir(&staged)?;
                staged.close()
            },
        )
        .unwrap_err();
        assert_eq!(error.operation(), Operation::Rename);
        assert!(error.cleanup_failed());
        assert!(error.cleanup_error().is_some());
        assert!(error.staging_path().unwrap().is_dir());
        assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
        fs::remove_dir(error.staging_path().unwrap()).unwrap();
        assert_eq!(entries(directory.path()), [path]);
    }

    #[test]
    fn preparation_order_and_commit_boundary_are_explicit() {
        let (directory, path) = fixture();
        let mut operations = Vec::new();
        let mut staging_path = None;
        replace_with(
            &path,
            REPLACEMENT,
            |operation, operation_path| {
                operations.push(operation);
                assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
                if operation == Operation::WriteStaging {
                    staging_path = Some(operation_path.to_owned());
                }
                if matches!(operation, Operation::SyncStaging | Operation::Rename) {
                    assert_eq!(fs::read(staging_path.as_ref().unwrap())?, REPLACEMENT);
                }
                Ok(())
            },
            |_| panic!("successful rename must not invoke fallible cleanup"),
        )
        .unwrap();
        assert_eq!(
            operations,
            [
                Operation::InspectTarget,
                Operation::CreateStaging,
                Operation::WriteStaging,
                Operation::PreservePermissions,
                Operation::SyncStaging,
                Operation::Rename,
            ]
        );
        assert_eq!(fs::read(&path).unwrap(), REPLACEMENT);
        assert_eq!(entries(directory.path()), [path]);
    }

    #[test]
    fn missing_and_non_regular_targets_are_refused_without_staging() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.yaml");
        let error = replace_layer_manifest(&missing, REPLACEMENT).unwrap_err();
        assert_eq!(error.operation(), Operation::InspectTarget);
        assert_eq!(error.io_error().kind(), io::ErrorKind::NotFound);
        assert_eq!(error.path(), missing);
        assert!(error.staging_path().is_none());
        assert!(!error.cleanup_failed());
        assert!(entries(directory.path()).is_empty());

        let path = directory.path().join("layer.yaml");
        fs::create_dir(&path).unwrap();
        let error = replace_layer_manifest(&path, REPLACEMENT).unwrap_err();
        assert_eq!(error.operation(), Operation::InspectTarget);
        assert_eq!(error.io_error().kind(), io::ErrorKind::InvalidInput);
        assert!(error.staging_path().is_none());
        assert!(path.is_dir());
        assert_eq!(entries(directory.path()), [path]);
    }

    #[test]
    fn error_paths_identify_the_failed_operation() {
        for failure in [
            Operation::CreateStaging,
            Operation::WriteStaging,
            Operation::PreservePermissions,
            Operation::SyncStaging,
            Operation::Rename,
        ] {
            let (directory, path) = fixture();
            let error = replace_with(
                &path,
                REPLACEMENT,
                |operation, _| {
                    if operation == failure {
                        Err(injected(operation))
                    } else {
                        Ok(())
                    }
                },
                TempPath::close,
            )
            .unwrap_err();
            let expected = match failure {
                Operation::CreateStaging => directory.path(),
                Operation::Rename => path.as_path(),
                _ => error.staging_path().unwrap(),
            };
            assert_eq!(error.path(), expected);
            assert!(error.to_string().contains(&expected.display().to_string()));
        }
    }

    #[test]
    fn a_bare_filename_uses_current_directory_as_sibling_base() {
        // Do not change the process-wide cwd in a parallel unit test.
        assert_eq!(sibling_directory(Path::new("layer.yaml")), Path::new("."));
        assert_eq!(
            sibling_directory(Path::new("nested/layer.yaml")),
            Path::new("nested")
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_permissions_including_read_only_survive_replacement() {
        use std::os::unix::fs::PermissionsExt;

        for mode in [0o640, 0o755, 0o444, 0o400] {
            let (directory, path) = fixture();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            replace_layer_manifest(&path, REPLACEMENT).unwrap();
            assert_eq!(fs::read(&path).unwrap(), REPLACEMENT);
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
                mode
            );
            assert_eq!(entries(directory.path()), [path]);
        }
    }

    #[cfg(unix)]
    #[test]
    fn permission_copy_precedes_sync_and_injected_failed_commit_cleans_read_only_stage() {
        use std::os::unix::fs::PermissionsExt;

        let (directory, path) = fixture();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        let mut staged_path = None;
        let error = replace_with(
            &path,
            REPLACEMENT,
            |operation, operation_path| {
                if operation == Operation::SyncStaging {
                    assert_eq!(
                        fs::metadata(operation_path)?.permissions().mode() & 0o7777,
                        0o444
                    );
                    staged_path = Some(operation_path.to_owned());
                }
                if operation == Operation::Rename {
                    return Err(injected(operation));
                }
                Ok(())
            },
            TempPath::close,
        )
        .unwrap_err();
        assert!(!error.cleanup_failed());
        assert!(!staged_path.unwrap().exists());
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
            0o444
        );
        assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
        assert_eq!(entries(directory.path()), [path]);
    }

    #[cfg(unix)]
    #[test]
    fn native_rename_replaces_the_file_not_an_already_open_handle() {
        use std::io::Read;
        use std::os::unix::fs::MetadataExt;

        let (directory, path) = fixture();
        let mut old_file = fs::File::open(&path).unwrap();
        let old_inode = old_file.metadata().unwrap().ino();
        replace_layer_manifest(&path, REPLACEMENT).unwrap();
        let mut old_bytes = Vec::new();
        old_file.read_to_end(&mut old_bytes).unwrap();
        assert_eq!(old_bytes, ORIGINAL);
        assert_ne!(fs::metadata(&path).unwrap().ino(), old_inode);
        assert_eq!(fs::read(&path).unwrap(), REPLACEMENT);
        assert_eq!(entries(directory.path()), [path]);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_and_dangling_symlinks_are_refused_without_following_them() {
        use std::os::unix::fs::symlink;

        for dangling in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let target = directory.path().join("target.yaml");
            if !dangling {
                fs::write(&target, ORIGINAL).unwrap();
            }
            let path = directory.path().join("layer.yaml");
            symlink(&target, &path).unwrap();
            let before = entries(directory.path());
            let error = replace_layer_manifest(&path, REPLACEMENT).unwrap_err();
            assert_eq!(error.operation(), Operation::InspectTarget);
            assert!(error.staging_path().is_none());
            assert!(error.to_string().contains("symlink"));
            assert_eq!(fs::read_link(&path).unwrap(), target);
            if !dangling {
                assert_eq!(fs::read(&target).unwrap(), ORIGINAL);
            }
            assert_eq!(entries(directory.path()), before);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_special_file_is_refused_without_opening_it() {
        use std::os::unix::net::UnixListener;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("layer.yaml");
        let listener = UnixListener::bind(&path).unwrap();
        let error = replace_layer_manifest(&path, REPLACEMENT).unwrap_err();
        assert_eq!(error.operation(), Operation::InspectTarget);
        assert_eq!(error.io_error().kind(), io::ErrorKind::InvalidInput);
        assert!(error.staging_path().is_none());
        assert_eq!(entries(directory.path()), [path]);
        drop(listener);
    }

    #[test]
    fn replacement_does_not_recheck_content_or_merge_a_later_edit() {
        let (directory, path) = fixture();
        replace_with(
            &path,
            REPLACEMENT,
            |operation, _| {
                if operation == Operation::Rename {
                    // Model a later edit in-process, not a concurrency guarantee.
                    fs::write(&path, b"later edit that is not merged\n")?;
                }
                Ok(())
            },
            TempPath::close,
        )
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), REPLACEMENT);
        assert_eq!(entries(directory.path()), [path]);
    }

    #[cfg(windows)]
    #[test]
    fn windows_read_only_target_refusal_retains_permissions_and_cleanup_error() {
        let (directory, path) = fixture();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&path, permissions).unwrap();

        let error = replace_layer_manifest(&path, REPLACEMENT).unwrap_err();
        assert_eq!(error.operation(), Operation::Rename);
        assert!(fs::metadata(&path).unwrap().permissions().readonly());
        assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
        // The copied read-only attribute also prevents remove_file on Windows.
        assert!(error.cleanup_failed());
        assert!(error.cleanup_error().is_some());
        let staged = error.staging_path().unwrap();
        assert!(fs::metadata(staged).unwrap().permissions().readonly());
        assert_eq!(fs::read(staged).unwrap(), REPLACEMENT);
        for file in [path.as_path(), staged] {
            let mut permissions = fs::metadata(file).unwrap().permissions();
            permissions.set_readonly(false);
            fs::set_permissions(file, permissions).unwrap();
        }
        fs::remove_file(staged).unwrap();
        assert_eq!(entries(directory.path()), [path]);
    }

    #[cfg(windows)]
    #[test]
    fn windows_native_rename_refusal_from_open_destination_has_no_copy_fallback() {
        use std::os::windows::fs::OpenOptionsExt;

        let (directory, path) = fixture();
        // FILE_SHARE_READ, without FILE_SHARE_DELETE: native replacement refuses.
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        let error = replace_layer_manifest(&path, REPLACEMENT).unwrap_err();
        assert_eq!(error.operation(), Operation::Rename);
        assert!(!error.cleanup_failed());
        assert!(!error.staging_path().unwrap().exists());
        assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
        assert_eq!(entries(directory.path()), [path]);
        drop(held);
    }
}
