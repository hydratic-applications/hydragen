//! One public-output contract, shared by captured and actual-terminal tests.

#[path = "../diagnostic_body_support/mod.rs"]
mod diagnostic_body_support;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Root,
    Init,
    Add,
}

impl Scope {
    pub fn command(self) -> &'static str {
        match self {
            Self::Root => "hydragen",
            Self::Init => "hydragen init",
            Self::Add => "hydragen add",
        }
    }

    pub fn usage(self) -> &'static str {
        match self {
            Self::Root => "Usage: hydragen [OPTIONS] <COMMAND>",
            Self::Init => "Usage: hydragen init [OPTIONS]",
            Self::Add => "Usage: hydragen add [OPTIONS] <SOURCE>...",
        }
    }
}

pub struct Case {
    pub label: &'static str,
    pub arguments: &'static [&'static str],
    pub scope: Scope,
    pub facts: &'static [&'static str],
    pub example: Option<&'static str>,
}

pub const CASES: &[Case] = &[
    Case {
        label: "root-missing-command",
        arguments: &[],
        scope: Scope::Root,
        facts: &["command", "Commands:", "init", "add"],
        example: None,
    },
    Case {
        label: "root-global-only",
        arguments: &["--non-interactive"],
        scope: Scope::Root,
        facts: &["command", "Commands:", "init", "add"],
        example: None,
    },
    Case {
        label: "root-unknown-flag",
        arguments: &["--bogus"],
        scope: Scope::Root,
        facts: &["--bogus"],
        example: None,
    },
    Case {
        label: "root-unknown-command",
        arguments: &["inti"],
        scope: Scope::Root,
        facts: &["inti", "Tip:", "init"],
        example: None,
    },
    Case {
        label: "init-unknown-flag",
        arguments: &["init", "--nme", "rules"],
        scope: Scope::Init,
        facts: &["--nme", "Tip:", "--name"],
        example: None,
    },
    Case {
        label: "add-unknown-flag",
        arguments: &["add", "--path", "../rules"],
        scope: Scope::Add,
        facts: &["--path", "Tip:"],
        example: None,
    },
    Case {
        label: "add-global-typo",
        arguments: &["add", "--non-interactiv"],
        scope: Scope::Add,
        facts: &["--non-interactiv", "Tip:", "--non-interactive"],
        example: None,
    },
    Case {
        label: "init-invalid-format",
        arguments: &["init", "--format", "yam"],
        scope: Scope::Init,
        facts: &[
            "Invalid value 'yam' for '--format <FORMAT>'.",
            "Possible values: yaml, yml, toml, json, jsonc",
            "Tip: Did you mean 'yaml'?",
        ],
        example: Some("hydragen init --format yaml"),
    },
    Case {
        label: "init-invalid-format-no-suggestion",
        arguments: &["init", "--format", "xml"],
        scope: Scope::Init,
        facts: &[
            "--format",
            "xml",
            "Possible values: yaml, yml, toml, json, jsonc",
        ],
        example: None,
    },
    Case {
        label: "init-missing-name",
        arguments: &["init", "--name"],
        scope: Scope::Init,
        facts: &["--name", "<NAME>", "value"],
        example: None,
    },
    Case {
        label: "init-missing-author",
        arguments: &["init", "--author"],
        scope: Scope::Init,
        facts: &["--author", "<AUTHOR>", "value"],
        example: None,
    },
    Case {
        label: "init-missing-description",
        arguments: &["init", "--description"],
        scope: Scope::Init,
        facts: &["--description", "<DESCRIPTION>", "value"],
        example: None,
    },
    Case {
        label: "init-missing-format",
        arguments: &["init", "--format"],
        scope: Scope::Init,
        facts: &[
            "--format",
            "value",
            "Possible values: yaml, yml, toml, json, jsonc",
        ],
        example: None,
    },
    Case {
        label: "init-repeat-conflict",
        arguments: &["init", "--name", "first", "--name", "second"],
        scope: Scope::Init,
        facts: &["--name"],
        example: None,
    },
    Case {
        label: "add-missing-source",
        arguments: &["add"],
        scope: Scope::Add,
        facts: &["Add requires at least one source."],
        example: Some("hydragen add ../shared-config"),
    },
];

/// Accept a normalised LF transcript, with or without the decorative cross.
/// This checks the grammar itself, not a separate snapshot for every error kind.
pub fn assert_body(text: &str, expected_scope: Option<Scope>) {
    let blocks = diagnostic_body_support::assert_compact(text);
    let lines = text.lines().collect::<Vec<_>>();
    for line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }
        let content = line.trim_start();
        for label in ["Possible values:", "Tip:", "Usage:", "Example:"] {
            if let Some(value) = content.strip_prefix(label) {
                assert!(!value.trim().is_empty(), "empty placeholder: {line:?}");
                assert_eq!(*line, format!("  {content}"), "label indentation: {line:?}");
            }
        }
        if content.starts_with("Possible values:") {
            assert!(
                !content.contains(['[', ']']),
                "bracketed choice style: {line:?}"
            );
        }
    }
    assert!(
        matches!(blocks.len(), 3 | 4),
        "expected headline, optional details, usage/example, footer: {blocks:?}"
    );
    let usage_block = &blocks[blocks.len() - 2];
    assert!(
        usage_block[0].starts_with("  Usage: "),
        "usage is not the penultimate block: {text:?}"
    );
    assert!(
        matches!(usage_block.len(), 1 | 2),
        "usage/example grouping: {text:?}"
    );
    if usage_block.len() == 2 {
        assert!(
            usage_block[1].starts_with("  Example: hydragen "),
            "example must directly follow usage: {text:?}"
        );
    }
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.trim_start().starts_with("Usage:"))
            .count(),
        1
    );
    assert!(
        lines
            .iter()
            .filter(|line| line.trim_start().starts_with("Example:"))
            .count()
            <= 1
    );
    if blocks.len() == 4 {
        let mut saw_correction = false;
        for line in &blocks[1] {
            assert!(!line.trim_start().starts_with("Usage:"));
            assert!(!line.trim_start().starts_with("Example:"));
            assert!(!line.contains("For more information,"));
            if line.trim_start().starts_with("Tip:") {
                saw_correction = true;
            }
            if line.trim_start().starts_with("Possible values:")
                || line.trim_start().starts_with("Commands:")
            {
                assert!(!saw_correction, "context must precede correction: {text:?}");
            }
        }
    }
    let footer = blocks.last().unwrap();
    assert_eq!(
        footer.len(),
        1,
        "footer must be its own final block: {text:?}"
    );
    let scope = [Scope::Root, Scope::Init, Scope::Add]
        .into_iter()
        .find(|scope| {
            footer[0] == format!("  For more information, try '{} --help'.", scope.command())
        })
        .expect("footer must name a full trusted help command");
    if let Some(expected_scope) = expected_scope {
        assert_eq!(scope, expected_scope, "wrong help command: {text:?}");
    }
    assert_eq!(
        usage_block[0],
        format!("  {}", scope.usage()),
        "usage and help must share canonical command context: {text:?}"
    );
    if usage_block.len() == 2 {
        assert_ne!(
            scope,
            Scope::Root,
            "root errors have no verified scoped example: {text:?}"
        );
        assert!(
            usage_block[1].starts_with(&format!("  Example: {} ", scope.command())),
            "example belongs to a different command: {text:?}"
        );
    }
    for forbidden in [
        "Options:",
        "Sources:",
        "Behaviour:",
        "Build projects from shared layers",
        "Context: InvalidArg",
        "Context: Usage",
    ] {
        assert!(
            !text.contains(forbidden),
            "unsolicited help or internal debug context: {text:?}"
        );
    }
}

pub fn assert_case(text: &str, case: &Case) {
    assert_body(text, Some(case.scope));
    for fact in case.facts {
        assert!(
            text.contains(fact),
            "{} lost {fact:?}: {text:?}",
            case.label
        );
    }
    if let Some(example) = case.example {
        assert!(
            text.contains(&format!("  Example: {example}\n")),
            "{} lost useful example: {text:?}",
            case.label
        );
    }
    let suggested = match case.label {
        "root-unknown-command" => Some("'init'"),
        "init-unknown-flag" => Some("'--name'"),
        "add-global-typo" => Some("'--non-interactive'"),
        _ => None,
    };
    if let Some(suggested) = suggested {
        assert!(
            text.lines()
                .any(|line| line.starts_with("  Tip:") && line.contains(suggested)),
            "{} lost its actual suggested correction {suggested}: {text:?}",
            case.label
        );
    }
    if case.label == "init-repeat-conflict" {
        assert!(
            [
                "more than once",
                "multiple times",
                "repeated",
                "only be used once"
            ]
            .iter()
            .any(|fact| text.contains(fact)),
            "repeat-count fact lost: {text:?}"
        );
    }
}
