//! YAML overflow regressions use the public loader, including both filename choices.

use std::fs;

use hydragen::{
    LayerManifestError, LayerManifestFormat as Format, Metadata, MetadataError, discover,
    encode_metadata,
};
use serde_json::{Value, json};

const YAML_FORMATS: [Format; 2] = [Format::Yaml, Format::Yml];

// Exact normal/overflow pairs from the runtime characterization. Keep these
// independent of the range-check implementation and the maintained schema.
const INTEGER_SPELLINGS: [(&str, &str); 21] = [
    ("42", "18446744073709551616"),
    ("+42", "+18446744073709551616"),
    ("-42", "-9223372036854775809"),
    ("4_2", "18_446_744_073_709_551_616"),
    ("+4_2", "+18_446_744_073_709_551_616"),
    ("-4_2", "-9_223_372_036_854_775_809"),
    ("0x2a", "0x10000000000000000"),
    ("0X2A", "0X10000000000000000"),
    ("0x2_a", "0x1_0000_0000_0000_0000"),
    ("+0x2a", "+0x10000000000000000"),
    ("-0x2a", "-0x8000000000000001"),
    ("0o52", "0o2000000000000000000000"),
    ("0O52", "0O2000000000000000000000"),
    ("0o5_2", "0o2_000000000000000000000"),
    ("+0o52", "+0o2000000000000000000000"),
    ("-0o52", "-0o1000000000000000000001"),
    (
        "0b101010",
        "0b10000000000000000000000000000000000000000000000000000000000000000",
    ),
    (
        "0B101010",
        "0B10000000000000000000000000000000000000000000000000000000000000000",
    ),
    (
        "0b10_1010",
        "0b1_0000000000000000000000000000000000000000000000000000000000000000",
    ),
    (
        "+0b101010",
        "+0b10000000000000000000000000000000000000000000000000000000000000000",
    ),
    (
        "-0b101010",
        "-0b1000000000000000000000000000000000000000000000000000000000000001",
    ),
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
        // Explicit-key notation avoids YAML's 1,024-character simple-key limit,
        // so the large-number cases exercise numeric classification, not syntax.
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

fn overflowing_integers() -> Vec<String> {
    let mut values = vec![
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
    ];
    values.extend(INTEGER_SPELLINGS.map(|(_, overflow)| overflow.to_owned()));
    values
}

fn assert_number_at_every_position(format: Format, scalar: &str, expected: &Value) {
    for (pointer, key, text) in positions(scalar) {
        if matches!(pointer, "/name" | "/author" | "/description") {
            assert!(
                matches!(
                    read(format, &text),
                    Err(LayerManifestError::Metadata {
                        source: MetadataError::NotString { .. },
                        ..
                    })
                ),
                "{format} {pointer}: {text}"
            );
        } else if key {
            let error = read(format, &text).unwrap_err();
            assert!(
                matches!(error, LayerManifestError::Decode { .. }),
                "{error}"
            );
            assert!(error.to_string().contains("string object key"), "{error}");
        } else {
            assert_eq!(
                read(format, &text).unwrap().pointer(pointer),
                Some(expected),
                "{format} {pointer}: {text}"
            );
        }
    }
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
            assert_number_at_every_position(format, scalar, expected);
        }
    }
}

#[test]
fn yaml_integer_extensions_keep_in_range_types_and_explicit_numeric_tags() {
    for format in YAML_FORMATS {
        for (normal, _) in INTEGER_SPELLINGS {
            let expected = if normal.starts_with('-') {
                json!(-42)
            } else {
                json!(42)
            };
            for spelling in [
                normal.to_owned(),
                format!("!!int '{normal}'"),
                format!("!<tag:yaml.org,2002:int> \"{normal}\""),
            ] {
                assert_number_at_every_position(format, &spelling, &expected);
            }
        }
        for (scalar, expected) in [
            ("18_446_744_073_709_551_615", json!(u64::MAX)),
            ("+18_446_744_073_709_551_615", json!(u64::MAX)),
            ("-9_223_372_036_854_775_808", json!(i64::MIN)),
        ] {
            assert_number_at_every_position(format, scalar, &expected);
        }
    }
}

#[test]
fn yaml_integer_radix_boundaries_cover_sign_case_and_underscore_combinations() {
    for format in YAML_FORMATS {
        for prefix in ["0x", "0X", "0o", "0O", "0b", "0B"] {
            for sign in ["", "+", "-"] {
                let magnitude = if sign == "-" {
                    1u128 << 63
                } else {
                    u64::MAX.into()
                };
                let expected = if sign == "-" {
                    json!(i64::MIN)
                } else {
                    json!(u64::MAX)
                };
                let digits = |value| match prefix {
                    "0x" | "0X" => format!("{value:X}"),
                    "0o" | "0O" => format!("{value:o}"),
                    _ => format!("{value:b}"),
                };
                for underscores in [false, true] {
                    let spelling = |value| {
                        let raw = digits(value);
                        let digits = if underscores {
                            raw.chars()
                                .map(|digit| digit.to_string())
                                .collect::<Vec<_>>()
                                .join("_")
                        } else {
                            raw
                        };
                        format!("{sign}{prefix}{digits}")
                    };
                    assert_number_at_every_position(format, &spelling(magnitude), &expected);
                    let overflow = spelling(magnitude + 1);
                    for (pointer, key, text) in positions(&overflow) {
                        let error = read(format, &text).unwrap_err();
                        assert!(
                            matches!(error, LayerManifestError::Decode { .. }),
                            "{error}"
                        );
                        assert!(
                            error.to_string().contains("i64/u64 range"),
                            "{format} {pointer} key={key}: {error}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn yaml_integer_looking_writer_strings_round_trip_in_all_metadata_fields() {
    for format in YAML_FORMATS {
        for (normal, overflow) in INTEGER_SPELLINGS {
            for value in [normal, overflow] {
                let metadata = Metadata {
                    name: value.into(),
                    author: Some(value.into()),
                    description: Some(value.into()),
                };
                let bytes = encode_metadata(&metadata, format).unwrap();
                let output = String::from_utf8(bytes).unwrap();
                assert_eq!(
                    output,
                    format!("name: \"{value}\"\nauthor: \"{value}\"\ndescription: \"{value}\"\n")
                );
                let expected = serde_json::to_value(&metadata).unwrap();
                assert_eq!(read(format, &output).unwrap(), expected);
                assert_eq!(serde_saphyr::from_str::<Value>(&output).unwrap(), expected);
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
            format!("+0X{}1", "0_".repeat(2048)),
            format!("0O{}1", "0_".repeat(2048)),
            format!("+0B{}1", "0".repeat(4096)),
        ] {
            let document =
                read(format, &format!("name: x\nextra: &n {scalar}\ncopy: [*n]")).unwrap();
            assert_eq!(document["extra"], 1);
            assert_eq!(document["copy"][0], 1);
        }
        let scalar = format!("-0B{}1", "0_".repeat(2048));
        let document = read(format, &format!("name: x\nextra: &n {scalar}\ncopy: [*n]")).unwrap();
        assert_eq!(document["extra"], -1);
        assert_eq!(document["copy"][0], -1);
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
            "0b2",
            "0Xg",
            "0B",
            "0x_10000000000000000",
            "0b_10000000000000000000000000000000000000000000000000000000000000000",
            "18__446_744_073_709_551_616",
            "18_446_744_073_709_551_616_",
            "_18_446_744_073_709_551_616",
            "018_446_744_073_709_551_616",
            "+018_446_744_073_709_551_616",
            "-09_223_372_036_854_775_809",
        ] {
            assert_string_at_every_position(format, scalar, scalar);
        }
        for scalar in [
            format!("0x{}g", "f".repeat(4096)),
            format!("0o{}8", "7".repeat(4096)),
            format!("{}x", "9".repeat(4096)),
            format!("0B{}2", "1".repeat(4096)),
            format!("0X{}g", "F_".repeat(4096)),
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
        for scalar in [
            "0X10000000000000000",
            "18_446_744_073_709_551_616",
            "-0B1000000000000000000000000000000000000000000000000000000000000001",
        ] {
            let document = read(format, &format!("{prefix}name: !s!tr {scalar}")).unwrap();
            assert_eq!(document["name"], scalar);
            assert!(read(format, &format!("{prefix}name: x\nextra: !i!nt '{scalar}'")).is_err());
        }
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
        for (_, scalar) in INTEGER_SPELLINGS {
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
