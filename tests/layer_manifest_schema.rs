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
            assert_eq!(violation.kind, ViolationKind::Type);
            assert_eq!(violation.keyword, "type");
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
            assert_eq!(metadata.author.as_deref(), author);
            assert_eq!(metadata.description.as_deref(), description);
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
                "name": "  配置 🦀  ",
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
            assert_eq!(loaded.metadata.name, "  配置 🦀  ");
            assert_eq!(loaded.metadata.author, None);
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
