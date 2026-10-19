use std::ffi::OsStr;
use std::io::{self, IsTerminal};

use console::Style;

// The blank braille cells, including the two-cell visible inset, are intentional.
const MOTIF: [&str; 8] = [
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⣰⣾⣿⣿⣷⡄⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⣿⣿⡁⠈⠻⡿⠀⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠙⠿⣿⡦⢠⣶⣄⠀⠀⠀⠀⠀⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠙⠿⠃⠺⣿⣶⣄⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⣾⣦⡀⢈⣿⣿⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠘⢿⣿⣿⡿⠏⠀⠀",
    "⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀",
];

pub fn ci_enabled(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| {
        let value = value.to_string_lossy();
        !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Presentation {
    pub decorated: bool,
    pub basic: bool,
}

impl Presentation {
    pub fn detect() -> Self {
        Self::select(
            [
                io::stdin().is_terminal(),
                io::stdout().is_terminal(),
                io::stderr().is_terminal(),
            ],
            std::env::var_os("CI").as_deref(),
            std::env::var_os("TERM").as_deref(),
            std::env::var_os("NO_COLOR").as_deref(),
            std::env::var_os("CLICOLOR").as_deref(),
        )
    }

    fn select(
        terminals: [bool; 3],
        ci: Option<&OsStr>,
        term: Option<&OsStr>,
        no_color: Option<&OsStr>,
        clicolor: Option<&OsStr>,
    ) -> Self {
        let basic = term == Some(OsStr::new("dumb"));
        let disabled =
            no_color.is_some_and(|value| !value.is_empty()) || clicolor == Some(OsStr::new("0"));
        Self {
            decorated: terminals.into_iter().all(|terminal| terminal)
                && !ci_enabled(ci)
                && !basic
                && !disabled,
            basic,
        }
    }

    pub fn configure(self) {
        // Some cliclack defaults use stdout styles even though prompts use stderr.
        // Explicit decisions also prevent force variables bypassing our stream gate.
        console::set_colors_enabled(self.decorated);
        console::set_colors_enabled_stderr(self.decorated);
    }

    pub fn style(self) -> Style {
        Style::new().force_styling(self.decorated)
    }

    pub fn init_header(self, width: Option<usize>, version: &str) -> String {
        let identity = format!(
            "{}  {}",
            self.style().bold().apply_to("Hydragen"),
            self.style().dim().apply_to(format!("v{version}"))
        );
        let operation = self.style().cyan().bold().apply_to("INIT").to_string();
        let description = "Initialise a layer";
        let text = [identity, operation, description.to_owned()];
        let needed = 17
            + text
                .iter()
                .map(|line| console::measure_text_width(line))
                .max()
                .unwrap_or(0);
        if self.basic || width.is_none_or(|width| width < needed) {
            return format!("{}\n\n{}\n{}\n", text[0], text[1], text[2]);
        }
        let mut header = String::new();
        for (row, motif) in MOTIF.iter().enumerate() {
            header.push_str(&self.style().cyan().apply_to(motif).to_string());
            if let Some(line) = match row {
                2 => Some(&text[0]),
                4 => Some(&text[1]),
                5 => Some(&text[2]),
                _ => None,
            } {
                header.push(' ');
                header.push_str(line);
            }
            header.push('\n');
        }
        header
    }

    pub fn success(self, filename: &str) -> String {
        let message = format!(
            "Layer initialised, layer manifest created at: {}",
            self.style().bold().apply_to(filename)
        );
        if self.decorated {
            self.style()
                .green()
                .apply_to(format!("✓ {message}"))
                .to_string()
        } else {
            message
        }
    }

    pub fn diagnostic(self, severity: Severity, message: &str) -> String {
        let mut lines = message.lines();
        let summary = lines.next().unwrap_or("");
        let headline = match severity.label() {
            Some(label) => format!("{label}: {summary}"),
            None => summary.to_owned(),
        };
        let mut output = if self.decorated {
            severity
                .headline_style(self)
                .apply_to(format!("{} {headline}", severity.symbol()))
                .to_string()
        } else {
            headline
        };
        for line in lines {
            output.push_str("\n  ");
            output.push_str(line);
        }
        output
    }

    /// A successfully completed add can have a neutral, informational no-op result.
    pub fn add_success(self, plan: &hydragen::dependencies::AddPlan) -> String {
        let name = &plan.current().metadata.name;
        let (symbol, headline_style) = if plan.is_noop() {
            (Severity::Info.symbol(), Severity::Info.headline_style(self))
        } else {
            ("✓", self.style().green())
        };
        let single = match (plan.added(), plan.already_present()) {
            ([entry], []) => Some(("Added ", &entry.name, " to ")),
            ([], [entry]) => Some((
                "No changes needed: ",
                &entry.name,
                " is already a dependency of ",
            )),
            _ => None,
        };
        if let Some((prefix, dependency, relation)) = single {
            let text = headline_style;
            let names = text.clone().bold();
            let prefix = if self.decorated {
                format!("{symbol} {prefix}")
            } else {
                prefix.to_owned()
            };
            return format!(
                "{}{}{}{}{}\n",
                text.apply_to(prefix),
                names.apply_to(dependency),
                text.apply_to(relation),
                names.apply_to(name),
                text.apply_to(".")
            );
        }
        let headline = if plan.is_noop() {
            format!("No changes needed for {name}.")
        } else {
            format!("Dependencies updated for {name}.")
        };
        let mut output = if self.decorated {
            let prefix = if plan.is_noop() {
                "No changes needed for "
            } else {
                "Dependencies updated for "
            };
            format!(
                "{}{}{}",
                headline_style.apply_to(format!("{symbol} {prefix}")),
                headline_style.clone().bold().apply_to(name),
                headline_style.apply_to(".")
            )
        } else {
            headline
        };
        for (label, entries) in [
            ("Added", plan.added()),
            ("Already present", plan.already_present()),
        ] {
            if entries.is_empty() {
                continue;
            }
            output.push_str(&format!(
                "\n\n{} {}",
                self.style().bold().apply_to(label),
                self.style().dim().apply_to(format!("({})", entries.len()))
            ));
            for entry in entries {
                output.push_str("\n  ");
                output.push_str(&entry.name);
            }
        }
        output.push('\n');
        output
    }
}

#[derive(Clone, Copy)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    fn label(self) -> Option<&'static str> {
        match self {
            Self::Info => None,
            Self::Warning => Some("Warning"),
            Self::Error => Some("Error"),
        }
    }

    fn headline_style(self, presentation: Presentation) -> Style {
        match self {
            Self::Info => presentation.style().blue(),
            Self::Warning => presentation.style().yellow(),
            Self::Error => presentation.style().red(),
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Info => "ℹ",
            Self::Warning => "▲",
            Self::Error => "✕",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presentation(decorated: bool) -> Presentation {
        Presentation {
            decorated,
            basic: false,
        }
    }

    fn add_plan(existing: &[&str], requested: &[&str]) -> hydragen::dependencies::AddPlan {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app-directory");
        std::fs::create_dir(&app).unwrap();
        std::fs::write(
            app.join("layer.json"),
            serde_json::json!({"name": "@example/app", "dependencies": existing}).to_string(),
        )
        .unwrap();
        for directory in ["rules", "tooling", "other"] {
            let path = root.path().join(directory);
            std::fs::create_dir(&path).unwrap();
            std::fs::write(
                path.join("layer.yaml"),
                format!("name: '@example/{directory}'\n"),
            )
            .unwrap();
        }
        hydragen::dependencies::prepare_add(
            &app,
            &requested
                .iter()
                .map(|value| (*value).into())
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    #[test]
    fn single_add_results_use_verified_names_and_effective_outcome_cardinality() {
        for existing in [&[][..], &["../other"][..]] {
            for requested in [
                vec!["../rules"],
                vec!["../rules", "../rules"],
                vec!["../rules/./", "../rules"],
            ] {
                for already_present in [false, true] {
                    let mut existing = existing.to_vec();
                    if already_present {
                        existing.push("../rules");
                    }
                    let plan = add_plan(&existing, &requested);
                    let expected = if already_present {
                        "No changes needed: @example/rules is already a dependency of @example/app.\n"
                    } else {
                        "Added @example/rules to @example/app.\n"
                    };
                    assert_eq!(presentation(false).add_success(&plan), expected);
                    let styled = presentation(true).add_success(&plan);
                    let symbol = if already_present { "ℹ" } else { "✓" };
                    assert_eq!(
                        console::strip_ansi_codes(&styled),
                        format!("{symbol} {expected}")
                    );
                    let names = if already_present {
                        presentation(true).style().blue().bold()
                    } else {
                        presentation(true).style().green().bold()
                    };
                    for name in ["@example/rules", "@example/app"] {
                        assert!(
                            styled.contains(&names.apply_to(name).to_string()),
                            "{styled:?}"
                        );
                    }
                    assert_eq!(styled.lines().count(), 1);
                    assert!(styled.ends_with('\n'));
                }
            }
        }
    }

    #[test]
    fn multiple_mixed_and_internal_empty_add_results_keep_wording_and_group_layout() {
        let style = presentation(true).style();
        for (existing, requested, headline, groups) in [
            (
                vec![],
                vec!["../tooling", "../rules", "../tooling"],
                "Dependencies updated for ",
                vec![("Added", vec!["@example/tooling", "@example/rules"])],
            ),
            (
                vec!["../rules", "../tooling", "../other"],
                vec!["../tooling", "../rules", "../tooling"],
                "No changes needed for ",
                vec![(
                    "Already present",
                    vec!["@example/tooling", "@example/rules"],
                )],
            ),
            (
                vec!["../rules", "../other"],
                vec!["../tooling", "../rules"],
                "Dependencies updated for ",
                vec![
                    ("Added", vec!["@example/tooling"]),
                    ("Already present", vec!["@example/rules"]),
                ],
            ),
            (vec![], vec![], "No changes needed for ", vec![]),
        ] {
            let plan = add_plan(&existing, &requested);
            let mut plain = format!("{headline}@example/app.");
            let (symbol, headline_style) = if plan.is_noop() {
                ("ℹ", style.clone().blue())
            } else {
                ("✓", style.clone().green())
            };
            let mut styled = format!(
                "{}{}{}",
                headline_style.apply_to(format!("{symbol} {headline}")),
                headline_style.clone().bold().apply_to("@example/app"),
                headline_style.apply_to(".")
            );
            for (label, entries) in groups {
                let count = format!("({})", entries.len());
                plain.push_str(&format!("\n\n{label} {count}"));
                styled.push_str(&format!(
                    "\n\n{} {}",
                    style.clone().bold().apply_to(label),
                    style.clone().dim().apply_to(count)
                ));
                for name in entries {
                    plain.push_str(&format!("\n  {name}"));
                    styled.push_str(&format!("\n  {name}"));
                }
            }
            plain.push('\n');
            styled.push('\n');
            assert_eq!(presentation(false).add_success(&plan), plain);
            assert_eq!(presentation(true).add_success(&plan), styled);
        }
    }

    #[test]
    fn decoration_requires_every_terminal_and_respects_explicit_disables() {
        for mask in 0..8 {
            for (ci, ci_on) in [
                (None, false),
                (Some(""), false),
                (Some("FaLsE"), false),
                (Some("0"), false),
                (Some("1"), true),
            ] {
                for term in [None, Some("xterm-256color"), Some("dumb")] {
                    for no_color in [None, Some(""), Some("1")] {
                        for clicolor in [None, Some("0"), Some("1")] {
                            // Force is deliberately not an input: it cannot bypass a gate.
                            let policy = Presentation::select(
                                [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0],
                                ci.map(OsStr::new),
                                term.map(OsStr::new),
                                no_color.map(OsStr::new),
                                clicolor.map(OsStr::new),
                            );
                            assert_eq!(
                                policy.decorated,
                                mask == 7
                                    && !ci_on
                                    && term != Some("dumb")
                                    && no_color != Some("1")
                                    && clicolor != Some("0")
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn motif_and_text_preserve_the_selected_grid() {
        assert!(MOTIF.iter().all(|row| row.chars().count() == 16));
        let header = presentation(false).init_header(Some(80), "12.34.56");
        let rows = header.lines().collect::<Vec<_>>();
        assert_eq!(rows.len(), 8);
        for (index, row) in rows.iter().enumerate() {
            assert!(row.starts_with(MOTIF[index]));
        }
        assert_eq!(rows[2], format!("{} Hydragen  v12.34.56", MOTIF[2]));
        assert_eq!(rows[4], format!("{} INIT", MOTIF[4]));
        assert_eq!(rows[5], format!("{} Initialise a layer", MOTIF[5]));
    }

    #[test]
    fn header_fit_uses_actual_visible_version_and_conservative_fallbacks() {
        let policy = presentation(true);
        let version = "1.2.3-a-very-long-version";
        let full = policy.init_header(Some(100), version);
        let needed = full.lines().map(console::measure_text_width).max().unwrap();
        assert!(policy.init_header(Some(needed), version).contains(MOTIF[1]));
        for width in [None, Some(needed - 1)] {
            let fallback = policy.init_header(width, version);
            assert!(!fallback.contains('⠀'));
            assert!(
                console::strip_ansi_codes(&fallback).contains(&format!("Hydragen  v{version}"))
            );
        }
        assert!(
            !Presentation {
                basic: true,
                ..policy
            }
            .init_header(Some(100), version)
            .contains('⠀')
        );
        let description = full.lines().nth(5).unwrap().split_once(' ').unwrap().1;
        assert_eq!(description, "Initialise a layer");
    }

    #[test]
    fn shared_info_has_no_label_and_only_colours_the_headline() {
        let message = "No changes needed.\nThe requested layers were already present.";
        let plain = presentation(false).diagnostic(Severity::Info, message);
        assert_eq!(
            plain,
            "No changes needed.\n  The requested layers were already present."
        );
        let styled = presentation(true).diagnostic(Severity::Info, message);
        assert_eq!(
            styled,
            format!(
                "{}\n  The requested layers were already present.",
                presentation(true)
                    .style()
                    .blue()
                    .apply_to("ℹ No changes needed.")
            )
        );
        assert_eq!(console::strip_ansi_codes(&styled), format!("ℹ {plain}"));
        assert!(!styled.contains("Info:"));
        assert!(!styled.contains('✓'));
    }

    #[test]
    fn statuses_share_wording_and_colour_only_the_headline() {
        for severity in [Severity::Info, Severity::Warning, Severity::Error] {
            let plain = presentation(false).diagnostic(severity, "Headline\nRecovery /full/path");
            let styled = presentation(true).diagnostic(severity, "Headline\nRecovery /full/path");
            assert!(!plain.contains('\x1b'));
            assert!(styled.starts_with('\x1b'));
            assert_eq!(styled.lines().nth(1).unwrap(), "  Recovery /full/path");
            assert!(console::strip_ansi_codes(&styled).ends_with(&plain));
        }
        for format in hydragen::LayerManifestFormat::ALL {
            let plain = presentation(false).success(format.filename());
            assert_eq!(
                plain,
                format!(
                    "Layer initialised, layer manifest created at: {}",
                    format.filename()
                )
            );
            assert_eq!(
                console::strip_ansi_codes(&presentation(true).success(format.filename())),
                format!("✓ {plain}")
            );
        }
    }
}
