//! Read-only manifest discovery, selected-only loading, and shared metadata codecs.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::Metadata;

mod decode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ManifestFormat {
    #[default]
    Yaml,
    Yml,
    Toml,
    Json,
    Jsonc,
}

impl ManifestFormat {
    /// Supported choices, from highest to lowest discovery priority.
    pub const ALL: [Self; 5] = [Self::Yaml, Self::Yml, Self::Toml, Self::Json, Self::Jsonc];

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Yml => "yml",
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Jsonc => "jsonc",
        }
    }

    pub const fn filename(self) -> &'static str {
        match self {
            Self::Yaml => "layer.yaml",
            Self::Yml => "layer.yml",
            Self::Toml => "layer.toml",
            Self::Json => "layer.json",
            Self::Jsonc => "layer.jsonc",
        }
    }
}

impl fmt::Display for ManifestFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.extension())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestCandidate {
    pub path: PathBuf,
    pub format: ManifestFormat,
}

/// An observation, not a reservation. Retain it to report duplicates even if loading fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestDiscovery {
    pub directory: PathBuf,
    pub candidates: Vec<ManifestCandidate>,
}

impl ManifestDiscovery {
    pub fn selected(&self) -> Option<&ManifestCandidate> {
        self.candidates.first()
    }

    pub fn has_duplicates(&self) -> bool {
        self.candidates.len() > 1
    }

    pub fn load(&self) -> Result<LoadedManifest, ManifestError> {
        load(self)
    }
}

/// Inspect only the supported names in this directory, without reading their contents.
pub fn discover(directory: &Path) -> Result<ManifestDiscovery, ManifestError> {
    let mut candidates = Vec::new();
    for format in ManifestFormat::ALL {
        let path = directory.join(format.filename());
        match fs::symlink_metadata(&path) {
            Ok(_) => candidates.push(ManifestCandidate { path, format }),
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(ManifestError::Io {
                    action: "inspect manifest path",
                    path,
                    source,
                });
            }
        }
    }
    Ok(ManifestDiscovery {
        directory: directory.to_owned(),
        candidates,
    })
}

#[derive(Debug)]
pub struct LoadedManifest {
    pub path: PathBuf,
    pub format: ManifestFormat,
    pub metadata: Metadata,
    /// Complete JSON-compatible data, including uninterpreted additional fields.
    pub document: Value,
}

/// Read only the candidate selected by the original observation. Never retry a lower priority.
pub fn load(discovery: &ManifestDiscovery) -> Result<LoadedManifest, ManifestError> {
    let candidate = discovery.selected().ok_or_else(|| ManifestError::Missing {
        directory: discovery.directory.clone(),
    })?;
    let ManifestCandidate { path, format } = candidate;
    let io_error = |source| ManifestError::Io {
        action: "read manifest",
        path: path.clone(),
        source,
    };
    // Follow symlinks, but do not intentionally open special files such as FIFOs.
    if !fs::metadata(path).map_err(io_error)?.is_file() {
        return Err(ManifestError::NotRegular { path: path.clone() });
    }
    // The string API rejects invalid UTF-8 before any parser can replace or reinterpret it.
    let text = fs::read_to_string(path).map_err(io_error)?;
    let document = decode::document(&text, *format).map_err(|message| ManifestError::Decode {
        path: path.clone(),
        format: *format,
        message,
    })?;
    let metadata = validate_document(&document).map_err(|source| ManifestError::Metadata {
        path: path.clone(),
        source,
    })?;
    Ok(LoadedManifest {
        path: path.clone(),
        format: *format,
        metadata,
        document,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataError {
    NotObject,
    MissingField { field: &'static str },
    NotString { field: &'static str },
    Blank { field: &'static str },
}

impl fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotObject => formatter.write_str("Manifest must be one object/mapping document."),
            Self::MissingField { field } => {
                write!(formatter, "Missing required metadata field '{field}'.")
            }
            Self::NotString { field } => write!(
                formatter,
                "Metadata field '{field}' must be a string; null is not omission."
            ),
            Self::Blank { field } => write!(
                formatter,
                "Metadata field '{field}' must contain at least one non-whitespace character."
            ),
        }
    }
}

impl std::error::Error for MetadataError {}

/// The common field rule used by file validation, init flags, defaults, and guided answers.
pub fn validate_field(field: &'static str, value: &str) -> Result<(), MetadataError> {
    if value.chars().any(|character| !character.is_whitespace()) {
        Ok(())
    } else {
        Err(MetadataError::Blank { field })
    }
}

/// Validate complete parsed data, without normalising strings or interpreting extra fields.
pub fn validate_document(document: &Value) -> Result<Metadata, MetadataError> {
    let object = document.as_object().ok_or(MetadataError::NotObject)?;
    let field_value = |field: &'static str| -> Result<Option<String>, MetadataError> {
        object
            .get(field)
            .map(|value| {
                let text = value.as_str().ok_or(MetadataError::NotString { field })?;
                validate_field(field, text)?;
                Ok(text.to_owned())
            })
            .transpose()
    };
    Ok(Metadata {
        name: field_value("name")?.ok_or(MetadataError::MissingField { field: "name" })?,
        author: field_value("author")?,
        description: field_value("description")?,
    })
}

#[derive(Debug)]
pub enum ManifestError {
    Missing {
        directory: PathBuf,
    },
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    NotRegular {
        path: PathBuf,
    },
    Decode {
        path: PathBuf,
        format: ManifestFormat,
        message: String,
    },
    Metadata {
        path: PathBuf,
        source: MetadataError,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { directory } => write!(
                formatter,
                "No layer manifest found in '{}'; expected one of: {}.",
                directory.display(),
                ManifestFormat::ALL.map(ManifestFormat::filename).join(", ")
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
            Self::NotRegular { path } => write!(
                formatter,
                "Could not read manifest '{}': target must be a regular file (or a symlink to one).",
                path.display()
            ),
            Self::Decode {
                path,
                format,
                message,
            } => write!(
                formatter,
                "Could not decode {format} manifest '{}': {message}",
                path.display()
            ),
            Self::Metadata { path, source } => {
                write!(formatter, "Invalid manifest '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Metadata { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum EncodeError {
    Metadata(MetadataError),
    Serialization {
        format: ManifestFormat,
        message: String,
    },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metadata(source) => source.fmt(formatter),
            Self::Serialization { format, message } => write!(
                formatter,
                "Could not serialise {format} layer metadata: {message}"
            ),
        }
    }
}

impl std::error::Error for EncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Metadata(source) => Some(source),
            _ => None,
        }
    }
}

/// Encode only init metadata, validating everything before callers open an output file.
pub fn encode_metadata(
    metadata: &Metadata,
    format: ManifestFormat,
) -> Result<Vec<u8>, EncodeError> {
    let serialization_error = |error: String| EncodeError::Serialization {
        format,
        message: error,
    };
    let value =
        serde_json::to_value(metadata).map_err(|error| serialization_error(error.to_string()))?;
    validate_document(&value).map_err(EncodeError::Metadata)?;
    // Serialize the struct, not the generic map, to retain name/author/description order.
    let mut text = match format {
        ManifestFormat::Yaml | ManifestFormat::Yml => serde_saphyr::to_string(metadata)
            .map_err(|error| serialization_error(error.to_string()))?,
        ManifestFormat::Toml => toml::to_string_pretty(metadata)
            .map_err(|error| serialization_error(error.to_string()))?,
        ManifestFormat::Json | ManifestFormat::Jsonc => serde_json::to_string_pretty(metadata)
            .map_err(|error| serialization_error(error.to_string()))?,
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text.into_bytes())
}
