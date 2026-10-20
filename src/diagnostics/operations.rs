//! Typed adapters for the remaining compact operation and interaction failures.

use std::error::Error;
use std::io;
use std::path::Path;

use hydragen::diagnostic_text::display;
use hydragen::layer_manifest::{EncodeError, LayerManifestError, MetadataError, schema};
use hydragen::{InitCreationError, InitCreationState, InitError};

use super::content::{metadata_details, metadata_diagnostic, schema_details};
use super::{AddCommandError, AddCompletionState, AuthorInputError, CliIoError, CompletionChannel};
use crate::diagnostic_body::Diagnostic;

fn unchanged(mut model: Diagnostic) -> Diagnostic {
    model.state = Some("No files were changed.".into());
    model
}

pub(super) fn diagnostic(error: &(dyn Error + 'static)) -> Diagnostic {
    if let Some(error) = error.downcast_ref::<AuthorInputError>() {
        return author_input(error);
    }
    if let Some(error) = error.downcast_ref::<AddCommandError>() {
        return add(error);
    }
    if let Some(error) = error.downcast_ref::<InitCreationError>() {
        return creation(error);
    }
    if let Some(error) = error.downcast_ref::<InitError>() {
        return init(error);
    }
    if let Some(error) = error.downcast_ref::<CliIoError>() {
        return io_context(error);
    }
    if let Some(error) = error.downcast_ref::<schema::Error>() {
        return unchanged(schema_error(error));
    }
    if let Some(error) = error.downcast_ref::<MetadataError>() {
        if let MetadataError::Schema(source @ schema::Error::Setup { .. }) = error {
            return unchanged(schema_error(source));
        }
        let mut model = Diagnostic::new("Invalid layer metadata.");
        metadata_details(error, &mut model);
        return unchanged(model);
    }
    if let Some(error) = error.downcast_ref::<LayerManifestError>() {
        return load(error);
    }
    if let Some(error) = error.downcast_ref::<EncodeError>() {
        return encoding(error);
    }
    if let Some(error) = error.downcast_ref::<io::Error>()
        && let Some(source) = error.get_ref()
        && (source.is::<CliIoError>() || source.is::<InitError>() || source.is::<schema::Error>())
    {
        return diagnostic(source);
    }
    let mut model = Diagnostic::new("Could not complete the command.");
    model.details.push(format!("Cause: {}", display(error)));
    model.correction =
        Some("Check the reported cause before retrying. Report this error if it persists.".into());
    model
}

pub(super) fn author_input(error: &AuthorInputError) -> Diagnostic {
    let model = match error {
        AuthorInputError::Invalid {
            flag,
            source: source @ schema::Error::Content(violation),
        } => {
            let mut model = Diagnostic::new(format!("Invalid value for '{flag}'."));
            let nonblank = violation.kind == schema::ViolationKind::NonBlank;
            if nonblank {
                model
                    .details
                    .push("The value must contain at least one non-whitespace character.".into());
                model
                    .details
                    .push(format!("Field: {}", display(&violation.instance_path)));
            } else {
                schema_details(source, &mut model);
            }
            model.correction = Some(match &violation.kind {
                schema::ViolationKind::NonBlank => {
                    format!("Supply a non-blank value for '{flag}', or omit this optional flag.")
                }
                schema::ViolationKind::Format { format } if format == "email" => format!(
                    "Correct the email in '{flag}'; use an address such as alex@example.com, or omit this optional flag."
                ),
                schema::ViolationKind::Format { format } if format == "uri" => format!(
                    "Correct the URL in '{flag}'; include an explicit scheme such as 'https:', or omit this optional flag."
                ),
                _ => format!(
                    "Correct '{flag}' using the constraint above, or omit this optional flag."
                ),
            });
            model
        }
        AuthorInputError::Invalid { source, .. } => schema_error(source),
        AuthorInputError::Conflict { flag, field } => {
            let mut model =
                Diagnostic::new(format!("Conflicting values in '--author' and '{flag}'."));
            model.details.push(format!("Field: {}", display(field)));
            model.correction = Some(format!(
                "Supply the same contact in both flags, or omit '{flag}' to use the shorthand contact."
            ));
            model
        }
    };
    unchanged(model)
}

pub(super) fn current_directory(source: &io::Error) -> Diagnostic {
    let mut model = Diagnostic::new("Could not determine the current directory.");
    model.details.push(format!("Cause: {}", display(source)));
    model.correction = Some("Check that the working directory still exists and is accessible, then run the command from an accessible directory.".into());
    unchanged(model)
}

pub(super) fn io_context(error: &CliIoError) -> Diagnostic {
    match error {
        CliIoError::CurrentDirectory { source } => current_directory(source),
        CliIoError::DirectoryName { directory, source } => {
            if let Some(InitError::Schema(error @ schema::Error::Setup { .. })) = source {
                return unchanged(schema_error(error));
            }
            let mut model = Diagnostic::new(format!(
                "Could not use directory '{}' as the layer name.",
                display(directory.display())
            ));
            if let Some(source) = source {
                match source {
                    InitError::InvalidName { guidance } => {
                        model.details.push(display(guidance).to_string())
                    }
                    InitError::InvalidMetadata { .. } => model.details.push(
                        "The directory name must contain at least one non-whitespace character."
                            .into(),
                    ),
                    InitError::Schema(error) => schema_details(error, &mut model),
                    other => {
                        let cause = init(other);
                        model.details.push(cause.summary);
                        model.details.extend(cause.details);
                    }
                }
            } else {
                model
                    .details
                    .push("The directory has no usable UTF-8 name.".into());
            }
            model.correction = Some("Supply --name explicitly with a valid layer name; directory names are not rewritten automatically.".into());
            unchanged(model)
        }
        CliIoError::Input { source } => {
            // Internal validation payloads retained by PromptValidation are typed,
            // not raw I/O prose. Do not escape their completed layout a second time.
            if let Some(inner) = source.get_ref()
                && (inner.is::<InitError>()
                    || inner.is::<schema::Error>()
                    || inner.is::<CliIoError>())
            {
                let mut model = diagnostic(inner);
                if model.state.is_none() {
                    model.state = Some("No files were changed.".into());
                }
                return model;
            }
            let mut model = Diagnostic::new("Could not collect initialisation input.");
            model.details.push(format!("Cause: {}", display(source)));
            model.correction = Some("Check the terminal and input stream, or supply input flags to run without questions.".into());
            unchanged(model)
        }
        CliIoError::CancellationFeedback { source } => {
            let mut model =
                Diagnostic::new("Initialisation was cancelled, but cancellation feedback failed.");
            model.details.push(format!("Cause: {}", display(source)));
            model.correction =
                Some("Check the terminal output destination before retrying.".into());
            unchanged(model)
        }
        CliIoError::InitCompletion {
            path,
            channel,
            source,
        } => {
            let mut model = Diagnostic::new(match channel {
                CompletionChannel::Output => "Could not report initialisation completion.",
                CompletionChannel::Terminal => {
                    "Could not deliver initialisation terminal feedback."
                }
            });
            model.details.push(format!("Cause: {}", display(source)));
            model.correction =
                Some("Check the output destination before running the command again.".into());
            model.state = Some(format!(
                "Layer manifest '{}' was already created; do not assume initialisation failed.",
                display(path.display())
            ));
            model
        }
    }
}

fn schema_error(error: &schema::Error) -> Diagnostic {
    let mut model = Diagnostic::new(match error {
        schema::Error::Setup { .. } => "Internal layer manifest schema error.",
        schema::Error::Content(_) => "Layer metadata does not satisfy the schema.",
    });
    schema_details(error, &mut model);
    if let schema::Error::Content(violation) = error {
        let flag = match violation.instance_path.as_str() {
            "/name" => Some("--name"),
            "/author" => Some("--author"),
            "/description" => Some("--description"),
            _ => None,
        };
        if let Some(flag) = flag {
            model.details.insert(0, format!("Input flag: {flag}"));
        }
    }
    model
}

pub(super) fn init(error: &InitError) -> Diagnostic {
    let mut model = match error {
        InitError::InvalidMetadata { field } => {
            let mut model = Diagnostic::new(format!("Invalid value for '--{}'.", display(field)));
            model
                .details
                .push("The value must contain at least one non-whitespace character.".into());
            model.correction = Some(if *field == "name" {
                "Supply a non-blank value for '--name'.".into()
            } else {
                format!(
                    "Supply a non-blank value for '--{}', or omit this optional flag.",
                    display(field)
                )
            });
            model
        }
        InitError::InvalidName { guidance } => {
            let mut model = Diagnostic::new("Invalid value for '--name'.");
            model.details.push(display(guidance).to_string());
            model.correction = Some("Correct '--name' using the constraint above; names are not rewritten automatically.".into());
            model
        }
        InitError::Schema(source) => schema_error(source),
        InitError::NestedRepository { root } => {
            let mut model = Diagnostic::new("Cannot initialise a layer below the repository root.");
            model.correction = Some(format!(
                "Run 'hydragen init' from the repository root '{}', not a subdirectory.",
                display(root.display())
            ));
            model
        }
        InitError::ExistingLayerManifest { path } => {
            let name = path.file_name().map_or_else(
                || display(path.display()).to_string(),
                |name| display(Path::new(name).display()).to_string(),
            );
            let mut model = Diagnostic::new(format!(
                "This directory already has a layer manifest ({name})."
            ));
            model.details.push(
                "Initialisation creates a new layer manifest; it won’t overwrite an existing one."
                    .into(),
            );
            // The approved collision explanation already says exactly what was
            // refused. Do not add deletion advice or assume valid contents.
            return model;
        }
        InitError::Io {
            action,
            path,
            source,
        } => io_failure(action, path, source),
        InitError::Serialization(source) => {
            let mut model = Diagnostic::new("Could not serialise layer metadata.");
            model.details.push(format!("Cause: {}", display(source)));
            model.correction = Some("Report this metadata-encoding error before retrying.".into());
            model
        }
        InitError::Encoding(source) => encoding(source),
        InitError::Discovery(source) => load(source),
    };
    model.state = Some("No files were changed.".into());
    model
}

fn creation(error: &InitCreationError) -> Diagnostic {
    match error.state {
        InitCreationState::NotCreated => init(&error.error),
        InitCreationState::PossiblyPartial => {
            let mut model = if let InitError::Io { path, source, .. } = &error.error {
                io_failure("write layer manifest", path, source)
            } else {
                init(&error.error)
            };
            // The actual write-error headline already names the exact target.
            model.state = Some("An empty or partially written layer manifest may remain; this command did not remove it.".into());
            model.correction = Some("Inspect the newly created file and resolve the reported write failure before retrying.".into());
            model
        }
    }
}

fn encoding(error: &EncodeError) -> Diagnostic {
    match error {
        EncodeError::Metadata(source) => {
            if let MetadataError::Schema(error @ schema::Error::Setup { .. }) = source {
                return schema_error(error);
            }
            let mut model = Diagnostic::new("Could not encode layer metadata.");
            metadata_details(source, &mut model);
            model
        }
        EncodeError::Serialization { format, message } => {
            let mut model =
                Diagnostic::new(format!("Could not serialise {format} layer metadata."));
            model.details.push(format!("Cause: {}", display(message)));
            model.correction = Some("Report this metadata-encoding error before retrying.".into());
            model
        }
    }
}

pub(super) fn io_failure(action: &str, path: &Path, source: &io::Error) -> Diagnostic {
    let mut model = Diagnostic::new(format!(
        "Could not {} '{}'.",
        display(action),
        display(path.display())
    ));
    model.details.push(format!("Cause: {}", display(source)));
    model.correction = Some(match source.kind() {
        io::ErrorKind::NotFound => "Check that the reported path and its parent directories exist and are accessible.",
        io::ErrorKind::PermissionDenied => "Check access permissions for the reported path and its parent directories before retrying.",
        _ => "Check the reported path and operating-system cause before retrying.",
    }.into());
    model
}

pub(super) fn load(error: &LayerManifestError) -> Diagnostic {
    match error {
        LayerManifestError::Missing { directory } => {
            let mut model = Diagnostic::new(format!(
                "No layer manifest found in '{}'.",
                display(directory.display())
            ));
            model.correction = Some("Check the directory path and choose an existing layer directory containing its own layer manifest.".into());
            model
        }
        LayerManifestError::Io {
            action,
            path,
            source,
        } => {
            let mut model = io_failure(action, path, source);
            if source.kind() == io::ErrorKind::InvalidData {
                model.correction =
                    Some("Check that the selected layer manifest is readable UTF-8 text.".into());
            }
            model
        }
        LayerManifestError::NotRegular { path } => {
            let mut model = Diagnostic::new(format!(
                "Could not read layer manifest '{}'.",
                display(path.display())
            ));
            model
                .details
                .push("The selected path must be a regular file (or a symlink to one).".into());
            model.correction = Some("Correct the selected path; lower-priority files and parent layers are not fallbacks.".into());
            model
        }
        LayerManifestError::Decode {
            path,
            format,
            message,
        } => {
            let mut model = Diagnostic::new(format!(
                "Could not decode {format} layer manifest '{}'.",
                display(path.display())
            ));
            model.details.push(format!("Details: {}", display(message)));
            model.correction = Some(format!(
                "Correct the {format} syntax in the selected layer manifest; lower-priority files and parent layers are not fallbacks."
            ));
            model
        }
        LayerManifestError::Metadata { path, source } => {
            metadata_diagnostic(source, &display(path.display()).to_string())
        }
    }
}

pub(super) fn add(error: &AddCommandError) -> Diagnostic {
    match error {
        AddCommandError::CurrentDirectory(source) => current_directory(source),
        AddCommandError::Verification {
            failure,
            invocation,
        } => super::add_verification::diagnostic(failure, invocation),
        AddCommandError::Edit(source) => edit(source),
        AddCommandError::Replacement(source) => replacement_parts(
            source.operation(),
            source.layer_manifest_path(),
            source.path(),
            source.io_error(),
            source.staging_path(),
            source.cleanup_error(),
        ),
        AddCommandError::Completion {
            state,
            layer,
            path,
            source,
        } => {
            let mut model = Diagnostic::new(match state {
                AddCompletionState::Updated => {
                    format!("Could not report completion for '{}'.", display(layer))
                }
                AddCompletionState::Unchanged => format!(
                    "Could not report the no-change result for '{}'.",
                    display(layer)
                ),
            });
            model
                .details
                .push(format!("Completion output failed: {}", display(source)));
            model.correction =
                Some("Check the output destination before running the command again.".into());
            model.state = Some(match state {
                AddCompletionState::Updated => format!("The layer manifest '{}' was already updated; do not assume the addition failed.", display(path.display())),
                AddCompletionState::Unchanged => "The requested dependencies were already present; the layer manifest was not updated.".into(),
            });
            model
        }
    }
}

fn edit(error: &hydragen::layer_manifest::edit::EditError) -> Diagnostic {
    use hydragen::layer_manifest::edit::EditError;
    let mut model = match error {
        EditError::UnsupportedEdit {
            path,
            format,
            reason,
        } => {
            let mut model = Diagnostic::new(format!(
                "Could not safely edit {format} layer manifest '{}'.",
                display(path.display())
            ));
            model.details.push(format!("Details: {}", display(reason)));
            model.correction = Some("Adjust the source construct identified above, retaining its intended values and comments.".into());
            model
        }
        EditError::InvalidCandidate {
            path,
            format,
            source,
        } => {
            let mut model = Diagnostic::new(
                if matches!(
                    source,
                    LayerManifestError::Metadata {
                        source: MetadataError::Schema(schema::Error::Setup { .. }),
                        ..
                    }
                ) {
                    format!(
                        "Could not validate the prepared {format} layer manifest '{}'.",
                        display(path.display())
                    )
                } else {
                    format!(
                        "Prepared {format} layer manifest '{}' is invalid.",
                        display(path.display())
                    )
                },
            );
            let cause = load(source);
            model.details.push(cause.summary);
            model.details.extend(cause.details);
            model.correction = Some("The prepared edit failed its safety checks; report this error. The original layer manifest has been retained.".into());
            model
        }
        EditError::UnintendedChange {
            path,
            format,
            reason,
        } => {
            let mut model = Diagnostic::new(format!(
                "Refused unsafe edit to {format} layer manifest '{}'.",
                display(path.display())
            ));
            model.details.push(format!("Details: {}", display(reason)));
            model.correction = Some(
                "Report this edit-safety failure; the original layer manifest has been retained."
                    .into(),
            );
            model
        }
    };
    model.state = Some("No files were changed.".into());
    model
}

fn replacement_parts(
    operation: hydragen::replacement::ReplaceLayerManifestOperation,
    destination: &Path,
    path: &Path,
    source: &io::Error,
    staging: Option<&Path>,
    cleanup: Option<&io::Error>,
) -> Diagnostic {
    let mut model = Diagnostic::new(format!(
        "Could not {operation} at '{}'.",
        display(path.display())
    ));
    model.details.push(format!("Cause: {}", display(source)));
    if let Some(cleanup) = cleanup {
        model.details.push(match staging {
            Some(path) => format!(
                "Could not remove staging file '{}': {}",
                display(path.display()),
                display(cleanup)
            ),
            None => format!("Staging-file cleanup failed: {}", display(cleanup)),
        });
        model.correction = Some("Inspect the reported staging path before removing anything, then resolve the cleanup failure before retrying.".into());
    } else {
        if let Some(staging) = staging
            && staging != path
        {
            model.details.push(format!(
                "Staging file: '{}' (cleanup completed).",
                display(staging.display())
            ));
        }
        model.correction = Some(match operation {
            hydragen::replacement::ReplaceLayerManifestOperation::InspectTarget => "Check that the selected layer manifest exists, is accessible and is a regular file rather than a symlink.",
            hydragen::replacement::ReplaceLayerManifestOperation::CreateStaging => "Check directory access and available space before retrying.",
            hydragen::replacement::ReplaceLayerManifestOperation::WriteStaging
            | hydragen::replacement::ReplaceLayerManifestOperation::PreservePermissions
            | hydragen::replacement::ReplaceLayerManifestOperation::SyncStaging
            | hydragen::replacement::ReplaceLayerManifestOperation::Rename => "Check directory access, file permissions and available space before retrying.",
        }.into());
    }
    let destination_state = if path == destination {
        "The selected layer manifest was not replaced.".into()
    } else {
        format!(
            "The selected layer manifest '{}' was not replaced.",
            display(destination.display())
        )
    };
    model.state = Some(format!(
        "{destination_state}{}",
        if cleanup.is_some() {
            "\nA staging file may remain."
        } else {
            ""
        }
    ));
    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::Presentation;
    use hydragen::LayerManifestFormat;

    const RAW: &str = "配置\\literal\x1b[2J\n\r\t\u{9b}\u{202e}";

    fn assert_family(model: &Diagnostic) {
        let body = model.body();
        assert!(body.lines().next().unwrap().ends_with('.'), "{body}");
        assert!(!body.contains("\n\n\n"), "{body}");
        assert!(
            body.chars().all(|ch| !ch.is_control() || ch == '\n'),
            "{body:?}"
        );
        assert!(!body.contains('\u{202e}'), "{body:?}");
        assert!(!body.contains("Usage:"), "{body}");
        let plain = model.render(Presentation {
            decorated: false,
            basic: false,
        });
        let styled = model.render(Presentation {
            decorated: true,
            basic: false,
        });
        assert_eq!(console::strip_ansi_codes(&styled), format!("✕ {plain}"));
        assert!(styled.lines().skip(1).all(|line| !line.contains('\x1b')));
    }

    fn violation() -> schema::Violation {
        schema::Violation {
            instance_path: "/author".into(),
            schema_path: "/future/pattern".into(),
            keyword: "pattern".into(),
            message: RAW.into(),
            kind: schema::ViolationKind::Other,
        }
    }

    fn setup() -> schema::Error {
        schema::Error::Setup {
            message: RAW.into(),
            detail: Some(Box::new(violation())),
        }
    }

    #[test]
    fn init_flags_and_defaults_keep_route_specific_corrections_without_blame_for_setup() {
        for field in ["name", "author", "description"] {
            let model = init(&InitError::InvalidMetadata { field });
            assert_family(&model);
            let text = model.body();
            assert!(text.starts_with(&format!("Invalid value for '--{field}'.")));
            assert!(text.contains("non-whitespace character"));
            assert_eq!(text.contains("omit this optional flag"), field != "name");
            assert!(!text.contains("Enter"));
            assert!(!text.contains("clear the field"));
            assert_eq!(text.matches("No files were changed.").count(), 1);
        }
        let model = init(&InitError::InvalidName {
            guidance: RAW.into(),
        });
        assert_family(&model);
        assert!(model.body().contains(&display(RAW).to_string()));
        assert!(model.body().contains("Correct '--name'"));
        let source = InitError::InvalidName {
            guidance: "approved lowercase policy".into(),
        };
        let model = io_context(&CliIoError::DirectoryName {
            directory: RAW.into(),
            source: Some(source),
        });
        assert_family(&model);
        assert!(model.body().contains("approved lowercase policy"));
        assert!(model.body().contains("Supply --name explicitly"));
        assert!(!model.body().contains("Invalid value for '--name'"));
        let model = io_context(&CliIoError::DirectoryName {
            directory: RAW.into(),
            source: None,
        });
        assert!(model.body().contains("no usable UTF-8 name"));
        let model = io_context(&CliIoError::DirectoryName {
            directory: RAW.into(),
            source: Some(InitError::Schema(setup())),
        });
        assert_family(&model);
        assert!(model.summary.starts_with("Internal"));
        assert!(!model.body().contains("Supply --name"));
    }

    #[test]
    fn author_input_failures_share_compact_composition_and_keep_actual_input_sources() {
        for (flag, field, value) in [
            ("--author", "author.email", RAW),
            ("--author-email", "author.email", "not-an-email"),
            ("--author-url", "author.url", "relative/path"),
        ] {
            let source = schema::validate_field(field, value).unwrap_err();
            let error = AuthorInputError::Invalid { flag, source };
            let model = diagnostic(&error);
            assert_family(&model);
            assert!(model.summary.contains(flag));
            assert!(model.body().contains(&field.replace('.', "/")));
            assert!(model.body().contains("Schema rule:"));
            assert_eq!(model.body().matches("No files were changed.").count(), 1);
            assert_eq!(model.body(), error.to_string());
            assert!(!model.body().contains("Enter"));
        }
        for (flag, field) in [
            ("--author-email", "author.email"),
            ("--author-url", "author.url"),
        ] {
            let error = AuthorInputError::Conflict { flag, field };
            let model = diagnostic(&error);
            assert_family(&model);
            assert!(model.summary.contains("'--author'") && model.summary.contains(flag));
            assert!(model.body().contains(field));
            assert!(model.body().contains("same contact"));
            assert_eq!(model.body(), error.to_string());
        }
        let model = diagnostic(&AuthorInputError::Invalid {
            flag: "--author-email",
            source: setup(),
        });
        assert_family(&model);
        assert!(model.summary.starts_with("Internal"));
        assert!(!model.body().contains("--author-email"));
        assert!(
            model
                .body()
                .contains("changing valid input will not resolve it")
        );
    }

    #[test]
    fn unknown_schema_constraints_keep_flags_fields_locations_and_rules_without_blank_relabelling()
    {
        let model = init(&InitError::Schema(schema::Error::Content(Box::new(
            violation(),
        ))));
        assert_family(&model);
        let body = model.body();
        for fact in [
            "Input flag: --author",
            "/author (pattern)",
            "/future/pattern",
            &display(RAW).to_string(),
        ] {
            assert!(body.contains(fact), "{body}");
        }
        assert!(!body.contains("non-blank"));
        for source in [
            InitError::Schema(setup()),
            InitError::Encoding(EncodeError::Metadata(MetadataError::Schema(setup()))),
        ] {
            let model = init(&source);
            assert_family(&model);
            assert!(
                model
                    .summary
                    .starts_with("Internal layer manifest schema error.")
            );
            assert!(model.body().contains("Report this internal error"));
            for wrong in [
                "Input flag:",
                "Correct '--name'",
                "non-blank",
                "retry valid",
            ] {
                assert!(!model.body().contains(wrong));
            }
            assert!(model.body().contains("/future/pattern"));
        }
    }

    #[test]
    fn init_creation_uses_typed_state_not_the_io_action_text() {
        for state in [
            InitCreationState::NotCreated,
            InitCreationState::PossiblyPartial,
        ] {
            let error = InitCreationError {
                state,
                discovery: None,
                error: InitError::Io {
                    action: "opaque action without lifecycle hints",
                    path: Path::new(RAW).join("layer.yaml"),
                    source: io::Error::other(RAW.to_owned()),
                },
            };
            let model = creation(&error);
            assert_family(&model);
            let text = model.body();
            assert!(text.contains(&display(RAW).to_string()));
            if state == InitCreationState::PossiblyPartial {
                assert!(model.summary.starts_with("Could not write layer manifest"));
                assert!(text.contains("An empty or partially written layer manifest may remain;"));
                assert!(text.contains("this command did not remove it"));
                let InitError::Io { path, .. } = &error.error else {
                    panic!("expected target path");
                };
                assert_eq!(
                    text.matches(&display(path.display()).to_string()).count(),
                    1
                );
                assert!(text.contains("Inspect the newly created file"));
                assert!(!text.contains("No files were changed"));
            } else {
                assert!(
                    model
                        .summary
                        .starts_with("Could not opaque action without lifecycle hints")
                );
                assert_eq!(text.matches("No files were changed.").count(), 1);
                assert!(!text.contains("partially written"));
            }
            assert_eq!(diagnostic(&error).body(), text);
            assert!(std::error::Error::source(&error).unwrap().is::<InitError>());
        }
    }

    #[test]
    fn partial_creation_recovery_is_cause_neutral_and_retains_limit_permission_and_opaque_failures()
    {
        for (kind, cause) in [
            (
                io::ErrorKind::FileTooLarge,
                "process file-size limit exceeded",
            ),
            (io::ErrorKind::PermissionDenied, "write permission denied"),
            (io::ErrorKind::Other, RAW),
        ] {
            let error = InitCreationError {
                state: InitCreationState::PossiblyPartial,
                discovery: None,
                error: InitError::Io {
                    action: "write layer manifest (a partial file may remain)",
                    path: "/work/layer.yaml".into(),
                    source: io::Error::new(kind, cause),
                },
            };
            let model = creation(&error);
            assert_family(&model);
            assert_eq!(
                model.correction.as_deref(),
                Some(
                    "Inspect the newly created file and resolve the reported write failure before retrying."
                )
            );
            let correction = model.correction.as_deref().unwrap();
            for prescription in ["directory access", "available space", "permissions"] {
                assert!(!correction.contains(prescription));
            }
            let text = model.body();
            assert!(text.contains(&format!("Cause: {}", display(cause))));
            assert!(text.contains("Could not write layer manifest '/work/layer.yaml'."));
            assert!(text.contains("An empty or partially written layer manifest may remain; this command did not remove it."));
            assert!(!text.contains("No files were changed"));
            assert_eq!(error.state, InitCreationState::PossiblyPartial);
            let InitError::Io { source, .. } = &error.error else {
                panic!("expected I/O cause");
            };
            assert_eq!(source.kind(), kind);
            assert_eq!(source.to_string(), cause);
        }
    }

    #[test]
    fn repository_io_encoding_and_all_load_variants_have_safe_complete_bodies() {
        let path = Path::new(RAW).join("layer.yaml");
        let errors = [
            InitError::NestedRepository { root: RAW.into() },
            InitError::Io {
                action: "inspect repository marker",
                path: path.clone(),
                source: io::Error::other(RAW.to_owned()),
            },
            InitError::Serialization(serde_json::from_str::<serde_json::Value>("[").unwrap_err()),
            InitError::Encoding(EncodeError::Serialization {
                format: LayerManifestFormat::Toml,
                message: RAW.into(),
            }),
            InitError::Encoding(EncodeError::Metadata(MetadataError::InvalidName {
                guidance: RAW.into(),
            })),
            InitError::Discovery(LayerManifestError::Missing {
                directory: RAW.into(),
            }),
            InitError::Discovery(LayerManifestError::Io {
                action: "inspect layer manifest path",
                path: path.clone(),
                source: io::Error::new(io::ErrorKind::PermissionDenied, RAW),
            }),
            InitError::Discovery(LayerManifestError::NotRegular { path: path.clone() }),
            InitError::Discovery(LayerManifestError::Decode {
                path: path.clone(),
                format: LayerManifestFormat::Yaml,
                message: format!("line 3 column 2 {RAW}"),
            }),
            InitError::Discovery(LayerManifestError::Metadata {
                path,
                source: MetadataError::Schema(schema::Error::Content(Box::new(violation()))),
            }),
        ];
        for error in errors {
            let model = init(&error);
            assert_family(&model);
            assert_eq!(model.body().matches("No files were changed.").count(), 1);
            assert!(model.body().contains("Tip:"));
            assert_eq!(diagnostic(&error).body(), model.body());
        }
        let decoded = load(&LayerManifestError::Decode {
            path: "/work/layer.toml".into(),
            format: LayerManifestFormat::Toml,
            message: "line 9 column 5: /external/cause\nretained".into(),
        });
        assert!(
            decoded
                .body()
                .contains(r"line 9 column 5: /external/cause\nretained")
        );
        assert!(!decoded.body().contains("hydragen init"));
        let missing = load(&LayerManifestError::Missing {
            directory: "/diagnostic-only/missing".into(),
        });
        assert!(missing.body().contains("Check the directory path"));
        assert!(!missing.body().contains("hydragen init"));
    }

    #[test]
    fn every_edit_variant_preserves_source_facts_and_does_not_blame_original_for_internal_candidate_errors()
     {
        use hydragen::layer_manifest::edit::EditError;
        let path = Path::new(RAW).join("layer.jsonc");
        let errors = [
            EditError::UnsupportedEdit {
                path: path.clone(),
                format: LayerManifestFormat::Jsonc,
                reason: RAW.into(),
            },
            EditError::UnintendedChange {
                path: path.clone(),
                format: LayerManifestFormat::Jsonc,
                reason: RAW.into(),
            },
            EditError::InvalidCandidate {
                path: path.clone(),
                format: LayerManifestFormat::Jsonc,
                source: LayerManifestError::Decode {
                    path: path.clone(),
                    format: LayerManifestFormat::Jsonc,
                    message: format!("line 2 column 4 {RAW}"),
                },
            },
            EditError::InvalidCandidate {
                path: path.clone(),
                format: LayerManifestFormat::Jsonc,
                source: LayerManifestError::Metadata {
                    path,
                    source: MetadataError::Schema(setup()),
                },
            },
        ];
        for error in errors {
            let model = edit(&error);
            assert_family(&model);
            assert!(model.body().contains(&display(RAW).to_string()));
            assert_eq!(model.body().matches("No files were changed.").count(), 1);
            if matches!(
                error,
                EditError::InvalidCandidate {
                    source: LayerManifestError::Metadata { .. },
                    ..
                }
            ) {
                assert!(model.summary.starts_with("Could not validate the prepared"));
                assert!(!model.summary.contains("is invalid"));
                assert!(
                    model
                        .body()
                        .contains("Internal layer manifest schema error")
                );
                assert!(model.body().contains("/future/pattern"));
            }
        }
    }

    #[test]
    fn all_replacement_operations_preserve_primary_secondary_and_exact_staging_paths_with_truthful_state()
     {
        use hydragen::replacement::ReplaceLayerManifestOperation as O;
        let destination = Path::new(RAW).join("layer.yaml");
        let staging = Path::new(RAW).join(".exact-staging-name");
        let source = io::Error::other(format!("primary {RAW}"));
        let cleanup = io::Error::other(format!("secondary {RAW}"));
        for operation in [
            O::InspectTarget,
            O::CreateStaging,
            O::WriteStaging,
            O::PreservePermissions,
            O::SyncStaging,
            O::Rename,
        ] {
            for failed_cleanup in [false, true] {
                let path = match operation {
                    O::CreateStaging => destination.parent().unwrap(),
                    O::WriteStaging | O::PreservePermissions | O::SyncStaging => &staging,
                    _ => &destination,
                };
                let model = replacement_parts(
                    operation,
                    &destination,
                    path,
                    &source,
                    Some(&staging),
                    failed_cleanup.then_some(&cleanup),
                );
                assert_family(&model);
                let text = model.body();
                assert!(text.contains(&format!("Cause: primary {}", display(RAW))));
                assert!(text.contains(&display(destination.display()).to_string()));
                assert_eq!(
                    text.matches(&display(destination.display()).to_string())
                        .count(),
                    1
                );
                assert!(text.contains(&display(staging.display()).to_string()));
                assert!(text.contains("was not replaced."));
                assert!(!text.contains("No files were changed"));
                assert!(!text.contains("already updated"));
                if failed_cleanup {
                    let line = format!(
                        "Could not remove staging file '{}': secondary {}",
                        display(staging.display()),
                        display(RAW)
                    );
                    assert!(text.contains(&line), "{text}");
                    assert!(text.find("primary").unwrap() < text.find("secondary").unwrap());
                    assert!(
                        text.contains("Inspect the reported staging path before removing anything")
                    );
                    assert!(text.contains("A staging file may remain"));
                } else {
                    assert!(!text.contains("secondary"));
                    assert!(!text.contains("A staging file may remain"));
                }
            }
        }
        let model = replacement_parts(
            O::InspectTarget,
            &destination,
            &destination,
            &source,
            None,
            Some(&cleanup),
        );
        assert!(
            model
                .body()
                .contains("Staging-file cleanup failed: secondary")
        );
    }

    #[test]
    fn completion_states_never_become_cancellation_or_no_files_changed_and_escape_names_too() {
        for state in [AddCompletionState::Updated, AddCompletionState::Unchanged] {
            let model = add(&AddCommandError::Completion {
                state,
                layer: RAW.into(),
                path: Path::new(RAW).join("layer.yaml"),
                source: io::Error::other(RAW.to_owned()),
            });
            assert_family(&model);
            assert!(model.body().contains(&display(RAW).to_string()));
            assert!(!model.body().contains("No files were changed"));
            assert!(!model.body().contains("cancelled"));
            assert_eq!(
                model.body().contains("was already updated"),
                state == AddCompletionState::Updated
            );
            assert_eq!(
                model.body().contains("was not updated"),
                state == AddCompletionState::Unchanged
            );
        }
        for channel in [CompletionChannel::Output, CompletionChannel::Terminal] {
            let context = CliIoError::InitCompletion {
                path: Path::new(RAW).join("layer.toml"),
                channel,
                source: io::Error::new(io::ErrorKind::Interrupted, RAW),
            };
            let error = context.into_io();
            assert_eq!(error.kind(), io::ErrorKind::Interrupted);
            let model = diagnostic(&error);
            assert_family(&model);
            assert!(model.body().contains("was already created"));
            assert!(!model.body().contains("No files were changed"));
            assert!(!crate::interaction::is_reported_cancellation(&error));
            assert_eq!(model.body(), error.to_string());
        }
    }

    #[test]
    fn typed_io_preserves_os_error_kind_cause_and_context_without_escaping_its_layout_twice() {
        let error = CliIoError::CurrentDirectory {
            source: io::Error::from_raw_os_error(2),
        }
        .into_io();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        let payload = error
            .get_ref()
            .unwrap()
            .downcast_ref::<CliIoError>()
            .unwrap();
        let original = std::error::Error::source(payload)
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap();
        assert_eq!(original.raw_os_error(), Some(2));
        let model = diagnostic(&error);
        assert_family(&model);
        assert!(
            model
                .summary
                .starts_with("Could not determine the current directory")
        );
        let nested = CliIoError::Input {
            source: io::Error::other(InitError::Schema(setup())),
        }
        .into_io();
        let model = diagnostic(&nested);
        assert_family(&model);
        assert!(model.summary.starts_with("Internal"));
        assert_eq!(model.body().matches("No files were changed.").count(), 1);
        assert!(model.body().contains("\n\n"));
        assert!(!model.body().contains(r"\n\n"));
        assert!(model.body().contains(&display(RAW).to_string()));
    }

    #[test]
    fn a_typed_input_wrapper_does_not_overwrite_known_completed_state() {
        let completion = CliIoError::InitCompletion {
            path: "/work/layer.yaml".into(),
            channel: CompletionChannel::Terminal,
            source: io::ErrorKind::BrokenPipe.into(),
        }
        .into_io();
        let wrapped = CliIoError::Input { source: completion }.into_io();
        let model = diagnostic(&wrapped);
        assert_family(&model);
        assert!(model.body().contains("was already created"));
        assert!(!model.body().contains("No files were changed"));
        assert!(!crate::interaction::is_reported_cancellation(&wrapped));
        let model = diagnostic(&MetadataError::Schema(setup()));
        assert!(
            model
                .summary
                .starts_with("Internal layer manifest schema error")
        );
        assert!(!model.body().contains("Invalid layer metadata"));
    }

    #[test]
    fn opaque_unexpected_io_is_one_safe_cause_not_trusted_layout_or_invented_write_state() {
        let error = io::Error::other(format!("{RAW}\nUsage: spoof\nNo files were changed."));
        let model = diagnostic(&error);
        assert!(model.summary.starts_with("Could not complete the command."));
        assert!(model.state.is_none());
        let text = model.body();
        assert!(text.contains(&display(error.to_string()).to_string()));
        assert!(!text.contains("\nUsage:"));
        assert!(!text.contains("\nNo files were changed."));
        assert!(!text.contains('\x1b'));
        assert!(!text.contains('\u{202e}'));
    }
}
