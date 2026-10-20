//! Read-only layer manifest discovery, selected-only loading, and shared metadata codecs.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::Metadata;
use crate::diagnostic_text::display;

mod decode;
pub mod edit;
pub mod schema;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayerManifestFormat {
    #[default]
    Yaml,
    Yml,
    Toml,
    Json,
    Jsonc,
}

impl LayerManifestFormat {
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

impl fmt::Display for LayerManifestFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.extension())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerManifestCandidate {
    pub path: PathBuf,
    pub format: LayerManifestFormat,
}

/// An observation, not a reservation. Retain it to report duplicates even if loading fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerManifestDiscovery {
    pub directory: PathBuf,
    pub candidates: Vec<LayerManifestCandidate>,
}

impl LayerManifestDiscovery {
    pub fn selected(&self) -> Option<&LayerManifestCandidate> {
        self.candidates.first()
    }

    pub fn has_duplicates(&self) -> bool {
        self.candidates.len() > 1
    }

    pub fn load(&self) -> Result<LoadedLayerManifest, LayerManifestError> {
        load(self)
    }
}

/// Inspect only the supported names in this directory, without reading their contents.
pub fn discover(directory: &Path) -> Result<LayerManifestDiscovery, LayerManifestError> {
    let mut candidates = Vec::new();
    for format in LayerManifestFormat::ALL {
        let path = directory.join(format.filename());
        match fs::symlink_metadata(&path) {
            Ok(_) => candidates.push(LayerManifestCandidate { path, format }),
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(LayerManifestError::Io {
                    action: "inspect layer manifest path",
                    path,
                    source,
                });
            }
        }
    }
    Ok(LayerManifestDiscovery {
        directory: directory.to_owned(),
        candidates,
    })
}

#[derive(Debug)]
pub struct LoadedLayerManifest {
    pub path: PathBuf,
    pub format: LayerManifestFormat,
    pub metadata: Metadata,
    /// Complete JSON-compatible data, including uninterpreted additional fields.
    pub document: Value,
    /// Original UTF-8 text, including comments, formatting, and line endings.
    pub source: String,
}

/// Read only the candidate selected by the original observation. Never retry a lower priority.
pub fn load(discovery: &LayerManifestDiscovery) -> Result<LoadedLayerManifest, LayerManifestError> {
    let candidate = discovery
        .selected()
        .ok_or_else(|| LayerManifestError::Missing {
            directory: discovery.directory.clone(),
        })?;
    let LayerManifestCandidate { path, format } = candidate;
    let io_error = |source| LayerManifestError::Io {
        action: "read layer manifest",
        path: path.clone(),
        source,
    };
    // Follow symlinks, but do not intentionally open special files such as FIFOs.
    if !fs::metadata(path).map_err(io_error)?.is_file() {
        return Err(LayerManifestError::NotRegular { path: path.clone() });
    }
    // The string API rejects invalid UTF-8 before any parser can replace or reinterpret it.
    let text = fs::read_to_string(path).map_err(io_error)?;
    parse_source(path, *format, text)
}

/// Decode and schema-check in-memory source with exactly the loading rules.
/// The path is diagnostic context only: this operation never accesses the filesystem.
pub fn parse_source(
    path: &Path,
    format: LayerManifestFormat,
    source: String,
) -> Result<LoadedLayerManifest, LayerManifestError> {
    let document =
        decode::document(&source, format).map_err(|message| LayerManifestError::Decode {
            path: path.to_owned(),
            format,
            message,
        })?;
    let metadata = validate_document(&document).map_err(|source| LayerManifestError::Metadata {
        path: path.to_owned(),
        source,
    })?;
    Ok(LoadedLayerManifest {
        path: path.to_owned(),
        format,
        metadata,
        document,
        source,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataError {
    NotObject,
    MissingField { field: &'static str },
    NotString { field: &'static str },
    Blank { field: &'static str },
    InvalidName { guidance: String },
    Schema(schema::Error),
}

/// Known root properties are diagnostic wiring, not independently maintained rules.
fn metadata_field(path: &str) -> Option<&'static str> {
    match path {
        "/name" => Some("name"),
        "/author" => Some("author"),
        "/description" => Some("description"),
        "/$schema" => Some("$schema"),
        _ => None,
    }
}

impl From<schema::Error> for MetadataError {
    fn from(error: schema::Error) -> Self {
        if let schema::Error::Content(violation) = &error {
            match &violation.kind {
                schema::ViolationKind::Name { guidance } if violation.instance_path == "/name" => {
                    return Self::InvalidName {
                        guidance: guidance.clone(),
                    };
                }
                schema::ViolationKind::Type if violation.instance_path.is_empty() => {
                    return Self::NotObject;
                }
                schema::ViolationKind::Required { property }
                    if violation.instance_path.is_empty() =>
                {
                    if let Some(field) = metadata_field(&format!("/{property}")) {
                        return Self::MissingField { field };
                    }
                }
                schema::ViolationKind::Type | schema::ViolationKind::NonBlank => {
                    if let Some(field) = metadata_field(&violation.instance_path) {
                        return if violation.kind == schema::ViolationKind::NonBlank {
                            Self::Blank { field }
                        } else {
                            Self::NotString { field }
                        };
                    }
                }
                _ => {}
            }
        }
        Self::Schema(error)
    }
}

impl fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotObject => {
                formatter.write_str("Layer manifest must be one object/mapping document.")
            }
            Self::MissingField { field } => {
                write!(formatter, "Missing required metadata field '{field}'.")
            }
            Self::NotString { field } => {
                write!(formatter, "Metadata field '{field}' must be a string.")
            }
            Self::Blank { field } => write!(
                formatter,
                "Metadata field '{field}' must contain at least one non-whitespace character."
            ),
            Self::Schema(source) => source.fmt(formatter),
            Self::InvalidName { guidance } => {
                write!(
                    formatter,
                    "Metadata field 'name' is invalid. {}",
                    display(guidance)
                )
            }
        }
    }
}

impl std::error::Error for MetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Schema(source) => Some(source),
            _ => None,
        }
    }
}

/// The common field rule used by file validation, init flags, defaults, and guided answers.
pub fn validate_field(field: &'static str, value: &str) -> Result<(), MetadataError> {
    schema::validate_field(field, value).map_err(MetadataError::from)
}

/// Validate complete parsed data, without normalising strings or interpreting extra fields.
pub fn validate_document(document: &Value) -> Result<Metadata, MetadataError> {
    schema::validate(document).map_err(MetadataError::from)?;
    // Validation is authoritative. These conversions only defend extraction if
    // the schema's supported shapes ever diverge from the typed metadata view.
    let object = document.as_object().ok_or(MetadataError::NotObject)?;
    let field_value = |field: &'static str| -> Result<Option<String>, MetadataError> {
        object
            .get(field)
            .map(|value| {
                let text = value.as_str().ok_or(MetadataError::NotString { field })?;
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
pub enum LayerManifestError {
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
        format: LayerManifestFormat,
        message: String,
    },
    Metadata {
        path: PathBuf,
        source: MetadataError,
    },
}

impl fmt::Display for LayerManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { directory } => write!(
                formatter,
                "No layer manifest found in '{}'; expected one of: {}.",
                display(directory.display()),
                LayerManifestFormat::ALL
                    .map(LayerManifestFormat::filename)
                    .join(", ")
            ),
            Self::Io {
                action,
                path,
                source,
            } => write!(
                formatter,
                "Could not {action} '{}': {}",
                display(path.display()),
                display(source)
            ),
            Self::NotRegular { path } => write!(
                formatter,
                "Could not read layer manifest '{}': target must be a regular file (or a symlink to one).",
                display(path.display())
            ),
            Self::Decode {
                path,
                format,
                message,
            } => write!(
                formatter,
                "Could not decode {format} layer manifest '{}': {}",
                display(path.display()),
                display(message)
            ),
            Self::Metadata { path, source } => {
                write!(
                    formatter,
                    "Invalid layer manifest '{}': {source}",
                    display(path.display())
                )
            }
        }
    }
}

impl std::error::Error for LayerManifestError {
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
        format: LayerManifestFormat,
        message: String,
    },
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Metadata(source) => source.fmt(formatter),
            Self::Serialization { format, message } => write!(
                formatter,
                "Could not serialise {format} layer metadata: {}",
                display(message)
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
    format: LayerManifestFormat,
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
        LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
            serde_saphyr::to_string(metadata)
                .map_err(|error| serialization_error(error.to_string()))?
        }
        LayerManifestFormat::Toml => toml::to_string_pretty(metadata)
            .map_err(|error| serialization_error(error.to_string()))?,
        LayerManifestFormat::Json | LayerManifestFormat::Jsonc => {
            serde_json::to_string_pretty(metadata)
                .map_err(|error| serialization_error(error.to_string()))?
        }
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text.into_bytes())
}
