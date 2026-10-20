use std::fs;
use std::path::{Path, PathBuf};

use hydragen::dependencies::{
    AddFailure, DependencyError, DependencySource, SourceOrigin, discover_current, prepare_add,
};
use hydragen::layer_manifest::{self, LayerManifestError, LayerManifestFormat, schema};
use serde_json::json;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn write_layer(directory: &Path, name: &str, dependencies: &[&str]) {
    fs::create_dir_all(directory).unwrap();
    fs::write(
        directory.join("layer.json"),
        json!({"name": name, "dependencies": dependencies, "custom": {"untouched": true}})
            .to_string(),
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let app = temporary.path().join("app");
    write_layer(&app, "app", &[]);
    (temporary, app)
}

fn argument_indexes(failure: &AddFailure) -> Vec<usize> {
    failure
        .errors
        .iter()
        .map(|failure| match failure.origin {
            SourceOrigin::Argument { index, .. } => index,
            ref other => panic!("expected argument error, got {other:?}"),
        })
        .collect()
}

fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, directory: &Path, result: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            let bytes = if metadata.file_type().is_symlink() {
                fs::read_link(&path)
                    .unwrap()
                    .to_string_lossy()
                    .as_bytes()
                    .to_vec()
            } else if metadata.is_dir() {
                Vec::new()
            } else {
                fs::read(&path).unwrap()
            };
            result.push((path.strip_prefix(root).unwrap().to_owned(), bytes));
            if metadata.is_dir() {
                walk(root, &path, result);
            }
        }
    }
    let mut result = Vec::new();
    walk(root, root, &mut result);
    result
}

#[test]
fn nested_invocation_crosses_git_boundaries_and_uses_the_declaring_base() {
    let (temporary, app) = fixture();
    let shared = temporary.path().join("Shared 配置 @literal");
    write_layer(&shared, "@scope/.config", &[]);
    let nested = app.join("src/deep");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir(app.join("src/.git")).unwrap();
    assert!(
        layer_manifest::discover(&nested)
            .unwrap()
            .selected()
            .is_none()
    );
    let supplied = args(&["../Shared 配置 @literal/./"]);
    let root_plan = prepare_add(&app, &supplied).unwrap();
    let nested_plan = prepare_add(&nested, &supplied).unwrap();
    assert_eq!(root_plan.added(), nested_plan.added());
    assert_eq!(nested_plan.directory(), fs::canonicalize(&app).unwrap());
    assert_eq!(
        nested_plan.new_references().collect::<Vec<_>>(),
        ["../Shared 配置 @literal/./"]
    );
    assert_eq!(nested_plan.added()[0].name, "@scope/.config");
}

#[test]
fn nearest_occupied_directory_blocks_valid_parent_and_lower_priority_fallback() {
    for content in [
        "broken: [",
        "name: Uppercase",
        "name: child\ndependencies: null",
    ] {
        let (_temporary, app) = fixture();
        let child = app.join("child");
        write_layer(&child, "child", &[]);
        fs::write(child.join("layer.yaml"), content).unwrap();
        let failure = prepare_add(&child, &args(&["../missing"])).unwrap_err();
        assert_eq!(failure.errors.len(), 1);
        assert_eq!(failure.errors[0].origin, SourceOrigin::Current);
        assert_eq!(failure.discoveries.len(), 1);
        let observation = &failure.discoveries[0];
        assert!(observation.has_duplicates());
        assert_eq!(
            observation.selected().unwrap().format,
            LayerManifestFormat::Yaml
        );
        assert_eq!(observation.directory, fs::canonicalize(child).unwrap());
    }
}

#[test]
fn current_priority_selects_each_supported_format_without_parent_search_changes() {
    for format in LayerManifestFormat::ALL {
        let (_temporary, app) = fixture();
        fs::remove_file(app.join("layer.json")).unwrap();
        let text = match format {
            LayerManifestFormat::Yaml | LayerManifestFormat::Yml => "# comment\nname: app\n",
            LayerManifestFormat::Toml => "# comment\nname = 'app'\n",
            LayerManifestFormat::Json => "{\"name\":\"app\"}",
            LayerManifestFormat::Jsonc => "{/* comment */\"name\":\"app\",}",
        };
        fs::write(app.join(format.filename()), text).unwrap();
        let plan = prepare_add(&app, &[]).unwrap();
        assert_eq!(plan.current().format, format);
        assert_eq!(plan.current().source, text);
        assert!(plan.is_noop());
    }
}

#[test]
fn targets_use_each_selected_format_and_validate_it_without_lower_priority_fallback() {
    for (index, format) in LayerManifestFormat::ALL.into_iter().enumerate() {
        let (temporary, app) = fixture();
        let target = temporary.path().join("target");
        fs::create_dir(&target).unwrap();
        for lower in &LayerManifestFormat::ALL[index + 1..] {
            fs::write(target.join(lower.filename()), "not selected or parsed").unwrap();
        }
        let text = match format {
            LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
                "name: target\ndependencies: ['../not-followed']"
            }
            LayerManifestFormat::Toml => "name = 'target'\ndependencies = ['../not-followed']",
            LayerManifestFormat::Json => {
                "{\"name\":\"target\",\"dependencies\":[\"../not-followed\"]}"
            }
            LayerManifestFormat::Jsonc => {
                "{/* comment */\"name\":\"target\",\"dependencies\":[\"../not-followed\"],}"
            }
        };
        let selected = target.join(format.filename());
        fs::write(&selected, text).unwrap();
        let plan = prepare_add(&app, &args(&["../target"])).unwrap();
        assert_eq!(plan.added()[0].name, "target");
        assert_eq!(plan.discoveries()[1].selected().unwrap().format, format);
        assert_eq!(plan.discoveries()[1].candidates.len(), 5 - index);
        fs::write(&selected, "invalid").unwrap();
        let failure = prepare_add(&app, &args(&["../target"])).unwrap_err();
        assert_eq!(failure.discoveries[1].selected().unwrap().format, format);
        assert_eq!(failure.discoveries[1].candidates.len(), 5 - index);
    }
}

#[test]
fn stored_local_paths_use_the_same_base_for_root_and_nested_verification() {
    let (temporary, app) = fixture();
    write_layer(&temporary.path().join("old"), "old", &[]);
    write_layer(&app, "app", &["../old"]);
    let nested = app.join("src/deep");
    fs::create_dir_all(&nested).unwrap();
    let root = prepare_add(&app, &[]).unwrap();
    let nested = prepare_add(&nested, &[]).unwrap();
    assert_eq!(nested.existing(), root.existing());
    assert_eq!(nested.discoveries(), root.discoveries());
}

#[test]
fn invalid_utf8_in_current_and_target_files_retains_the_selected_discoveries() {
    let (temporary, app) = fixture();
    fs::write(app.join("layer.yaml"), b"name: \xff").unwrap();
    let failure = prepare_add(&app, &[]).unwrap_err();
    assert!(failure.discoveries[0].has_duplicates());
    assert!(
        matches!(&failure.errors[0].error, DependencyError::LayerManifest(error) if matches!(error.as_ref(), LayerManifestError::Io { source, .. } if source.kind() == std::io::ErrorKind::InvalidData))
    );
    fs::remove_file(app.join("layer.yaml")).unwrap();
    let target = temporary.path().join("target");
    write_layer(&target, "fallback", &[]);
    fs::write(target.join("layer.yaml"), b"name: \xff").unwrap();
    let failure = prepare_add(&app, &args(&["../target"])).unwrap_err();
    assert!(failure.discoveries[1].has_duplicates());
    assert!(
        matches!(&failure.errors[0].error, DependencyError::LayerManifest(error) if matches!(error.as_ref(), LayerManifestError::Io { source, .. } if source.kind() == std::io::ErrorKind::InvalidData))
    );
}

#[test]
fn all_current_candidates_are_reported_in_priority_order() {
    let (_temporary, app) = fixture();
    for format in LayerManifestFormat::ALL {
        fs::write(app.join(format.filename()), "invalid").unwrap();
    }
    let failure = prepare_add(&app, &[]).unwrap_err();
    let formats: Vec<_> = failure.discoveries[0]
        .candidates
        .iter()
        .map(|candidate| candidate.format)
        .collect();
    assert_eq!(formats, LayerManifestFormat::ALL);
}

#[test]
fn unavailable_current_prerequisite_stops_before_any_argument_inspection() {
    let (temporary, app) = fixture();
    let target = temporary.path().join("target");
    write_layer(&target, "target", &[]);
    fs::write(target.join("layer.yaml"), "name: target").unwrap();
    fs::write(app.join("layer.json"), "{bad").unwrap();
    let failure = prepare_add(&app, &args(&["../target", "invalid", "../missing"])).unwrap_err();
    assert_eq!(failure.errors.len(), 1);
    assert_eq!(failure.errors[0].origin, SourceOrigin::Current);
    assert_eq!(failure.discoveries.len(), 1);
}

#[test]
fn non_directory_invocation_and_sources_are_structured_errors() {
    let (temporary, app) = fixture();
    let file = temporary.path().join("file");
    fs::write(&file, "not a directory").unwrap();
    assert!(matches!(
        discover_current(&file),
        Err(DependencyError::NotDirectory { .. })
    ));
    assert!(matches!(
        discover_current(&temporary.path().join("missing")),
        Err(DependencyError::Io { .. })
    ));
    let failure = prepare_add(&app, &args(&["../file"])).unwrap_err();
    assert!(matches!(
        failure.errors[0].error,
        DependencyError::NotDirectory { .. }
    ));
}

#[test]
fn target_ancestors_are_not_searched_and_chains_are_not_traversed() {
    let (temporary, app) = fixture();
    let target = temporary.path().join("target");
    write_layer(&target, "target", &["../unavailable", "app@1.2.3"]);
    let empty = target.join("empty");
    fs::create_dir(&empty).unwrap();
    let plan = prepare_add(&app, &args(&["../target"])).unwrap();
    assert_eq!(plan.added()[0].name, "target");
    assert_eq!(plan.discoveries().len(), 2);
    let failure = prepare_add(&app, &args(&["../target/empty"])).unwrap_err();
    assert!(
        matches!(&failure.errors[0].error, DependencyError::LayerManifest(error) if matches!(error.as_ref(), LayerManifestError::Missing { .. }))
    );
    assert_eq!(failure.discoveries.len(), 2);
    assert!(failure.discoveries[1].candidates.is_empty());
    // Valid syntax is still required in the untraversed declarations.
    write_layer(&target, "target", &["bare-name"]);
    assert!(prepare_add(&app, &args(&["../target"])).is_err());
}

#[test]
fn existing_broken_sources_and_independent_arguments_are_all_checked_in_order() {
    let (temporary, app) = fixture();
    write_layer(&app, "app", &["../old-missing", "../old-bad"]);
    fs::create_dir(temporary.path().join("old-bad")).unwrap();
    fs::write(
        temporary.path().join("old-bad/layer.yaml"),
        "name: Uppercase",
    )
    .unwrap();
    let bad = temporary.path().join("new-bad");
    fs::create_dir(&bad).unwrap();
    fs::write(bad.join("layer.yaml"), "name: [").unwrap();
    fs::write(bad.join("layer.json"), "{\"name\":\"fallback\"}").unwrap();
    let failure =
        prepare_add(&app, &args(&["../new-missing", "../new-bad", "pkg@1.2.3"])).unwrap_err();
    assert_eq!(failure.errors.len(), 5);
    assert!(matches!(
        failure.errors[0].origin,
        SourceOrigin::Existing { index: 0, .. }
    ));
    assert!(matches!(
        failure.errors[1].origin,
        SourceOrigin::Existing { index: 1, .. }
    ));
    assert_eq!(
        failure.errors[2..]
            .iter()
            .map(|item| match item.origin {
                SourceOrigin::Argument { index, .. } => index,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(failure.discoveries.len(), 3);
    assert!(failure.discoveries[2].has_duplicates());
}

#[test]
fn batch_failures_include_every_independent_source_in_argument_order() {
    let (temporary, app) = fixture();
    write_layer(&temporary.path().join("good"), "good", &[]);
    fs::create_dir(temporary.path().join("bad")).unwrap();
    fs::write(temporary.path().join("bad/layer.json"), "{").unwrap();
    let failure = prepare_add(
        &app,
        &args(&[
            "../missing",
            "../good",
            "../bad",
            "pkg@latest",
            "pkg@1.2.3",
            ".",
        ]),
    )
    .unwrap_err();
    assert_eq!(argument_indexes(&failure), [0, 2, 3, 4, 5]);
    assert!(matches!(
        failure.errors[2].error,
        DependencyError::Schema(_)
    ));
    assert!(matches!(
        failure.errors[3].error,
        DependencyError::PublishedAdditionUnsupported { .. }
    ));
    assert!(
        matches!(failure.errors[4].error, DependencyError::NameConflict { ref previous, .. } if *previous == SourceOrigin::Current)
    );
}

#[test]
fn references_stay_unchanged_and_first_new_occurrence_controls_order() {
    let (temporary, app) = fixture();
    let a = temporary.path().join("A @literal");
    let b = temporary.path().join("b");
    write_layer(&a, "a", &[]);
    write_layer(&b, "b", &[]);
    let absolute = b.to_str().unwrap().to_owned();
    let sources = args(&["../A @literal/./", &absolute, "../A @literal", "../b"]);
    let plan = prepare_add(&app, &sources).unwrap();
    assert_eq!(
        plan.new_references().collect::<Vec<_>>(),
        ["../A @literal/./", absolute.as_str()]
    );
    assert_eq!(
        plan.added()
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert!(plan.already_present().is_empty());
    assert_eq!(plan.discoveries().len(), 3);
    assert!(!plan.is_noop());
}

#[test]
fn standalone_parent_is_an_explicit_local_source_and_preserves_its_spelling() {
    let (temporary, app) = fixture();
    write_layer(temporary.path(), "parent", &[]);
    let plan = prepare_add(&app, &args(&["..", "../."])).unwrap();
    assert_eq!(plan.added().len(), 1);
    assert_eq!(plan.added()[0].name, "parent");
    assert_eq!(plan.added()[0].reference, "..");
    assert!(plan.already_present().is_empty());
}

#[test]
fn mixed_outcomes_preserve_existing_order_and_only_report_requested_preexisting_names() {
    let (temporary, app) = fixture();
    for name in ["old-a", "old-b", "unrequested", "new-a", "new-b"] {
        write_layer(&temporary.path().join(name), name, &[]);
    }
    write_layer(
        &app,
        "app",
        &[
            "../old-a/./",
            "../old-b",
            "../unrequested",
            "offline@1.2.3+build.007",
        ],
    );
    let plan = prepare_add(
        &app,
        &args(&[
            "../new-b",
            "../old-b/./",
            "../new-a",
            "../old-a",
            "../new-b/./",
            "../old-b",
        ]),
    )
    .unwrap();
    assert_eq!(
        plan.existing()
            .iter()
            .map(|item| item.reference.as_str())
            .collect::<Vec<_>>(),
        [
            "../old-a/./",
            "../old-b",
            "../unrequested",
            "offline@1.2.3+build.007"
        ]
    );
    assert_eq!(
        plan.added()
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["new-b", "new-a"]
    );
    assert_eq!(
        plan.already_present()
            .iter()
            .map(|item| (item.name.as_str(), item.reference.as_str()))
            .collect::<Vec<_>>(),
        [("old-b", "../old-b"), ("old-a", "../old-a/./")]
    );
}

#[test]
fn repeat_only_and_empty_source_verification_are_noops() {
    let (temporary, app) = fixture();
    write_layer(&temporary.path().join("old"), "old", &[]);
    write_layer(&app, "app", &["../old/./"]);
    let empty = prepare_add(&app, &[]).unwrap();
    assert!(empty.is_noop());
    assert!(empty.already_present().is_empty());
    assert_eq!(empty.existing().len(), 1);
    let repeated = prepare_add(&app, &args(&["../old", "../old/./"])).unwrap();
    assert!(repeated.is_noop());
    assert_eq!(repeated.new_references().len(), 0);
    assert_eq!(repeated.already_present().len(), 1);
    assert_eq!(repeated.already_present()[0].reference, "../old/./");
    fs::remove_dir_all(temporary.path().join("old")).unwrap();
    assert!(prepare_add(&app, &[]).is_err());
}

#[test]
fn existing_duplicates_fail_even_when_the_same_source_is_repeated() {
    let (temporary, app) = fixture();
    write_layer(&temporary.path().join("old"), "old", &[]);
    for refs in [
        vec!["../old", "../old"],
        vec!["../old", "../old/./"],
        vec!["old@1.2.3", "old@1.2.3"],
        vec!["old@1.2.3", "old@2.0.0"],
        vec!["../old", "old@1.2.3"],
        vec!["old@1.2.3", "../old"],
    ] {
        write_layer(&app, "app", &refs);
        let failure = prepare_add(&app, &[]).unwrap_err();
        assert_eq!(failure.errors.len(), 1, "{refs:?}");
        assert!(matches!(
            failure.errors[0].origin,
            SourceOrigin::Existing { index: 1, .. }
        ));
        assert!(
            matches!(failure.errors[0].error, DependencyError::NameConflict { ref name, ref previous, .. } if name == "old" && matches!(previous, SourceOrigin::Existing { index: 0, .. }))
        );
    }
}

#[test]
fn duplicate_names_in_different_directories_reject_the_whole_batch() {
    let (temporary, app) = fixture();
    for directory in ["one", "two", "three"] {
        write_layer(&temporary.path().join(directory), "same", &[]);
    }
    let failure = prepare_add(&app, &args(&["../one", "../two", "../three"])).unwrap_err();
    assert_eq!(argument_indexes(&failure), [1, 2]);
    for error in failure.errors {
        assert!(matches!(
            error.error,
            DependencyError::NameConflict {
                previous_directory: Some(_),
                directory: Some(_),
                ..
            }
        ));
    }
    write_layer(&app, "app", &["../one", "../two"]);
    assert!(prepare_add(&app, &[]).is_err());
    write_layer(&app, "app", &["../one"]);
    assert!(prepare_add(&app, &args(&["../two"])).is_err());
}

#[test]
fn self_name_is_reserved_for_local_and_published_existing_and_supplied_sources() {
    let (temporary, app) = fixture();
    write_layer(&temporary.path().join("imposter"), "app", &[]);
    for reference in [".", "../imposter", "app@1.2.3"] {
        write_layer(&app, "app", &[reference]);
        let failure = prepare_add(&app, &[]).unwrap_err();
        assert!(
            matches!(failure.errors[0].error, DependencyError::NameConflict { ref previous, .. } if *previous == SourceOrigin::Current)
        );
    }
    write_layer(&app, "app", &[]);
    let failure = prepare_add(&app, &args(&[".", "../imposter"])).unwrap_err();
    assert_eq!(argument_indexes(&failure), [0, 1]);
}

#[test]
fn published_names_are_checked_offline_and_new_published_arguments_stay_unsupported() {
    let (temporary, app) = fixture();
    let long_name = "a".repeat(214);
    let long_reference = format!("{long_name}@999999999999999999999999999999.0.0-0+001");
    write_layer(
        &app,
        "app",
        &["@scope/.config@1.2.3-beta.1+build.007", &long_reference],
    );
    let plan = prepare_add(&app, &[]).unwrap();
    assert_eq!(plan.existing()[0].name, "@scope/.config");
    assert!(
        matches!(plan.existing()[0].source, DependencySource::Published { ref version } if version == "1.2.3-beta.1+build.007")
    );
    assert_eq!(plan.discoveries().len(), 1);
    write_layer(&temporary.path().join("collision"), "@scope/.config", &[]);
    let failure = prepare_add(
        &app,
        &args(&[
            "../collision",
            "@scope/.config@1.2.3-beta.1+build.007",
            &long_reference,
        ]),
    )
    .unwrap_err();
    assert_eq!(argument_indexes(&failure), [0, 1, 2]);
    assert!(matches!(
        failure.errors[0].error,
        DependencyError::NameConflict {
            previous_directory: None,
            ..
        }
    ));
    assert!(matches!(
        failure.errors[2].error,
        DependencyError::PublishedAdditionUnsupported { .. }
    ));
}

#[test]
fn invalid_reference_errors_are_the_original_schema_errors_without_a_second_grammar() {
    let (_temporary, app) = fixture();
    write_layer(&app.join("bare"), "bare", &[]);
    for value in [
        "bare",
        "@scope/name",
        "file:../old",
        "link:../old",
        "~/old",
        "$HOME/old",
        "pkg@latest",
        "pkg@^1.2.3",
        "pkg@01.2.3",
        "pkg@1.2.3-01",
        "pkg@1.2.3 ",
    ] {
        let expected = schema::classify_reference(value).unwrap_err();
        let failure = prepare_add(&app, &args(&[value])).unwrap_err();
        assert!(
            matches!(&failure.errors[0].error, DependencyError::Schema(actual) if actual == &expected),
            "{value:?}"
        );
        assert_eq!(
            failure.discoveries.len(),
            1,
            "bare source must not be inspected"
        );
    }
}

#[test]
fn drive_relative_forms_are_host_errors_after_portable_schema_acceptance() {
    let (_temporary, app) = fixture();
    for value in ["C:", "C:relative", "z:relative/path"] {
        assert!(matches!(
            schema::classify_reference(value).unwrap(),
            schema::Reference::Local(_)
        ));
        let failure = prepare_add(&app, &args(&[value])).unwrap_err();
        assert!(matches!(
            failure.errors[0].error,
            DependencyError::UnsupportedPath { .. }
        ));
    }
}

#[cfg(not(windows))]
#[test]
fn foreign_windows_paths_are_not_mistaken_for_published_references_or_translated() {
    let (_temporary, app) = fixture();
    for value in [
        r".\shared",
        r"..\shared",
        r"\rooted",
        r"C:\shared",
        "C:/shared",
        r"\\server\share",
        r"\\?\C:\shared",
        r"\\.\device",
    ] {
        schema::classify_reference(value).unwrap();
        let failure = prepare_add(&app, &args(&[value])).unwrap_err();
        assert!(
            matches!(
                failure.errors[0].error,
                DependencyError::UnsupportedPath { .. }
            ),
            "{value:?}"
        );
        assert_eq!(failure.discoveries.len(), 1);
    }
}

#[cfg(unix)]
#[test]
fn native_posix_explicit_paths_can_contain_literal_backslashes_and_unexpanded_text() {
    let (temporary, app) = fixture();
    for name in [r"literal\backslash", "$HOME", "~"] {
        write_layer(&temporary.path().join(name), "literal", &[]);
        let reference = format!("../{name}");
        let plan = prepare_add(&app, std::slice::from_ref(&reference)).unwrap();
        assert_eq!(plan.added()[0].reference, reference);
    }
}

#[test]
fn successful_failed_and_noop_preparation_never_modify_any_file_or_stage_output() {
    let (temporary, app) = fixture();
    write_layer(&temporary.path().join("old"), "old", &[]);
    write_layer(&temporary.path().join("new"), "new", &[]);
    write_layer(&app, "app", &["../old"]);
    fs::write(app.join("notes.txt"), "user data").unwrap();
    let before = snapshot(temporary.path());
    let modified = fs::metadata(app.join("layer.json"))
        .unwrap()
        .modified()
        .unwrap();
    prepare_add(&app, &args(&["../new", "../old"])).unwrap();
    prepare_add(&app, &args(&["../old/./", "../old"])).unwrap();
    prepare_add(&app, &args(&["../new", "../missing"])).unwrap_err();
    assert_eq!(snapshot(temporary.path()), before);
    assert_eq!(
        fs::metadata(app.join("layer.json"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
}

#[test]
fn in_memory_source_parsing_reuses_guarded_decoders_and_schema_without_file_access() {
    let temporary = tempfile::tempdir().unwrap();
    let nonexistent = temporary.path().join("absent/layer.json");
    for (format, source) in [
        (
            LayerManifestFormat::Yaml,
            "# keep\r\nname: app\r\ndependencies: []\r\ncustom: !!str yes",
        ),
        (
            LayerManifestFormat::Yml,
            "name: app\ndependencies: []\ncustom: [1, true]",
        ),
        (
            LayerManifestFormat::Toml,
            "# keep\nname = 'app'\ndependencies = []\n[custom]\nx = true",
        ),
        (
            LayerManifestFormat::Json,
            "{\"name\":\"app\",\"dependencies\":[],\"custom\":true}",
        ),
        (
            LayerManifestFormat::Jsonc,
            "{/*keep*/\"name\":\"app\",\"dependencies\":[],\"custom\":true,}",
        ),
    ] {
        let parsed = layer_manifest::parse_source(&nonexistent, format, source.to_owned()).unwrap();
        assert_eq!(parsed.path, nonexistent);
        assert_eq!(parsed.source, source);
        assert!(parsed.document.get("custom").is_some());
        assert!(!nonexistent.exists());
    }
    for (format, source) in [
        (LayerManifestFormat::Json, "{/*comment*/\"name\":\"app\"}"),
        (LayerManifestFormat::Json, "{\"name\":\"app\",}"),
        (LayerManifestFormat::Jsonc, "{name:'app'}"),
        (
            LayerManifestFormat::Jsonc,
            "{\"name\":\"app\",\"custom\":{\"x\":1,\"x\":2}}",
        ),
        (LayerManifestFormat::Yaml, "name: app\ncustom: {x: 1, x: 2}"),
        (LayerManifestFormat::Yaml, "name: app\ncustom: {<<: {x: 1}}"),
        (
            LayerManifestFormat::Toml,
            "name = 'app'\ncustom = 2026-10-18",
        ),
        (LayerManifestFormat::Json, "{\"name\":\"Uppercase\"}"),
        (LayerManifestFormat::Yaml, "name: app\ndependencies: [bare]"),
    ] {
        assert!(
            layer_manifest::parse_source(&nonexistent, format, source.to_owned()).is_err(),
            "{format}: {source}"
        );
    }
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 0);
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    #[test]
    fn lower_priority_current_symlink_does_not_refuse_the_regular_selected_file() {
        let (temporary, app) = fixture();
        fs::write(app.join("layer.yaml"), "name: app\n").unwrap();
        fs::remove_file(app.join("layer.json")).unwrap();
        symlink(temporary.path().join("absent"), app.join("layer.json")).unwrap();
        let plan = prepare_add(&app, &[]).unwrap();
        assert!(plan.discoveries()[0].has_duplicates());
        assert_eq!(plan.current().format, LayerManifestFormat::Yaml);
    }

    #[test]
    fn known_current_symlinks_are_refused_with_discoveries_without_touching_link_or_target() {
        for dangling in [false, true] {
            let (temporary, app) = fixture();
            let original = temporary.path().join("original.yaml");
            if !dangling {
                fs::write(&original, "name: original\n").unwrap();
            }
            symlink(&original, app.join("layer.yaml")).unwrap();
            let before = snapshot(temporary.path());
            let failure = prepare_add(&app, &args(&["invalid", "../missing"])).unwrap_err();
            assert_eq!(failure.errors.len(), 1);
            assert!(matches!(
                failure.errors[0].error,
                DependencyError::CurrentSymlink { .. }
            ));
            assert!(failure.discoveries[0].has_duplicates());
            assert_eq!(snapshot(temporary.path()), before);
        }
    }

    #[test]
    fn dependency_layer_manifest_symlinks_are_readable_but_dangling_links_do_not_fall_back() {
        let (temporary, app) = fixture();
        let target = temporary.path().join("target");
        write_layer(&target, "fallback", &[]);
        let actual = temporary.path().join("actual.yaml");
        fs::write(&actual, "name: selected\n").unwrap();
        symlink(&actual, target.join("layer.yaml")).unwrap();
        let plan = prepare_add(&app, &args(&["../target"])).unwrap();
        assert_eq!(plan.added()[0].name, "selected");
        assert!(plan.discoveries()[1].has_duplicates());
        fs::remove_file(&actual).unwrap();
        let failure = prepare_add(&app, &args(&["../target"])).unwrap_err();
        assert!(failure.discoveries[1].has_duplicates());
        assert!(
            matches!(&failure.errors[0].error, DependencyError::LayerManifest(error) if matches!(error.as_ref(), LayerManifestError::Io { .. }))
        );
    }

    #[test]
    fn canonical_directory_aliases_deduplicate_discoveries_on_success_and_failure() {
        let (temporary, app) = fixture();
        let target = temporary.path().join("target");
        write_layer(&target, "target", &[]);
        fs::write(target.join("layer.yaml"), "name: target").unwrap();
        symlink(&target, temporary.path().join("alias")).unwrap();
        let sources = args(&["../target", "../alias", "../target/./"]);
        let plan = prepare_add(&app, &sources).unwrap();
        assert_eq!(plan.discoveries().len(), 2);
        assert_eq!(plan.added().len(), 1);
        write_layer(&app, "app", &["../target/./"]);
        let plan = prepare_add(&app, &sources).unwrap();
        assert_eq!(plan.discoveries().len(), 2);
        assert_eq!(plan.already_present().len(), 1);
        assert_eq!(plan.already_present()[0].reference, "../target/./");
        fs::write(target.join("layer.yaml"), "name: [").unwrap();
        let failure = prepare_add(&app, &sources).unwrap_err();
        assert_eq!(failure.errors.len(), 4);
        assert_eq!(failure.discoveries.len(), 2);
        assert!(failure.discoveries[1].has_duplicates());
        let errors: Vec<_> = failure
            .errors
            .iter()
            .map(|item| match &item.error {
                DependencyError::LayerManifest(error) => error,
                other => panic!("expected cached layer manifest error, got {other:?}"),
            })
            .collect();
        assert!(
            errors
                .windows(2)
                .all(|pair| std::sync::Arc::ptr_eq(pair[0], pair[1]))
        );
    }

    #[test]
    fn symlink_then_parent_resolution_is_not_lexically_simplified() {
        let (temporary, app) = fixture();
        let real = temporary.path().join("real");
        fs::create_dir_all(real.join("child")).unwrap();
        write_layer(&real, "real", &[]);
        symlink(real.join("child"), app.join("jump")).unwrap();
        let plan = prepare_add(&app, &args(&["./jump/.."])).unwrap();
        assert_eq!(plan.added()[0].name, "real");
        assert_eq!(plan.added()[0].reference, "./jump/..");
        assert_eq!(
            plan.added()[0].source,
            DependencySource::Local {
                directory: fs::canonicalize(real).unwrap()
            }
        );
    }

    #[test]
    fn existing_directory_alias_declarations_still_conflict_and_current_alias_is_self() {
        let (temporary, app) = fixture();
        let target = temporary.path().join("target");
        write_layer(&target, "target", &[]);
        symlink(&target, temporary.path().join("alias")).unwrap();
        write_layer(&app, "app", &["../target", "../alias"]);
        let failure = prepare_add(&app, &[]).unwrap_err();
        assert_eq!(failure.discoveries.len(), 2);
        assert!(matches!(
            failure.errors[0].error,
            DependencyError::NameConflict { .. }
        ));
        write_layer(&app, "app", &[]);
        symlink(&app, temporary.path().join("app-alias")).unwrap();
        let failure = prepare_add(&app, &args(&["../app-alias"])).unwrap_err();
        assert_eq!(failure.discoveries.len(), 1);
        assert!(matches!(
            failure.errors[0].error,
            DependencyError::NameConflict { .. }
        ));
    }

    #[test]
    fn non_regular_selected_current_and_target_entries_fail_without_blocking_or_fallback() {
        for kind in ["directory", "socket", "fifo"] {
            let (temporary, app) = fixture();
            let target = temporary.path().join("target");
            write_layer(&target, "target", &[]);
            for directory in [&app, &target] {
                let path = directory.join("layer.yaml");
                match kind {
                    "directory" => {
                        fs::create_dir(&path).unwrap();
                    }
                    "socket" => {
                        let _listener = UnixListener::bind(&path).unwrap();
                    }
                    "fifo" => {
                        assert!(
                            std::process::Command::new("mkfifo")
                                .arg(&path)
                                .status()
                                .unwrap()
                                .success()
                        );
                    }
                    _ => unreachable!(),
                }
            }
            let failure = prepare_add(&app, &[]).unwrap_err();
            assert!(failure.discoveries[0].has_duplicates());
            assert!(
                matches!(&failure.errors[0].error, DependencyError::LayerManifest(error) if matches!(error.as_ref(), LayerManifestError::NotRegular { .. })),
                "{kind}"
            );
            match kind {
                "directory" => fs::remove_dir(app.join("layer.yaml")).unwrap(),
                _ => fs::remove_file(app.join("layer.yaml")).unwrap(),
            }
            let failure = prepare_add(&app, &args(&["../target"])).unwrap_err();
            assert!(failure.discoveries[1].has_duplicates());
            assert!(
                matches!(&failure.errors[0].error, DependencyError::LayerManifest(error) if matches!(error.as_ref(), LayerManifestError::NotRegular { .. })),
                "{kind}"
            );
        }
    }
}
