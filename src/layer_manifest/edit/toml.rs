use std::fmt::Write as _;

use toml_edit::{Array, Document, Table};

/// Insert new dependency literals without rendering any existing TOML nodes.
/// The caller validates content and proves the candidate's intended decoded change.
pub(super) fn append(source: &str, references: &[&str]) -> Result<String, String> {
    if references.is_empty() {
        return Ok(source.to_owned());
    }

    // Keep the immutable document: converting into DocumentMut discards spans.
    let document = Document::parse(source)
        .map_err(|error| format!("Could not locate TOML layer manifest edits: {error}"))?;
    let literals: Vec<_> = references
        .iter()
        .map(|reference| quote(reference))
        .collect();

    match document.get("dependencies") {
        Some(item) => {
            let array = item.as_array().ok_or_else(|| {
                "Cannot append to the TOML layer manifest: dependencies is not an array.".to_owned()
            })?;
            append_array(source, array, &literals)
        }
        None => insert_root(source, document.as_table(), &literals),
    }
}

fn append_array(source: &str, array: &Array, literals: &[String]) -> Result<String, String> {
    let span = array.span().ok_or_else(missing_span)?;
    let text = source.get(span.clone()).ok_or_else(missing_span)?;
    if !text.starts_with('[') || !text.ends_with(']') {
        return Err(missing_span());
    }
    let close = span.end - 1;
    let last = array.iter().last();
    let last_span = last
        .map(|value| value.span().ok_or_else(missing_span))
        .transpose()?;
    let mut output = source.to_owned();

    if text.contains('\n') {
        // Insert after existing trailing comments, but before the closing line's
        // indentation. Adding a missing separator at the old value's end keeps
        // that value's inline comment on the same line and in the same order.
        let closing_line = line_start(source, close);
        let closing_indent = &source[closing_line..close];
        let close_on_own_line = horizontal_space(closing_indent);
        let opening_indent = indentation(source, span.start).unwrap_or("");
        let indent = last_span
            .as_ref()
            .and_then(|span| indentation(source, span.start))
            .map(str::to_owned)
            .unwrap_or_else(|| {
                format!(
                    "{}  ",
                    if close_on_own_line {
                        closing_indent
                    } else {
                        opening_indent
                    }
                )
            });
        let newline = newline(source);
        let mut insertion = String::new();
        let position = if close_on_own_line {
            closing_line
        } else {
            insertion.push_str(newline);
            close
        };
        for (index, literal) in literals.iter().enumerate() {
            insertion.push_str(&indent);
            insertion.push_str(literal);
            if array.trailing_comma() || index + 1 < literals.len() {
                insertion.push(',');
            }
            insertion.push_str(newline);
        }
        if !close_on_own_line {
            insertion.push_str(opening_indent);
        }
        output.insert_str(position, &insertion);
        if let Some(last_span) = last_span
            && !array.trailing_comma()
        {
            output.insert(last_span.end, ',');
        }
    } else if let Some(last_span) = last_span {
        if array.trailing_comma() {
            // This parser-bounded suffix contains only whitespace and the final
            // separator (there are no comments in a valid single-line array).
            let suffix = source.get(last_span.end..close).ok_or_else(missing_span)?;
            let comma = suffix.find(',').ok_or_else(missing_span)?;
            output.insert_str(
                last_span.end + comma + 1,
                &format!(" {},", literals.join(", ")),
            );
        } else {
            output.insert_str(last_span.end, &format!(", {}", literals.join(", ")));
        }
    } else {
        output.insert_str(span.start + 1, &literals.join(", "));
    }

    Ok(output)
}

fn insert_root(source: &str, root: &Table, literals: &[String]) -> Result<String, String> {
    // get_values includes dotted root assignments, but not values in explicit
    // [tables] or [[arrays of tables]]. Taking the greatest source end also avoids
    // relying on the logical map order of interleaved dotted assignments.
    let last_value_end =
        root.get_values()
            .into_iter()
            .try_fold(None, |latest: Option<usize>, (_, value)| {
                let end = value.span().ok_or_else(missing_span)?.end;
                Ok::<_, String>(Some(latest.map_or(end, |latest| latest.max(end))))
            })?;
    let position = match last_value_end {
        Some(end) => source[end..]
            .find('\n')
            .map_or(source.len(), |offset| end + offset + 1),
        // Also handle empty/comment-only documents and tables without root
        // assignments, without moving their leading comments below the new key.
        None => source
            .split_inclusive('\n')
            .take_while(|line| {
                let line = line.trim_start_matches([' ', '\t', '\r', '\n']);
                line.is_empty() || line.starts_with('#')
            })
            .map(str::len)
            .sum(),
    };
    let newline = newline(source);
    let mut insertion = String::new();
    if position > 0 && !source[..position].ends_with('\n') {
        insertion.push_str(newline);
    }
    write!(insertion, "dependencies = [{}]", literals.join(", ")).unwrap();
    // Preserve whether EOF has a newline; a declaration inserted before other
    // text always needs a line break, even when EOF itself has none.
    if position < source.len() || source.ends_with('\n') {
        insertion.push_str(newline);
    }
    let mut output = source.to_owned();
    output.insert_str(position, &insertion);
    Ok(output)
}

/// Encode only new values as single-line TOML basic strings, so references with
/// newlines cannot introduce a different physical line ending into the document.
fn quote(value: &str) -> String {
    let mut literal = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\u{0008}' => literal.push_str("\\b"),
            '\t' => literal.push_str("\\t"),
            '\n' => literal.push_str("\\n"),
            '\u{000c}' => literal.push_str("\\f"),
            '\r' => literal.push_str("\\r"),
            '\u{0000}'..='\u{001f}' | '\u{007f}' => {
                write!(literal, "\\u{:04X}", character as u32).unwrap();
            }
            _ => literal.push(character),
        }
    }
    literal.push('"');
    literal
}

fn line_start(source: &str, position: usize) -> usize {
    source[..position].rfind('\n').map_or(0, |index| index + 1)
}

fn horizontal_space(text: &str) -> bool {
    text.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn indentation(source: &str, position: usize) -> Option<&str> {
    let prefix = &source[line_start(source, position)..position];
    horizontal_space(prefix).then_some(prefix)
}

fn newline(source: &str) -> &str {
    match source.find('\n') {
        Some(index) if index > 0 && source.as_bytes()[index - 1] == b'\r' => "\r\n",
        _ => "\n",
    }
}

fn missing_span() -> String {
    "Cannot safely locate the dependency insertion in the TOML layer manifest.".to_owned()
}

#[cfg(test)]
mod tests {
    use super::append;

    fn check(source: &str, references: &[&str], expected: &str) {
        let output = append(source, references).unwrap();
        // Source preservation is a separate assertion, not inferred from parsing.
        assert_eq!(output, expected);
        let mut intended: ::toml::Value = ::toml::from_str(source).unwrap();
        let root = intended.as_table_mut().unwrap();
        let dependencies = root
            .entry("dependencies")
            .or_insert_with(|| ::toml::Value::Array(Vec::new()))
            .as_array_mut()
            .unwrap();
        dependencies.extend(
            references
                .iter()
                .map(|value| ::toml::Value::String((*value).to_owned())),
        );
        let actual: ::toml::Value = ::toml::from_str(&output).unwrap();
        assert_eq!(actual, intended);
        assert_eq!(output.ends_with('\n'), source.ends_with('\n'));
    }

    #[test]
    fn empty_references_are_byte_identical_even_without_parsing() {
        for source in ["", "# café\r\nname = 'app'", "not TOML at all"] {
            assert_eq!(append(source, &[]).unwrap(), source);
        }
    }

    #[test]
    fn absent_dependencies_preserve_top_metadata_and_trailing_comments() {
        check(
            "# top\nname = 'app' # name\nauthor=\"Zoë 🦀\" # author\n\n# footer\n",
            &["../one", "../two"],
            "# top\nname = 'app' # name\nauthor=\"Zoë 🦀\" # author\ndependencies = [\"../one\", \"../two\"]\n\n# footer\n",
        );
    }

    #[test]
    fn absent_dependencies_stay_at_root_before_custom_tables() {
        let suffix = "\n# custom table\n[custom]\ndependencies = ['not-the-root-list']\n[[custom.entries]]\ntext = '[dependencies]'";
        check(
            &format!("name='app' # keep\n{suffix}"),
            &["../one"],
            &format!("name='app' # keep\ndependencies = [\"../one\"]\n{suffix}"),
        );
    }

    #[test]
    fn absent_dependencies_do_not_reorder_interleaved_dotted_keys() {
        check(
            "name = 'app'\ncustom.z=1\nother.x=2\ncustom.a=3 # last root assignment\n\n[custom.child]\nx=4\n",
            &["../new"],
            "name = 'app'\ncustom.z=1\nother.x=2\ncustom.a=3 # last root assignment\ndependencies = [\"../new\"]\n\n[custom.child]\nx=4\n",
        );
    }

    #[test]
    fn absent_dependencies_preserve_crlf_and_missing_final_newline() {
        for ending in ["\n", "\r\n"] {
            for final_newline in ["", ending] {
                let source = format!("# Unicode 日本語{ending}name = 'app' # final{final_newline}");
                let expected = format!(
                    "# Unicode 日本語{ending}name = 'app' # final{ending}dependencies = [\"../new\"]{final_newline}"
                );
                check(&source, &["../new"], &expected);
            }
        }
    }

    #[test]
    fn ordinary_inline_arrays_keep_old_spelling_and_closing_trivia() {
        for (array, expected) in [
            ("[]", "[\"../new\", \"../next\"]"),
            ("[  ]", "[\"../new\", \"../next\"  ]"),
            ("['../old']", "['../old', \"../new\", \"../next\"]"),
            ("[ '../old'  ]", "[ '../old', \"../new\", \"../next\"  ]"),
            ("['../old',]", "['../old', \"../new\", \"../next\",]"),
            ("['../old' ,  ]", "['../old' , \"../new\", \"../next\",  ]"),
            (
                "[\"../\\u006Fld\"]",
                "[\"../\\u006Fld\", \"../new\", \"../next\"]",
            ),
        ] {
            check(
                &format!("name='app'\ndependencies = {array} # original end\n[custom]\nx=1"),
                &["../new", "../next"],
                &format!("name='app'\ndependencies = {expected} # original end\n[custom]\nx=1"),
            );
        }
    }

    #[test]
    fn multiline_append_preserves_inline_and_standalone_comments() {
        check(
            "# 日本語\nname='app'\n'dependencies' = [ # start\n    '../old', # old one\n    # between\n    \"../\\u006Fld-two\" # old two\n    # trailing standalone comma , and bracket ]\n] # closing\n\n[custom]\nvalue = 'kept'\n",
            &["../new", "../next"],
            "# 日本語\nname='app'\n'dependencies' = [ # start\n    '../old', # old one\n    # between\n    \"../\\u006Fld-two\", # old two\n    # trailing standalone comma , and bracket ]\n    \"../new\",\n    \"../next\"\n] # closing\n\n[custom]\nvalue = 'kept'\n",
        );
    }

    #[test]
    fn multiline_trailing_comma_and_indented_close_are_preserved() {
        check(
            "name='app'\ndependencies = [\n\t'../old' , # attached\n\t# final note\n  ]",
            &["../new"],
            "name='app'\ndependencies = [\n\t'../old' , # attached\n\t# final note\n\t\"../new\",\n  ]",
        );
    }

    #[test]
    fn empty_multiline_arrays_keep_all_comments_and_line_endings() {
        for ending in ["\n", "\r\n"] {
            check(
                &format!(
                    "name='app'{ending}dependencies = [ # empty{ending}  # no entries yet{ending}] # done"
                ),
                &["../new"],
                &format!(
                    "name='app'{ending}dependencies = [ # empty{ending}  # no entries yet{ending}  \"../new\"{ending}] # done"
                ),
            );
        }
    }

    #[test]
    fn multiline_arrays_with_values_on_the_closing_line_remain_supported() {
        check(
            "name='app'\ndependencies = [ '../one',\n  '../two' ] # end",
            &["../new"],
            "name='app'\ndependencies = [ '../one',\n  '../two', \n  \"../new\"\n] # end",
        );
    }

    #[test]
    fn crlf_dependency_comments_and_no_final_newline_survive() {
        check(
            "name='app'\r\nauthor='Élodie'\r\ndependencies = [\r\n  '../old' # retain\r\n] # closing",
            &["../new"],
            "name='app'\r\nauthor='Élodie'\r\ndependencies = [\r\n  '../old', # retain\r\n  \"../new\"\r\n] # closing",
        );
    }

    #[test]
    fn existing_array_does_not_move_dotted_keys_or_nested_dependency_names() {
        check(
            "name='app'\ncustom.z=1\ndependencies=['../old'] # exact\nother.y=2\ncustom.a=3\n[other.child]\ndependencies=['nested']",
            &["../new"],
            "name='app'\ncustom.z=1\ndependencies=['../old', \"../new\"] # exact\nother.y=2\ncustom.a=3\n[other.child]\ndependencies=['nested']",
        );
    }

    #[test]
    fn encodes_only_new_strings_including_all_basic_string_escapes() {
        check(
            "# 🦀\r\nname='app'\r\ndependencies=['../unchanged']",
            &["../quote\"/back\\slash\t\n\r\u{8}\u{c}\u{0}\u{1f}\u{7f}/日本語"],
            "# 🦀\r\nname='app'\r\ndependencies=['../unchanged', \"../quote\\\"/back\\\\slash\\t\\n\\r\\b\\f\\u0000\\u001F\\u007F/日本語\"]",
        );
    }

    #[test]
    fn existing_multiline_string_literals_are_not_reencoded() {
        check(
            "name='app'\ndependencies = [\"\"\"../old\npath\"\"\"]",
            &["../new"],
            "name='app'\ndependencies = [\"\"\"../old\npath\"\"\",\n  \"../new\"\n]",
        );
    }

    #[test]
    fn escaped_dependency_keys_and_delimiters_inside_old_strings_are_untouched() {
        check(
            "name='app'\n\"dependen\\u0063ies\" = ['../old,]#path', # comma , in a comment\n  # trailing ]\n]\n[custom]\nquoted='unchanged'",
            &["../new"],
            "name='app'\n\"dependen\\u0063ies\" = ['../old,]#path', # comma , in a comment\n  # trailing ]\n  \"../new\",\n]\n[custom]\nquoted='unchanged'",
        );
    }

    #[test]
    fn root_insert_uses_the_end_of_multiline_values_not_embedded_header_text() {
        check(
            "# leading\nname='app'\ndescription=\"\"\"Notes:\n[not-a-table]\n日本語\"\"\" # retained\n\n[custom.nested]\nx=1",
            &["../new"],
            "# leading\nname='app'\ndescription=\"\"\"Notes:\n[not-a-table]\n日本語\"\"\" # retained\ndependencies = [\"../new\"]\n\n[custom.nested]\nx=1",
        );
    }

    #[test]
    fn inline_and_multiline_layouts_preserve_each_line_ending_and_eof_style() {
        let layouts = [
            "[]",
            "[  ]",
            "['../old',  ]",
            "[\n  '../old' # attached\n  # closing\n]",
            "[ '../old', # attached\n]",
            "[\n  '../old', # attached\n  ]",
            "[ # empty\n  # comment\n]",
            "[\n  '../old' ]",
        ];
        for ending in ["\n", "\r\n"] {
            for final_newline in ["", ending] {
                for layout in layouts {
                    let source = format!(
                        "# é🦀{ending}name='app'{ending}dependencies = {} # end{ending}\n[custom]{ending}x=1{final_newline}",
                        layout.replace('\n', ending),
                    );
                    let output = append(&source, &["../new"]).unwrap();
                    let actual: ::toml::Value = ::toml::from_str(&output).unwrap();
                    let mut intended: ::toml::Value = ::toml::from_str(&source).unwrap();
                    intended["dependencies"]
                        .as_array_mut()
                        .unwrap()
                        .push(::toml::Value::String("../new".to_owned()));
                    assert_eq!(actual, intended);
                    assert_eq!(output.ends_with('\n'), source.ends_with('\n'));
                    assert!(output.starts_with(&format!("# é🦀{ending}name='app'{ending}")));
                    assert!(output.ends_with(&format!(
                        " # end{ending}\n[custom]{ending}x=1{final_newline}"
                    )));
                    if ending == "\r\n" {
                        // The deliberately mixed blank line is unrelated source;
                        // remove it before checking that every inserted LF is CRLF.
                        assert!(
                            !output
                                .replace("\r\n\n[custom]", "\r\n[custom]")
                                .replace("\r\n", "")
                                .contains('\n')
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn root_insertion_also_handles_empty_sources_and_header_only_tables() {
        for (source, expected) in [
            ("", "dependencies = [\"../new\"]"),
            ("# top", "# top\ndependencies = [\"../new\"]"),
            (
                "# top\n[custom]",
                "# top\ndependencies = [\"../new\"]\n[custom]",
            ),
        ] {
            check(source, &["../new"], expected);
        }
    }

    #[test]
    fn parser_failures_and_non_array_shapes_are_actionable_errors() {
        assert!(
            append("dependencies=[", &["../new"])
                .unwrap_err()
                .contains("TOML layer manifest")
        );
        assert!(
            append("dependencies = 'wrong'", &["../new"])
                .unwrap_err()
                .contains("dependencies is not an array")
        );
    }
}
