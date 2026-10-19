//! Presentation of the complete read-only add-verification family.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::Path;

use hydragen::dependencies::{AddFailure, DependencyError, DependencyFailure, SourceOrigin};
use hydragen::diagnostic_text::display;
#[cfg(test)]
use hydragen::layer_manifest::MetadataError;
use hydragen::layer_manifest::{LayerManifestError, schema};

use crate::diagnostic_body::{Diagnostic, SourceGroup};

use super::content::{
    DECLARED_REFERENCE_CORRECTION, known_reference_rule, metadata_diagnostic, schema_details,
};

pub(super) fn diagnostic(failure: &AddFailure, invocation: &Path) -> Diagnostic {
    // This prerequisite is an approved transcript, not a source batch.
    if matches!(
        failure.errors.as_slice(),
        [DependencyFailure {
            origin: SourceOrigin::Current,
            error: DependencyError::MissingCurrent { .. },
        }]
    ) {
        return Diagnostic {
            summary: "No layer manifest found in this directory or its parents.".into(),
            correction: Some(
                concat!(
                    "Run this command from an existing layer directory, or run\n",
                    "'hydragen init' in the directory you want to use as a layer."
                )
                .into(),
            ),
            state: Some("No files were changed.".into()),
            ..Diagnostic::default()
        };
    }

    let labels = ambiguous_labels(failure);
    let single = failure.errors.len() == 1;
    let mut model = if let [problem] = failure.errors.as_slice() {
        adapt(&problem.error, &problem.origin, failure, &labels, true)
    } else {
        Diagnostic::new(format!(
            "Dependency checks failed ({} errors).",
            failure.errors.len()
        ))
    };
    if let Some(base) = &failure.directory
        && base != invocation
        && failure.errors.iter().any(needs_base)
    {
        model.details.insert(
            0,
            format!("Paths are relative to '{}'.", display(base.display())),
        );
    }
    if !single {
        for problem in &failure.errors {
            let content = adapt(&problem.error, &problem.origin, failure, &labels, false);
            model.groups.push(SourceGroup {
                label: origin_label(&problem.origin, &labels),
                summary: content.summary,
                details: content.details,
                correction: content.correction,
            });
        }
    }
    model.state = Some("No files were changed.".into());
    model
}

fn reference(origin: &SourceOrigin) -> Option<&str> {
    match origin {
        SourceOrigin::Current => None,
        SourceOrigin::Existing { reference, .. } | SourceOrigin::Argument { reference, .. } => {
            Some(reference)
        }
    }
}

fn needs_base(problem: &DependencyFailure) -> bool {
    let relative = |origin| reference(origin).is_some_and(|text| Path::new(text).is_relative());
    // These families already establish local-path interpretation. Do not add a
    // second acceptance grammar here, or treat a published claim as a path.
    match &problem.error {
        DependencyError::Schema(_)
        | DependencyError::UnsupportedPath { .. }
        | DependencyError::PublishedAdditionUnsupported { .. } => false,
        DependencyError::NameConflict {
            previous,
            previous_directory,
            directory,
            ..
        } => {
            (directory.is_some() && relative(&problem.origin))
                || (previous_directory.is_some() && relative(previous))
        }
        DependencyError::LayerManifest(_)
        | DependencyError::Io { .. }
        | DependencyError::NotDirectory { .. }
        | DependencyError::MissingCurrent { .. }
        | DependencyError::CurrentSymlink { .. } => relative(&problem.origin),
    }
}

fn ambiguous_labels(failure: &AddFailure) -> HashSet<String> {
    // Escaped raw controls can look identical to literal escape text. Compare the
    // visible labels and original positions. Build this once, not by rescanning
    // the entire batch for every label (including conflict references).
    let mut positions = HashMap::new();
    let mut ambiguous = HashSet::new();
    for problem in &failure.errors {
        let previous = match &problem.error {
            DependencyError::NameConflict { previous, .. } => Some(previous),
            _ => None,
        };
        for origin in std::iter::once(&problem.origin).chain(previous) {
            let index = match origin {
                SourceOrigin::Current => continue,
                SourceOrigin::Argument { index, .. } | SourceOrigin::Existing { index, .. } => {
                    *index
                }
            };
            let label = unpositioned_label(origin);
            if let Some(first) = positions.get(&label) {
                if *first != index {
                    ambiguous.insert(label);
                }
            } else {
                positions.insert(label, index);
            }
        }
    }
    ambiguous
}

fn unpositioned_label(origin: &SourceOrigin) -> String {
    match origin {
        SourceOrigin::Current => "Current layer".into(),
        SourceOrigin::Argument { reference, .. } => format!("Source '{}'", display(reference)),
        SourceOrigin::Existing { reference, .. } => {
            format!("Existing dependency '{}'", display(reference))
        }
    }
}

fn origin_label(origin: &SourceOrigin, ambiguous: &HashSet<String>) -> String {
    let mut label = unpositioned_label(origin);
    if ambiguous.contains(&label) {
        match origin {
            SourceOrigin::Current => {}
            SourceOrigin::Argument { index, .. } => {
                label.push_str(&format!(" (argument {})", index + 1))
            }
            SourceOrigin::Existing { index, .. } => {
                label.push_str(&format!(" (declaration {})", index + 1))
            }
        }
    }
    label
}

/// Only derive a selected filename from a source spelling when an existing
/// observation confirms that exact selected path. Never inspect or normalise it.
fn selected_path(path: &Path, origin: &SourceOrigin, failure: &AddFailure) -> String {
    if let Some(reference) = reference(origin)
        && observed_selection(path, failure)
        && let Some(filename) = path.file_name()
    {
        return display(Path::new(reference).join(filename).display()).to_string();
    }
    display(path.display()).to_string()
}

fn observed_selection(path: &Path, failure: &AddFailure) -> bool {
    failure.discoveries.iter().any(|discovery| {
        Some(discovery.directory.as_path()) == path.parent()
            && discovery
                .selected()
                .is_some_and(|selected| selected.path == path)
    })
}

fn needs_origin_context(
    error: &DependencyError,
    origin: &SourceOrigin,
    failure: &AddFailure,
) -> bool {
    match (origin, error) {
        (SourceOrigin::Current, _) => false,
        (_, DependencyError::NameConflict { .. } | DependencyError::NotDirectory { .. }) => false,
        (_, DependencyError::LayerManifest(error))
            if matches!(error.as_ref(), LayerManifestError::Missing { .. }) =>
        {
            false
        }
        (SourceOrigin::Existing { .. }, _) => true,
        (_, DependencyError::Schema(schema::Error::Setup { .. })) => true,
        (
            _,
            DependencyError::Schema(_)
            | DependencyError::UnsupportedPath { .. }
            | DependencyError::PublishedAdditionUnsupported { .. },
        ) => false,
        (_, DependencyError::LayerManifest(error)) => match error.as_ref() {
            LayerManifestError::Missing { .. } => false,
            LayerManifestError::Io { path, .. }
            | LayerManifestError::NotRegular { path }
            | LayerManifestError::Decode { path, .. }
            | LayerManifestError::Metadata { path, .. } => !observed_selection(path, failure),
        },
        (_, DependencyError::Io { path, .. }) => !reference(origin).is_some_and(|reference| {
            failure
                .directory
                .as_ref()
                .is_some_and(|base| base.join(reference) == *path)
        }),
        (_, DependencyError::MissingCurrent { .. } | DependencyError::CurrentSymlink { .. }) => {
            true
        }
    }
}

fn directory_text(path: &Path, origin: &SourceOrigin) -> String {
    reference(origin).map_or_else(
        || display(path.display()).to_string(),
        |reference| display(reference).to_string(),
    )
}

fn adapt(
    error: &DependencyError,
    origin: &SourceOrigin,
    failure: &AddFailure,
    labels: &HashSet<String>,
    single: bool,
) -> Diagnostic {
    let mut model = match error {
        DependencyError::Schema(source) => {
            let kind = if matches!(origin, SourceOrigin::Existing { .. }) {
                "dependency"
            } else {
                "source"
            };
            let summary = if matches!(source, schema::Error::Setup { .. }) {
                "Internal layer manifest schema error.".into()
            } else if matches!(origin, SourceOrigin::Current) {
                "Layer manifest schema validation failed.".into()
            } else if single && let Some(reference) = reference(origin) {
                format!("Invalid {kind} reference '{}'.", display(reference))
            } else {
                format!("Invalid {kind} reference.")
            };
            let mut model = Diagnostic::new(summary);
            // This exact reference rule already has a useful correction. Its
            // root-level oneOf explanation is internal grammar, not a field
            // location or an additional constraint the developer can act on.
            if let schema::Error::Content(violation) = source
                && reference(origin).is_some()
                && known_reference_rule(violation)
                && violation.instance_path.is_empty()
            {
                model.correction = Some(if matches!(origin, SourceOrigin::Existing { .. }) {
                    DECLARED_REFERENCE_CORRECTION
                } else {
                    "Use an explicit local path beginning with ./ or ../, or an absolute path. Bare names are not local paths; published additions are not supported yet."
                }.into());
            } else {
                schema_details(source, &mut model);
            }
            model
        }
        DependencyError::LayerManifest(source) => layer_manifest(source, origin, failure, single),
        DependencyError::Io {
            action,
            path,
            source,
        } => {
            // directory resolution may report the original joined path or a
            // canonical identity. Only the former is a proven spelling match.
            let shown = if reference(origin).is_some_and(|reference| {
                failure
                    .directory
                    .as_ref()
                    .is_some_and(|base| base.join(reference) == *path)
            }) {
                directory_text(path, origin)
            } else {
                display(path.display()).to_string()
            };
            io_diagnostic(action, &shown, source, false)
        }
        DependencyError::NotDirectory { path } => {
            let summary = if single {
                let role = match origin {
                    SourceOrigin::Current => "Current layer directory",
                    SourceOrigin::Existing { .. } => "Existing dependency",
                    SourceOrigin::Argument { .. } => "Local source",
                };
                format!(
                    "{role} '{}' is not a directory.",
                    directory_text(path, origin)
                )
            } else {
                "Local source must be a directory.".into()
            };
            let mut model = Diagnostic::new(summary);
            model.correction = Some(if matches!(origin, SourceOrigin::Current) {
                "Run this command from an existing layer directory or a directory below it."
            } else {
                "Choose an existing directory containing its own valid layer manifest, not a file."
            }.into());
            model
        }
        DependencyError::MissingCurrent { invocation } => {
            let mut model = Diagnostic::new(format!(
                "No layer manifest found in '{}' or its parents through the filesystem root.",
                display(invocation.display())
            ));
            model.correction = Some("Run this command from an existing layer directory, or run 'hydragen init' in the directory you want to use as a layer.".into());
            model
        }
        DependencyError::CurrentSymlink { path } => {
            let mut model = Diagnostic::new(format!(
                "Cannot edit symlinked current layer manifest '{}'.",
                display(path.display())
            ));
            model.correction = Some("Use a regular layer manifest file instead; the link and its target have not been changed.".into());
            model
        }
        DependencyError::UnsupportedPath { reference, reason } => {
            let summary = if single {
                format!(
                    "Cannot resolve local path '{}' on this host.",
                    display(reference)
                )
            } else {
                "Cannot resolve this local path on this host.".into()
            };
            let mut model = Diagnostic::new(summary);
            model.details.push(format!("Reason: {}.", display(reason)));
            model.correction = Some("Supply an explicit native path without translating the stored reference. Literal home and environment-variable syntax is not expanded.".into());
            model
        }
        DependencyError::PublishedAdditionUnsupported { name, version } => {
            let summary = if single {
                format!(
                    "Adding published layer '{}@{}' is not supported yet.",
                    display(name),
                    display(version)
                )
            } else {
                "Adding published layers is not supported yet.".into()
            };
            let mut model = Diagnostic::new(summary);
            model.correction = Some(
                "Supply an explicit local path instead. This command does not access a registry."
                    .into(),
            );
            model
        }
        DependencyError::NameConflict {
            name,
            previous,
            previous_directory,
            directory,
        } => {
            // Verification supplies canonical identities. Compare those paths,
            // not escaped text: distinct paths can have identical display text.
            let same_existing_directory = matches!(origin, SourceOrigin::Existing { .. })
                && matches!(previous, SourceOrigin::Existing { .. })
                && previous_directory
                    .as_deref()
                    .zip(directory.as_deref())
                    .is_some_and(|(previous, incoming)| previous == incoming);
            let mut model =
                Diagnostic::new(format!("Layer name '{}' is already in use.", display(name)));
            if single {
                model.details.push(origin_label(origin, labels));
            }
            model.details.push(format!(
                "Conflicting source: {}",
                origin_label(previous, labels)
            ));
            if same_existing_directory {
                model
                    .details
                    .push("Both declarations refer to the same layer directory.".into());
            } else {
                if let Some(path) = previous_directory {
                    let label = match previous {
                        SourceOrigin::Current => "Current layer path",
                        SourceOrigin::Existing { .. } => "Existing source path",
                        SourceOrigin::Argument { .. } => "Earlier source path",
                    };
                    model
                        .details
                        .push(format!("{label}: {}", display(path.display())));
                }
                if let Some(path) = directory {
                    let label = if matches!(origin, SourceOrigin::Existing { .. }) {
                        "Declared source path"
                    } else {
                        "Requested source path"
                    };
                    model
                        .details
                        .push(format!("{label}: {}", display(path.display())));
                }
            }
            model.details.push(
                "Each direct dependency must have a unique name, different from the current layer."
                    .into(),
            );
            model.correction = Some(match (origin, previous) {
                (SourceOrigin::Existing { .. }, SourceOrigin::Existing { .. })
                    if same_existing_directory =>
                {
                    "Remove the redundant existing dependency declaration from the current layer manifest; existing declarations are not replaced."
                }
                (SourceOrigin::Existing { .. }, SourceOrigin::Current) => {
                    "Remove or correct the existing dependency declaration that uses the current layer's reserved name; existing declarations are not replaced."
                }
                (SourceOrigin::Existing { .. }, _) => {
                    "Remove or correct the conflicting existing dependency declarations so each dependency has a unique name; existing declarations are not replaced."
                }
                (SourceOrigin::Argument { .. }, _) => {
                    "Choose a source with a different layer name; existing declarations are not replaced."
                }
                (SourceOrigin::Current, _) => {
                    "Correct the conflicting dependency declarations so each dependency has a unique name, different from the current layer; existing declarations are not replaced."
                }
            }.into());
            model
        }
    };
    if single && needs_origin_context(error, origin, failure) {
        model.details.insert(0, origin_label(origin, labels));
    }
    model
}

fn io_diagnostic(action: &str, path: &str, source: &io::Error, layer_manifest: bool) -> Diagnostic {
    let mut model = Diagnostic::new(format!("Could not {} '{path}'.", display(action)));
    model.details.push(format!("Cause: {}", display(source)));
    model.correction = Some(match source.kind() {
        io::ErrorKind::NotFound => "Check that the path and its parent directories exist, and correct the reference if it points to a missing location.",
        io::ErrorKind::PermissionDenied => "Check access permissions for the path and its parent directories before retrying.",
        io::ErrorKind::InvalidData if layer_manifest => "Check that the selected layer manifest is UTF-8 text and readable before retrying.",
        _ => "Check the reported path and operating-system cause before retrying.",
    }.into());
    model
}

fn layer_manifest(
    error: &LayerManifestError,
    origin: &SourceOrigin,
    failure: &AddFailure,
    single: bool,
) -> Diagnostic {
    match error {
        LayerManifestError::Missing { directory } => {
            let path = directory_text(directory, origin);
            let summary = if !single {
                "No layer manifest found.".into()
            } else if matches!(origin, SourceOrigin::Existing { .. }) {
                format!("No layer manifest found for existing dependency '{path}'.")
            } else {
                format!("No layer manifest found in '{path}'.")
            };
            let mut model = Diagnostic::new(summary);
            model.correction = Some(format!(
                "Choose an existing layer directory, or run 'hydragen init'\nin '{path}' to initialise it as a layer."
            ));
            model
        }
        LayerManifestError::Io {
            action,
            path,
            source,
        } => io_diagnostic(action, &selected_path(path, origin, failure), source, true),
        LayerManifestError::NotRegular { path } => {
            let mut model = Diagnostic::new(format!(
                "Could not read layer manifest '{}'.",
                selected_path(path, origin, failure)
            ));
            model.details.push(
                if matches!(origin, SourceOrigin::Current) {
                    "The selected current layer manifest must be a regular file, not a symlink."
                } else {
                    "The selected path must be a regular file (or a symlink to one)."
                }
                .into(),
            );
            model.correction = Some("Use a regular layer manifest file at the selected path. Lower-priority files and parent layers are not fallbacks.".into());
            model
        }
        LayerManifestError::Decode {
            path,
            format,
            message,
        } => {
            let mut model = Diagnostic::new(format!(
                "Could not decode {format} layer manifest '{}'.",
                selected_path(path, origin, failure)
            ));
            model.details.push(format!("Details: {}", display(message)));
            model.correction = Some(format!(
                "Correct the {format} syntax in the selected layer manifest. Lower-priority files and parent layers are not fallbacks."
            ));
            model
        }
        LayerManifestError::Metadata { path, source } => {
            metadata_diagnostic(source, &selected_path(path, origin, failure))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::Presentation;
    use hydragen::{LayerManifestCandidate, LayerManifestDiscovery, LayerManifestFormat};
    use std::path::PathBuf;
    use std::sync::Arc;

    fn argument(index: usize, value: &str) -> SourceOrigin {
        SourceOrigin::Argument {
            index,
            reference: value.into(),
        }
    }

    fn existing(index: usize, value: &str) -> SourceOrigin {
        SourceOrigin::Existing {
            index,
            reference: value.into(),
        }
    }

    fn failure(errors: Vec<(SourceOrigin, DependencyError)>) -> AddFailure {
        AddFailure {
            directory: Some("/work/app".into()),
            discoveries: vec![],
            errors: errors
                .into_iter()
                .map(|(origin, error)| DependencyFailure { origin, error })
                .collect(),
        }
    }

    fn body(origin: SourceOrigin, error: DependencyError) -> String {
        diagnostic(&failure(vec![(origin, error)]), Path::new("/work/app")).body()
    }

    fn missing() -> DependencyError {
        DependencyError::LayerManifest(Arc::new(LayerManifestError::Missing {
            directory: "/work/layer-b".into(),
        }))
    }

    fn not_directory() -> DependencyError {
        DependencyError::NotDirectory {
            path: "/work/not-directory".into(),
        }
    }

    fn assert_family(model: &Diagnostic) {
        let body = model.body();
        assert!(body.lines().next().unwrap().ends_with('.'), "{body}");
        assert_eq!(body.matches("No files were changed.").count(), 1, "{body}");
        assert!(!body.contains("\n\n\n"));
        assert!(!body.contains("source argument"), "{body}");
        assert!(!body.contains("\n1."), "{body}");
        for filler in [
            "Usage:",
            "Example:",
            "For more information",
            "Could not add dependencies (",
        ] {
            assert!(!body.contains(filler), "{body}");
        }
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
        assert!(
            plain.chars().all(|ch| !ch.is_control() || ch == '\n'),
            "{plain:?}"
        );
    }

    #[test]
    fn single_existing_failure_identifies_the_declaration_not_the_new_argument() {
        let text = body(existing(4, "../stored"), missing());
        assert!(
            text.starts_with("No layer manifest found for existing dependency '../stored'.\n"),
            "{text}"
        );
        assert!(text.contains("existing dependency '../stored'"));
        assert!(!text.contains("Source '"));
        assert!(!text.contains("declaration 5"));
        assert!(!text.contains("/work/layer-b"));
        assert!(text.contains("in '../stored' to initialise it as a layer."));
    }

    #[test]
    fn mixed_batch_preserves_all_failures_order_and_distinct_roles_without_outer_numbering() {
        let errors = failure(vec![
            (SourceOrigin::Current, not_directory()),
            (existing(0, "../same"), not_directory()),
            (existing(2, "../same"), not_directory()),
            (argument(0, "../same"), missing()),
            (
                argument(1, "bare-name"),
                DependencyError::Schema(schema::classify_reference("bare-name").unwrap_err()),
            ),
            (argument(3, "../same"), missing()),
        ]);
        let model = diagnostic(&errors, Path::new("/work/app"));
        assert_family(&model);
        assert_eq!(model.summary, "Dependency checks failed (6 errors).");
        assert_eq!(
            model
                .groups
                .iter()
                .map(|group| group.label.as_str())
                .collect::<Vec<_>>(),
            [
                "Current layer",
                "Existing dependency '../same' (declaration 1)",
                "Existing dependency '../same' (declaration 3)",
                "Source '../same' (argument 1)",
                "Source 'bare-name'",
                "Source '../same' (argument 4)",
            ]
        );
        let text = model.render(Presentation {
            decorated: false,
            basic: false,
        });
        assert!(
            text.contains(
                "\n  Source '../same' (argument 1)\n    No layer manifest found.\n    Tip:"
            ),
            "{text}"
        );
        assert_eq!(text.matches("No layer manifest found.").count(), 2);
        assert_eq!(text.matches("Invalid source reference.").count(), 1);
    }

    #[test]
    fn different_roles_do_not_need_positions_even_for_the_same_reference() {
        let errors = failure(vec![
            (existing(5, "../same"), missing()),
            (argument(9, "../same"), missing()),
        ]);
        let model = diagnostic(&errors, Path::new("/work/app"));
        assert_eq!(model.groups[0].label, "Existing dependency '../same'");
        assert_eq!(model.groups[1].label, "Source '../same'");
        assert_family(&model);
    }

    #[test]
    fn repeated_mentions_of_one_conflict_claim_do_not_create_positional_noise() {
        let errors = failure(
            (1..4)
                .map(|index| {
                    (
                        argument(index, &format!("../incoming-{index}")),
                        DependencyError::NameConflict {
                            name: "shared".into(),
                            previous: argument(0, "../first"),
                            previous_directory: Some("/work/first".into()),
                            directory: Some(format!("/work/incoming-{index}").into()),
                        },
                    )
                })
                .collect(),
        );
        let model = diagnostic(&errors, Path::new("/work/app"));
        assert_family(&model);
        let text = model.body();
        assert_eq!(
            text.matches("Conflicting source: Source '../first'")
                .count(),
            3
        );
        assert!(!text.contains("(argument"));
    }

    #[test]
    fn labels_that_collide_after_escaping_still_get_occurrence_positions() {
        let errors = failure(vec![
            (argument(0, "../raw\npath"), missing()),
            (argument(1, r"../raw\npath"), missing()),
        ]);
        let model = diagnostic(&errors, Path::new("/work/app"));
        assert_eq!(model.groups[0].label, r"Source '../raw\npath' (argument 1)");
        assert_eq!(model.groups[1].label, r"Source '../raw\npath' (argument 2)");
        assert_family(&model);
    }

    #[test]
    fn declaring_base_is_only_shown_for_relative_sources_when_not_known_equal_to_invocation() {
        for origin in [
            argument(0, "../layer-b"),
            existing(0, "./layers/b"),
            argument(0, "."),
            existing(0, ".."),
        ] {
            let errors = failure(vec![(origin, missing())]);
            let same = diagnostic(&errors, Path::new("/work/app")).body();
            assert!(!same.contains("Paths are relative"));
            for invocation in ["/work/app/nested", "/work/app-alias", "/work/app/."] {
                let text = diagnostic(&errors, Path::new(invocation)).body();
                // Path's lexical equality recognises a trailing '.', without any
                // filesystem work. Unknown aliases remain conservatively explicit.
                assert_eq!(
                    text.contains("Paths are relative to '/work/app'."),
                    Path::new(invocation) != Path::new("/work/app"),
                    "{text}"
                );
            }
        }
        for (origin, error) in [
            (argument(0, "/work/layer-b"), missing()),
            (
                argument(0, "published@1.2.3"),
                DependencyError::PublishedAdditionUnsupported {
                    name: "published".into(),
                    version: "1.2.3".into(),
                },
            ),
            (
                argument(0, "bare-name"),
                DependencyError::Schema(schema::classify_reference("bare-name").unwrap_err()),
            ),
            (
                argument(0, r"..\layer-b"),
                DependencyError::UnsupportedPath {
                    reference: r"..\layer-b".into(),
                    reason: "not native here",
                },
            ),
        ] {
            let text = diagnostic(
                &failure(vec![(origin, error)]),
                Path::new("/work/app/nested"),
            )
            .body();
            assert!(!text.contains("Paths are relative"), "{text}");
        }
    }

    #[test]
    fn conflict_base_accounts_for_previous_relative_local_claims_not_published_names() {
        for (previous, previous_directory, needed) in [
            (
                existing(0, "../stored"),
                Some(PathBuf::from("/work/stored")),
                true,
            ),
            (existing(0, "shared@1.2.3"), None, false),
            (
                SourceOrigin::Current,
                Some(PathBuf::from("/work/app")),
                false,
            ),
        ] {
            let errors = failure(vec![(
                argument(0, "/work/incoming"),
                DependencyError::NameConflict {
                    name: "shared".into(),
                    previous,
                    previous_directory,
                    directory: Some("/work/incoming".into()),
                },
            )]);
            let text = diagnostic(&errors, Path::new("/work/app/nested")).body();
            assert_eq!(
                text.contains("Paths are relative to '/work/app'."),
                needed,
                "{text}"
            );
        }
    }

    #[test]
    fn conservative_base_context_is_inert_and_does_not_require_a_real_directory() {
        let mut errors = failure(vec![(argument(0, "."), missing())]);
        let raw = "/no-such-base/配置\x1b[2J\n\u{202e}";
        errors.directory = Some(raw.into());
        let model = diagnostic(&errors, Path::new("/different-invocation"));
        assert_family(&model);
        let text = model.body();
        assert!(text.contains(&format!("Paths are relative to '{}'.", display(raw))));
        assert!(!text.contains('\u{202e}'));
        errors.directory = None;
        let text = diagnostic(&errors, Path::new("/different-invocation")).body();
        assert!(!text.contains("Paths are relative"));
    }

    #[test]
    fn selected_filename_uses_original_reference_only_when_the_selection_is_known() {
        let path = PathBuf::from("/work/real-layer/layer.jsonc");
        let origin = argument(0, "../alias/./layer");
        let mut errors = failure(vec![]);
        assert_eq!(
            selected_path(&path, &origin, &errors),
            "/work/real-layer/layer.jsonc"
        );
        errors.discoveries.push(LayerManifestDiscovery {
            directory: "/work/real-layer".into(),
            candidates: vec![LayerManifestCandidate {
                path: path.clone(),
                format: LayerManifestFormat::Jsonc,
            }],
        });
        assert_eq!(
            selected_path(&path, &origin, &errors),
            "../alias/./layer/layer.jsonc"
        );
        assert_eq!(
            selected_path(&path, &SourceOrigin::Current, &errors),
            "/work/real-layer/layer.jsonc"
        );
        assert_eq!(
            selected_path(Path::new("/elsewhere/layer.jsonc"), &origin, &errors),
            "/elsewhere/layer.jsonc"
        );
        // A known non-selected candidate is not a licence to claim it was loaded.
        errors.discoveries[0].candidates.insert(
            0,
            LayerManifestCandidate {
                path: "/work/real-layer/layer.yaml".into(),
                format: LayerManifestFormat::Yaml,
            },
        );
        assert_eq!(
            selected_path(&path, &origin, &errors),
            "/work/real-layer/layer.jsonc"
        );
    }

    #[test]
    fn decoder_details_are_inert_complete_and_never_have_paths_rewritten_inside_them() {
        for format in LayerManifestFormat::ALL {
            let path = Path::new("/work/layer-b").join(format.filename());
            let message = "line 7 column 3: /external/layer.yaml\nspoof\x1b[2J\u{202e}";
            let mut errors = failure(vec![(
                argument(0, "../layer-b"),
                DependencyError::LayerManifest(Arc::new(LayerManifestError::Decode {
                    path: path.clone(),
                    format,
                    message: message.into(),
                })),
            )]);
            errors.discoveries.push(LayerManifestDiscovery {
                directory: "/work/layer-b".into(),
                candidates: vec![LayerManifestCandidate { path, format }],
            });
            let model = diagnostic(&errors, Path::new("/work/app/nested"));
            let text = model.body();
            assert_family(&model);
            assert!(
                text.starts_with(&format!(
                    "Could not decode {format} layer manifest '../layer-b/{}'.",
                    format.filename()
                )),
                "{text}"
            );
            assert!(text.contains(&display(message).to_string()));
            assert!(text.contains("/external/layer.yaml"));
            assert!(text.contains(&format!("Tip: Correct the {format} syntax")));
            assert!(!text.contains("hydragen init"));
            assert!(text.contains("Lower-priority files and parent layers are not fallbacks."));
        }
    }

    #[test]
    fn io_recovery_distinguishes_missing_paths_permissions_encoding_and_opaque_causes() {
        for (kind, tip) in [
            (
                io::ErrorKind::NotFound,
                "Check that the path and its parent directories exist",
            ),
            (io::ErrorKind::PermissionDenied, "Check access permissions"),
            (io::ErrorKind::InvalidData, "UTF-8"),
            (io::ErrorKind::Other, "operating-system cause"),
        ] {
            for manifest in [false, true] {
                let source = io::Error::new(kind, "raw\x1b[2J\n/external/layer.yaml\u{202e}");
                let path = if manifest {
                    "/work/layer-b/layer.yaml"
                } else {
                    "/work/app/../layer-b"
                };
                let error = if manifest {
                    DependencyError::LayerManifest(Arc::new(LayerManifestError::Io {
                        action: "read layer manifest",
                        path: path.into(),
                        source,
                    }))
                } else {
                    DependencyError::Io {
                        action: "resolve local directory",
                        path: path.into(),
                        source,
                    }
                };
                let errors = failure(vec![(argument(0, "../layer-b"), error)]);
                let model = diagnostic(&errors, Path::new("/work/app"));
                assert_family(&model);
                let text = model.body();
                if !manifest && kind == io::ErrorKind::InvalidData {
                    assert!(text.contains("operating-system cause"));
                    assert!(!text.contains("UTF-8"));
                } else {
                    assert!(text.contains(tip), "{text}");
                }
                assert!(text.contains(r"raw\x1b[2J\n/external/layer.yaml\u{202e}"));
                assert!(!text.contains("hydragen init"));
                if !manifest {
                    assert!(
                        text.starts_with("Could not resolve local directory '../layer-b'."),
                        "{text}"
                    );
                }
            }
        }
    }

    #[test]
    fn canonical_io_identity_is_not_blindly_replaced_by_a_reference() {
        let text = body(
            argument(0, "../alias"),
            DependencyError::Io {
                action: "inspect local directory",
                path: "/other/real".into(),
                source: io::ErrorKind::PermissionDenied.into(),
            },
        );
        assert!(text.starts_with("Could not inspect local directory '/other/real'."));
        assert!(text.contains("Source '../alias'"));
    }

    #[test]
    fn not_directory_and_current_symlink_recovery_never_suggest_initialising() {
        for origin in [
            SourceOrigin::Current,
            argument(0, "../file"),
            existing(0, "../file"),
        ] {
            let current = matches!(origin, SourceOrigin::Current);
            let errors = failure(vec![(origin, not_directory())]);
            let model = diagnostic(&errors, Path::new("/work/app"));
            assert_family(&model);
            assert!(model.body().contains("not a directory"));
            if current {
                assert!(model.summary.starts_with("Current layer directory"));
                assert!(model.body().contains("or a directory below it"));
            } else {
                assert!(model.body().contains("not a file"));
            }
            assert!(!model.body().contains("hydragen init"));
        }
        let text = body(
            SourceOrigin::Current,
            DependencyError::CurrentSymlink {
                path: "/work/current/layer.yaml".into(),
            },
        );
        assert!(text.starts_with(
            "Cannot edit symlinked current layer manifest '/work/current/layer.yaml'."
        ));
        assert!(text.contains("Use a regular layer manifest file instead"));
        assert!(text.contains("the link and its target have not been changed"));
        assert!(!text.contains("hydragen init"));
    }

    #[test]
    fn selected_non_regular_path_has_read_only_symlink_guidance_and_exact_path() {
        let text = body(
            argument(0, "../dependency"),
            DependencyError::LayerManifest(Arc::new(LayerManifestError::NotRegular {
                path: "/work/dependency/layer.yaml".into(),
            })),
        );
        assert!(text.contains("'/work/dependency/layer.yaml'"));
        assert!(text.contains("regular file (or a symlink to one)"));
        assert!(!text.contains("hydragen init"));
        let text = body(
            SourceOrigin::Current,
            DependencyError::LayerManifest(Arc::new(LayerManifestError::NotRegular {
                path: "/work/current/layer.yaml".into(),
            })),
        );
        assert!(text.contains("must be a regular file, not a symlink"));
        assert!(!text.contains("or a symlink to one"));
    }

    #[test]
    fn unsupported_host_and_published_add_have_different_recovery_from_reference_validity() {
        let unsupported = body(
            argument(0, r"C:\layer"),
            DependencyError::UnsupportedPath {
                reference: r"C:\layer".into(),
                reason: "Windows drive or backslash paths are not native here",
            },
        );
        assert!(unsupported.starts_with(r"Cannot resolve local path 'C:\layer' on this host."));
        assert!(unsupported.contains("Windows drive or backslash paths are not native here"));
        assert!(unsupported.contains("explicit native path"));
        assert!(unsupported.contains("not expanded"));
        let published = body(
            argument(0, "@scope/layer@1.2.3-beta+build"),
            DependencyError::PublishedAdditionUnsupported {
                name: "@scope/layer".into(),
                version: "1.2.3-beta+build".into(),
            },
        );
        assert!(published.starts_with(
            "Adding published layer '@scope/layer@1.2.3-beta+build' is not supported yet."
        ));
        assert!(published.contains("does not access a registry"));
        for text in [unsupported, published] {
            assert!(!text.contains("hydragen init"));
            assert!(!text.contains("non-blank"));
        }
    }

    #[test]
    fn known_reference_oneof_uses_friendly_correction_without_internal_grammar() {
        let source = schema::classify_reference("bare-name").unwrap_err();
        let schema::Error::Content(violation) = &source else {
            panic!("expected content failure");
        };
        assert_eq!(violation.schema_path, "/$defs/reference/oneOf");
        assert_eq!(violation.keyword, "oneOf");
        assert_eq!(violation.instance_path, "");
        for (origin, kind) in [
            (argument(0, "bare-name"), "source"),
            (existing(2, "bare-name"), "dependency"),
        ] {
            let text = body(origin, DependencyError::Schema(source.clone()));
            assert!(
                text.starts_with(&format!("Invalid {kind} reference 'bare-name'.")),
                "{text}"
            );
            assert!(text.contains("./ or ../"));
            if kind == "source" {
                assert!(text.contains("Bare names are not local paths"));
                assert!(text.contains("published additions are not supported yet"));
            } else {
                assert!(text.contains("published reference with an exact version"));
                assert!(!text.contains("published additions are not supported"));
            }
            for internal in ["oneOf", "schemas listed", "Schema rule:"] {
                assert!(!text.contains(internal), "{text}");
            }
            assert!(!text.contains("non-blank"));
            assert!(!text.contains("--name"));
        }
        let text = body(SourceOrigin::Current, DependencyError::Schema(source));
        assert!(text.starts_with("Layer manifest schema validation failed."));
        assert!(text.contains("Schema rule: '/$defs/reference/oneOf'"));
        assert!(!text.contains("Use an explicit"));
        assert!(!text.contains("Invalid source reference"));
    }

    #[test]
    fn friendly_reference_failures_preserve_original_spelling_roles_groups_and_state() {
        for reference in [
            "bare-name",
            "~/layer-b",
            "$HOME/layer-b",
            "raw\x1b[2J\nsource",
        ] {
            let source = schema::classify_reference(reference).unwrap_err();
            let escaped = display(reference).to_string();
            for origin in [argument(0, reference), existing(2, reference)] {
                let declaration = matches!(origin, SourceOrigin::Existing { .. });
                let text = body(origin, DependencyError::Schema(source.clone()));
                assert!(
                    text.lines()
                        .next()
                        .unwrap()
                        .contains(&format!("'{escaped}'")),
                    "{text}"
                );
                if declaration {
                    assert!(text.contains(&format!("Tip: {DECLARED_REFERENCE_CORRECTION}")));
                    assert!(!text.contains("published additions are not supported"));
                } else {
                    assert!(text.contains(
                        "Tip: Use an explicit local path beginning with ./ or ../, or an absolute path."
                    ));
                    assert!(text.contains("published additions are not supported yet"));
                }
                for internal in ["oneOf", "schemas listed", "Schema rule:"] {
                    assert!(!text.contains(internal), "{text}");
                }
                assert_eq!(text.matches("No files were changed.").count(), 1);
                assert!(!text.contains('\x1b'));
            }
            let errors = failure(vec![
                (
                    existing(2, reference),
                    DependencyError::Schema(source.clone()),
                ),
                (argument(0, reference), DependencyError::Schema(source)),
            ]);
            let model = diagnostic(&errors, Path::new("/work/app/nested"));
            assert_family(&model);
            assert_eq!(
                model.groups[0].label,
                format!("Existing dependency '{escaped}'")
            );
            assert_eq!(model.groups[1].label, format!("Source '{escaped}'"));
            assert_eq!(
                model.groups[0].correction.as_deref(),
                Some(DECLARED_REFERENCE_CORRECTION)
            );
            assert!(
                model.groups[1]
                    .correction
                    .as_ref()
                    .unwrap()
                    .contains("published additions are not supported yet")
            );
            for internal in ["oneOf", "schemas listed", "Schema rule:"] {
                assert!(!model.body().contains(internal));
            }
        }
    }

    fn violation(kind: schema::ViolationKind) -> schema::Violation {
        schema::Violation {
            instance_path: "/dependencies/3".into(),
            schema_path: "/future/rule".into(),
            keyword: "rule".into(),
            message: "constraint detail".into(),
            kind,
        }
    }

    #[test]
    fn unfamiliar_schema_constraints_and_kinds_keep_fields_locations_and_rules() {
        for kind in [
            schema::ViolationKind::NonBlank,
            schema::ViolationKind::Name {
                guidance: "approved name guidance".into(),
            },
            schema::ViolationKind::Type,
            schema::ViolationKind::Required {
                property: "required-property".into(),
            },
            schema::ViolationKind::Other,
        ] {
            let violation = violation(kind);
            for source in [
                DependencyError::Schema(schema::Error::Content(Box::new(violation.clone()))),
                DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                    path: "/work/dependency/layer.json".into(),
                    source: MetadataError::Schema(schema::Error::Content(Box::new(
                        violation.clone(),
                    ))),
                })),
            ] {
                let model = diagnostic(
                    &failure(vec![(argument(0, "../dependency"), source)]),
                    Path::new("/work/app"),
                );
                assert_family(&model);
                let text = model.body();
                for fact in [
                    &violation.instance_path,
                    &violation.schema_path,
                    &violation.keyword,
                    &violation.message,
                ] {
                    assert!(text.contains(fact), "{fact}: {text}");
                }
                assert!(!text.contains("Bare names are not local paths"));
                assert!(!text.contains("hydragen init"));
            }
        }
    }

    #[test]
    fn reference_correction_requires_the_complete_known_schema_location() {
        let known = schema::Violation {
            instance_path: "".into(),
            schema_path: "/$defs/reference/oneOf".into(),
            keyword: "oneOf".into(),
            message: "constraint".into(),
            kind: schema::ViolationKind::Other,
        };
        for field in ["instance", "schema", "keyword"] {
            let mut violation = known.clone();
            match field {
                "instance" => violation.instance_path = "/dependencies/3".into(),
                "schema" => violation.schema_path = "/future/oneOf".into(),
                _ => violation.keyword = "pattern".into(),
            }
            let text = body(
                argument(0, "input"),
                DependencyError::Schema(schema::Error::Content(Box::new(violation.clone()))),
            );
            assert!(text.contains(&violation.to_string()));
            assert!(text.contains(&violation.schema_path));
            assert!(!text.contains("Use an explicit"));
        }
    }

    #[test]
    fn setup_failures_preserve_optional_detail_without_blaming_valid_input() {
        for detail in [
            None,
            Some(Box::new(violation(schema::ViolationKind::Other))),
        ] {
            let setup = schema::Error::Setup {
                message: "internal setup failure".into(),
                detail: detail.clone(),
            };
            for source in [
                DependencyError::Schema(setup.clone()),
                DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                    path: "/work/dependency/layer.yaml".into(),
                    source: MetadataError::Schema(setup.clone()),
                })),
            ] {
                let text = body(argument(0, "../valid"), source);
                assert!(text.starts_with("Internal layer manifest schema error."));
                assert!(text.contains("internal setup failure"));
                assert!(text.contains("Report this internal error"));
                for wrong in [
                    "Invalid source",
                    "Correct",
                    "Use an explicit",
                    "--name",
                    "hydragen init",
                    "retry",
                ] {
                    assert!(!text.contains(wrong), "{text}");
                }
                if let Some(detail) = &detail {
                    assert!(text.contains(&detail.to_string()));
                    assert!(text.contains(&detail.schema_path));
                }
            }
        }
    }

    #[test]
    fn internal_setup_detail_is_not_hidden_even_when_it_names_the_known_reference_rule() {
        let detail = schema::Violation {
            instance_path: "".into(),
            schema_path: "/$defs/reference/oneOf".into(),
            keyword: "oneOf".into(),
            message: "schema compilation constraint detail".into(),
            kind: schema::ViolationKind::Other,
        };
        let text = body(
            argument(0, "../valid"),
            DependencyError::Schema(schema::Error::Setup {
                message: "internal setup failure".into(),
                detail: Some(Box::new(detail.clone())),
            }),
        );
        assert!(text.starts_with("Internal layer manifest schema error."));
        assert!(text.contains(&detail.to_string()));
        assert!(text.contains("Schema rule: '/$defs/reference/oneOf'"));
        assert!(text.contains("Report this internal error"));
        assert!(!text.contains("Use an explicit"));
    }

    #[test]
    fn metadata_variants_keep_field_and_constraint_specific_recovery() {
        for (source, detail, tip) in [
            (
                MetadataError::NotObject,
                "one object/mapping document",
                "Use one object/mapping document",
            ),
            (
                MetadataError::MissingField { field: "name" },
                "Missing required metadata field 'name'",
                "Add the required 'name' field",
            ),
            (
                MetadataError::NotString { field: "author" },
                "null is not omission",
                "Set 'author' to a string",
            ),
            (
                MetadataError::Blank {
                    field: "description",
                },
                "at least one non-whitespace character",
                "Provide a non-blank 'description' string",
            ),
            (
                MetadataError::InvalidName {
                    guidance: "approved lowercase policy".into(),
                },
                "approved lowercase policy",
                "Correct the 'name' field",
            ),
        ] {
            for origin in [
                SourceOrigin::Current,
                existing(0, "../stored"),
                argument(0, "../source"),
            ] {
                let errors = failure(vec![(
                    origin,
                    DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                        path: "/work/dependency/layer.yaml".into(),
                        source: source.clone(),
                    })),
                )]);
                let model = diagnostic(&errors, Path::new("/work/app"));
                assert_family(&model);
                let text = model.body();
                assert!(text.starts_with("Invalid layer manifest '/work/dependency/layer.yaml'."));
                assert!(text.contains(detail));
                assert!(text.contains(&format!("Tip: {tip}")), "{text}");
                assert!(!text.contains("hydragen init"));
                assert!(!text.contains("--name"));
            }
        }
    }

    #[test]
    fn conflicts_preserve_previous_roles_and_all_available_identity_paths() {
        for (previous, previous_label) in [
            (SourceOrigin::Current, "Current layer path"),
            (existing(2, "../stored-reference"), "Existing source path"),
            (argument(1, "../first-input"), "Earlier source path"),
        ] {
            for (origin, incoming_label) in [
                (argument(3, "../incoming"), "Requested source path"),
                (existing(3, "../incoming"), "Declared source path"),
            ] {
                for previous_directory in [None, Some(PathBuf::from("/work/previous/source"))] {
                    let errors = failure(vec![(
                        origin.clone(),
                        DependencyError::NameConflict {
                            name: "@example/shared-config".into(),
                            previous: previous.clone(),
                            previous_directory: previous_directory.clone(),
                            directory: Some("/work/requested/source".into()),
                        },
                    )]);
                    let model = diagnostic(&errors, Path::new("/work/app/nested"));
                    assert_family(&model);
                    let text = model.body();
                    assert!(
                        text.starts_with(
                            "Layer name '@example/shared-config' is already in use.\n"
                        )
                    );
                    assert!(text.contains(&format!(
                        "Conflicting source: {}",
                        origin_label(&previous, &ambiguous_labels(&errors))
                    )));
                    if let Some(path) = previous_directory {
                        assert!(text.contains(&format!("{previous_label}: {}", path.display())));
                    } else {
                        assert!(!text.contains(previous_label));
                    }
                    assert!(text.contains(&format!("{incoming_label}: /work/requested/source")));
                    assert!(text.contains("unique name, different from the current layer"));
                    assert!(text.contains("existing declarations are not replaced"));
                    assert!(!text.contains("hydragen upgrade"));
                }
            }
        }
    }

    #[test]
    fn ambiguous_conflict_claims_have_positions_but_same_identity_is_not_called_different() {
        let errors = failure(vec![(
            existing(1, "../same"),
            DependencyError::NameConflict {
                name: "shared".into(),
                previous: existing(0, "../same"),
                previous_directory: Some("/work/same".into()),
                directory: Some("/work/same".into()),
            },
        )]);
        let text = diagnostic(&errors, Path::new("/work/app/nested")).body();
        assert!(text.contains("Existing dependency '../same' (declaration 2)"));
        assert!(text.contains("Conflicting source: Existing dependency '../same' (declaration 1)"));
        assert_eq!(
            text.matches("Both declarations refer to the same layer directory.")
                .count(),
            1
        );
        assert!(!text.contains("Existing source path:"));
        assert!(!text.contains("Declared source path:"));
        assert!(!text.contains("/work/same"));
        assert!(!text.contains("different source"));
    }

    #[test]
    fn every_untrusted_typed_leaf_is_escaped_without_debug_field_dumps() {
        let raw = "raw\x1b[2J\n\r\t\u{9b}\u{202e}";
        let safe = display(raw).to_string();
        let mut bad_violation = violation(schema::ViolationKind::Required {
            property: raw.into(),
        });
        bad_violation.instance_path = raw.into();
        bad_violation.schema_path = raw.into();
        bad_violation.keyword = raw.into();
        bad_violation.message = raw.into();
        let typed = vec![
            DependencyError::Schema(schema::Error::Setup {
                message: raw.into(),
                detail: Some(Box::new(bad_violation.clone())),
            }),
            DependencyError::Schema(schema::Error::Content(Box::new(bad_violation))),
            DependencyError::LayerManifest(Arc::new(LayerManifestError::Decode {
                path: Path::new(raw).join("layer.yaml"),
                format: LayerManifestFormat::Yaml,
                message: raw.into(),
            })),
            DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                path: raw.into(),
                source: MetadataError::InvalidName {
                    guidance: raw.into(),
                },
            })),
            DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                path: raw.into(),
                source: MetadataError::MissingField {
                    field: "raw\x1b[2J\n\r\t\u{9b}\u{202e}",
                },
            })),
            DependencyError::Io {
                action: "raw\x1b[2J\n\r\t\u{9b}\u{202e}",
                path: raw.into(),
                source: io::Error::other(raw.to_owned()),
            },
            DependencyError::NotDirectory { path: raw.into() },
            DependencyError::MissingCurrent {
                invocation: raw.into(),
            },
            DependencyError::CurrentSymlink { path: raw.into() },
            DependencyError::UnsupportedPath {
                reference: raw.into(),
                reason: "raw\x1b[2J\n\r\t\u{9b}\u{202e}",
            },
            DependencyError::PublishedAdditionUnsupported {
                name: raw.into(),
                version: raw.into(),
            },
            DependencyError::NameConflict {
                name: raw.into(),
                previous: existing(0, raw),
                previous_directory: Some(raw.into()),
                directory: Some(raw.into()),
            },
        ];
        for error in typed {
            let errors = failure(vec![(argument(0, raw), error)]);
            let model = diagnostic(&errors, Path::new("/work/app"));
            assert_family(&model);
            let text = model.body();
            assert!(text.contains(&safe), "{text}");
            assert!(!text.contains('\u{202e}'));
            for debug in [
                "SourceOrigin",
                "DependencyError",
                "instance_path:",
                "schema_path:",
                "Setup {",
                "Content(",
            ] {
                assert!(!text.contains(debug), "{text}");
            }
        }
    }

    #[test]
    fn current_prerequisites_are_direct_keep_selected_paths_and_do_not_fall_back() {
        use std::fs;
        for (contents, expected) in [
            ("[", "Could not decode yaml layer manifest"),
            ("name: Uppercase\n", "Invalid layer manifest"),
        ] {
            let root = tempfile::tempdir().unwrap();
            let app = root.path().join("app");
            fs::create_dir(&app).unwrap();
            fs::write(root.path().join("layer.yaml"), "name: parent\n").unwrap();
            let selected = app.join("layer.yaml");
            fs::write(&selected, contents).unwrap();
            fs::write(app.join("layer.json"), r#"{"name":"fallback"}"#).unwrap();
            let errors =
                hydragen::dependencies::prepare_add(&app, &["../unused".into()]).unwrap_err();
            assert_eq!(errors.errors.len(), 1);
            assert!(matches!(errors.errors[0].origin, SourceOrigin::Current));
            let model = diagnostic(&errors, &app);
            assert_family(&model);
            let text = model.body();
            assert!(text.starts_with(expected), "{text}");
            assert!(text.contains(&display(selected.display()).to_string()));
            assert!(!text.contains("Dependency checks failed"));
            assert!(!text.contains("Source '../unused'"));
            assert!(!text.contains("hydragen init"));
            assert_eq!(fs::read_to_string(&selected).unwrap(), contents);
            assert_eq!(errors.discoveries.len(), 1);
            assert!(errors.discoveries[0].has_duplicates());
        }
    }

    #[test]
    fn loaded_dependency_reference_failure_is_friendly_and_keeps_file_and_instance_location() {
        use std::fs;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("layer.yaml");
        let original = "name: app\ndependencies: [bare-name]\n";
        fs::write(&path, original).unwrap();
        let errors =
            hydragen::dependencies::prepare_add(root.path(), &["../unused".into()]).unwrap_err();
        assert_eq!(errors.errors.len(), 1);
        assert!(matches!(errors.errors[0].origin, SourceOrigin::Current));
        let model = diagnostic(&errors, root.path());
        assert_family(&model);
        let text = model.body();
        assert!(
            text.starts_with("Invalid dependency reference in layer manifest '"),
            "{text}"
        );
        assert!(text.contains(&display(path.canonicalize().unwrap().display()).to_string()));
        assert!(text.contains("Field: /dependencies/0"));
        assert!(text.contains("./ or ../"));
        assert!(text.contains("absolute path"));
        assert!(text.contains("published reference with an exact version"));
        assert!(text.contains("'@scope/shared-config@1.2.3'"));
        for wrong in [
            "oneOf",
            "schemas listed",
            "Schema rule:",
            "published additions are not supported",
            "Invalid source reference",
            "../unused",
        ] {
            assert!(!text.contains(wrong), "{text}");
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
    }

    #[test]
    fn declared_reference_guidance_matches_schema_accepted_published_versions_without_registry_access()
     {
        use std::fs;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("layer.yaml");
        for reference in [
            "shared-config@1.2.3",
            "@scope/shared-config@1.2.3",
            "@scope/shared-config@1.2.3-beta.1+build.02",
        ] {
            let original = format!("name: app\ndependencies: ['{reference}']\n");
            fs::write(&path, &original).unwrap();
            let plan = hydragen::dependencies::prepare_add(root.path(), &[]).unwrap();
            assert!(plan.is_noop());
            assert_eq!(plan.existing()[0].reference, reference);
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
        }
    }

    #[test]
    fn loaded_reference_explanation_requires_the_exact_rule_and_direct_item_location() {
        let known = schema::Violation {
            instance_path: "/dependencies/0".into(),
            schema_path: "/$defs/reference/oneOf".into(),
            keyword: "oneOf".into(),
            message: "opaque constraint detail".into(),
            kind: schema::ViolationKind::Other,
        };
        for (field, value) in [
            ("schema", "/future/oneOf"),
            ("schema", "/properties/dependencies/items/type"),
            ("keyword", "type"),
            ("keyword", "pattern"),
            ("instance", ""),
            ("instance", "/dependencies"),
            ("instance", "/dependencies/"),
            ("instance", "/dependencies/0/name"),
            ("instance", "/other/0"),
            ("instance", "/dependencies/-1"),
            ("instance", "/dependencies/01"),
            ("instance", "/dependencies/not-an-index"),
        ] {
            let mut violation = known.clone();
            match field {
                "schema" => violation.schema_path = value.into(),
                "keyword" => violation.keyword = value.into(),
                _ => violation.instance_path = value.into(),
            }
            let text = body(
                SourceOrigin::Current,
                DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                    path: "/work/app/layer.yaml".into(),
                    source: MetadataError::Schema(schema::Error::Content(Box::new(
                        violation.clone(),
                    ))),
                })),
            );
            assert!(
                text.starts_with("Invalid layer manifest '/work/app/layer.yaml'."),
                "{field} {value}: {text}"
            );
            assert!(text.contains(&violation.to_string()), "{text}");
            assert!(text.contains(&format!("Schema rule: '{}'", violation.schema_path)));
            assert!(!text.contains(DECLARED_REFERENCE_CORRECTION));
        }
        let text = body(
            SourceOrigin::Current,
            DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                path: "/work/app/layer.yaml".into(),
                source: MetadataError::Schema(schema::Error::Setup {
                    message: "schema setup detail".into(),
                    detail: Some(Box::new(known.clone())),
                }),
            })),
        );
        assert!(text.starts_with("Internal layer manifest schema error."));
        assert!(text.contains("Layer manifest: '/work/app/layer.yaml'"));
        assert!(text.contains(&known.to_string()));
        assert!(text.contains("Schema rule: '/$defs/reference/oneOf'"));
        assert!(text.contains("schema setup detail"));
        assert!(text.contains("Report this internal error"));
        assert!(!text.contains(DECLARED_REFERENCE_CORRECTION));
    }

    #[test]
    fn loaded_reference_locations_preserve_indices_and_dependency_file_context() {
        let known = schema::Violation {
            instance_path: "/dependencies/12".into(),
            schema_path: "/$defs/reference/oneOf".into(),
            keyword: "oneOf".into(),
            message: "opaque value not available as a typed reference".into(),
            kind: schema::ViolationKind::Other,
        };
        let path = PathBuf::from("/work/dependency/layer.yaml");
        let mut errors = failure(vec![(
            argument(0, "../alias/./dependency"),
            DependencyError::LayerManifest(Arc::new(LayerManifestError::Metadata {
                path: path.clone(),
                source: MetadataError::Schema(schema::Error::Content(Box::new(known))),
            })),
        )]);
        errors.discoveries.push(LayerManifestDiscovery {
            directory: "/work/dependency".into(),
            candidates: vec![LayerManifestCandidate {
                path,
                format: LayerManifestFormat::Yaml,
            }],
        });
        let model = diagnostic(&errors, Path::new("/work/app/nested"));
        assert_family(&model);
        let text = model.body();
        assert!(text.starts_with(
            "Invalid dependency reference in layer manifest '../alias/./dependency/layer.yaml'."
        ));
        assert!(text.contains("Paths are relative to '/work/app'."));
        assert!(text.contains("Field: /dependencies/12"));
        assert!(text.contains(DECLARED_REFERENCE_CORRECTION));
        assert!(!text.contains("opaque value"));
        assert!(!text.contains("Schema rule:"));
    }

    #[test]
    fn duplicate_existing_declarations_recommend_removing_the_redundant_declaration() {
        use std::fs;
        for second in ["../dep", "../dep/."] {
            let root = tempfile::tempdir().unwrap();
            let app = root.path().join("app");
            let dep = root.path().join("dep");
            fs::create_dir(&app).unwrap();
            fs::create_dir(&dep).unwrap();
            fs::write(dep.join("layer.yaml"), "name: shared\n").unwrap();
            let original = format!("name: app\ndependencies: [../dep, {second}]\n");
            fs::write(app.join("layer.yaml"), &original).unwrap();
            let errors = hydragen::dependencies::prepare_add(&app, &[]).unwrap_err();
            assert_eq!(errors.errors.len(), 1);
            let DependencyError::NameConflict {
                previous,
                previous_directory,
                directory,
                ..
            } = &errors.errors[0].error
            else {
                panic!("expected a name conflict");
            };
            assert!(matches!(previous, SourceOrigin::Existing { index: 0, .. }));
            let identity = dep.canonicalize().unwrap();
            assert_eq!(previous_directory.as_deref(), Some(identity.as_path()));
            assert_eq!(directory.as_deref(), Some(identity.as_path()));
            let model = diagnostic(&errors, &app);
            assert_family(&model);
            let text = model.body();
            assert!(text.contains("Tip: Remove the redundant existing dependency declaration from the current layer manifest"), "{text}");
            assert!(!text.contains("Choose a source"), "{text}");
            assert_eq!(
                text.matches("Both declarations refer to the same layer directory.")
                    .count(),
                1
            );
            assert!(!text.contains("Existing source path:"));
            assert!(!text.contains("Declared source path:"));
            assert!(!text.contains(&display(identity.display()).to_string()));
            if second == "../dep" {
                assert!(text.contains("Existing dependency '../dep' (declaration 2)"));
                assert!(
                    text.contains(
                        "Conflicting source: Existing dependency '../dep' (declaration 1)"
                    )
                );
            } else {
                assert!(text.contains("Existing dependency '../dep/.'"));
                assert!(!text.contains("(declaration"));
            }
            assert_eq!(
                fs::read_to_string(app.join("layer.yaml")).unwrap(),
                original
            );
        }
    }

    #[test]
    fn actual_mixed_conflicts_recover_existing_declarations_and_supplied_sources_differently_in_order()
     {
        use std::fs;
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir(&app).unwrap();
        for name in ["dep", "other", "third"] {
            let directory = root.path().join(name);
            fs::create_dir(&directory).unwrap();
            fs::write(directory.join("layer.yaml"), "name: shared\n").unwrap();
        }
        let original = "name: app\ndependencies: [../dep, ../dep, ../other]\n";
        fs::write(app.join("layer.yaml"), original).unwrap();
        let errors =
            hydragen::dependencies::prepare_add(&app, &["../third".into(), "../missing".into()])
                .unwrap_err();
        assert_eq!(errors.errors.len(), 4);
        for (problem, directory) in errors.errors.iter().take(3).zip(["dep", "other", "third"]) {
            let DependencyError::NameConflict {
                previous_directory,
                directory: incoming,
                ..
            } = &problem.error
            else {
                panic!("expected name conflict");
            };
            assert_eq!(
                previous_directory.as_deref(),
                Some(root.path().join("dep").canonicalize().unwrap().as_path())
            );
            assert_eq!(
                incoming.as_deref(),
                Some(
                    root.path()
                        .join(directory)
                        .canonicalize()
                        .unwrap()
                        .as_path()
                )
            );
        }
        let model = diagnostic(&errors, &app);
        assert_family(&model);
        assert_eq!(model.summary, "Dependency checks failed (4 errors).");
        assert_eq!(
            model
                .groups
                .iter()
                .map(|group| group.label.as_str())
                .collect::<Vec<_>>(),
            [
                "Existing dependency '../dep' (declaration 2)",
                "Existing dependency '../other'",
                "Source '../third'",
                "Source '../missing'",
            ]
        );
        assert!(
            model.groups[0]
                .correction
                .as_ref()
                .unwrap()
                .starts_with("Remove the redundant existing dependency declaration")
        );
        assert!(
            model.groups[1]
                .correction
                .as_ref()
                .unwrap()
                .starts_with("Remove or correct the conflicting existing dependency declarations")
        );
        assert!(
            model.groups[2]
                .correction
                .as_ref()
                .unwrap()
                .starts_with("Choose a source with a different layer name")
        );
        let text = model.body();
        assert_eq!(
            text.matches("Layer name 'shared' is already in use.")
                .count(),
            3
        );
        assert_eq!(
            text.matches("Both declarations refer to the same layer directory.")
                .count(),
            1
        );
        assert_eq!(text.matches("Existing source path:").count(), 2);
        assert_eq!(text.matches("Declared source path:").count(), 1);
        assert_eq!(text.matches("Requested source path:").count(), 1);
        assert_eq!(
            fs::read_to_string(app.join("layer.yaml")).unwrap(),
            original
        );
    }

    #[test]
    fn actual_existing_self_name_conflicts_require_correcting_the_existing_declaration() {
        use std::fs;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("layer.yaml");
        for reference in [".", "app@1.2.3"] {
            let original = format!("name: app\ndependencies: ['{reference}']\n");
            fs::write(&path, &original).unwrap();
            let errors = hydragen::dependencies::prepare_add(root.path(), &[]).unwrap_err();
            assert_eq!(errors.errors.len(), 1);
            let DependencyError::NameConflict { previous, .. } = &errors.errors[0].error else {
                panic!("expected name conflict");
            };
            assert!(matches!(previous, SourceOrigin::Current));
            let model = diagnostic(&errors, root.path());
            assert_family(&model);
            let text = model.body();
            assert!(text.contains("Tip: Remove or correct the existing dependency declaration that uses the current layer's reserved name"), "{text}");
            assert!(text.contains(&format!("Existing dependency '{reference}'")));
            assert!(text.contains("Conflicting source: Current layer"));
            assert!(text.contains(&format!(
                "Current layer path: {}",
                display(root.path().canonicalize().unwrap().display())
            )));
            assert!(!text.contains("Choose a source"));
            assert!(!text.contains("redundant existing dependency declaration"));
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
        }
    }

    #[test]
    fn missing_identity_evidence_does_not_claim_an_existing_declaration_is_redundant() {
        for (previous_directory, directory) in [
            (None, None),
            (Some(PathBuf::from("/work/same")), None),
            (None, Some(PathBuf::from("/work/same"))),
        ] {
            let previous_path = previous_directory
                .as_ref()
                .map(|path| format!("Existing source path: {}", display(path.display())));
            let incoming_path = directory
                .as_ref()
                .map(|path| format!("Declared source path: {}", display(path.display())));
            let text = body(
                existing(1, "shared@1.2.3"),
                DependencyError::NameConflict {
                    name: "shared".into(),
                    previous: existing(0, "shared@1.2.3"),
                    previous_directory,
                    directory,
                },
            );
            assert!(
                text.contains("Remove or correct the conflicting existing dependency declarations")
            );
            assert!(!text.contains("redundant existing dependency declaration"));
            assert!(!text.contains("Both declarations refer to the same layer directory."));
            if let Some(path) = previous_path {
                assert!(text.contains(&path));
            } else {
                assert!(!text.contains("Existing source path:"));
            }
            if let Some(path) = incoming_path {
                assert!(text.contains(&path));
            } else {
                assert!(!text.contains("Declared source path:"));
            }
            assert!(!text.contains("Choose a source"));
            assert!(text.contains("Existing dependency 'shared@1.2.3' (declaration 2)"));
            assert!(text.contains(
                "Conflicting source: Existing dependency 'shared@1.2.3' (declaration 1)"
            ));
        }
    }

    #[test]
    fn different_typed_directory_identities_keep_both_paths_even_when_their_escaped_text_collides()
    {
        for (previous, incoming) in [
            (PathBuf::from("/work/first"), PathBuf::from("/work/second")),
            (
                PathBuf::from("/work/raw\npath"),
                PathBuf::from(r"/work/raw\npath"),
            ),
        ] {
            assert_ne!(previous, incoming);
            let previous_text = display(previous.display()).to_string();
            let incoming_text = display(incoming.display()).to_string();
            if previous_text.contains(r"\n") {
                assert_eq!(
                    previous_text, incoming_text,
                    "exercise a visible escaping collision"
                );
            }
            let errors = failure(vec![(
                existing(1, "../incoming"),
                DependencyError::NameConflict {
                    name: "shared".into(),
                    previous: existing(0, "../previous"),
                    previous_directory: Some(previous),
                    directory: Some(incoming),
                },
            )]);
            let model = diagnostic(&errors, Path::new("/work/app"));
            assert_family(&model);
            let text = model.body();
            assert!(text.contains(&format!("Existing source path: {previous_text}")));
            assert!(text.contains(&format!("Declared source path: {incoming_text}")));
            assert_eq!(text.matches("Existing source path:").count(), 1);
            assert_eq!(text.matches("Declared source path:").count(), 1);
            assert!(!text.contains("Both declarations refer to the same layer directory."));
            assert!(!text.contains("redundant existing dependency declaration"));
            assert!(
                text.contains("Remove or correct the conflicting existing dependency declarations")
            );
        }
    }

    #[test]
    fn equal_identity_paths_are_not_collapsed_for_current_or_supplied_source_conflicts() {
        for (origin, previous, previous_label, incoming_label) in [
            (
                existing(0, "."),
                SourceOrigin::Current,
                "Current layer path",
                "Declared source path",
            ),
            (
                argument(0, "."),
                SourceOrigin::Current,
                "Current layer path",
                "Requested source path",
            ),
            (
                argument(1, "../alias"),
                existing(0, "../stored"),
                "Existing source path",
                "Requested source path",
            ),
        ] {
            let text = body(
                origin,
                DependencyError::NameConflict {
                    name: "shared".into(),
                    previous,
                    previous_directory: Some("/work/same".into()),
                    directory: Some("/work/same".into()),
                },
            );
            assert!(text.contains(&format!("{previous_label}: /work/same")));
            assert!(text.contains(&format!("{incoming_label}: /work/same")));
            assert_eq!(text.matches("/work/same").count(), 2);
            assert!(!text.contains("Both declarations refer to the same layer directory."));
            assert!(!text.contains("redundant existing dependency declaration"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn dependency_symlink_and_alias_noop_policies_remain_unchanged_but_current_symlink_is_actionable()
     {
        use std::fs;
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        let dependency = root.path().join("dependency");
        fs::create_dir(&app).unwrap();
        fs::create_dir(&dependency).unwrap();
        let original = "name: app\ndependencies: [../dependency]\n";
        fs::write(app.join("layer.yaml"), original).unwrap();
        fs::write(root.path().join("shared.yaml"), "name: shared\n").unwrap();
        symlink(
            root.path().join("shared.yaml"),
            dependency.join("layer.yaml"),
        )
        .unwrap();
        symlink(&dependency, root.path().join("alias")).unwrap();
        let plan =
            hydragen::dependencies::prepare_add(&app, &["../alias".into(), "../dependency".into()])
                .unwrap();
        assert!(plan.is_noop());
        assert_eq!(plan.already_present().len(), 1);
        assert_eq!(plan.already_present()[0].reference, "../dependency");
        assert_eq!(
            fs::read_to_string(app.join("layer.yaml")).unwrap(),
            original
        );
        let linked = root.path().join("linked");
        fs::create_dir(&linked).unwrap();
        symlink(app.join("layer.yaml"), linked.join("layer.yaml")).unwrap();
        let errors =
            hydragen::dependencies::prepare_add(&linked, &["../dependency".into()]).unwrap_err();
        let text = diagnostic(&errors, &linked).body();
        assert!(text.contains(&display(linked.join("layer.yaml").display()).to_string()));
        assert!(text.contains("Use a regular layer manifest file instead"));
        assert_eq!(
            fs::read_to_string(app.join("layer.yaml")).unwrap(),
            original
        );
    }
}
