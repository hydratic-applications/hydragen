#![cfg(unix)]

use std::fs;
use std::path::PathBuf;

mod add_diagnostic_support;
mod add_pty_support;
mod diagnostic_body_support;
use add_pty_support::{Output, Settings, run};

#[test]
fn verification_family_at_review_widths_uses_same_public_contract_as_captured_output() {
    for label in add_diagnostic_support::CASES {
        let case = add_diagnostic_support::Case::new(label);
        let args = case
            .arguments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        for width in [40, 80, 100, 120] {
            for (mode, environment) in [("styled", &[][..]), ("plain", &[("NO_COLOR", "1")][..])] {
                let output = run(
                    &case.invocation,
                    &args,
                    Settings {
                        width,
                        environment,
                        ..Settings::default()
                    },
                );
                assert_eq!(output.status.exit_code(), 1, "{label}: {}", output.plain());
                if mode == "styled" {
                    assert!(
                        output.raw.lines().next().unwrap().contains("\x1b[31m"),
                        "{}",
                        output.raw
                    );
                    assert!(output.plain().starts_with("✕ Error:"));
                } else {
                    assert!(!output.raw.contains('\x1b'), "{}", output.raw);
                    assert!(output.plain().starts_with("Error:"));
                }
                assert!(
                    output
                        .raw
                        .lines()
                        .skip(1)
                        .all(|line| !line.contains('\x1b')),
                    "details must remain neutral: {}",
                    output.raw
                );
                case.assert_diagnostic(&output.plain());
                output.evidence(&format!("phase2-{label}-{mode}-{width}"));
            }
        }
    }
}

#[test]
fn terminal_alias_noop_preserves_stored_reference_inode_mtime_and_symlink_targets() {
    use std::os::unix::fs::{MetadataExt, symlink};
    let fixture = Fixture::new();
    fixture.layer("rules", "rules");
    let rules = fixture.root.path().join("rules");
    let target = fixture.root.path().join("rules-source.yaml");
    fs::rename(rules.join("layer.yaml"), &target).unwrap();
    symlink(&target, rules.join("layer.yaml")).unwrap();
    symlink(&rules, fixture.root.path().join("rules-link")).unwrap();
    let path = fixture.app.join("layer.yaml");
    let original = "name: app\ndependencies: [../rules-link]\n";
    fs::write(&path, original).unwrap();
    let before = fs::metadata(&path).unwrap();
    let target_before = fs::metadata(&target).unwrap();
    for width in [40, 80, 100, 120] {
        for (mode, environment) in [("styled", &[][..]), ("plain", &[("NO_COLOR", "1")][..])] {
            let output = fixture.run(
                &["add", "../rules", "../rules-link", rules.to_str().unwrap()],
                Settings {
                    width,
                    environment,
                    ..Settings::default()
                },
            );
            assert!(output.status.success(), "{}", output.plain());
            let expected = "No changes needed: rules is already a dependency of app.\n";
            assert_eq!(
                output.plain(),
                if mode == "styled" {
                    format!("ℹ {expected}")
                } else {
                    expected.into()
                }
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
            assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
            assert_eq!(
                fs::metadata(&path).unwrap().modified().unwrap(),
                before.modified().unwrap()
            );
            assert_eq!(fs::read_to_string(&target).unwrap(), "name: \"rules\"\n");
            assert_eq!(fs::metadata(&target).unwrap().ino(), target_before.ino());
            assert_eq!(
                fs::metadata(&target).unwrap().modified().unwrap(),
                target_before.modified().unwrap()
            );
            assert!(
                fs::symlink_metadata(rules.join("layer.yaml"))
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
            assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
            output.evidence(&format!("phase2-alias-noop-{mode}-{width}"));
        }
    }
}

struct Fixture {
    root: tempfile::TempDir,
    app: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("layer.yaml"), "name: app\n").unwrap();
        Self { root, app }
    }

    fn layer(&self, directory: &str, name: &str) {
        let path = self.root.path().join(directory);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("layer.yaml"),
            format!("name: {}\n", serde_json::to_string(name).unwrap()),
        )
        .unwrap();
    }

    fn run(&self, args: &[&str], settings: Settings<'_>) -> Output {
        let output = run(&self.app, args, settings);
        let plain = output.plain();
        for frame in ['┌', '│', '└', '◇', '◆', '⠀'] {
            assert!(!plain.contains(frame), "{plain}");
        }
        assert!(!plain.contains("Hydragen"), "{plain}");
        output
    }
}

#[test]
fn single_results_at_review_widths_cover_names_repeats_aliases_and_unrequested_dependencies() {
    use std::os::unix::fs::{MetadataExt, symlink};
    for width in [40, 80, 100, 120] {
        for (mode, environment) in [("styled", &[][..]), ("plain", &[("NO_COLOR", "1")][..])] {
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
                fixture.layer("Shared Rules @team", name);
                fixture.layer("unrequested", "unrequested");
                let rules = fixture.root.path().join("Shared Rules @team");
                symlink(&rules, fixture.root.path().join("rules-link")).unwrap();
                let path = fixture.app.join("layer.yaml");
                fs::write(
                    &path,
                    format!(
                        "name: {}\n{}",
                        serde_json::to_string(destination).unwrap(),
                        if unrelated {
                            "dependencies: [../unrequested, unrequested-published@1.2.3]\n"
                        } else {
                            ""
                        }
                    ),
                )
                .unwrap();
                let reference = "../Shared Rules @team/./";
                let mut arguments = vec!["add", reference];
                if requests == "repeated" {
                    arguments.extend([reference, reference, "--non-interactive"]);
                } else if requests == "aliases" {
                    arguments.extend(["../rules-link", rules.to_str().unwrap(), reference]);
                }
                if label == "scoped" || unrelated {
                    arguments.insert(0, "--non-interactive");
                }
                for already_present in [false, true] {
                    let before = fs::metadata(&path).unwrap();
                    let original = fs::read(&path).unwrap();
                    let output = fixture.run(
                        &arguments,
                        Settings {
                            width,
                            environment,
                            ..Settings::default()
                        },
                    );
                    assert!(output.status.success(), "{label}: {}", output.plain());
                    let (prefix, relation) = if already_present {
                        ("No changes needed: ", " is already a dependency of ")
                    } else {
                        ("Added ", " to ")
                    };
                    let sentence = format!("{prefix}{name}{relation}{destination}.\n");
                    if mode == "styled" {
                        let symbol = if already_present { "ℹ" } else { "✓" };
                        assert_eq!(output.plain(), format!("{symbol} {sentence}"));
                        let text = if already_present {
                            console::Style::new().force_styling(true).blue()
                        } else {
                            console::Style::new().force_styling(true).green()
                        };
                        let names = text.clone().bold();
                        assert_eq!(
                            output.raw.replace('\r', ""),
                            format!(
                                "{}{}{}{}{}\n",
                                text.apply_to(format!("{symbol} {prefix}")),
                                names.apply_to(name),
                                text.apply_to(relation),
                                names.apply_to(destination),
                                text.apply_to(".")
                            )
                        );
                    } else {
                        assert_eq!(output.plain(), sentence);
                        assert!(!output.raw.contains('\x1b'), "{}", output.raw);
                    }
                    if already_present {
                        assert_eq!(fs::read(&path).unwrap(), original);
                        assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
                        assert_eq!(
                            fs::metadata(&path).unwrap().modified().unwrap(),
                            before.modified().unwrap()
                        );
                    }
                    let expected = if unrelated {
                        vec!["../unrequested", "unrequested-published@1.2.3", reference]
                    } else {
                        vec![reference]
                    };
                    assert_eq!(
                        hydragen::discover(&fixture.app)
                            .unwrap()
                            .load()
                            .unwrap()
                            .document["dependencies"],
                        serde_json::json!(expected)
                    );
                    assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
                    assert_eq!(
                        fs::read_to_string(rules.join("layer.yaml")).unwrap(),
                        format!("name: {}\n", serde_json::to_string(name).unwrap())
                    );
                    output.evidence(&format!(
                        "single-{label}-{}-{mode}-{width}",
                        if already_present { "present" } else { "added" }
                    ));
                }
            }
        }
    }
}

#[test]
fn two_effective_results_at_review_widths_keep_grouping_including_one_added_one_present() {
    for width in [40, 80, 100, 120] {
        for (mode, environment) in [("styled", &[][..]), ("plain", &[("NO_COLOR", "1")][..])] {
            for (label, existing, expected) in [
                (
                    "added",
                    "[]",
                    "Dependencies updated for app.\n\nAdded (2)\n  tooling\n  rules\n",
                ),
                (
                    "present",
                    "[../rules, ../tooling, ../unrequested]",
                    "No changes needed for app.\n\nAlready present (2)\n  tooling\n  rules\n",
                ),
                (
                    "mixed",
                    "[../rules, ../unrequested]",
                    "Dependencies updated for app.\n\nAdded (1)\n  tooling\n\nAlready present (1)\n  rules\n",
                ),
            ] {
                let fixture = Fixture::new();
                for name in ["rules", "tooling", "unrequested"] {
                    fixture.layer(name, name);
                }
                fs::write(
                    fixture.app.join("layer.yaml"),
                    format!("name: app\ndependencies: {existing}\n"),
                )
                .unwrap();
                let output = fixture.run(
                    &["add", "../tooling", "../rules", "../tooling"],
                    Settings {
                        width,
                        environment,
                        ..Settings::default()
                    },
                );
                assert!(output.status.success(), "{}", output.plain());
                assert_eq!(
                    output.plain(),
                    if mode == "styled" {
                        format!("{} {expected}", if label == "present" { "ℹ" } else { "✓" })
                    } else {
                        expected.into()
                    }
                );
                assert_eq!(output.raw.contains('\x1b'), mode == "styled");
                output.evidence(&format!("batch-{label}-{mode}-{width}"));
            }
        }
    }
}

#[test]
fn all_terminal_mixed_success_and_noop_are_compact_with_normal_foreground_details() {
    let fixture = Fixture::new();
    fixture.layer("rules", "@example/shared-rules");
    fixture.layer("tooling", "tooling");
    fixture.layer("old", "old");
    fixture.layer("unrequested", "unrequested");
    fs::write(
        fixture.app.join("layer.yaml"),
        "name: app\ndependencies: [../old, ../unrequested]\n",
    )
    .unwrap();
    for (label, args) in [
        (
            "mixed-success-80",
            vec![
                "--non-interactive",
                "add",
                "../rules",
                "../old",
                "../tooling",
                "../rules",
            ],
        ),
        (
            "noop-success-80",
            vec!["add", "../rules", "../old", "../rules", "--non-interactive"],
        ),
    ] {
        let output = fixture.run(&args, Settings::default());
        assert!(output.status.success(), "{}", output.plain());
        assert!(output.raw.contains('\x1b'));
        let noop = label.starts_with("noop");
        let colour = if noop { "\x1b[34m" } else { "\x1b[32m" };
        assert!(output.raw.contains(colour), "{}", output.raw);
        let plain = output.plain();
        let symbol = if noop { "ℹ" } else { "✓" };
        assert!(plain.starts_with(&format!("{symbol} ")));
        assert_eq!(plain.matches(symbol).count(), 1);
        assert!(!plain.contains(if noop { '✓' } else { 'ℹ' }));
        assert!(!plain.contains("layer.yaml"));
        assert!(!plain.contains("unrequested"));
        assert!(!plain.contains("Warning:"));
        let style = console::Style::new().force_styling(true);
        let headline_style = if noop {
            style.clone().blue()
        } else {
            style.clone().green()
        };
        let headline = output.raw.lines().next().unwrap();
        assert!(
            headline.contains(&headline_style.bold().apply_to("app").to_string()),
            "destination name must be bold in the status colour: {headline:?}"
        );
        for line in output.raw.lines().skip(1) {
            let plain_line = console::strip_ansi_codes(line);
            if let Some((label, count)) = plain_line.trim_end_matches('\r').rsplit_once(" (")
                && ["Added", "Already present"].contains(&label)
            {
                assert_eq!(
                    line.trim_end_matches('\r'),
                    format!(
                        "{} {}",
                        style.clone().bold().apply_to(label),
                        style.clone().dim().apply_to(format!("({count}"))
                    )
                );
            } else {
                assert!(
                    !line.contains('\x1b'),
                    "dependency details must be unstyled: {line:?}"
                );
            }
        }
        if label.starts_with("mixed") {
            assert_eq!(
                plain,
                "✓ Dependencies updated for app.\n\nAdded (2)\n  @example/shared-rules\n  tooling\n\nAlready present (1)\n  old\n"
            );
        } else {
            assert_eq!(
                plain,
                "ℹ No changes needed for app.\n\nAlready present (2)\n  @example/shared-rules\n  old\n"
            );
        }
        output.evidence(label);
    }
}

#[test]
fn grouped_noop_info_obeys_plain_controls_without_writes_or_stderr_output() {
    use std::os::unix::fs::MetadataExt;
    let fixture = Fixture::new();
    fixture.layer("rules", "rules");
    fixture.layer("tooling", "tooling");
    let path = fixture.app.join("layer.yaml");
    let original = "name: app\ndependencies: [../rules, ../tooling]\n";
    fs::write(&path, original).unwrap();
    let before = fs::metadata(&path).unwrap();
    let expected = "No changes needed for app.\n\nAlready present (2)\n  tooling\n  rules\n";
    for control in [
        "no-color",
        "clicolor-zero",
        "ci",
        "dumb",
        "stdin",
        "stdout",
        "stderr",
        "stdin,stdout",
        "stdin,stderr",
        "stdout,stderr",
        "stdin,stdout,stderr",
    ] {
        let capture = tempfile::NamedTempFile::new().unwrap();
        let mut environment = vec![("FORCE_COLOR", "1")];
        match control {
            "no-color" => environment.push(("NO_COLOR", "1")),
            "clicolor-zero" => environment.push(("CLICOLOR", "0")),
            "ci" => environment.push(("CI", "true")),
            "dumb" => environment.push(("TERM", "dumb")),
            _ => {}
        }
        let redirected =
            control.contains("stdin") || control.contains("stdout") || control.contains("stderr");
        let output = fixture.run(
            &[
                "--non-interactive",
                "add",
                "../tooling",
                "../rules",
                "../tooling",
            ],
            Settings {
                environment: &environment,
                redirect: redirected.then_some((control, capture.path())),
                ..Settings::default()
            },
        );
        assert_eq!(
            output.status.exit_code(),
            0,
            "{control}: {}",
            output.plain()
        );
        let captured = fs::read_to_string(capture.path()).unwrap();
        let combined = format!("{}{captured}", output.raw).replace('\r', "");
        assert_eq!(combined, expected, "{control}");
        assert!(!combined.contains(['\x1b', 'ℹ', '✓']));
        if !control.contains("stdout") {
            assert!(
                captured.is_empty(),
                "info must not be emitted on stderr: {control}"
            );
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(fs::metadata(&path).unwrap().ino(), before.ino());
        assert_eq!(
            fs::metadata(&path).unwrap().modified().unwrap(),
            before.modified().unwrap()
        );
        assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
        output.evidence(&format!("info-grouped-control-{control}"));
    }
}

#[test]
fn missing_current_layer_is_concise_in_styled_and_plain_terminals() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("not-a-layer");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("keep.txt"), "unchanged").unwrap();
    let expected = concat!(
        "Error: No layer manifest found in this directory or its parents.\n  \n",
        "  Tip: Run this command from an existing layer directory, or run\n",
        "  'hydragen init' in the directory you want to use as a layer.\n  \n",
        "  No files were changed.\n"
    );
    for width in [40, 80, 120] {
        for (mode, environment) in [
            ("styled", vec![]),
            ("plain", vec![("NO_COLOR", "1")]),
            ("ci", vec![("CI", "true")]),
        ] {
            let output = run(
                &directory,
                &["add", "test", "--non-interactive"],
                Settings {
                    width,
                    environment: &environment,
                    ..Settings::default()
                },
            );
            assert_eq!(output.status.exit_code(), 1);
            let plain = output.plain();
            if mode == "styled" {
                assert_eq!(plain, format!("✕ {expected}"));
                assert!(output.raw.lines().next().unwrap().contains("\x1b[31m"));
                for line in output.raw.lines().skip(1) {
                    assert!(
                        !line.contains('\x1b'),
                        "details must stay neutral: {line:?}"
                    );
                }
            } else {
                assert_eq!(plain, expected);
                assert!(!output.raw.contains('\x1b'));
            }
            assert!(!plain.contains(&directory.display().to_string()));
            assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
            assert_eq!(
                fs::read_to_string(directory.join("keep.txt")).unwrap(),
                "unchanged"
            );
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
            output.evidence(&format!("missing-current-layer-{mode}-{width}"));
        }
    }
}

#[test]
fn all_terminal_errors_are_grouped_in_order_and_only_the_headline_is_red() {
    let fixture = Fixture::new();
    fixture.layer("bad", "bad");
    fs::write(fixture.root.path().join("bad/layer.yaml"), "name: [").unwrap();
    let output = fixture.run(
        &["add", "../missing", "../bad", "bare-name", "rules@1.2.0"],
        Settings::default(),
    );
    assert!(!output.status.success());
    assert!(output.raw.contains("\x1b[31m"), "{}", output.raw);
    let plain = output.plain();
    assert!(
        plain.starts_with("✕ Error: Dependency checks failed (4 errors).\n"),
        "{plain}"
    );
    for line in output.raw.lines().skip(1) {
        assert!(!line.contains('\x1b'));
    }
    let mut last = 0;
    for phrase in [
        "Source '../missing'",
        "Source '../bad'",
        "Source 'bare-name'",
        "Source 'rules@1.2.0'",
    ] {
        let index = plain.find(phrase).unwrap();
        assert!(index > last);
        last = index;
    }
    diagnostic_body_support::assert_verification(
        &plain,
        4,
        &[
            "Source '../missing'",
            "Source '../bad'",
            "Source 'bare-name'",
            "Source 'rules@1.2.0'",
        ],
    );
    assert!(!plain.contains("Paths are relative"));
    assert!(plain.contains("No files were changed"));
    assert!(plain.contains("Invalid source reference."));
    assert!(!plain.contains("oneOf"));
    assert!(!plain.contains("schemas listed"));
    assert_eq!(
        fs::read_to_string(fixture.app.join("layer.yaml")).unwrap(),
        "name: app\n"
    );
    output.evidence("aggregated-errors-80");
}

#[test]
fn explicit_disables_ci_and_dumb_term_win_over_forced_colour_on_real_terminals() {
    for width in [40, 80, 100, 120] {
        for (label, environment) in [
            (
                "no-color-force",
                vec![("NO_COLOR", "1"), ("FORCE_COLOR", "1")],
            ),
            ("clicolor-zero-force", vec![("CLICOLOR", "0")]),
            ("ci-force", vec![("CI", "true")]),
            ("dumb-force", vec![("TERM", "dumb")]),
        ] {
            let fixture = Fixture::new();
            fixture.layer("rules", "rules");
            let output = fixture.run(
                &["add", "../rules"],
                Settings {
                    width,
                    environment: &environment,
                    ..Settings::default()
                },
            );
            assert!(output.status.success());
            assert!(!output.raw.contains('\x1b'), "{label}: {}", output.raw);
            assert!(!output.plain().contains(['✓', 'ℹ']));
            assert_eq!(output.plain(), "Added rules to app.\n");
            output.evidence(&format!("single-{label}-added-{width}"));
            let output = fixture.run(
                &["--non-interactive", "add", "../rules", "../rules"],
                Settings {
                    width,
                    environment: &environment,
                    ..Settings::default()
                },
            );
            assert!(output.status.success(), "{}", output.plain());
            assert_eq!(
                output.plain(),
                "No changes needed: rules is already a dependency of app.\n"
            );
            assert!(!output.raw.contains('\x1b'));
            output.evidence(&format!("single-{label}-present-{width}"));
            let output = fixture.run(
                &["add", "../missing"],
                Settings {
                    width,
                    environment: &environment,
                    ..Settings::default()
                },
            );
            assert!(!output.status.success());
            assert!(!output.raw.contains('\x1b'));
            assert!(output.plain().starts_with("Error:"));
        }
    }
}

#[test]
fn mixed_stream_redirection_stays_plain_despite_force_and_does_not_prompt() {
    for width in [40, 80, 100, 120] {
        for stream in ["stdin", "stdout", "stderr"] {
            let fixture = Fixture::new();
            fixture.layer("rules", "rules");
            let capture = fixture.root.path().join("capture.txt");
            let settings = || Settings {
                width,
                redirect: Some((stream, capture.as_path())),
                ..Settings::default()
            };
            let output = fixture.run(&["add", "../rules", "--non-interactive"], settings());
            assert!(output.status.success(), "{}", output.plain());
            let file = fs::read_to_string(&capture).unwrap_or_default();
            let combined = format!("{}{file}", output.raw);
            assert!(!combined.contains('\x1b'), "{stream}: {combined}");
            assert_eq!(combined.replace('\r', ""), "Added rules to app.\n");
            assert!(!combined.contains(['✓', 'ℹ']));
            output.evidence(&format!("single-redirect-{stream}-added-{width}"));
            let output = fixture.run(
                &["add", "../rules", "../rules", "--non-interactive"],
                settings(),
            );
            assert!(output.status.success(), "{}", output.plain());
            let file = fs::read_to_string(&capture).unwrap_or_default();
            let combined = format!("{}{file}", output.raw);
            assert_eq!(
                combined.replace('\r', ""),
                "No changes needed: rules is already a dependency of app.\n"
            );
            output.evidence(&format!("single-redirect-{stream}-present-{width}"));
            let output = fixture.run(&["add", "../missing"], settings());
            assert!(!output.status.success());
            let file = fs::read_to_string(&capture).unwrap_or_default();
            let combined = format!("{}{file}", output.raw);
            assert!(!combined.contains('\x1b'));
            assert!(combined.contains("Error: Could not resolve local directory '../missing'."));
        }
    }
}

#[test]
fn terminal_help_stays_plain_and_usage_failures_are_styled_without_discovery() {
    let fixture = Fixture::new();
    fs::write(fixture.app.join("layer.yaml"), "[").unwrap();
    fs::write(fixture.app.join("layer.json"), "{}").unwrap();
    for (label, args, success) in [
        ("help-40", vec!["add", "--help"], true),
        ("version-40", vec!["--version"], true),
        ("missing-args-40", vec!["add"], false),
        ("unknown-path-40", vec!["add", "--path", "../rules"], false),
    ] {
        let output = fixture.run(
            &args,
            Settings {
                width: 40,
                ..Settings::default()
            },
        );
        assert_eq!(output.status.success(), success, "{}", output.plain());
        if success {
            assert!(!output.raw.contains('\x1b'), "{}", output.raw);
            assert!(!output.plain().contains("Error:"));
        } else {
            assert_eq!(output.status.exit_code(), 2);
            assert!(output.raw.contains("\x1b[31m"), "{}", output.raw);
            assert!(output.plain().starts_with("✕ Error:"));
            for line in output.raw.lines().skip(1) {
                assert!(!line.contains('\x1b'), "{line:?}");
            }
        }
        assert!(!output.plain().contains("Warning:"));
        assert!(!output.plain().contains("Could not add dependencies"));
        if label == "missing-args-40" {
            assert!(output.plain().contains("Add requires at least one source."));
            assert!(output.plain().contains("<SOURCE>..."));
        }
        output.evidence(label);
    }
    assert_eq!(
        fs::read_to_string(fixture.app.join("layer.yaml")).unwrap(),
        "["
    );
}

#[test]
fn narrow_terminals_and_long_scoped_names_keep_compact_results_and_full_names() {
    let fixture = Fixture::new();
    let name = format!("@example/{}", "shared-config-".repeat(10));
    fixture.layer("rules", &name);
    let output = fixture.run(
        &["add", "../rules"],
        Settings {
            width: 32,
            ..Settings::default()
        },
    );
    assert!(output.status.success());
    assert_eq!(output.plain(), format!("✓ Added {name} to app.\n"));
    assert_eq!(output.plain().matches(&name).count(), 1);
    output.evidence("long-scoped-name-32");
    fixture.layer("conflict", &name);
    let existing = fs::canonicalize(fixture.root.path().join("rules")).unwrap();
    let requested = fs::canonicalize(fixture.root.path().join("conflict")).unwrap();
    let existing_label = format!("Existing source path: {}", existing.display());
    let requested_label = format!("Requested source path: {}", requested.display());
    let original = fs::read(fixture.app.join("layer.yaml")).unwrap();
    for width in [32, 80] {
        let output = fixture.run(
            &["add", "../conflict", "../missing", "not-a-local-path"],
            Settings {
                width,
                ..Settings::default()
            },
        );
        assert!(!output.status.success());
        assert!(output.plain().contains(&name));
        assert!(output.plain().contains("3 errors"));
        assert!(
            output
                .plain()
                .contains("Conflicting source: Existing dependency '../rules'")
        );
        assert!(output.plain().contains(&existing_label));
        assert!(output.plain().contains(&requested_label));
        assert!(!output.plain().contains("oneOf"));
        assert_eq!(fs::read(fixture.app.join("layer.yaml")).unwrap(), original);
        assert_eq!(fs::read_dir(&fixture.app).unwrap().count(), 1);
        output.evidence(&format!("long-scoped-errors-{width}"));
    }
}

#[test]
fn duplicate_warning_uses_shared_yellow_headline_once_before_red_errors() {
    let fixture = Fixture::new();
    fixture.layer("rules", "rules");
    let rules = fixture.root.path().join("rules");
    fs::write(rules.join("layer.yaml"), "[").unwrap();
    fs::write(rules.join("layer.json"), "{\"name\":\"ignored\"}").unwrap();
    let output = fixture.run(&["add", "../rules", "../rules"], Settings::default());
    let plain = output.plain();
    assert!(!output.status.success());
    assert!(output.raw.contains("\x1b[33m"));
    assert!(output.raw.contains("\x1b[31m"));
    assert_eq!(plain.matches("▲ Warning:").count(), 1);
    assert!(plain.find("▲ Warning:").unwrap() < plain.find("✕ Error:").unwrap());
    assert!(plain.contains("layer.yaml, layer.json."));
    assert!(plain.contains("layer.yaml takes priority"));
    output.evidence("warning-before-errors-80");
}
