use std::fs;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

mod add_diagnostic_support;
mod diagnostic_body_support;

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn universal_compact_rules_do_not_force_parser_sections_or_a_no_change_claim() {
    for text in [
        "Error: The operation failed.\n",
        "Error: Could not read './escaped\\nError: text'.\n",
        "Error: Completion output failed.\n  \n  The layer manifest was already updated.\n",
        "Error: Dependency checks failed (2 errors).\n  \n  Source '../a'\n    Could not read the file.\n    Tip: Check access permissions.\n  \n  Existing dependency '../b'\n    Invalid layer name.\n",
    ] {
        diagnostic_body_support::assert_compact(text);
    }
    for text in [
        "Error: unpunctuated\n",
        "Error: Failed.\nDetails without supporting indent.\n",
        "Error: Failed.\n  \n  \n  Tip: Retry.\n",
        "Error: Failed.\n  \n  Tip:\n",
        "Error: Failed.\n  \n  Details: forged\x1b[2J\n",
        "Error: Failed.\n  \n  Details: forged\u{202e}\n",
    ] {
        assert!(
            std::panic::catch_unwind(|| diagnostic_body_support::assert_compact(text)).is_err(),
            "accepted {text:?}"
        );
    }
}

#[test]
fn captured_verification_family_retains_facts_structure_and_exact_no_write_state() {
    for label in add_diagnostic_support::CASES {
        let case = add_diagnostic_support::Case::new(label);
        let stdout = tempfile::NamedTempFile::new().unwrap();
        let stderr = tempfile::NamedTempFile::new().unwrap();
        let mut child = Running(
            Command::new(env!("CARGO_BIN_EXE_hydragen"))
                .current_dir(&case.invocation)
                .args(&case.arguments)
                .env("CI", "1")
                .env("CLICOLOR_FORCE", "1")
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
            assert!(Instant::now() < deadline, "{label} timed out");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(1), "{label}");
        assert!(fs::read(stdout.path()).unwrap().is_empty(), "{label}");
        let text = fs::read_to_string(stderr.path()).unwrap();
        assert!(
            !text.contains(['\x1b', '✕', '┌', '│', '└']),
            "{label}: {text}"
        );
        case.assert_diagnostic(&text);
        if let Some(directory) = std::env::var_os("HYDRAGEN_ADD_EVIDENCE") {
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                std::path::Path::new(&directory).join(format!("phase2-captured-{label}.plain.txt")),
                text,
            )
            .unwrap();
        }
    }
}
