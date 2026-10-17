use std::fs;

use hydragen::{InitError, LayerManifestFormat, Metadata, create_init, initialize, inspect_init};

fn metadata(name: &str) -> Metadata {
    Metadata {
        name: name.into(),
        author: None,
        description: None,
    }
}

#[test]
fn independently_prepared_formats_both_succeed_without_a_lock() {
    for (first, second) in [
        (LayerManifestFormat::Yaml, LayerManifestFormat::Json),
        (LayerManifestFormat::Json, LayerManifestFormat::Yaml),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let prepare = || inspect_init(directory.path()).unwrap().prepare().unwrap();
        let one = prepare();
        let two = prepare();
        assert_eq!(one.directory(), directory.path().canonicalize().unwrap());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
        let one = create_init(one, &metadata("first"), first).unwrap();
        assert!(!one.observation.unwrap().has_duplicates());
        let bytes = fs::read(&one.path).unwrap();
        let two = create_init(two, &metadata("second"), second).unwrap();
        let discovery = two.observation.unwrap();
        assert!(discovery.has_duplicates());
        assert_eq!(
            discovery.selected().unwrap().format,
            LayerManifestFormat::Yaml
        );
        assert_eq!(two.path.file_name().unwrap(), second.filename());
        assert_eq!(fs::read(&one.path).unwrap(), bytes);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}

#[test]
fn alternate_format_after_preparation_is_only_an_observation() {
    for created in LayerManifestFormat::ALL {
        for appeared in LayerManifestFormat::ALL {
            if created == appeared {
                continue;
            }
            let directory = tempfile::tempdir().unwrap();
            let prepared = inspect_init(directory.path()).unwrap().prepare().unwrap();
            let other = directory.path().join(appeared.filename());
            fs::write(&other, "not parsed by init").unwrap();
            let outcome = create_init(prepared, &metadata("new"), created).unwrap();
            assert_eq!(outcome.path.file_name().unwrap(), created.filename());
            let discovery = outcome.observation.unwrap();
            assert!(discovery.has_duplicates());
            assert_eq!(
                discovery.selected().unwrap().format,
                LayerManifestFormat::ALL
                    .into_iter()
                    .find(|format| *format == created || *format == appeared)
                    .unwrap()
            );
            assert_eq!(fs::read_to_string(other).unwrap(), "not parsed by init");
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
        }
    }
}

#[test]
fn exact_target_collision_retains_error_and_duplicate_diagnostics_without_retry() {
    for format in LayerManifestFormat::ALL {
        let directory = tempfile::tempdir().unwrap();
        let prepared = inspect_init(directory.path()).unwrap().prepare().unwrap();
        for candidate in LayerManifestFormat::ALL {
            fs::write(directory.path().join(candidate.filename()), "keep").unwrap();
        }
        let failure = create_init(prepared, &metadata("replacement"), format).unwrap_err();
        assert!(
            matches!(failure.error, InitError::ExistingLayerManifest { ref path } if path.file_name().unwrap() == format.filename())
        );
        let discovery = failure.discovery.unwrap();
        assert_eq!(discovery.candidates.len(), 5);
        assert_eq!(
            discovery.selected().unwrap().format,
            LayerManifestFormat::Yaml
        );
        for candidate in LayerManifestFormat::ALL {
            assert_eq!(
                fs::read_to_string(directory.path().join(candidate.filename())).unwrap(),
                "keep"
            );
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 5);
    }
}

#[cfg(unix)]
#[test]
fn exact_target_directory_and_dangling_symlink_collisions_preserve_entries() {
    use std::os::unix::fs::symlink;
    for format in LayerManifestFormat::ALL {
        for link in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let prepared = inspect_init(directory.path()).unwrap().prepare().unwrap();
            let path = directory.path().join(format.filename());
            if link {
                symlink("missing", &path).unwrap();
            } else {
                fs::create_dir(&path).unwrap();
            }
            let failure = create_init(prepared, &metadata("new"), format).unwrap_err();
            assert!(matches!(
                failure.error,
                InitError::ExistingLayerManifest { .. }
            ));
            if link {
                assert_eq!(
                    fs::read_link(path).unwrap(),
                    std::path::Path::new("missing")
                );
            } else {
                assert!(path.is_dir());
            }
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn initial_inspection_retains_duplicates_before_refusal_without_reading_them() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("layer.yaml")).unwrap();
    fs::write(directory.path().join("layer.json"), "invalid").unwrap();
    let inspection = inspect_init(directory.path()).unwrap();
    assert!(inspection.discovery().has_duplicates());
    assert_eq!(
        inspection.discovery().selected().unwrap().format,
        LayerManifestFormat::Yaml
    );
    assert!(matches!(
        inspection.prepare(),
        Err(InitError::ExistingLayerManifest { .. })
    ));
    assert_eq!(
        fs::read_to_string(directory.path().join("layer.json")).unwrap(),
        "invalid"
    );
}

#[test]
fn prepared_creation_validates_before_opening_the_target() {
    for format in LayerManifestFormat::ALL {
        let directory = tempfile::tempdir().unwrap();
        let prepared = inspect_init(directory.path()).unwrap().prepare().unwrap();
        let failure = create_init(prepared, &metadata(""), format).unwrap_err();
        assert!(matches!(
            failure.error,
            InitError::InvalidMetadata { field: "name" }
        ));
        assert!(failure.discovery.is_none());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn convenience_initialization_defaults_to_yaml() {
    let directory = tempfile::tempdir().unwrap();
    let path = initialize(directory.path(), &metadata("example")).unwrap();
    assert_eq!(path.file_name().unwrap(), "layer.yaml");
    assert_eq!(fs::read_to_string(path).unwrap(), "name: example\n");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}
