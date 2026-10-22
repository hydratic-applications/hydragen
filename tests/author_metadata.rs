use std::path::Path;

use hydragen::layer_manifest::{parse_source, schema};
use hydragen::{
    Author, AuthorObject, LayerManifestError, LayerManifestFormat as Format, Metadata,
    MetadataError, encode_metadata, validate_document,
};
use serde_json::{Value, json};

fn parsed(
    format: Format,
    source: &str,
) -> Result<hydragen::LoadedLayerManifest, LayerManifestError> {
    parse_source(
        &Path::new("/diagnostic-only").join(format.filename()),
        format,
        source.into(),
    )
}

fn source(format: Format, value: &Value) -> String {
    match format {
        Format::Yaml | Format::Yml => serde_saphyr::to_string(value).unwrap(),
        Format::Toml => toml::to_string(value).unwrap(),
        Format::Json => value.to_string(),
        Format::Jsonc => format!("// author fixture\n{value}\n"),
    }
}

fn contains_null(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(values) => values.iter().any(contains_null),
        Value::Object(values) => values.values().any(contains_null),
        _ => false,
    }
}

fn violation(error: schema::Error) -> schema::Violation {
    let schema::Error::Content(violation) = error else {
        panic!("expected content error: {error:?}")
    };
    *violation
}

#[test]
fn maintained_valid_author_fixtures_retain_representation_and_all_data_in_every_format() {
    let authors: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/layer-manifest/authors/valid.json")).unwrap();
    for author in authors {
        let expected =
            json!({"name": "example", "author": author, "description": "  exact description  "});
        schema::validate_value("author", &author).unwrap();
        let metadata = validate_document(&expected).unwrap();
        assert_eq!(
            serde_json::to_value(metadata.author.as_ref().unwrap()).unwrap(),
            author
        );
        assert_eq!(
            matches!(metadata.author, Some(Author::String(_))),
            author.is_string()
        );
        for format in Format::ALL {
            let text = source(format, &expected);
            let loaded = parsed(format, &text).unwrap();
            assert_eq!(loaded.metadata, metadata, "{format}: {author}");
            assert_eq!(loaded.document, expected);
            assert_eq!(loaded.source, text);
            // General library encoding retains either representation, including
            // unknown object data; only init's input layer chooses object form.
            let bytes = encode_metadata(&metadata, format).unwrap();
            let roundtrip = parsed(format, std::str::from_utf8(&bytes).unwrap()).unwrap();
            assert_eq!(roundtrip.document, expected, "{format}: {author}");
        }
    }
}

#[test]
fn maintained_invalid_author_fixtures_fail_the_same_rule_through_fields_and_all_loaders() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/layer-manifest/authors/invalid.json")).unwrap();
    for fixture in fixtures {
        let author = fixture["author"].clone();
        let expected = json!({"name": "example", "author": author});
        let failure = violation(schema::validate_value("author", &author).unwrap_err());
        assert_eq!(
            failure.instance_path,
            fixture["path"].as_str().unwrap(),
            "{fixture}"
        );
        assert_eq!(
            failure.keyword,
            fixture["keyword"].as_str().unwrap(),
            "{fixture}"
        );
        assert_eq!(failure, violation(schema::validate(&expected).unwrap_err()));
        assert!(!["anyOf", "oneOf", "if"].contains(&failure.keyword.as_str()));
        for format in Format::ALL {
            if format == Format::Toml && contains_null(&expected) {
                continue;
            }
            let text = source(format, &expected);
            let error = parsed(format, &text).unwrap_err();
            assert!(
                matches!(error, LayerManifestError::Metadata { .. }),
                "{format}: {fixture}: {error:?}"
            );
            assert!(error.to_string().contains("author"), "{error}");
            // Nested failures remain structured, with no string-only extraction.
            if failure.instance_path.starts_with("/author/") {
                let LayerManifestError::Metadata {
                    source: MetadataError::Schema(schema::Error::Content(actual)),
                    ..
                } = error
                else {
                    panic!("lost nested failure: {error:?}")
                };
                assert_eq!(*actual, failure);
            }
        }
    }
}

#[test]
fn toml_inline_table_explicit_table_and_dotted_author_forms_are_equivalent_and_read_only() {
    let expected = json!({"name": "example", "description": "root description", "author": {"name": "  配置 🦀  ", "email": "Alex@example.invalid", "url": "https://example.invalid", "extra": {"array": [true, 7]}}});
    for text in [
        "name = 'example'\nauthor = { url = 'https://example.invalid', extra = { array = [true, 7] }, name = '  配置 🦀  ', email = 'Alex@example.invalid' } # inline author\ndescription = 'root description'\n",
        "name = 'example'\ndescription = 'root description'\n\n# author table\n[author]\nurl = 'https://example.invalid'\nname = '  配置 🦀  '\nemail = 'Alex@example.invalid' # email\n[author.extra]\narray = [true, 7]\n",
        "author.url = 'https://example.invalid'\nname = 'example'\nauthor.name = '  配置 🦀  '\nauthor.extra.array = [true, 7]\ndescription = 'root description'\nauthor.email = 'Alex@example.invalid' # email\n",
    ] {
        let loaded = parsed(Format::Toml, text).unwrap();
        assert_eq!(loaded.document, expected);
        assert_eq!(loaded.metadata, validate_document(&expected).unwrap());
        assert_eq!(loaded.source, text);
        let Some(Author::Object(author)) = loaded.metadata.author else {
            panic!()
        };
        assert_eq!(author.extra()["extra"], json!({"array": [true, 7]}));
    }
}

#[test]
fn nested_duplicate_known_and_unknown_author_keys_remain_decoder_errors() {
    for (format, text) in [
        (
            Format::Yaml,
            "name: example\nauthor: {name: Alex, name: Other}",
        ),
        (
            Format::Yml,
            "name: example\nauthor:\n  name: Alex\n  email: a@example.invalid\n  email: b@example.invalid\n",
        ),
        (
            Format::Yaml,
            "name: example\nauthor: {name: Alex, extra: {x: 1, x: 2}}",
        ),
        (
            Format::Json,
            r#"{"name":"example","author":{"name":"Alex","\u006eame":"Other"}}"#,
        ),
        (
            Format::Jsonc,
            r#"{"name":"example","author":{"name":"Alex","extra":{"x":1,"\u0078":2}}}"#,
        ),
        (
            Format::Toml,
            "name = 'example'\nauthor = {name = 'Alex', name = 'Other'}",
        ),
        (
            Format::Toml,
            "name = 'example'\n[author]\nname = 'Alex'\nname = 'Other'",
        ),
        (
            Format::Toml,
            "name = 'example'\nauthor.name = 'Alex'\n[author]\nname = 'Other'",
        ),
        (Format::Toml, "name = 'example'\n[[author]]\nname = 'Alex'"),
    ] {
        let error = parsed(format, text).unwrap_err();
        if text.contains("[[author]]") {
            let LayerManifestError::Metadata {
                source: MetadataError::Schema(schema::Error::Content(failure)),
                ..
            } = error
            else {
                panic!("{error:?}")
            };
            assert_eq!(failure.instance_path, "/author");
            assert_eq!(failure.kind, schema::ViolationKind::Type);
        } else {
            assert!(
                matches!(error, LayerManifestError::Decode { .. }),
                "{error:?}"
            );
            assert!(
                error.to_string().to_lowercase().contains("duplicate"),
                "{error}"
            );
        }
    }
}

#[test]
fn object_encoding_has_stable_root_and_nested_order_independent_contacts_and_exact_values() {
    for email in [None, Some("Alex+work@unreachable.invalid")] {
        for url in [None, Some("mailto:Alex@unreachable.invalid")] {
            for description in [None, Some("Root description")] {
                let mut author = AuthorObject::new("  配置 🦀  ");
                author.email = email.map(str::to_owned);
                author.url = url.map(str::to_owned);
                let metadata = Metadata {
                    name: "example".into(),
                    author: Some(author.into()),
                    description: description.map(str::to_owned),
                };
                let expected = serde_json::to_value(&metadata).unwrap();
                for format in Format::ALL {
                    let bytes = encode_metadata(&metadata, format).unwrap();
                    let text = std::str::from_utf8(&bytes).unwrap();
                    assert!(text.ends_with('\n'));
                    let independently_decoded: Value = match format {
                        Format::Yaml | Format::Yml => serde_saphyr::from_str(text).unwrap(),
                        Format::Toml => {
                            serde_json::to_value(toml::from_str::<toml::Table>(text).unwrap())
                                .unwrap()
                        }
                        Format::Json | Format::Jsonc => serde_json::from_str(text).unwrap(),
                    };
                    assert_eq!(independently_decoded, expected, "{text}");
                    assert_eq!(parsed(format, text).unwrap().metadata, metadata);
                    let root_author = match format {
                        Format::Json | Format::Jsonc => text.find("  \"author\": {").unwrap(),
                        _ => text.find("author").unwrap(),
                    };
                    assert!(text.find("name").unwrap() < root_author);
                    if let Some(position) = text.find("description") {
                        assert!(root_author < position);
                    }
                    if let (Some(email), Some(url)) = (text.find("email"), text.find("url")) {
                        assert!(email < url);
                    }
                    match format {
                        Format::Toml => {
                            let line = text
                                .lines()
                                .find(|line| line.starts_with("author = "))
                                .unwrap();
                            assert!(line.starts_with("author = { name = "), "{text}");
                            assert!(line.ends_with(" }"), "{text}");
                            assert!(!text.contains("[author]"));
                        }
                        Format::Yaml | Format::Yml => {
                            assert!(text.contains("author:\n  name:"), "{text}")
                        }
                        Format::Json | Format::Jsonc => {
                            assert!(text.contains("\"author\": {\n    \"name\":"), "{text}")
                        }
                    }
                    assert_eq!(
                        independently_decoded.as_object().unwrap().len(),
                        2 + usize::from(description.is_some())
                    );
                    assert_eq!(
                        independently_decoded["author"].as_object().unwrap().len(),
                        1 + usize::from(email.is_some()) + usize::from(url.is_some())
                    );
                }
            }
        }
    }
}

#[test]
fn object_name_escaping_and_additional_null_data_are_not_normalized_or_lost() {
    for name in [
        "true",
        "123",
        "null",
        "2026-10-20",
        "  A \"quote\" \\path\n配置 🦀  ",
        "a\0b",
    ] {
        let metadata = Metadata {
            name: "example".into(),
            author: Some(AuthorObject::new(name).into()),
            description: Some(name.into()),
        };
        for format in Format::ALL {
            let bytes = encode_metadata(&metadata, format).unwrap();
            assert_eq!(
                parsed(format, std::str::from_utf8(&bytes).unwrap())
                    .unwrap()
                    .metadata,
                metadata
            );
        }
    }
    let document = json!({"name": "example", "author": {"name": "Alex", "extra": {"null": null, "array": [null, {"name": false}]}}});
    for format in [Format::Yaml, Format::Yml, Format::Json, Format::Jsonc] {
        let loaded = parsed(format, &source(format, &document)).unwrap();
        assert_eq!(
            serde_json::to_value(loaded.metadata.author).unwrap(),
            document["author"]
        );
    }
}

#[test]
fn invalid_object_candidates_fail_before_exact_target_creation() {
    for (field, value) in [
        ("name", "\u{2003}"),
        ("email", "not-an-email"),
        ("url", "relative/path"),
    ] {
        let mut author = AuthorObject::new("Alex");
        match field {
            "name" => author.name = value.into(),
            "email" => author.email = Some(value.into()),
            _ => author.url = Some(value.into()),
        }
        let metadata = Metadata {
            name: "example".into(),
            author: Some(author.into()),
            description: None,
        };
        for format in Format::ALL {
            let root = tempfile::tempdir().unwrap();
            let prepared = hydragen::inspect_init(root.path())
                .unwrap()
                .prepare()
                .unwrap();
            let error = hydragen::create_init(prepared, &metadata, format).unwrap_err();
            assert_eq!(error.state, hydragen::InitCreationState::NotCreated);
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        }
    }
}

#[test]
fn every_known_author_field_rejects_all_non_string_types_without_coercion() {
    for (field, selector) in [
        ("name", "author.name"),
        ("email", "author.email"),
        ("url", "author.url"),
    ] {
        for value in [
            Value::Null,
            json!(false),
            json!(42),
            json!(1.5),
            json!([]),
            json!({}),
        ] {
            let mut author = json!({"name": "Alex"});
            author[field] = value.clone();
            let field_error = violation(schema::validate_value(selector, &value).unwrap_err());
            assert_eq!(field_error.instance_path, format!("/author/{field}"));
            assert_eq!(field_error.kind, schema::ViolationKind::Type);
            assert_eq!(field_error.keyword, "type");
            let expected = json!({"name": "example", "author": author});
            assert_eq!(
                field_error,
                violation(schema::validate(&expected).unwrap_err())
            );
            for format in Format::ALL {
                if format == Format::Toml && value.is_null() {
                    continue;
                }
                let error = parsed(format, &source(format, &expected)).unwrap_err();
                let LayerManifestError::Metadata {
                    source: MetadataError::Schema(schema::Error::Content(actual)),
                    ..
                } = error
                else {
                    panic!("{format}: {expected}: {error:?}")
                };
                assert_eq!(*actual, field_error);
            }
        }
    }
}

#[test]
fn verified_add_and_atomic_replacement_keep_existing_author_representation() {
    for format in Format::ALL {
        for author in [
            json!("  Alex <bad> arbitrary existing author  "),
            json!({"name": "  Alex  ", "email": "a@example.invalid", "url": "urn:example:author", "extra": {"data": [false, 7]}}),
        ] {
            let root = tempfile::tempdir().unwrap();
            let app = root.path().join("app");
            let shared = root.path().join("shared");
            std::fs::create_dir(&app).unwrap();
            std::fs::create_dir(&shared).unwrap();
            let path = app.join(format.filename());
            let document =
                json!({"name": "app", "author": author, "description": "root description"});
            let original = source(format, &document);
            std::fs::write(&path, &original).unwrap();
            std::fs::write(shared.join("layer.yaml"), "name: shared\n").unwrap();
            let plan = hydragen::dependencies::prepare_add(&app, &["../shared".into()]).unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
            assert_eq!(plan.current().document, document);
            assert_eq!(
                serde_json::to_value(&plan.current().metadata.author).unwrap(),
                author
            );
            let references: Vec<_> = plan.new_references().collect();
            let candidate =
                hydragen::layer_manifest::edit::append_dependencies(plan.current(), &references)
                    .unwrap();
            hydragen::replacement::replace_layer_manifest(&path, candidate.as_bytes()).unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), candidate);
            let loaded = hydragen::discover(&app).unwrap().load().unwrap();
            assert_eq!(loaded.document["author"], author);
            assert_eq!(loaded.metadata, plan.current().metadata);
            assert_eq!(loaded.document["dependencies"], json!(["../shared"]));
            assert_eq!(std::fs::read_dir(&app).unwrap().count(), 1);
            assert!(
                hydragen::dependencies::prepare_add(&app, &["../shared".into()])
                    .unwrap()
                    .is_noop()
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), candidate);
        }
    }
}
