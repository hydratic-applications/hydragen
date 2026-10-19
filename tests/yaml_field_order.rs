use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use hydragen::layer_manifest::edit::{EditError, append_dependencies};
use hydragen::layer_manifest::{LayerManifestFormat, parse_source};
use serde_json::json;

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run_add(directory: &Path) -> Output {
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let stderr = tempfile::NamedTempFile::new().unwrap();
    let mut child = Running(
        Command::new(env!("CARGO_BIN_EXE_hydragen"))
            .current_dir(directory)
            .args(["add", "./dep"])
            .env("CI", "true")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(stdout.reopen().unwrap())
            .stderr(stderr.reopen().unwrap())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "YAML field-order CLI test timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: std::fs::read(stdout.path()).unwrap(),
        stderr: std::fs::read(stderr.path()).unwrap(),
    }
}

fn assert_exact(source: &str, expected: &str) {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        let path = Path::new(format.filename());
        let original = parse_source(path, format, source.into())
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let result = append_dependencies(&original, &["../new", "../second"])
            .unwrap_or_else(|error| panic!("{source:?}: {error}"));
        assert_eq!(result, expected, "{format}: {source:?}");
        let mut intended = original.document.clone();
        intended["dependencies"] = json!(["../new", "../second"]);
        let decoded = parse_source(path, format, result.clone()).unwrap();
        assert_eq!(decoded.document, intended, "{source:?}");
        assert_eq!(original.source, source);
        assert_eq!(
            result.ends_with(['\r', '\n']),
            source.ends_with(['\r', '\n'])
        );

        // Exactly one insertion, not merely a subsequence of original bytes.
        let common = source
            .bytes()
            .zip(result.bytes())
            .take_while(|(before, after)| before == after)
            .count();
        assert!(result[common..].ends_with(&source[common..]), "{result:?}");
    }
}

fn assert_tail(prefix: &str, footer: &str, indent: &str) {
    let source = format!("{prefix}{footer}");
    let before = if prefix.ends_with('\n') { "" } else { "\n" };
    let after = if !footer.is_empty() || source.ends_with('\n') {
        "\n"
    } else {
        ""
    };
    let expected =
        format!("{prefix}{before}{indent}dependencies: [\"../new\", \"../second\"]{after}{footer}");
    for eol in ["\n", "\r\n", "\r"] {
        let expected = if source.contains('\n') {
            expected.replace('\n', eol)
        } else {
            expected.clone()
        };
        assert_exact(&source.replace('\n', eol), &expected);
    }
}

#[test]
fn final_subtrees_and_their_comments_precede_the_new_root_field() {
    for value in [
        "custom: {nested: [one,\n  two]} # closing inline\n",
        "custom:\n  nested:\n    - {flow: [one, two]}\n    - [three,\n       four] # last inline\n",
        "custom:\n- one\n- nested:\n    value: two # last inline\n",
        "custom:\n  ? !!str &key nested\n  : &value\n    - first\n    - second # last inline\n",
        "custom: &empty [] # empty flow\n",
        "custom: &empty {} # empty mapping\n",
        "custom:\n",  // Implicit null has no scalar token.
        "? custom\n", // Implicit null without a value indicator.
        "? custom\n: # null inline\n",
        "custom: &nothing # null property\n",
        "custom: !!str # empty tagged string\n",
        "custom: '' # empty quoted string\n",
        "custom:\n  - one\n  - # empty final item\n",
        "custom:\n- # empty indentless item\n",
        "custom:\n  nested: # empty final nested value\n",
    ] {
        let prefix = format!(
            "# Header 💧\nname: app\n{value}  # Attached to final subtree\n\n    # More detail\n"
        );
        for footer in [
            "# Root footer\n... # End\n# Outside document\n",
            "# Root footer without final newline",
            "# Root footer\n  # Footer continuation\n...\n",
            "... # End\n# Outside document",
            "",
        ] {
            assert_tail(&prefix, footer, "");
        }
    }
}

#[test]
fn root_and_key_properties_directives_bom_and_indentation_remain_in_place() {
    for prefix in [
        "\u{feff}%YAML 1.2\n--- &root !!map # root properties\n# Name\nname: app\ncustom: value # final inline\n",
        "%TAG !e! tag:yaml.org,2002:\n--- !e!map &root\n  # Name\n  name: app\n  ? !e!str &key custom\n  : [one, two] # inline\n    # Attached detail\n",
        "# Name\n&name name: app\n? !!str &key custom\n: *name # alias value\n",
        "# Name\n!!str name: app\n? 'custom'\n: &value\n  nested: [one, two]\n",
        "# Name\n  name: app\n  ? custom\n  : - one\n    - two # compact sequence\n",
        "name: app\nkey: &custom custom\n? *custom\n: [one, two] # aliased final key\n",
    ] {
        let indent = if prefix.contains("  name: app") {
            "  "
        } else {
            ""
        };
        let footer = format!("{indent}# Root footer\n... # End\n# Outside document");
        assert_tail(prefix, &footer, indent);
    }
    assert_tail("\u{feff}name: app", "", "");
    assert_tail("\u{feff}  name: app", "", "  ");
}

#[test]
fn multiline_quoted_and_plain_strings_are_not_split_by_colons_or_comment_text() {
    for value in [
        "\"first: # not a comment\n  second 💧\"",
        "'first: # not a comment\n  second ''quoted'''",
        "first line\n  second line",
        "&value !!str \"first\n  second\"",
    ] {
        for suffix in ["", "\n", " # inline\n  # Attached\n"] {
            let prefix = format!("name: app\ncustom: {value}{suffix}");
            assert_tail(&prefix, "", "");
            let ended = format!("{prefix}{}", if prefix.ends_with('\n') { "" } else { "\n" });
            assert_tail(&ended, "\n# Root footer\n...\n", "");
        }
    }
}

#[test]
fn literal_and_folded_strings_preserve_chomping_and_meaningful_blank_lines() {
    for style in [
        "|", "|-", "|+", ">", ">-", ">+", "|2", "|2-", "|+2", ">2", ">2-", ">+2",
    ] {
        for body in [
            "",
            "\n",
            "  \n",
            "  first: value\n  second # string content\n",
            "  first\n\n  last\n\n\n",
            "  first\n    more indented\n\n  last\n  \n",
            "  # Only comment-looking content\n",
        ] {
            let prefix = format!("name: app\ncustom: {style} # header comment\n{body}");
            for footer in ["", "# Root footer\n... # End\n# Outside", "...\n"] {
                assert_tail(&prefix, footer, "");
            }
        }
    }
}

#[test]
fn empty_and_comment_only_block_strings_and_safe_eof_arrangements_are_editable() {
    for style in ["|", "|-", "|+", ">", ">-", ">+"] {
        for body in [
            "",
            "\n",
            "\n\n",
            "\n  ",
            "\n    ",
            "\n  text",
            "\n  text\n",
            "\n  text\n\n",
            "\n  text\n    ",
            "\n  # Comment-looking content",
            " # Header comment",
            " # Header comment\n",
        ] {
            assert_tail(&format!("name: app\ncustom: {style}{body}"), "", "");
        }
        // Strip and clip safely ignore an additional blank trailing line;
        // keep is covered separately by the specific refusal test below.
        if !style.ends_with('+') {
            for body in ["\n  text\n  ", "\n  text\n\n  "] {
                assert_tail(&format!("name: app\ncustom: {style}{body}"), "", "");
            }
        }
    }
    assert_tail(
        "name: app\nearlier: |+\n  earlier\n\ncustom: >+\n  final",
        "",
        "",
    );
}

#[test]
fn nested_final_block_strings_keep_their_whole_value_and_attached_comments() {
    for style in ["|", "|-", "|+", ">", ">-", ">+"] {
        for value in [
            format!("custom:\n  nested: {style}\n    text\n\n"),
            format!("custom:\n- nested:\n    - {style}\n      text\n\n"),
            format!("custom:\n  - {style}\n    # This is string content\n"),
            format!("custom:\n  nested: {style}\n"),
        ] {
            assert_tail(&format!("name: app\n{value}"), "# Footer\n...\n", "");
            assert_tail(&format!("name: app\n{value}  # Attached\n"), "# Footer", "");
        }
    }
}

#[test]
fn unsafe_keep_chomping_eof_separator_is_refused_specifically_before_staging() {
    for format in [LayerManifestFormat::Yaml, LayerManifestFormat::Yml] {
        for style in ["|+", ">+"] {
            for prefix in ["custom: ", "custom:\n  nested: "] {
                let indent = if prefix.ends_with("nested: ") {
                    "    "
                } else {
                    "  "
                };
                for gap in ["\n", "\n\n"] {
                    for eol in ["\n", "\r\n", "\r"] {
                        let source = format!("name: app\nearlier: |-\n  Earlier block string\n{prefix}{style}\n{indent}text{gap}{indent}").replace('\n', eol);
                        let directory = tempfile::tempdir().unwrap();
                        let target = directory.path().join(format.filename());
                        std::fs::write(&target, &source).unwrap();
                        let layer = parse_source(&target, format, source.clone()).unwrap();
                        let error = append_dependencies(&layer, &["../new"]).unwrap_err();
                        assert!(
                            matches!(error, EditError::UnsupportedEdit { .. }),
                            "{error}"
                        );
                        let message = error.to_string();
                        assert!(message.contains("final YAML block string"), "{message}");
                        assert!(
                            message.contains("trailing whitespace or newlines"),
                            "{message}"
                        );
                        assert_eq!(layer.source, source);
                        let separated = format!("{source}{eol}dependencies: [\"../new\"]");
                        let changed = parse_source(&target, format, separated).unwrap();
                        assert_ne!(layer.document["custom"], changed.document["custom"]);

                        // Exercise the actual command path as well: it must stop
                        // during preparation, not create a replacement staging file.
                        let dependency = directory.path().join("dep");
                        std::fs::create_dir(&dependency).unwrap();
                        std::fs::write(dependency.join("layer.yaml"), "name: dep\n").unwrap();
                        let output = run_add(directory.path());
                        assert!(!output.status.success(), "{source:?}");
                        assert!(
                            String::from_utf8_lossy(&output.stderr)
                                .contains("final YAML block string")
                        );
                        assert_eq!(std::fs::read_to_string(&target).unwrap(), source);
                        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
                    }
                }
            }
        }
    }
}
