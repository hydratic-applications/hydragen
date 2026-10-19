use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use hydragen::layer_manifest::edit::append_dependencies;
use hydragen::layer_manifest::parse_source;
use hydragen::{
    LayerManifestFormat as Format, Metadata, discover, encode_metadata, validate_document,
};
use serde_json::{Value, json};

#[path = "support/insertion_oracle.rs"]
mod insertion_oracle;

const AUTHOR: &str = "  description: '配置 🦀' \\team  ";
const DESCRIPTION: &str = "  author: \"name\" and dependencies: []  ";

struct Fixture {
    root: tempfile::TempDir,
    app: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir(&app).unwrap();
        Self { root, app }
    }

    fn dependency(&self, name: &str) {
        let directory = self.root.path().join(name);
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("layer.yaml"), format!("name: {name}\n")).unwrap();
    }

    fn write(&self, format: Format, source: &str) -> PathBuf {
        let path = self.app.join(format.filename());
        fs::write(&path, source).unwrap();
        path
    }

    fn run(&self, arguments: &[&str]) -> Output {
        // File captures and a bounded child guard keep an accidental prompt or
        // noisy command from hanging the suite or leaving a process behind.
        let stdout = tempfile::NamedTempFile::new().unwrap();
        let stderr = tempfile::NamedTempFile::new().unwrap();
        let mut child = Running(
            Command::new(env!("CARGO_BIN_EXE_hydragen"))
                .current_dir(&self.app)
                .args(arguments)
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
            assert!(Instant::now() < deadline, "field-order CLI test timed out");
            std::thread::sleep(Duration::from_millis(10));
        };
        Output {
            status,
            stdout: fs::read(stdout.path()).unwrap(),
            stderr: fs::read(stderr.path()).unwrap(),
        }
    }
}

struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn assert_success_without_warnings(output: &Output) {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(!output.stdout.contains(&b'\x1b'), "{output:?}");
}

fn metadata(author: bool, description: bool) -> Metadata {
    Metadata {
        name: "app".into(),
        author: author.then(|| AUTHOR.into()),
        description: description.then(|| DESCRIPTION.into()),
    }
}

// These fixtures put each root key on its own line. Match key syntax at its
// actual indentation, never a bare word that could occur in a metadata value.
fn root_position(format: Format, source: &str, key: &str) -> usize {
    let prefix = match format {
        Format::Yaml | Format::Yml => format!("{key}:"),
        Format::Toml => format!("{key} ="),
        Format::Json | Format::Jsonc => format!("  \"{key}\":"),
    };
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if line.starts_with(&prefix) {
            return offset;
        }
        offset += line.len();
    }
    panic!("missing root key {key:?} in {format}:\n{source}");
}

fn assert_root_order(format: Format, source: &str, keys: &[&str]) {
    let positions: Vec<_> = keys
        .iter()
        .map(|key| root_position(format, source, key))
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "expected {keys:?} source order in {format}:\n{source}"
    );
}

fn assert_generated(format: Format, source: &str, expected: &Metadata) {
    let loaded = parse_source(
        &Path::new("/diagnostic-only").join(format.filename()),
        format,
        source.into(),
    )
    .unwrap();
    assert_eq!(loaded.metadata, *expected);
    // Exact object equality checks omission of dependencies, $schema, version,
    // and unsupplied optional fields as well as preserving accepted strings.
    assert_eq!(loaded.document, serde_json::to_value(expected).unwrap());
    assert!(source.ends_with('\n'));
    let mut keys = vec!["name"];
    if expected.author.is_some() {
        keys.push("author");
    }
    if expected.description.is_some() {
        keys.push("description");
    }
    assert_root_order(format, source, &keys);
}

#[test]
fn shared_guided_encoder_keeps_typed_metadata_order_and_omissions() {
    for format in Format::ALL {
        for author in [false, true] {
            for description in [false, true] {
                let expected = metadata(author, description);
                let encoded = encode_metadata(&expected, format).unwrap();
                assert_generated(format, std::str::from_utf8(&encoded).unwrap(), &expected);
            }
        }
    }
}

#[test]
fn cli_init_order_is_independent_of_flag_order_in_every_optional_combination() {
    for format in Format::ALL {
        for author in [false, true] {
            for description in [false, true] {
                for reversed in [false, true] {
                    let fixture = Fixture::new();
                    let expected = metadata(author, description);
                    let mut fields = vec![("--name", expected.name.as_str())];
                    if author {
                        fields.push(("--author", AUTHOR));
                    }
                    if description {
                        fields.push(("--description", DESCRIPTION));
                    }
                    if reversed {
                        fields.reverse();
                    }
                    let mut arguments = vec!["init", "--format", format.extension()];
                    for (flag, value) in fields {
                        arguments.extend([flag, value]);
                    }
                    let output = fixture.run(&arguments);
                    assert_success_without_warnings(&output);
                    assert_eq!(
                        String::from_utf8_lossy(&output.stdout),
                        format!(
                            "Layer initialised, layer manifest created at: {}\n",
                            format.filename()
                        )
                    );
                    let source = fs::read_to_string(fixture.app.join(format.filename())).unwrap();
                    assert_generated(format, &source, &expected);
                    assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
                }
            }
        }
    }
}

// Keep field order explicit in fixtures instead of serialising a generic map,
// whose enumeration could erase the alternative layout this suite exercises.
fn ordered_source(format: Format, fields: &[(&str, Value)]) -> String {
    match format {
        Format::Yaml | Format::Yml => fields
            .iter()
            .map(|(key, value)| format!("{key}: {value}\n"))
            .collect(),
        Format::Toml => fields
            .iter()
            .map(|(key, value)| {
                let key = if *key == "$schema" {
                    "\"$schema\""
                } else {
                    key
                };
                // The shared fixture values are JSON strings and ordinary arrays,
                // whose spellings are also valid TOML values.
                format!("{key} = {value}\n")
            })
            .collect(),
        Format::Json | Format::Jsonc => format!(
            "{{\n{}\n}}\n",
            fields
                .iter()
                .map(|(key, value)| format!("  \"{key}\": {value}"))
                .collect::<Vec<_>>()
                .join(",\n")
        ),
    }
}

fn flexible_fields() -> Vec<(&'static str, Value)> {
    vec![
        ("dependencies", json!(["../z-old", "../a-old"])),
        ("description", json!(DESCRIPTION)),
        ("custom", json!([true, 7, "name: dependencies"])),
        ("$schema", json!("https://example.invalid/never-fetch.json")),
        ("name", json!("app")),
        ("author", json!(AUTHOR)),
    ]
}

#[test]
fn loading_and_schema_accept_alternative_orders_without_rewrites_or_cli_warnings() {
    let fields = flexible_fields();
    let expected = Value::Object(
        fields
            .iter()
            .map(|(key, value)| ((*key).into(), value.clone()))
            .collect(),
    );
    assert_eq!(validate_document(&expected).unwrap(), metadata(true, true));
    for format in Format::ALL {
        // Dependencies-first and interleaved metadata are independently exercised.
        for order in [[0, 1, 2, 3, 4, 5], [1, 2, 5, 0, 3, 4]] {
            let fixture = Fixture::new();
            fixture.dependency("z-old");
            fixture.dependency("a-old");
            let reordered: Vec<_> = order.iter().map(|index| fields[*index].clone()).collect();
            let source = ordered_source(format, &reordered);
            let path = fixture.write(format, &source);
            let loaded = discover(&fixture.app).unwrap().load().unwrap();
            assert_eq!(loaded.document, expected, "{format}: {source}");
            assert_eq!(loaded.metadata, metadata(true, true));
            assert_eq!(loaded.source, source);
            assert_eq!(fs::read_to_string(&path).unwrap(), source);
            // An all-repeat command exercises operational loading without any
            // legitimate reason to warn or rewrite the hand-authored layout.
            let output = fixture.run(&["add", "../a-old", "../z-old"]);
            assert_success_without_warnings(&output);
            assert!(
                String::from_utf8_lossy(&output.stdout).starts_with("No changes needed for app.")
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), source);
            assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
        }
    }
}

#[test]
fn flexible_order_does_not_relax_invalid_content_duplicate_keys_or_syntax() {
    for format in Format::ALL {
        for invalid in [
            vec![("dependencies", json!([])), ("name", json!("Invalid-Name"))],
            vec![
                ("dependencies", json!(["shared@latest"])),
                ("name", json!("app")),
            ],
            vec![("description", json!("  ")), ("name", json!("app"))],
            vec![
                ("dependencies", json!([])),
                ("name", json!("app")),
                ("name", json!("other")),
            ],
        ] {
            let fixture = Fixture::new();
            let source = ordered_source(format, &invalid);
            let path = fixture.write(format, &source);
            assert!(
                discover(&fixture.app).unwrap().load().is_err(),
                "{format}: {source}"
            );
            let output = fixture.run(&["add", "../missing"]);
            assert!(!output.status.success(), "{output:?}");
            assert!(output.stdout.is_empty(), "{output:?}");
            assert_eq!(fs::read_to_string(path).unwrap(), source);
            assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
        }
        let invalid_syntax = match format {
            Format::Yaml | Format::Yml => "dependencies: [\nname: app\n",
            Format::Toml => "dependencies = []\n[layer]\nname = 'nested-not-root'\n",
            Format::Json | Format::Jsonc => "{\"dependencies\": [] \"name\": \"app\"}",
        };
        assert!(parse_source(Path::new(format.filename()), format, invalid_syntax.into()).is_err());
    }
}

fn assert_original_bytes_in_order(source: &str, result: &str) {
    // Coarse matrix check only. Missing-field fixtures additionally check exact
    // permitted sites/text below; subsequence alone cannot preserve comments.
    let mut remaining = result.as_bytes().iter();
    for byte in source.as_bytes() {
        assert!(
            remaining.any(|candidate| candidate == byte),
            "source bytes removed or reordered:\n{source}\n=>\n{result}"
        );
    }
}

fn assert_missing_field_insertions(
    format: Format,
    source: &str,
    result: &str,
    references: &[&str],
) {
    // These callers supply generated/ordered fixtures with known pretty JSON
    // closing lines and plain reference strings, not arbitrary input layouts.
    let values = references
        .iter()
        .map(|reference| format!("\"{reference}\""))
        .collect::<Vec<_>>()
        .join(", ");
    match format {
        Format::Yaml | Format::Yml | Format::Toml => {
            let assignment = if format == Format::Toml { " =" } else { ":" };
            let text = format!("dependencies{assignment} [{values}]\n");
            insertion_oracle::assert_insertions(source, result, &[(source.len(), &text)]);
        }
        Format::Json | Format::Jsonc => {
            assert!(source.ends_with("\n}\n"));
            let text = format!("  \"dependencies\": [{values}]\n");
            insertion_oracle::assert_insertions(
                source,
                result,
                &[(source.len() - 3, ","), (source.len() - 2, &text)],
            );
        }
    }
}

fn assert_append(format: Format, source: &str, references: &[&str]) -> String {
    let path = Path::new("/diagnostic-only").join(format.filename());
    let loaded = parse_source(&path, format, source.into()).unwrap();
    let mut expected = loaded.document.clone();
    expected
        .as_object_mut()
        .unwrap()
        .entry("dependencies")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .unwrap()
        .extend(references.iter().map(|reference| json!(reference)));
    let result = append_dependencies(&loaded, references).unwrap();
    assert_eq!(
        parse_source(&path, format, result.clone())
            .unwrap()
            .document,
        expected
    );
    assert_eq!(loaded.source, source);
    assert_original_bytes_in_order(source, &result);
    result
}

fn init_then_add(format: Format) {
    for author in [false, true] {
        for description in [false, true] {
            let fixture = Fixture::new();
            fixture.dependency("shared");
            let mut arguments = vec!["init", "--format", format.extension()];
            if description {
                arguments.extend(["--description", DESCRIPTION]);
            }
            if author {
                arguments.extend(["--author", AUTHOR]);
            }
            arguments.extend(["--name", "app"]);
            assert_success_without_warnings(&fixture.run(&arguments));
            let path = fixture.app.join(format.filename());
            let source = fs::read_to_string(&path).unwrap();
            assert_success_without_warnings(&fixture.run(&["add", "../shared"]));
            let result = fs::read_to_string(path).unwrap();
            let loaded = discover(&fixture.app).unwrap().load().unwrap();
            let expected_metadata = metadata(author, description);
            assert_eq!(loaded.metadata, expected_metadata);
            let mut expected = serde_json::to_value(&expected_metadata).unwrap();
            expected["dependencies"] = json!(["../shared"]);
            assert_eq!(loaded.document, expected);
            assert_original_bytes_in_order(&source, &result);
            assert_missing_field_insertions(format, &source, &result, &["../shared"]);
            let mut keys = vec!["name"];
            if author {
                keys.push("author");
            }
            if description {
                keys.push("description");
            }
            keys.push("dependencies");
            assert_root_order(format, &result, &keys);
            assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
        }
    }
}

// Separate tests expose each format's placement result even when YAML is red.
macro_rules! format_tests {
    ($helper:ident; $($name:ident => $format:ident),+ $(,)?) => {
        $(#[test] fn $name() { $helper(Format::$format); })+
    };
}

format_tests!(init_then_add;
    init_then_add_yaml => Yaml,
    init_then_add_yml => Yml,
    init_then_add_toml => Toml,
    init_then_add_json => Json,
    init_then_add_jsonc => Jsonc,
);

fn missing_dependencies_follow_existing_root_entries(format: Format) {
    let fields = vec![
        ("description", json!(DESCRIPTION)),
        ("custom", json!([true, 7])),
        ("name", json!("app")),
        ("$schema", json!("https://example.invalid/schema.json")),
        ("author", json!(AUTHOR)),
        ("tail", json!(["dependencies: literal", false])),
    ];
    let source = ordered_source(format, &fields);
    let result = assert_append(format, &source, &["../z-new", "../a-new"]);
    assert_missing_field_insertions(format, &source, &result, &["../z-new", "../a-new"]);
    // $schema has quoted TOML spelling; checking the final root assignment plus
    // exact prefix preservation covers it without pretending it is bare TOML.
    assert_root_order(
        format,
        &result,
        &[
            "description",
            "custom",
            "name",
            "author",
            "tail",
            "dependencies",
        ],
    );
    assert!(
        result.starts_with(source.trim_end_matches(['\n', '}'])),
        "{format}: {result}"
    );
}

format_tests!(missing_dependencies_follow_existing_root_entries;
    root_tail_yaml => Yaml,
    root_tail_yml => Yml,
    root_tail_toml => Toml,
    root_tail_json => Json,
    root_tail_jsonc => Jsonc,
);

#[test]
fn flow_yaml_missing_dependencies_are_last_before_the_root_close() {
    for format in [Format::Yaml, Format::Yml] {
        let source = "{description: 'author: literal', custom: {dependencies: [false]}, name: app, $schema: 'https://example.invalid/schema', author: Team, tail: [7, true]} # root footer\n";
        let result = assert_append(format, source, &["../z-new", "../a-new"]);
        assert!(result.starts_with(source.split_once("} # root footer").unwrap().0));
        let last_old = result.find("tail: [7, true]").unwrap();
        let new = result.rfind("dependencies:").unwrap();
        let close = result.rfind("} # root footer\n").unwrap();
        assert!(last_old < new && new < close, "{format}: {result}");
    }
}

#[test]
fn jsonc_root_tail_keeps_closing_comments_and_optional_trailing_commas() {
    for comma in ["", ","] {
        let source = format!(
            "{{\n  \"description\": \"name: literal\",\n  \"name\": \"app\",\n  \"$schema\": \"https://example.invalid/schema\",\n  \"custom\": {{\"dependencies\": [false]}}{comma} // custom attachment\n  /* root closing comment */\n}}\n"
        );
        let result = assert_append(Format::Jsonc, &source, &["../new"]);
        assert_root_order(
            Format::Jsonc,
            &result,
            &["description", "name", "$schema", "custom", "dependencies"],
        );
        assert!(result.contains("\"custom\": {\"dependencies\": [false]}, // custom attachment\n"));
        assert!(result.ends_with("  /* root closing comment */\n}\n"));
        assert!(
            root_position(Format::Jsonc, &result, "dependencies")
                < result.find("  /* root closing comment */").unwrap()
        );
    }
}

#[test]
fn toml_root_tail_follows_interleaved_dotted_assignments_before_table_comments() {
    for sections in [
        "\n# Settings table documentation\n[settings]\ndependencies = ['nested-not-root']\n",
        "\n# Item collection documentation\n[[items]]\nname = 'first'\n[[items]]\nname = 'second'\n",
        "\n# Nested table documentation\n[custom.child]\nkeep = true\n\n# Item documentation\n[[items]]\nid = 1\n",
    ] {
        let root = "description = 'name = literal'\ncustom.z = 1\nname = 'app'\n\"$schema\" = 'https://example.invalid/schema'\nother.x = 2\ncustom.a = 3 # last root assignment\n";
        let source = format!("{root}{sections}");
        let result = assert_append(Format::Toml, &source, &["../new"]);
        assert!(result.starts_with(root), "{result}");
        assert!(result.ends_with(sections), "{result}");
        let dependencies = root_position(Format::Toml, &result, "dependencies");
        assert!(dependencies >= root.len());
        assert!(dependencies < result.len() - sections.len());
    }
}

#[test]
fn existing_dependencies_before_name_extend_in_place_and_repeats_do_not_stage() {
    for format in Format::ALL {
        for empty in [false, true] {
            let fixture = Fixture::new();
            for name in ["z-old", "a-old", "z-new", "a-new"] {
                fixture.dependency(name);
            }
            let mut fields = flexible_fields();
            if empty {
                fields[0].1 = json!([]);
            }
            let source = ordered_source(format, &fields);
            let path = fixture.write(format, &source);
            let suffix = &source[root_position(format, &source, "description")..];
            let output = fixture.run(&["add", "../z-new", "../a-new", "../z-new"]);
            assert_success_without_warnings(&output);
            let result = fs::read_to_string(&path).unwrap();
            assert!(result.ends_with(suffix), "{format}: {result}");
            assert_root_order(
                format,
                &result,
                &["dependencies", "description", "custom", "name", "author"],
            );
            let expected = if empty {
                json!(["../z-new", "../a-new"])
            } else {
                json!(["../z-old", "../a-old", "../z-new", "../a-new"])
            };
            assert_eq!(
                discover(&fixture.app).unwrap().load().unwrap().document["dependencies"],
                expected
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                "Dependencies updated for app.\n\nAdded (2)\n  z-new\n  a-new\n"
            );

            let before = fs::metadata(&path).unwrap();
            let directory_modified = fs::metadata(&fixture.app).unwrap().modified().unwrap();
            let output = fixture.run(&["add", "../a-new", "../z-new", "../a-new"]);
            assert_success_without_warnings(&output);
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                "No changes needed for app.\n\nAlready present (2)\n  a-new\n  z-new\n"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), result);
            assert_eq!(
                fs::metadata(&path).unwrap().modified().unwrap(),
                before.modified().unwrap()
            );
            // Creating and then cleaning a transient staging entry would also
            // modify the directory, even if its final listing looked unchanged.
            assert_eq!(
                fs::metadata(&fixture.app).unwrap().modified().unwrap(),
                directory_modified
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
            }
            assert_eq!(
                fs::read_dir(&fixture.app).unwrap().count(),
                1,
                "staging file left behind"
            );
        }
    }
}
