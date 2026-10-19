#![cfg(unix)]

mod add_pty_support;
mod diagnostic_body_support;
mod operation_diagnostic_support;
mod output_failure_support;
mod permission_support;

#[test]
fn terminal_operations_share_captured_contract_and_plain_styled_meaning_at_review_widths() {
    for label in operation_diagnostic_support::labels() {
        for width in [40, 80, 100, 120] {
            let mut previous = None;
            for (mode, environment) in [("styled", &[][..]), ("plain", &[("NO_COLOR", "1")][..])] {
                let case = match operation_diagnostic_support::Case::try_new(&label) {
                    Ok(permission_support::Setup::Ready(case)) => {
                        if matches!(
                            label.as_str(),
                            "init-open-permission" | "replacement-create-permission"
                        ) {
                            eprintln!(
                                "CAPABILITY {label}: native create denial verified; running terminal assertions"
                            );
                        }
                        case
                    }
                    Ok(permission_support::Setup::Unsupported(reason)) => {
                        eprintln!("SKIP {label}: {reason}");
                        continue;
                    }
                    Err(error) => panic!("{label}: fixture setup failed: {error}"),
                };
                // Completion-failure fixtures deliberately redirect stdout and
                // cannot be styled. Use their group-owned bounded runner rather
                // than placing a FIFO reader under the generic one-shot PTY guard.
                if !case.terminal_eligible {
                    let output = case.captured(environment);
                    assert_eq!(output.status.code(), Some(1), "{label}: {output:?}");
                    assert!(output.stdout.is_empty());
                    let text = String::from_utf8(output.stderr).unwrap();
                    assert!(!text.contains('\x1b'));
                    case.assert_output(&text);
                    let comparable = case.comparable(&text);
                    if let Some(previous) = &previous {
                        assert_eq!(&comparable, previous);
                    }
                    previous = Some(comparable);
                    if let Some(directory) = std::env::var_os("HYDRAGEN_OPERATION_EVIDENCE") {
                        std::fs::create_dir_all(&directory).unwrap();
                        std::fs::write(
                            std::path::Path::new(&directory)
                                .join(format!("phase4-{label}-{mode}-{width}.plain.txt")),
                            text,
                        )
                        .unwrap();
                    }
                    continue;
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
            let case = match operation_diagnostic_support::Case::try_new(label) {
                Ok(permission_support::Setup::Ready(case)) => {
                    if label == "replacement-create-permission" {
                        eprintln!(
                            "CAPABILITY {label}: native create denial verified; running control assertions"
                        );
                    }
                    case
                }
                Ok(permission_support::Setup::Unsupported(reason)) => {
                    eprintln!("SKIP {label}: {reason}");
                    continue;
                }
                Err(error) => panic!("{label}: fixture setup failed: {error}"),
            };
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
