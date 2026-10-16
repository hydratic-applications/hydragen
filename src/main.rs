mod interaction;

use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use hydragen::{Metadata, initialize, preflight};
use interaction::Mode;

#[derive(Parser)]
#[command(
    name = "hydragen",
    version,
    color = clap::ColorChoice::Never,
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
#[command(after_help = "Examples:
  hydragen init
  hydragen init --name shared-agent-rules
  hydragen init --name shared-agent-rules --author \"Example team\" --description \"Shared coding-agent instructions.\"
  hydragen init --non-interactive --name shared-agent-rules

No metadata flags: guided prompts when stdin, stdout and stderr are terminals and CI is disabled.
Any metadata flag: flag-only input; no follow-up questions.
Name defaults to the current directory name when omitted. Author and description are optional.
--non-interactive always disables prompts.")]
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
    use std::path::Path;

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
