#![cfg(unix)]

mod add_pty_support;
mod diagnostic_body_support;
mod operation_diagnostic_support;

#[test]
fn terminal_operations_share_captured_contract_and_plain_styled_meaning_at_review_widths() {
    for label in operation_diagnostic_support::labels() {
        for width in [40, 80, 100, 120] {
            let mut previous = None;
            for (mode, environment) in [("styled", &[][..]), ("plain", &[("NO_COLOR", "1")][..])] {
                let case = operation_diagnostic_support::Case::new(&label);
                let arguments = case
                    .arguments
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                let output = add_pty_support::run(
                    &case.app,
                    &arguments,
                    add_pty_support::Settings {
                        width,
                        environment,
                        shell_prelude: case.shell_prelude.as_deref(),
                        ..Default::default()
                    },
                );
                assert_eq!(output.status.exit_code(), 1, "{label}: {}", output.plain());
                let text = output.plain();
                let styled = mode == "styled" && case.terminal_eligible;
                assert_eq!(
                    output.raw.contains('\x1b'),
                    styled,
                    "{label}: {}",
                    output.raw
                );
                assert_eq!(text.contains("✕ Error:"), styled, "{label}: {text}");
                for line in output.raw.lines() {
                    let plain = console::strip_ansi_codes(line);
                    if plain.starts_with("✕ Error:") {
                        assert!(
                            line.contains("\x1b[31m"),
                            "error headline must be red: {line:?}"
                        );
                    }
                    if plain.starts_with("▲ Warning:") {
                        assert!(
                            line.contains("\x1b[33m"),
                            "warning headline must be yellow: {line:?}"
                        );
                    }
                    if !(plain.starts_with("✕ Error:") || plain.starts_with("▲ Warning:")) {
                        assert!(
                            !line.contains('\x1b'),
                            "supporting text is styled: {line:?}"
                        );
                    }
                }
                case.assert_output(&text);
                let comparable =
                    case.comparable(&text.replace("▲ ", "").replace("\n✕ Error:", "\nError:"));
                if let Some(previous) = &previous {
                    assert_eq!(
                        &comparable, previous,
                        "plain/styled meaning changed for {label}"
                    );
                }
                previous = Some(comparable);
                output.evidence(&format!("phase3-{label}-{mode}-{width}"));
            }
        }
    }
}

#[test]
fn operation_families_obey_explicit_disables_ci_dumb_and_each_redirected_stream() {
    for label in [
        "name-blank",
        "repo-nested",
        "unsafe-yaml-edit",
        "duplicate-warning",
        "replacement-create-permission",
    ] {
        for control in [
            "no-color",
            "clicolor-zero",
            "ci",
            "dumb",
            "stdin",
            "stdout",
            "stderr",
        ] {
            let case = operation_diagnostic_support::Case::new(label);
            let capture = tempfile::NamedTempFile::new().unwrap();
            let mut environment = vec![("FORCE_COLOR", "1")];
            match control {
                "no-color" => environment.push(("NO_COLOR", "1")),
                "clicolor-zero" => environment.push(("CLICOLOR", "0")),
                "ci" => environment.push(("CI", "true")),
                "dumb" => environment.push(("TERM", "dumb")),
                _ => {}
            }
            let arguments = case
                .arguments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let output = add_pty_support::run(
                &case.app,
                &arguments,
                add_pty_support::Settings {
                    environment: &environment,
                    redirect: ["stdin", "stdout", "stderr"]
                        .contains(&control)
                        .then_some((control, capture.path())),
                    ..Default::default()
                },
            );
            assert_eq!(
                output.status.exit_code(),
                1,
                "{label} {control}: {}",
                output.plain()
            );
            let redirected = std::fs::read_to_string(capture.path()).unwrap();
            let combined = format!("{}{redirected}", output.raw);
            assert!(
                !combined.contains(['\x1b', '✕', '▲', '┌', '│', '└']),
                "{label} {control}: {combined}"
            );
            case.assert_output(&combined.replace('\r', ""));
            output.evidence(&format!("phase3-controls-{label}-{control}"));
        }
    }
}
