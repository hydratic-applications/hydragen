use std::error::Error as _;
use std::fs;

use hydragen::layer_manifest::schema::{self, Error, ViolationKind};
use hydragen::{
    InitError, LayerManifestError, LayerManifestFormat, MetadataError, discover, validate_document,
};
use serde_json::{Value, json};

fn content(error: Error) -> schema::Violation {
    match error {
        Error::Content(violation) => *violation,
        other => panic!("expected a content failure, got {other:?}"),
    }
}

// Author the candidate independently of the init encoder. TOML cases deliberately
// avoid null, which TOML cannot represent, rather than testing a parse failure.
fn source(document: &Value, format: LayerManifestFormat) -> String {
    match format {
        LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
            serde_saphyr::to_string(document).unwrap()
        }
        LayerManifestFormat::Toml => toml::to_string(document).unwrap(),
        LayerManifestFormat::Json => document.to_string(),
        LayerManifestFormat::Jsonc => format!("// schema integration\n{document}\n"),
    }
}

#[test]
fn public_content_validation_covers_shapes_the_typed_writer_cannot_construct() {
    schema::ensure_ready().unwrap();
    for document in [
        Value::Null,
        json!(true),
        json!(42),
        json!("text"),
        json!([]),
    ] {
        let violation = content(schema::validate(&document).unwrap_err());
        assert_eq!(violation.instance_path, "");
        assert_eq!(violation.kind, ViolationKind::Type);
        assert_eq!(violation.keyword, "type");
    }
    let violation = content(schema::validate(&json!({})).unwrap_err());
    assert_eq!(violation.instance_path, "");
    assert_eq!(
        violation.kind,
        ViolationKind::Required {
            property: "name".into()
        }
    );
    for field in ["name", "author", "description", "$schema"] {
        for value in [
            Value::Null,
            json!(true),
            json!(42),
            json!(1.5),
            json!([]),
            json!({}),
        ] {
            let mut document = json!({"name": "example"});
            document[field] = value;
            let violation = content(schema::validate(&document).unwrap_err());
            assert_eq!(violation.instance_path, format!("/{field}"));
            let missing_author_name = field == "author" && document[field].is_object();
            assert_eq!(
                violation.kind,
                if missing_author_name {
                    ViolationKind::Required {
                        property: "name".into(),
                    }
                } else {
                    ViolationKind::Type
                }
            );
            assert_eq!(
                violation.keyword,
                if missing_author_name {
                    "required"
                } else {
                    "type"
                }
            );
        }
    }
}

#[test]
fn schema_field_and_complete_document_paths_agree_without_normalizing() {
    for field in ["name", "author", "description", "$schema"] {
        for value in [
            "",
            " \t\r\n",
            "\u{85}",
            "\u{a0}",
            "\u{2003}",
            "\u{2028}\u{3000}",
        ] {
            let mut document = json!({"name": "example"});
            document[field] = json!(value);
            let whole = content(schema::validate(&document).unwrap_err());
            let single = content(schema::validate_field(field, value).unwrap_err());
            assert_eq!(single.instance_path, format!("/{field}"));
            assert_eq!(whole.instance_path, single.instance_path);
            assert_eq!(whole.kind, ViolationKind::NonBlank);
            assert_eq!(single.kind, ViolationKind::NonBlank);
        }
        // Name has a separate grammar; arbitrary accepted strings belong to
        // optional metadata and the uninterpreted editor-schema reference.
        if field == "name" {
            continue;
        }
        for value in [
            "\u{feff}",
            "\u{85}visible\u{2003}",
            "  配置 🦀  ",
            "a\u{0}b",
            "\"quote\" \\path\n",
        ] {
            let mut document = json!({"name": "example"});
            document[field] = json!(value);
            schema::validate_field(field, value).unwrap();
            schema::validate(&document).unwrap();
            let original = document.clone();
            validate_document(&document).unwrap();
            assert_eq!(document, original);
        }
    }
}

#[test]
fn independent_optional_fields_and_uninterpreted_data_are_valid() {
    for author in [None, Some("Team")] {
        for description in [None, Some("Shared rules")] {
            let mut document = json!({"name": "example", "extra": {"name": null, "$schema": false, "values": [null, true, 3, {"nested": "配置"}]}});
            if let Some(author) = author {
                document["author"] = json!(author);
            }
            if let Some(description) = description {
                document["description"] = json!(description);
            }
            schema::validate(&document).unwrap();
            let metadata = validate_document(&document).unwrap();
            assert_eq!(metadata.author.as_ref().map(hydragen::Author::name), author);
            assert_eq!(metadata.description.as_deref(), description);
        }
    }
}

fn valid_names() -> Vec<String> {
    let mut names: Vec<_> = [
        "a",
        "0",
        "123",
        "-config",
        "shared-config.v2_rules",
        "@example/shared-config",
        "@example/.config",
        "@example/_config",
        "@example/.",
        "@example/_",
        "@-team/config",
        "node_modules",
        "favicon.ico",
        "fs",
        "http",
        "constructor",
        "prototype",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    names.extend([
        "a".repeat(214),
        format!("@s/{}", "a".repeat(211)),
        format!("@{}/a", "s".repeat(211)),
    ]);
    names
}

fn invalid_names() -> Vec<String> {
    let mut names: Vec<_> = [
        "Shared-config",
        "shared-Config",
        " example",
        "example ",
        "two words",
        "a\tb",
        "a\nb",
        "配置",
        "café",
        "a\0b",
        ".config",
        "_config",
        ".",
        "_",
        "@",
        "@scope",
        "@/name",
        "@scope/",
        "@.scope/name",
        "@_scope/name",
        "@Scope/name",
        "@scope/Name",
        "@scope/name/extra",
        "scope/name",
        "@scope/name@other",
        "name+extra",
        "name%20extra",
        "\u{feff}",
        "\u{85}visible\u{2003}",
        "example\n",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    names.extend([
        "a".repeat(215),
        format!("@s/{}", "a".repeat(212)),
        format!("@{}/a", "s".repeat(212)),
    ]);
    names
}

#[test]
fn names_follow_the_same_explicit_schema_rules_in_fields_documents_and_every_codec() {
    for (names, valid) in [(valid_names(), true), (invalid_names(), false)] {
        for name in names {
            let expected =
                json!({"name": name, "extra": {"name": "NOT a layer name", "dependencies": false}});
            let original = expected.clone();
            assert_eq!(
                schema::validate_field("name", &name).is_ok(),
                valid,
                "{name:?}"
            );
            assert_eq!(schema::validate(&expected).is_ok(), valid, "{name:?}");
            assert_eq!(validate_document(&expected).is_ok(), valid, "{name:?}");
            assert_eq!(expected, original);
            for format in LayerManifestFormat::ALL {
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path().join(format.filename());
                let text = source(&expected, format);
                fs::write(&path, &text).unwrap();
                let loaded = discover(directory.path()).unwrap().load();
                if valid {
                    let loaded =
                        loaded.unwrap_or_else(|error| panic!("{format}: {name:?}: {error}"));
                    assert_eq!(loaded.metadata.name, name);
                    assert_eq!(loaded.document, expected);
                } else {
                    assert!(
                        matches!(loaded, Err(LayerManifestError::Metadata { .. })),
                        "{format}: {name:?}: {loaded:?}"
                    );
                    let violation = content(schema::validate(&expected).unwrap_err());
                    assert_eq!(violation.instance_path, "/name");
                    assert_ne!(violation.kind, ViolationKind::NonBlank, "{name:?}");
                }
                assert_eq!(fs::read_to_string(path).unwrap(), text);
                assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
            }
        }
    }
}

fn valid_references() -> Vec<String> {
    let mut references: Vec<_> = [
        ".",
        "..",
        "./",
        "../",
        "/",
        "./Shared Rules",
        "../Shared Config/../Agent Rules ",
        "/Not Present/Shared Rules",
        r".\Shared Rules",
        r"..\Shared Rules",
        r"C:\Shared Rules",
        r"\\server\Shared Rules",
        "shared-config@0.0.0",
        "shared-config@1.2.0",
        "shared-config@1.2.0-beta.1",
        "@example/shared-config@1.2.0+build.7",
        "@example/.config@1.2.3-alpha.0-rc+BUILD.007",
        "@example/_config@1.2.3-0",
        "shared-config@1.2.3-0A.01a.a-1+001.000",
        "node_modules@1.0.0",
        "project@1.0.0",
        "project@2.0.0",
        "../Same Path",
        "../Same Path",
        "shared-config@999999999999999999999999.0.0-999999999999999999999999+build",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    // The 214 limit is for the name, not the full reference or a path.
    references.extend([
        format!("{}@1.2.3-prerelease+build.7", "a".repeat(214)),
        format!("@s/{}@1.2.3", "a".repeat(211)),
        format!("../Shared Rules/{}", "A".repeat(300)),
    ]);
    references
}

fn invalid_references() -> Vec<String> {
    let mut references: Vec<_> = [
        "",
        " ",
        "shared-config",
        "Shared Rules",
        "shared/config",
        "@example/shared-config",
        "shared-config@",
        "shared-config@latest",
        "shared-config@^1.2.0",
        "shared-config@~1.2.0",
        "shared-config@*",
        "shared-config@1",
        "shared-config@1.2",
        "shared-config@1.2.3.4",
        "shared-config@v1.2.3",
        "shared-config@01.2.3",
        "shared-config@1.02.3",
        "shared-config@1.2.03",
        "shared-config@1.2.3-01",
        "shared-config@1.2.3-alpha.01",
        "shared-config@1.2.3-",
        "shared-config@1.2.3+",
        "shared-config@1.2.3-alpha..1",
        "shared-config@1.2.3+build..1",
        "shared-config@1.2.3-α",
        "shared-config@1.2.3+a_b",
        "shared-config@1.2.3@other",
        "shared-config@1.2.3 || 2.0.0",
        "shared-config@>=1.2.3",
        "shared-config@1.2.3 - 2.0.0",
        " shared-config@1.2.3",
        "shared-config@1.2.3 ",
        "shared-config@1.2.3\n",
        "Shared-config@1.2.3",
        ".config@1.2.3",
        "_config@1.2.3",
        "@Scope/name@1.2.3",
        "@scope/Name@1.2.3",
        "@.scope/name@1.2.3",
        "@/name@1.2.3",
        "@scope/@1.2.3",
        "file:../shared",
        "link:../shared",
        "npm:shared-config@1.2.3",
        "git+https://example.test/repo",
        "https://example.test/repo",
        "git@example.test:repo",
        "~/shared",
        "$HOME/shared",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    references.extend([
        format!("{}@1.2.3", "a".repeat(215)),
        format!("@s/{}@1.2.3", "a".repeat(212)),
    ]);
    references
}

#[test]
fn ordered_dependency_content_is_offline_and_identical_after_every_format_parser() {
    let references = valid_references();
    for dependencies in [None, Some(json!([])), Some(json!(references))] {
        let mut expected = json!({"name": "project", "$schema": "https://unavailable.invalid/schema.json", "extra": {"dependencies": null}});
        if let Some(dependencies) = dependencies {
            expected["dependencies"] = dependencies;
        }
        schema::validate(&expected).unwrap();
        validate_document(&expected).unwrap();
        for format in LayerManifestFormat::ALL {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(format.filename());
            // TOML has no null, even in unknown fields.
            let mut document = expected.clone();
            if format == LayerManifestFormat::Toml {
                document["extra"]["dependencies"] = json!(false);
            }
            let text = source(&document, format);
            fs::write(&path, &text).unwrap();
            let loaded = discover(directory.path()).unwrap().load().unwrap();
            assert_eq!(loaded.document, document, "{format}");
            assert_eq!(fs::read_to_string(path).unwrap(), text);
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
    }
}

#[test]
fn dependency_shapes_and_invalid_items_are_content_errors_in_every_format() {
    let mut invalid = vec![
        Value::Null,
        json!(true),
        json!(42),
        json!("../shared"),
        json!({"shared": "../shared"}),
        json!([null]),
        json!([true]),
        json!([42]),
        json!([["shared", "1.2.3"]]),
        json!([{"source": "../shared", "version": "1.2.3"}]),
    ];
    invalid.extend(
        invalid_references()
            .into_iter()
            .map(|reference| json!(["../valid", reference])),
    );
    for dependencies in invalid {
        let expected = json!({"name": "project", "dependencies": dependencies, "uninterpreted": {"dependencies": false}});
        let violation = content(schema::validate(&expected).unwrap_err());
        assert!(
            violation.instance_path.starts_with("/dependencies"),
            "{violation:?}"
        );
        assert_ne!(violation.kind, ViolationKind::NonBlank);
        assert!(validate_document(&expected).is_err());
        for format in LayerManifestFormat::ALL {
            if format == LayerManifestFormat::Toml
                && (dependencies.is_null()
                    || dependencies
                        .as_array()
                        .is_some_and(|items| items.iter().any(Value::is_null)))
            {
                continue;
            }
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join(format.filename());
            let text = source(&expected, format);
            fs::write(&path, &text).unwrap();
            let error = discover(directory.path()).unwrap().load().unwrap_err();
            assert!(
                matches!(error, LayerManifestError::Metadata { .. }),
                "{format}: {dependencies}: {error:?}"
            );
            assert!(error.to_string().contains("/dependencies"), "{error}");
            assert_eq!(fs::read_to_string(path).unwrap(), text);
        }
    }
}

#[test]
fn every_loader_format_validates_schema_property_and_retains_complete_data() {
    for format in LayerManifestFormat::ALL {
        for reference in [
            None,
            Some("https://unavailable.invalid/schema.json"),
            Some("file:///not-present/schema.json"),
            Some("\u{feff}"),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let mut expected = json!({
                "name": "@example/.config",
                "author": "  配置 🦀  ",
                "extra": {"name": false, "$schema": 42, "objects": [{"values": [true, 1, 2.5, "text"]}], "empty": {}}
            });
            if let Some(reference) = reference {
                expected["$schema"] = json!(reference);
            }
            fs::write(
                directory.path().join(format.filename()),
                source(&expected, format),
            )
            .unwrap();
            let loaded = discover(directory.path()).unwrap().load().unwrap();
            assert_eq!(loaded.document, expected, "{format}");
            assert_eq!(loaded.metadata.name, "@example/.config");
            assert_eq!(
                loaded.metadata.author.as_ref().map(hydragen::Author::name),
                Some("  配置 🦀  ")
            );
            assert_eq!(loaded.metadata.description, None);
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
        let mut invalid = vec![
            json!(""),
            json!(" \t\n"),
            json!("\u{85}"),
            json!(true),
            json!(12),
            json!(1.5),
            json!([]),
            json!({}),
        ];
        if format != LayerManifestFormat::Toml {
            invalid.push(Value::Null);
        }
        for value in invalid {
            let directory = tempfile::tempdir().unwrap();
            let expected = json!({"name": "example", "$schema": value});
            let path = directory.path().join(format.filename());
            let original = source(&expected, format);
            fs::write(&path, &original).unwrap();
            let error = discover(directory.path()).unwrap().load().unwrap_err();
            let LayerManifestError::Metadata {
                path: rejected,
                source,
            } = error
            else {
                panic!("expected schema rejection for {format}: {value}, got {error:?}");
            };
            assert_eq!(rejected, path);
            if value.is_string() {
                assert_eq!(source, MetadataError::Blank { field: "$schema" });
            } else {
                assert_eq!(source, MetadataError::NotString { field: "$schema" });
                assert!(!source.to_string().contains("non-whitespace"));
            }
            assert_eq!(fs::read_to_string(path).unwrap(), original);
        }
    }
}

#[test]
fn schema_invalid_higher_priority_never_falls_back_or_loses_discovery() {
    for (index, higher) in LayerManifestFormat::ALL.into_iter().enumerate() {
        for lower in LayerManifestFormat::ALL.into_iter().skip(index + 1) {
            let directory = tempfile::tempdir().unwrap();
            let invalid = source(&json!({"name": "higher", "$schema": false}), higher);
            fs::write(directory.path().join(higher.filename()), &invalid).unwrap();
            fs::write(
                directory.path().join(lower.filename()),
                source(&json!({"name": "lower"}), lower),
            )
            .unwrap();
            let discovery = discover(directory.path()).unwrap();
            assert!(
                matches!(discovery.load(), Err(LayerManifestError::Metadata { path, source: MetadataError::NotString { field: "$schema" } }) if path.file_name().unwrap() == higher.filename())
            );
            assert!(discovery.has_duplicates());
            assert_eq!(discovery.candidates.len(), 2);
            assert_eq!(discovery.selected().unwrap().format, higher);
            assert_eq!(
                fs::read_to_string(&discovery.selected().unwrap().path).unwrap(),
                invalid
            );
        }
    }
}

#[test]
fn error_adapters_do_not_relabel_arbitrary_rules_or_internal_failures_as_blank() {
    let setup = schema::validate_field("unknown", "otherwise valid").unwrap_err();
    assert!(matches!(setup, Error::Setup { .. }));
    let init = InitError::from(setup.clone());
    assert!(matches!(init, InitError::Schema(Error::Setup { .. })));
    assert!(
        init.to_string()
            .contains("Internal layer manifest schema error")
    );
    assert!(!init.to_string().contains("non-whitespace"));
    assert!(init.source().is_some());
    assert_eq!(
        MetadataError::from(setup.clone()),
        MetadataError::Schema(setup)
    );

    for (kind, keyword, path) in [
        (ViolationKind::Other, "maxLength", "/name"),
        (ViolationKind::Other, "pattern", "/author"),
        (ViolationKind::Other, "maxProperties", ""),
        (ViolationKind::Type, "type", "/extra/name"),
    ] {
        let error = Error::Content(Box::new(schema::Violation {
            instance_path: path.into(),
            schema_path: "/properties/extra".into(),
            keyword: keyword.into(),
            message: "independent test violation".into(),
            kind,
        }));
        let init = InitError::from(error.clone());
        let metadata = MetadataError::from(error);
        for diagnostic in [init.to_string(), metadata.to_string()] {
            assert!(diagnostic.contains(if path.is_empty() { "<root>" } else { path }));
            assert!(diagnostic.contains(keyword));
            assert!(!diagnostic.contains("non-whitespace"));
        }
        assert!(matches!(init, InitError::Schema(Error::Content(_))));
        assert!(matches!(metadata, MetadataError::Schema(Error::Content(_))));
    }
}
