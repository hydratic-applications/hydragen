use std::error::Error;
use std::path::Path;

use hydragen::layer_manifest::edit::{EditError, append_dependencies};
use hydragen::layer_manifest::{LayerManifestError, LayerManifestFormat, parse_source};
use serde_json::Value;

fn loaded(
    format: LayerManifestFormat,
    source: &str,
) -> hydragen::layer_manifest::LoadedLayerManifest {
    // A diagnostic path, deliberately unrelated to a real file: editing must be pure.
    parse_source(
        &Path::new("/diagnostic-only/app").join(format.filename()),
        format,
        source.into(),
    )
    .unwrap_or_else(|error| panic!("invalid {format} fixture: {error}\n{source}"))
}

fn assert_append(
    format: LayerManifestFormat,
    source: &str,
    references: &[&str],
    preserved: &[&str],
) -> String {
    let layer = loaded(format, source);
    let mut expected = layer.document.clone();
    expected
        .as_object_mut()
        .unwrap()
        .entry("dependencies")
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .unwrap()
        .extend(
            references
                .iter()
                .map(|reference| Value::String((*reference).into())),
        );

    let result = append_dependencies(&layer, references)
        .unwrap_or_else(|error| panic!("{format} append failed: {error}\n{source}"));
    let prepared = loaded(format, &result);
    assert_eq!(prepared.document, expected, "{format}: {result}");
    assert_eq!(layer.source, source, "editing changed its input");

    // Independent of decoded equality: insertion-only editing retains every original
    // byte in order, including comments, quote styles, and multibyte source text.
    let mut remaining = result.as_bytes().iter();
    for byte in source.as_bytes() {
        assert!(
            remaining.any(|candidate| candidate == byte),
            "original {format} bytes were removed or reordered:\n{source}\n=>\n{result}"
        );
    }
    for fragment in preserved {
        assert!(
            result.contains(fragment),
            "{format} changed surrounding text or an existing dependency comment {fragment:?}:\n{result}"
        );
    }
    result
}

#[test]
fn absent_dependencies_preserve_metadata_and_unknown_source_in_all_formats() {
    for (format, source, preserved) in [
        (
            LayerManifestFormat::Json,
            "{\n  \"name\" : \"app\",\n  \"author\":\" 配置 🦀\",\n  \"custom\" : {\"nested\":[true,7,null]}\n}\n",
            vec![
                "  \"name\" : \"app\",",
                "\"custom\" : {\"nested\":[true,7,null]}",
            ],
        ),
        (
            LayerManifestFormat::Jsonc,
            "{\n  // metadata 配置\n  \"name\": \"app\",\n  \"custom\": {\"keep\": true}, // custom comment\n  // closing comment\n}\n",
            vec![
                "// metadata 配置\n",
                "\"custom\": {\"keep\": true}, // custom comment\n",
                "// closing comment\n",
            ],
        ),
        (
            LayerManifestFormat::Toml,
            "# metadata 配置\nname = 'app' # name comment\ncustom.nested = [true, 7]\n\n[settings]\nkeep = 'yes' # table comment\n",
            vec![
                "name = 'app' # name comment\n",
                "custom.nested = [true, 7]\n",
                "[settings]\nkeep = 'yes' # table comment\n",
            ],
        ),
        (
            LayerManifestFormat::Yaml,
            "# metadata 配置\nname: 'app' # name comment\ncustom: {nested: [true, 7, null]} # custom comment\n",
            vec![
                "name: 'app' # name comment\n",
                "custom: {nested: [true, 7, null]} # custom comment\n",
            ],
        ),
        (
            LayerManifestFormat::Yml,
            "# metadata 配置\nname: 'app' # name comment\ncustom: {nested: [true, 7, null]} # custom comment\n",
            vec![
                "name: 'app' # name comment\n",
                "custom: {nested: [true, 7, null]} # custom comment\n",
            ],
        ),
    ] {
        assert_append(
            format,
            source,
            &["../first", "../Second 配置 @literal/./"],
            &preserved,
        );
    }
}

fn assert_yaml_leading_comment_attachment(source: &str, attached_fragment: &str) {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        for ending in ["\n", "\r\n"] {
            let source = source.replace('\n', ending);
            let attached_fragment = attached_fragment.replace('\n', ending);
            // Keep the comment and its original field together as one exact fragment.
            // Retaining their bytes separately would permit a new dependencies field
            // between them, incorrectly making the old comment describe that field.
            let result = assert_append(format, &source, &["../new"], &[&attached_fragment]);
            assert_eq!(result.ends_with('\n'), source.ends_with('\n'));
            if ending == "\r\n" {
                assert!(
                    !result.replace("\r\n", "").contains('\n'),
                    "{format} inserted a bare LF: {result:?}"
                );
            }
        }
    }
}

#[test]
fn yaml_absent_dependencies_keep_leading_comments_attached_to_first_field() {
    assert_yaml_leading_comment_attachment(
        "# Name of this app\nname: app\ncustom: true",
        "# Name of this app\nname: app\n",
    );
}

#[test]
fn yaml_absent_dependencies_keep_leading_comments_on_a_name_only_root() {
    for final_newline in [false, true] {
        let fragment = "# Name of this app\nname: app";
        let source = format!("{fragment}{}", if final_newline { "\n" } else { "" });
        assert_yaml_leading_comment_attachment(&source, fragment);
    }
}

#[test]
fn yaml_absent_dependencies_keep_leading_comments_on_an_indented_name_only_root() {
    for final_newline in [false, true] {
        let fragment = "  # Name of this app\n  name: app";
        let source = format!("{fragment}{}", if final_newline { "\n" } else { "" });
        assert_yaml_leading_comment_attachment(&source, fragment);
    }
}

#[test]
fn yaml_absent_dependencies_keep_leading_comments_on_an_indented_root() {
    assert_yaml_leading_comment_attachment(
        "  # Name of this app\n  name: app\n  custom: true\n",
        "  # Name of this app\n  name: app\n",
    );
}

#[test]
fn yaml_absent_dependencies_keep_leading_comments_between_document_markers() {
    assert_yaml_leading_comment_attachment(
        "%YAML 1.2\n---\n# Name of this app\nname: app\ncustom: true\n... # document end\n",
        "# Name of this app\nname: app\n",
    );
}

#[test]
fn yaml_absent_dependencies_keep_leading_comments_after_a_root_anchor() {
    assert_yaml_leading_comment_attachment(
        "--- &app\n# Name of this app\nname: app\ncustom: true\n...\n",
        "# Name of this app\nname: app\n",
    );
}

#[test]
fn yaml_absent_dependencies_keep_leading_comments_after_a_root_mapping_tag() {
    for properties in ["!!map", "&app !!map"] {
        let source = format!(
            "%YAML 1.2\n--- {properties}\n# Name of this app\nname: app\ncustom: true\n...\n"
        );
        assert_yaml_leading_comment_attachment(&source, "# Name of this app\nname: app\n");
    }
}

#[test]
fn empty_declarations_are_supported_in_all_formats() {
    for format in LayerManifestFormat::ALL {
        let (source, preserved) = match format {
            LayerManifestFormat::Json => (
                "{\"name\":\"app\",\"dependencies\": [  ],\"custom\":false}",
                vec!["\"name\":\"app\"", "\"custom\":false"],
            ),
            LayerManifestFormat::Jsonc => (
                "{\"name\":\"app\",\"dependencies\": [ /* empty list */ ],\"custom\":false}",
                vec!["/* empty list */", "\"custom\":false"],
            ),
            LayerManifestFormat::Toml => (
                "name = 'app'\ndependencies = [ # empty list\n] # dependency close\ncustom = false\n",
                vec!["# empty list\n", "] # dependency close\ncustom = false\n"],
            ),
            LayerManifestFormat::Yaml | LayerManifestFormat::Yml => (
                "name: 'app'\ndependencies: [ ] # empty list\ncustom: false\n",
                vec!["# empty list\ncustom: false\n"],
            ),
        };
        assert_append(format, source, &["../first", "../second"], &preserved);
    }
}

#[test]
fn inline_arrays_keep_existing_literals_and_surrounding_text() {
    for (format, source, preserved) in [
        (
            LayerManifestFormat::Json,
            r#"{"name":"app","dependencies":["..\/old","old@1.2.0"],"custom":{"keep":1}}"#,
            vec![r#""..\/old","old@1.2.0""#, r#""custom":{"keep":1}"#],
        ),
        (
            LayerManifestFormat::Jsonc,
            r#"{"name":"app","dependencies":["../old" /* old dependency */,"old@1.2.0", /* closing */],"custom":7}"#,
            vec![
                r#""../old" /* old dependency */"#,
                r#""old@1.2.0","#,
                "/* closing */",
                r#""custom":7"#,
            ],
        ),
        (
            LayerManifestFormat::Toml,
            "name = 'app'\ndependencies = [ '../old', 'old@1.2.0' ] # dependency list\ncustom = { keep = 7 }\n",
            vec![
                "'../old', 'old@1.2.0'",
                "# dependency list\ncustom = { keep = 7 }\n",
            ],
        ),
        (
            LayerManifestFormat::Yaml,
            "name: app\ndependencies: [ '../old', 'old@1.2.0' ] # dependency list\ncustom: {keep: 7}\n",
            vec![
                "'../old', 'old@1.2.0'",
                "# dependency list\ncustom: {keep: 7}\n",
            ],
        ),
        (
            LayerManifestFormat::Yml,
            "name: app\ndependencies: [ '../old', 'old@1.2.0' ] # dependency list\ncustom: {keep: 7}\n",
            vec![
                "'../old', 'old@1.2.0'",
                "# dependency list\ncustom: {keep: 7}\n",
            ],
        ),
    ] {
        assert_append(format, source, &["../new-a", "../new-b"], &preserved);
    }
}

#[test]
fn multiline_arrays_keep_dependency_comments_and_unknown_fields() {
    for (format, source, preserved) in [
        (
            LayerManifestFormat::Json,
            "{\n  \"name\": \"app\",\n  \"dependencies\": [\n    \"../old\"\n  ],\n  \"custom\": [1, {\"keep\": true}]\n}\n",
            vec!["    \"../old\"", "  \"custom\": [1, {\"keep\": true}]\n"],
        ),
        (
            LayerManifestFormat::Jsonc,
            "{\n  \"name\": \"app\",\n  \"dependencies\": [\n    // existing dependency\n    \"../old\", // attached dependency\n    /* closing dependency comment */\n  ],\n  \"custom\": [1, {\"keep\": true}],\n}\n",
            vec![
                "// existing dependency\n",
                "\"../old\", // attached dependency\n",
                "/* closing dependency comment */\n",
                "  \"custom\": [1, {\"keep\": true}],\n",
            ],
        ),
        (
            LayerManifestFormat::Toml,
            "name = 'app'\ndependencies = [\n  # existing dependency\n  '../old', # attached dependency\n  # closing dependency comment\n] # list end\ncustom = { keep = true }\n",
            vec![
                "# existing dependency\n",
                "'../old', # attached dependency\n",
                "# closing dependency comment\n",
                "] # list end\ncustom = { keep = true }\n",
            ],
        ),
        (
            LayerManifestFormat::Yaml,
            "name: app\ndependencies: # dependency heading\n  # existing dependency\n  - '../old' # attached dependency\n  # closing dependency comment\ncustom: {keep: true}\n",
            vec![
                "dependencies: # dependency heading\n",
                "# existing dependency\n",
                "- '../old' # attached dependency\n",
                "# closing dependency comment\n",
                "custom: {keep: true}\n",
            ],
        ),
        (
            LayerManifestFormat::Yml,
            "name: app\ndependencies: # dependency heading\n  # existing dependency\n  - '../old' # attached dependency\n  # closing dependency comment\ncustom: {keep: true}\n",
            vec![
                "dependencies: # dependency heading\n",
                "# existing dependency\n",
                "- '../old' # attached dependency\n",
                "# closing dependency comment\n",
                "custom: {keep: true}\n",
            ],
        ),
    ] {
        assert_append(format, source, &["../new"], &preserved);
    }
}

#[test]
fn jsonc_closing_line_comments_with_and_without_trailing_commas_survive() {
    for comma in ["", ","] {
        for comment in ["// closing comment", "/* closing comment */"] {
            let source = format!(
                "{{\n \"name\": \"app\",\n \"dependencies\": [\"../old\"{comma} {comment}\n ],\n \"custom\": true\n}}"
            );
            assert_append(
                LayerManifestFormat::Jsonc,
                &source,
                &["../new"],
                &[comment, "\"custom\": true\n"],
            );
        }
    }
}

#[test]
fn jsonc_absent_property_preserves_final_property_and_root_closing_comments() {
    for comma in ["", ","] {
        let source = format!(
            "{{\n \"name\": \"app\"{comma} // name comment\n /* root closing comment */\n}}"
        );
        assert_append(
            LayerManifestFormat::Jsonc,
            &source,
            &["../new"],
            &["// name comment\n", "/* root closing comment */\n"],
        );
    }
}

#[test]
fn jsonc_array_keeps_comments_attached_to_a_comma_on_the_next_line() {
    for comment in [
        "// Required by old",
        "/* Required by old */",
        "/* Required by old\n       Keep this explanation with old */",
    ] {
        for ending in ["\n", "\r\n"] {
            let old_group = format!("    \"../old\"\n    , {comment}\n");
            let source = format!(
                "{{\n  \"name\": \"app\",\n  \"dependencies\": [\n{old_group}  ],\n  \"custom\": {{\"dependencies\": [false]}}\n}}\n"
            )
            .replace('\n', ending);
            let old_group = old_group.replace('\n', ending);
            let result = assert_append(
                LayerManifestFormat::Jsonc,
                &source,
                &["../new-a", "../new-b"],
                &[&old_group, "\"custom\": {\"dependencies\": [false]}"],
            );
            let old_end = result.find(&old_group).unwrap() + old_group.len();
            assert!(old_end <= result.find("../new-a").unwrap());
            assert_new_edit_line_endings(&source, &result);
        }
    }
}

#[test]
fn jsonc_absent_dependencies_keep_the_final_property_comma_comment_group() {
    for comment in [
        "// Name of this app",
        "/* Name of this app */",
        "/* Name of this app\n     Keep this explanation with name */",
    ] {
        for ending in ["\n", "\r\n"] {
            let old_group = format!("  \"name\": \"app\"\n  , {comment}\n");
            let source = format!("{{\n{old_group}}}").replace('\n', ending);
            let old_group = old_group.replace('\n', ending);
            let result = assert_append(
                LayerManifestFormat::Jsonc,
                &source,
                &["../new-a", "../new-b"],
                &[&old_group],
            );
            let old_end = result.find(&old_group).unwrap() + old_group.len();
            assert!(old_end <= result.find("\"dependencies\"").unwrap());
            assert_new_edit_line_endings(&source, &result);
        }
    }
}

#[test]
fn toml_missing_root_key_is_not_inserted_into_table_or_array_of_tables() {
    for tail in [
        "[custom]\nkeep = true\ndependencies = ['not-a-layer-reference']\n",
        "[[custom]]\nkeep = true\n[[custom]]\nkeep = false\n",
        "[custom.deep]\nkeep = true\n",
    ] {
        let source = format!("name = 'app'\n# root metadata\n{tail}");
        assert_append(
            LayerManifestFormat::Toml,
            &source,
            &["../new"],
            &["name = 'app'\n", "# root metadata\n", tail],
        );
    }
}

#[test]
fn toml_dotted_key_order_and_quoted_root_key_are_preserved() {
    for dependencies in ["", "'dependencies' = ['../old'] # list\n"] {
        let source = format!(
            "name = 'app'\ncustom.z = 'last'\n{dependencies}custom.a = 'first'\n[other]\nkeep = true"
        );
        let result = assert_append(
            LayerManifestFormat::Toml,
            &source,
            &["../new"],
            &[
                "custom.z = 'last'\n",
                "custom.a = 'first'\n",
                "[other]\nkeep = true",
            ],
        );
        assert!(result.find("custom.z").unwrap() < result.find("custom.a").unwrap());
        if !dependencies.is_empty() {
            assert!(result.contains("# list\n"));
        }
    }
}

#[test]
fn yaml_directives_and_document_end_markers_survive_absent_and_existing_arrays() {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        for dependencies in [
            "",
            "dependencies: [] # empty\n",
            "dependencies:\n  - '../old' # old\n",
        ] {
            let source = format!(
                "%YAML 1.2\n---\nname: app\n{dependencies}custom: '配置' # custom\n... # document end\n# stream end\n"
            );
            let result = assert_append(
                format,
                &source,
                &["../new"],
                &[
                    "%YAML 1.2\n---\n",
                    "custom: '配置' # custom\n",
                    "... # document end\n# stream end\n",
                ],
            );
            assert!(result.find("dependencies:").unwrap() < result.find("... #").unwrap());
        }
    }
}

#[test]
fn yaml_indentless_arrays_keep_attached_and_following_comments() {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        assert_append(
            format,
            "name: app\ndependencies:\n# old dependency\n- '../old' # attached\n# next field\ncustom: true\n",
            &["../new-a", "../new-b"],
            &[
                "# old dependency\n",
                "- '../old' # attached\n",
                "# next field\ncustom: true\n",
            ],
        );
    }
}

#[test]
fn yaml_last_item_keeps_more_indented_comment_lines_at_eof() {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        for ending in ["\n", "\r\n"] {
            for final_newline in [false, true] {
                let old_group =
                    "  - ../old\n    # Required by old\n    # Keep this explanation with old";
                let source = format!(
                    "name: app\ndependencies:\n{old_group}{}",
                    if final_newline { "\n" } else { "" }
                )
                .replace('\n', ending);
                let old_group = old_group.replace('\n', ending);
                let result =
                    assert_append(format, &source, &["../new-a", "../new-b"], &[&old_group]);
                let old_end = result.find(&old_group).unwrap() + old_group.len();
                assert!(old_end <= result.find("../new-a").unwrap());
                assert_new_edit_line_endings(&source, &result);
            }
        }
    }
}

#[test]
fn yaml_old_item_comments_stay_before_new_items_but_footers_stay_after_them() {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        for ending in ["\n", "\r\n"] {
            let old_group =
                "  - ../old # Existing dependency\n    # Required by old\n    # Old item details\n";
            let footer = "  # Sequence footer\n# Custom field\ncustom: {dependencies: [false]}\n";
            let source =
                format!("name: app\ndependencies:\n{old_group}{footer}").replace('\n', ending);
            let old_group = old_group.replace('\n', ending);
            let footer = footer.replace('\n', ending);
            let result = assert_append(
                format,
                &source,
                &["../new-a", "../new-b"],
                &[&old_group, &footer],
            );
            let old_end = result.find(&old_group).unwrap() + old_group.len();
            assert!(old_end <= result.find("../new-a").unwrap());
            assert!(result.find("../new-b").unwrap() < result.find(&footer).unwrap());
            assert_new_edit_line_endings(&source, &result);
        }
    }
}

#[test]
fn yaml_compact_explicit_dependency_key_accepts_an_existing_sequence() {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        for ending in ["\n", "\r\n"] {
            let source = "name: app\n? dependencies\n: - ../old".replace('\n', ending);
            let declaration = "? dependencies\n: - ../old".replace('\n', ending);
            let result = assert_append(format, &source, &["../new-a", "../new-b"], &[&declaration]);
            assert_new_edit_line_endings(&source, &result);
        }
    }
}

#[test]
fn yaml_compact_explicit_keys_preserve_comment_groups_and_neighboring_ranges() {
    // Vary key spelling and root indentation without multiplying unrelated fixtures.
    for (key, indent) in [
        ("dependencies", ""),
        ("'dependencies'", "  "),
        ("!!str dependencies", ""),
        ("!!str \"dependencies\"", "  "),
    ] {
        for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
            for ending in ["\n", "\r\n"] {
                let heading = format!("{indent}? {key}\n{indent}: - ../first # First item\n");
                let old_group = format!(
                    "{indent}  - '../old' # Last existing item\n{indent}    # Required by old\n"
                );
                let neighbor = format!(
                    "{indent}# Custom field\n{indent}custom:\n{indent}  dependencies: [false]\n{indent}  literal: '? dependencies : - ../old'\n"
                );
                let source = format!(
                    "{indent}name: app\n{indent}description: '配置 before ranges'\n{heading}{old_group}{neighbor}"
                )
                .replace('\n', ending);
                let heading = heading.replace('\n', ending);
                let old_group = old_group.replace('\n', ending);
                let neighbor = neighbor.replace('\n', ending);
                let result = assert_append(
                    format,
                    &source,
                    &["../new-a", "../new-b"],
                    &[&heading, &old_group, &neighbor],
                );
                let old_end = result.find(&old_group).unwrap() + old_group.len();
                assert!(old_end <= result.find("../new-a").unwrap());
                assert!(result.find("../new-b").unwrap() < result.find(&neighbor).unwrap());
                assert_new_edit_line_endings(&source, &result);
            }
        }
    }
}

fn assert_new_edit_line_endings(source: &str, result: &str) {
    assert_eq!(result.ends_with('\n'), source.ends_with('\n'));
    if source.contains("\r\n") {
        assert!(
            !result.replace("\r\n", "").contains('\n'),
            "edit inserted a bare LF: {result:?}"
        );
    } else {
        assert!(!result.contains('\r'), "edit inserted a CR: {result:?}");
    }
}

#[test]
fn yaml_flow_root_maps_and_multiline_flow_lists_preserve_comments() {
    for (source, fragments) in [
        (
            "{name: app, custom: {keep: true}} # root comment",
            vec!["name: app", "custom: {keep: true}", "# root comment"],
        ),
        (
            "{name: app, dependencies: ['../old'], custom: true}\n",
            vec!["name: app", "'../old'", "custom: true"],
        ),
        (
            "name: app\ndependencies: [\n  '../old', # attached\n  # closing comment\n] # list end\ncustom: true\n",
            vec![
                "'../old', # attached\n",
                "# closing comment\n",
                "] # list end\ncustom: true\n",
            ],
        ),
    ] {
        for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
            assert_append(format, source, &["../new"], &fragments);
        }
    }
}

#[test]
fn safe_yaml_unrelated_and_dependency_scalar_anchors_remain_unchanged() {
    for source in [
        "name: app\ncustom: &settings {keep: true}\ncopy: *settings\ndependencies: ['../old']\n",
        "name: app\ndependencies:\n  - &old '../old' # anchored scalar\ncustom: *old\n",
        "name: app\ndependencies: &unshared ['../old'] # unshared sequence\ncustom: true\n",
    ] {
        for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
            let fragments = source
                .lines()
                .filter(|line| line.contains('&') || line.contains('*'))
                .map(|line| {
                    // A flow array necessarily gains text before its close; preserve its
                    // existing scalar/anchor prefix, not the whole edited declaration.
                    line.split(']').next().unwrap()
                })
                .collect::<Vec<_>>();
            assert_append(format, source, &["../new"], &fragments);
        }
    }
}

#[test]
fn unsafe_yaml_sequence_aliases_are_refused_without_mutating_the_input() {
    for source in [
        "name: app\ncustom: &shared ['../old'] # shared declaration\ndependencies: *shared # dependency alias\n",
        "name: app\ndependencies: &shared ['../old'] # shared declaration\ncustom: *shared # unrelated alias\n",
    ] {
        for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
            let layer = loaded(format, source);
            let original_data = layer.document.clone();
            let error = append_dependencies(&layer, &["../new"]).unwrap_err();
            assert!(matches!(
                error,
                EditError::UnsupportedEdit { .. } | EditError::UnintendedChange { .. }
            ));
            let message = error.to_string();
            assert!(
                message.contains("alias")
                    || message.contains("anchor")
                    || message.contains("beyond"),
                "{message}"
            );
            assert_eq!(layer.source, source);
            assert_eq!(layer.document, original_data);
        }
    }
}

fn basic_source(format: LayerManifestFormat) -> &'static str {
    match format {
        LayerManifestFormat::Json | LayerManifestFormat::Jsonc => {
            "{\n  \"name\": \"app\",\n  \"description\": \"配置 🦀 and \\\"quoted\\\"\",\n  \"dependencies\": [\"../old\"],\n  \"custom\": {\"keep\": true}\n}\n"
        }
        LayerManifestFormat::Toml => {
            "name = 'app'\ndescription = '配置 🦀 and \"quoted\"'\ndependencies = ['../old']\ncustom = { keep = true }\n"
        }
        LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
            "name: app\ndescription: '配置 🦀 and \"quoted\"'\ndependencies: ['../old']\ncustom: {keep: true}\n"
        }
    }
}

#[test]
fn line_endings_and_final_newline_state_are_preserved_in_all_formats() {
    for format in LayerManifestFormat::ALL {
        for ending in ["\n", "\r\n"] {
            for final_newline in [false, true] {
                let mut source = basic_source(format).replace('\n', ending);
                if !final_newline {
                    source.truncate(source.len() - ending.len());
                }
                let result = assert_append(format, &source, &["../new"], &["配置 🦀"]);
                assert_eq!(
                    result.ends_with('\n'),
                    final_newline,
                    "{format}: {result:?}"
                );
                if ending == "\r\n" {
                    assert!(
                        !result.replace("\r\n", "").contains('\n'),
                        "{format} inserted a bare LF: {result:?}"
                    );
                } else {
                    assert!(!result.contains('\r'));
                }
            }
        }
    }
}

#[test]
fn absent_declarations_preserve_no_final_newline_and_crlf() {
    for (format, source) in [
        (LayerManifestFormat::Json, "{\r\n\"name\":\"app\"\r\n}"),
        (
            LayerManifestFormat::Jsonc,
            "{\r\n\"name\":\"app\" // name\r\n}",
        ),
        (LayerManifestFormat::Toml, "name = 'app'\r\ncustom = '配置'"),
        (LayerManifestFormat::Yaml, "name: app\r\ncustom: '配置'"),
        (LayerManifestFormat::Yml, "name: app\r\ncustom: '配置'"),
    ] {
        let result = assert_append(format, source, &["../new"], &[]);
        assert!(!result.ends_with('\n'), "{format}: {result:?}");
        assert!(
            !result.replace("\r\n", "").contains('\n'),
            "{format}: {result:?}"
        );
    }
}

#[test]
fn new_references_are_encoded_without_rewriting_multibyte_or_escaped_source() {
    for format in LayerManifestFormat::ALL {
        assert_append(
            format,
            basic_source(format),
            &[
                "../Space 配置 @literal/./",
                "../quote\"and\\slash\nand\ttab",
                "@scope/shared@1.2.3-beta.1+build.07",
            ],
            &["配置 🦀", "../old"],
        );
    }
}

#[test]
fn decoded_property_names_not_spelling_identify_the_dependency_declaration() {
    for format in [LayerManifestFormat::Json, LayerManifestFormat::Jsonc] {
        assert_append(
            format,
            r#"{"name":"app","description":"配置","depend\u0065ncies":["../old"],"custom":{"dependencies":[false]}}"#,
            &["../new"],
            &[
                r#""depend\u0065ncies":"#,
                r#""custom":{"dependencies":[false]}"#,
            ],
        );
    }
    assert_append(
        LayerManifestFormat::Yaml,
        "name: app\n'dependencies': ['../old'] # quoted key\ncustom: {dependencies: [false]}\n",
        &["../new"],
        &[
            "'dependencies':",
            "# quoted key\n",
            "custom: {dependencies: [false]}\n",
        ],
    );
}

#[test]
fn empty_batch_is_byte_identical_even_for_unsafe_to_edit_yaml() {
    for format in LayerManifestFormat::ALL {
        let source = basic_source(format);
        assert_eq!(
            append_dependencies(&loaded(format, source), &[]).unwrap(),
            source
        );
    }
    let source = "name: app\ncustom: &shared ['../old']\ndependencies: *shared";
    assert_eq!(
        append_dependencies(&loaded(LayerManifestFormat::Yaml, source), &[]).unwrap(),
        source
    );
}

#[test]
fn invalid_supplied_references_fail_the_maintained_schema_with_error_sources() {
    for format in LayerManifestFormat::ALL {
        for invalid in [
            "",
            "bare-name",
            "shared@latest",
            "shared@^1.2.3",
            "shared@01.2.3",
        ] {
            let layer = loaded(format, basic_source(format));
            let error = append_dependencies(&layer, &["../valid", invalid]).unwrap_err();
            let EditError::InvalidCandidate {
                path,
                format: actual_format,
                source,
            } = &error
            else {
                panic!("expected schema failure for {invalid:?}, got {error:?}");
            };
            assert_eq!(path, &layer.path);
            assert_eq!(*actual_format, format);
            assert!(matches!(source, LayerManifestError::Metadata { .. }));
            assert!(error.source().is_some());
            assert!(source.source().is_some());
            assert!(error.to_string().contains("/dependencies/2"), "{error}");
            assert_eq!(layer.source, basic_source(format));
        }
    }
}

#[test]
fn schema_failure_is_not_hidden_by_an_unsupported_source_structure() {
    let source = "name: app\ncustom: &shared ['../old']\ndependencies: *shared";
    let layer = loaded(LayerManifestFormat::Yaml, source);
    assert!(matches!(
        append_dependencies(&layer, &["bare-name"]),
        Err(EditError::InvalidCandidate { .. })
    ));
}
