#![cfg(unix)]

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

mod add_pty_support;
use add_pty_support::{Output, Settings, run};

fn assert_no_session(text: &str) {
    for frame in ['┌', '│', '└', '◇', '◆', '⠀'] {
        assert!(!text.contains(frame), "{text:?}");
    }
    for forbidden in [
        "Hydragen",
        "Warning:",
        "Could not add dependencies",
        "Author (optional",
    ] {
        assert!(!text.contains(forbidden), "{text:?}");
    }
}

fn assert_styled_failure(output: &Output) {
    assert_eq!(output.status.exit_code(), 2, "{}", output.plain());
    let plain = output.plain();
    assert!(plain.starts_with("✕ Error:"), "{plain:?}");
    assert_eq!(plain.matches("✕ Error:").count(), 1);
    assert!(
        output.raw.lines().next().unwrap().contains("\x1b[31m"),
        "{:?}",
        output.raw
    );
    for line in output.raw.lines().skip(1) {
        assert!(
            !line.contains('\x1b'),
            "supporting text is styled: {line:?}"
        );
    }
    assert_no_session(&plain);
}

#[test]
fn root_init_and_add_usage_failures_have_one_red_headline_at_narrow_and_normal_widths() {
    let directory = tempfile::tempdir().unwrap();
    for width in [40, 80, 100] {
        for (label, arguments) in [
            ("root-missing-command", vec![]),
            ("root-global-only", vec!["--non-interactive"]),
            ("root-unknown-flag", vec!["--bogus"]),
            ("root-unknown-command", vec!["inti"]),
            ("init-unknown-flag", vec!["init", "--nme", "rules"]),
            ("init-invalid-format", vec!["init", "--format", "yam"]),
            ("init-missing-value", vec!["init", "--name"]),
            (
                "init-conflict",
                vec!["init", "--name", "one", "--name", "two"],
            ),
            ("add-missing-source", vec!["add"]),
            ("add-unknown-flag", vec!["add", "--path", "../rules"]),
        ] {
            let output = run(
                directory.path(),
                &arguments,
                Settings {
                    width,
                    ..Settings::default()
                },
            );
            output.evidence(&format!("usage-{label}-{width}"));
            assert_styled_failure(&output);
            if label == "add-missing-source" {
                let plain = output.plain();
                assert!(plain.starts_with("✕ Error: Add requires at least one source.\n"));
                assert_eq!(plain.matches("hydragen add ../shared-config").count(), 1);
                assert!(plain.contains("hydragen add --help"));
                assert!(!plain.contains("Sources:"));
                assert!(!plain.contains("Behaviour:"));
            }
            if label == "init-invalid-format" {
                for choice in ["yaml", "yml", "toml", "json", "jsonc", "similar"] {
                    assert!(output.plain().contains(choice));
                }
            }
        }
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn non_interactive_before_or_after_each_subcommand_does_not_disable_error_styling() {
    let directory = tempfile::tempdir().unwrap();
    for (label, arguments) in [
        ("add-before", vec!["--non-interactive", "add"]),
        ("add-after", vec!["add", "--non-interactive"]),
        ("init-before", vec!["--non-interactive", "init", "--name"]),
        ("init-after", vec!["init", "--non-interactive", "--name"]),
    ] {
        let output = run(directory.path(), &arguments, Settings::default());
        output.evidence(&format!("usage-non-interactive-{label}"));
        assert_styled_failure(&output);
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn shared_colour_gates_win_over_both_force_variables_for_every_command() {
    let directory = tempfile::tempdir().unwrap();
    for (label, environment) in [
        ("no-color", vec![("NO_COLOR", "1"), ("FORCE_COLOR", "1")]),
        (
            "clicolor-zero",
            vec![("CLICOLOR", "0"), ("FORCE_COLOR", "1")],
        ),
        ("ci", vec![("CI", "true"), ("FORCE_COLOR", "1")]),
        ("dumb", vec![("TERM", "dumb"), ("FORCE_COLOR", "1")]),
    ] {
        for (command, arguments) in [
            ("root", vec!["--bogus"]),
            ("init", vec!["init", "--format", "yam"]),
            ("add", vec!["add"]),
        ] {
            let output = run(
                directory.path(),
                &arguments,
                Settings {
                    environment: &environment,
                    ..Settings::default()
                },
            );
            output.evidence(&format!("usage-{label}-beats-force-{command}"));
            assert_eq!(output.status.exit_code(), 2);
            assert!(!output.raw.contains('\x1b'), "{label}: {:?}", output.raw);
            assert!(output.plain().starts_with("Error:"));
            assert!(!output.plain().contains('✕'));
            assert_no_session(&output.plain());
        }
    }
    // Empty NO_COLOR and false CI are not explicit disables.
    let output = run(
        directory.path(),
        &["add"],
        Settings {
            environment: &[("NO_COLOR", ""), ("CI", "false"), ("FORCE_COLOR", "1")],
            ..Settings::default()
        },
    );
    assert_styled_failure(&output);
    output.evidence("usage-empty-no-color-false-ci");
    for arguments in [vec!["--bogus"], vec!["init", "--name"], vec!["add"]] {
        let output = run(
            directory.path(),
            &arguments,
            Settings {
                environment: &[("CLICOLOR_FORCE", "0"), ("FORCE_COLOR", "0")],
                ..Settings::default()
            },
        );
        assert_styled_failure(&output);
    }
}

#[test]
fn every_redirected_stream_mask_is_plain_and_usage_errors_stay_on_stderr() {
    let directory = tempfile::tempdir().unwrap();
    let captures = tempfile::tempdir().unwrap();
    for streams in [
        "stdin",
        "stdout",
        "stderr",
        "stdin,stdout",
        "stdin,stderr",
        "stdout,stderr",
        "stdin,stdout,stderr",
    ] {
        for (label, arguments) in [
            ("root", vec!["--bogus"]),
            ("init", vec!["init", "--format", "yam"]),
            ("add", vec!["add"]),
        ] {
            let capture = captures.path().join(format!("{streams}-{label}.txt"));
            let output = run(
                directory.path(),
                &arguments,
                Settings {
                    environment: &[("FORCE_COLOR", "1")],
                    redirect: Some((streams, &capture)),
                    ..Settings::default()
                },
            );
            output.evidence(&format!(
                "usage-redirect-{}-{label}",
                streams.replace(',', "-")
            ));
            let file = fs::read_to_string(&capture).unwrap_or_default();
            assert_eq!(output.status.exit_code(), 2);
            let combined = format!("{}{file}", output.raw);
            assert!(!combined.contains('\x1b'), "{streams}: {combined:?}");
            assert!(combined.starts_with("Error:"), "{combined:?}");
            assert!(!combined.contains('✕'));
            assert_no_session(&combined);
            if streams.split(',').any(|stream| stream == "stderr") {
                assert!(
                    output.raw.is_empty(),
                    "stdout unexpectedly used: {:?}",
                    output.raw
                );
                assert!(!file.is_empty());
            } else {
                assert!(file.is_empty(), "stdout unexpectedly used: {file:?}");
            }
        }
    }
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn explicit_help_and_version_stay_plain_at_existing_widths_in_hostile_layer_state() {
    let directory = tempfile::tempdir().unwrap();
    for filename in [
        "layer.yaml",
        "layer.yml",
        "layer.toml",
        "layer.json",
        "layer.jsonc",
    ] {
        fs::write(directory.path().join(filename), "invalid [\n").unwrap();
    }
    let before = state(directory.path());
    for width in [40, 80, 100] {
        for (label, arguments) in [
            ("root-short-help", vec!["-h"]),
            ("root-help", vec!["--help"]),
            ("root-help-command", vec!["help"]),
            ("init-short-help", vec!["init", "-h"]),
            ("init-help", vec!["init", "--help"]),
            ("init-help-command", vec!["help", "init"]),
            ("add-short-help", vec!["add", "-h"]),
            ("add-help", vec!["add", "--help"]),
            ("add-help-command", vec!["help", "add"]),
            ("short-version", vec!["-V"]),
            ("version", vec!["--version"]),
        ] {
            let capture = tempfile::NamedTempFile::new().unwrap();
            let output = run(
                directory.path(),
                &arguments,
                Settings {
                    width,
                    // Capture stderr separately so success must be plain stdout.
                    // Eligibility itself is also tested by the unredirected pass.
                    redirect: Some(("stderr", capture.path())),
                    ..Settings::default()
                },
            );
            assert!(fs::read(capture.path()).unwrap().is_empty());
            assert!(output.status.success(), "{}", output.plain());
            assert!(!output.raw.contains('\x1b'), "{:?}", output.raw);
            let plain = output.plain();
            assert!(!plain.contains("Error:"));
            assert_no_session(&plain);
            if label.contains("version") {
                assert_eq!(plain, format!("hydragen {}\n", env!("CARGO_PKG_VERSION")));
            } else {
                assert!(plain.contains("Usage: hydragen"));
                for line in plain.lines() {
                    // clap keeps this existing 41-column usage synopsis intact
                    // at 40 columns; preserve it rather than reflowing help.
                    if width == 40 && line == "Usage: hydragen add [OPTIONS] <SOURCE>..." {
                        continue;
                    }
                    assert!(line.len() <= usize::from(width), "{width}: {line:?}");
                }
            }
            let eligible = run(
                directory.path(),
                &arguments,
                Settings {
                    width,
                    ..Settings::default()
                },
            );
            eligible.evidence(&format!("usage-{label}-{width}"));
            assert!(eligible.status.success());
            assert!(!eligible.raw.contains('\x1b'));
            assert_eq!(eligible.plain(), plain);
            assert_eq!(state(directory.path()), before);
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FileState {
    path: std::path::PathBuf,
    bytes: Option<Vec<u8>>,
    device: u64,
    inode: u64,
    mtime: i64,
    mtime_nsec: i64,
}

fn state(directory: &Path) -> Vec<FileState> {
    let mut entries = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .chain(std::iter::once(directory.to_owned()))
        .map(|path| {
            let metadata = fs::metadata(&path).unwrap();
            FileState {
                bytes: metadata.is_file().then(|| fs::read(&path).unwrap()),
                path,
                device: metadata.dev(),
                inode: metadata.ino(),
                mtime: metadata.mtime(),
                mtime_nsec: metadata.mtime_nsec(),
            }
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}

#[test]
fn parser_failures_never_warn_or_change_invalid_duplicate_layer_manifests_on_a_terminal() {
    let directory = tempfile::tempdir().unwrap();
    for filename in [
        "layer.yaml",
        "layer.yml",
        "layer.toml",
        "layer.json",
        "layer.jsonc",
    ] {
        fs::write(directory.path().join(filename), "[invalid\n").unwrap();
    }
    let before = state(directory.path());
    for arguments in [
        vec![],
        vec!["--bogus"],
        vec!["inti"],
        vec!["init", "--name"],
        vec!["init", "--format", "yam"],
        vec!["add"],
        vec!["add", "--path", "../rules"],
    ] {
        let output = run(directory.path(), &arguments, Settings::default());
        assert_styled_failure(&output);
        assert_eq!(state(directory.path()), before);
    }
}

#[test]
fn terminal_parser_security_escapes_raw_controls_but_not_generated_layout_or_headline_style() {
    let directory = tempfile::tempdir().unwrap();
    let raw = "--bad\u{1b}[2J\r\n\t\u{85}\u{9b}\u{202e}\u{2028}";
    let escaped = r"--bad\x1b[2J\r\n\t\u{85}\u{9b}\u{202e}\u{2028}";
    for arguments in [vec![raw], vec!["init", raw], vec!["add", raw]] {
        let output = run(directory.path(), &arguments, Settings::default());
        assert_styled_failure(&output);
        let plain = output.plain();
        assert!(plain.contains(escaped), "{plain:?}");
        assert!(plain.contains("Usage:"));
        for forbidden in ['\u{85}', '\u{9b}', '\u{202e}', '\u{2028}', '\t'] {
            assert!(!plain.contains(forbidden), "{plain:?}");
        }
        assert!(!output.raw.contains("\x1b[2J"), "{:?}", output.raw);
        if arguments[0] == "add" {
            assert!(
                plain.matches(escaped).count() >= 2,
                "tip was not escaped: {plain:?}"
            );
        }
        output.evidence(&format!(
            "usage-control-safety-{}",
            arguments[0]
                .trim_start_matches('-')
                .chars()
                .take(3)
                .collect::<String>()
        ));
    }
}
