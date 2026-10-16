//! YAML overflow regressions use the public loader, including both filename choices.

use std::fs;

use hydragen::{ManifestError, ManifestFormat as Format, MetadataError, discover};
use serde_json::{Value, json};

const YAML_FORMATS: [Format; 2] = [Format::Yaml, Format::Yml];

fn read(format: Format, text: &str) -> Result<Value, ManifestError> {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join(format.filename()), text).unwrap();
    discover(directory.path())
        .unwrap()
        .load()
        .map(|loaded| loaded.document)
}

fn positions(scalar: &str) -> [(&'static str, bool, String); 8] {
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
        // Explicit-key notation avoids YAML's 1,024-character simple-key limit,
        // so the large-number cases exercise numeric classification, not syntax.
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

fn overflowing_integers() -> Vec<String> {
    vec![
        "18446744073709551616".into(),
        "+18446744073709551616".into(),
        "-9223372036854775809".into(),
        "0x10000000000000000".into(),
        "0o2000000000000000000000".into(),
        "340282366920938463463374607431768211456".into(),
        "+340282366920938463463374607431768211456".into(),
        "-340282366920938463463374607431768211456".into(),
        format!("0x1{}", "0".repeat(32)),
        format!("0o4{}", "0".repeat(42)),
        format!("0x{}", "f".repeat(4096)),
        format!("0o{}", "7".repeat(4096)),
        "9".repeat(4096),
    ]
}

fn assert_string_at_every_position(format: Format, spelling: &str, expected: &str) {
    for (pointer, key, text) in positions(spelling) {
        let document = read(format, &text)
            .unwrap_or_else(|error| panic!("{format} {pointer} key={key}: {error}"));
        let value = document.pointer(pointer).unwrap();
        if key {
            assert_eq!(
                value.as_object().unwrap().get(expected),
                Some(&json!("value"))
            );
        } else {
            assert_eq!(value.as_str(), Some(expected), "{format} {pointer}");
        }
    }
}

#[test]
fn yaml_integer_overflow_is_rejected_in_every_metadata_value_and_key_position() {
    for format in YAML_FORMATS {
        for scalar in overflowing_integers() {
            for (pointer, key, text) in positions(&scalar) {
                let error = read(format, &text).unwrap_err().to_string();
                assert!(error.contains(format.filename()), "{error}");
                assert!(
                    error.contains("i64/u64 range")
                        || error.contains("finite")
                        || (key && error.contains("string object key")),
                    "{format} {pointer} key={key}: {error}"
                );
            }
        }
    }
}

#[test]
fn yaml_integer_overflow_quoted_and_explicit_string_controls_remain_strings() {
    for format in YAML_FORMATS {
        for scalar in overflowing_integers() {
            for spelling in [
                format!("'{scalar}'"),
                format!("\"{scalar}\""),
                format!("!!str {scalar}"),
                format!("!<tag:yaml.org,2002:str> {scalar}"),
                format!("!!str '{scalar}'"),
                format!("! {scalar}"),
            ] {
                assert_string_at_every_position(format, &spelling, &scalar);
            }
        }
    }
}

#[test]
fn yaml_integer_supported_boundaries_remain_numbers_not_metadata_or_string_keys() {
    let cases = [
        ("0", json!(0)),
        ("-0", json!(0)),
        ("+0", json!(0)),
        ("9223372036854775807", json!(i64::MAX)),
        ("-9223372036854775808", json!(i64::MIN)),
        ("18446744073709551615", json!(u64::MAX)),
        ("+18446744073709551615", json!(u64::MAX)),
        ("0x7FFFFFFFFFFFFFFF", json!(i64::MAX)),
        ("0x8000000000000000", json!(1u64 << 63)),
        ("0xffffffffffffffff", json!(u64::MAX)),
        ("0o1777777777777777777777", json!(u64::MAX)),
    ];
    for format in YAML_FORMATS {
        for (scalar, expected) in &cases {
            for (pointer, key, text) in positions(scalar) {
                if matches!(pointer, "/name" | "/author" | "/description") {
                    assert!(matches!(
                        read(format, &text),
                        Err(ManifestError::Metadata {
                            source: MetadataError::NotString { .. },
                            ..
                        })
                    ));
                } else if key {
                    assert!(read(format, &text).is_err());
                } else {
                    assert_eq!(
                        read(format, &text).unwrap().pointer(pointer),
                        Some(expected)
                    );
                }
            }
        }
    }
}

#[test]
fn yaml_integer_guard_accepts_arbitrarily_padded_in_range_values() {
    for format in YAML_FORMATS {
        for scalar in [
            format!("0x{}1", "0".repeat(4096)),
            format!("0o{}1", "0".repeat(4096)),
        ] {
            let document =
                read(format, &format!("name: x\nextra: &n {scalar}\ncopy: [*n]")).unwrap();
            assert_eq!(document["extra"], 1);
            assert_eq!(document["copy"][0], 1);
        }
    }
}

#[test]
fn yaml_integer_guard_preserves_ordinary_numeric_looking_strings() {
    for format in YAML_FORMATS {
        for scalar in [
            "yes",
            "on",
            "0xword",
            "0o8",
            "0x",
            "0o",
            "+text",
            "-text",
            "2026-10-17",
            "18446744073709551616text",
        ] {
            assert_string_at_every_position(format, scalar, scalar);
        }
        for scalar in [
            format!("0x{}g", "f".repeat(4096)),
            format!("0o{}8", "7".repeat(4096)),
            format!("{}x", "9".repeat(4096)),
        ] {
            assert_string_at_every_position(format, &scalar, &scalar);
        }
        // Scalar content inside comments and block strings is not tokenized or
        // reinterpreted by the integer guard.
        let text = "# 0x10000000000000000\nname: |-\n  0x10000000000000000\nextra: >-\n  0o2000000000000000000000\n";
        let document = read(format, text).unwrap();
        assert_eq!(document["name"], "0x10000000000000000");
        assert_eq!(document["extra"], "0o2000000000000000000000");
    }
}

#[test]
fn yaml_integer_explicit_numeric_tags_apply_even_to_quoted_scalars() {
    for format in YAML_FORMATS {
        for tag in ["!!int", "!<tag:yaml.org,2002:int>"] {
            for scalar in ["42", "18446744073709551615", "-9223372036854775808"] {
                for spelling in [format!("{tag} '{scalar}'"), format!("{tag} \"{scalar}\"")] {
                    for (pointer, key, text) in positions(&spelling) {
                        if matches!(pointer, "/name" | "/author" | "/description") || key {
                            assert!(read(format, &text).is_err());
                        } else {
                            assert!(
                                read(format, &text)
                                    .unwrap()
                                    .pointer(pointer)
                                    .unwrap()
                                    .is_number()
                            );
                        }
                    }
                }
            }
            for scalar in overflowing_integers() {
                for (pointer, key, text) in positions(&format!("{tag} '{scalar}'")) {
                    assert!(read(format, &text).is_err(), "{format} {pointer} key={key}");
                }
            }
        }
        // An explicit float tag selects a different supported numeric type;
        // integer-looking content must not accidentally invoke integer bounds.
        for scalar in ["18446744073709551616", "'18446744073709551616'"] {
            let document = read(format, &format!("name: x\nextra: !!float {scalar}")).unwrap();
            assert_eq!(
                document["extra"].as_f64(),
                Some(18_446_744_073_709_551_616.0)
            );
        }
    }
}

#[test]
fn yaml_integer_guard_uses_resolved_tag_uris_not_handle_spelling() {
    for format in YAML_FORMATS {
        let prefix = "%TAG !s! tag:yaml.org,2002:s\n%TAG !i! tag:yaml.org,2002:i\n---\n";
        let document = read(format, &format!("{prefix}name: !s!tr 0x10000000000000000")).unwrap();
        assert_eq!(document["name"], "0x10000000000000000");
        let document = read(format, &format!("{prefix}name: x\nextra: !i!nt '42'")).unwrap();
        assert_eq!(document["extra"], 42);
        assert!(
            read(
                format,
                &format!("{prefix}name: x\nextra: !i!nt '0x10000000000000000'")
            )
            .is_err()
        );
    }
}

#[test]
fn yaml_integer_aliases_retain_the_original_anchor_type_and_bounds() {
    for format in YAML_FORMATS {
        for scalar in ["0x10000000000000000", "0o2000000000000000000000"] {
            for text in [
                format!("extra: &number {scalar}\nname: *number"),
                format!("name: x\nnumber: &number {scalar}\nextra: [*number]"),
                format!("name: x\nnumber: &number {scalar}\nextra:\n  ? *number\n  : value"),
            ] {
                assert!(read(format, &text).is_err());
            }
            for spelling in [format!("'{scalar}'"), format!("!!str {scalar}")] {
                let document = read(format, &format!("extra: &text {spelling}\nname: *text\nauthor: *text\ndescription: *text\nother: [*text]")).unwrap();
                for field in ["name", "author", "description"] {
                    assert_eq!(document[field], scalar);
                }
                assert_eq!(document["other"][0], scalar);
                let document = read(
                    format,
                    &format!("name: x\ntext: &text {spelling}\nextra:\n  ? *text\n  : value"),
                )
                .unwrap();
                assert_eq!(document["extra"][scalar], "value");
            }
        }
        let document = read(
            format,
            "name: x\nnumber: &n 18446744073709551615\nextra: [*n]",
        )
        .unwrap();
        assert_eq!(document["extra"][0], json!(u64::MAX));
        assert!(read(format, "name: x\nextra: &cycle [*cycle]").is_err());
        let excessive_aliases = format!(
            "name: x\ntext: &text quoted\nextra: [{}]",
            vec!["*text"; 101].join(", ")
        );
        assert!(read(format, &excessive_aliases).is_err());
        assert!(read(format, "name: x\na: &a [0]\nb: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]\nc: [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]\nd: [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]").is_ok());
    }
}
