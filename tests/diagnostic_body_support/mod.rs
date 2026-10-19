//! Universal compact-status checks. Usage/help and operation state are separate
//! contracts: neither belongs on every error (or on successful no-op output).

pub fn assert_compact(text: &str) -> Vec<Vec<&str>> {
    assert_status(text, "✕ ", "Error: ")
}

#[allow(dead_code)]
pub fn assert_warning(text: &str) -> Vec<Vec<&str>> {
    assert_status(text, "▲ ", "Warning: ")
}

fn assert_status<'a>(text: &'a str, icon: &str, label: &str) -> Vec<Vec<&'a str>> {
    assert!(text.ends_with('\n'), "missing final newline: {text:?}");
    assert!(
        text.chars().all(|ch| !ch.is_control() || ch == '\n'),
        "unsafe compact text: {text:?}"
    );
    for ch in ['\u{2028}', '\u{2029}', '\u{202e}', '\u{2066}'] {
        assert!(!text.contains(ch), "unsafe compact text: {text:?}");
    }
    let lines = text.lines().collect::<Vec<_>>();
    let headline = lines[0].strip_prefix(icon).unwrap_or(lines[0]);
    let summary = headline
        .strip_prefix(label)
        .expect("one compact status headline");
    assert!(
        summary.chars().next().is_some_and(char::is_uppercase) || summary.starts_with('\''),
        "sentence-case summary: {text:?}"
    );
    assert!(
        summary.ends_with('.') && !summary.ends_with(".."),
        "summary punctuation: {text:?}"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.strip_prefix(icon).unwrap_or(line).starts_with(label))
            .count(),
        1,
        "one status headline, not occurrences within escaped data: {text:?}"
    );
    let mut blocks = vec![vec![lines[0]]];
    let mut after_blank = false;
    for line in &lines[1..] {
        if line.is_empty() {
            assert!(!after_blank, "more than one blank line: {text:?}");
            after_blank = true;
            continue;
        }
        assert!(
            !line.chars().all(char::is_whitespace),
            "separator must be literally empty: {text:?}"
        );
        assert!(
            line.starts_with("  "),
            "two-space supporting indent: {line:?}"
        );
        if after_blank {
            blocks.push(Vec::new());
            after_blank = false;
        }
        blocks.last_mut().unwrap().push(line);
        for obsolete in [
            "tip:",
            "possible values:",
            "[possible values:",
            "[Possible values:",
            "Examples:",
        ] {
            assert!(
                !line.trim_start().starts_with(obsolete),
                "old label: {line:?}"
            );
        }
        for label in ["Tip:", "Usage:", "Example:", "Possible values:"] {
            if let Some(value) = line.trim_start().strip_prefix(label) {
                assert!(!value.trim().is_empty(), "empty placeholder: {line:?}");
            }
        }
    }
    assert!(!after_blank, "trailing blank block: {text:?}");
    assert_eq!(
        blocks[0].len(),
        1,
        "summary must be its own block: {text:?}"
    );
    blocks
}

#[allow(dead_code)]
pub fn assert_verification(text: &str, failures: usize, labels: &[&str]) {
    let blocks = assert_compact(text);
    assert_eq!(text.matches("No files were changed.").count(), 1, "{text}");
    assert_eq!(
        blocks.last().unwrap().as_slice(),
        ["  No files were changed."]
    );
    for noise in [
        "source argument",
        "Could not add dependencies (",
        "Local paths resolve from",
        "Usage:",
        "Example:",
        "For more information,",
    ] {
        assert!(
            !text.contains(noise),
            "obsolete/unneeded wrapper {noise:?}: {text}"
        );
    }
    for line in text.lines() {
        assert!(
            !line
                .trim_start()
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit()),
            "outer ordinal: {line:?}"
        );
    }
    if failures == 1 {
        assert!(labels.is_empty());
        assert!(
            !text.contains("Dependency checks failed ("),
            "counted single failure: {text}"
        );
    } else {
        assert_eq!(labels.len(), failures);
        assert_eq!(
            text.lines()
                .next()
                .unwrap()
                .strip_prefix("✕ ")
                .unwrap_or(text.lines().next().unwrap()),
            format!("Error: Dependency checks failed ({failures} errors).")
        );
        let group_blocks = blocks
            .iter()
            .filter(|block| {
                block[0].starts_with("  Source '")
                    || block[0].starts_with("  Existing dependency '")
                    || block[0] == "  Current layer"
            })
            .collect::<Vec<_>>();
        assert_eq!(
            group_blocks.len(),
            failures,
            "missing failure groups: {text}"
        );
        for (block, label) in group_blocks.iter().zip(labels) {
            assert_eq!(
                block[0],
                format!("  {label}"),
                "group order/identity: {text}"
            );
            assert!(block.len() > 1, "empty group: {text}");
            for line in &block[1..] {
                assert!(
                    line.starts_with("    ") && !line.starts_with("     "),
                    "group indent: {line:?}"
                );
            }
        }
    }
}
