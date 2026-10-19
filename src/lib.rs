use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

pub mod dependencies;
pub mod diagnostic_text;
pub mod layer_manifest;
pub mod replacement;
pub use layer_manifest::{
    EncodeError, LayerManifestCandidate, LayerManifestDiscovery, LayerManifestError,
    LayerManifestFormat, LoadedLayerManifest, MetadataError, discover, encode_metadata, load,
    validate_document,
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
        self.validate_with(layer_manifest::schema::validate)
    }

    fn validate_with(
        &self,
        validate: impl FnOnce(&serde_json::Value) -> Result<(), layer_manifest::schema::Error>,
    ) -> Result<(), InitError> {
        let candidate = serde_json::to_value(self).map_err(InitError::Serialization)?;
        validate(&candidate).map_err(InitError::from)
    }
}

/// Validate a supplied value without changing its whitespace or spelling.
pub fn validate_metadata_value(field: &'static str, value: &str) -> Result<(), InitError> {
    layer_manifest::schema::validate_field(field, value).map_err(InitError::from)
}

#[derive(Debug)]
pub enum InitError {
    InvalidMetadata {
        field: &'static str,
    },
    InvalidName {
        guidance: String,
    },
    Schema(layer_manifest::schema::Error),
    NestedRepository {
        root: PathBuf,
    },
    ExistingLayerManifest {
        path: PathBuf,
    },
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Serialization(serde_json::Error),
    Encoding(EncodeError),
    Discovery(LayerManifestError),
}

impl From<layer_manifest::schema::Error> for InitError {
    fn from(error: layer_manifest::schema::Error) -> Self {
        if let layer_manifest::schema::Error::Content(violation) = &error
            && violation.instance_path == "/name"
            && let layer_manifest::schema::ViolationKind::Name { guidance } = &violation.kind
        {
            return Self::InvalidName {
                guidance: guidance.clone(),
            };
        }
        if let layer_manifest::schema::Error::Content(violation) = &error
            && violation.kind == layer_manifest::schema::ViolationKind::NonBlank
        {
            let field = match violation.instance_path.as_str() {
                "/name" => Some("name"),
                "/author" => Some("author"),
                "/description" => Some("description"),
                _ => None,
            };
            if let Some(field) = field {
                return Self::InvalidMetadata { field };
            }
        }
        Self::Schema(error)
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata { field } => write!(
                formatter,
                "--{field} must contain at least one non-whitespace character."
            ),
            Self::Schema(source) => source.fmt(formatter),
            Self::InvalidName { guidance } => write!(
                formatter,
                "--name is invalid. {}",
                diagnostic_text::display(guidance)
            ),
            Self::NestedRepository { root } => write!(
                formatter,
                "Run 'hydragen init' from the repository root '{}', not a subdirectory. No files were changed.",
                diagnostic_text::display(root.display())
            ),
            Self::ExistingLayerManifest { path } => write!(
                formatter,
                "'{}' already exists.\n'hydragen init' does not overwrite existing entries.\nInspect the entry or run 'hydragen init' in a different directory.\nThis command did not change any files.",
                diagnostic_text::display(path.display())
            ),
            Self::Io {
                action,
                path,
                source,
            } => write!(
                formatter,
                "Could not {action} '{}': {}",
                diagnostic_text::display(path.display()),
                diagnostic_text::display(source)
            ),
            Self::Serialization(source) => {
                write!(
                    formatter,
                    "Could not serialise layer metadata: {}",
                    diagnostic_text::display(source)
                )
            }
            Self::Encoding(source) => source.fmt(formatter),
            Self::Discovery(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Schema(source) => Some(source),
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
    initialize_validating(directory, metadata, layer_manifest::schema::validate)
}

fn initialize_validating(
    directory: &Path,
    metadata: &Metadata,
    validate: impl FnOnce(&serde_json::Value) -> Result<(), layer_manifest::schema::Error>,
) -> Result<PathBuf, InitError> {
    metadata.validate_with(validate)?;
    let prepared = inspect_init(directory)?.prepare()?;
    create_init(prepared, metadata, LayerManifestFormat::default())
        .map(|created| created.path)
        .map_err(|failure| failure.error)
}

/// Initial inspection retains discovery even when an occupied name will refuse init.
#[derive(Debug)]
pub struct InitInspection {
    discovery: LayerManifestDiscovery,
}

impl InitInspection {
    pub fn discovery(&self) -> &LayerManifestDiscovery {
        &self.discovery
    }

    /// Refuse occupied names, without reserving the directory or reading contents.
    pub fn prepare(self) -> Result<PreparedInit, InitError> {
        if let Some(candidate) = self.discovery.selected() {
            return Err(InitError::ExistingLayerManifest {
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
pub struct CreatedLayerManifest {
    pub path: PathBuf,
    pub observation: Result<LayerManifestDiscovery, LayerManifestError>,
}

/// What this invocation did before creation failed, not whether a target exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitCreationState {
    /// This invocation did not successfully create the target. Another entry may exist.
    NotCreated,
    /// Exclusive creation succeeded, but writing failed; an empty or partial file may remain.
    PossiblyPartial,
}

/// Creation failed. A collision may retain discovery for diagnostics, never retry.
#[derive(Debug)]
pub struct InitCreationError {
    pub error: InitError,
    pub discovery: Option<LayerManifestDiscovery>,
    pub state: InitCreationState,
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
    /// For failures before successful exclusive creation only. Post-create write
    /// failures must explicitly retain `PossiblyPartial` instead of using this conversion.
    fn from(error: InitError) -> Self {
        Self {
            error,
            discovery: None,
            state: InitCreationState::NotCreated,
        }
    }
}

/// Validate and encode before exclusively creating only the requested target.
/// Never repeat the initial all-format refusal check, even if other formats appeared.
pub fn create_init(
    prepared: PreparedInit,
    metadata: &Metadata,
    format: LayerManifestFormat,
) -> Result<CreatedLayerManifest, InitCreationError> {
    create_init_observing(prepared, metadata, format, discover)
}

fn create_init_observing(
    prepared: PreparedInit,
    metadata: &Metadata,
    format: LayerManifestFormat,
    observe: impl FnOnce(&Path) -> Result<LayerManifestDiscovery, LayerManifestError>,
) -> Result<CreatedLayerManifest, InitCreationError> {
    create_init_writing(prepared, metadata, format, observe, |file, bytes| {
        file.write_all(bytes)
    })
}

fn create_init_writing(
    prepared: PreparedInit,
    metadata: &Metadata,
    format: LayerManifestFormat,
    observe: impl FnOnce(&Path) -> Result<LayerManifestDiscovery, LayerManifestError>,
    write: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
) -> Result<CreatedLayerManifest, InitCreationError> {
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
                    error: InitError::ExistingLayerManifest { path: path.clone() },
                    discovery: observe(&prepared.directory).ok(),
                    state: InitCreationState::NotCreated,
                }
            } else {
                InitError::Io {
                    action: "create layer manifest",
                    path: path.clone(),
                    source,
                }
                .into()
            });
        }
    };
    write(&mut file, &bytes).map_err(|source| InitCreationError {
        error: InitError::Io {
            action: "write layer manifest (a partial file may remain)",
            path: path.clone(),
            source,
        },
        discovery: None,
        state: InitCreationState::PossiblyPartial,
    })?;
    Ok(CreatedLayerManifest {
        path,
        observation: observe(&prepared.directory),
    })
}

/// Check known target problems without creating files or reading a layer manifest.
/// Creation must still be exclusive because another process can change the target.
pub fn preflight(directory: &Path) -> Result<PathBuf, InitError> {
    Ok(inspect_init(directory)?
        .prepare()?
        .directory
        .join(LayerManifestFormat::default().filename()))
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
    fn post_create_write_failure_retains_partial_bytes_state_cause_and_skips_observation() {
        for count in [0, 5] {
            let root = tempfile::tempdir().unwrap();
            let prepared = inspect_init(root.path()).unwrap().prepare().unwrap();
            let path = prepared.directory().join("layer.yaml");
            let expected = encode_metadata(&metadata(), LayerManifestFormat::Yaml).unwrap();
            let failure = create_init_writing(
                prepared,
                &metadata(),
                LayerManifestFormat::Yaml,
                |_| panic!("must not observe after a failed write"),
                |file, bytes| {
                    file.write_all(&bytes[..count])?;
                    Err(io::Error::other("injected write failure"))
                },
            )
            .unwrap_err();
            assert_eq!(failure.state, InitCreationState::PossiblyPartial);
            assert!(failure.discovery.is_none());
            assert_eq!(fs::read(&path).unwrap(), expected[..count]);
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
            let InitError::Io {
                path: affected,
                source,
                ..
            } = &failure.error
            else {
                panic!("expected write error");
            };
            assert_eq!(affected, &path);
            assert_eq!(source.kind(), io::ErrorKind::Other);
            assert_eq!(source.to_string(), "injected write failure");
            assert_eq!(failure.to_string(), failure.error.to_string());
            assert!(
                std::error::Error::source(&failure)
                    .unwrap()
                    .is::<InitError>()
            );
        }
    }

    #[test]
    fn pre_create_validation_open_and_collision_failures_do_not_claim_creation() {
        let root = tempfile::tempdir().unwrap();
        let prepared = inspect_init(root.path()).unwrap().prepare().unwrap();
        let invalid = Metadata {
            name: "".into(),
            ..metadata()
        };
        let failure = create_init_writing(
            prepared,
            &invalid,
            LayerManifestFormat::Yaml,
            |_| panic!("no observation before creation"),
            |_, _| panic!("no write before validation"),
        )
        .unwrap_err();
        assert_eq!(failure.state, InitCreationState::NotCreated);
        assert!(!root.path().join("layer.yaml").exists());
        let prepared = inspect_init(root.path()).unwrap().prepare().unwrap();
        let path = prepared.directory().join("layer.yaml");
        fs::write(&path, "keep").unwrap();
        let failure = create_init_writing(
            prepared,
            &metadata(),
            LayerManifestFormat::Yaml,
            discover,
            |_, _| panic!("collision must not write"),
        )
        .unwrap_err();
        assert_eq!(failure.state, InitCreationState::NotCreated);
        assert!(failure.discovery.is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep");
        let missing = root.path().join("missing");
        let failure = create_init_writing(
            PreparedInit {
                directory: missing.clone(),
            },
            &metadata(),
            LayerManifestFormat::Yaml,
            |_| panic!("failed open must not observe"),
            |_, _| panic!("failed open must not write"),
        )
        .unwrap_err();
        assert_eq!(failure.state, InitCreationState::NotCreated);
        assert!(!missing.exists());
    }

    #[test]
    fn pre_creation_conversion_retains_encoding_and_setup_causes_without_guessing_from_text() {
        for error in [
            InitError::Encoding(EncodeError::Serialization {
                format: LayerManifestFormat::Yaml,
                message: "encoding failure".into(),
            }),
            InitError::Schema(layer_manifest::schema::Error::Setup {
                message: "setup failure".into(),
                detail: None,
            }),
            InitError::Io {
                action: "opaque pre-creation action",
                path: "/diagnostic-only/layer.yaml".into(),
                source: io::Error::other("underlying failure"),
            },
        ] {
            let original = error.to_string();
            let error = InitCreationError::from(error);
            assert_eq!(error.state, InitCreationState::NotCreated);
            assert!(error.discovery.is_none());
            assert_eq!(error.to_string(), original);
            assert!(std::error::Error::source(&error).unwrap().is::<InitError>());
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
                let bytes = encode_metadata(&metadata, LayerManifestFormat::Json).unwrap();
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
            name: "@example/.config".into(),
            author: Some(original.into()),
            description: Some(original.into()),
        };
        let value: Value =
            serde_json::from_slice(&encode_metadata(&metadata, LayerManifestFormat::Json).unwrap())
                .unwrap();
        assert_eq!(value["name"], "@example/.config");
        for field in ["author", "description"] {
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
    fn whole_object_schema_constraints_fail_before_library_filesystem_access() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("must-not-be-inspected-or-created");
        let metadata = metadata();
        validate_metadata_value("name", &metadata.name).unwrap();
        let mut schema: Value =
            serde_json::from_str(include_str!("../schemas/layer-manifest.schema.json")).unwrap();
        // A whole-object rule that every individual field validator would miss.
        schema["maxProperties"] = json!(0);
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .offline()
            .build(&schema)
            .unwrap();
        let error = initialize_validating(&missing, &metadata, |candidate| {
            assert_eq!(candidate, &json!({"name": "example"}));
            let error = validator.validate(candidate).unwrap_err();
            Err(layer_manifest::schema::Error::Content(Box::new(
                layer_manifest::schema::Violation {
                    instance_path: error.instance_path().to_string(),
                    schema_path: error.schema_path().to_string(),
                    keyword: error.kind().keyword().into(),
                    message: error.to_string(),
                    kind: layer_manifest::schema::ViolationKind::Other,
                },
            )))
        })
        .unwrap_err();
        assert!(matches!(
            error,
            InitError::Schema(layer_manifest::schema::Error::Content(_))
        ));
        assert!(error.to_string().contains("maxProperties"));
        assert!(!error.to_string().contains("non-whitespace"));
        assert!(!missing.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn internal_schema_failure_precedes_library_filesystem_access() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing");
        let setup = layer_manifest::schema::validate_field("unknown", "valid").unwrap_err();
        let error = initialize_validating(&missing, &metadata(), |_| Err(setup)).unwrap_err();
        assert!(matches!(
            error,
            InitError::Schema(layer_manifest::schema::Error::Setup { .. })
        ));
        assert!(
            error
                .to_string()
                .contains("Internal layer manifest schema error")
        );
        assert!(!error.to_string().contains("non-whitespace"));
        assert!(!missing.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
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
        let layer_manifest = preflight(directory.path()).unwrap();
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
        fs::write(&layer_manifest, "another writer").unwrap();
        assert!(matches!(
            initialize(directory.path(), &metadata()),
            Err(InitError::ExistingLayerManifest { .. })
        ));
        assert_eq!(
            fs::read_to_string(layer_manifest).unwrap(),
            "another writer"
        );
    }

    #[test]
    fn failed_post_write_observation_preserves_completed_creation() {
        let directory = tempfile::tempdir().unwrap();
        let prepared = inspect_init(directory.path()).unwrap().prepare().unwrap();
        let created =
            create_init_observing(prepared, &metadata(), LayerManifestFormat::Jsonc, |root| {
                // This seam runs only after the file has been completely written.
                assert_eq!(
                    fs::read(root.join("layer.jsonc")).unwrap(),
                    b"{\n  \"name\": \"example\"\n}\n"
                );
                Err(LayerManifestError::Io {
                    action: "inspect layer manifest path",
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
        let failure =
            create_init_observing(prepared, &metadata(), LayerManifestFormat::Yaml, |root| {
                Err(LayerManifestError::Io {
                    action: "inspect layer manifest path",
                    path: root.join("layer.json"),
                    source: io::ErrorKind::PermissionDenied.into(),
                })
            })
            .unwrap_err();
        assert!(matches!(
            failure.error,
            InitError::ExistingLayerManifest { .. }
        ));
        assert_eq!(failure.state, InitCreationState::NotCreated);
        assert!(failure.discovery.is_none());
        assert_eq!(fs::read_to_string(path).unwrap(), "keep");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
