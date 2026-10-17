//! Unsupported YAML tags are rejected before their source type can be lost.

use std::fs;

use hydragen::{LayerManifestError, LayerManifestFormat as Format, discover};
use serde_json::{Value, json};

const YAML_FORMATS: [Format; 2] = [Format::Yaml, Format::Yml];
const BINARY_TAGS: [(&str, &str); 20] = [
    ("", "!!binary"),
    ("", "!<tag:yaml.org,2002:binary>"),
    ("%TAG !b! tag:yaml.org,2002:\n---\n", "!b!binary"),
    ("%TAG !b! tag:yaml.org,2002:bi\n---\n", "!b!nary"),
    ("%TAG !b! tag:yaml.org,\n---\n", "!b!2002:binary"),
    ("%TAG ! tag:yaml.org,2002:\n---\n", "!binary"),
    ("%TAG !! tag:yaml.org,2002:b\n---\n", "!!inary"),
    // serde-saphyr also recognizes local and legacy compatibility identities.
    // Their directive spellings must be checked after parser resolution too.
    ("", "!binary"),
    ("", "!<!binary>"),
    ("", "!<!!binary>"),
    ("", "!<tag:yaml.org,2002:!binary>"),
    ("%TAG !b! !\n---\n", "!b!binary"),
    ("%TAG !b! !bi\n---\n", "!b!nary"),
    ("%TAG !b! !!\n---\n", "!b!binary"),
    ("%TAG !b! !!bi\n---\n", "!b!nary"),
    ("%TAG !b! tag:yaml.org,2002:!\n---\n", "!b!binary"),
    ("%TAG !b! tag:yaml.org,2002:!bi\n---\n", "!b!nary"),
    ("%TAG !b! tag:yaml.org,\n---\n", "!b!2002:%21binary"),
    ("%TAG ! tag:yaml.org,2002:!\n---\n", "!binary"),
    ("%TAG !! !\n---\n", "!!binary"),
];

fn read(format: Format, text: &str) -> Result<Value, LayerManifestError> {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join(format.filename()), text).unwrap();
    discover(directory.path())
        .unwrap()
        .load()
        .map(|loaded| loaded.document)
}

fn positions(scalar: &str) -> [(&'static str, bool, String); 9] {
    [
        ("/name", false, format!("name: {scalar}")),
        ("/author", false, format!("name: root\nauthor: {scalar}")),
        (
            "/description",
            false,
            format!("name: root\ndescription: {scalar}"),
        ),
        ("/extra", false, format!("name: root\nextra: {scalar}")),
        (
            "/extra/nested",
            false,
            format!("name: root\nextra: {{nested: {scalar}}}"),
        ),
        (
            "/extra/nested/0",
            false,
            format!("name: root\nextra: {{nested: [{scalar}]}}"),
        ),
        ("", true, format!("name: root\n? {scalar}\n: value")),
        (
            "/extra",
            true,
            format!("name: root\nextra:\n  ? {scalar}\n  : value"),
        ),
        (
            "/extra/nested/0",
            true,
            format!("name: root\nextra:\n  nested:\n  - ? {scalar}\n    : value"),
        ),
    ]
}

fn assert_binary_rejected(format: Format, text: &str) {
    let error = match read(format, text) {
        Err(error) => error,
        Ok(document) => panic!("{format}: expected binary rejection for {text:?}, got {document}"),
    };
    assert!(
        matches!(error, LayerManifestError::Decode { .. }),
        "{error}"
    );
    let message = error.to_string();
    assert!(message.contains(format.filename()), "{message}");
    assert!(
        message.contains("YAML binary tags are not supported"),
        "{text}\n{message}"
    );
    assert!(
        message.contains("line ") && message.contains("column "),
        "{message}"
    );
}

#[test]
fn yaml_binary_tag_matrix_matches_the_maintained_parsers_binary_semantics() {
    // This independent check makes sure the rejection matrix is limited to
    // binary aliases actually recognized by the current maintained parser.
    for (prefix, tag) in BINARY_TAGS {
        let text = format!("{prefix}name: {tag} SGVsbG8=");
        let document: Value = serde_saphyr::from_str(&text).unwrap();
        assert_eq!(document, json!({"name": "Hello"}), "{text}");
    }
}

#[test]
fn yaml_binary_tags_reject_utf8_non_utf8_and_invalid_base64_at_every_position() {
    for format in YAML_FORMATS {
        for (prefix, tag) in BINARY_TAGS {
            for payload in [
                "SGVsbG8=", "8J+agA==", "bmFtZQ==", "/w==", "", "!!!", "null", "~",
            ] {
                for scalar in [format!("{tag} '{payload}'"), format!("{tag} \"{payload}\"")] {
                    for (_, _, text) in positions(&scalar) {
                        assert_binary_rejected(format, &format!("{prefix}{text}"));
                    }
                }
            }
            // Plain scalars are distinct from quoted scalars, but the resolved
            // binary tag rejects both. Empty/invalid payloads above stay quoted
            // so their rejection cannot be attributed to YAML syntax instead.
            for payload in ["SGVsbG8=", "8J+agA==", "/w=="] {
                for (_, _, text) in positions(&format!("{tag} {payload}")) {
                    assert_binary_rejected(format, &format!("{prefix}{text}"));
                }
            }
        }
    }
}

#[test]
fn yaml_binary_tags_reject_block_scalars_and_explicit_block_keys() {
    for format in YAML_FORMATS {
        for (prefix, tag) in BINARY_TAGS {
            for payload in ["SGVs\n  bG8=", "/w=="] {
                let scalar = format!("{tag} |\n  {payload}");
                for text in [
                    format!("name: {scalar}\n"),
                    format!("name: root\nauthor: {scalar}\n"),
                    format!("name: root\ndescription: {scalar}\n"),
                    format!("name: root\nextra: {scalar}\n"),
                    format!(
                        "name: root\nextra:\n  nested: {}\n",
                        scalar.replace('\n', "\n  ")
                    ),
                    format!(
                        "name: root\nextra:\n  nested:\n    - {}\n",
                        scalar.replace('\n', "\n    ")
                    ),
                    format!("name: root\n? {scalar}\n: value\n"),
                    format!(
                        "name: root\nextra:\n  ? {}\n  : value\n",
                        scalar.replace('\n', "\n  ")
                    ),
                    format!(
                        "name: root\nextra:\n  nested:\n    - ? {}\n      : value\n",
                        scalar.replace('\n', "\n      ")
                    ),
                ] {
                    assert_binary_rejected(format, &format!("{prefix}{text}"));
                }
            }
        }
    }
}

#[test]
fn yaml_binary_anchors_cannot_hide_tags_in_values_or_keys() {
    for format in YAML_FORMATS {
        for (prefix, tag) in BINARY_TAGS {
            for payload in ["SGVsbG8=", "/w=="] {
                for text in [
                    format!("extra: &data {tag} {payload}\nname: *data"),
                    format!(
                        "name: root\nextra: &data {tag} {payload}\nauthor: *data\ndescription: *data"
                    ),
                    format!("name: root\nextra: &data {tag} {payload}\ncopy: {{nested: [*data]}}"),
                    format!(
                        "name: root\nextra: &data {tag} {payload}\ncopy:\n  ? *data\n  : value"
                    ),
                    format!(
                        "name: root\nextra:\n  ? &data {tag} {payload}\n  : value\ncopy: [*data]"
                    ),
                ] {
                    assert_binary_rejected(format, &format!("{prefix}{text}"));
                }
            }
        }
    }
}

#[test]
fn yaml_binary_tag_rejection_does_not_change_ordinary_or_explicit_strings() {
    for format in YAML_FORMATS {
        for value in [
            "SGVsbG8=",
            "/w==",
            "!!binary SGVsbG8=",
            "tag:yaml.org,2002:binary",
            "!b!nary SGVsbG8=",
            "!binary SGVsbG8=",
            "tag:yaml.org,2002:!binary",
            "!<!!binary> SGVsbG8=",
        ] {
            for scalar in [
                format!("'{value}'"),
                format!("\"{value}\""),
                format!("!!str '{value}'"),
                format!("!<tag:yaml.org,2002:str> '{value}'"),
                format!("!str '{value}'"),
                format!("!<tag:yaml.org,2002:!str> '{value}'"),
                format!("! '{value}'"),
            ] {
                for (pointer, key, text) in positions(&scalar) {
                    let document = read(format, &text).unwrap();
                    if key {
                        assert_eq!(document.pointer(pointer).unwrap()[value], "value");
                    } else {
                        assert_eq!(document.pointer(pointer), Some(&json!(value)));
                    }
                }
            }
        }
        let prefix = "%TAG !b! tag:yaml.org,2002:s\n---\n";
        let document = read(format, &format!("{prefix}name: &data !b!tr SGVsbG8=\nauthor: *data\ndescription: *data\nextra:\n  ? *data\n  : /w==")).unwrap();
        assert_eq!(
            document,
            json!({"name":"SGVsbG8=", "author":"SGVsbG8=", "description":"SGVsbG8=", "extra":{"SGVsbG8=":"/w=="}})
        );
        let document = read(format, "# !!binary SGVsbG8=\nname: |-\n  !!binary SGVsbG8=\nextra: >-\n  !<tag:yaml.org,2002:binary> /w==\n").unwrap();
        assert_eq!(document["name"], "!!binary SGVsbG8=");
        assert_eq!(document["extra"], "!<tag:yaml.org,2002:binary> /w==");
    }
}

#[test]
fn yaml_local_and_legacy_integer_tags_retain_explicit_numeric_semantics() {
    let tags = [
        ("", "!int"),
        ("", "!<!int>"),
        ("", "!<!!int>"),
        ("", "!<tag:yaml.org,2002:!int>"),
        ("%TAG !i! !\n---\n", "!i!int"),
        ("%TAG !i! tag:yaml.org,2002:!i\n---\n", "!i!nt"),
    ];
    for format in YAML_FORMATS {
        for (prefix, tag) in tags {
            let document =
                read(format, &format!("{prefix}name: root\nextra: {tag} '0X2A'")).unwrap();
            assert_eq!(document["extra"], 42);
            for scalar in [
                "18_446_744_073_709_551_616",
                "-0X8000000000000001",
                "0b10000000000000000000000000000000000000000000000000000000000000000",
            ] {
                for spelling in [format!("{tag} {scalar}"), format!("{tag} '{scalar}'")] {
                    for (_, _, text) in positions(&spelling) {
                        let error = read(format, &format!("{prefix}{text}")).unwrap_err();
                        assert!(
                            matches!(error, LayerManifestError::Decode { .. }),
                            "{error}"
                        );
                        assert!(
                            error.to_string().contains("invalid u64")
                                || error.to_string().contains("invalid i64"),
                            "{error}"
                        );
                    }
                }
            }
            let error = read(
                format,
                &format!("{prefix}extra: &number {tag} '0X10000000000000000'\nname: *number"),
            )
            .unwrap_err();
            assert!(
                matches!(error, LayerManifestError::Decode { .. }),
                "{error}"
            );
        }
    }
}

#[test]
fn yaml_scalar_checks_preserve_deep_data_and_parser_limits() {
    for format in YAML_FORMATS {
        let opening = "[{nested: ".repeat(20);
        let closing = "}]".repeat(20);
        let document = read(
            format,
            &format!("name: root\nextra: {opening}!!str 0X10000000000000000{closing}"),
        )
        .unwrap();
        let pointer = format!("/extra{}", "/0/nested".repeat(20));
        assert_eq!(
            document.pointer(&pointer),
            Some(&json!("0X10000000000000000"))
        );
        let error = read(
            format,
            &format!("name: root\nextra: {opening}0X10000000000000000{closing}"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("i64/u64 range"), "{error}");
        assert_binary_rejected(
            format,
            &format!("name: root\nextra: {opening}!!binary SGVsbG8={closing}"),
        );

        let depth = serde_saphyr::Budget::default().max_depth + 1;
        let error = read(
            format,
            &format!(
                "name: root\nextra: {}text{}",
                "[".repeat(depth),
                "]".repeat(depth)
            ),
        )
        .unwrap_err();
        assert!(error.to_string().contains("depth"), "{error}");
        assert!(read(format, "name: root\nextra: &cycle [*cycle]").is_err());
        let excessive_aliases = format!(
            "name: root\ntext: &text quoted\nextra: [{}]",
            vec!["*text"; 101].join(", ")
        );
        assert!(read(format, &excessive_aliases).is_err());
    }
}
