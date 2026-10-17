use std::io::{self, Write};

use hydragen::{CreatedLayerManifest, LayerManifestDiscovery, LayerManifestFormat};

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
        "Warning: Multiple layer manifests found in '{}': {names}. {selected} takes priority (priority: {priority}). Layer manifests are not merged; keep only the intended layer manifest.",
        discovery.directory.display()
    ))
}

pub fn warn_duplicates(discovery: &LayerManifestDiscovery) {
    if let Some(warning) = duplicate_warning(discovery) {
        // Diagnostic delivery must not turn a completed write into a failure.
        let _ = writeln!(io::stderr().lock(), "{warning}");
    }
}

fn observation_warning(created: &CreatedLayerManifest) -> Option<String> {
    match &created.observation {
        Ok(discovery) => duplicate_warning(discovery),
        Err(error) => Some(format!(
            "Warning: Layer manifest '{}' was created, but duplicate checking failed: {error}",
            created.path.display()
        )),
    }
}

pub fn report_observation(created: &CreatedLayerManifest) {
    if let Some(warning) = observation_warning(created) {
        let _ = writeln!(io::stderr().lock(), "{warning}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
        assert!(output.contains("layer.yaml, layer.json. layer.yaml takes priority"));
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
            assert!(warning.contains("layer.yaml, layer.json. layer.yaml takes priority"));
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
