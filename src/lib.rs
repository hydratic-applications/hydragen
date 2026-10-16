use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

pub mod manifest;
pub use manifest::{
    EncodeError, LoadedManifest, ManifestCandidate, ManifestDiscovery, ManifestError,
    ManifestFormat, MetadataError, discover, encode_metadata, load, validate_document,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Metadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Metadata {
    pub fn validate(&self) -> Result<(), InitError> {
        for (field, value) in [
            ("name", Some(self.name.as_str())),
            ("author", self.author.as_deref()),
            ("description", self.description.as_deref()),
        ] {
            if let Some(value) = value {
                validate_metadata_value(field, value)?;
            }
        }
        Ok(())
    }
}

/// Validate a supplied value without changing its whitespace or spelling.
pub fn validate_metadata_value(field: &'static str, value: &str) -> Result<(), InitError> {
    manifest::validate_field(field, value).map_err(|_| InitError::InvalidMetadata { field })
}

#[derive(Debug)]
pub enum InitError {
    InvalidMetadata {
        field: &'static str,
    },
    NestedRepository {
        root: PathBuf,
    },
    ExistingManifest {
        path: PathBuf,
    },
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Serialization(serde_json::Error),
    Encoding(EncodeError),
    Discovery(ManifestError),
}

impl fmt::Display for InitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata { field } => write!(
                formatter,
                "--{field} must contain at least one non-whitespace character."
            ),
            Self::NestedRepository { root } => write!(
                formatter,
                "Run 'hydragen init' from the repository root '{}', not a subdirectory. No files were changed.",
                root.display()
            ),
            Self::ExistingManifest { path } => write!(
                formatter,
                "'{}' already exists. Initialisation was cancelled; no files were changed.",
                path.display()
            ),
            Self::Io {
                action,
                path,
                source,
            } => write!(
                formatter,
                "Could not {action} '{}': {source}",
                path.display()
            ),
            Self::Serialization(source) => {
                write!(formatter, "Could not serialise layer metadata: {source}")
            }
            Self::Encoding(source) => source.fmt(formatter),
            Self::Discovery(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Serialization(source) => Some(source),
            Self::Encoding(source) => Some(source),
            Self::Discovery(source) => Some(source),
            _ => None,
        }
    }
}

/// Initialise with the YAML default. Invalid metadata is rejected before filesystem access.
/// CLI callers should inspect, report diagnostics, prepare, then call `create_init` instead.
pub fn initialize(directory: &Path, metadata: &Metadata) -> Result<PathBuf, InitError> {
    metadata.validate()?;
    let prepared = inspect_init(directory)?.prepare()?;
    create_init(prepared, metadata, ManifestFormat::default())
        .map(|created| created.path)
        .map_err(|failure| failure.error)
}

/// Initial inspection retains discovery even when an occupied name will refuse init.
#[derive(Debug)]
pub struct InitInspection {
    discovery: ManifestDiscovery,
}

impl InitInspection {
    pub fn discovery(&self) -> &ManifestDiscovery {
        &self.discovery
    }

    /// Refuse occupied names, without reserving the directory or reading contents.
    pub fn prepare(self) -> Result<PreparedInit, InitError> {
        if let Some(candidate) = self.discovery.selected() {
            return Err(InitError::ExistingManifest {
                path: candidate.path.clone(),
            });
        }
        Ok(PreparedInit {
            directory: self.discovery.directory,
        })
    }
}

/// Permission from one initially empty inspection, not a lock or reservation.
#[derive(Debug)]
pub struct PreparedInit {
    directory: PathBuf,
}

impl PreparedInit {
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

/// A completed write and its best-effort, content-free duplicate observation.
#[derive(Debug)]
pub struct CreatedManifest {
    pub path: PathBuf,
    pub observation: Result<ManifestDiscovery, ManifestError>,
}

/// Creation failed. A collision may retain discovery for diagnostics, never retry.
#[derive(Debug)]
pub struct InitCreationError {
    pub error: InitError,
    pub discovery: Option<ManifestDiscovery>,
}

impl fmt::Display for InitCreationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl std::error::Error for InitCreationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

impl From<InitError> for InitCreationError {
    fn from(error: InitError) -> Self {
        Self {
            error,
            discovery: None,
        }
    }
}

/// Validate and encode before exclusively creating only the requested target.
/// Never repeat the initial all-format refusal check, even if other formats appeared.
pub fn create_init(
    prepared: PreparedInit,
    metadata: &Metadata,
    format: ManifestFormat,
) -> Result<CreatedManifest, InitCreationError> {
    create_init_observing(prepared, metadata, format, discover)
}

fn create_init_observing(
    prepared: PreparedInit,
    metadata: &Metadata,
    format: ManifestFormat,
    observe: impl FnOnce(&Path) -> Result<ManifestDiscovery, ManifestError>,
) -> Result<CreatedManifest, InitCreationError> {
    metadata.validate()?;
    let bytes = encode_metadata(metadata, format).map_err(InitError::Encoding)?;
    let path = prepared.directory.join(format.filename());

    // Exclusive creation is the no-overwrite guarantee, including concurrent init
    // and dangling symlinks. An existence check alone would race with other writers.
    let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => file,
        Err(source) => {
            return Err(if source.kind() == io::ErrorKind::AlreadyExists {
                InitCreationError {
                    error: InitError::ExistingManifest { path: path.clone() },
                    discovery: observe(&prepared.directory).ok(),
                }
            } else {
                InitError::Io {
                    action: "create manifest",
                    path: path.clone(),
                    source,
                }
                .into()
            });
        }
    };
    file.write_all(&bytes).map_err(|source| InitError::Io {
        action: "write manifest (a partial file may remain)",
        path: path.clone(),
        source,
    })?;
    Ok(CreatedManifest {
        path,
        observation: observe(&prepared.directory),
    })
}

/// Check known target problems without creating files or reading a manifest.
/// Creation must still be exclusive because another process can change the target.
pub fn preflight(directory: &Path) -> Result<PathBuf, InitError> {
    Ok(inspect_init(directory)?
        .prepare()?
        .directory
        .join(ManifestFormat::default().filename()))
}

/// Canonicalise and check repository boundaries once, then discover all supported names.
pub fn inspect_init(directory: &Path) -> Result<InitInspection, InitError> {
    let directory = fs::canonicalize(directory).map_err(|source| InitError::Io {
        action: "resolve initialisation directory",
        path: directory.to_owned(),
        source,
    })?;
    check_repository_location(&directory)?;

    let discovery = discover(&directory).map_err(InitError::Discovery)?;
    Ok(InitInspection { discovery })
}

fn check_repository_location(directory: &Path) -> Result<(), InitError> {
    for ancestor in directory.ancestors() {
        let marker = ancestor.join(".git");
        match fs::metadata(&marker) {
            Ok(metadata) if metadata.is_file() || metadata.is_dir() => {
                return if ancestor == directory {
                    Ok(())
                } else {
                    Err(InitError::NestedRepository {
                        root: ancestor.to_owned(),
                    })
                };
            }
            Ok(_) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(InitError::Io {
                    action: "inspect repository marker",
                    path: marker,
                    source,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn metadata() -> Metadata {
        Metadata {
            name: "example".into(),
            author: None,
            description: None,
        }
    }

    #[test]
    fn metadata_contract_matches_the_manifest_schema() {
        let schema: Value = serde_json::from_str(include_str!(
            "../openspec/changes/archive/2026-10-16-define-layer-manifest/layer-manifest.schema.json"
        ))
        .unwrap();
        assert_eq!(schema["required"], json!(["name"]));
        for field in ["name", "author", "description"] {
            assert_eq!(schema["properties"][field]["type"], "string");
            assert_eq!(schema["properties"][field]["pattern"], r"\S");
        }
    }

    #[test]
    fn optional_fields_are_independent_and_omitted_when_absent() {
        for author in [None, Some("Example team".to_owned())] {
            for description in [None, Some("Example layer".to_owned())] {
                let metadata = Metadata {
                    author: author.clone(),
                    description: description.clone(),
                    ..metadata()
                };
                let bytes = encode_metadata(&metadata, ManifestFormat::Json).unwrap();
                let value: Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(value["name"], "example");
                assert_eq!(
                    value.get("author"),
                    author.as_ref().map(|text| json!(text)).as_ref()
                );
                assert_eq!(
                    value.get("description"),
                    description.as_ref().map(|text| json!(text)).as_ref()
                );
                assert_eq!(
                    value.as_object().unwrap().len(),
                    1 + usize::from(author.is_some()) + usize::from(description.is_some())
                );
                let text = std::str::from_utf8(&bytes).unwrap();
                assert!(text.starts_with("{\n  \""));
                assert!(text.ends_with("\n}\n"));
                assert!(
                    text.lines()
                        .skip(1)
                        .take(value.as_object().unwrap().len())
                        .all(|line| line.starts_with("  \""))
                );
            }
        }
    }

    #[test]
    fn blank_supplied_values_are_rejected_for_every_field() {
        for field in ["name", "author", "description"] {
            for value in ["", " \t\r\n", "\u{00a0}\u{2003}"] {
                let mut metadata = metadata();
                match field {
                    "name" => metadata.name = value.into(),
                    "author" => metadata.author = Some(value.into()),
                    "description" => metadata.description = Some(value.into()),
                    _ => unreachable!(),
                }
                assert!(
                    matches!(metadata.validate(), Err(InitError::InvalidMetadata { field: invalid }) if invalid == field)
                );
            }
        }
    }

    #[test]
    fn accepted_strings_round_trip_without_normalisation() {
        let original = "  Rules: \"quoted\" \\path\n配置 🦀  ";
        let metadata = Metadata {
            name: original.into(),
            author: Some(original.into()),
            description: Some(original.into()),
        };
        let value: Value =
            serde_json::from_slice(&encode_metadata(&metadata, ManifestFormat::Json).unwrap())
                .unwrap();
        for field in ["name", "author", "description"] {
            assert_eq!(value[field], original);
        }
    }

    #[test]
    fn invalid_metadata_is_rejected_before_filesystem_access() {
        let metadata = Metadata {
            name: "".into(),
            ..metadata()
        };
        assert!(matches!(
            initialize(Path::new("nonexistent-directory"), &metadata),
            Err(InitError::InvalidMetadata { field: "name" })
        ));
    }

    #[test]
    fn missing_target_directory_is_not_created() {
        let temporary = tempfile::tempdir().unwrap();
        let missing = temporary.path().join("missing");
        let error = initialize(&missing, &metadata()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("resolve initialisation directory")
        );
        assert!(!missing.exists());
    }

    #[test]
    fn preflight_does_not_write_or_authorise_a_later_overwrite() {
        let directory = tempfile::tempdir().unwrap();
        let manifest = preflight(directory.path()).unwrap();
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
        fs::write(&manifest, "another writer").unwrap();
        assert!(matches!(
            initialize(directory.path(), &metadata()),
            Err(InitError::ExistingManifest { .. })
        ));
        assert_eq!(fs::read_to_string(manifest).unwrap(), "another writer");
    }

    #[test]
    fn failed_post_write_observation_preserves_completed_creation() {
        let directory = tempfile::tempdir().unwrap();
        let prepared = inspect_init(directory.path()).unwrap().prepare().unwrap();
        let created = create_init_observing(prepared, &metadata(), ManifestFormat::Jsonc, |root| {
            // This seam runs only after the file has been completely written.
            assert_eq!(
                fs::read(root.join("layer.jsonc")).unwrap(),
                b"{\n  \"name\": \"example\"\n}\n"
            );
            Err(ManifestError::Io {
                action: "inspect manifest path",
                path: root.join("layer.yaml"),
                source: io::ErrorKind::PermissionDenied.into(),
            })
        })
        .unwrap();
        assert_eq!(created.path.file_name().unwrap(), "layer.jsonc");
        assert!(created.observation.is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        assert!(created.path.is_file());
    }

    #[test]
    fn failed_collision_observation_preserves_the_original_creation_error() {
        let directory = tempfile::tempdir().unwrap();
        let prepared = inspect_init(directory.path()).unwrap().prepare().unwrap();
        let path = directory.path().join("layer.yaml");
        fs::write(&path, "keep").unwrap();
        let failure = create_init_observing(prepared, &metadata(), ManifestFormat::Yaml, |root| {
            Err(ManifestError::Io {
                action: "inspect manifest path",
                path: root.join("layer.json"),
                source: io::ErrorKind::PermissionDenied.into(),
            })
        })
        .unwrap_err();
        assert!(matches!(failure.error, InitError::ExistingManifest { .. }));
        assert!(failure.discovery.is_none());
        assert_eq!(fs::read_to_string(path).unwrap(), "keep");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
