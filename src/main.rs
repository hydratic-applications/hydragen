mod interaction;

use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use hydragen::{Metadata, initialize, preflight};
use interaction::Mode;

const INIT_HELP: &str = r#"Examples:
  hydragen init
  hydragen init --name shared-rules
  hydragen init \
    --name shared-agent-rules \
    --author "Example team" \
    --description "Shared agent rules."
  hydragen init --non-interactive

Input:
  No metadata flags: guided prompts only when stdin, stdout and stderr are terminals and CI is disabled.
  Any metadata flag: flag-only input; no follow-up questions.
  Name defaults to the current directory name. Author and description are optional and omitted when not supplied.
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
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    let directory = std::env::current_dir().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("Could not determine the current directory: {error}"),
        )
    })?;
    match cli.command {
        Command::Init(arguments) => {
            let metadata_flags = arguments.name.is_some()
                || arguments.author.is_some()
                || arguments.description.is_some();
            let mode = Mode::detect(metadata_flags, cli.non_interactive);
            let metadata = if mode == Mode::Guided {
                preflight(&directory)?;
                interaction::collect_metadata(&interaction::directory_name(&directory)?)?
            } else {
                flag_metadata(arguments, &directory)?
            };
            let manifest = initialize(&directory, &metadata)?;
            interaction::finish(mode, &manifest)?;
        }
    }
    Ok(())
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
                            "Optional name override",
                            "current directory name",
                            "Any metadata flag: flag-only input; no follow-up questions.",
                            "stdin, stdout and stderr are terminals and CI is disabled.",
                            "Author and description are optional and omitted when not supplied.",
                            "--non-interactive always disables prompts.",
                        ] {
                            assert!(prose.contains(phrase), "{phrase}: {help}");
                        }
                        assert!(help.contains("--name <NAME>\n"));
                        assert!(help.contains("--author <AUTHOR>\n"));
                        assert!(help.contains("--description <DESCRIPTION>\n"));
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
        };
        assert_eq!(
            flag_metadata(arguments("explicit"), Path::new("/"))
                .unwrap()
                .name,
            "explicit"
        );
        assert!(flag_metadata(arguments(""), Path::new("valid-directory")).is_err());
    }
}
