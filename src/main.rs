mod author_discovery;
mod diagnostics;
mod interaction;
mod presentation;

use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use hydragen::{LayerManifestFormat, Metadata, create_init, inspect_init};
use interaction::Mode;

const INIT_HELP: &str = r#"Examples:
  hydragen init
  hydragen init --name shared-rules
  hydragen init \
    --name shared-agent-rules \
    --author "Example team" \
    --description "Shared agent rules."
  hydragen init --non-interactive
  hydragen init --format json

Input:
  No input flags: guided prompts only when stdin, stdout and stderr are terminals and CI is disabled.
  Any input flag: flag-only input; no follow-up questions. Input flags are --name, --author, --description and --format.
  Name defaults to the current directory name. Author and description are optional and omitted when not supplied.
  Guided Author offers project-declared authors first; other suggestions can come from Git, local accounts and optional GitHub lookups, with grouped source hints. Choose a name, Enter another author, or Skip. With no suggestions or after Enter another author, use an empty optional input; empty text and Skip omit author.
  Optional GitHub requests use only existing gh authentication, never login or extra permissions. Offline use and use without gh are supported. Flag-only input performs no author discovery.
  Format defaults to yaml (layer.yaml). --format selects only the newly created layer manifest, not read-time priority.
  --non-interactive always disables prompts."#;

#[derive(Parser)]
#[command(
    name = "hydragen",
    version,
    color = clap::ColorChoice::Never,
    max_term_width = 100,
    about = "Build projects from shared layers"
)]
struct Cli {
    /// Never prompt, even in a terminal
    #[arg(long, global = true)]
    non_interactive: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialise a layer in the current directory
    Init(InitArgs),
}

#[derive(Args)]
#[command(next_line_help = true, after_help = INIT_HELP)]
struct InitArgs {
    /// Optional name override (defaults to the current directory name; must not be blank)
    #[arg(long)]
    name: Option<String>,

    /// Optional person or organisation credited with authoring the layer
    #[arg(long)]
    author: Option<String>,

    /// Optional explanation of what the layer provides or is used for
    #[arg(long)]
    description: Option<String>,

    /// Layer manifest format (defaults to yaml, creating layer.yaml)
    #[arg(long, value_parser = ["yaml", "yml", "toml", "json", "jsonc"])]
    format: Option<String>,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) if error.kind() == clap::error::ErrorKind::DisplayVersion => {
            return match writeln!(io::stdout().lock(), "{}", env!("CARGO_PKG_VERSION")) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            };
        }
        Err(error) => error.exit(),
    };
    let presentation = presentation::Presentation::detect();
    presentation.configure();
    match run(cli, presentation) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !interaction::is_reported_cancellation(error.as_ref()) {
                let _ = writeln!(
                    io::stderr().lock(),
                    "{}",
                    presentation.diagnostic(
                        presentation::Severity::Error,
                        &diagnostics::init_error_message(error.as_ref())
                    )
                );
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli, presentation: presentation::Presentation) -> Result<(), Box<dyn Error>> {
    let directory = std::env::current_dir().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Could not determine the current directory: {error}"),
        )
    })?;
    run_in(
        cli,
        &directory,
        presentation,
        Mode::detect,
        author_discovery::Preparation::start,
        interaction::collect_init_input,
    )
}

// The same boundary is injected in tests to observe *all* discovery startup,
// including file/account work that a fake git/gh executable cannot see.
fn run_in(
    cli: Cli,
    directory: &std::path::Path,
    presentation: presentation::Presentation,
    select_mode: impl FnOnce(bool, bool) -> Mode,
    start: impl FnOnce(&std::path::Path) -> io::Result<author_discovery::Preparation>,
    collect: impl FnOnce(
        &str,
        &interaction::GuidedSession,
        &author_discovery::Preparation,
    ) -> io::Result<interaction::InitInput>,
) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Command::Init(arguments) => {
            let input_flags = arguments.name.is_some()
                || arguments.author.is_some()
                || arguments.description.is_some()
                || arguments.format.is_some();
            let mode = select_mode(input_flags, cli.non_interactive);
            let inspection = inspect_init(directory)?;
            diagnostics::warn_duplicates(inspection.discovery(), presentation);
            let prepared = inspection.prepare()?;
            let author = prepare_author(
                mode,
                directory,
                hydragen::layer_manifest::schema::ensure_ready,
                start,
            )?;
            // Keep the theme through selection, writing, and the guided outro;
            // dropping it resets cliclack on every success or error path.
            let session =
                (mode == Mode::Guided).then(|| interaction::GuidedSession::install(presentation));
            let interaction::InitInput { metadata, format } = if mode == Mode::Guided {
                let (default_name, preparation) = author.as_ref().expect("guided preparation");
                collect(
                    default_name,
                    session.as_ref().expect("guided mode installs its theme"),
                    preparation,
                )?
            } else {
                let format = arguments.format.as_deref().map_or_else(
                    LayerManifestFormat::default,
                    |extension| {
                        LayerManifestFormat::ALL
                            .into_iter()
                            .find(|format| format.extension() == extension)
                            .expect("clap validates format choices")
                    },
                );
                interaction::InitInput {
                    metadata: flag_metadata(arguments, directory)?,
                    format,
                }
            };
            let created = create_init(prepared, &metadata, format).inspect_err(|failure| {
                if let Some(discovery) = &failure.discovery {
                    diagnostics::warn_duplicates(discovery, presentation);
                }
            })?;
            diagnostics::report_observation(&created, presentation);
            interaction::finish(mode, &created.path, presentation)?;
        }
    }
    Ok(())
}

fn prepare_author(
    mode: Mode,
    directory: &std::path::Path,
    ready: impl FnOnce() -> Result<(), hydragen::layer_manifest::schema::Error>,
    start: impl FnOnce(&std::path::Path) -> io::Result<author_discovery::Preparation>,
) -> io::Result<Option<(String, author_discovery::Preparation)>> {
    ready().map_err(io::Error::other)?;
    if mode == Mode::FlagOnly {
        return Ok(None);
    }
    let default_name = interaction::directory_name(directory)?;
    let preparation = start(directory)?;
    Ok(Some((default_name, preparation)))
}

fn flag_metadata(
    arguments: InitArgs,
    directory: &std::path::Path,
) -> Result<Metadata, Box<dyn Error>> {
    // Resolve defaults after mode selection: an inferred name is not a supplied flag.
    let name = match arguments.name {
        Some(name) => name,
        None => interaction::directory_name(directory)?,
    };
    let metadata = Metadata {
        name,
        author: arguments.author,
        description: arguments.description,
    };
    metadata.validate()?;
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use std::path::Path;

    fn help_at_width(width: usize, init: bool, long: bool) -> String {
        let mut root = Cli::command();
        root.build();
        let mut command = if init {
            root.find_subcommand("init").unwrap().clone()
        } else {
            root
        }
        .term_width(width);
        if long {
            command.render_long_help().to_string()
        } else {
            command.render_help().to_string()
        }
    }

    #[test]
    fn help_wraps_prose_and_preserves_content_at_review_widths() {
        for width in [40, 80, 100] {
            for init in [false, true] {
                for long in [false, true] {
                    let help = help_at_width(width, init, long);
                    for line in help.lines() {
                        assert!(line.len() <= width, "width {width}: {line:?}");
                    }
                    assert!(!help.contains('\x1b'));
                    assert!(help.contains("Usage:"));
                    let prose = help.split_whitespace().collect::<Vec<_>>().join(" ");
                    assert!(prose.contains("--non-interactive"));
                    if init {
                        for phrase in [
                            "Layer manifest format (defaults to yaml, creating layer.yaml)",
                            "Optional name override",
                            "current directory name",
                            "Any input flag: flag-only input; no follow-up questions.",
                            "Input flags are --name, --author, --description and --format.",
                            "stdin, stdout and stderr are terminals and CI is disabled.",
                            "Author and description are optional and omitted when not supplied.",
                            "Guided Author offers project-declared authors first; other suggestions can come from Git, local accounts and optional GitHub lookups, with grouped source hints.",
                            "Choose a name, Enter another author, or Skip.",
                            "With no suggestions or after Enter another author, use an empty optional input; empty text and Skip omit author.",
                            "Optional GitHub requests use only existing gh authentication, never login or extra permissions.",
                            "Offline use and use without gh are supported.",
                            "Flag-only input performs no author discovery.",
                            "Format defaults to yaml (layer.yaml).",
                            "--format selects only the newly created layer manifest, not read-time priority.",
                            "--non-interactive always disables prompts.",
                        ] {
                            assert!(prose.contains(phrase), "{phrase}: {help}");
                        }
                        assert!(help.contains("--name <NAME>\n"));
                        assert!(help.contains("--author <AUTHOR>\n"));
                        assert!(help.contains("--description <DESCRIPTION>\n"));
                        assert!(help.contains("--format <FORMAT>\n"));
                        assert!(prose.contains("yaml, yml, toml, json, jsonc"));
                        let examples = INIT_HELP.split("\n\nInput:").next().unwrap();
                        assert!(help.contains(examples), "examples changed: {help}");
                    }
                }
            }
        }
    }

    #[test]
    fn help_keeps_an_unbreakable_token_without_exempting_other_prose() {
        let token = "x".repeat(60);
        let help = clap::Command::new("test")
            .term_width(40)
            .after_help(format!(
                "Short words before {token} and ordinary words after it."
            ))
            .render_help()
            .to_string();
        assert!(help.contains(&token), "{help}");
        for line in help.lines().filter(|line| line.len() > 40) {
            assert_eq!(line.trim(), token);
        }
    }

    #[test]
    fn explicit_names_bypass_directory_defaults_but_still_validate() {
        let arguments = |name: &str| InitArgs {
            name: Some(name.to_owned()),
            author: None,
            description: None,
            format: None,
        };
        assert_eq!(
            flag_metadata(arguments("explicit"), Path::new("/"))
                .unwrap()
                .name,
            "explicit"
        );
        assert!(flag_metadata(arguments(""), Path::new("valid-directory")).is_err());
    }

    #[test]
    fn production_route_starts_no_discovery_for_flags_or_disabled_interaction() {
        use std::cell::Cell;
        for arguments in [
            vec!["hydragen", "init"],
            vec!["hydragen", "init", "--name", "explicit"],
            vec!["hydragen", "init", "--author", " Explicit author "],
            vec!["hydragen", "init", "--description", "description"],
            vec!["hydragen", "init", "--format", "json"],
        ] {
            for non_interactive in [false, true] {
                for terminals in [
                    [true; 3],
                    [false, true, true],
                    [true, false, true],
                    [true, true, false],
                ] {
                    for ci in [None, Some(std::ffi::OsStr::new("true"))] {
                        let directory = tempfile::tempdir().unwrap();
                        let starts = Cell::new(0);
                        let mut cli = Cli::try_parse_from(&arguments).unwrap();
                        cli.non_interactive = non_interactive;
                        let guided = arguments.len() == 2
                            && !non_interactive
                            && terminals == [true; 3]
                            && ci.is_none();
                        let _ = run_in(
                            cli,
                            directory.path(),
                            presentation::Presentation {
                                decorated: false,
                                basic: true,
                            },
                            |flags, non_interactive| {
                                Mode::select(flags, non_interactive, terminals, ci)
                            },
                            |actual| {
                                assert_eq!(actual, directory.path());
                                starts.set(starts.get() + 1);
                                Err(io::Error::other("observed discovery startup"))
                            },
                            |_, _, _| panic!("injected startup never returns a handle"),
                        );
                        assert_eq!(
                            starts.get(),
                            usize::from(guided),
                            "{arguments:?}, {terminals:?}, {ci:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn early_exit_boundaries_never_start_discovery() {
        use std::cell::Cell;
        let starts = Cell::new(0);
        let directory = tempfile::tempdir().unwrap();
        for arguments in [
            vec!["hydragen", "--help"],
            vec!["hydragen", "init", "--help"],
            vec!["hydragen", "--version"],
            vec!["hydragen", "init", "--unknown"],
            vec!["hydragen", "init", "--format", "invalid"],
        ] {
            let dispatched = Cli::try_parse_from(arguments).map(|cli| {
                run_in(
                    cli,
                    directory.path(),
                    presentation::Presentation {
                        decorated: false,
                        basic: true,
                    },
                    |_, _| Mode::Guided,
                    |_| {
                        starts.set(starts.get() + 1);
                        Err(io::Error::other("unexpected discovery startup"))
                    },
                    |_, _, _| panic!("early argument exits cannot prompt"),
                )
            });
            assert!(dispatched.is_err());
        }
        std::fs::write(directory.path().join("layer.json"), "occupied").unwrap();
        assert!(
            run_in(
                Cli::try_parse_from(["hydragen", "init"]).unwrap(),
                directory.path(),
                presentation::Presentation {
                    decorated: false,
                    basic: true
                },
                |_, _| Mode::Guided,
                |_| {
                    starts.set(starts.get() + 1);
                    panic!("preflight must precede startup")
                },
                |_, _, _| panic!("preflight must precede input")
            )
            .is_err()
        );
        assert!(
            prepare_author(
                Mode::Guided,
                Path::new("/"),
                || Ok(()),
                |_| {
                    starts.set(starts.get() + 1);
                    panic!("directory default must validate before startup")
                }
            )
            .is_err()
        );
        assert!(
            prepare_author(
                Mode::Guided,
                Path::new("valid-directory"),
                || {
                    Err(hydragen::layer_manifest::schema::Error::Setup {
                        message: "injected readiness failure".into(),
                        detail: None,
                    })
                },
                |_| {
                    starts.set(starts.get() + 1);
                    panic!("schema readiness must precede startup")
                }
            )
            .is_err()
        );
        assert_eq!(starts.get(), 0);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("layer.json")).unwrap(),
            "occupied"
        );
    }
}
