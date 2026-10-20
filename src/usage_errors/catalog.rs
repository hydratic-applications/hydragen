//! Failure-only metadata lookup, not parser replay or a universal clap origin API.
//!
//! clap 4.6 has no public error-command path. For the supported static graph,
//! generated Usage can name a complete canonical path; typed value errors without
//! Usage can identify one exact, uniquely owned, non-global argument display.
//! Unsupported graphs and ambiguous evidence explicitly remain Unknown.

use clap::{
    Command,
    error::{ContextKind, ContextValue, ErrorKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Origin {
    Known(usize),
    Unknown,
}

pub(super) struct Catalog<'a> {
    entries: Vec<Entry<'a>>,
    supported: bool,
}

pub(super) struct Entry<'a> {
    command: &'a Command,
    pub path: String,
    pub usage: String,
    arguments: Vec<Argument>,
}

struct Argument {
    id: String,
    display: String,
    global: bool,
}

pub(super) struct Recovery {
    path: &'static str,
    argument: &'static str,
    kind: ErrorKind,
    pub summary: Option<&'static str>,
    pub example: &'static str,
}

// Optional recovery content, never separate command-specific body templates.
const RECOVERIES: &[Recovery] = &[
    Recovery {
        path: "hydragen add",
        argument: "sources",
        kind: ErrorKind::MissingRequiredArgument,
        summary: Some("Add requires at least one source."),
        example: "hydragen add ../shared-config",
    },
    Recovery {
        path: "hydragen init",
        argument: "format",
        kind: ErrorKind::InvalidValue,
        summary: None,
        example: "hydragen init --format yaml",
    },
];

impl<'a> Catalog<'a> {
    pub fn new(command: &'a Command) -> Self {
        let mut catalog = Self {
            entries: Vec::new(),
            supported: true,
        };
        catalog.collect(command, "hydragen".into());
        catalog
    }

    fn collect(&mut self, command: &'a Command, path: String) {
        let has_children = command.get_subcommands().next().is_some();
        let usage = command
            .clone()
            .bin_name(path.clone())
            .render_usage()
            .ansi()
            .to_string();
        let supported = command.get_bin_name().is_none_or(|name| name == path)
            && command
                .get_name()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
            && !command.get_name().is_empty()
            && command.get_overridden_usage().is_none()
            && command.get_all_aliases().next().is_none()
            && command.get_short_flag().is_none()
            && command.get_long_flag().is_none()
            && command.get_all_short_flag_aliases().next().is_none()
            && command.get_all_long_flag_aliases().next().is_none()
            && !command.is_multicall_set()
            && !command.is_no_binary_name_set()
            && !command.is_allow_external_subcommands_set()
            && !command.is_subcommand_negates_reqs_set()
            && !(has_children
                && (command.get_arguments().any(|arg| arg.is_required_set())
                    || command.get_groups().any(|group| group.is_required_set())))
            && command.get_arguments().all(|arg| {
                arg.get_all_aliases()
                    .is_none_or(|aliases| aliases.is_empty())
                    && arg
                        .get_all_short_aliases()
                        .is_none_or(|aliases| aliases.is_empty())
            })
            && usage_tail(&usage, &path).is_some();
        self.supported &= supported;
        self.entries.push(Entry {
            command,
            path: path.clone(),
            usage,
            arguments: command
                .get_arguments()
                .map(|arg| Argument {
                    id: arg.get_id().to_string(),
                    display: arg.to_string(),
                    global: arg.is_global_set(),
                })
                .collect(),
        });
        for child in command.get_subcommands() {
            self.collect(child, format!("{path} {}", child.get_name()));
        }
    }

    pub fn entry(&self, origin: Origin) -> Option<&Entry<'a>> {
        match origin {
            Origin::Known(index) if self.supported => self.entries.get(index),
            _ => None,
        }
    }

    pub fn resolve(&self, error: &clap::Error) -> Origin {
        if !self.supported {
            return Origin::Unknown;
        }
        if let Some(usage) = error.get(ContextKind::Usage) {
            return match usage {
                ContextValue::StyledStr(usage) => self.usage_origin(&usage.ansi().to_string()),
                _ => Origin::Unknown,
            };
        }
        // Only typed argument/value errors can use exact argument ownership.
        // SuggestedArg and SuggestedValue are corrections, never provenance.
        if !matches!(
            error.kind(),
            ErrorKind::InvalidValue
                | ErrorKind::ValueValidation
                | ErrorKind::NoEquals
                | ErrorKind::TooManyValues
                | ErrorKind::TooFewValues
                | ErrorKind::WrongNumberOfValues
        ) {
            return Origin::Unknown;
        }
        let Some(ContextValue::String(invalid)) = error.get(ContextKind::InvalidArg) else {
            return Origin::Unknown;
        };
        let mut owners = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            for argument in &entry.arguments {
                if argument.display == *invalid {
                    if argument.global {
                        return Origin::Unknown;
                    }
                    owners.push(index);
                }
            }
        }
        match owners.as_slice() {
            [index] => Origin::Known(*index),
            _ => Origin::Unknown,
        }
    }

    fn usage_origin(&self, usage: &str) -> Origin {
        // The sole envelope exception: map only fixed generated Usage metadata,
        // longest COMPLETE canonical path first, never English diagnostic prose.
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| usage_tail(usage, &entry.path).is_some())
            .max_by_key(|(_, entry)| entry.path.len())
            .map_or(Origin::Unknown, |(index, _)| Origin::Known(index))
    }

    pub fn pseudo_help_origin(&self, encoded: &str) -> Origin {
        if !self.supported {
            return Origin::Unknown;
        }
        // Help is deliberately lazy: ordinary failures do not generate documents.
        let mut matches = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let help = entry.command.clone().render_help().ansi().to_string();
                (help == encoded).then_some(index)
            });
        match (matches.next(), matches.next()) {
            (Some(index), None) => Origin::Known(index),
            _ => Origin::Unknown,
        }
    }

    pub fn commands(&self, origin: Origin) -> Option<Vec<String>> {
        let entry = self.entry(origin)?;
        Some(
            entry
                .command
                .get_subcommands()
                .filter(|child| !child.is_hide_set())
                .map(|child| child.get_name().to_owned())
                .collect(),
        )
    }

    pub fn recovery(
        &self,
        origin: Origin,
        kind: ErrorKind,
        argument: &str,
    ) -> Option<&'static Recovery> {
        let entry = self.entry(origin)?;
        RECOVERIES.iter().find(|recovery| {
            recovery.path == entry.path
                && recovery.kind == kind
                && entry.arguments.iter().any(|arg| {
                    !arg.global && arg.id == recovery.argument && arg.display == argument
                })
        })
    }
}

fn usage_tail<'a>(usage: &'a str, path: &str) -> Option<&'a str> {
    let rest = usage.strip_prefix("Usage: ")?.strip_prefix(path)?;
    // Current supported synopses are single-line, plain, ordinary command paths
    // followed by generated argument syntax. A name prefix is not a boundary.
    if usage.chars().any(char::is_control) {
        return None;
    }
    if rest.is_empty() {
        return Some(rest);
    }
    let tail = rest.strip_prefix(' ')?;
    matches!(tail.chars().next(), Some('[' | '<' | '-')).then_some(tail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, CommandFactory};

    fn plain(command: Command) -> Command {
        command
            .bin_name("hydragen")
            .color(clap::ColorChoice::Never)
            .styles(clap::builder::Styles::plain())
    }

    fn value_error(argument: &str) -> clap::Error {
        let mut error = clap::Error::new(ErrorKind::InvalidValue);
        error.insert(
            ContextKind::InvalidArg,
            ContextValue::String(argument.into()),
        );
        error
    }

    #[test]
    fn current_static_graph_and_recovery_metadata_satisfy_catalog_invariants() {
        let mut command = crate::Cli::command();
        command.build();
        let catalog = Catalog::new(&command);
        assert!(catalog.supported);
        let paths = catalog
            .entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            [
                "hydragen",
                "hydragen init",
                "hydragen add",
                "hydragen help",
                "hydragen help init",
                "hydragen help add",
                "hydragen help help"
            ]
        );
        for (path, synopsis) in [
            ("hydragen", "Usage: hydragen [OPTIONS] <COMMAND>"),
            ("hydragen init", "Usage: hydragen init [OPTIONS]"),
            ("hydragen add", "Usage: hydragen add [OPTIONS] <SOURCE>..."),
        ] {
            let entry = catalog
                .entries
                .iter()
                .find(|entry| entry.path == path)
                .unwrap();
            assert_eq!(entry.usage, synopsis);
        }
        for content in RECOVERIES {
            let entry = catalog
                .entries
                .iter()
                .find(|entry| entry.path == content.path)
                .unwrap();
            assert_eq!(
                entry
                    .arguments
                    .iter()
                    .filter(|arg| arg.id == content.argument && !arg.global)
                    .count(),
                1
            );
            assert!(content.example.starts_with(&format!("{} ", entry.path)));
        }
        let format = command
            .find_subcommand("init")
            .unwrap()
            .get_arguments()
            .find(|argument| argument.get_id() == "format")
            .unwrap();
        assert!(
            format
                .get_possible_values()
                .iter()
                .any(|value| value.get_name() == "yaml")
        );
    }

    #[test]
    fn usage_origin_requires_a_complete_longest_canonical_path_and_generated_tail() {
        let mut command = plain(
            Command::new("hydragen")
                .subcommand(Command::new("in"))
                .subcommand(Command::new("init").subcommand(Command::new("deep"))),
        );
        command.build();
        let catalog = Catalog::new(&command);
        assert!(catalog.supported);
        for (usage, expected) in [
            (
                "Usage: hydragen init deep [OPTIONS]",
                Some("hydragen init deep"),
            ),
            ("Usage: hydragen init [OPTIONS]", Some("hydragen init")),
            ("Usage: hydragen in <VALUE>", Some("hydragen in")),
            ("Usage: hydragen [OPTIONS]", Some("hydragen")),
            ("Usage: hydragen initial [OPTIONS]", None),
            ("Usage: hydragenevil [OPTIONS]", None),
            ("Usage: hydragen init-deep [OPTIONS]", None),
            ("Usage: hydragen init\nspoof", None),
            ("error: Usage: hydragen init [OPTIONS]", None),
        ] {
            let origin = catalog.usage_origin(usage);
            assert_eq!(
                catalog.entry(origin).map(|entry| entry.path.as_str()),
                expected,
                "{usage}"
            );
            if expected.is_none() {
                assert_eq!(origin, Origin::Unknown);
            }
        }
    }

    #[test]
    fn exact_typed_ownership_is_unique_nonglobal_and_never_inferred_from_suggestions() {
        let mut command = plain(
            Command::new("hydragen")
                .arg(Arg::new("global").long("global").global(true))
                .subcommand(
                    Command::new("one")
                        .arg(Arg::new("shared").long("shared"))
                        .arg(Arg::new("unique").long("unique")),
                )
                .subcommand(Command::new("two").arg(Arg::new("shared").long("shared"))),
        );
        command.build();
        let catalog = Catalog::new(&command);
        assert!(catalog.supported);
        for invalid in [
            "--shared <shared>",
            "--global <global>",
            "--unknown <unknown>",
        ] {
            assert_eq!(catalog.resolve(&value_error(invalid)), Origin::Unknown);
        }
        let known = catalog.resolve(&value_error("--unique <unique>"));
        assert_eq!(catalog.entry(known).unwrap().path, "hydragen one");
        for kind in [
            ErrorKind::UnknownArgument,
            ErrorKind::InvalidSubcommand,
            ErrorKind::ArgumentConflict,
        ] {
            let mut error = clap::Error::new(kind);
            error.insert(
                ContextKind::InvalidArg,
                ContextValue::String("--unique <unique>".into()),
            );
            error.insert(
                ContextKind::SuggestedArg,
                ContextValue::String("--unique <unique>".into()),
            );
            assert_eq!(catalog.resolve(&error), Origin::Unknown);
        }
    }

    #[test]
    fn unsupported_command_graphs_fail_closed_instead_of_inventing_root_ownership() {
        for graph in [
            Command::new("hydragen")
                .arg(Arg::new("required").required(true))
                .subcommand(Command::new("child")),
            Command::new("hydragen").override_usage("something else"),
            Command::new("hydragen").subcommand(Command::new("child").alias("alias")),
            Command::new("hydragen").subcommand(Command::new("child").short_flag('c')),
            Command::new("hydragen").subcommand(Command::new("child").long_flag("child")),
            Command::new("hydragen").subcommand(Command::new("child").override_usage("special")),
            Command::new("hydragen").subcommand(Command::new("child").bin_name("different")),
            Command::new("hydragen").arg(Arg::new("value").long("value").alias("alias")),
            Command::new("hydragen").subcommand(Command::new("with space")),
            Command::new("hydragen").allow_external_subcommands(true),
        ] {
            let mut graph = plain(graph);
            graph.build();
            let catalog = Catalog::new(&graph);
            assert!(
                !catalog.supported,
                "graph should require a deliberate new origin policy: {graph:?}"
            );
            let mut error = value_error("--value <value>");
            error.insert(
                ContextKind::Usage,
                ContextValue::StyledStr("Usage: hydragen [OPTIONS]".into()),
            );
            assert_eq!(catalog.resolve(&error), Origin::Unknown);
            assert!(catalog.entry(Origin::Known(0)).is_none());
            assert_eq!(
                catalog.pseudo_help_origin(&graph.clone().render_help().ansi().to_string()),
                Origin::Unknown
            );
        }
    }

    #[test]
    fn pseudo_help_requires_one_unique_complete_static_help_document() {
        let mut command = plain(
            Command::new("hydragen")
                .help_template("identical help")
                .subcommand(Command::new("child").help_template("identical help")),
        );
        command.build();
        let catalog = Catalog::new(&command);
        assert!(catalog.supported);
        let encoded = command.clone().render_help().ansi().to_string();
        assert_eq!(catalog.pseudo_help_origin(&encoded), Origin::Unknown);
        assert_eq!(
            catalog.pseudo_help_origin("not a complete help document"),
            Origin::Unknown
        );
    }
}
