use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Barrier};

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
    assert!(stdout.contains("layer.json"), "{stdout}");
    assert!(!output.stdout.contains(&b'\x1b'));
}

fn assert_failure(output: &Output, message: &str) {
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(message), "{stderr}");
    assert!(!output.stderr.contains(&b'\x1b'));
}

fn assert_occupied_manifest(output: &Output) {
    assert_failure(output, "already exists");
    let stderr = String::from_utf8_lossy(&output.stderr);
    for phrase in [
        "does not overwrite existing entries",
        "Inspect the entry",
        "in a different directory",
        "This command did not change any files.",
    ] {
        assert!(stderr.contains(phrase), "{stderr}");
    }
    let advice = stderr.split_once("already exists.").unwrap().1;
    for unsafe_advice in ["delete", "remove", "--force"] {
        assert!(!advice.contains(unsafe_advice), "{stderr}");
    }
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
    serde_json::from_slice(&fs::read(directory.join("layer.json")).unwrap()).unwrap()
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
        for value in ["", " \t\n", "\u{00a0}\u{2003}"] {
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
        let prose = help.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(help.contains("Usage:"));
        if arguments.len() == 2 {
            for expected in [
                "Optional name override",
                "current directory name",
                "Optional person",
                "Optional explanation",
                "hydragen init --name shared-rules",
                "--author \"Example team\"",
            ] {
                assert!(prose.contains(expected), "{help}");
            }
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn captured_help_uses_the_unknown_width_fallback_and_stays_plain() {
    let directory = tempdir().unwrap();
    let output = command(directory.path())
        .env_remove("COLUMNS")
        .env("CLICOLOR_FORCE", "1")
        .args(["init", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(!output.stdout.contains(&b'\x1b'));
    let help = String::from_utf8(output.stdout).unwrap();
    let longest = help.lines().map(str::len).max().unwrap();
    assert!((81..=100).contains(&longest), "{help}");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn copied_help_examples_keep_their_shell_arguments() {
    let parent = tempdir().unwrap();
    let output = command(parent.path())
        .args(["init", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    let examples = help
        .split("Examples:\n")
        .nth(1)
        .unwrap()
        .split("\n\nInput:")
        .next()
        .unwrap();
    let mut commands = Vec::new();
    let mut current = String::new();
    for line in examples.lines() {
        current.push_str(line.trim_start());
        current.push('\n');
        if !line.ends_with('\\') {
            commands.push(std::mem::take(&mut current));
        }
    }
    assert_eq!(commands.len(), 4);
    assert!(current.is_empty());
    for (index, script) in commands.iter().enumerate() {
        let directory = parent.path().join(format!("example-{index}"));
        fs::create_dir(&directory).unwrap();
        let script = script.replacen("hydragen", "\"$HYDRAGEN\"", 1);
        let output = Command::new("/bin/sh")
            .args(["-c", &script])
            .env("HYDRAGEN", env!("CARGO_BIN_EXE_hydragen"))
            .current_dir(&directory)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_success(&output);
        let expected = match index {
            0 | 3 => json!({"name": format!("example-{index}")}),
            1 => json!({"name": "shared-rules"}),
            2 => {
                json!({"name": "shared-agent-rules", "author": "Example team", "description": "Shared agent rules."})
            }
            _ => unreachable!(),
        };
        assert_eq!(read_manifest(&directory), expected);
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
    assert!(!directory.join("layer.json").exists());
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
        assert!(!directory.path().join("layer.json").exists());
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
    assert!(!directory.path().join("layer.json").exists());
}

#[test]
fn refuses_existing_manifests_even_when_malformed() {
    for original in [b"not valid JSON".as_slice(), b"{\"name\":\"existing\"}\n"] {
        let directory = tempdir().unwrap();
        let path = directory.path().join("layer.json");
        fs::write(&path, original).unwrap();
        let output = init(directory.path(), &["--name", "replacement"]);
        assert_occupied_manifest(&output);
        assert!(String::from_utf8_lossy(&output.stderr).contains("layer.json"));
        assert_eq!(fs::read(path).unwrap(), original);
    }
}

#[test]
fn occupied_manifest_guidance_preserves_a_long_unicode_path() {
    let parent = tempdir().unwrap();
    let directory = parent
        .path()
        .join(format!("Project 配置 🦀 {}", "long-name-".repeat(15)));
    fs::create_dir(&directory).unwrap();
    let path = directory.join("layer.json");
    fs::write(&path, "not a valid manifest").unwrap();
    let output = init(&directory, &["--name", "example"]);
    assert_occupied_manifest(&output);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&path.canonicalize().unwrap().display().to_string()));
    assert_eq!(fs::read_to_string(&path).unwrap(), "not a valid manifest");
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
}

#[test]
fn repeated_init_leaves_the_manifest_unchanged() {
    let directory = tempdir().unwrap();
    assert_success(&init(directory.path(), &["--name", "first"]));
    let original = fs::read(directory.path().join("layer.json")).unwrap();
    assert_occupied_manifest(&init(directory.path(), &["--name", "second"]));
    assert_eq!(
        fs::read(directory.path().join("layer.json")).unwrap(),
        original
    );
}

#[test]
fn a_directory_at_the_manifest_path_is_not_replaced() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("layer.json");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("keep.txt"), "keep").unwrap();
    assert_occupied_manifest(&init(directory.path(), &["--name", "example"]));
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
        assert_occupied_manifest(&init(directory.path(), &["--name", "example"]));
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
            assert_occupied_manifest(output);
        }
    }
    let (winner, _) = outputs
        .iter()
        .find(|(_, output)| output.status.success())
        .unwrap();
    assert_eq!(read_manifest(directory.path())["name"], *winner);
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
    assert!(!directory.path().join("layer.json").exists());
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
    assert!(stderr.contains("layer.json"));
    assert!(stderr.contains("partial file may remain"));
    assert_eq!(fs::read(directory.path().join("layer.json")).unwrap(), b"");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}
