use std::io::{self, Write};

use hydragen::{CreatedManifest, InitCreationError, InitError, ManifestDiscovery, ManifestFormat};

use crate::presentation::{Presentation, Severity};

pub fn init_error_message(error: &(dyn std::error::Error + 'static)) -> String {
    let init_error = error.downcast_ref::<InitError>().or_else(|| {
        error
            .downcast_ref::<InitCreationError>()
            .map(|failure| &failure.error)
    });
    if let Some(InitError::ExistingManifest { path }) = init_error
        && let Some(filename) = path.file_name()
    {
        return format!(
            "{} already exists in the current directory.\n'hydragen init' does not overwrite existing entries.\nInspect the entry or run 'hydragen init' in a different directory.\nThis command did not change any files.",
            std::path::Path::new(filename).display()
        );
    }
    error.to_string()
}

/// Shared by init and future manifest readers, before refusing or loading a candidate.
fn duplicate_warning(discovery: &ManifestDiscovery) -> Option<String> {
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
    let priority = ManifestFormat::ALL
        .map(ManifestFormat::filename)
        .join(" > ");
    Some(format!(
        "Warning: Multiple layer manifests found in '{}': {names}.\n{selected} takes priority (priority: {priority}).\nManifests are not merged; keep only the intended manifest.",
        discovery.directory.display()
    ))
}

pub fn warn_duplicates(discovery: &ManifestDiscovery, presentation: Presentation) {
    if let Some(warning) = duplicate_warning(discovery) {
        // Diagnostic delivery must not turn a completed write into a failure.
        report_warning(&warning, presentation);
    }
}

fn observation_warning(created: &CreatedManifest) -> Option<String> {
    match &created.observation {
        Ok(discovery) => duplicate_warning(discovery),
        Err(error) => Some(format!(
            "Warning: Manifest '{}' was created, but duplicate checking failed:\n{error}",
            created.path.display()
        )),
    }
}

pub fn report_observation(created: &CreatedManifest, presentation: Presentation) {
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
    fn current_directory_collisions_name_the_file_for_preflight_and_creation() {
        for format in ManifestFormat::ALL {
            let path = std::path::Path::new("/a/long/project/directory").join(format.filename());
            let error = InitError::ExistingManifest { path: path.clone() };
            let message = init_error_message(&error);
            assert_eq!(
                message.lines().next().unwrap(),
                format!(
                    "{} already exists in the current directory.",
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
                action: "inspect manifest path",
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
        let created = CreatedManifest {
            path: path.clone(),
            observation: Err(hydragen::ManifestError::Io {
                action: "inspect manifest path",
                path,
                source: io::ErrorKind::PermissionDenied.into(),
            }),
        };
        let warning = observation_warning(&created).unwrap();
        assert!(warning.contains("Manifest '/project/layer.jsonc' was created"));
        assert!(warning.contains("duplicate checking failed"));
        assert!(!warning.contains("No files were changed"));
    }

    #[test]
    fn successful_observation_warns_about_priority_not_the_created_format() {
        for created_format in [ManifestFormat::Yaml, ManifestFormat::Json] {
            let directory = tempfile::tempdir().unwrap();
            let prepared = hydragen::inspect_init(directory.path())
                .unwrap()
                .prepare()
                .unwrap();
            let appeared = if created_format == ManifestFormat::Yaml {
                ManifestFormat::Json
            } else {
                ManifestFormat::Yaml
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
