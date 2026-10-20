//! Compact CLI composition, after adapters have escaped every untrusted leaf.
//!
//! Strings here are presentation text, not raw errors or paths. Do not escape a
//! completed body: its line breaks are intentional. This module does no I/O.

use crate::presentation::{Presentation, Severity};

#[derive(Default)]
pub(crate) struct Diagnostic {
    pub summary: String,
    pub details: Vec<String>,
    pub correction: Option<String>,
    pub groups: Vec<SourceGroup>,
    pub usage: Option<String>,
    pub example: Option<&'static str>,
    pub help: Option<String>,
    /// Supplied by the operation's control flow, never inferred from its cause.
    pub state: Option<String>,
}

pub(crate) struct SourceGroup {
    pub label: String,
    pub summary: String,
    pub details: Vec<String>,
    pub correction: Option<String>,
}

fn supporting_lines(details: &[String], correction: Option<&str>) -> Vec<String> {
    let mut lines = details.to_vec();
    if let Some(correction) = correction {
        lines.push(format!("Tip: {correction}"));
    }
    lines
}

impl Diagnostic {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            ..Self::default()
        }
    }

    /// No status prefix, base indentation, or trailing newline. Existing compact
    /// delivery can pass this to Presentation just as it passes other bodies.
    pub fn body(&self) -> String {
        let mut blocks = vec![self.summary.clone()];
        let lines = supporting_lines(&self.details, self.correction.as_deref());
        if !lines.is_empty() {
            blocks.push(lines.join("\n"));
        }
        for group in &self.groups {
            let mut lines = vec![group.label.clone(), format!("  {}", group.summary)];
            for line in supporting_lines(&group.details, group.correction.as_deref()) {
                for line in line.lines() {
                    lines.push(format!("  {line}"));
                }
            }
            blocks.push(lines.join("\n"));
        }
        if let Some(usage) = &self.usage {
            let mut block = usage.clone();
            if let Some(example) = self.example {
                block.push_str(&format!("\nExample: {example}"));
            }
            blocks.push(block);
        }
        if let Some(help) = &self.help {
            blocks.push(format!("For more information, try '{help}'."));
        }
        if let Some(state) = &self.state {
            blocks.push(state.clone());
        }
        blocks.join("\n\n")
    }

    pub fn render(&self, presentation: Presentation) -> String {
        format!(
            "{}\n",
            presentation.diagnostic(Severity::Error, &self.body())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydragen::diagnostic_text::display;

    #[test]
    fn omitted_blocks_have_no_labels_or_filler_and_example_requires_usage() {
        let mut model = Diagnostic::new("A problem occurred.");
        model.example = Some("hydragen init");
        assert_eq!(model.body(), "A problem occurred.");
        model.state = Some("The layer manifest was already updated.".into());
        assert_eq!(
            model.body(),
            "A problem occurred.\n\nThe layer manifest was already updated."
        );
        assert!(!model.body().contains("No files"));
    }

    #[test]
    fn optional_blocks_order_groups_and_indent_once_without_ordinals() {
        let model = Diagnostic {
            summary: "Dependency checks failed (2 errors).".into(),
            details: vec!["Paths are relative to '/work/app'.".into()],
            groups: vec![
                SourceGroup {
                    label: "Source '../first'".into(),
                    summary: "No layer manifest found.".into(),
                    details: vec![],
                    correction: Some("Choose a layer directory.\nThen retry.".into()),
                },
                SourceGroup {
                    label: "Existing dependency '../second'".into(),
                    summary: "Invalid contents.".into(),
                    details: vec!["Field: /name".into()],
                    correction: None,
                },
            ],
            usage: Some("Usage: hydragen add [OPTIONS] <SOURCE>...".into()),
            example: Some("hydragen add ../rules"),
            help: Some("hydragen add --help".into()),
            state: Some("No files were changed.".into()),
            ..Diagnostic::default()
        };
        let plain = model.render(Presentation {
            decorated: false,
            basic: false,
        });
        assert_eq!(
            plain,
            concat!(
                "Error: Dependency checks failed (2 errors).\n\n",
                "  Paths are relative to '/work/app'.\n\n",
                "  Source '../first'\n    No layer manifest found.\n",
                "    Tip: Choose a layer directory.\n    Then retry.\n\n",
                "  Existing dependency '../second'\n    Invalid contents.\n    Field: /name\n\n",
                "  Usage: hydragen add [OPTIONS] <SOURCE>...\n  Example: hydragen add ../rules\n\n",
                "  For more information, try 'hydragen add --help'.\n\n  No files were changed.\n"
            )
        );
        let styled = model.render(Presentation {
            decorated: true,
            basic: false,
        });
        assert_eq!(console::strip_ansi_codes(&styled), format!("✕ {plain}"));
        assert!(styled.lines().skip(1).all(|line| !line.contains('\x1b')));
    }

    #[test]
    fn escaped_leaves_cannot_inject_layout_in_any_block() {
        let raw = "配置 🦀\\literal\x1b[2J\n\r\t\u{85}\u{202e}";
        let safe = display(raw).to_string();
        let model = Diagnostic {
            summary: format!("Problem '{safe}'."),
            details: vec![format!("Cause: {safe}")],
            correction: Some(format!("Check '{safe}'.")),
            groups: vec![SourceGroup {
                label: format!("Source '{safe}'"),
                summary: format!("Problem '{safe}'."),
                details: vec![format!("Cause: {safe}")],
                correction: Some(format!("Check '{safe}'.")),
            }],
            state: Some(format!("Created '{safe}'.")),
            ..Diagnostic::default()
        };
        let body = model.body();
        assert_eq!(body.matches(&safe).count(), 8);
        assert!(body.chars().all(|ch| !ch.is_control() || ch == '\n'));
        assert!(!body.contains('\u{202e}'));
        assert!(body.contains("配置 🦀\\literal"));
    }
}
