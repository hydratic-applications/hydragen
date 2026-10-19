//! Presentation-only adaptation of clap errors from the static CLI definition.

mod catalog;
mod facts;

use std::error::Error as _;

use clap::error::{ContextKind, ContextValue, ErrorFormatter, ErrorKind, RichFormatter};
use hydragen::diagnostic_text::display;

use crate::diagnostic_body::Diagnostic;
use crate::presentation::Presentation;
use catalog::{Catalog, Origin};

impl Diagnostic {
    fn for_usage(summary: String, catalog: &Catalog<'_>, origin: Origin) -> Self {
        let entry = catalog.entry(origin);
        Self {
            summary,
            details: Vec::new(),
            usage: entry.map(|entry| entry.usage.clone()),
            example: None,
            help: Some(entry.map_or_else(
                || "hydragen --help".to_owned(),
                |entry| format!("{} --help", entry.path),
            )),
            ..Self::default()
        }
    }
}

/// The command and its error come from the same built, static, plain-styled CLI.
/// Only clap-generated Usage/static help can retain layout; data leaves cannot.
pub fn render(
    mut error: clap::Error,
    command: &clap::Command,
    presentation: Presentation,
) -> String {
    // This catalog is constructed only here: successful parsing and explicit
    // help/version never pay for it. It reads metadata, never validators or argv.
    let catalog = Catalog::new(command);
    if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
        let origin = catalog.pseudo_help_origin(&error.render().ansi().to_string());
        if origin != Origin::Unknown
            && error.source().is_none()
            && let Some(model) = facts::adapt(&error, &catalog, origin)
        {
            return model.render(presentation);
        }
        return opaque(&error, &catalog, Origin::Unknown).render(presentation);
    }

    // Escape leaves before either the model or the rich-format consistency gate.
    // Unknown context kinds/shapes cannot become unchecked presentation layout.
    let Some(context) = error
        .context()
        .map(|(kind, value)| safe_context(kind, value).map(|value| (kind, value)))
        .collect::<Option<Vec<_>>>()
    else {
        return opaque(&error, &catalog, Origin::Unknown).render(presentation);
    };
    for (kind, value) in context {
        error.insert(kind, value);
    }

    // Error::render can bypass RichFormatter for raw/preformatted payloads. There
    // is no public message accessor: compare against the supported formatter,
    // after sanitising its inputs. The finished English body is never interpreted
    // or patched, nor used to identify the error kind or originating command.
    let generated = RichFormatter::format_error(&error).ansi().to_string();
    if error.render().ansi().to_string() != generated {
        return opaque(&error, &catalog, Origin::Unknown).render(presentation);
    }
    let origin = catalog.resolve(&error);
    // External validation sources cannot be mutated and are opaque data, not
    // generated prose. Retain their safe explanation in the same outer model.
    if error.source().is_none()
        && let Some(model) = facts::adapt(&error, &catalog, origin)
    {
        return model.render(presentation);
    }
    opaque(&error, &catalog, origin).render(presentation)
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

fn opaque(error: &clap::Error, catalog: &Catalog<'_>, origin: Origin) -> Diagnostic {
    // Preserve encoded content, including hostile ANSI, then escape it as ONE
    // unstructured leaf. Display on StyledStr would silently strip ANSI instead.
    let mut model = Diagnostic::for_usage(
        "Could not parse command-line arguments.".into(),
        catalog,
        origin,
    );
    model
        .details
        .push(format!("Details: {}", display(error.render().ansi())));
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
        let label = match kind {
            ContextKind::InvalidArg | ContextKind::PriorArg => "Arguments",
            ContextKind::InvalidSubcommand => "Command",
            ContextKind::InvalidValue => "Value",
            ContextKind::ValidValue => "Possible values",
            ContextKind::ValidSubcommand => "Commands",
            ContextKind::ActualNumValues => "Values supplied",
            ContextKind::ExpectedNumValues => "Values required",
            ContextKind::MinValues => "Minimum values",
            ContextKind::Suggested
            | ContextKind::SuggestedArg
            | ContextKind::SuggestedValue
            | ContextKind::SuggestedSubcommand
            | ContextKind::SuggestedCommand => "Tip",
            _ => "Details",
        };
        if !payload.is_empty() {
            model.details.push(format!("{label}: {}", display(payload)));
        }
    }
    if let Some(source) = error.source() {
        model.details.push(format!("Cause: {}", display(source)));
    }
    model
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
    fn approved_missing_source_and_invalid_format_bodies_share_block_grammar() {
        for (arguments, expected) in [
            (
                vec!["hydragen", "add"],
                "Error: Add requires at least one source.\n\n  Usage: hydragen add [OPTIONS] <SOURCE>...\n  Example: hydragen add ../shared-config\n\n  For more information, try 'hydragen add --help'.\n",
            ),
            (
                vec!["hydragen", "init", "--format", "yam"],
                "Error: Invalid value 'yam' for '--format <FORMAT>'.\n\n  Possible values: yaml, yml, toml, json, jsonc\n  Tip: Did you mean 'yaml'?\n\n  Usage: hydragen init [OPTIONS]\n  Example: hydragen init --format yaml\n\n  For more information, try 'hydragen init --help'.\n",
            ),
        ] {
            let (error, command) = failure(&arguments);
            let text = render(error, &command, plain());
            assert_eq!(text, expected);
        }
    }

    #[test]
    fn pseudohelp_uses_command_choices_not_a_second_document_template() {
        let (error, command) = failure(&["hydragen"]);
        assert_eq!(
            error.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
        let text = render(error, &command, plain());
        assert!(text.contains("Commands: init, add, help"), "{text}");
        assert!(!text.contains("Options:"));
        assert!(!text.contains("Build projects from shared layers"));
        assert!(text.ends_with("For more information, try 'hydragen --help'.\n"));
    }

    #[test]
    fn structured_errors_keep_choices_and_suggestions_under_shared_headlines() {
        for (arguments, kind, details) in [
            (
                vec!["hydragen", "init", "--format", "jsno"],
                ErrorKind::InvalidValue,
                vec![
                    "Error: Invalid value 'jsno'",
                    "--format <FORMAT>",
                    "yaml, yml, toml, json, jsonc",
                    "json",
                    "Tip:",
                ],
            ),
            (
                vec!["hydragen", "init", "--nme", "rules"],
                ErrorKind::UnknownArgument,
                vec![
                    "Error: Unexpected argument '--nme'",
                    "--name",
                    "Tip:",
                    "Usage:",
                ],
            ),
            (
                vec!["hydragen", "ad"],
                ErrorKind::InvalidSubcommand,
                vec!["Error: Unrecognized subcommand 'ad'", "add", "Tip:"],
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
    fn actual_conflicts_use_structured_context_without_new_application_arguments() {
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
        assert_family(&text);
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
                assert_family(&text);
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
                assert!(
                    !text.contains("\n  Usage:"),
                    "raw origin must stay unknown: {text}"
                );
                assert!(text.contains("For more information, try 'hydragen --help'."));
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
            assert_family(&text);
            assert!(
                text.starts_with("Error: Could not parse command-line arguments."),
                "{text}"
            );
            assert!(!text.contains("Context:"));
            for internal in [
                "InvalidArg:",
                "StyledStr(",
                "ContextKind",
                "ActualNumValues:",
            ] {
                assert!(!text.contains(internal), "{text}");
            }
            assert!(text.contains("spoof"));
            assert!(!text.contains('\x1b'));
            assert!(!text.contains("\nspoof"));
            assert!(!text.contains("\n  Usage:"));
            assert!(text.contains("For more information, try 'hydragen --help'."));
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

    fn assert_family(text: &str) {
        assert!(!text.contains("\n\n\n"), "{text}");
        for line in text.lines() {
            assert!(
                line.is_empty() || !line.chars().all(char::is_whitespace),
                "separator must be literally empty: {text:?}"
            );
        }
        let blocks = text
            .strip_suffix('\n')
            .expect("one final newline")
            .split("\n\n")
            .collect::<Vec<_>>();
        assert!(blocks[0].starts_with("Error: "), "{text}");
        assert!(blocks[0].ends_with('.'), "summary punctuation: {text}");
        assert_eq!(blocks[0].lines().count(), 1);
        assert!(
            blocks
                .last()
                .unwrap()
                .starts_with("  For more information, try 'hydragen")
        );
        assert!(blocks.last().unwrap().ends_with("--help'."));
        for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
            assert!(line.starts_with("  "), "base indent: {text}");
            assert!(!line.contains("[possible values:"));
            assert!(!line.contains("tip:"));
        }
        let mut seen_usage = false;
        for block in blocks.iter().skip(1).take(blocks.len() - 2) {
            if block.starts_with("  Usage:") {
                assert!(!seen_usage);
                seen_usage = true;
                let lines = block.lines().collect::<Vec<_>>();
                assert!(lines.len() <= 2, "at most one example: {text}");
                if lines.len() == 2 {
                    assert!(lines[1].starts_with("  Example: hydragen"));
                }
            } else {
                assert!(!seen_usage, "context must precede usage: {text}");
                assert!(!block.contains("Example:"));
            }
        }
    }

    #[test]
    fn all_thirteen_known_kinds_use_one_family_contract_from_typed_facts() {
        use ContextKind as K;
        use ContextValue as V;
        use ErrorKind as E;
        let command = command();
        let cases = [
            (
                E::ArgumentConflict,
                vec![
                    (K::InvalidArg, V::String("--left".into())),
                    (K::PriorArg, V::String("--right".into())),
                ],
                vec!["cannot be used with '--right'."],
            ),
            (
                E::NoEquals,
                vec![(K::InvalidArg, V::String("--value=<VALUE>".into()))],
                vec!["An equals sign is required"],
            ),
            (
                E::InvalidValue,
                vec![
                    (K::InvalidArg, V::String("--format <FORMAT>".into())),
                    (K::InvalidValue, V::String("yam".into())),
                    (
                        K::ValidValue,
                        V::Strings(vec!["yaml".into(), "json".into()]),
                    ),
                    (K::SuggestedValue, V::String("yaml".into())),
                ],
                vec!["Possible values: yaml, json", "Tip: Did you mean 'yaml'?"],
            ),
            (
                E::InvalidSubcommand,
                vec![
                    (K::InvalidSubcommand, V::String("ad".into())),
                    (K::SuggestedSubcommand, V::Strings(vec!["add".into()])),
                ],
                vec!["Unrecognized subcommand 'ad'.", "Tip: Did you mean 'add'?"],
            ),
            (
                E::MissingRequiredArgument,
                vec![(
                    K::InvalidArg,
                    V::Strings(vec!["<FIRST>".into(), "<SECOND>".into()]),
                )],
                vec!["Required arguments: <FIRST>, <SECOND>"],
            ),
            (
                E::MissingSubcommand,
                vec![
                    (K::InvalidSubcommand, V::String("hydragen".into())),
                    (
                        K::ValidSubcommand,
                        V::Strings(vec!["init".into(), "add".into()]),
                    ),
                ],
                vec!["requires a subcommand.", "Commands: init, add"],
            ),
            (E::InvalidUtf8, vec![], vec!["Invalid UTF-8 was detected"]),
            (
                E::TooManyValues,
                vec![
                    (K::InvalidArg, V::String("--name <NAME>".into())),
                    (K::InvalidValue, V::String("extra".into())),
                ],
                vec!["Unexpected value 'extra'", "no more values were expected."],
            ),
            (
                E::TooFewValues,
                vec![
                    (K::InvalidArg, V::String("--pair <PAIR>...".into())),
                    (K::MinValues, V::Number(2)),
                    (K::ActualNumValues, V::Number(1)),
                ],
                vec!["At least 2 values are required", "1 value was provided."],
            ),
            (
                E::ValueValidation,
                vec![
                    (K::InvalidArg, V::String("--value <VALUE>".into())),
                    (K::InvalidValue, V::String("bad".into())),
                ],
                vec!["Invalid value 'bad' for '--value <VALUE>'."],
            ),
            (
                E::WrongNumberOfValues,
                vec![
                    (K::InvalidArg, V::String("--pair <PAIR>...".into())),
                    (K::ExpectedNumValues, V::Number(2)),
                    (K::ActualNumValues, V::Number(3)),
                ],
                vec!["Expected 2 values", "3 values were provided."],
            ),
            (
                E::UnknownArgument,
                vec![
                    (K::InvalidArg, V::String("--nme".into())),
                    (K::SuggestedArg, V::String("--name".into())),
                    (
                        K::Suggested,
                        V::StyledStrs(vec![
                            "'init --name' exists".into(),
                            "to pass '--nme' as a value, use '-- --nme'".into(),
                        ]),
                    ),
                ],
                vec![
                    "Unexpected argument '--nme'.",
                    "Tip: Did you mean '--name'?",
                    "'init --name' exists.",
                    "to pass '--nme' as a value, use '-- --nme'.",
                ],
            ),
        ];
        assert_eq!(cases.len(), 12);
        for (kind, context, snippets) in cases {
            let make_error = || {
                let mut error = clap::Error::new(kind).with_cmd(&command);
                error.insert(K::Usage, V::StyledStr(command.clone().render_usage()));
                for (key, value) in &context {
                    error.insert(*key, value.clone());
                }
                error
            };
            let catalog = Catalog::new(&command);
            let error = make_error();
            assert!(
                facts::adapt(&error, &catalog, catalog.resolve(&error)).is_some(),
                "{kind:?} must use the facts adapter, not fallback"
            );
            let text = render(error, &command, plain());
            assert_family(&text);
            assert!(!text.contains("Could not parse"), "{kind:?}: {text}");
            for snippet in snippets {
                assert!(text.contains(snippet), "{kind:?}, {snippet}: {text}");
            }
            let styled = render(
                make_error(),
                &command,
                Presentation {
                    decorated: true,
                    basic: false,
                },
            );
            assert_eq!(console::strip_ansi_codes(&styled), format!("✕ {text}"));
            assert!(styled.lines().skip(1).all(|line| !line.contains('\x1b')));
        }
        let (error, command) = failure(&["hydragen"]);
        assert_eq!(error.kind(), E::DisplayHelpOnMissingArgumentOrSubcommand);
        assert_family(&render(error, &command, plain()));
    }

    #[test]
    fn optional_blocks_have_no_filler_and_usage_example_are_always_one_group() {
        let command = command();
        let catalog = Catalog::new(&command);
        let mut model =
            Diagnostic::for_usage("A problem occurred.".into(), &catalog, Origin::Unknown);
        let text = model.render(plain());
        assert_family(&text);
        assert_eq!(text.strip_suffix('\n').unwrap().split("\n\n").count(), 2);
        for label in ["Usage:", "Example:", "Tip:", "Possible values:", "Context:"] {
            assert!(!text.contains(label));
        }
        model.usage = Some("Usage: hydragen [OPTIONS] <COMMAND>".into());
        model.example = Some("hydragen init --format yaml");
        model.details = vec![
            "Possible values: yaml, json".into(),
            "Tip: Did you mean 'yaml'?".into(),
        ];
        let text = model.render(plain());
        assert_family(&text);
        assert_eq!(text.strip_suffix('\n').unwrap().split("\n\n").count(), 4);
        assert!(text.contains("<COMMAND>\n  Example:"));
    }

    #[test]
    fn repeated_use_plural_conflicts_empty_values_and_counts_preserve_their_meaning() {
        use ContextKind as K;
        use ContextValue as V;
        let command = command();
        for (prior, expected) in [
            (V::String("--left".into()), "cannot be used multiple times."),
            (
                V::Strings(vec!["--one".into(), "--two".into()]),
                "Conflicting arguments: '--one', '--two'",
            ),
            (V::None, "cannot be used with other arguments."),
        ] {
            let mut error = clap::Error::new(ErrorKind::ArgumentConflict).with_cmd(&command);
            error.insert(K::InvalidArg, V::String("--left".into()));
            error.insert(K::PriorArg, prior);
            let text = render(error, &command, plain());
            assert_family(&text);
            assert!(text.contains(expected), "{text}");
        }
        let (error, command) = failure(&["hydragen", "init", "--format"]);
        let text = render(error, &command, plain());
        assert_family(&text);
        assert!(text.starts_with("Error: A value is required for '--format <FORMAT>'."));
        assert!(text.contains("Possible values: yaml, yml, toml, json, jsonc"));
        assert!(text.contains("Usage: hydragen init [OPTIONS]"));
        assert!(text.contains("'hydragen init --help'."));
    }

    #[test]
    fn parent_value_error_after_child_selection_keeps_parent_scope_without_replaying_validators() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let validator_calls = Arc::clone(&calls);
        let mut command = clap::Command::new("hydragen")
            .bin_name("hydragen")
            .styles(clap::builder::Styles::plain())
            .color(clap::ColorChoice::Never)
            .arg(
                clap::Arg::new("parent")
                    .long("parent")
                    .value_parser(["good"]),
            )
            .subcommand(clap::Command::new("child").arg(
                clap::Arg::new("probe").long("probe").value_parser(
                    move |value: &str| -> Result<String, String> {
                        validator_calls.fetch_add(1, Ordering::SeqCst);
                        Ok(value.to_owned())
                    },
                ),
            ));
        command.build();
        let error = command
            .try_get_matches_from_mut(["hydragen", "--parent", "child", "child", "--probe", "good"])
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidValue);
        assert!(error.get(ContextKind::Usage).is_none());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "clap parsed the child before rejecting the parent value"
        );
        let text = render(error, &command, plain());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "metadata adaptation must not call validators again"
        );
        assert_family(&text);
        assert!(
            text.contains("Invalid value 'child' for '--parent <parent>'."),
            "{text}"
        );
        assert!(
            text.contains("Usage: hydragen [OPTIONS] [COMMAND]"),
            "{text}"
        );
        assert!(text.contains("'hydragen --help'."));
        assert!(!text.contains("hydragen child --help"));
    }

    #[test]
    fn ambiguous_global_and_unsupported_origins_offer_only_general_help_without_scoped_examples() {
        use clap::{Arg, Command};
        for graph in [
            Command::new("hydragen")
                .subcommand(
                    Command::new("one")
                        .arg(Arg::new("format").long("format").value_parser(["yaml"])),
                )
                .subcommand(
                    Command::new("two")
                        .arg(Arg::new("format").long("format").value_parser(["yaml"])),
                ),
            Command::new("hydragen")
                .arg(
                    Arg::new("format")
                        .long("format")
                        .global(true)
                        .value_parser(["yaml"]),
                )
                .subcommand(Command::new("one")),
            Command::new("hydragen").subcommand(
                Command::new("one")
                    .alias("alias")
                    .arg(Arg::new("format").long("format").value_parser(["yaml"])),
            ),
        ] {
            let mut graph = graph
                .bin_name("hydragen")
                .styles(clap::builder::Styles::plain())
                .color(clap::ColorChoice::Never);
            graph.build();
            let error = graph
                .try_get_matches_from_mut(["hydragen", "one", "--format", "yam"])
                .unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidValue);
            assert!(error.get(ContextKind::Usage).is_none());
            let text = render(error, &graph, plain());
            assert_family(&text);
            assert!(text.contains("Possible values: yaml"));
            assert!(text.contains("'hydragen --help'."));
            assert!(!text.contains("\n  Usage:"));
            assert!(!text.contains("Example:"));
            assert!(!text.contains("one --help"));
        }
    }

    #[test]
    fn real_clap_equals_and_count_constructors_match_the_fact_adapter() {
        use clap::{Arg, Command};
        for (argument, arguments, kind, snippets) in [
            (
                Arg::new("value").long("value").require_equals(true),
                vec!["hydragen", "--value", "one"],
                ErrorKind::NoEquals,
                vec!["An equals sign is required", "--value=<value>"],
            ),
            (
                Arg::new("value").num_args(1..=2),
                vec!["hydragen", "one", "two", "three"],
                ErrorKind::TooManyValues,
                vec!["Unexpected value 'three'", "no more values were expected."],
            ),
            (
                Arg::new("value").long("value").num_args(3..),
                vec!["hydragen", "--value", "one"],
                ErrorKind::TooFewValues,
                vec!["At least 3 values are required", "1 value was provided."],
            ),
            (
                Arg::new("value").long("value").num_args(2),
                vec!["hydragen", "--value", "one"],
                ErrorKind::WrongNumberOfValues,
                vec!["Expected 2 values", "1 value was provided."],
            ),
        ] {
            let mut command = Command::new("hydragen")
                .bin_name("hydragen")
                .styles(clap::builder::Styles::plain())
                .color(clap::ColorChoice::Never)
                .arg(argument);
            command.build();
            let error = command.try_get_matches_from_mut(arguments).unwrap_err();
            assert_eq!(error.kind(), kind);
            let text = render(error, &command, plain());
            assert_family(&text);
            assert!(!text.contains("Could not parse"), "{kind:?}: {text}");
            for snippet in snippets {
                assert!(text.contains(snippet), "{kind:?}, {snippet}: {text}");
            }
        }
    }

    #[test]
    fn unexpected_extra_or_mistyped_facts_do_not_silently_disappear() {
        use ContextKind as K;
        use ContextValue as V;
        let command = command();
        for context in [
            vec![
                (K::InvalidArg, V::String("--arg".into())),
                (K::PriorArg, V::None),
                (K::ActualNumValues, V::Number(17)),
            ],
            vec![
                (K::InvalidArg, V::Strings(vec!["--wrong-shape".into()])),
                (K::InvalidSubcommand, V::String("child".into())),
                (K::PriorArg, V::None),
            ],
        ] {
            let mut error = clap::Error::new(ErrorKind::ArgumentConflict).with_cmd(&command);
            for (kind, value) in context {
                error.insert(kind, value);
            }
            let text = render(error, &command, plain());
            assert_family(&text);
            assert!(
                text.starts_with("Error: Could not parse command-line arguments."),
                "{text}"
            );
            assert!(!text.contains("ActualNumValues:"));
            assert!(!text.contains("Context:"));
            assert!(
                text.contains("17") || text.contains("--wrong-shape"),
                "opaque fallback must retain the extra fact: {text}"
            );
        }
    }
}
