use std::fs;
use std::path::Path;

use hydragen::{
    EncodeError, LayerManifestError, LayerManifestFormat as Format, LoadedLayerManifest, Metadata,
    MetadataError, discover, encode_metadata, validate_document,
};
use serde_json::{Value, json};

fn read(format: Format, text: impl AsRef<[u8]>) -> Result<LoadedLayerManifest, LayerManifestError> {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join(format.filename()), text).unwrap();
    discover(directory.path()).unwrap().load()
}

fn reject(format: Format, text: impl AsRef<[u8]>) -> String {
    let error = read(format, text.as_ref()).unwrap_err().to_string();
    assert!(error.contains(format.filename()), "{error}");
    assert!(error.contains("layer manifest"), "{error}");
    error
}

fn metadata() -> Metadata {
    Metadata {
        name: "example".into(),
        author: None,
        description: None,
    }
}

#[test]
fn format_choices_default_and_exact_filenames() {
    assert_eq!(Format::default(), Format::Yaml);
    assert_eq!(
        Format::ALL,
        [
            Format::Yaml,
            Format::Yml,
            Format::Toml,
            Format::Json,
            Format::Jsonc
        ]
    );
    assert_eq!(
        Format::ALL.map(Format::extension),
        ["yaml", "yml", "toml", "json", "jsonc"]
    );
    assert_eq!(
        Format::ALL.map(Format::filename),
        [
            "layer.yaml",
            "layer.yml",
            "layer.toml",
            "layer.json",
            "layer.jsonc"
        ]
    );
    for format in Format::ALL {
        assert_eq!(format.to_string(), format.extension());
    }
}

#[test]
fn every_presence_subset_has_stable_priority_without_parsing_or_writes() {
    for bits in 0..32 {
        let directory = tempfile::tempdir().unwrap();
        let expected: Vec<_> = Format::ALL
            .into_iter()
            .enumerate()
            .filter_map(|(index, format)| {
                if bits & (1 << index) == 0 {
                    return None;
                }
                // Invalid bytes in every candidate prove discovery does not require parsing.
                fs::write(directory.path().join(format.filename()), [0xff, 0x00]).unwrap();
                Some(format)
            })
            .collect();
        let discovery = discover(directory.path()).unwrap();
        assert_eq!(discovery.directory, directory.path());
        assert_eq!(
            discovery
                .candidates
                .iter()
                .map(|candidate| candidate.format)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            discovery.selected().map(|candidate| candidate.format),
            expected.first().copied()
        );
        assert_eq!(discovery.has_duplicates(), expected.len() > 1);
        for candidate in &discovery.candidates {
            assert_eq!(
                candidate.path,
                directory.path().join(candidate.format.filename())
            );
            assert_eq!(fs::read(&candidate.path).unwrap(), [0xff, 0x00]);
        }
        assert_eq!(
            fs::read_dir(directory.path()).unwrap().count(),
            expected.len()
        );
    }
}

#[test]
fn discovery_ignores_other_names_nested_paths_and_ancestors() {
    let parent = tempfile::tempdir().unwrap();
    let directory = parent.path().join("project");
    fs::create_dir(&directory).unwrap();
    fs::create_dir(directory.join("nested")).unwrap();
    for format in Format::ALL {
        fs::write(parent.path().join(format.filename()), "invalid").unwrap();
        fs::write(directory.join("nested").join(format.filename()), "invalid").unwrap();
    }
    for filename in [
        "example.yaml",
        "layer.json5",
        "layer.yaml.bak",
        "layer",
        "layer-manifest.json",
    ] {
        fs::write(directory.join(filename), "invalid").unwrap();
    }
    assert!(discover(&directory).unwrap().candidates.is_empty());
}

#[test]
fn every_supported_directory_counts_as_occupied() {
    for format in Format::ALL {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join(format.filename())).unwrap();
        let discovery = discover(directory.path()).unwrap();
        assert_eq!(discovery.selected().unwrap().format, format);
        let error = discovery.load().unwrap_err();
        assert!(matches!(error, LayerManifestError::NotRegular { .. }));
        assert!(
            error
                .to_string()
                .starts_with("Could not read layer manifest")
        );
    }
}

#[test]
fn unexpected_inspection_errors_are_not_absence() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("not-a-directory");
    fs::write(&file, "unchanged").unwrap();
    let error = discover(&file).unwrap_err();
    assert!(error.to_string().contains("inspect layer manifest path"));
    assert!(error.to_string().contains("layer.yaml"));
    assert_eq!(fs::read_to_string(file).unwrap(), "unchanged");
}

#[test]
fn missing_layer_manifest_error_names_directory_and_every_supported_file() {
    let directory = tempfile::tempdir().unwrap();
    let discovery = discover(directory.path()).unwrap();
    assert!(discovery.selected().is_none());
    let error = discovery.load().unwrap_err().to_string();
    assert!(error.starts_with("No layer manifest found"), "{error}");
    assert!(error.contains(directory.path().to_str().unwrap()));
    for format in Format::ALL {
        assert!(error.contains(format.filename()));
    }
}

#[test]
fn selection_errors_never_fall_back_and_leave_discovery_available() {
    for invalid in ["name: [", "name: null", "name: '  '"] {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("layer.yaml"), invalid).unwrap();
        fs::write(directory.path().join("layer.json"), r#"{"name":"lower"}"#).unwrap();
        let discovery = discover(directory.path()).unwrap();
        let error = discovery.load().unwrap_err().to_string();
        assert!(error.contains("layer.yaml"));
        assert_eq!(discovery.candidates.len(), 2);
        assert_eq!(discovery.selected().unwrap().format, Format::Yaml);
        assert_eq!(
            fs::read_to_string(&discovery.selected().unwrap().path).unwrap(),
            invalid
        );
    }
}

#[test]
fn selected_disappearance_is_an_error_not_new_selection() {
    let directory = tempfile::tempdir().unwrap();
    let selected = directory.path().join("layer.yaml");
    fs::write(&selected, "name: selected").unwrap();
    fs::write(directory.path().join("layer.json"), r#"{"name":"lower"}"#).unwrap();
    let discovery = discover(directory.path()).unwrap();
    fs::remove_file(&selected).unwrap();
    assert!(
        matches!(discovery.load(), Err(LayerManifestError::Io { path, .. }) if path == selected)
    );
    assert_eq!(discovery.candidates.len(), 2);
}

#[test]
fn invalid_utf8_fails_for_every_format() {
    for format in Format::ALL {
        let error = reject(format, [0xff, 0xfe, b'{', b'}']);
        assert!(error.contains("read layer manifest"), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn symlinks_count_even_when_dangling_and_format_comes_from_layer_manifest_name() {
    use std::os::unix::fs::symlink;
    for format in Format::ALL {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("unrelated.txt");
        let path = directory.path().join(format.filename());
        symlink(&target, &path).unwrap();
        let discovery = discover(directory.path()).unwrap();
        assert_eq!(discovery.selected().unwrap().path, path);
        assert!(
            discovery
                .load()
                .unwrap_err()
                .to_string()
                .contains(format.filename())
        );
        let bytes = encode_metadata(&metadata(), format).unwrap();
        fs::write(&target, &bytes).unwrap();
        let loaded = discovery.load().unwrap();
        assert_eq!(loaded.metadata, metadata());
        assert_eq!(loaded.format, format);
        assert!(fs::symlink_metadata(&path).unwrap().is_symlink());
        assert_eq!(fs::read(&target).unwrap(), bytes);
    }
}

#[cfg(unix)]
#[test]
fn nonregular_selected_and_lower_priority_paths_are_never_read() {
    use std::os::unix::net::UnixListener;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("layer.yaml");
    let _socket = UnixListener::bind(&path).unwrap();
    fs::write(directory.path().join("layer.json"), r#"{"name":"lower"}"#).unwrap();
    let discovery = discover(directory.path()).unwrap();
    assert!(
        matches!(discovery.load(), Err(LayerManifestError::NotRegular { path: rejected }) if rejected == path)
    );
    assert!(discovery.has_duplicates());

    // Conversely, a lower-priority socket cannot affect selected-only loading.
    let other = tempfile::tempdir().unwrap();
    fs::write(other.path().join("layer.yaml"), "name: selected").unwrap();
    let _lower = UnixListener::bind(other.path().join("layer.json")).unwrap();
    assert_eq!(
        discover(other.path())
            .unwrap()
            .load()
            .unwrap()
            .metadata
            .name,
        "selected"
    );
}

#[cfg(unix)]
#[test]
fn fifo_is_rejected_without_opening_for_input() {
    const CHILD_PATH: &str = "HYDRAGEN_LAYER_MANIFEST_FIFO_TEST_PATH";
    if let Some(path) = std::env::var_os(CHILD_PATH) {
        let discovery = discover(Path::new(&path)).unwrap();
        assert!(matches!(
            discovery.load(),
            Err(LayerManifestError::NotRegular { .. })
        ));
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("layer.yaml");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    // Keep a future regression that opens the FIFO from hanging the test suite.
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fifo_is_rejected_without_opening_for_input"])
        .env(CHILD_PATH, directory.path())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("FIFO loading waited for input");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(unix)]
#[test]
fn unreadable_or_nonfile_selected_paths_do_not_fall_back() {
    use std::os::unix::fs::symlink;
    for kind in ["directory", "dangling", "loop"] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("layer.yaml");
        match kind {
            "directory" => fs::create_dir(&path).unwrap(),
            "dangling" => symlink("missing-target", &path).unwrap(),
            "loop" => symlink("layer.yaml", &path).unwrap(),
            _ => unreachable!(),
        }
        fs::write(directory.path().join("layer.json"), r#"{"name":"lower"}"#).unwrap();
        let discovery = discover(directory.path()).unwrap();
        assert!(discovery.has_duplicates());
        assert!(
            discovery
                .load()
                .unwrap_err()
                .to_string()
                .contains("layer.yaml")
        );
        assert_eq!(discovery.selected().unwrap().path, path);
    }
}

#[test]
fn common_metadata_validation_distinguishes_absent_null_and_wrong_types() {
    assert_eq!(
        validate_document(&json!({"name": "example"})).unwrap(),
        metadata()
    );
    assert!(matches!(
        validate_document(&json!({})),
        Err(MetadataError::MissingField { field: "name" })
    ));
    for value in [
        Value::Null,
        json!(true),
        json!(123),
        json!([]),
        json!({}),
        json!(""),
        json!(" \t\n\u{a0}\u{2003}"),
    ] {
        for field in ["name", "author", "description"] {
            let mut document = json!({"name":"example"});
            document[field] = value.clone();
            assert!(validate_document(&document).is_err(), "{document}");
        }
    }
    for value in [
        Value::Null,
        json!(false),
        json!(12),
        json!("text"),
        json!([]),
    ] {
        assert!(matches!(
            validate_document(&value),
            Err(MetadataError::NotObject)
        ));
    }
}

#[test]
fn file_metadata_rules_are_the_same_for_every_format() {
    for format in Format::ALL {
        let valid = if format == Format::Toml {
            "name = 'example'"
        } else {
            r#"{"name":"example"}"#
        };
        assert_eq!(read(format, valid).unwrap().metadata, metadata());
        for field in ["name", "author", "description"] {
            for value in ["true", "123", "[]", "{}", "''", "'  '"] {
                let text = if format == Format::Toml {
                    if field == "name" {
                        format!("name = {value}")
                    } else {
                        format!("name = 'example'\n{field} = {value}")
                    }
                } else if matches!(format, Format::Yaml | Format::Yml) {
                    if field == "name" {
                        format!("name: {value}")
                    } else {
                        format!("name: example\n{field}: {value}")
                    }
                } else {
                    let value = value.replace('\'', "\"");
                    if field == "name" {
                        format!(r#"{{"name":{value}}}"#)
                    } else {
                        format!(r#"{{"name":"example","{field}":{value}}}"#)
                    }
                };
                reject(format, text);
            }
        }
        if format != Format::Toml {
            for field in ["name", "author", "description"] {
                let mut document = json!({"name":"example"});
                document[field] = Value::Null;
                if field == "author" {
                    let Err(LayerManifestError::Metadata {
                        source:
                            MetadataError::Schema(hydragen::layer_manifest::schema::Error::Content(
                                violation,
                            )),
                        ..
                    }) = read(format, document.to_string())
                    else {
                        panic!("expected string/object author type failure");
                    };
                    assert_eq!(violation.instance_path, "/author");
                    assert_eq!(
                        violation.kind,
                        hydragen::layer_manifest::schema::ViolationKind::Type
                    );
                    assert!(
                        violation.message.contains("string")
                            && violation.message.contains("object")
                    );
                    continue;
                }
                assert!(
                    matches!(read(format, document.to_string()), Err(LayerManifestError::Metadata { source: MetadataError::NotString { field: invalid }, .. }) if invalid == field)
                );
            }
        }
    }
}

#[test]
fn jsonc_accepts_only_comments_and_trailing_commas_beyond_json() {
    let text = r#"// before
    {"name": "example", "description": "https://example.test/*literal*/", /* middle */ "extra": [1, true, null,],} // after"#;
    let loaded = read(Format::Jsonc, text).unwrap();
    assert_eq!(
        loaded.metadata.description.as_deref(),
        Some("https://example.test/*literal*/")
    );
    assert_eq!(loaded.document["extra"], json!([1, true, null]));
    reject(Format::Json, text);
    for text in [
        r#"{"name":"x",}"#,
        "// comment\n{\"name\":\"x\"}",
        r#"{"name":"x","x":[1,]}"#,
    ] {
        assert!(read(Format::Jsonc, text).is_ok());
        reject(Format::Json, text);
    }
}

#[test]
fn json_and_jsonc_reject_disabled_json5_syntax_and_invalid_numbers() {
    for format in [Format::Json, Format::Jsonc] {
        for text in [
            "{name: \"x\"}",
            "{'name':'x'}",
            r#"{"name":"x" "extra":1}"#,
            r#"{"name":"x","extra":[1 2]}"#,
            r#"{"name":"x","extra":0x10}"#,
            r#"{"name":"x","extra":+1}"#,
            r#"{"name":"x","extra":.5}"#,
            r#"{"name":"x","extra":5.}"#,
            r#"{"name":"x","extra":Infinity}"#,
            r#"{"name":"x","extra":-Infinity}"#,
            r#"{"name":"x","extra":NaN}"#,
            r#"{"name":"\x41"}"#,
            r#"{"name":"\v"}"#,
            "{\"name\":\"line\\\ncontinuation\"}",
            r#"{"name":"x","extra":01}"#,
            r#"{"name":"x","extra":1e999}"#,
            r#"{"name":"x","extra":1e}"#,
            "{\"name\":\"raw\nnewline\"}",
            "{\"name\":\"raw\ttab\"}",
            "{\"name\":\"raw\u{0000}control\"}",
            "{\"name\":\"raw\u{001f}control\"}",
            "{\"name\":\"x\",\"raw\nkey\":1}",
            "\u{000b}{\"name\":\"x\"}",
            "{\"name\":\u{00a0}\"x\"}",
            "{\"name\":\"x\"}\u{000c}",
        ] {
            reject(format, text);
        }
    }
}

#[test]
fn json_numbers_preserve_integer_precision_and_finite_values() {
    for format in [Format::Json, Format::Jsonc] {
        let loaded = read(format, r#"{"name":"x","values":[9007199254740993,18446744073709551615,-9223372036854775808,1.5,1e10]}"#).unwrap();
        assert_eq!(
            loaded.document["values"][0].as_u64(),
            Some(9_007_199_254_740_993)
        );
        assert_eq!(loaded.document["values"][1].as_u64(), Some(u64::MAX));
        assert_eq!(loaded.document["values"][2].as_i64(), Some(i64::MIN));
        assert_eq!(loaded.document["values"][3], json!(1.5));
        assert_eq!(loaded.document["values"][4], json!(1e10));
    }
}

#[test]
fn json_decoded_duplicates_are_rejected_at_every_depth() {
    for format in [Format::Json, Format::Jsonc] {
        for text in [
            r#"{"name":"a","name":"b"}"#,
            r#"{"name":"x","other":1,"other":2}"#,
            r#"{"name":"x","other":{"x":1,"x":2}}"#,
            r#"{"name":"x","other":[{"x":1,"\u0078":2}]}"#,
            r#"{"name":"x","\u006eame":"y"}"#,
            r#"{"name":"x","😀":1,"\ud83d\ude00":2}"#,
        ] {
            assert!(reject(format, text).contains("duplicate"));
        }
    }
}

#[test]
fn empty_nonobjects_and_trailing_input_fail() {
    for format in [Format::Yaml, Format::Yml, Format::Json, Format::Jsonc] {
        for text in [
            "",
            " ",
            "null",
            "true",
            "123",
            "\"name\"",
            "[]",
            "{}",
            "{} {}",
            "{}\n[]",
            r#"{"name":"x"} {}"#,
            "{\"name\":\"x\"}\n[]",
        ] {
            reject(format, text);
        }
    }
    reject(Format::Jsonc, "// comments only\n/* no document */");
    for text in [
        "",
        "# comments",
        "name = 'x'\n42",
        "[layer]\nname = 'nested'",
    ] {
        reject(Format::Toml, text);
    }
}

#[test]
fn ordinary_yaml_strings_tags_aliases_comments_and_additional_data() {
    let text = "# layer manifest\nname: example\nauthor: &label '  配置 🦀  '\ncopy: *label\ndescription: |\n  first line\n  second line\nextra:\n  values: [true, 123, null]\n  literal: ${UNCHANGED}\n  tagged: !!str 123\n";
    for format in [Format::Yaml, Format::Yml] {
        let loaded = read(format, text).unwrap();
        assert_eq!(loaded.metadata.name, "example");
        assert_eq!(loaded.document["copy"], "  配置 🦀  ");
        assert_eq!(
            loaded.metadata.author.as_ref().map(hydragen::Author::name),
            Some("  配置 🦀  ")
        );
        assert_eq!(
            loaded.metadata.description.as_deref(),
            Some("first line\nsecond line\n")
        );
        assert_eq!(
            loaded.document["extra"],
            json!({"values":[true,123,null],"literal":"${UNCHANGED}","tagged":"123"})
        );
        assert_eq!(
            read(format, "name: !!str true").unwrap().metadata.name,
            "true"
        );
        for text in ["yes", "no", "on", "off"] {
            assert_eq!(
                read(format, format!("name: {text}")).unwrap().metadata.name,
                text
            );
        }
        assert_eq!(
            read(
                format,
                "name: example\nextra: {\"123\": number-key-as-text, \"true\": bool-key-as-text}"
            )
            .unwrap()
            .document["extra"]["123"],
            "number-key-as-text"
        );
    }
}

#[test]
fn yaml_never_coerces_scalar_metadata_or_nonstring_keys() {
    for format in [Format::Yaml, Format::Yml] {
        for value in ["true", "false", "123", "1.5", "null", "~", "", "[]", "{}"] {
            reject(format, format!("name: {value}"));
            reject(format, format!("name: example\nauthor: {value}"));
            reject(format, format!("name: example\ndescription: {value}"));
        }
        for text in [
            "name: example\n123: invalid",
            "name: example\ntrue: invalid",
            "name: example\nnull: invalid",
            "name: example\n? [one, two]\n: invalid",
            "name: example\n? {one: two}\n: invalid",
            "name: example\nextra: {123: invalid}",
            "name: example\nextra: [{false: invalid}]",
        ] {
            assert!(reject(format, text).contains("string object key"));
        }
    }
}

#[test]
fn yaml_rejects_duplicates_composition_tags_nonfinite_and_document_streams() {
    for format in [Format::Yaml, Format::Yml] {
        for text in [
            "name: a\nname: b",
            "name: x\nextra: 1\nextra: 2",
            "name: x\nextra: {x: 1, x: 2}",
            "name: x\nextra: [{x: 1, \"\\u0078\": 2}]",
        ] {
            let error = reject(format, text).to_lowercase();
            assert!(error.contains("duplicate"), "{error}");
        }
        for text in [
            "name: x\n<<: {extra: 1}",
            "name: x\nextra: {<<: {a: b}}",
            "name: x\n!!merge <<: {extra: 1}",
            "name: !custom x",
            "name: x\nextra: !custom [a]",
            "name: !include /etc/passwd",
            "name: !degrees 42",
            "name: x\nextra: .inf",
            "name: x\nextra: -.inf",
            "name: x\nextra: .nan",
            "name: x\nextra: 1e999",
            "name: x\n---\nname: y",
            "name: x\n---",
            "name: x\n...\nname: y",
            "name: x\nextra: &recursive [*recursive]",
        ] {
            reject(format, text);
        }
        let deeply_nested = format!("name: x\nextra: {}0{}", "[".repeat(70), "]".repeat(70));
        reject(format, deeply_nested);
    }
}

#[test]
fn toml_supports_ordinary_strings_tables_dotted_keys_and_arrays_of_tables() {
    let text = r#"# top-level metadata
name = 'example'
author = "quoted \"author\""
description = """
first line
second line"""
extra.inline = { enabled = true, values = [1, 1.5, "text"] }
[settings]
literal = '''a literal
multiline string'''
[[items]]
id = 1
[[items]]
id = 2
"#;
    let loaded = read(Format::Toml, text).unwrap();
    assert_eq!(
        loaded.metadata,
        Metadata {
            name: "example".into(),
            author: Some("quoted \"author\"".into()),
            description: Some("first line\nsecond line".into()),
        }
    );
    assert_eq!(
        loaded.document["extra"]["inline"],
        json!({"enabled":true,"values":[1,1.5,"text"]})
    );
    assert_eq!(
        loaded.document["settings"]["literal"],
        "a literal\nmultiline string"
    );
    assert_eq!(loaded.document["items"], json!([{"id":1},{"id":2}]));
}

#[test]
fn toml_rejects_duplicates_redefinitions_and_nonstring_metadata() {
    for text in [
        "name = 'x'\nname = 'y'",
        "name = 'x'\nextra = {a = 1, a = 2}",
        "name = 'x'\nextra.a = 1\n[extra]\na = 2",
        "name = 'x'\n[extra]\n[extra]",
        "name = 'x'\nextra = 1\n[extra]",
        "name = true",
        "name = 123",
        "name = []",
        "name = {}",
        "name = 'x'\nauthor = false",
        "name = 'x'\ndescription = 123",
        "name = null",
        "name = 'x'\n[author]\ntext = 'not a string'",
        "[layer]\nname = 'not top-level'",
    ] {
        reject(Format::Toml, text);
    }
    assert!(
        reject(
            Format::Toml,
            "name = 'x'\n\"extra\" = 1\n\"\\u0065xtra\" = 2"
        )
        .contains("duplicate")
    );
}

#[test]
fn toml_rejects_native_datetime_and_nonfinite_at_any_depth_with_guidance() {
    for value in [
        "2026-10-17",
        "12:30:00",
        "2026-10-17T12:30:00",
        "2026-10-17T12:30:00Z",
        "2026-10-17T12:30:00+02:00",
    ] {
        for prefix in [
            "name = ",
            "name = 'x'\nextra = ",
            "name = 'x'\nextra = [{when = ",
        ] {
            let suffix = if prefix.ends_with("{when = ") {
                "}]"
            } else {
                ""
            };
            let error = reject(Format::Toml, format!("{prefix}{value}{suffix}"));
            assert!(error.contains("date/time"), "{error}");
            assert!(error.contains("quoted string"), "{error}");
        }
        assert_eq!(
            read(
                Format::Toml,
                format!("name = 'example'\nauthor = '{value}'")
            )
            .unwrap()
            .metadata
            .author
            .as_ref()
            .map(hydragen::Author::name),
            Some(value)
        );
    }
    for value in ["inf", "+inf", "-inf", "nan", "+nan", "-nan"] {
        let error = reject(Format::Toml, format!("name = 'x'\nextra = [{value}]"));
        assert!(error.contains("non-finite"), "{error}");
    }
}

#[test]
fn equivalent_metadata_and_additional_data_across_all_five_formats() {
    let expected = json!({"name":"@example/.config","author":"  配置 🦀 a \"quote\" \\path  ","description":"line one\nline two","extra":{"enabled":true,"values":[1,2.5,"text"]}});
    for format in Format::ALL {
        let text = match format {
            Format::Yaml | Format::Yml => "name: '@example/.config'\nauthor: '  配置 🦀 a \"quote\" \\path  '\ndescription: |-\n  line one\n  line two\nextra: {enabled: true, values: [1, 2.5, text]}\n".into(),
            Format::Toml => "name = '@example/.config'\nauthor = '  配置 🦀 a \"quote\" \\path  '\ndescription = \"line one\\nline two\"\nextra = {enabled = true, values = [1, 2.5, 'text']}\n".into(),
            Format::Json | Format::Jsonc => expected.to_string(),
        };
        let loaded = read(format, text).unwrap();
        assert_eq!(loaded.document, expected);
        assert_eq!(loaded.metadata, validate_document(&expected).unwrap());
    }
}

#[test]
fn exact_minimal_output_fixtures_and_independent_parser_expectations() {
    for format in Format::ALL {
        let bytes = encode_metadata(&metadata(), format).unwrap();
        let expected = match format {
            Format::Yaml | Format::Yml => "name: example\n",
            Format::Toml => "name = \"example\"\n",
            Format::Json | Format::Jsonc => "{\n  \"name\": \"example\"\n}\n",
        };
        assert_eq!(bytes, expected.as_bytes());
        assert_eq!(
            independent_decode(&bytes, format),
            json!({"name":"example"})
        );
        let complete = Metadata {
            author: Some("Example team".into()),
            description: Some("Shared rules".into()),
            ..metadata()
        };
        let expected = match format {
            Format::Yaml | Format::Yml => {
                "name: example\nauthor: Example team\ndescription: Shared rules\n"
            }
            Format::Toml => {
                "name = \"example\"\nauthor = \"Example team\"\ndescription = \"Shared rules\"\n"
            }
            Format::Json | Format::Jsonc => {
                "{\n  \"name\": \"example\",\n  \"author\": \"Example team\",\n  \"description\": \"Shared rules\"\n}\n"
            }
        };
        assert_eq!(
            encode_metadata(&complete, format).unwrap(),
            expected.as_bytes()
        );
    }
}

fn independent_decode(bytes: &[u8], format: Format) -> Value {
    let text = std::str::from_utf8(bytes).unwrap();
    match format {
        Format::Yaml | Format::Yml => serde_saphyr::from_str(text).unwrap(),
        Format::Toml => {
            let table: toml::Table = toml::from_str(text).unwrap();
            serde_json::to_value(table).unwrap()
        }
        // Generated JSONC deliberately is strict JSON, with no comments/header.
        Format::Json | Format::Jsonc => serde_json::from_str(text).unwrap(),
    }
}

#[test]
fn every_codec_optional_combination_is_minimal_ordered_and_round_trips() {
    for format in Format::ALL {
        for author in [None, Some("Example team".to_owned())] {
            for description in [None, Some("Shared rules".to_owned())] {
                let metadata = Metadata {
                    author: author.clone().map(Into::into),
                    description: description.clone(),
                    ..metadata()
                };
                let bytes = encode_metadata(&metadata, format).unwrap();
                assert_eq!(bytes.last(), Some(&b'\n'));
                let text = std::str::from_utf8(&bytes).unwrap();
                let expected = serde_json::to_value(&metadata).unwrap();
                assert_eq!(independent_decode(&bytes, format), expected);
                assert_eq!(read(format, &bytes).unwrap().metadata, metadata);
                assert_eq!(
                    expected.as_object().unwrap().len(),
                    1 + usize::from(author.is_some()) + usize::from(description.is_some())
                );
                let name_position = text.find("name").unwrap();
                if author.is_some() {
                    assert!(name_position < text.find("author").unwrap());
                }
                if description.is_some() {
                    assert!(name_position < text.find("description").unwrap());
                }
                if author.is_some() && description.is_some() {
                    assert!(text.find("author").unwrap() < text.find("description").unwrap());
                }
            }
        }
    }
}

#[test]
fn every_codec_preserves_ambiguous_multiline_escaped_and_unicode_strings() {
    for value in [
        "true",
        "false",
        "yes",
        "on",
        "null",
        "~",
        "123",
        "1.5",
        "1e10",
        "2026-10-17",
        "# comment",
        "name: value",
        "---",
        "[]",
        "{}",
        "  surrounding whitespace  ",
        "quotes: \"double\" 'single'",
        "C:\\path\\next",
        "line one\nline two",
        "line one\n\n",
        "\ttext\r\nnext",
        "配置 🦀",
        " leading\n trailing ",
        "a\u{0000}b",
    ] {
        let metadata = Metadata {
            name: "example".into(),
            author: Some(value.into()),
            description: Some(value.into()),
        };
        for format in Format::ALL {
            let bytes = encode_metadata(&metadata, format).unwrap();
            assert_eq!(
                independent_decode(&bytes, format),
                serde_json::to_value(&metadata).unwrap(),
                "{format}: {value:?}"
            );
            assert_eq!(
                read(format, bytes).unwrap().metadata,
                metadata,
                "{format}: {value:?}"
            );
        }
    }
}

#[test]
fn every_codec_preserves_schema_valid_scalar_looking_names_and_rejects_invalid_names() {
    for format in Format::ALL {
        for name in [
            "true",
            "false",
            "yes",
            "on",
            "null",
            "123",
            "1.5",
            "1e10",
            "2026-10-17",
            "---",
            "18446744073709551616",
            "0x1_0000_0000_0000_0000",
            "-9223372036854775809",
        ] {
            let candidate = Metadata {
                name: name.into(),
                ..metadata()
            };
            let bytes = encode_metadata(&candidate, format).unwrap();
            assert_eq!(independent_decode(&bytes, format), json!({"name": name}));
            assert_eq!(read(format, bytes).unwrap().metadata, candidate);
        }
        for name in [
            "Shared-config",
            " example ",
            "配置",
            "_config",
            "@Scope/config",
        ] {
            let candidate = Metadata {
                name: name.into(),
                ..metadata()
            };
            assert!(
                matches!(
                    encode_metadata(&candidate, format),
                    Err(EncodeError::Metadata(_))
                ),
                "{format}: {name:?}"
            );
        }
    }
}

#[test]
fn codecs_and_existing_init_share_the_same_nonblank_rule() {
    for field in ["name", "author", "description"] {
        for blank in ["", " \t\r\n", "\u{85}\u{a0}\u{2003}"] {
            let mut metadata = metadata();
            match field {
                "name" => metadata.name = blank.into(),
                "author" => metadata.author = Some(blank.into()),
                "description" => metadata.description = Some(blank.into()),
                _ => unreachable!(),
            }
            assert!(metadata.validate().is_err());
            assert!(hydragen::validate_metadata_value(field, blank).is_err());
            for format in Format::ALL {
                assert!(
                    matches!(encode_metadata(&metadata, format), Err(EncodeError::Metadata(MetadataError::Blank { field: invalid })) if invalid == field)
                );
            }
            assert!(hydragen::initialize(Path::new("missing-directory"), &metadata).is_err());
        }
    }
}
