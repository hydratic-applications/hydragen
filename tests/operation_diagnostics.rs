mod diagnostic_body_support;
mod operation_diagnostic_support;
#[cfg(unix)]
mod output_failure_support;
mod permission_support;

#[test]
fn captured_operations_share_structure_without_flattening_write_state_or_constraints() {
    for label in operation_diagnostic_support::labels() {
        for environment in [
            &[][..],
            &[("CI", "1"), ("FORCE_COLOR", "1"), ("CLICOLOR_FORCE", "1")][..],
        ] {
            let case = match operation_diagnostic_support::Case::try_new(&label) {
                Ok(permission_support::Setup::Ready(case)) => {
                    if matches!(
                        label.as_str(),
                        "init-open-permission" | "replacement-create-permission"
                    ) {
                        eprintln!(
                            "CAPABILITY {label}: native create denial verified; running CLI assertions"
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
            let output = case.captured(environment);
            assert_eq!(output.status.code(), Some(1), "{label}: {output:?}");
            assert!(output.stdout.is_empty(), "{label}: {output:?}");
            let text = String::from_utf8(output.stderr).unwrap();
            assert!(
                !text.contains(['\x1b', '✕', '▲', '┌', '│', '└']),
                "{label}: {text}"
            );
            case.assert_output(&text);
            if environment.is_empty()
                && let Some(directory) = std::env::var_os("HYDRAGEN_OPERATION_EVIDENCE")
            {
                std::fs::create_dir_all(&directory).unwrap();
                std::fs::write(
                    std::path::Path::new(&directory)
                        .join(format!("phase3-captured-{label}.plain.txt")),
                    text,
                )
                .unwrap();
            }
        }
    }
}

#[test]
fn explicit_name_bypasses_invalid_defaults_without_rewriting_optional_metadata() {
    for label in operation_diagnostic_support::labels()
        .into_iter()
        .filter(|label| label.starts_with("default-"))
    {
        let mut case = operation_diagnostic_support::Case::new(&label);
        let failed = case.captured(&[]);
        assert_eq!(failed.status.code(), Some(1));
        case.assert_output(&String::from_utf8(failed.stderr).unwrap());
        let keep = case.app.join("keep.txt");
        let before = std::fs::metadata(&keep).unwrap();
        let raw = "  配置 \\literal\n\u{202e}\x1b[2J  ";
        case.arguments = [
            "init",
            "--name",
            "valid",
            "--author",
            raw,
            "--description",
            raw,
            "--format",
            "json",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let output = case.captured(&[]);
        assert!(output.status.success(), "{label}: {output:?}");
        assert!(output.stderr.is_empty());
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(case.app.join("layer.json")).unwrap()).unwrap();
        assert_eq!(
            document,
            serde_json::json!({"name":"valid", "author":{"name":raw}, "description":raw})
        );
        assert_eq!(
            std::fs::read_to_string(&keep).unwrap(),
            "untouched user content\n"
        );
        assert_eq!(
            std::fs::metadata(&keep).unwrap().modified().unwrap(),
            before.modified().unwrap()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(std::fs::metadata(&keep).unwrap().ino(), before.ino());
        }
        assert_eq!(std::fs::read_dir(&case.app).unwrap().count(), 2);
    }
}
