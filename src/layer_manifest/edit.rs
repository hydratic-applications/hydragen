//! Source-preserving dependency edits, validated before any filesystem mutation.

use std::fmt;
use std::path::PathBuf;

use serde_json::Value;

use super::{
    LayerManifestError, LayerManifestFormat, LoadedLayerManifest, parse_source, validate_document,
};

mod json;
mod toml;
mod yaml;

/// A preparation failure. No variant implies that a file was opened or changed.
#[derive(Debug)]
pub enum EditError {
    /// The source structure cannot be edited without losing its original meaning or text.
    UnsupportedEdit {
        path: PathBuf,
        format: LayerManifestFormat,
        reason: String,
    },
    /// The complete intended data or prepared source failed the existing loading rules.
    InvalidCandidate {
        path: PathBuf,
        format: LayerManifestFormat,
        source: LayerManifestError,
    },
    /// The candidate decoded successfully, but changed more than the requested append.
    UnintendedChange {
        path: PathBuf,
        format: LayerManifestFormat,
        reason: String,
    },
}

impl fmt::Display for EditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEdit {
                path,
                format,
                reason,
            } => write!(
                formatter,
                "Could not safely edit {format} layer manifest '{}': {reason}",
                path.display()
            ),
            Self::InvalidCandidate {
                path,
                format,
                source,
            } => write!(
                formatter,
                "Prepared {format} layer manifest '{}' is invalid: {source}",
                path.display()
            ),
            Self::UnintendedChange {
                path,
                format,
                reason,
            } => write!(
                formatter,
                "Refused unsafe edit to {format} layer manifest '{}': {reason}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for EditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidCandidate { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Append references in supplied order while retaining the original format and source text.
///
/// This helper never accesses the filesystem or verifies dependency targets. Callers supply
/// the new references from their verified plan. Content validity still comes exclusively
/// from the maintained schema, including for direct callers of this function. An empty
/// batch returns the original source without invoking an editor.
pub fn append_dependencies(
    layer: &LoadedLayerManifest,
    references: &[&str],
) -> Result<String, EditError> {
    if references.is_empty() {
        return Ok(layer.source.clone());
    }

    let expected = expected_document(layer, references)?;
    // Check the complete intended content before editing, so even unsupported source
    // structures cannot obscure a schema-invalid supplied reference.
    validate_document(&expected).map_err(|source| {
        invalid_candidate(
            layer,
            LayerManifestError::Metadata {
                path: layer.path.clone(),
                source,
            },
        )
    })?;

    let candidate = match layer.format {
        LayerManifestFormat::Json | LayerManifestFormat::Jsonc => {
            json::append(&layer.source, references, layer.format)
        }
        LayerManifestFormat::Toml => toml::append(&layer.source, references),
        LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
            yaml::append(&layer.source, references)
        }
    }
    .map_err(|reason| unsupported(layer, reason))?;

    verify_candidate(layer, &expected, candidate)
}

fn expected_document(layer: &LoadedLayerManifest, references: &[&str]) -> Result<Value, EditError> {
    let mut expected = layer.document.clone();
    let object = expected
        .as_object_mut()
        .ok_or_else(|| unsupported(layer, "Loaded layer manifest data is not an object.".into()))?;
    let dependencies = object
        .entry("dependencies")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| {
            unsupported(
                layer,
                "Loaded layer manifest dependencies are not an array.".into(),
            )
        })?;
    dependencies.extend(references.iter().map(|text| Value::String((*text).into())));
    Ok(expected)
}

fn verify_candidate(
    layer: &LoadedLayerManifest,
    expected: &Value,
    candidate: String,
) -> Result<String, EditError> {
    let prepared = parse_source(&layer.path, layer.format, candidate)
        .map_err(|source| invalid_candidate(layer, source))?;
    if prepared.document != *expected {
        return Err(EditError::UnintendedChange {
            path: layer.path.clone(),
            format: layer.format,
            reason: "Prepared content changes data beyond the requested dependency append; no update was prepared."
                .into(),
        });
    }
    Ok(prepared.source)
}

fn unsupported(layer: &LoadedLayerManifest, reason: String) -> EditError {
    EditError::UnsupportedEdit {
        path: layer.path.clone(),
        format: layer.format,
        reason,
    }
}

fn invalid_candidate(layer: &LoadedLayerManifest, source: LayerManifestError) -> EditError {
    EditError::InvalidCandidate {
        path: layer.path.clone(),
        format: layer.format,
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn original() -> LoadedLayerManifest {
        parse_source(
            Path::new("/diagnostic-only/layer.json"),
            LayerManifestFormat::Json,
            r#"{"name":"app","dependencies":["../old"],"custom":{"keep":true}}"#.into(),
        )
        .unwrap()
    }

    #[test]
    fn proof_rejects_changes_to_metadata_unknown_data_or_dependency_order() {
        let original = original();
        let expected = expected_document(&original, &["../new"]).unwrap();
        for candidate in [
            r#"{"name":"other","dependencies":["../old","../new"],"custom":{"keep":true}}"#,
            r#"{"name":"app","dependencies":["../old","../new"],"custom":{"keep":false}}"#,
            r#"{"name":"app","dependencies":["../new","../old"],"custom":{"keep":true}}"#,
            r#"{"name":"app","dependencies":["../old","../new"],"custom":{}}"#,
            r#"{"name":"app","dependencies":["../old"],"custom":{"keep":true}}"#,
        ] {
            assert!(matches!(
                verify_candidate(&original, &expected, candidate.into()),
                Err(EditError::UnintendedChange { .. })
            ));
        }
    }

    #[test]
    fn proof_reuses_strict_decoder_and_complete_schema() {
        let original = original();
        let expected = expected_document(&original, &["../new"]).unwrap();
        for candidate in [
            r#"{"name":"app","dependencies":["../old","../new",],"custom":{"keep":true}}"#,
            r#"{"name":"app","dependencies":["../old","../new"],"custom":{"keep":true,"keep":true}}"#,
            r#"{"name":"app","dependencies":["bare-name"],"custom":{"keep":true}}"#,
        ] {
            assert!(matches!(
                verify_candidate(&original, &expected, candidate.into()),
                Err(EditError::InvalidCandidate { .. })
            ));
        }
    }
}
