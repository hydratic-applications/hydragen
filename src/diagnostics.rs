use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use hydragen::diagnostic_text::display;
use hydragen::{CreatedLayerManifest, LayerManifestDiscovery, LayerManifestFormat};

use crate::presentation::{Presentation, Severity};

mod add_verification;
mod content;
mod io_context;
mod operations;

pub(crate) use io_context::{CliIoError, CompletionChannel};

#[derive(Debug)]
pub enum AuthorInputError {
    Invalid {
        flag: &'static str,
        source: hydragen::layer_manifest::schema::Error,
    },
    Conflict {
        flag: &'static str,
        field: &'static str,
    },
}

impl fmt::Display for AuthorInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&operations::author_input(self).body())
    }
}

impl std::error::Error for AuthorInputError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Invalid { source, .. } => Some(source),
            Self::Conflict { .. } => None,
        }
    }
}

/// The state of the layer manifest when completion reporting starts, not an
/// inference from the error text or a fresh filesystem observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddCompletionState {
    Updated,
    Unchanged,
}

#[derive(Debug)]
pub enum AddCommandError {
    CurrentDirectory(io::Error),
    Verification {
        failure: hydragen::dependencies::AddFailure,
        invocation: PathBuf,
    },
    Edit(hydragen::layer_manifest::edit::EditError),
    Replacement(hydragen::replacement::ReplaceLayerManifestError),
    Completion {
        state: AddCompletionState,
        layer: String,
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for AddCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&operations::add(self).body())
    }
}

impl std::error::Error for AddCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::CurrentDirectory(source) | Self::Completion { source, .. } => source,
            Self::Verification { failure, .. } => failure,
            Self::Edit(source) => source,
            Self::Replacement(source) => source,
        })
    }
}

pub fn error_message(error: &(dyn std::error::Error + 'static)) -> String {
    operations::diagnostic(error).body()
}

/// Shared by init and future layer manifest readers, before refusing or loading a candidate.
fn duplicate_warning(discovery: &LayerManifestDiscovery) -> Option<String> {
    if !discovery.has_duplicates() {
        return None;
    }
    let names = discovery
        .candidates
        .iter()
        .map(|candidate| candidate.format.filename())
        .collect::<Vec<_>>()
        .join(", ");
    let selected = discovery.selected()?.format.filename();
    let priority = LayerManifestFormat::ALL
        .map(LayerManifestFormat::filename)
        .join(" > ");
    let mut model = crate::diagnostic_body::Diagnostic::new(format!(
        "Multiple layer manifests found in '{}': {names}.",
        display(discovery.directory.display())
    ));
    model
        .details
        .push(format!("{selected} takes priority (priority: {priority})."));
    model.correction =
        Some("Layer manifests are not merged; keep only the intended layer manifest.".into());
    Some(model.body())
}

pub fn warn_duplicates(discovery: &LayerManifestDiscovery, presentation: Presentation) {
    if let Some(warning) = duplicate_warning(discovery) {
        // Diagnostic delivery must not turn a completed write into a failure.
        report_warning(&warning, presentation);
    }
}

fn observation_warning(created: &CreatedLayerManifest) -> Option<String> {
    match &created.observation {
        Ok(discovery) => duplicate_warning(discovery),
        Err(error) => {
            let mut model = crate::diagnostic_body::Diagnostic::new(format!(
                "Layer manifest '{}' was created, but duplicate checking failed.",
                display(created.path.display())
            ));
            let cause = operations::load(error);
            model.details.push(cause.summary);
            model.details.extend(cause.details);
            model.correction = if matches!(
                error,
                hydragen::LayerManifestError::Io { .. }
                    | hydragen::LayerManifestError::Metadata {
                        source: hydragen::MetadataError::Schema(
                            hydragen::layer_manifest::schema::Error::Setup { .. }
                        ),
                        ..
                    }
            ) {
                cause.correction
            } else {
                Some("Inspect the reported paths and resolve the duplicate-checking failure; the layer manifest was already created.".into())
            };
            Some(model.body())
        }
    }
}

pub fn report_observation(created: &CreatedLayerManifest, presentation: Presentation) {
    if let Some(warning) = observation_warning(created) {
        report_warning(&warning, presentation);
    }
}

fn report_warning(warning: &str, presentation: Presentation) {
    let _ = writeln!(
        io::stderr().lock(),
        "{}",
        presentation.diagnostic(Severity::Warning, warning)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragen::{InitCreationError, InitError};
    use std::fs;

    fn verification(failure: hydragen::dependencies::AddFailure) -> AddCommandError {
        AddCommandError::Verification {
            failure,
            invocation: "/work/app".into(),
        }
    }

    #[test]
    fn supplied_missing_layer_manifest_uses_the_approved_direct_body() {
        use hydragen::dependencies::{
            AddFailure, DependencyError, DependencyFailure, SourceOrigin,
        };
        let failure = AddFailure {
            directory: Some("/work/app".into()),
            discoveries: vec![],
            errors: vec![DependencyFailure {
                origin: SourceOrigin::Argument {
                    index: 0,
                    reference: "../layer-b".into(),
                },
                error: DependencyError::LayerManifest(std::sync::Arc::new(
                    hydragen::LayerManifestError::Missing {
                        directory: "/work/layer-b".into(),
                    },
                )),
            }],
        };
        assert_eq!(
            verification(failure).to_string(),
            concat!(
                "No layer manifest found in '../layer-b'.\n\n",
                "Tip: Choose an existing layer directory, or run 'hydragen init'\n",
                "in '../layer-b' to initialise it as a layer.\n\n",
                "No files were changed."
            )
        );
    }

    #[test]
    fn missing_current_layer_is_a_direct_prerequisite_without_path_or_batch_wrapper() {
        use hydragen::dependencies::{
            AddFailure, DependencyError, DependencyFailure, SourceOrigin,
        };
        let invocation = PathBuf::from("/work/private\u{1b}[2J\n/project");
        let failure = AddFailure {
            directory: None,
            discoveries: Vec::new(),
            errors: vec![DependencyFailure {
                origin: SourceOrigin::Current,
                error: DependencyError::MissingCurrent {
                    invocation: invocation.clone(),
                },
            }],
        };
        let expected = concat!(
            "No layer manifest found in this directory or its parents.\n\n",
            "Tip: Run this command from an existing layer directory, or run\n",
            "'hydragen init' in the directory you want to use as a layer.\n\n",
            "No files were changed."
        );
        assert_eq!(verification(failure).to_string(), expected);

        // Do not discard path/source details from other origins or a batch.
        for (origin, extra_failure) in [
            (
                SourceOrigin::Argument {
                    index: 0,
                    reference: "../source".into(),
                },
                false,
            ),
            (SourceOrigin::Current, true),
        ] {
            let mut errors = vec![DependencyFailure {
                origin,
                error: DependencyError::MissingCurrent {
                    invocation: invocation.clone(),
                },
            }];
            if extra_failure {
                errors.push(DependencyFailure {
                    origin: SourceOrigin::Argument {
                        index: 1,
                        reference: "../missing".into(),
                    },
                    error: DependencyError::NotDirectory {
                        path: "/work/missing".into(),
                    },
                });
            }
            let message = verification(AddFailure {
                directory: None,
                discoveries: Vec::new(),
                errors,
            })
            .to_string();
            assert!(message.starts_with(if extra_failure {
                "Dependency checks failed"
            } else {
                "No layer manifest found"
            }));
            assert!(message.contains(&display(invocation.display()).to_string()));
            if extra_failure {
                assert!(message.contains("Source '../missing'"));
            }
            assert!(!message.contains('\u{1b}'));
            assert_eq!(message.matches("No files were changed.").count(), 1);
        }
    }

    #[test]
    fn completion_and_current_directory_errors_escape_data_but_preserve_state_and_style() {
        let raw = "bad\u{1b}[2J\r\n\t\u{9b}\u{202e}";
        let escaped = r"bad\x1b[2J\r\n\t\u{9b}\u{202e}";
        for state in [AddCompletionState::Updated, AddCompletionState::Unchanged] {
            let error = AddCommandError::Completion {
                state,
                layer: "app".into(),
                path: std::path::Path::new(raw).join("layer.json"),
                source: io::Error::other(raw.to_owned()),
            };
            let message = error.to_string();
            assert!(
                message.contains(&format!("\nCompletion output failed: {escaped}\n")),
                "{message:?}"
            );
            assert!(
                !message.chars().any(|ch| ch.is_control() && ch != '\n'),
                "{message:?}"
            );
            assert!(!message.contains('\u{202e}'));
            assert!(!message.contains("No files were changed"));
            match state {
                AddCompletionState::Updated => {
                    assert!(message.contains(&format!(
                        "{escaped}{}layer.json' was already updated",
                        std::path::MAIN_SEPARATOR
                    )))
                }
                AddCompletionState::Unchanged => assert!(message.contains("was not updated")),
            }
            let presentation = Presentation {
                decorated: true,
                basic: false,
            };
            let styled = presentation.diagnostic(Severity::Error, &message);
            assert!(styled.contains("\u{1b}[31m"), "{styled:?}");
            assert!(!styled.contains("\u{1b}[2J"), "{styled:?}");
            let plain = console::strip_ansi_codes(&styled);
            assert!(plain.starts_with("✕ Error: Could not report"));
            assert!(
                plain.contains(&format!("\n  Completion output failed: {escaped}\n")),
                "{plain:?}"
            );
        }
        let error = AddCommandError::CurrentDirectory(io::Error::other(raw.to_owned()));
        let message = error.to_string();
        assert!(
            message.contains(&format!("Cause: {escaped}\n")),
            "{message:?}"
        );
    }

    #[test]
    fn observation_warning_escapes_raw_data_without_escaping_structured_layout() {
        let raw = "bad\u{1b}[2J\n\t";
        let escaped = r"bad\x1b[2J\n\t";
        let path = std::path::Path::new(raw).join("layer.json");
        let created = CreatedLayerManifest {
            path: path.clone(),
            observation: Err(hydragen::LayerManifestError::Io {
                action: "inspect layer manifest path",
                path,
                source: io::Error::other(raw.to_owned()),
            }),
        };
        let warning = observation_warning(&created).unwrap();
        assert_eq!(warning.matches(escaped).count(), 3, "{warning:?}");
        assert!(warning.contains("\n\n"), "{warning:?}");
        assert!(warning.contains("Tip:"), "{warning:?}");
        assert!(!warning.contains('\u{1b}'), "{warning:?}");
        assert!(!warning.contains("No files were changed"));
    }

    #[test]
    fn replacement_failure_preserves_primary_error_without_unconditional_no_files_claim() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing-layer.yaml");
        let error =
            hydragen::replacement::replace_layer_manifest(&path, b"name: app\n").unwrap_err();
        let cause = error.io_error().to_string();
        let message = AddCommandError::Replacement(error).to_string();
        assert!(message.starts_with("Could not inspect the selected layer manifest at '"));
        assert!(message.contains(&cause));
        assert!(message.contains("was not replaced."));
        assert!(message.contains(&path.display().to_string()));
        assert!(!message.contains("No files were changed"));
        assert!(!message.contains("already updated"));
    }

    #[test]
    fn current_directory_collisions_name_the_file_for_preflight_and_creation() {
        for format in LayerManifestFormat::ALL {
            let path = std::path::Path::new("/a/long/project/directory").join(format.filename());
            let error = InitError::ExistingLayerManifest { path: path.clone() };
            let message = error_message(&error);
            assert_eq!(
                message,
                format!(
                    "This directory already has a layer manifest ({}).\n\nInitialisation creates a new layer manifest; it won’t overwrite an existing one.",
                    format.filename()
                )
            );
            assert!(!message.contains(&path.display().to_string()));
            assert!(error.to_string().contains(&path.display().to_string()));
            let creation_error = InitCreationError::from(error);
            assert_eq!(error_message(&creation_error), message);
        }
    }

    #[test]
    fn other_init_errors_retain_their_full_diagnostic_paths() {
        let path = std::path::Path::new("/another/project/directory");
        for error in [
            InitError::NestedRepository {
                root: path.to_owned(),
            },
            InitError::Io {
                action: "inspect layer manifest path",
                path: path.join("layer.yaml"),
                source: io::ErrorKind::PermissionDenied.into(),
            },
        ] {
            let message = error_message(&error);
            assert!(message.contains(&path.display().to_string()));
            assert!(message.contains("Tip:"));
            assert_eq!(message.matches("No files were changed.").count(), 1);
        }
    }

    #[test]
    fn warning_survives_load_failure_and_precedes_the_error() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("layer.json"), "{}").unwrap();
        fs::write(directory.path().join("layer.yaml"), "[").unwrap();
        let discovery = hydragen::discover(directory.path()).unwrap();
        let warning = duplicate_warning(&discovery).unwrap();
        let error = discovery.load().unwrap_err();
        let output = format!(
            "{}\nError: {error}\n",
            Presentation {
                decorated: false,
                basic: false
            }
            .diagnostic(Severity::Warning, &warning)
        );
        assert!(output.starts_with("Warning: Multiple layer manifests"));
        assert!(output.contains("layer.yaml, layer.json."));
        assert!(output.contains("layer.yaml takes priority"));
        assert!(
            output
                .contains("Layer manifests are not merged; keep only the intended layer manifest.")
        );
        assert_eq!(output.matches("Warning:").count(), 1);
        assert!(!output.contains("Using"));
        assert!(!output.contains('\x1b'));
    }

    #[test]
    fn no_duplicates_means_no_warning() {
        let directory = tempfile::tempdir().unwrap();
        assert!(duplicate_warning(&hydragen::discover(directory.path()).unwrap()).is_none());
        fs::create_dir(directory.path().join("layer.yaml")).unwrap();
        assert!(duplicate_warning(&hydragen::discover(directory.path()).unwrap()).is_none());
    }

    #[test]
    fn failed_observation_names_the_already_created_path() {
        let path = std::path::PathBuf::from("/project/layer.jsonc");
        let created = CreatedLayerManifest {
            path: path.clone(),
            observation: Err(hydragen::LayerManifestError::Io {
                action: "inspect layer manifest path",
                path,
                source: io::ErrorKind::PermissionDenied.into(),
            }),
        };
        let warning = observation_warning(&created).unwrap();
        assert!(warning.contains("Layer manifest '/project/layer.jsonc' was created"));
        assert!(warning.contains("duplicate checking failed"));
        assert!(!warning.contains("No files were changed"));
    }

    #[test]
    fn unexpected_observation_causes_remain_safe_and_do_not_recommend_reinitialising_a_completed_write()
     {
        let created = CreatedLayerManifest {
            path: "/project/layer.yaml".into(),
            observation: Err(hydragen::LayerManifestError::Missing {
                directory: "/project".into(),
            }),
        };
        let warning = observation_warning(&created).unwrap();
        assert!(warning.contains("was created, but duplicate checking failed."));
        assert!(warning.contains("No layer manifest found in '/project'."));
        assert!(warning.contains("Inspect the reported paths"));
        assert!(!warning.contains("hydragen init"));
        assert!(!warning.contains("No files were changed"));
        for decorated in [false, true] {
            let output = Presentation {
                decorated,
                basic: false,
            }
            .diagnostic(Severity::Warning, &warning);
            assert_eq!(output.matches("Warning:").count(), 1);
            assert!(output.lines().skip(1).all(|line| !line.contains('\x1b')));
        }
    }

    #[test]
    fn successful_observation_warns_about_priority_not_the_created_format() {
        for created_format in [LayerManifestFormat::Yaml, LayerManifestFormat::Json] {
            let directory = tempfile::tempdir().unwrap();
            let prepared = hydragen::inspect_init(directory.path())
                .unwrap()
                .prepare()
                .unwrap();
            let appeared = if created_format == LayerManifestFormat::Yaml {
                LayerManifestFormat::Json
            } else {
                LayerManifestFormat::Yaml
            };
            fs::write(directory.path().join(appeared.filename()), "untouched").unwrap();
            let created = hydragen::create_init(
                prepared,
                &hydragen::Metadata {
                    name: "example".into(),
                    author: None,
                    description: None,
                },
                created_format,
            )
            .unwrap();
            let warning = observation_warning(&created).unwrap();
            assert!(warning.contains("layer.yaml, layer.json."));
            assert!(warning.contains("layer.yaml takes priority"));
            assert_eq!(warning.matches("Multiple layer manifests found").count(), 1);
            assert!(!warning.contains("another process"));
            assert_eq!(created.path.file_name().unwrap(), created_format.filename());
            assert_eq!(
                fs::read_to_string(directory.path().join(appeared.filename())).unwrap(),
                "untouched"
            );
        }
    }
}
