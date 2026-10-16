use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Serialize)]
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

    fn to_json(&self) -> Result<Vec<u8>, InitError> {
        self.validate()?;
        let mut json = serde_json::to_vec_pretty(self).map_err(InitError::Serialization)?;
        json.push(b'\n');
        Ok(json)
    }
}

/// Validate a supplied value without changing its whitespace or spelling.
pub fn validate_metadata_value(field: &'static str, value: &str) -> Result<(), InitError> {
    if value.chars().any(|character| !character.is_whitespace()) {
        Ok(())
    } else {
        Err(InitError::InvalidMetadata { field })
    }
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
                "'{}' already exists.\n'hydragen init' does not overwrite existing entries.\nInspect the entry or run 'hydragen init' in a different directory.\nThis command did not change any files.",
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
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Serialization(source) => Some(source),
            _ => None,
        }
    }
}

/// Create only the manifest, without overwriting an existing entry or invoking Git.
pub fn initialize(directory: &Path, metadata: &Metadata) -> Result<PathBuf, InitError> {
    let json = metadata.to_json()?;
    let path = preflight(directory)?;

    // Exclusive creation is the no-overwrite guarantee, including concurrent init
    // and dangling symlinks. An existence check alone would race with other writers.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| {
            if source.kind() == io::ErrorKind::AlreadyExists {
                InitError::ExistingManifest { path: path.clone() }
            } else {
                InitError::Io {
                    action: "create manifest",
                    path: path.clone(),
                    source,
                }
            }
        })?;
    file.write_all(&json).map_err(|source| InitError::Io {
        action: "write manifest (a partial file may remain)",
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Check known target problems without creating files or reading a manifest.
/// Creation must still be exclusive because another process can change the target.
pub fn preflight(directory: &Path) -> Result<PathBuf, InitError> {
    let directory = fs::canonicalize(directory).map_err(|source| InitError::Io {
        action: "resolve initialisation directory",
        path: directory.to_owned(),
        source,
    })?;
    check_repository_location(&directory)?;

    let path = directory.join("layer.json");
    match fs::symlink_metadata(&path) {
        Ok(_) => Err(InitError::ExistingManifest { path }),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(path),
        Err(source) => Err(InitError::Io {
            action: "inspect manifest path",
            path,
            source,
        }),
    }
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
                let bytes = metadata.to_json().unwrap();
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
                    matches!(metadata.to_json(), Err(InitError::InvalidMetadata { field: invalid }) if invalid == field)
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
        let value: Value = serde_json::from_slice(&metadata.to_json().unwrap()).unwrap();
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
}
