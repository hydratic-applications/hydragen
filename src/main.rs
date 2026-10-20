mod diagnostic_body;
mod diagnostics;
mod interaction;
mod presentation;
mod usage_errors;

use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
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
  Names use lowercase ASCII letters, digits, dots, underscores and hyphens, optionally as @scope/name (214 characters maximum). Unscoped names and scopes cannot start with a dot or underscore.
  Invalid names are not rewritten; supply --name to override an invalid directory default.
  Format defaults to yaml (layer.yaml). --format selects only the newly created layer manifest, not read-time priority.
  --non-interactive always disables prompts."#;

const ADD_HELP: &str = r#"Examples:
  hydragen add ../shared-config
  hydragen add ./rules ../shared-tooling
  hydragen --non-interactive add ../ui

Sources:
  Supply explicit local paths: ./, ../ or absolute paths. Local references have no version and are stored unchanged.
  Add searches upward for the nearest layer manifest, through the filesystem root without stopping at Git boundaries.
  Relative arguments resolve from that layer manifest's directory, NOT the invocation directory.
  Adding published references (name@version) is not supported yet. Bare names are not local paths.

Behaviour:
  Verifies the complete batch and existing direct dependencies before one update. Repeated sources are harmless.
  Only declares dependencies; does not fetch content, render files or change Git. Never prompts, in any environment."#;

#[derive(Parser)]
#[command(
    name = "hydragen",
    bin_name = "hydragen",
    version,
    color = clap::ColorChoice::Never,
    styles = clap::builder::Styles::plain(),
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
    /// Declare verified local layer dependencies without fetching or rendering
    Add(AddArgs),
}

#[derive(Args)]
#[command(next_line_help = true, after_help = ADD_HELP)]
struct AddArgs {
    /// Explicit local source paths, relative to the declaring layer manifest directory
    #[arg(value_name = "SOURCE", required = true, num_args = 1..)]
    sources: Vec<String>,
}

#[derive(Args)]
#[command(next_line_help = true, after_help = INIT_HELP)]
struct InitArgs {
    /// Optional name override: lowercase, optionally @scope/name (defaults to the current directory name)
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
    let presentation = presentation::Presentation::detect();
    presentation.configure();
    let cli = match parse_cli(std::env::args_os(), presentation) {
        Ok(cli) => cli,
        Err(output) => {
            // Every parsing outcome returns here, before even resolving current_dir.
            let exit_code = if output.stderr {
                output.write_to(&mut io::stderr().lock())
            } else {
                output.write_to(&mut io::stdout().lock())
            };
            return ExitCode::from(exit_code);
        }
    };
    match run(cli, presentation) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if !interaction::is_reported_cancellation(error.as_ref()) {
                let _ = writeln!(
                    io::stderr().lock(),
                    "{}",
                    presentation.diagnostic(
                        presentation::Severity::Error,
                        &diagnostics::error_message(error.as_ref())
                    )
                );
            }
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug)]
struct ParseOutput {
    text: String,
    stderr: bool,
    exit_code: u8,
    fail_on_write_error: bool,
}

impl ParseOutput {
    fn write_to(&self, output: &mut impl Write) -> u8 {
        if output.write_all(self.text.as_bytes()).is_err() && self.fail_on_write_error {
            1
        } else {
            self.exit_code
        }
    }
}

fn parse_cli<I, T>(
    arguments: I,
    presentation: presentation::Presentation,
) -> Result<Cli, ParseOutput>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    // Keep the built static definition: error adaptation needs its argument metadata.
    let mut command = Cli::command();
    command.build();
    command
        .try_get_matches_from_mut(arguments)
        .and_then(|mut matches| Cli::from_arg_matches_mut(&mut matches))
        .map_err(|error| {
            // Capture these before the renderer consumes the error. Help-on-missing-
            // input is not DisplayHelp: clap deliberately keeps it stderr/status 2.
            let stderr = error.use_stderr();
            let exit_code = u8::try_from(error.exit_code()).expect("clap exit code fits in u8");
            let fail_on_write_error = error.kind() == clap::error::ErrorKind::DisplayVersion;
            let text = match error.kind() {
                clap::error::ErrorKind::DisplayHelp => error.render().to_string(),
                clap::error::ErrorKind::DisplayVersion => {
                    format!("{}\n", env!("CARGO_PKG_VERSION"))
                }
                _ => usage_errors::render(error, &command, presentation),
            };
            ParseOutput {
                text,
                stderr,
                exit_code,
                fail_on_write_error,
            }
        })
}

fn run(cli: Cli, presentation: presentation::Presentation) -> Result<(), Box<dyn Error>> {
    // Keep command-specific control flow and carry typed facts to shared composition.
    if let Command::Add(arguments) = cli.command {
        let directory =
            std::env::current_dir().map_err(diagnostics::AddCommandError::CurrentDirectory)?;
        return run_add(
            &directory,
            &arguments.sources,
            presentation,
            &mut io::stdout().lock(),
        )
        .map_err(Into::into);
    }
    let directory = std::env::current_dir()
        .map_err(|source| diagnostics::CliIoError::CurrentDirectory { source }.into_io())?;
    match cli.command {
        Command::Init(arguments) => {
            let input_flags = arguments.name.is_some()
                || arguments.author.is_some()
                || arguments.description.is_some()
                || arguments.format.is_some();
            let mode = Mode::detect(input_flags, cli.non_interactive);
            let inspection = inspect_init(&directory)?;
            diagnostics::warn_duplicates(inspection.discovery(), presentation);
            let prepared = inspection.prepare()?;
            hydragen::layer_manifest::schema::ensure_ready()?;
            // Keep the theme through selection, writing, and the guided outro;
            // dropping it resets cliclack on every success or error path.
            let session =
                (mode == Mode::Guided).then(|| interaction::GuidedSession::install(presentation));
            let interaction::InitInput { metadata, format } = if mode == Mode::Guided {
                interaction::collect_init_input(
                    &interaction::directory_name(&directory)?,
                    session.as_ref().expect("guided mode installs its theme"),
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
                    metadata: flag_metadata(arguments, &directory)?,
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
        Command::Add(_) => unreachable!("add dispatched before init"),
    }
    Ok(())
}

fn run_add(
    directory: &std::path::Path,
    sources: &[String],
    presentation: presentation::Presentation,
    output: &mut impl Write,
) -> Result<(), diagnostics::AddCommandError> {
    use diagnostics::{AddCommandError, AddCompletionState};
    let plan = hydragen::dependencies::prepare_add(directory, sources).map_err(|failure| {
        for discovery in &failure.discoveries {
            diagnostics::warn_duplicates(discovery, presentation);
        }
        AddCommandError::Verification {
            failure,
            invocation: directory.to_owned(),
        }
    })?;
    for discovery in plan.discoveries() {
        diagnostics::warn_duplicates(discovery, presentation);
    }
    let state = if plan.is_noop() {
        AddCompletionState::Unchanged
    } else {
        let candidate = hydragen::layer_manifest::edit::append_dependencies(
            plan.current(),
            &plan.new_references().collect::<Vec<_>>(),
        )
        .map_err(AddCommandError::Edit)?;
        hydragen::replacement::replace_layer_manifest(&plan.current().path, candidate.as_bytes())
            .map_err(AddCommandError::Replacement)?;
        AddCompletionState::Updated
    };
    // Replacement has already committed before any completion bytes are sent.
    // Flush too: buffered writers can fail after accepting the whole message.
    let message = presentation.add_success(&plan);
    output
        .write_all(message.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|source| AddCommandError::Completion {
            state,
            layer: plan.current().metadata.name.clone(),
            path: plan.current().path.clone(),
            source,
        })
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

    fn parse_output(arguments: &[&str], decorated: bool) -> ParseOutput {
        match parse_cli(
            arguments,
            presentation::Presentation {
                decorated,
                basic: false,
            },
        ) {
            Err(output) => output,
            Ok(_) => panic!("expected a parser outcome, not operational input"),
        }
    }

    #[test]
    fn root_missing_input_and_explicit_help_keep_distinct_streams_and_statuses() {
        for decorated in [false, true] {
            let root = parse_output(&["hydragen"], decorated);
            assert_eq!(root.exit_code, 2);
            assert!(root.stderr);
            assert!(
                root.text.contains("Error: A command is required."),
                "{}",
                root.text
            );
            for detail in ["Usage: hydragen", "Commands:", "init", "add", "--help"] {
                assert!(root.text.contains(detail), "{detail}: {}", root.text);
            }
            let global_only = parse_output(&["hydragen", "--non-interactive"], decorated);
            assert_eq!(global_only.exit_code, 2);
            assert!(global_only.stderr);
            assert!(global_only.text.contains("requires a subcommand"));
            assert!(global_only.text.contains("Usage: hydragen"));
            assert!(!global_only.text.contains("Could not parse"));

            for arguments in [
                vec!["hydragen", "--help"],
                vec!["hydragen", "-h"],
                vec!["hydragen", "init", "--help"],
                vec!["hydragen", "init", "-h"],
                vec!["hydragen", "add", "--help"],
                vec!["hydragen", "add", "-h"],
                vec!["hydragen", "help", "init"],
                vec!["hydragen", "help", "add"],
                vec!["hydragen", "--version"],
                vec!["hydragen", "-V"],
            ] {
                let output = parse_output(&arguments, decorated);
                assert_eq!(output.exit_code, 0, "{arguments:?}");
                assert!(!output.stderr, "{arguments:?}");
                assert!(!output.text.contains("Error:"));
                assert!(!output.text.contains('\x1b'));
                if arguments
                    .iter()
                    .any(|argument| ["--version", "-V"].contains(argument))
                {
                    assert_eq!(output.text, format!("{}\n", env!("CARGO_PKG_VERSION")));
                } else {
                    assert!(output.text.contains("hydragen"));
                }
                assert!(output.text.ends_with('\n'));
            }
        }
    }

    #[test]
    fn command_context_never_uses_untrusted_argv_zero() {
        for tail in [
            vec![],
            vec!["--help"],
            vec!["--version"],
            vec!["--non-interactive"],
            vec!["init", "--nme"],
            vec!["add"],
            vec!["add", "--unknown"],
        ] {
            let mut arguments = vec!["untrusted\x1b[2J\nname"];
            arguments.extend(tail.iter().copied());
            let output = parse_output(&arguments, false);
            assert!(!output.text.contains("untrusted"), "{}", output.text);
            assert!(!output.text.contains('\x1b'));
            if tail == ["--version"] {
                assert_eq!(output.text, format!("{}\n", env!("CARGO_PKG_VERSION")));
            } else {
                assert!(output.text.contains("hydragen"));
            }
        }
    }

    #[test]
    fn successful_parse_does_not_rewrite_argument_values() {
        let raw = "./配置 🦀\\literal\\n\x1b[2J\npath";
        let cli = parse_cli(
            ["hydragen", "add", raw],
            presentation::Presentation {
                decorated: false,
                basic: false,
            },
        )
        .unwrap_or_else(|error| panic!("{}", error.text));
        assert!(matches!(cli.command, Command::Add(AddArgs { sources }) if sources == [raw]));
    }

    #[cfg(unix)]
    #[test]
    fn invalid_utf8_is_a_parser_outcome_not_an_args_unicode_panic() {
        use std::os::unix::ffi::OsStringExt;
        let arguments = [
            std::ffi::OsString::from("hydragen"),
            std::ffi::OsString::from("add"),
            std::ffi::OsString::from_vec(vec![b'.', b'/', 0xff]),
        ];
        let output = match parse_cli(
            arguments,
            presentation::Presentation {
                decorated: false,
                basic: false,
            },
        ) {
            Err(output) => output,
            Ok(_) => panic!("String-valued sources must reject invalid UTF-8"),
        };
        assert!(output.stderr);
        assert_eq!(output.exit_code, 2);
        assert!(
            output.text.starts_with("Error: Invalid UTF-8"),
            "{}",
            output.text
        );
        assert!(output.text.contains("Usage: hydragen add"));
    }

    struct FailingOutput {
        remaining: usize,
        fail_flush: bool,
    }

    #[test]
    fn version_output_write_failure_returns_failure_without_changing_other_parse_statuses() {
        for (arguments, expected) in [
            (vec!["hydragen", "--version"], 1),
            (vec!["hydragen", "-V"], 1),
            (vec!["hydragen", "--help"], 0),
            (vec!["hydragen", "add"], 2),
        ] {
            let output = parse_output(&arguments, false);
            assert_eq!(
                output.write_to(&mut FailingOutput {
                    remaining: 0,
                    fail_flush: false
                }),
                expected,
                "{arguments:?}"
            );
            assert_eq!(output.write_to(&mut Vec::new()), output.exit_code);
        }
    }

    impl Write for FailingOutput {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            let count = bytes.len().min(self.remaining);
            self.remaining -= count;
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            if self.fail_flush {
                Err(io::ErrorKind::BrokenPipe.into())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn add_completion_failures_retain_committed_or_noop_state() {
        for noop in [false, true] {
            for (remaining, fail_flush) in [(0, false), (12, false), (usize::MAX, true)] {
                let root = tempfile::tempdir().unwrap();
                let app = root.path().join("app");
                let dependency = root.path().join("rules");
                std::fs::create_dir(&app).unwrap();
                std::fs::create_dir(&dependency).unwrap();
                std::fs::write(dependency.join("layer.yaml"), "name: rules\n").unwrap();
                let path = app.join("layer.yaml");
                // A shared sequence alias cannot be edited safely, so a successful
                // no-op here also proves that the editor is entirely bypassed.
                let original = if noop {
                    "name: app\ndependencies: &deps [../rules]\ncustom: *deps\n"
                } else {
                    "name: app\n"
                };
                std::fs::write(&path, original).unwrap();
                let failure = run_add(
                    &app,
                    &["../rules".into()],
                    presentation::Presentation {
                        decorated: false,
                        basic: false,
                    },
                    &mut FailingOutput {
                        remaining,
                        fail_flush,
                    },
                )
                .unwrap_err();
                let diagnostics::AddCommandError::Completion { state, source, .. } = &failure
                else {
                    panic!("unexpected error: {failure}");
                };
                assert_eq!(source.kind(), io::ErrorKind::BrokenPipe);
                let message = failure.to_string();
                if noop {
                    assert_eq!(*state, diagnostics::AddCompletionState::Unchanged);
                    assert!(message.contains("was not updated"));
                    assert!(!message.contains("was already updated"));
                    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
                } else {
                    assert_eq!(*state, diagnostics::AddCompletionState::Updated);
                    assert!(message.contains("was already updated"));
                    assert!(message.contains(&path.display().to_string()));
                    assert!(!message.contains("No files were changed"));
                    let loaded = hydragen::discover(&app).unwrap().load().unwrap();
                    assert_eq!(loaded.document["dependencies"][0], "../rules");
                }
                assert_eq!(std::fs::read_dir(&app).unwrap().count(), 1);
            }
        }
    }

    #[test]
    fn add_verification_carries_invocation_context_without_changing_the_library_failure() {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        let nested = app.join("nested");
        let target = root.path().join("layer-b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(app.join("layer.yaml"), "name: app\n").unwrap();
        let mut output = Vec::new();
        let error = run_add(
            &nested,
            &["../layer-b".into()],
            presentation::Presentation {
                decorated: false,
                basic: false,
            },
            &mut output,
        )
        .unwrap_err();
        let diagnostics::AddCommandError::Verification {
            failure,
            invocation,
        } = &error
        else {
            panic!("unexpected error: {error}");
        };
        assert_eq!(invocation, &nested);
        assert_eq!(
            failure.directory.as_deref(),
            Some(app.canonicalize().unwrap().as_path())
        );
        assert!(
            error
                .source()
                .unwrap()
                .downcast_ref::<hydragen::dependencies::AddFailure>()
                .is_some()
        );
        let text = error.to_string();
        assert!(text.starts_with("No layer manifest found in '../layer-b'."));
        assert!(text.contains(&format!(
            "Paths are relative to '{}'.",
            hydragen::diagnostic_text::display(failure.directory.as_ref().unwrap().display())
        )));
        assert!(output.is_empty());
        assert_eq!(
            std::fs::read_to_string(app.join("layer.yaml")).unwrap(),
            "name: app\n"
        );
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
    }

    #[test]
    fn add_help_wraps_and_usage_requires_sources_without_path_flag() {
        for width in [40, 80, 100] {
            let mut command = Cli::command();
            command.build();
            let help = command
                .find_subcommand("add")
                .unwrap()
                .clone()
                .term_width(width)
                .render_long_help()
                .to_string();
            for line in help.lines() {
                // Clap keeps generated usage on one line; prose still wraps.
                assert!(
                    line.starts_with("Usage:") || line.len() <= width,
                    "width {width}: {line:?}"
                );
            }
            let prose = help.split_whitespace().collect::<Vec<_>>().join(" ");
            for phrase in [
                "./, ../ or absolute paths",
                "NOT the invocation directory",
                "without stopping at Git boundaries",
                "no version",
                "stored unchanged",
                "Never prompts",
                "not supported yet",
            ] {
                assert!(prose.contains(phrase), "{phrase}: {help}");
            }
            assert!(!help.contains("--path"));
            assert!(!help.contains('\x1b'));
        }
        assert!(Cli::try_parse_from(["hydragen", "add"]).is_err());
        for arguments in [
            vec!["hydragen", "--non-interactive", "add", "../rules"],
            vec!["hydragen", "add", "../rules", "--non-interactive"],
        ] {
            let cli = Cli::try_parse_from(arguments).unwrap();
            assert!(cli.non_interactive);
            assert!(
                matches!(cli.command, Command::Add(AddArgs { sources }) if sources == ["../rules"])
            );
        }
    }

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
        assert_eq!(
            flag_metadata(
                arguments("@example/_config"),
                Path::new("Invalid Directory")
            )
            .unwrap()
            .name,
            "@example/_config"
        );
        for name in ["Shared-Config", " shared-config ", "配置", "_config"] {
            let error = flag_metadata(arguments(name), Path::new("valid-directory")).unwrap_err();
            assert!(error.to_string().contains("--name"));
        }
    }
}
