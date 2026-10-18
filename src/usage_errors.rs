//! Presentation-only adaptation of clap errors from the static CLI definition.

use std::error::Error as _;

use clap::error::{ContextKind, ContextValue, ErrorFormatter, ErrorKind, RichFormatter};
use hydragen::diagnostic_text::display;

use crate::presentation::{Presentation, Severity};

/// The command and its error come from the same built, static, plain-styled CLI.
/// Only clap-generated Usage/static help can retain layout; data leaves cannot.
pub fn render(
    mut error: clap::Error,
    command: &clap::Command,
    presentation: Presentation,
) -> String {
    if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
        let help = command.clone().render_help().ansi().to_string();
        if error.render().ansi().to_string() == help {
            return diagnostic(presentation, &format!("A command is required.\n\n{help}"));
        }
        return opaque(&error, command, presentation);
    }

    // The external validation source is private to clap and cannot be rewritten.
    // Unknown context kinds/shapes must never slip through the rich formatter.
    if error.source().is_some() || !rich_kind(error.kind()) {
        return opaque(&error, command, presentation);
    }
    let Some(context) = error
        .context()
        .map(|(kind, value)| safe_context(kind, value).map(|value| (kind, value)))
        .collect::<Option<Vec<_>>>()
    else {
        return opaque(&error, command, presentation);
    };
    for (kind, value) in context {
        error.insert(kind, value);
    }

    // Error::render can bypass RichFormatter for raw/preformatted payloads. There
    // is no public message accessor: compare against the supported formatter,
    // after sanitising its inputs, before trusting its generated envelope/layout.
    let generated = RichFormatter::format_error(&error).ansi().to_string();
    if error.render().ansi().to_string() != generated {
        return opaque(&error, command, presentation);
    }
    let Some(body) = generated.strip_prefix("error: ") else {
        return opaque(&error, command, presentation);
    };

    if missing_add_sources(&error, command) {
        let usage = command
            .find_subcommand("add")
            .expect("missing sources identified from add metadata")
            .clone()
            .render_usage();
        return diagnostic(
            presentation,
            &format!(
                "Add requires at least one source.\n\n{usage}\n\nExample: hydragen add ../shared-config\n\nFor more information, try 'hydragen add --help'."
            ),
        );
    }

    let mut body = body.to_owned();
    // RichFormatter starts these known kinds with generated prose (or a quote),
    // never a raw data leaf. Do not case-convert argument/value text.
    if let Some(first) = body.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    diagnostic(presentation, &body)
}

fn diagnostic(presentation: Presentation, body: &str) -> String {
    format!("{}\n", presentation.diagnostic(Severity::Error, body))
}

fn rich_kind(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::ArgumentConflict
            | ErrorKind::NoEquals
            | ErrorKind::InvalidValue
            | ErrorKind::InvalidSubcommand
            | ErrorKind::MissingRequiredArgument
            | ErrorKind::MissingSubcommand
            | ErrorKind::InvalidUtf8
            | ErrorKind::TooManyValues
            | ErrorKind::TooFewValues
            | ErrorKind::ValueValidation
            | ErrorKind::WrongNumberOfValues
            | ErrorKind::UnknownArgument
    )
}

fn safe_context(kind: ContextKind, value: &ContextValue) -> Option<ContextValue> {
    use ContextKind as K;
    use ContextValue as V;

    match (kind, value) {
        (
            K::InvalidSubcommand
            | K::InvalidArg
            | K::PriorArg
            | K::ValidSubcommand
            | K::ValidValue
            | K::InvalidValue
            | K::SuggestedCommand
            | K::SuggestedSubcommand
            | K::SuggestedArg
            | K::SuggestedValue,
            V::String(value),
        ) => Some(V::String(display(value).to_string())),
        (
            K::InvalidSubcommand
            | K::InvalidArg
            | K::PriorArg
            | K::ValidSubcommand
            | K::ValidValue
            | K::InvalidValue
            | K::SuggestedCommand
            | K::SuggestedSubcommand
            | K::SuggestedArg
            | K::SuggestedValue,
            V::Strings(values),
        ) => Some(V::Strings(
            values
                .iter()
                .map(|value| display(value).to_string())
                .collect(),
        )),
        (K::PriorArg, V::None) => Some(V::None),
        (K::ActualNumValues | K::ExpectedNumValues | K::MinValues, V::Number(value)) => {
            Some(V::Number(*value))
        }
        (K::TrailingArg, V::Bool(value)) => Some(V::Bool(*value)),
        (K::Suggested, V::StyledStrs(values)) => Some(V::StyledStrs(
            values
                .iter()
                .map(|value| display(value.ansi()).to_string().into())
                .collect(),
        )),
        // Usage is generated entirely from fixed bin_name and static definitions.
        (K::Usage, V::StyledStr(value)) => Some(V::StyledStr(value.clone())),
        // Custom/unknown kinds and unexpected value shapes are opaque, not layout.
        _ => None,
    }
}

fn missing_add_sources(error: &clap::Error, command: &clap::Command) -> bool {
    if error.kind() != ErrorKind::MissingRequiredArgument {
        return false;
    }
    let Some(source) = command.find_subcommand("add").and_then(|add| {
        add.get_arguments()
            .find(|argument| argument.get_id() == "sources")
    }) else {
        return false;
    };
    matches!(error.get(ContextKind::InvalidArg), Some(ContextValue::Strings(missing))
        if missing.as_slice() == [source.to_string()])
}

fn opaque(error: &clap::Error, command: &clap::Command, presentation: Presentation) -> String {
    // Preserve encoded content, including hostile ANSI, then escape it as ONE
    // unstructured leaf. Display on StyledStr would silently strip ANSI instead.
    let mut body = format!(
        "Could not parse command-line arguments.\n\n{}",
        display(error.render().ansi())
    );
    // A raw payload or unsupported formatter can ignore even known contexts.
    // Preserve every context as opaque data, reading styled content before any
    // ANSI-stripping Display. Debug is only for future non-exhaustive variants.
    for (kind, value) in error.context() {
        let payload = match value {
            ContextValue::String(value) => value.clone(),
            ContextValue::Strings(values) => values.join(", "),
            ContextValue::StyledStr(value) => value.ansi().to_string(),
            ContextValue::StyledStrs(values) => values
                .iter()
                .map(|value| value.ansi().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            ContextValue::None | ContextValue::Bool(_) | ContextValue::Number(_) => {
                value.to_string()
            }
            _ => format!("{value:?}"),
        };
        body.push_str(&format!("\nContext: {kind:?}: {}", display(payload)));
    }
    if let Some(source) = error.source() {
        body.push_str(&format!("\nCause: {}", display(source)));
    }
    // This usage is independently generated, not taken from the opaque payload.
    body.push_str(&format!(
        "\n\n{}\n\nFor more information, try 'hydragen --help'.",
        command.clone().render_usage()
    ));
    diagnostic(presentation, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, error::ErrorKind};

    fn plain() -> Presentation {
        Presentation {
            decorated: false,
            basic: false,
        }
    }

    fn command() -> clap::Command {
        let mut command = crate::Cli::command()
            .bin_name("hydragen")
            .styles(clap::builder::Styles::plain());
        command.build();
        command
    }

    fn failure(arguments: &[&str]) -> (clap::Error, clap::Command) {
        let mut command = command();
        let error = command.try_get_matches_from_mut(arguments).unwrap_err();
        (error, command)
    }

    #[test]
    fn missing_sources_have_one_concise_example_with_metadata_based_classification() {
        for arguments in [
            vec!["hydragen", "add"],
            vec!["hydragen", "--non-interactive", "add"],
            vec!["hydragen", "add", "--non-interactive"],
        ] {
            let (error, command) = failure(&arguments);
            assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
            let text = render(error, &command, plain());
            assert!(
                text.starts_with("Error: Add requires at least one source.\n"),
                "{text}"
            );
            assert!(text.contains("Usage: hydragen add"), "{text}");
            assert_eq!(
                text.matches("Example: hydragen add ../shared-config")
                    .count(),
                1
            );
            assert!(text.contains("For more information, try 'hydragen add --help'."));
            assert!(!text.contains("Sources:"));
        }
    }

    #[test]
    fn rich_errors_keep_choices_and_suggestions_under_shared_headlines() {
        for (arguments, kind, details) in [
            (
                vec!["hydragen", "init", "--format", "jsno"],
                ErrorKind::InvalidValue,
                vec![
                    "Error: Invalid value 'jsno'",
                    "--format <FORMAT>",
                    "yaml, yml, toml, json, jsonc",
                    "json",
                    "tip:",
                ],
            ),
            (
                vec!["hydragen", "init", "--nme", "rules"],
                ErrorKind::UnknownArgument,
                vec![
                    "Error: Unexpected argument '--nme'",
                    "--name",
                    "tip:",
                    "Usage:",
                ],
            ),
            (
                vec!["hydragen", "ad"],
                ErrorKind::InvalidSubcommand,
                vec!["Error: Unrecognized subcommand 'ad'", "add", "tip:"],
            ),
            (
                vec!["hydragen", "init", "--name"],
                ErrorKind::InvalidValue,
                vec!["Error: A value is required", "--name <NAME>"],
            ),
        ] {
            let (error, command) = failure(&arguments);
            assert_eq!(error.kind(), kind);
            let text = render(error, &command, plain());
            for detail in details {
                assert!(text.contains(detail), "{detail}: {text}");
            }
            assert!(!text.contains('\x1b'));
        }
    }

    #[test]
    fn controls_in_values_and_copied_suggestions_are_escaped_not_stripped() {
        let malicious = "--bad\x1b[2J\r\n\t\u{85}\u{202e}";
        let (error, command) = failure(&["hydragen", "add", malicious]);
        let text = render(error, &command, plain());
        let escaped = hydragen::diagnostic_text::display(malicious).to_string();
        assert!(
            text.matches(&escaped).count() >= 2,
            "argument must survive in headline and copied tip: {text:?}"
        );
        for control in ['\x1b', '\r', '\t', '\u{85}', '\u{202e}'] {
            assert!(!text.contains(control), "{text:?}");
        }
        assert!(
            text.contains("\n  Usage:"),
            "generated layout must survive: {text}"
        );
    }

    #[test]
    fn decoration_colours_only_the_headline_with_the_same_plain_details() {
        let (error, command) = failure(&["hydragen", "add"]);
        let plain_text = render(error, &command, plain());
        let (error, command) = failure(&["hydragen", "add"]);
        let styled = render(
            error,
            &command,
            Presentation {
                decorated: true,
                basic: false,
            },
        );
        assert!(styled.starts_with("\x1b[31m✕ Error:"), "{styled:?}");
        assert_eq!(
            console::strip_ansi_codes(&styled),
            format!("✕ {plain_text}")
        );
        assert!(styled.lines().skip(1).all(|line| !line.contains('\x1b')));
    }

    #[test]
    fn ordinary_unicode_and_literal_backslashes_survive_diagnostics() {
        let value = r"配置 🦀\n\x1b[2J";
        let (error, command) = failure(&["hydragen", "init", "--format", value]);
        let text = render(error, &command, plain());
        assert!(text.contains(value), "{text}");
        assert!(!text.contains("Could not parse"));
    }

    #[test]
    fn actual_conflicts_use_rich_context_without_new_application_arguments() {
        let mut command = clap::Command::new("hydragen")
            .bin_name("hydragen")
            .color(clap::ColorChoice::Never)
            .styles(clap::builder::Styles::plain())
            .arg(
                clap::Arg::new("left")
                    .long("left")
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with("right"),
            )
            .arg(
                clap::Arg::new("right")
                    .long("right")
                    .action(clap::ArgAction::SetTrue),
            );
        command.build();
        let error = command
            .try_get_matches_from_mut(["hydragen", "--left", "--right"])
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
        let text = render(error, &command, plain());
        assert!(
            text.starts_with("Error: The argument '--left' cannot be used with '--right'"),
            "{text}"
        );
        assert!(text.contains("Usage: hydragen"));
    }

    #[test]
    fn external_validation_sources_keep_encoded_payload_in_opaque_fallback() {
        let mut command = clap::Command::new("hydragen")
            .bin_name("hydragen")
            .color(clap::ColorChoice::Never)
            .styles(clap::builder::Styles::plain())
            .arg(
                clap::Arg::new("value").value_parser(|_: &str| -> Result<String, String> {
                    Err("external\x1b[2J\nspoof\u{202e}".to_owned())
                }),
            );
        command.build();
        let error = command
            .try_get_matches_from_mut(["hydragen", "value"])
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::ValueValidation);
        assert!(error.source().is_some());
        let text = render(error, &command, plain());
        assert!(text.starts_with("Error: Could not parse command-line arguments.\n"));
        assert!(text.contains(r"external\x1b[2J\nspoof\u{202e}"), "{text:?}");
        assert!(!text.contains('\x1b'));
        assert!(!text.contains("\nspoof"));
        assert!(text.contains("\n  Usage: hydragen"));
    }

    #[test]
    fn raw_and_preformatted_errors_cannot_claim_trusted_layout_or_labels() {
        for kind in [
            ErrorKind::UnknownArgument,
            ErrorKind::Io,
            ErrorKind::Format,
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
        ] {
            let mut command = command();
            for formatted in [false, true] {
                let error = clap::Error::raw(kind, "error: invented\x1b[2J\nUsage: spoof\r\n");
                let error = if formatted {
                    error.format(&mut command)
                } else {
                    error
                };
                let text = render(error, &command, plain());
                assert!(
                    text.starts_with("Error: Could not parse command-line arguments."),
                    "{text}"
                );
                assert!(
                    text.contains(r"invented\x1b[2J\nUsage: spoof\r\n"),
                    "{text:?}"
                );
                assert!(!text.contains('\x1b'));
                assert!(!text.contains("\nUsage: spoof"));
                assert!(text.contains("\n  Usage: hydragen"));
            }
        }
    }

    #[test]
    fn custom_and_unexpected_context_shapes_are_opaque_not_unchecked_rich_output() {
        use ContextKind as K;
        use ContextValue as V;
        for (kind, value) in [
            (K::Custom, V::String("custom\x1b[2J\nspoof".into())),
            (K::InvalidArg, V::StyledStr("styled\x1b[2J\nspoof".into())),
            (K::Suggested, V::StyledStr("suggested\x1b[2J\nspoof".into())),
            (
                K::Suggested,
                V::Strings(vec!["strings\x1b[2J\nspoof".into()]),
            ),
            (K::ActualNumValues, V::String("number\x1b[2J\nspoof".into())),
            (K::Usage, V::String("usage\x1b[2J\nspoof".into())),
        ] {
            let command = command();
            let mut error = clap::Error::new(ErrorKind::UnknownArgument).with_cmd(&command);
            error.insert(kind, value);
            let text = render(error, &command, plain());
            assert!(
                text.starts_with("Error: Could not parse command-line arguments."),
                "{text}"
            );
            assert!(text.contains("Context:"));
            assert!(text.contains("spoof"));
            assert!(!text.contains('\x1b'));
            assert!(!text.contains("\nspoof"));
            assert!(text.contains("\n  Usage: hydragen"));
        }
    }

    #[test]
    fn every_supported_string_list_and_suggestion_leaf_is_display_escaped() {
        use ContextKind as K;
        use ContextValue as V;
        let raw = "raw\x1b[2J\n\r\t\u{2028}\u{2066}";
        let escaped = display(raw).to_string();
        for kind in [
            K::InvalidSubcommand,
            K::InvalidArg,
            K::PriorArg,
            K::ValidSubcommand,
            K::ValidValue,
            K::InvalidValue,
            K::SuggestedCommand,
            K::SuggestedSubcommand,
            K::SuggestedArg,
            K::SuggestedValue,
        ] {
            assert_eq!(
                safe_context(kind, &V::String(raw.into())),
                Some(V::String(escaped.clone()))
            );
            assert_eq!(
                safe_context(kind, &V::Strings(vec![raw.into()])),
                Some(V::Strings(vec![escaped.clone()]))
            );
        }
        let value = safe_context(K::Suggested, &V::StyledStrs(vec![raw.into()])).unwrap();
        let V::StyledStrs(values) = value else {
            panic!("wrong suggestion shape")
        };
        assert_eq!(values[0].ansi().to_string(), escaped);
    }
}
