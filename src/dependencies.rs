//! Read-only direct-dependency verification and ordered local-add preparation.
//!
//! No operation here edits files, stages output, writes terminal messages, consults
//! a registry, or traverses a target's dependencies. A plan is an observation, not
//! a lock or a promise that the filesystem will remain unchanged.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::diagnostic_text::display;
use crate::layer_manifest::schema::{self, Reference};
use crate::layer_manifest::{
    self, LayerManifestDiscovery, LayerManifestError, LoadedLayerManifest,
};

/// Indexes are zero-based, relative to the original declaration list or argument list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceOrigin {
    Current,
    Existing { index: usize, reference: String },
    Argument { index: usize, reference: String },
}

impl fmt::Display for SourceOrigin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => formatter.write_str("the current layer"),
            Self::Existing { index, reference } => {
                write!(
                    formatter,
                    "existing dependency {} ('{}')",
                    index + 1,
                    display(reference)
                )
            }
            Self::Argument { index, reference } => {
                write!(
                    formatter,
                    "source argument {} ('{}')",
                    index + 1,
                    display(reference)
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependencySource {
    /// Canonical directory identity, separate from the unchanged reference text.
    Local { directory: PathBuf },
    /// Syntax-checked only; no existence or availability claim is made.
    Published { version: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredDependency {
    pub name: String,
    pub reference: String,
    pub source: DependencySource,
    pub origin: SourceOrigin,
}

/// A verified batch. Existing declarations and both outcome groups retain their
/// order. Already-present entries retain the original stored reference, not an alias
/// supplied in this invocation. Repeated new arguments occur only in `added`.
#[derive(Debug)]
pub struct AddPlan {
    current: Arc<LoadedLayerManifest>,
    directory: PathBuf,
    existing: Vec<DeclaredDependency>,
    added: Vec<DeclaredDependency>,
    already_present: Vec<DeclaredDependency>,
    discoveries: Vec<LayerManifestDiscovery>,
}

impl AddPlan {
    pub fn current(&self) -> &LoadedLayerManifest {
        &self.current
    }

    /// The base used for both supplied paths and stored local references.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn existing(&self) -> &[DeclaredDependency] {
        &self.existing
    }

    pub fn added(&self) -> &[DeclaredDependency] {
        &self.added
    }

    pub fn already_present(&self) -> &[DeclaredDependency] {
        &self.already_present
    }

    /// One observation per canonical directory, in first-observation order.
    pub fn discoveries(&self) -> &[LayerManifestDiscovery] {
        &self.discoveries
    }

    /// Only these strings should be passed to the source-preserving editor.
    pub fn new_references(&self) -> impl ExactSizeIterator<Item = &str> {
        self.added.iter().map(|entry| entry.reference.as_str())
    }

    pub fn is_noop(&self) -> bool {
        self.added.is_empty()
    }
}

#[derive(Debug)]
pub struct DependencyFailure {
    pub origin: SourceOrigin,
    pub error: DependencyError,
}

/// Verification failed before any mutation. Current prerequisites fail alone;
/// otherwise existing-entry failures precede supplied-source failures in input order.
#[derive(Debug)]
pub struct AddFailure {
    pub directory: Option<PathBuf>,
    pub discoveries: Vec<LayerManifestDiscovery>,
    pub errors: Vec<DependencyFailure>,
}

impl fmt::Display for AddFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Dependency verification failed ({} error(s)).",
            self.errors.len()
        )
    }
}

impl std::error::Error for AddFailure {}

#[derive(Debug)]
pub enum DependencyError {
    Schema(schema::Error),
    LayerManifest(Arc<LayerManifestError>),
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    NotDirectory {
        path: PathBuf,
    },
    MissingCurrent {
        invocation: PathBuf,
    },
    CurrentSymlink {
        path: PathBuf,
    },
    /// A host-interpretation refusal, not a portable schema-validity failure.
    UnsupportedPath {
        reference: String,
        reason: &'static str,
    },
    PublishedAdditionUnsupported {
        name: String,
        version: String,
    },
    NameConflict {
        name: String,
        previous: SourceOrigin,
        previous_directory: Option<PathBuf>,
        directory: Option<PathBuf>,
    },
}

impl fmt::Display for DependencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(error) => error.fmt(formatter),
            Self::LayerManifest(error) => error.fmt(formatter),
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
            Self::NotDirectory { path } => write!(
                formatter,
                "Local source '{}' must be a directory containing a layer manifest.",
                display(path.display())
            ),
            Self::MissingCurrent { invocation } => write!(
                formatter,
                "No layer manifest found in '{}' or its parents through the filesystem root. Run 'hydragen init' in the intended layer directory first.",
                display(invocation.display())
            ),
            Self::CurrentSymlink { path } => write!(
                formatter,
                "Cannot edit symlinked current layer manifest '{}'. Use a regular layer manifest file instead; the link and its target have not been changed.",
                display(path.display())
            ),
            Self::UnsupportedPath { reference, reason } => write!(
                formatter,
                "Cannot resolve local path '{}' on this host: {reason}. Supply an explicit native path without translating the stored reference.",
                display(reference)
            ),
            Self::PublishedAdditionUnsupported { name, version } => write!(
                formatter,
                "Adding published layers is not supported yet ('{}@{}'). Supply an explicit local path instead.",
                display(name),
                display(version)
            ),
            Self::NameConflict {
                name,
                previous,
                previous_directory,
                directory,
            } => {
                write!(
                    formatter,
                    "Layer name '{}' conflicts with {previous}",
                    display(name)
                )?;
                if let Some(path) = previous_directory {
                    write!(formatter, " in '{}'", display(path.display()))?;
                }
                if let Some(path) = directory {
                    write!(
                        formatter,
                        "; the incoming layer is in '{}'",
                        display(path.display())
                    )?;
                }
                formatter.write_str(". Each direct dependency must have a unique name, different from the current layer; existing declarations are not replaced.")
            }
        }
    }
}

impl std::error::Error for DependencyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Schema(error) => Some(error),
            Self::LayerManifest(error) => Some(error.as_ref()),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn directory_identity(path: &Path) -> Result<PathBuf, DependencyError> {
    // Do not lexically simplify first: symlink/.. resolution belongs to the OS.
    let directory = fs::canonicalize(path).map_err(|source| DependencyError::Io {
        action: "resolve local directory",
        path: path.to_owned(),
        source,
    })?;
    let metadata = fs::metadata(&directory).map_err(|source| DependencyError::Io {
        action: "inspect local directory",
        path: directory.clone(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(DependencyError::NotDirectory {
            path: path.to_owned(),
        });
    }
    Ok(directory)
}

/// Find the nearest directory with any occupied supported filename. This does not
/// load it, try lower-priority files, or stop at Git markers. Shared `discover`
/// remains a single-directory operation.
pub fn discover_current(invocation: &Path) -> Result<LayerManifestDiscovery, DependencyError> {
    let directory = directory_identity(invocation)?;
    search_ancestors(&directory, layer_manifest::discover)
}

fn search_ancestors(
    directory: &Path,
    mut discover: impl FnMut(&Path) -> Result<LayerManifestDiscovery, LayerManifestError>,
) -> Result<LayerManifestDiscovery, DependencyError> {
    for ancestor in directory.ancestors() {
        let discovery =
            discover(ancestor).map_err(|error| DependencyError::LayerManifest(Arc::new(error)))?;
        if discovery.selected().is_some() {
            return Ok(discovery);
        }
    }
    Err(DependencyError::MissingCurrent {
        invocation: directory.to_owned(),
    })
}

/// Interpret only schema-classified local text for this host. The checks below
/// recognise OS path conventions; they do not define name, version, or portable
/// reference validity. In particular, no filesystem-dependent published fallback exists.
fn local_path(base: &Path, reference: &str) -> Result<PathBuf, DependencyError> {
    let drive =
        reference.as_bytes().get(1) == Some(&b':') && reference.as_bytes()[0].is_ascii_alphabetic();
    let drive_absolute = drive && matches!(reference.as_bytes().get(2), Some(b'/' | b'\\'));
    let reason = if drive && !drive_absolute {
        Some("drive-relative paths depend on an implicit drive working directory")
    } else if !cfg!(windows)
        && (drive
            || reference.starts_with('\\')
            || reference.starts_with(".\\")
            || reference.starts_with("..\\"))
    {
        Some("Windows drive or backslash paths are not native here")
    } else if cfg!(windows)
        && !Path::new(reference).is_absolute()
        && (reference.starts_with('/') || reference.starts_with('\\'))
    {
        Some("root-relative paths do not specify a drive")
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(DependencyError::UnsupportedPath {
            reference: reference.to_owned(),
            reason,
        });
    }
    Ok(base.join(reference))
}

type LoadedResult = Result<Arc<LoadedLayerManifest>, Arc<LayerManifestError>>;

struct CachedDirectory {
    discovery: Result<LayerManifestDiscovery, Arc<LayerManifestError>>,
    loaded: Option<LoadedResult>,
}

#[derive(Default)]
struct DirectoryCache {
    directories: HashMap<PathBuf, CachedDirectory>,
    discoveries: Vec<LayerManifestDiscovery>,
}

impl DirectoryCache {
    fn insert(
        &mut self,
        directory: PathBuf,
        discovery: Result<LayerManifestDiscovery, LayerManifestError>,
    ) {
        if let Ok(observation) = &discovery {
            self.discoveries.push(observation.clone());
        }
        self.directories.insert(
            directory,
            CachedDirectory {
                discovery: discovery.map_err(Arc::new),
                loaded: None,
            },
        );
    }

    fn load(&mut self, directory: &Path) -> Result<Arc<LoadedLayerManifest>, DependencyError> {
        if !self.directories.contains_key(directory) {
            self.insert(directory.to_owned(), layer_manifest::discover(directory));
        }
        let cached = self
            .directories
            .get_mut(directory)
            .expect("directory inserted above");
        let discovery = cached
            .discovery
            .as_ref()
            .map_err(|error| DependencyError::LayerManifest(Arc::clone(error)))?;
        cached
            .loaded
            .get_or_insert_with(|| discovery.load().map(Arc::new).map_err(Arc::new))
            .clone()
            .map_err(DependencyError::LayerManifest)
    }

    fn local(
        &mut self,
        base: &Path,
        reference: &str,
        origin: SourceOrigin,
    ) -> Result<DeclaredDependency, DependencyError> {
        let path = local_path(base, reference)?;
        let directory = directory_identity(&path)?;
        let loaded = self.load(&directory)?;
        Ok(DeclaredDependency {
            name: loaded.metadata.name.clone(),
            reference: reference.to_owned(),
            source: DependencySource::Local { directory },
            origin,
        })
    }
}

#[derive(Clone)]
struct NameClaim {
    origin: SourceOrigin,
    directory: Option<PathBuf>,
}

impl NameClaim {
    fn dependency(entry: &DeclaredDependency) -> Self {
        Self {
            origin: entry.origin.clone(),
            directory: match &entry.source {
                DependencySource::Local { directory } => Some(directory.clone()),
                DependencySource::Published { .. } => None,
            },
        }
    }

    fn conflict(&self, entry: &DeclaredDependency) -> DependencyError {
        DependencyError::NameConflict {
            name: entry.name.clone(),
            previous: self.origin.clone(),
            previous_directory: self.directory.clone(),
            directory: Self::dependency(entry).directory,
        }
    }
}

/// Verify the current layer, every existing direct declaration, and a complete
/// supplied batch. An empty source slice performs the same read-only checks for
/// other callers. The current selected file must be regular rather than a symlink;
/// dependency layer manifest symlinks remain readable. No staging or writes occur.
pub fn prepare_add(invocation: &Path, sources: &[String]) -> Result<AddPlan, AddFailure> {
    let mut cache = DirectoryCache::default();
    let discovery = discover_current(invocation).map_err(|error| AddFailure {
        directory: None,
        discoveries: Vec::new(),
        errors: vec![DependencyFailure {
            origin: SourceOrigin::Current,
            error,
        }],
    })?;
    let directory = discovery.directory.clone();
    let path = discovery
        .selected()
        .expect("ancestor search found an occupied entry")
        .path
        .clone();
    cache.insert(directory.clone(), Ok(discovery));
    let current = (|| {
        let metadata = fs::symlink_metadata(&path).map_err(|source| DependencyError::Io {
            action: "inspect current layer manifest",
            path: path.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() {
            return Err(DependencyError::CurrentSymlink { path });
        }
        cache.load(&directory)
    })();
    let current = match current {
        Ok(current) => current,
        Err(error) => {
            return Err(AddFailure {
                directory: Some(directory),
                discoveries: cache.discoveries,
                errors: vec![DependencyFailure {
                    origin: SourceOrigin::Current,
                    error,
                }],
            });
        }
    };

    let mut claims = HashMap::from([(
        current.metadata.name.clone(),
        NameClaim {
            origin: SourceOrigin::Current,
            directory: Some(directory.clone()),
        },
    )]);
    let mut existing = Vec::new();
    let mut errors = Vec::new();
    // The guarded decoder and complete schema validation already establish shape.
    let references = current
        .document
        .get("dependencies")
        .and_then(|value| value.as_array());
    for (index, value) in references.into_iter().flatten().enumerate() {
        let reference = value.as_str().expect("validated dependency string");
        let origin = SourceOrigin::Existing {
            index,
            reference: reference.to_owned(),
        };
        let entry = match schema::classify_reference(reference) {
            Ok(Reference::Local(local)) => cache.local(&directory, local, origin.clone()),
            Ok(Reference::Published { name, version }) => Ok(DeclaredDependency {
                name: name.to_owned(),
                reference: reference.to_owned(),
                source: DependencySource::Published {
                    version: version.to_owned(),
                },
                origin: origin.clone(),
            }),
            Err(error) => Err(DependencyError::Schema(error)),
        };
        match entry {
            Ok(entry) => {
                if let Some(previous) = claims.get(&entry.name) {
                    errors.push(DependencyFailure {
                        origin,
                        error: previous.conflict(&entry),
                    });
                } else {
                    claims.insert(entry.name.clone(), NameClaim::dependency(&entry));
                }
                existing.push(entry);
            }
            Err(error) => errors.push(DependencyFailure { origin, error }),
        }
    }

    let mut added = Vec::new();
    let mut already_present = Vec::new();
    let mut reported_present = HashSet::new();
    for (index, reference) in sources.iter().enumerate() {
        let origin = SourceOrigin::Argument {
            index,
            reference: reference.clone(),
        };
        let entry = match schema::classify_reference(reference) {
            Ok(Reference::Local(local)) => cache.local(&directory, local, origin.clone()),
            Ok(Reference::Published { name, version }) => {
                Err(DependencyError::PublishedAdditionUnsupported {
                    name: name.to_owned(),
                    version: version.to_owned(),
                })
            }
            Err(error) => Err(DependencyError::Schema(error)),
        };
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(DependencyFailure { origin, error });
                continue;
            }
        };
        if let Some(previous) = claims.get(&entry.name) {
            let incoming = NameClaim::dependency(&entry);
            if previous.origin != SourceOrigin::Current && previous.directory == incoming.directory
            {
                if matches!(previous.origin, SourceOrigin::Existing { .. })
                    && reported_present.insert(entry.name.clone())
                {
                    let SourceOrigin::Existing { reference, .. } = &previous.origin else {
                        unreachable!("existing claim checked above");
                    };
                    already_present.push(DeclaredDependency {
                        name: entry.name,
                        reference: reference.clone(),
                        source: entry.source,
                        origin: previous.origin.clone(),
                    });
                }
                // A repeat of a new entry stays exclusively in Added.
            } else {
                errors.push(DependencyFailure {
                    origin,
                    error: previous.conflict(&entry),
                });
            }
        } else {
            claims.insert(entry.name.clone(), NameClaim::dependency(&entry));
            added.push(entry);
        }
    }
    if !errors.is_empty() {
        return Err(AddFailure {
            directory: Some(directory),
            discoveries: cache.discoveries,
            errors,
        });
    }
    Ok(AddPlan {
        current,
        directory,
        existing,
        added,
        already_present,
        discoveries: cache.discoveries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_manifest::{LayerManifestCandidate, LayerManifestFormat};

    #[test]
    fn directory_cache_reuses_success_and_discovery_failure_without_rechecking() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = fs::canonicalize(temporary.path()).unwrap();
        let path = directory.join("layer.yaml");
        fs::write(&path, "name: before\n").unwrap();
        let mut cache = DirectoryCache::default();
        let first = cache.load(&directory).unwrap();
        fs::write(&path, "name: after\n").unwrap();
        let second = cache.load(&directory).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(second.source, "name: before\n");
        assert_eq!(cache.discoveries.len(), 1);

        let inaccessible = directory.join("inaccessible");
        cache.insert(
            inaccessible.clone(),
            Err(LayerManifestError::Io {
                action: "inspect layer manifest path",
                path: inaccessible.join("layer.yaml"),
                source: io::ErrorKind::PermissionDenied.into(),
            }),
        );
        let first = cache.load(&inaccessible).unwrap_err();
        let second = cache.load(&inaccessible).unwrap_err();
        let (DependencyError::LayerManifest(first), DependencyError::LayerManifest(second)) =
            (first, second)
        else {
            panic!("expected original inspection failure");
        };
        assert!(Arc::ptr_eq(&first, &second));
        assert!(
            matches!(first.as_ref(), LayerManifestError::Io { source, .. } if source.kind() == io::ErrorKind::PermissionDenied)
        );
        // Earlier observations survive a later directory's inspection failure.
        assert_eq!(cache.discoveries.len(), 1);
    }

    #[test]
    fn search_visits_filesystem_root_and_preserves_inspection_errors() {
        let start = fs::canonicalize(std::env::temp_dir()).unwrap();
        let expected: Vec<_> = start.ancestors().map(Path::to_owned).collect();
        let root = expected.last().unwrap();
        let mut visited = Vec::new();
        let result = search_ancestors(&start, |directory| {
            visited.push(directory.to_owned());
            Ok(LayerManifestDiscovery {
                directory: directory.to_owned(),
                candidates: if directory == root {
                    vec![LayerManifestCandidate {
                        path: directory.join("layer.yaml"),
                        format: LayerManifestFormat::Yaml,
                    }]
                } else {
                    Vec::new()
                },
            })
        })
        .unwrap();
        assert_eq!(visited, expected);
        assert_eq!(&result.directory, root);
        let mut visited = Vec::new();
        let error = search_ancestors(&start, |directory| {
            visited.push(directory.to_owned());
            Err(LayerManifestError::Io {
                action: "inspect layer manifest path",
                path: directory.join("layer.yaml"),
                source: io::ErrorKind::PermissionDenied.into(),
            })
        })
        .unwrap_err();
        assert_eq!(visited, std::slice::from_ref(&start));
        assert!(matches!(error, DependencyError::LayerManifest(_)));
        let mut visited = Vec::new();
        let error = search_ancestors(&start, |directory| {
            visited.push(directory.to_owned());
            Ok(LayerManifestDiscovery {
                directory: directory.to_owned(),
                candidates: Vec::new(),
            })
        })
        .unwrap_err();
        assert_eq!(visited, expected);
        assert!(matches!(error, DependencyError::MissingCurrent { .. }));
    }
}
