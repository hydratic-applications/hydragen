use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use hydragen::LayerManifestFormat;
use serde_json::json;

mod diagnostic_body_support;

struct Fixture {
    _root: tempfile::TempDir,
    app: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir(&app).unwrap();
        Self { _root: root, app }
    }

    fn layer(&self, directory: &str, name: &str) -> PathBuf {
        let path = self._root.path().join(directory);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("layer.yaml"),
            format!("name: {}\n", serde_json::to_string(name).unwrap()),
        )
        .unwrap();
        path
    }

    fn current(&self, format: LayerManifestFormat, source: &str) -> PathBuf {
        let path = self.app.join(format.filename());
        fs::write(&path, source).unwrap();
        path
    }

    fn run(&self, arguments: &[&str]) -> Output {
        run(&self.app, arguments, &[])
    }
}

// File captures avoid pipe backpressure; the bound and Drop guard also make a
// prompt regression fail rather than leaving a child or the test runner blocked.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn run(directory: &Path, arguments: &[&str], environment: &[(&str, &str)]) -> Output {
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let stderr = tempfile::NamedTempFile::new().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_hydragen"));
    command
        .current_dir(directory)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(stdout.reopen().unwrap())
        .stderr(stderr.reopen().unwrap());
    for key in [
        "CI",
        "NO_COLOR",
        "CLICOLOR",
        "CLICOLOR_FORCE",
        "FORCE_COLOR",
        "TERM",
    ] {
        command.env_remove(key);
    }
    command.envs(environment.iter().copied());
    let mut child = Running(command.spawn().unwrap());
    let status = wait(&mut child);
    Output {
        status,
        stdout: fs::read(stdout.path()).unwrap(),
        stderr: fs::read(stderr.path()).unwrap(),
    }
}

fn wait(child: &mut Running) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            return status;
        }
        assert!(Instant::now() < deadline, "add child timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn assert_plain(output: &Output) {
    for bytes in [&output.stdout, &output.stderr] {
        let message = text(bytes);
        for unwanted in ['\x1b', '✓', '✕', '▲', '┌', '│', '└', '⠀'] {
            assert!(!message.contains(unwanted), "{message}");
        }
        assert!(!message.contains("Hydragen"), "{message}");
    }
}

#[test]
fn actual_batch_edits_preserve_every_format_and_report_mixed_outcomes_once() {
    for format in LayerManifestFormat::ALL {
        let fixture = Fixture::new();
        fixture.layer("old", "old");
        fixture.layer("unrequested", "unrequested");
        fixture.layer("Shared Rules @team", "@example/shared-rules");
        fixture.layer("tooling", "tooling");
        let (source, preserved) = match format {
            LayerManifestFormat::Yaml | LayerManifestFormat::Yml => (
                "# app metadata\nname: app\ndependencies:\n  - ../old # keep dependency comment\n  - ../unrequested\n# keep custom attached\ncustom: {greeting: 'héllo', count: 7}\n",
                "# keep custom attached\ncustom: {greeting: 'héllo', count: 7}\n",
            ),
            LayerManifestFormat::Toml => (
                "# app metadata\nname = 'app'\ndependencies = [\n  '../old', # keep dependency comment\n  '../unrequested',\n]\n# keep custom attached\n[custom]\ngreeting = 'héllo'\ncount = 7\n",
                "# keep custom attached\n[custom]\ngreeting = 'héllo'\ncount = 7\n",
            ),
            LayerManifestFormat::Json => (
                "{\n  \"name\": \"app\",\n  \"dependencies\": [\"../old\", \"../unrequested\"],\n  \"custom\": {\"greeting\": \"héllo\", \"count\": 7}\n}\n",
                "\"custom\": {\"greeting\": \"héllo\", \"count\": 7}",
            ),
            LayerManifestFormat::Jsonc => (
                "{\n  // app metadata\n  \"name\": \"app\",\n  \"dependencies\": [\"../old\", /* keep dependency comment */ \"../unrequested\",],\n  // keep custom attached\n  \"custom\": {\"greeting\": \"héllo\", \"count\": 7},\n}\n",
                "// keep custom attached\n  \"custom\": {\"greeting\": \"héllo\", \"count\": 7},",
            ),
        };
        let path = fixture.current(format, source);
        fs::write(fixture.app.join("content.txt"), "project content").unwrap();
        let output = fixture.run(&[
            "add",
            "../Shared Rules @team",
            "../old",
            "../tooling",
            "../Shared Rules @team",
            "../old",
        ]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(
            text(&output.stdout),
            "Dependencies updated for app.\n\nAdded (2)\n  @example/shared-rules\n  tooling\n\nAlready present (1)\n  old\n"
        );
        assert!(output.stderr.is_empty());
        assert_plain(&output);
        let edited = fs::read_to_string(path).unwrap();
        assert!(edited.contains(preserved), "{format}: {edited}");
        if format != LayerManifestFormat::Json {
            assert!(edited.contains("keep dependency comment"));
        }
        let loaded = hydragen::discover(&fixture.app).unwrap().load().unwrap();
        assert_eq!(
            loaded.document,
            json!({"name":"app", "dependencies":["../old","../unrequested","../Shared Rules @team","../tooling"], "custom":{"greeting":"héllo","count":7}})
        );
        assert_eq!(
            fs::read_to_string(fixture.app.join("content.txt")).unwrap(),
            "project content"
        );
        assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 2);
        assert_eq!(
            fs::read_to_string(fixture._root.path().join("tooling/layer.yaml")).unwrap(),
            "name: \"tooling\"\n"
        );
    }
}

#[test]
fn single_results_cover_scoped_names_repeats_aliases_and_unrequested_declarations() {
    for (label, name, destination, requests, unrelated) in [
        ("single", "rules", "app", "single", false),
        (
            "scoped",
            "@example/shared-rules",
            "@example/project",
            "single",
            false,
        ),
        ("repeated", "rules", "app", "repeated", false),
        ("aliases", "rules", "app", "aliases", false),
        (
            "unrequested",
            "@example/shared-rules",
            "@example/project",
            "aliases",
            true,
        ),
    ] {
        let fixture = Fixture::new();
        let rules = fixture.layer("Shared Rules @team", name);
        fixture.layer("unrequested", "unrequested");
        let mut document = json!({"name": destination});
        if unrelated {
            document["dependencies"] = json!(["../unrequested", "unrequested-published@1.2.3"]);
        }
        let path = fixture.current(LayerManifestFormat::Json, &document.to_string());
        let reference = "../Shared Rules @team/./";
        let mut arguments = vec!["add", reference];
        if requests == "repeated" {
            arguments.extend([reference, reference]);
        } else if requests == "aliases" {
            arguments.extend(["../Shared Rules @team", rules.to_str().unwrap(), reference]);
        }
        let output = fixture.run(&arguments);
        assert!(output.status.success(), "{label}: {}", text(&output.stderr));
        assert_eq!(
            text(&output.stdout),
            format!("Added {name} to {destination}.\n"),
            "{label}"
        );
        assert!(output.stderr.is_empty());
        assert_plain(&output);
        let mut expected = if unrelated {
            vec!["../unrequested", "unrequested-published@1.2.3"]
        } else {
            vec![]
        };
        expected.push(reference);
        document["dependencies"] = json!(expected);
        assert_eq!(
            hydragen::discover(&fixture.app)
                .unwrap()
                .load()
                .unwrap()
                .document,
            document
        );
        let original = fs::read(&path).unwrap();
        let before = fs::metadata(&path).unwrap();
        let output = fixture.run(&arguments);
        assert!(output.status.success(), "{label}: {}", text(&output.stderr));
        assert_eq!(
            text(&output.stdout),
            format!("No changes needed: {name} is already a dependency of {destination}.\n"),
            "{label}"
        );
        assert!(output.stderr.is_empty());
        assert_plain(&output);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(
            fs::metadata(&path).unwrap().modified().unwrap(),
            before.modified().unwrap()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
        }
        assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
        assert_eq!(
            fs::read_to_string(rules.join("layer.yaml")).unwrap(),
            format!("name: {}\n", serde_json::to_string(name).unwrap())
        );
    }
}

#[test]
fn two_effective_results_keep_grouped_output_even_with_one_name_in_each_group() {
    for (existing, expected) in [
        (
            "[]",
            "Dependencies updated for app.\n\nAdded (2)\n  tooling\n  rules\n",
        ),
        (
            "[../rules, ../tooling, ../unrequested]",
            "No changes needed for app.\n\nAlready present (2)\n  tooling\n  rules\n",
        ),
        (
            "[../rules, ../unrequested]",
            "Dependencies updated for app.\n\nAdded (1)\n  tooling\n\nAlready present (1)\n  rules\n",
        ),
    ] {
        let fixture = Fixture::new();
        for name in ["rules", "tooling", "unrequested"] {
            fixture.layer(name, name);
        }
        fixture.current(
            LayerManifestFormat::Yaml,
            &format!("name: app\ndependencies: {existing}\n"),
        );
        let output = fixture.run(&["add", "../tooling", "../rules", "../tooling"]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(text(&output.stdout), expected);
        assert!(output.stderr.is_empty());
        assert_plain(&output);
        assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
    }
}

#[test]
fn absent_and_empty_arrays_are_editable_in_every_format() {
    for format in LayerManifestFormat::ALL {
        for empty in [false, true] {
            let fixture = Fixture::new();
            fixture.layer("rules", "rules");
            let mut document = json!({"name":"app", "custom":{"keep":true}});
            if empty {
                document["dependencies"] = json!([]);
            }
            let source = match format {
                LayerManifestFormat::Yaml | LayerManifestFormat::Yml => {
                    format!(
                        "name: app\n{}custom: {{keep: true}}\n",
                        if empty { "dependencies: []\n" } else { "" }
                    )
                }
                LayerManifestFormat::Toml => format!(
                    "name = 'app'\n{}[custom]\nkeep = true\n",
                    if empty { "dependencies = []\n" } else { "" }
                ),
                _ => serde_json::to_string_pretty(&document).unwrap(),
            };
            fixture.current(format, &source);
            let output = fixture.run(&["add", "../rules", "../rules"]);
            assert!(output.status.success(), "{}", text(&output.stderr));
            assert!(!text(&output.stdout).contains("Already present"));
            assert_eq!(text(&output.stdout), "Added rules to app.\n");
            document["dependencies"] = json!(["../rules"]);
            assert_eq!(
                hydragen::discover(&fixture.app)
                    .unwrap()
                    .load()
                    .unwrap()
                    .document,
                document
            );
            assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
        }
    }
}

#[test]
fn noop_keeps_source_inode_and_uneditable_aliases_and_lists_only_requested_names() {
    let fixture = Fixture::new();
    let rules = fixture.layer("rules", "rules");
    fixture.layer("other", "other");
    let source = "name: app\ndependencies: &deps [../rules, ../other] # keep\ncustom: *deps\n";
    let path = fixture.current(LayerManifestFormat::Yaml, source);
    let before = fs::metadata(&path).unwrap();
    let absolute = rules.to_str().unwrap();
    let output = fixture.run(&["add", absolute, "../rules", absolute]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        text(&output.stdout),
        "No changes needed: rules is already a dependency of app.\n"
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    assert_eq!(
        fs::metadata(&path).unwrap().modified().unwrap(),
        before.modified().unwrap()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
    }
    assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
}

#[test]
fn nested_invocation_crosses_git_boundary_but_uses_layer_directory_as_base() {
    let fixture = Fixture::new();
    fixture.layer("rules", "rules");
    fixture.current(LayerManifestFormat::Yaml, "name: app\n");
    let nested = fixture.app.join("src/deep");
    fs::create_dir_all(nested.join(".git")).unwrap();
    let output = run(&nested, &["--non-interactive", "add", "../rules"], &[]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(
        hydragen::discover(&fixture.app)
            .unwrap()
            .load()
            .unwrap()
            .document["dependencies"],
        json!(["../rules"])
    );
    let output = run(&nested, &["add", "../rules", "--non-interactive"], &[]);
    assert!(output.status.success());
    assert!(text(&output.stdout).contains("No changes needed"));
    assert_eq!(fs::read_dir(nested).unwrap().count(), 1);
}

#[test]
fn independent_failures_are_grouped_in_source_order_with_base_and_no_partial_update() {
    let fixture = Fixture::new();
    fixture.layer("valid", "valid");
    let invalid = fixture.layer("invalid", "invalid");
    fs::write(invalid.join("layer.yaml"), "name: [\n").unwrap();
    let original = "name: app\n";
    let path = fixture.current(LayerManifestFormat::Yaml, original);
    let output = fixture.run(&[
        "add",
        "../valid",
        "../missing",
        "../invalid",
        "bare-name",
        "shared@1.2.0",
    ]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = text(&output.stderr);
    assert!(
        error.starts_with("Error: Dependency checks failed (4 errors).\n"),
        "{error}"
    );
    let mut previous = 0;
    for phrase in [
        "Source '../missing'",
        "Source '../invalid'",
        "Source 'bare-name'",
        "Source 'shared@1.2.0'",
    ] {
        let index = error
            .find(phrase)
            .unwrap_or_else(|| panic!("{phrase}: {error}"));
        assert!(index > previous);
        previous = index;
    }
    diagnostic_body_support::assert_verification(
        &error,
        4,
        &[
            "Source '../missing'",
            "Source '../invalid'",
            "Source 'bare-name'",
            "Source 'shared@1.2.0'",
        ],
    );
    assert!(
        !error.contains("Paths are relative"),
        "same-directory base is noise: {error}"
    );
    assert!(
        !error.contains(&invalid.display().to_string()),
        "known selection should use supplied spelling: {error}"
    );
    assert!(error.contains("./ or ../"));
    assert!(error.contains("Adding published layers is not supported yet"));
    assert!(error.contains("No files were changed"));
    assert!(!error.contains("--name"));
    assert_eq!(error.matches("Error:").count(), 1);
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
    assert_plain(&output);
}

#[test]
fn existing_failures_and_conflicts_identify_their_sources_without_upgrade_advice() {
    for existing in ["../missing", "rules@1.2.0"] {
        let fixture = Fixture::new();
        fixture.layer("rules", "rules");
        fixture.layer("conflict", "app");
        let original = format!("name: app\ndependencies: ['{existing}']\n");
        let path = fixture.current(LayerManifestFormat::Yaml, &original);
        let output = fixture.run(&["add", "../rules", "../conflict"]);
        assert!(!output.status.success());
        let error = text(&output.stderr);
        assert!(
            error.contains(&format!("Existing dependency '{existing}'")),
            "{error}"
        );
        assert!(error.contains("Source '../conflict'"));
        assert!(error.contains("Conflicting source: Current layer"));
        assert!(error.contains("Current layer path:"));
        assert!(error.contains("Requested source path:"));
        if existing == "../missing" {
            assert!(
                error.find("Existing dependency '../missing'").unwrap()
                    < error.find("Source '../conflict'").unwrap()
            );
        } else {
            assert!(error.contains("existing declarations are not replaced"));
        }
        assert!(!error.contains("hydragen upgrade"));
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }
}

#[test]
fn discovery_warnings_are_once_per_directory_before_selected_failure_without_fallback() {
    let fixture = Fixture::new();
    fixture.current(LayerManifestFormat::Yaml, "name: app\n");
    fixture.current(LayerManifestFormat::Json, "{\"name\":\"ignored-current\"}");
    let rules = fixture.layer("rules", "rules");
    fs::write(rules.join("layer.json"), "{\"name\":\"ignored\"}").unwrap();
    for malformed in [false, true] {
        fs::write(
            rules.join("layer.yaml"),
            if malformed { "[" } else { "name: rules\n" },
        )
        .unwrap();
        let output = fixture.run(&["add", "../rules", rules.to_str().unwrap(), "../rules"]);
        assert_eq!(output.status.success(), !malformed);
        let error = text(&output.stderr);
        assert_eq!(
            error.matches("Warning: Multiple layer manifests").count(),
            2,
            "{error}"
        );
        assert_eq!(error.matches("layer.yaml, layer.json.").count(), 2);
        assert!(error.contains("layer.yaml takes priority"));
        if malformed {
            assert!(error.rfind("Warning:").unwrap() < error.find("Error:").unwrap());
            assert!(output.stdout.is_empty());
            diagnostic_body_support::assert_verification(
                &error[error.find("Error:").unwrap()..],
                4,
                &[
                    "Existing dependency '../rules'",
                    "Source '../rules' (argument 1)",
                    &format!("Source '{}'", rules.display()),
                    "Source '../rules' (argument 3)",
                ],
            );
        }
        assert!(!text(&output.stdout).contains("ignored"));
    }
}

#[test]
fn nearest_occupied_selection_is_not_skipped_for_valid_parent_or_lower_priority() {
    for occupied_directory in [false, true] {
        let fixture = Fixture::new();
        let parent = fixture.current(LayerManifestFormat::Yaml, "name: app\n");
        let nested = fixture.app.join("nested");
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("layer.json"), "{\"name\":\"valid-lower\"}").unwrap();
        if occupied_directory {
            fs::create_dir(nested.join("layer.yaml")).unwrap();
        } else {
            fs::write(nested.join("layer.yaml"), "[").unwrap();
        }
        let output = run(&nested, &["add", "../missing"], &[]);
        assert!(!output.status.success());
        let error = text(&output.stderr);
        assert!(error.contains(&nested.join("layer.yaml").display().to_string()));
        assert!(
            !error.contains("source argument"),
            "prerequisite must fail alone: {error}"
        );
        assert_eq!(error.matches("Warning:").count(), 1);
        assert_eq!(fs::read_to_string(parent).unwrap(), "name: app\n");
    }
}

#[test]
fn clap_help_version_and_usage_do_not_discover_or_prompt_even_with_duplicates() {
    let fixture = Fixture::new();
    fixture.current(LayerManifestFormat::Yaml, "[");
    fixture.current(LayerManifestFormat::Json, "{}");
    for arguments in [
        vec!["add", "--help"],
        vec!["--help"],
        vec!["--version"],
        vec!["add"],
        vec!["add", "--path", "../rules"],
    ] {
        let output = fixture.run(&arguments);
        assert_eq!(
            output.status.success(),
            arguments
                .iter()
                .any(|arg| ["--help", "--version"].contains(arg))
        );
        assert!(!text(&output.stderr).contains("Warning:"));
        assert!(!text(&output.stderr).contains("Could not add dependencies"));
        assert_plain(&output);
    }
    assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 2);
}

#[test]
fn captured_output_stays_plain_even_with_force_and_interaction_controls() {
    for environment in [
        vec![("CI", "1"), ("CLICOLOR_FORCE", "1")],
        vec![("CI", "false"), ("FORCE_COLOR", "1")],
        vec![("TERM", "dumb"), ("CLICOLOR_FORCE", "1")],
        vec![("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")],
    ] {
        let fixture = Fixture::new();
        fixture.layer("rules", "rules");
        fixture.current(LayerManifestFormat::Yaml, "name: app\n");
        let output = run(
            &fixture.app,
            &["add", "../rules", "--non-interactive"],
            &environment,
        );
        assert!(output.status.success());
        assert_plain(&output);
        assert_eq!(text(&output.stdout), "Added rules to app.\n");
        assert!(output.stderr.is_empty());
        let output = run(
            &fixture.app,
            &["--non-interactive", "add", "../rules", "../rules"],
            &environment,
        );
        assert!(output.status.success());
        assert_plain(&output);
        assert_eq!(
            text(&output.stdout),
            "No changes needed: rules is already a dependency of app.\n"
        );
        assert!(output.stderr.is_empty());
        let output = run(&fixture.app, &["add", "../missing"], &environment);
        assert!(!output.status.success());
        assert_plain(&output);
    }
}

#[test]
fn unsafe_edit_refuses_before_staging_but_noop_does_not_invoke_editor() {
    let fixture = Fixture::new();
    fixture.layer("rules", "rules");
    fixture.layer("new", "new");
    let original = "name: app\ndependencies: &deps [../rules]\ncustom: *deps\n";
    let path = fixture.current(LayerManifestFormat::Yaml, original);
    let output = fixture.run(&["add", "../new"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("safely edit"));
    assert!(text(&output.stderr).contains("No files were changed"));
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
}

#[test]
fn invalid_layer_names_are_content_errors_without_init_flag_guidance() {
    for current in [false, true] {
        let fixture = Fixture::new();
        fixture.layer("rules", if current { "rules" } else { "Invalid-Name" });
        let original = if current {
            "name: Invalid-Name\n"
        } else {
            "name: app\n"
        };
        let path = fixture.current(LayerManifestFormat::Yaml, original);
        let output = fixture.run(&["add", "../rules"]);
        assert!(!output.status.success());
        let error = text(&output.stderr);
        assert!(
            error.contains("Metadata field 'name' is invalid"),
            "{error}"
        );
        assert!(error.contains("lowercase"));
        assert!(!error.contains("--name"));
        assert!(!error.contains("non-blank"));
        assert_eq!(fs::read_to_string(path).unwrap(), original);
        assert!(output.stdout.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn warning_delivery_is_best_effort_and_cannot_turn_completed_add_into_failure() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    let fixture = Fixture::new();
    let rules = fixture.layer("rules", "rules");
    fs::write(rules.join("layer.json"), "{}").unwrap();
    fixture.current(LayerManifestFormat::Yaml, "name: app\n");
    let stdout = tempfile::NamedTempFile::new().unwrap();
    let (reader, writer) = UnixStream::pair().unwrap();
    drop(reader);
    let mut child = Running(
        Command::new(env!("CARGO_BIN_EXE_hydragen"))
            .current_dir(&fixture.app)
            .args(["add", "../rules"])
            .stdin(Stdio::null())
            .stdout(stdout.reopen().unwrap())
            .stderr(Stdio::from(OwnedFd::from(writer)))
            .spawn()
            .unwrap(),
    );
    assert!(wait(&mut child).success());
    assert!(
        fs::read_to_string(stdout.path())
            .unwrap()
            .starts_with("Added rules to app.")
    );
    assert_eq!(
        hydragen::discover(&fixture.app)
            .unwrap()
            .load()
            .unwrap()
            .document["dependencies"],
        json!(["../rules"])
    );
}

#[cfg(unix)]
#[test]
fn symlinked_current_is_refused_but_dependency_links_and_directory_aliases_are_readable() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let external = fixture._root.path().join("external.yaml");
    fs::write(&external, "name: app\n").unwrap();
    symlink(&external, fixture.app.join("layer.yaml")).unwrap();
    let output = fixture.run(&["add", "../rules"]);
    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("symlinked current layer manifest"));
    assert!(
        fs::symlink_metadata(fixture.app.join("layer.yaml"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_to_string(&external).unwrap(), "name: app\n");
    fs::remove_file(fixture.app.join("layer.yaml")).unwrap();
    fixture.current(LayerManifestFormat::Yaml, "name: app\n");
    let rules = fixture.layer("rules", "rules");
    let source = fixture._root.path().join("rules-source.yaml");
    fs::rename(rules.join("layer.yaml"), &source).unwrap();
    symlink(&source, rules.join("layer.yaml")).unwrap();
    fs::write(rules.join("layer.json"), "{}").unwrap();
    symlink(&rules, fixture._root.path().join("rules-link")).unwrap();
    let output = fixture.run(&["add", "../rules-link", "../rules"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "Added rules to app.\n");
    assert_eq!(text(&output.stderr).matches("Warning:").count(), 1);
    assert_eq!(
        hydragen::discover(&fixture.app)
            .unwrap()
            .load()
            .unwrap()
            .document["dependencies"],
        json!(["../rules-link"])
    );
    let path = fixture.app.join("layer.yaml");
    let before = fs::metadata(&path).unwrap();
    let original = fs::read(&path).unwrap();
    let repeated = fixture.run(&["add", "../rules", "../rules-link", rules.to_str().unwrap()]);
    assert!(repeated.status.success(), "{}", text(&repeated.stderr));
    assert_eq!(
        text(&repeated.stdout),
        "No changes needed: rules is already a dependency of app.\n"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(
        fs::metadata(&path).unwrap().modified().unwrap(),
        before.modified().unwrap()
    );
    use std::os::unix::fs::MetadataExt;
    assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
    assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn closed_stdout_reports_already_updated_after_actual_child_replacement() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    for noop in [false, true] {
        let fixture = Fixture::new();
        fixture.layer("rules", "rules");
        fixture.current(
            LayerManifestFormat::Yaml,
            if noop {
                "name: app\ndependencies: [../rules]\n"
            } else {
                "name: app\n"
            },
        );
        let (reader, writer) = UnixStream::pair().unwrap();
        drop(reader);
        let stderr = tempfile::NamedTempFile::new().unwrap();
        let mut child = Running(
            Command::new(env!("CARGO_BIN_EXE_hydragen"))
                .current_dir(&fixture.app)
                .args(["add", "../rules"])
                .stdin(Stdio::null())
                .stdout(Stdio::from(OwnedFd::from(writer)))
                .stderr(stderr.reopen().unwrap())
                .spawn()
                .unwrap(),
        );
        assert!(!wait(&mut child).success());
        let error = fs::read_to_string(stderr.path()).unwrap();
        assert_eq!(error.matches("Error:").count(), 1, "{error}");
        assert!(error.contains("Completion output failed"));
        assert_eq!(error.contains("was already updated"), !noop);
        assert_eq!(error.contains("was not updated"), noop);
        assert!(!error.contains("No files were changed"));
        assert_eq!(
            hydragen::discover(&fixture.app)
                .unwrap()
                .load()
                .unwrap()
                .document["dependencies"],
            json!(["../rules"])
        );
    }
}
