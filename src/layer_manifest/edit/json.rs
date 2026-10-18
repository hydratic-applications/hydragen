//! Insertion-only JSON/JSONC dependency edits, using parser-owned byte ranges.

use jsonc_parser::{common::Ranged, tokens::Token, tokens::TokenAndRange};

use crate::layer_manifest::LayerManifestFormat;

pub(super) fn append(
    source: &str,
    references: &[&str],
    format: LayerManifestFormat,
) -> Result<String, String> {
    if references.is_empty() {
        return Ok(source.to_owned());
    }
    if !matches!(
        format,
        LayerManifestFormat::Json | LayerManifestFormat::Jsonc
    ) {
        return Err(preparation_error("expected JSON or JSONC format"));
    }

    // Reuse the decoder's additional checks (decoded duplicate keys, JSON string
    // escapes/control characters, whitespace and numbers), not a second policy.
    // This is syntax checking only; content/schema validity belongs to the caller.
    super::super::decode::document(source, format).map_err(preparation_error)?;
    let options = parse_options(format == LayerManifestFormat::Jsonc);
    let collect = jsonc_parser::CollectOptions {
        comments: jsonc_parser::CommentCollectionStrategy::AsTokens,
        tokens: true,
    };
    let parsed = jsonc_parser::parse_to_ast(source, &collect, &options)
        .map_err(|error| preparation_error(error.to_string()))?;
    let root = parsed
        .value
        .as_ref()
        .and_then(|value| value.as_object())
        .ok_or_else(|| preparation_error("the layer manifest root must be an object"))?;
    let tokens = parsed
        .tokens
        .as_deref()
        .ok_or_else(|| preparation_error("the parser did not supply structural tokens"))?;
    let literals = references
        .iter()
        .map(|reference| serde_json::to_string(reference).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(preparation_error)?;

    if let Some(property) = root.get("dependencies") {
        let array = property.value.as_array().ok_or_else(|| {
            preparation_error("the existing dependencies declaration must be an array")
        })?;
        insert_items(
            source,
            tokens,
            array.range,
            &array.elements.iter().map(Ranged::range).collect::<Vec<_>>(),
            &literals,
            false,
        )
    } else {
        insert_items(
            source,
            tokens,
            root.range,
            &root
                .properties
                .iter()
                .map(Ranged::range)
                .collect::<Vec<_>>(),
            &[format!("\"dependencies\": [{}]", literals.join(", "))],
            true,
        )
    }
}

fn preparation_error(reason: impl std::fmt::Display) -> String {
    format!("Cannot safely prepare the JSON/JSONC dependency edit: {reason}.")
}

fn parse_options(jsonc: bool) -> jsonc_parser::ParseOptions {
    jsonc_parser::ParseOptions {
        allow_comments: jsonc,
        allow_trailing_commas: jsonc,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    }
}

fn insert_items(
    source: &str,
    tokens: &[TokenAndRange<'_>],
    range: jsonc_parser::common::Range,
    items: &[jsonc_parser::common::Range],
    additions: &[String],
    object: bool,
) -> Result<String, String> {
    let open = tokens.iter().find(|token| {
        token.range.start == range.start
            && matches!(
                (&token.token, object),
                (Token::OpenBrace, true) | (Token::OpenBracket, false)
            )
    });
    let close = tokens.iter().find(|token| {
        token.range.end == range.end
            && matches!(
                (&token.token, object),
                (Token::CloseBrace, true) | (Token::CloseBracket, false)
            )
    });
    let (Some(open), Some(close)) = (open, close) else {
        return Err(preparation_error(
            "the parser did not identify the collection's opening and closing tokens",
        ));
    };
    let end = items.last().map_or(open.range.end, |item| item.end);
    let tail = tokens
        .iter()
        .filter(|token| token.range.start >= end && token.range.start < close.range.start)
        .collect::<Vec<_>>();
    if tail.iter().any(|token| {
        !matches!(
            token.token,
            Token::Comma | Token::CommentLine(_) | Token::CommentBlock(_)
        )
    }) {
        return Err(preparation_error(
            "unexpected structural tokens follow the last collection entry",
        ));
    }
    let comma = tail
        .iter()
        .find(|token| matches!(token.token, Token::Comma));

    // A trailing comma belongs to the old entry even on a later line. Start
    // after that token so its own inline comment group remains attached too;
    // everything between the entry and comma is already protected. Multiline
    // block comments stay in the group, as do comments on their closing line.
    // Only a line break between tokens starts a separate closing comment.
    let attachment_start = comma.map_or(end, |token| token.range.end);
    let mut attached_end = attachment_start;
    if !items.is_empty() {
        for token in tail
            .iter()
            .filter(|token| token.range.start >= attachment_start)
        {
            if has_line_break(&source[attached_end..token.range.start]) {
                break;
            }
            attached_end = token.range.end;
        }
    }
    let minimum = attached_end;
    let next = tail
        .iter()
        .find(|token| token.range.start >= minimum)
        .map_or(close.range.start, |token| token.range.start);
    let next_line = line_start(source, next);
    let own_line = next_line >= minimum
        && source[next_line..next]
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t'));
    let multiline = has_line_break(&source[open.range.end..close.range.start]);
    let newline = line_ending(source);
    let base_indent = leading_indent(source, open.range.start);
    let indent = items
        .iter()
        .find_map(|item| {
            let start = line_start(source, item.start);
            let prefix = &source[start..item.start];
            prefix
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t'))
                .then_some(prefix.to_owned())
        })
        .unwrap_or_else(|| format!("{base_indent}  "));
    let trailing = comma.is_some();
    let mut edits = Vec::new();
    if !items.is_empty() && !trailing {
        edits.push((end, ",".to_owned()));
    }
    let (position, text) = if multiline && own_line {
        let mut text = String::new();
        for (index, addition) in additions.iter().enumerate() {
            text.push_str(&indent);
            text.push_str(addition);
            if trailing || index + 1 < additions.len() {
                text.push(',');
            }
            text.push_str(newline);
        }
        (next_line, text)
    } else if multiline {
        let joined = additions.join(&format!(",{newline}{indent}"));
        // The next token shares a line with existing source (for example an
        // inline closing bracket). Give inserted text its own lines rather than
        // risking a line comment swallowing it or a closing comment attaching to it.
        (
            minimum,
            format!(
                "{newline}{indent}{joined}{}{newline}{base_indent}",
                if trailing { "," } else { "" }
            ),
        )
    } else {
        (
            minimum,
            format!(
                "{}{}{}",
                if items.is_empty() { "" } else { " " },
                additions.join(", "),
                if trailing { "," } else { "" }
            ),
        )
    };
    edits.push((position, text));

    // Stable sorting keeps a newly needed separator before new entries when
    // both insertions have the same byte offset. Never replace original bytes.
    edits.sort_by_key(|(position, _)| *position);
    let mut result = String::new();
    let mut previous = 0;
    for (position, text) in edits {
        let untouched = source.get(previous..position).ok_or_else(|| {
            preparation_error("the parser supplied an invalid UTF-8 insertion range")
        })?;
        result.push_str(untouched);
        result.push_str(&text);
        previous = position;
    }
    result.push_str(source.get(previous..).ok_or_else(|| {
        preparation_error("the parser supplied an invalid final insertion range")
    })?);
    Ok(result)
}

fn has_line_break(text: &str) -> bool {
    text.contains(['\r', '\n'])
}

fn line_start(source: &str, position: usize) -> usize {
    source[..position]
        .rfind(['\r', '\n'])
        .map_or(0, |index| index + 1)
}

fn leading_indent(source: &str, position: usize) -> &str {
    let start = line_start(source, position);
    let line = &source[start..position];
    let length = line
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    &line[..length]
}

fn line_ending(source: &str) -> &str {
    match source.find(['\r', '\n']) {
        Some(index) if source[index..].starts_with("\r\n") => "\r\n",
        Some(index) if source.as_bytes()[index] == b'\r' => "\r",
        _ => "\n",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer_manifest::decode;

    fn check(source: &str, references: &[&str], format: LayerManifestFormat, expected: &str) {
        let candidate = append(source, references, format).unwrap();
        // These exact source assertions are separate from the decoded proof:
        // numbers, escaped literals, whitespace and comment placement all matter.
        assert_eq!(candidate, expected);
        let mut intended = decode::document(source, format).unwrap();
        let dependencies = intended
            .as_object_mut()
            .unwrap()
            .entry("dependencies")
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .unwrap();
        dependencies.extend(
            references
                .iter()
                .map(|reference| serde_json::json!(reference)),
        );
        assert_eq!(decode::document(&candidate, format).unwrap(), intended);
    }

    #[test]
    fn json_absent_dependencies_preserve_property_bytes_and_order() {
        check(
            r#"{"name":"app","custom":{"n":1e+2,"text":"[,]}"}}"#,
            &["../one", "../two"],
            LayerManifestFormat::Json,
            r#"{"name":"app","custom":{"n":1e+2,"text":"[,]}"}, "dependencies": ["../one", "../two"]}"#,
        );
        check(
            "{\n\t\"name\" : \"app\"\n}\n",
            &["../one"],
            LayerManifestFormat::Json,
            "{\n\t\"name\" : \"app\",\n\t\"dependencies\": [\"../one\"]\n}\n",
        );
    }

    #[test]
    fn json_empty_root_and_empty_arrays() {
        for (source, expected) in [
            ("{}", "{\"dependencies\": [\"../new\"]}"),
            ("{ }", "{\"dependencies\": [\"../new\"] }"),
            ("{\"dependencies\":[]}", "{\"dependencies\":[\"../new\"]}"),
            (
                "{\"dependencies\":[\n  ]}",
                "{\"dependencies\":[\n  \"../new\"\n  ]}",
            ),
        ] {
            check(source, &["../new"], LayerManifestFormat::Json, expected);
        }
    }

    #[test]
    fn json_inline_and_multiline_arrays_keep_original_representations() {
        check(
            r#"{"dependencies": ["..\/old", "../two"],"custom":-0}"#,
            &["../three", "../four"],
            LayerManifestFormat::Json,
            r#"{"dependencies": ["..\/old", "../two", "../three", "../four"],"custom":-0}"#,
        );
        check(
            "{\n  \"dependencies\": [\n    \"../old\"\n  ]\n}",
            &["../new", "../last"],
            LayerManifestFormat::Json,
            "{\n  \"dependencies\": [\n    \"../old\",\n    \"../new\",\n    \"../last\"\n  ]\n}",
        );
    }

    #[test]
    fn jsonc_inline_comments_stay_with_existing_dependencies() {
        check(
            "{\"dependencies\": [\n  \"../old\" // old entry, not ], or }\n  // closing comment\n]}",
            &["../new"],
            LayerManifestFormat::Jsonc,
            "{\"dependencies\": [\n  \"../old\", // old entry, not ], or }\n  \"../new\"\n  // closing comment\n]}",
        );
        check(
            "{\"dependencies\": [\n  \"../old\", /* inline */ // old\n  /* closing ] , */\n]}",
            &["../new", "../last"],
            LayerManifestFormat::Jsonc,
            "{\"dependencies\": [\n  \"../old\", /* inline */ // old\n  \"../new\",\n  \"../last\",\n  /* closing ] , */\n]}",
        );
        check(
            r#"{"dependencies":["../old" /* old */, /* ] closing */ ]}"#,
            &["../new"],
            LayerManifestFormat::Jsonc,
            r#"{"dependencies":["../old" /* old */, /* ] closing */ "../new", ]}"#,
        );
    }

    #[test]
    fn jsonc_absent_property_keeps_final_property_and_closing_comments() {
        check(
            "// header\n{\n  \"name\": \"app\" // name\n  // closing } ,\n}\n// footer",
            &["../new"],
            LayerManifestFormat::Jsonc,
            "// header\n{\n  \"name\": \"app\", // name\n  \"dependencies\": [\"../new\"]\n  // closing } ,\n}\n// footer",
        );
        check(
            "{\n  \"custom\": {\"text\": \"},]\"}, // custom\n  /* root closing */\n}",
            &["../new"],
            LayerManifestFormat::Jsonc,
            "{\n  \"custom\": {\"text\": \"},]\"}, // custom\n  \"dependencies\": [\"../new\"],\n  /* root closing */\n}",
        );
    }

    #[test]
    fn jsonc_newline_before_comma_keeps_old_dependency_inline_comment() {
        check(
            "{\n \"name\":\"app\",\n \"dependencies\":[\n \"../old\"\n , // Required by old, not new\n ]\n}\n",
            &["../new"],
            LayerManifestFormat::Jsonc,
            "{\n \"name\":\"app\",\n \"dependencies\":[\n \"../old\"\n , // Required by old, not new\n \"../new\",\n ]\n}\n",
        );
    }

    #[test]
    fn jsonc_newline_before_comma_keeps_old_property_inline_comment() {
        check(
            "{\n \"name\":\"app\"\n , // About name, not dependencies\n // Root footer\n}\n",
            &["../new", "../last"],
            LayerManifestFormat::Jsonc,
            "{\n \"name\":\"app\"\n , // About name, not dependencies\n \"dependencies\": [\"../new\", \"../last\"],\n // Root footer\n}\n",
        );
    }

    #[test]
    fn jsonc_newline_before_comma_preserves_inline_groups_and_separate_footers() {
        let groups = [
            "// Required by old, not new",
            "/* Required by old, not new */",
            "/* Old first */ /* Old second */ // Still old",
            "/* Required by old\n    old continuation 🦀 */",
            "/* Required by old\n    old continuation */ /* Still old */ // Last old",
            "\t/* Old first */\t/* Old second\n    old continuation */\t// Still old",
        ];
        for newline in ["\n", "\r\n"] {
            for final_newline in ["", "\n"] {
                for group in groups {
                    for references in [&["../new"][..], &["../new", "../last"][..]] {
                        let literals = references
                            .iter()
                            .map(|reference| format!("\"{reference}\""))
                            .collect::<Vec<_>>();
                        let array_prefix = "{\n \"name\":\"app\",\n \"é🦀\":{\"text\":\"値\\u0041\",\"n\":1e+2},\n \"depend\\u0065ncies\":[\n \"..\\/old\" /* Old entry */ // Old entry line\n";
                        let root_prefix = "{\n \"é🦀\":{\"text\":\"値\\u0041\",\"n\":1e+2},\n \"name\":\"app\" /* Old field */ // Old field line\n";
                        for (prefix, additions, suffix) in [
                            (
                                array_prefix,
                                format!(" {},\n", literals.join(",\n ")),
                                format!(
                                    " // Array footer\n /* Array footer block */\n ]\n}}{final_newline}"
                                ),
                            ),
                            (
                                root_prefix,
                                format!(" \"dependencies\": [{}],\n", literals.join(", ")),
                                format!(
                                    " // Root footer\n /* Root footer block */\n}}{final_newline}"
                                ),
                            ),
                        ] {
                            let original_group = format!(" , {group}\n");
                            let source =
                                format!("{prefix}{original_group}{suffix}").replace('\n', newline);
                            let expected = format!("{prefix}{original_group}{additions}{suffix}")
                                .replace('\n', newline);
                            let candidate =
                                append(&source, references, LayerManifestFormat::Jsonc).unwrap();
                            // The comma and its entire inline group must remain
                            // contiguous before additions, not merely survive as
                            // comment tokens now describing the new declaration.
                            assert!(candidate.starts_with(
                                &format!("{prefix}{original_group}").replace('\n', newline)
                            ));
                            assert!(candidate.ends_with(&suffix.replace('\n', newline)));
                            check(&source, references, LayerManifestFormat::Jsonc, &expected);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn jsonc_empty_commented_collections_are_supported() {
        check(
            "{\n  // root closing\n}",
            &["../new"],
            LayerManifestFormat::Jsonc,
            "{\n  \"dependencies\": [\"../new\"]\n  // root closing\n}",
        );
        check(
            "{\"dependencies\": [\n  /* empty array */\n]}",
            &["../new"],
            LayerManifestFormat::Jsonc,
            "{\"dependencies\": [\n  \"../new\"\n  /* empty array */\n]}",
        );
        check(
            r#"{"dependencies":[/* empty */]}"#,
            &["../new"],
            LayerManifestFormat::Jsonc,
            r#"{"dependencies":["../new"/* empty */]}"#,
        );
    }

    #[test]
    fn jsonc_crlf_and_unicode_ranges_preserve_every_existing_byte() {
        check(
            "{\r\n  \"é🦀\": \"値\\u0041\",\r\n  \"depend\\u0065ncies\": [\r\n    \"../é\", // 🦀\r\n  ]\r\n}",
            &["../新", "../quote\"slash\\line\n"],
            LayerManifestFormat::Jsonc,
            "{\r\n  \"é🦀\": \"値\\u0041\",\r\n  \"depend\\u0065ncies\": [\r\n    \"../é\", // 🦀\r\n    \"../新\",\r\n    \"../quote\\\"slash\\\\line\\n\",\r\n  ]\r\n}",
        );
    }

    #[test]
    fn jsonc_unusual_but_valid_separator_and_closing_layouts() {
        for source in [
            "{\"dependencies\":[\"../old\" // old\n,]}",
            "{\"dependencies\":[\"../old\" /* old\ncontinued */]}",
            "{\"dependencies\":[\n  \"../old\"]}",
            "{\"dependencies\":[\"../old\"\n/* closing */ , ]}",
            "{\"name\":\"app\" // name\n}",
            "{\"name\":\"app\"\n/* closing */ ,}",
            "{\"dependencies\":[//empty\n]}",
        ] {
            let candidate = append(source, &["../new"], LayerManifestFormat::Jsonc).unwrap();
            let mut original_bytes = source.bytes();
            let mut next = original_bytes.next();
            for byte in candidate.bytes() {
                if next == Some(byte) {
                    next = original_bytes.next();
                }
            }
            assert_eq!(
                next, None,
                "source bytes were removed or reordered: {source}"
            );
            let mut expected = decode::document(source, LayerManifestFormat::Jsonc).unwrap();
            let dependencies = expected
                .as_object_mut()
                .unwrap()
                .entry("dependencies")
                .or_insert_with(|| serde_json::json!([]))
                .as_array_mut()
                .unwrap();
            dependencies.push(serde_json::json!("../new"));
            assert_eq!(
                decode::document(&candidate, LayerManifestFormat::Jsonc).unwrap(),
                expected,
                "candidate: {candidate}"
            );
        }
    }

    #[test]
    fn jsonc_trivia_matrix_keeps_bytes_comments_and_intended_data() {
        let endings = ["\n", "\r\n"];
        let openings = ["", " ", "\n", "\n  // leading [,]\n", "/* 🦀 [,] */"];
        let tails = [
            "",
            " ",
            ",",
            ", ",
            " /* inline [,] */",
            " /* inline */ , /* still inline */",
            " // inline [,]\n",
            ", // inline [,]\n",
            " // inline\n,",
            "\n  // closing [,]\n",
            ",\n  // closing [,]\n",
            "\n  /* closing [,] */",
            "\n  /* closing */ ,\n",
            " /* inline\ncontinued [,] */ // inline after block\n",
        ];
        for newline in endings {
            for opening in openings {
                for tail in tails {
                    for final_newline in ["", "\n"] {
                        // The same trivia exercises an array tail and the last
                        // property of a root object lacking dependencies.
                        for (source, original_dependencies) in [
                            (
                                format!(
                                    "/* é */ {{\"custom\": 1e+2, \"dependencies\":[{opening}\"..\\/old\"{tail}]}}{final_newline}"
                                ),
                                vec!["../old"],
                            ),
                            (
                                format!(
                                    "/* é */ {{{opening}\"custom\": {{\"n\":1e+2}}{tail}}}{final_newline}"
                                ),
                                vec![],
                            ),
                        ] {
                            let source = source.replace('\n', newline);
                            let candidate =
                                append(&source, &["../new", "../last"], LayerManifestFormat::Jsonc)
                                    .unwrap();
                            let mut original_bytes = source.bytes();
                            let mut next = original_bytes.next();
                            for byte in candidate.bytes() {
                                if next == Some(byte) {
                                    next = original_bytes.next();
                                }
                            }
                            assert_eq!(next, None, "removed/reordered source: {source}");
                            let collect = jsonc_parser::CollectOptions {
                                comments: jsonc_parser::CommentCollectionStrategy::AsTokens,
                                tokens: true,
                            };
                            let comments = |text: &str| {
                                jsonc_parser::parse_to_ast(text, &collect, &parse_options(true))
                                    .unwrap()
                                    .tokens
                                    .unwrap()
                                    .into_iter()
                                    .filter(|token| {
                                        matches!(
                                            token.token,
                                            Token::CommentLine(_) | Token::CommentBlock(_)
                                        )
                                    })
                                    .map(|token| {
                                        text[token.range.start..token.range.end].to_owned()
                                    })
                                    .collect::<Vec<_>>()
                            };
                            assert_eq!(comments(&source), comments(&candidate));
                            let mut expected =
                                decode::document(&source, LayerManifestFormat::Jsonc).unwrap();
                            let mut dependencies = original_dependencies;
                            dependencies.extend(["../new", "../last"]);
                            expected["dependencies"] = serde_json::json!(dependencies);
                            assert_eq!(
                                decode::document(&candidate, LayerManifestFormat::Jsonc).unwrap(),
                                expected,
                                "source: {source}\ncandidate: {candidate}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn json_edit_rejects_malformed_or_extended_syntax_without_panicking() {
        for source in [
            "",
            "// nothing",
            "[]",
            "null",
            "{} {}",
            "{\"dependencies\":null}",
            "{\"dependencies\":[}",
            "{\"dependencies\":[],\"depend\\u0065ncies\":[]}",
            "{\"custom\":{\"é\":1,\"é\":2}}",
            "{unquoted:1}",
            "{'single':1}",
            "{\"number\":0xff}",
            "{\"number\":+1}",
            "{\"number\":.5}",
            "{\"number\":1.}",
            "{\"number\":NaN}",
            "{\"custom\":\"\\x41\"}",
            "{\"custom\":\"raw\tcontrol\"}",
            "{\"custom\":1,\u{00a0}\"other\":2}",
            "{\"custom\":1 \"other\":2}",
        ] {
            for format in [LayerManifestFormat::Json, LayerManifestFormat::Jsonc] {
                let error = append(source, &["../new"], format).unwrap_err();
                assert!(error.contains("Cannot safely prepare"), "{error}");
            }
        }
        for source in ["{/* comment */}", "{\"name\":\"app\",}"] {
            assert!(append(source, &["../new"], LayerManifestFormat::Json).is_err());
            assert!(append(source, &["../new"], LayerManifestFormat::Jsonc).is_ok());
        }
        assert!(append("{}", &["../new"], LayerManifestFormat::Yaml).is_err());
    }

    #[test]
    fn json_empty_append_is_an_exact_noop_even_for_unparsed_source() {
        for source in ["", "not JSON", "{}\r\n", "{\"dependencies\":[]}"] {
            assert_eq!(
                append(source, &[], LayerManifestFormat::Json).unwrap(),
                source
            );
        }
    }
}
