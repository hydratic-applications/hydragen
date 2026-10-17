use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Barrier};

use hydragen::ManifestFormat;
use serde_json::{Value, json};
use tempfile::tempdir;

fn command(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hydragen"));
    command.current_dir(directory).stdin(Stdio::null());
    command
}

fn init(directory: &Path, arguments: &[&str]) -> Output {
    command(directory)
        .arg("init")
        .args(arguments)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Initialised layer:"), "{stdout}");
    assert!(stdout.contains("layer.yaml"), "{stdout}");
    assert!(!output.stdout.contains(&b'\x1b'));
}

fn assert_failure(output: &Output, message: &str) {
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(message), "{stderr}");
    assert!(!output.stderr.contains(&b'\x1b'));
}

#[test]
fn non_interactive_override_is_accepted_before_and_after_init() {
    for arguments in [
        vec!["--non-interactive", "init", "--name", "example"],
        vec!["init", "--name", "example", "--non-interactive"],
    ] {
        let directory = tempdir().unwrap();
        assert_success(&command(directory.path()).args(arguments).output().unwrap());
        assert_eq!(read_manifest(directory.path()), json!({"name": "example"}));
    }
}

#[test]
fn ci_and_explicit_override_use_the_directory_name_without_prompts() {
    for ci in ["true", "1", "false", "0"] {
        let directory = tempdir().unwrap();
        let output = command(directory.path())
            .env("CI", ci)
            .args(["init", "--non-interactive"])
            .output()
            .unwrap();
        assert_success(&output);
        assert_eq!(
            read_manifest(directory.path()),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
    }
}

#[test]
fn help_and_version_bypass_input_collection() {
    for arguments in [vec!["--help"], vec!["init", "--help"], vec!["--version"]] {
        let directory = tempdir().unwrap();
        let output = command(directory.path())
            .env("CI", "true")
            .args(arguments)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(!output.stdout.contains(&b'\x1b'));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn a_live_input_pipe_is_not_read_or_awaited() {
    use std::time::{Duration, Instant};

    let directory = tempdir().unwrap();
    let mut child = command(directory.path())
        .arg("init")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _input = child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Non-interactive init waited for input");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_success(&child.wait_with_output().unwrap());
    assert_eq!(
        read_manifest(directory.path()),
        json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
    );
}

#[cfg(unix)]
#[test]
fn a_result_output_failure_does_not_claim_the_manifest_was_not_created() {
    use std::io::Write;

    let directory = tempdir().unwrap();
    // Delay exec until the parent has closed the result pipe's reader. This
    // deterministically exercises a reporting failure after manifest creation.
    let mut child = Command::new("/bin/sh")
        .args([
            "-c",
            "read -r ready\nexec \"$@\"",
            "output-failure",
            env!("CARGO_BIN_EXE_hydragen"),
            "init",
            "--name",
            "example",
        ])
        .current_dir(directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    child.stdin.take().unwrap().write_all(b"ready\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_failure(&output, "was created");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("No files were changed"));
    assert_eq!(read_manifest(directory.path()), json!({"name": "example"}));
}

fn read_manifest(directory: &Path) -> Value {
    // Independent parser validation: do not rely on Hydragen's matching reader.
    independently_read_format(directory, ManifestFormat::Yaml)
}

#[test]
fn initialises_without_git_and_creates_only_the_manifest() {
    let directory = tempdir().unwrap();
    assert_success(&init(directory.path(), &["--name", "example"]));
    assert_eq!(read_manifest(directory.path()), json!({"name": "example"}));
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn accepts_independent_optional_fields() {
    for (arguments, expected) in [
        (
            vec!["--author", "Team"],
            json!({"name": "example", "author": "Team"}),
        ),
        (
            vec!["--description", "Layer"],
            json!({"name": "example", "description": "Layer"}),
        ),
        (
            vec!["--author", "Team", "--description", "Layer"],
            json!({"name": "example", "author": "Team", "description": "Layer"}),
        ),
    ] {
        let directory = tempdir().unwrap();
        let mut flags = vec!["--name", "example"];
        flags.extend(arguments);
        assert_success(&init(directory.path(), &flags));
        assert_eq!(read_manifest(directory.path()), expected);
    }
}

#[test]
fn preserves_whitespace_escaping_and_unicode() {
    let directory = tempdir().unwrap();
    let text = "  \"quoted\" \\path\n配置 🦀  ";
    assert_success(&init(
        directory.path(),
        &["--name", text, "--author", text, "--description", text],
    ));
    assert_eq!(
        read_manifest(directory.path()),
        json!({"name": text, "author": text, "description": text})
    );
}

#[test]
fn omitted_name_defaults_to_the_current_directory() {
    for arguments in [
        vec![],
        vec!["--author", "Team"],
        vec!["--description", "Layer"],
        vec!["--author", "Team", "--description", "Layer"],
    ] {
        let directory = tempdir().unwrap();
        let output = init(directory.path(), &arguments);
        assert_success(&output);
        let result = read_manifest(directory.path());
        assert_eq!(
            result["name"],
            directory.path().file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(
            result.get("author").is_some(),
            arguments.contains(&"--author")
        );
        assert_eq!(
            result.get("description").is_some(),
            arguments.contains(&"--description")
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[test]
fn supplied_blank_metadata_is_an_error_without_writes() {
    for flag in ["--name", "--author", "--description"] {
        for value in ["", " \t\n", "\u{00a0}\u{2003}", "\u{85}"] {
            let directory = tempdir().unwrap();
            let arguments = if flag == "--name" {
                vec![flag, value]
            } else {
                vec!["--name", "example", flag, value]
            };
            assert_failure(&init(directory.path(), &arguments), flag);
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
        }
    }
}

#[test]
fn metadata_flags_still_require_values_when_used() {
    for flag in ["--name", "--author", "--description"] {
        let directory = tempdir().unwrap();
        assert_failure(&init(directory.path(), &[flag]), flag);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn help_explains_defaults_and_optional_metadata_without_writes() {
    let directory = tempdir().unwrap();
    for arguments in [vec!["--help"], vec!["init", "--help"]] {
        let output = command(directory.path()).args(&arguments).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(help.contains("Usage:"));
        if arguments.len() == 2 {
            for expected in [
                "Optional name override",
                "current directory name",
                "Optional person",
                "Optional explanation",
                "hydragen init --name shared-agent-rules",
                "--author \"Example team\"",
                "--format <FORMAT>",
                "yaml, yml, toml, json, jsonc",
                "Format defaults to yaml (layer.yaml)",
                "Any input flag",
            ] {
                assert!(help.contains(expected), "{help}");
            }
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn directory_name_default_preserves_spaces_and_unicode() {
    let parent = tempdir().unwrap();
    let directory = parent.path().join(" My Project 配置 🦀 ");
    fs::create_dir(&directory).unwrap();
    assert_success(&init(&directory, &[]));
    assert_eq!(
        read_manifest(&directory),
        json!({"name": " My Project 配置 🦀 "})
    );
}

#[cfg(unix)]
#[test]
fn an_unusable_directory_name_needs_an_explicit_override() {
    let parent = tempdir().unwrap();
    let directory = parent.path().join("   ");
    fs::create_dir(&directory).unwrap();
    assert_failure(&init(&directory, &[]), "Supply --name explicitly");
    assert!(!directory.join("layer.yaml").exists());
    assert_success(&init(&directory, &["--name", "explicit"]));
    assert_eq!(read_manifest(&directory), json!({"name": "explicit"}));
}

#[test]
fn initialises_at_git_roots_without_modifying_existing_files() {
    for git_directory in [true, false] {
        let directory = tempdir().unwrap();
        let marker = directory.path().join(".git");
        let git_file = if git_directory {
            fs::create_dir(&marker).unwrap();
            marker.join("config")
        } else {
            marker.clone()
        };
        fs::write(&git_file, b"existing git metadata\n").unwrap();
        fs::create_dir(directory.path().join("src")).unwrap();
        let source = directory.path().join("src/main.txt");
        fs::write(&source, b"existing source\n").unwrap();
        fs::write(directory.path().join(".gitignore"), b"existing ignore\n").unwrap();
        assert_success(&init(directory.path(), &["--name", "example"]));
        assert_eq!(fs::read(&git_file).unwrap(), b"existing git metadata\n");
        assert_eq!(fs::read(source).unwrap(), b"existing source\n");
        assert_eq!(
            fs::read(directory.path().join(".gitignore")).unwrap(),
            b"existing ignore\n"
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 4);
    }
}

#[test]
fn refuses_nested_locations_and_identifies_the_repository_root() {
    for git_directory in [true, false] {
        let directory = tempdir().unwrap();
        let marker = directory.path().join(".git");
        if git_directory {
            fs::create_dir(marker).unwrap();
        } else {
            fs::write(marker, "gitdir: somewhere\n").unwrap();
        }
        let nested = directory.path().join("nested");
        fs::create_dir(&nested).unwrap();
        let output = init(&nested, &["--name", "example"]);
        assert_failure(&output, "repository root");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&*directory.path().canonicalize().unwrap().to_string_lossy())
        );
        assert!(!directory.path().join("layer.yaml").exists());
        assert_eq!(fs::read_dir(nested).unwrap().count(), 0);
    }
}

#[test]
fn a_nested_repository_uses_its_own_root() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join(".git")).unwrap();
    let nested = directory.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join(".git"), "gitdir: somewhere\n").unwrap();
    assert_success(&init(&nested, &["--name", "example"]));
    assert!(!directory.path().join("layer.yaml").exists());
}

#[test]
fn refuses_existing_manifests_even_when_malformed() {
    for original in [b"not valid JSON".as_slice(), b"{\"name\":\"existing\"}\n"] {
        let directory = tempdir().unwrap();
        let path = directory.path().join("layer.json");
        fs::write(&path, original).unwrap();
        let output = init(directory.path(), &["--name", "replacement"]);
        assert_failure(&output, "already exists");
        assert!(String::from_utf8_lossy(&output.stderr).contains("layer.json"));
        assert_eq!(fs::read(path).unwrap(), original);
    }
}

#[test]
fn repeated_init_leaves_the_manifest_unchanged() {
    let directory = tempdir().unwrap();
    assert_success(&init(directory.path(), &["--name", "first"]));
    let original = fs::read(directory.path().join("layer.yaml")).unwrap();
    assert_failure(
        &init(directory.path(), &["--name", "second"]),
        "already exists",
    );
    assert_eq!(
        fs::read(directory.path().join("layer.yaml")).unwrap(),
        original
    );
}

#[test]
fn a_directory_at_the_manifest_path_is_not_replaced() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("layer.json");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("keep.txt"), "keep").unwrap();
    assert_failure(
        &init(directory.path(), &["--name", "example"]),
        "layer.json",
    );
    assert_eq!(fs::read_to_string(path.join("keep.txt")).unwrap(), "keep");
}

#[cfg(unix)]
#[test]
fn existing_and_dangling_symlinks_are_not_followed_or_replaced() {
    use std::os::unix::fs::symlink;

    for target_exists in [true, false] {
        let directory = tempdir().unwrap();
        let target = directory.path().join("target");
        if target_exists {
            fs::write(&target, "keep").unwrap();
        }
        let path = directory.path().join("layer.json");
        symlink(&target, &path).unwrap();
        assert_failure(
            &init(directory.path(), &["--name", "example"]),
            "already exists",
        );
        assert_eq!(fs::read_link(path).unwrap(), target);
        if target_exists {
            assert_eq!(fs::read_to_string(target).unwrap(), "keep");
        } else {
            assert!(!target.exists());
        }
    }
}

#[test]
fn concurrent_init_has_one_winner_without_overwriting_it() {
    let directory = tempdir().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = ["first", "second"]
        .into_iter()
        .map(|name| {
            let directory = directory.path().to_owned();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                (name, init(&directory, &["--name", name]))
            })
        })
        .collect();
    let outputs: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        outputs
            .iter()
            .filter(|(_, output)| output.status.success())
            .count(),
        1
    );
    for (_, output) in &outputs {
        if output.status.success() {
            assert_success(output);
        } else {
            assert_failure(output, "already exists");
        }
    }
    let (winner, _) = outputs
        .iter()
        .find(|(_, output)| output.status.success())
        .unwrap();
    assert_eq!(read_manifest(directory.path())["name"], *winner);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn runs_without_stdin_git_or_service_configuration() {
    let directory = tempdir().unwrap();
    let output = command(directory.path())
        .env_clear()
        .env("PATH", directory.path())
        .args(["init", "--name", "example"])
        .output()
        .unwrap();
    assert_success(&output);
    assert_eq!(read_manifest(directory.path()), json!({"name": "example"}));
}

#[cfg(unix)]
#[test]
fn inspection_errors_are_reported_without_writes() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().unwrap();
    symlink(".git", directory.path().join(".git")).unwrap();
    let output = init(directory.path(), &["--name", "example"]);
    assert_failure(&output, "inspect repository marker");
    assert!(String::from_utf8_lossy(&output.stderr).contains(".git"));
    assert!(!directory.path().join("layer.yaml").exists());
}

#[cfg(unix)]
#[test]
fn write_failures_report_the_path_without_false_success() {
    let directory = tempdir().unwrap();
    // Limit only the child process. Ignoring SIGXFSZ makes write return an error
    // instead of killing the CLI, so this exercises its real write-error branch.
    let output = Command::new("/bin/sh")
        .args([
            "-c",
            "trap '' XFSZ\nulimit -f 0\nexec \"$1\" init --name example",
            "write-failure",
            env!("CARGO_BIN_EXE_hydragen"),
        ])
        .current_dir(directory.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_failure(&output, "write manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("layer.yaml"));
    assert!(stderr.contains("partial file may remain"));
    assert_eq!(fs::read(directory.path().join("layer.yaml")).unwrap(), b"");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

fn independently_read_format(directory: &Path, format: ManifestFormat) -> Value {
    let text = fs::read_to_string(directory.join(format.filename())).unwrap();
    assert!(text.ends_with('\n'));
    let document = match format {
        ManifestFormat::Yaml | ManifestFormat::Yml => serde_saphyr::from_str(&text).unwrap(),
        ManifestFormat::Toml => {
            let table: toml::Table = toml::from_str(&text).unwrap();
            serde_json::to_value(table).unwrap()
        }
        ManifestFormat::Json | ManifestFormat::Jsonc => serde_json::from_str(&text).unwrap(),
    };
    hydragen::manifest::schema::validate(&document).unwrap();
    document
}

#[test]
fn every_format_flag_alone_uses_directory_name_and_exact_filename() {
    for format in ManifestFormat::ALL {
        let directory = tempdir().unwrap();
        let output = init(directory.path(), &["--format", format.extension()]);
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(format.filename()), "{stdout}");
        assert!(!stdout.contains('\x1b'));
        assert_eq!(
            independently_read_format(directory.path(), format),
            json!({"name": directory.path().file_name().unwrap().to_str().unwrap()})
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[test]
fn all_formats_preserve_metadata_and_omit_unsupplied_fields() {
    for format in ManifestFormat::ALL {
        for author in [false, true] {
            for description in [false, true] {
                let directory = tempdir().unwrap();
                let text = "  true # \"quoted\" \\path\n配置 🦀  ";
                let mut args = vec!["--format", format.extension(), "--name", text];
                if author {
                    args.extend(["--author", text]);
                }
                if description {
                    args.extend(["--description", text]);
                }
                let output = init(directory.path(), &args);
                assert!(output.status.success(), "{output:?}");
                assert!(output.stderr.is_empty(), "{output:?}");
                let document = independently_read_format(directory.path(), format);
                assert_eq!(document["name"], text);
                assert_eq!(document.get("author"), author.then(|| json!(text)).as_ref());
                assert_eq!(
                    document.get("description"),
                    description.then(|| json!(text)).as_ref()
                );
                assert_eq!(
                    document.as_object().unwrap().len(),
                    1 + usize::from(author) + usize::from(description)
                );
                assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
            }
        }
    }
}

#[test]
fn all_formats_validate_preserved_unicode_directory_defaults_and_optional_combinations() {
    for format in ManifestFormat::ALL {
        for author in [false, true] {
            for description in [false, true] {
                let parent = tempdir().unwrap();
                let directory = parent.path().join(" My Project 配置 🦀 \u{feff} ");
                fs::create_dir(&directory).unwrap();
                let mut args = vec!["--format", format.extension()];
                let mut expected = json!({"name": " My Project 配置 🦀 \u{feff} "});
                if author {
                    args.extend(["--author", "Team"]);
                    expected["author"] = json!("Team");
                }
                if description {
                    args.extend(["--description", "Layer"]);
                    expected["description"] = json!("Layer");
                }
                let output = init(&directory, &args);
                assert!(output.status.success(), "{output:?}");
                assert!(output.stderr.is_empty(), "{output:?}");
                assert_eq!(independently_read_format(&directory, format), expected);
                assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
            }
        }
    }
}

#[test]
fn all_cli_codecs_preserve_schema_unicode_and_overflow_looking_strings() {
    for format in ManifestFormat::ALL {
        for value in [
            "\u{feff}",
            "\u{85}visible\u{2003}",
            "18446744073709551616",
            "0x1_0000_0000_0000_0000",
            "-9223372036854775809",
            "0B10000000000000000000000000000000000000000000000000000000000000000",
        ] {
            let directory = tempdir().unwrap();
            // Equals syntax permits a leading minus without changing clap policy.
            let name = format!("--name={value}");
            let author = format!("--author={value}");
            let description = format!("--description={value}");
            let output = init(
                directory.path(),
                &["--format", format.extension(), &name, &author, &description],
            );
            assert!(output.status.success(), "{format}: {value:?}: {output:?}");
            assert!(output.stderr.is_empty(), "{output:?}");
            assert_eq!(
                independently_read_format(directory.path(), format),
                json!({"name": value, "author": value, "description": value})
            );
            // The real loader must also preserve writer-quoted numeric-looking strings.
            assert_eq!(
                hydragen::discover(directory.path())
                    .unwrap()
                    .load()
                    .unwrap()
                    .document,
                json!({"name": value, "author": value, "description": value})
            );
        }
    }
}

#[test]
fn every_format_retains_exact_full_output_and_invalid_input_diagnostics() {
    for format in ManifestFormat::ALL {
        let directory = tempdir().unwrap();
        let output = init(
            directory.path(),
            &[
                "--format",
                format.extension(),
                "--name",
                "example",
                "--author",
                "Example team",
                "--description",
                "Shared rules",
            ],
        );
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty());
        let expected = match format {
            ManifestFormat::Yaml | ManifestFormat::Yml => {
                "name: example\nauthor: Example team\ndescription: Shared rules\n"
            }
            ManifestFormat::Toml => {
                "name = \"example\"\nauthor = \"Example team\"\ndescription = \"Shared rules\"\n"
            }
            ManifestFormat::Json | ManifestFormat::Jsonc => {
                "{\n  \"name\": \"example\",\n  \"author\": \"Example team\",\n  \"description\": \"Shared rules\"\n}\n"
            }
        };
        assert_eq!(
            fs::read_to_string(directory.path().join(format.filename())).unwrap(),
            expected
        );
        for field in ["--name", "--author", "--description"] {
            let invalid = tempdir().unwrap();
            let output = init(
                invalid.path(),
                &["--format", format.extension(), field, "\u{85}"],
            );
            assert_failure(
                &output,
                &format!("{field} must contain at least one non-whitespace character."),
            );
            assert_eq!(fs::read_dir(invalid.path()).unwrap().count(), 0);
        }
    }
}

#[test]
fn every_format_rejects_schema_blank_defaults_but_preserves_explicit_overrides() {
    for format in ManifestFormat::ALL {
        for name in ["   ", "\u{85}"] {
            let parent = tempdir().unwrap();
            let directory = parent.path().join(name);
            fs::create_dir(&directory).unwrap();
            let output = init(&directory, &["--format", format.extension()]);
            assert_failure(&output, "Supply --name explicitly");
            assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
            let output = init(
                &directory,
                &["--format", format.extension(), "--name", "explicit"],
            );
            assert!(output.status.success(), "{output:?}");
            assert!(output.stderr.is_empty());
            assert_eq!(
                independently_read_format(&directory, format),
                json!({"name": "explicit"})
            );
        }
    }
}

#[test]
fn explicit_json_retains_two_space_pretty_style_and_escaping() {
    let directory = tempdir().unwrap();
    let output = init(directory.path(), &["--format", "json", "--name", "a\"b\nc"]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(directory.path().join("layer.json")).unwrap(),
        "{\n  \"name\": \"a\\\"b\\nc\"\n}\n"
    );
}

#[test]
fn every_existing_and_requested_format_pair_refuses_without_reading_or_changes() {
    for existing in ManifestFormat::ALL {
        for requested in ManifestFormat::ALL {
            for is_directory in [false, true] {
                let directory = tempdir().unwrap();
                let path = directory.path().join(existing.filename());
                if is_directory {
                    fs::create_dir(&path).unwrap();
                    fs::write(path.join("keep"), "keep").unwrap();
                } else {
                    fs::write(&path, b"invalid UTF-8 \xff").unwrap();
                }
                let output = init(directory.path(), &["--format", requested.extension()]);
                assert_failure(&output, "already exists");
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(stderr.contains(existing.filename()));
                assert!(!stderr.contains("Warning:"));
                if is_directory {
                    assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "keep");
                } else {
                    assert_eq!(fs::read(path).unwrap(), b"invalid UTF-8 \xff");
                }
                assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn all_existing_and_requested_formats_preserve_regular_and_dangling_symlinks() {
    use std::os::unix::fs::symlink;
    for existing in ManifestFormat::ALL {
        for requested in ManifestFormat::ALL {
            for target_exists in [false, true] {
                let directory = tempdir().unwrap();
                let target = directory.path().join("target");
                if target_exists {
                    fs::write(&target, "keep").unwrap();
                }
                let path = directory.path().join(existing.filename());
                symlink(&target, &path).unwrap();
                let output = init(directory.path(), &["--format", requested.extension()]);
                assert_failure(&output, "already exists");
                assert!(!String::from_utf8_lossy(&output.stderr).contains("Warning:"));
                assert_eq!(fs::read_link(path).unwrap(), target);
                if target_exists {
                    assert_eq!(fs::read_to_string(target).unwrap(), "keep");
                } else {
                    assert!(!target.exists());
                }
                assert_eq!(
                    fs::read_dir(directory.path()).unwrap().count(),
                    1 + usize::from(target_exists)
                );
            }
        }
    }
}

#[test]
fn duplicate_refusal_warns_once_in_priority_order_before_error() {
    for selected_is_directory in [false, true] {
        let directory = tempdir().unwrap();
        for format in ManifestFormat::ALL.into_iter().rev() {
            let path = directory.path().join(format.filename());
            if selected_is_directory && format == ManifestFormat::Yaml {
                fs::create_dir(&path).unwrap();
            } else {
                fs::write(path, b"not parsed \xff").unwrap();
            }
        }
        let output = init(directory.path(), &["--format", "jsonc"]);
        assert_failure(&output, "already exists");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(stderr.matches("Warning:").count(), 1, "{stderr}");
        assert!(stderr.starts_with("Warning: Multiple layer manifests"));
        assert!(stderr.contains(&*directory.path().canonicalize().unwrap().to_string_lossy()));
        assert!(stderr.contains(
            "layer.yaml, layer.yml, layer.toml, layer.json, layer.jsonc. layer.yaml takes priority"
        ));
        assert!(
            stderr.contains(
                "priority: layer.yaml > layer.yml > layer.toml > layer.json > layer.jsonc"
            )
        );
        assert!(stderr.contains("not merged"));
        assert!(!stderr.contains("loaded"));
        assert!(!stderr.contains("Using"));
        assert!(stderr.find("Warning:").unwrap() < stderr.find("Error:").unwrap());
        for format in ManifestFormat::ALL {
            let path = directory.path().join(format.filename());
            if selected_is_directory && format == ManifestFormat::Yaml {
                assert!(path.is_dir());
            } else {
                assert_eq!(fs::read(path).unwrap(), b"not parsed \xff");
            }
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 5);
    }
}

#[test]
fn help_version_and_usage_errors_bypass_duplicate_discovery() {
    for (args, success) in [
        (vec!["--help"], true),
        (vec!["init", "--help"], true),
        (vec!["--version"], true),
        (vec!["init", "--unknown"], false),
        (vec!["init", "--format"], false),
        (vec!["init", "--name"], false),
    ] {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("layer.yaml"), "bad").unwrap();
        fs::write(directory.path().join("layer.json"), "bad").unwrap();
        let output = command(directory.path()).args(args).output().unwrap();
        assert_eq!(output.status.success(), success, "{output:?}");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Warning:"));
        assert!(!String::from_utf8_lossy(&output.stderr).contains("already exists"));
        assert!(!output.stderr.contains(&b'\x1b'));
        if success {
            assert!(output.stderr.is_empty());
        } else {
            assert!(output.stdout.is_empty());
        }
        assert_eq!(
            fs::read_to_string(directory.path().join("layer.yaml")).unwrap(),
            "bad"
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("layer.json")).unwrap(),
            "bad"
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}

#[test]
fn invalid_format_values_are_clap_errors_not_paths_or_prompts() {
    for value in [
        "",
        "YAML",
        "Yaml",
        "JSON",
        "json5",
        ".yaml",
        "layer.yaml",
        "../layer.json",
        "xml",
    ] {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("layer.yaml"), "keep").unwrap();
        fs::write(directory.path().join("layer.json"), "keep").unwrap();
        let output = init(directory.path(), &["--format", value]);
        assert_failure(
            &output,
            if value.is_empty() {
                "a value is required"
            } else {
                "invalid value"
            },
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("yaml, yml, toml, json, jsonc"));
        assert!(!stderr.contains("Warning:"));
        assert!(!stderr.contains("already exists"));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
