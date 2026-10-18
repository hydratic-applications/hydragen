#![cfg(unix)]

use std::fs;
use std::path::PathBuf;

mod add_pty_support;
use add_pty_support::{Output, Settings, run};

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
        assert!(output.raw.contains("\x1b[32m"), "{}", output.raw);
        let plain = output.plain();
        assert!(plain.starts_with("✓ "));
        assert_eq!(plain.matches("✓").count(), 1);
        assert!(!plain.contains("layer.yaml"));
        assert!(!plain.contains("unrequested"));
        assert!(!plain.contains("Warning:"));
        let style = console::Style::new().force_styling(true);
        let headline = output.raw.lines().next().unwrap();
        assert!(
            headline.contains(&style.clone().green().bold().apply_to("app").to_string()),
            "destination name must be bold and green: {headline:?}"
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
                "✓ No changes needed for app.\n\nAlready present (2)\n  @example/shared-rules\n  old\n"
            );
        }
        output.evidence(label);
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
        plain.starts_with("✕ Error: Could not add dependencies (4 errors).\n"),
        "{plain}"
    );
    for line in output.raw.lines().skip(1) {
        assert!(!line.contains('\x1b'));
    }
    let mut last = 0;
    for phrase in [
        "source argument 1",
        "source argument 2",
        "source argument 3",
        "source argument 4",
    ] {
        let index = plain.find(phrase).unwrap();
        assert!(index > last);
        last = index;
    }
    assert!(plain.contains("declaring layer directory:"));
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
    for (label, environment) in [
        ("no-color-force", vec![("NO_COLOR", "1")]),
        ("clicolor-zero-force", vec![("CLICOLOR", "0")]),
        ("ci-force", vec![("CI", "true")]),
        ("dumb-force", vec![("TERM", "dumb")]),
    ] {
        let fixture = Fixture::new();
        fixture.layer("rules", "rules");
        let output = fixture.run(
            &["add", "../rules"],
            Settings {
                environment: &environment,
                ..Settings::default()
            },
        );
        assert!(output.status.success());
        assert!(!output.raw.contains('\x1b'), "{label}: {}", output.raw);
        assert!(!output.plain().contains('✓'));
        assert!(output.plain().starts_with("Dependencies updated for app."));
        output.evidence(label);
        let output = fixture.run(
            &["add", "../missing"],
            Settings {
                environment: &environment,
                ..Settings::default()
            },
        );
        assert!(!output.status.success());
        assert!(!output.raw.contains('\x1b'));
        assert!(output.plain().starts_with("Error:"));
    }
}

#[test]
fn mixed_stream_redirection_stays_plain_despite_force_and_does_not_prompt() {
    for stream in ["stdin", "stdout", "stderr"] {
        let fixture = Fixture::new();
        fixture.layer("rules", "rules");
        let capture = fixture.root.path().join("capture.txt");
        let settings = || Settings {
            redirect: Some((stream, capture.as_path())),
            ..Settings::default()
        };
        let output = fixture.run(&["add", "../rules", "--non-interactive"], settings());
        assert!(output.status.success(), "{}", output.plain());
        let file = fs::read_to_string(&capture).unwrap_or_default();
        let combined = format!("{}{file}", output.raw);
        assert!(!combined.contains('\x1b'), "{stream}: {combined}");
        assert!(combined.contains("Dependencies updated for app."));
        assert!(!combined.contains('✓'));
        output.evidence(&format!("redirect-{stream}-success"));
        let output = fixture.run(&["add", "../missing"], settings());
        assert!(!output.status.success());
        let file = fs::read_to_string(&capture).unwrap_or_default();
        let combined = format!("{}{file}", output.raw);
        assert!(!combined.contains('\x1b'));
        assert!(combined.contains("Error: Could not add dependencies"));
    }
}

#[test]
fn terminal_help_and_missing_arguments_are_plain_and_never_discover_layers() {
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
        assert!(!output.raw.contains('\x1b'), "{}", output.raw);
        assert!(!output.plain().contains("Warning:"));
        assert!(!output.plain().contains("Could not add dependencies"));
        if label == "missing-args-40" {
            assert!(output.plain().contains("required arguments"));
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
fn narrow_terminals_and_long_scoped_names_keep_counts_and_full_names() {
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
    assert!(output.plain().contains(&format!("Added (1)\n  {name}\n")));
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
                .contains("Conflicting source: existing dependency 1 ('../rules')")
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
