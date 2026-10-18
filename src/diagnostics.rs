use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use hydragen::diagnostic_text::display;
use hydragen::{
    CreatedLayerManifest, InitCreationError, InitError, LayerManifestDiscovery, LayerManifestFormat,
};

use crate::presentation::{Presentation, Severity};

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
    Verification(hydragen::dependencies::AddFailure),
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
        match self {
            Self::CurrentDirectory(source) => write!(
                formatter,
                "Could not add dependencies.\nCould not determine the current directory: {}\nCheck that the working directory still exists and is accessible.",
                display(source)
            ),
            Self::Verification(failure) => {
                let count = failure.errors.len();
                let noun = if count == 1 { "error" } else { "errors" };
                writeln!(formatter, "Could not add dependencies ({count} {noun}).")?;
                if let Some(directory) = &failure.directory {
                    writeln!(
                        formatter,
                        "Local paths resolve from the declaring layer directory: {}",
                        display(directory.display())
                    )?;
                }
                for (index, failure) in failure.errors.iter().enumerate() {
                    writeln!(formatter, "\n{}. {}", index + 1, failure.origin)?;
                    for line in dependency_error_message(&failure.error, &failure.origin).lines() {
                        writeln!(formatter, "   {line}")?;
                    }
                }
                formatter.write_str("\nNo files were changed.")
            }
            Self::Edit(source) => {
                write!(
                    formatter,
                    "Could not add dependencies.\n{source}\nNo files were changed."
                )?;
                if matches!(
                    source,
                    hydragen::layer_manifest::edit::EditError::UnsupportedEdit { .. }
                ) {
                    formatter.write_str(" Adjust the source construct identified above, retaining its intended values and comments.")?;
                } else {
                    formatter.write_str(" The prepared edit failed its safety checks; the original has been retained.")?;
                }
                Ok(())
            }
            Self::Replacement(source) => {
                write!(formatter, "Could not add dependencies.\n{source}")?;
                if source.cleanup_failed() {
                    formatter.write_str(
                        "\nResolve the reported staging-file cleanup failure before retrying.",
                    )?;
                } else {
                    formatter.write_str("\nCheck directory access, file permissions and available space before retrying.")?;
                }
                Ok(())
            }
            Self::Completion {
                state,
                layer,
                path,
                source,
            } => {
                match state {
                    AddCompletionState::Updated => write!(
                        formatter,
                        "Could not report completion for {layer}.\nThe layer manifest '{}' was already updated; do not assume the addition failed.",
                        display(path.display())
                    )?,
                    AddCompletionState::Unchanged => write!(
                        formatter,
                        "Could not report the no-change result for {layer}.\nThe requested dependencies were already present; the layer manifest was not updated."
                    )?,
                }
                write!(
                    formatter,
                    "\nCompletion output failed: {}\nCheck the output destination before running the command again.",
                    display(source)
                )
            }
        }
    }
}

impl std::error::Error for AddCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::CurrentDirectory(source) | Self::Completion { source, .. } => source,
            Self::Verification(source) => source,
            Self::Edit(source) => source,
            Self::Replacement(source) => source,
        })
    }
}

fn dependency_error_message(
    error: &hydragen::dependencies::DependencyError,
    origin: &hydragen::dependencies::SourceOrigin,
) -> String {
    use hydragen::dependencies::{DependencyError, SourceOrigin};
    use hydragen::layer_manifest::{LayerManifestError, MetadataError, schema};
    // This is diagnostic wiring for a known schema rule, not another reference
    // validator. Other rules, instance paths, and origins keep their diagnostic detail.
    if let DependencyError::Schema(schema::Error::Content(violation)) = error
        && violation.schema_path == "/$defs/reference/oneOf"
        && violation.keyword == "oneOf"
        && violation.instance_path.is_empty()
    {
        let reference_kind = match origin {
            SourceOrigin::Argument { .. } => Some("source"),
            SourceOrigin::Existing { .. } => Some("dependency"),
            SourceOrigin::Current => None,
        };
        if let Some(reference_kind) = reference_kind {
            return format!(
                "Invalid {reference_kind} reference.\nUse an explicit local path beginning with ./ or ../, or an absolute path. Bare names are not local paths; published additions are not supported yet."
            );
        }
    }
    if let DependencyError::NameConflict {
        name,
        previous,
        previous_directory,
        directory,
    } = error
    {
        let mut message = format!(
            "Layer name '{}' is already in use.\nConflicting source: {previous}",
            display(name)
        );
        if let Some(path) = previous_directory {
            let label = match previous {
                SourceOrigin::Current => "Current layer path",
                SourceOrigin::Existing { .. } => "Existing source path",
                SourceOrigin::Argument { .. } => "Earlier source path",
            };
            message.push_str(&format!("\n{label}: {}", display(path.display())));
        }
        if let Some(path) = directory {
            let label = match origin {
                SourceOrigin::Existing { .. } => "Declared source path",
                _ => "Requested source path",
            };
            message.push_str(&format!("\n{label}: {}", display(path.display())));
        }
        message.push_str("\nEach direct dependency must have a unique name, different from the current layer.\nChoose a source with a different layer name; existing declarations are not replaced.");
        return message;
    }
    let guidance = match error {
        DependencyError::Io { .. } | DependencyError::NotDirectory { .. } => {
            "Check the source path and access permissions. Each local source must be a directory containing its own valid layer manifest."
        }
        DependencyError::LayerManifest(error)
            if matches!(
                error.as_ref(),
                LayerManifestError::Metadata {
                    source: MetadataError::Schema(schema::Error::Setup { .. }),
                    ..
                }
            ) =>
        {
            ""
        }
        DependencyError::LayerManifest(_) => {
            "Correct the selected layer manifest or supply a directory with its own valid layer manifest. Lower-priority files and parent layers are not fallbacks."
        }
        _ => "",
    };
    if guidance.is_empty() {
        error.to_string()
    } else {
        format!("{error}\n{guidance}")
    }
}

pub fn init_error_message(error: &(dyn std::error::Error + 'static)) -> String {
    let init_error = error.downcast_ref::<InitError>().or_else(|| {
        error
            .downcast_ref::<InitCreationError>()
            .map(|failure| &failure.error)
    });
    if let Some(InitError::ExistingLayerManifest { path }) = init_error
        && let Some(filename) = path.file_name()
    {
        return format!(
            "This directory already has a layer manifest ({}).\nInitialisation creates a new layer manifest; it won’t overwrite an existing one.",
            display(std::path::Path::new(filename).display())
        );
    }
    error.to_string()
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
    Some(format!(
        "Warning: Multiple layer manifests found in '{}': {names}.\n{selected} takes priority (priority: {priority}).\nLayer manifests are not merged; keep only the intended layer manifest.",
        display(discovery.directory.display())
    ))
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
        Err(error) => Some(format!(
            "Warning: Layer manifest '{}' was created, but duplicate checking failed:\n{error}",
            display(created.path.display())
        )),
    }
}

pub fn report_observation(created: &CreatedLayerManifest, presentation: Presentation) {
    if let Some(warning) = observation_warning(created) {
        report_warning(&warning, presentation);
    }
}

fn report_warning(warning: &str, presentation: Presentation) {
    let message = warning.strip_prefix("Warning: ").unwrap_or(warning);
    let _ = writeln!(
        io::stderr().lock(),
        "{}",
        presentation.diagnostic(Severity::Warning, message)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
            message.contains(&format!("current directory: {escaped}\n")),
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
        assert_eq!(warning.matches('\n').count(), 1, "{warning:?}");
        assert!(!warning.contains('\u{1b}'), "{warning:?}");
        assert!(!warning.contains("No files were changed"));
    }

    #[test]
    fn add_setup_failures_do_not_blame_valid_input_or_use_init_flags() {
        use hydragen::dependencies::DependencyError;
        use hydragen::layer_manifest::{LayerManifestError, MetadataError, schema};
        let setup = schema::Error::Setup {
            message: "test setup failure".into(),
            detail: None,
        };
        for error in [
            DependencyError::Schema(setup.clone()),
            DependencyError::LayerManifest(std::sync::Arc::new(LayerManifestError::Metadata {
                path: "/project/layer.yaml".into(),
                source: MetadataError::Schema(setup),
            })),
        ] {
            let message = dependency_error_message(
                &error,
                &hydragen::dependencies::SourceOrigin::Argument {
                    index: 0,
                    reference: "valid@1.2.0".into(),
                },
            );
            assert!(message.contains("Internal layer manifest schema error"));
            assert!(!message.contains("Correct"));
            assert!(!message.contains("Use an explicit"));
            assert!(!message.contains("--name"));
        }
    }

    #[test]
    fn known_reference_oneof_gets_plain_correction_for_reference_origins_only() {
        use hydragen::dependencies::{DependencyError, SourceOrigin};
        let schema_error =
            hydragen::layer_manifest::schema::classify_reference("bare-name").unwrap_err();
        let hydragen::layer_manifest::schema::Error::Content(violation) = &schema_error else {
            panic!("expected content failure");
        };
        assert_eq!(violation.schema_path, "/$defs/reference/oneOf");
        assert_eq!(violation.keyword, "oneOf");
        assert_eq!(violation.instance_path, "");
        let error = DependencyError::Schema(schema_error);
        for (origin, kind) in [
            (
                SourceOrigin::Argument {
                    index: 0,
                    reference: "bare-name".into(),
                },
                "source",
            ),
            (
                SourceOrigin::Existing {
                    index: 2,
                    reference: "bare-name".into(),
                },
                "dependency",
            ),
        ] {
            let message = dependency_error_message(&error, &origin);
            assert!(message.starts_with(&format!("Invalid {kind} reference.\n")));
            assert!(message.contains("./ or ../"));
            assert!(message.contains("Bare names are not local paths"));
            assert!(!message.contains("oneOf"));
            assert!(!message.contains("schemas listed"));
            assert!(!message.contains("non-blank"));
            assert!(!message.contains("--name"));
        }
        assert_eq!(
            dependency_error_message(&error, &SourceOrigin::Current),
            error.to_string()
        );
    }

    #[test]
    fn unfamiliar_schema_constraints_keep_the_exact_field_path_and_rule() {
        use hydragen::dependencies::{DependencyError, SourceOrigin};
        use hydragen::layer_manifest::schema::{Error, Violation, ViolationKind};
        let known = Violation {
            instance_path: "".into(),
            schema_path: "/$defs/reference/oneOf".into(),
            keyword: "oneOf".into(),
            message: "diagnostic detail".into(),
            kind: ViolationKind::Other,
        };
        for (field, value) in [
            ("instance_path", "/dependencies/3"),
            ("schema_path", "/future/oneOf"),
            ("keyword", "pattern"),
        ] {
            let mut violation = known.clone();
            match field {
                "instance_path" => violation.instance_path = value.into(),
                "schema_path" => violation.schema_path = value.into(),
                _ => violation.keyword = value.into(),
            }
            let error = DependencyError::Schema(Error::Content(Box::new(violation)));
            let origin = SourceOrigin::Argument {
                index: 0,
                reference: "input".into(),
            };
            assert_eq!(dependency_error_message(&error, &origin), error.to_string());
        }
        let error = DependencyError::Schema(Error::Setup {
            message: "internal setup detail".into(),
            detail: Some(Box::new(known)),
        });
        let origin = SourceOrigin::Argument {
            index: 0,
            reference: "input".into(),
        };
        assert_eq!(dependency_error_message(&error, &origin), error.to_string());
    }

    #[test]
    fn conflict_adapter_preserves_names_previous_origins_and_all_available_paths() {
        use hydragen::dependencies::{DependencyError, SourceOrigin};
        for (previous, previous_label) in [
            (SourceOrigin::Current, "Current layer path"),
            (
                SourceOrigin::Existing {
                    index: 2,
                    reference: "../stored-reference".into(),
                },
                "Existing source path",
            ),
            (
                SourceOrigin::Argument {
                    index: 1,
                    reference: "../first-input".into(),
                },
                "Earlier source path",
            ),
        ] {
            for (origin, incoming_label) in [
                (
                    SourceOrigin::Argument {
                        index: 3,
                        reference: "../incoming".into(),
                    },
                    "Requested source path",
                ),
                (
                    SourceOrigin::Existing {
                        index: 3,
                        reference: "../incoming".into(),
                    },
                    "Declared source path",
                ),
            ] {
                for previous_directory in [None, Some(PathBuf::from("/work/previous/source"))] {
                    let error = DependencyError::NameConflict {
                        name: "@example/shared-config".into(),
                        previous: previous.clone(),
                        previous_directory: previous_directory.clone(),
                        directory: Some("/work/requested/source".into()),
                    };
                    let message = dependency_error_message(&error, &origin);
                    assert!(
                        message.starts_with(
                            "Layer name '@example/shared-config' is already in use.\n"
                        )
                    );
                    assert!(message.contains(&format!("\nConflicting source: {previous}\n")));
                    if let Some(path) = previous_directory {
                        assert!(
                            message.contains(&format!("\n{previous_label}: {}\n", path.display()))
                        );
                    } else {
                        assert!(!message.contains(previous_label));
                    }
                    assert!(
                        message.contains(&format!("\n{incoming_label}: /work/requested/source\n"))
                    );
                    assert!(message.contains("unique name, different from the current layer"));
                    assert!(message.contains("existing declarations are not replaced"));
                    assert!(!message.contains("hydragen upgrade"));
                }
            }
        }
    }

    #[test]
    fn replacement_failure_preserves_primary_error_without_unconditional_no_files_claim() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing-layer.yaml");
        let error =
            hydragen::replacement::replace_layer_manifest(&path, b"name: app\n").unwrap_err();
        let original = error.to_string();
        let message = AddCommandError::Replacement(error).to_string();
        assert!(message.starts_with("Could not add dependencies.\n"));
        assert!(message.contains(&original));
        assert!(message.contains(&path.display().to_string()));
        assert!(!message.contains("No files were changed"));
        assert!(!message.contains("already updated"));
    }

    #[test]
    fn current_directory_collisions_name_the_file_for_preflight_and_creation() {
        for format in LayerManifestFormat::ALL {
            let path = std::path::Path::new("/a/long/project/directory").join(format.filename());
            let error = InitError::ExistingLayerManifest { path: path.clone() };
            let message = init_error_message(&error);
            assert_eq!(
                message,
                format!(
                    "This directory already has a layer manifest ({}).\nInitialisation creates a new layer manifest; it won’t overwrite an existing one.",
                    format.filename()
                )
            );
            assert!(!message.contains(&path.display().to_string()));
            assert!(error.to_string().contains(&path.display().to_string()));
            let creation_error = InitCreationError::from(error);
            assert_eq!(init_error_message(&creation_error), message);
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
            assert_eq!(init_error_message(&error), error.to_string());
            assert!(init_error_message(&error).contains(&path.display().to_string()));
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
        let output = format!("{warning}\nError: {error}\n");
        assert!(output.starts_with("Warning: Multiple layer manifests"));
        assert!(output.contains("layer.yaml, layer.json.\nlayer.yaml takes priority"));
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
            assert!(warning.contains("layer.yaml, layer.json.\nlayer.yaml takes priority"));
            assert_eq!(warning.matches("Warning:").count(), 1);
            assert!(!warning.contains("another process"));
            assert_eq!(created.path.file_name().unwrap(), created_format.filename());
            assert_eq!(
                fs::read_to_string(directory.path().join(appeared.filename())).unwrap(),
                "untouched"
            );
        }
    }
}
