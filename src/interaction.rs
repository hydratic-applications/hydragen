use std::ffi::OsStr;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use hydragen::{InitError, ManifestFormat, Metadata, validate_metadata_value};

struct DefaultTheme;
impl cliclack::Theme for DefaultTheme {}

struct InitTheme;
impl cliclack::Theme for InitTheme {
    fn format_placeholder(
        &self,
        state: &cliclack::ThemeState,
        cursor: &cliclack::StringCursor,
    ) -> String {
        if matches!(state, cliclack::ThemeState::Submit) && cursor.is_empty() {
            let mut summary = cliclack::StringCursor::default();
            let skipped = console::Style::new()
                .for_stderr()
                .italic()
                .apply_to("Skipped")
                .to_string();
            summary.extend(&skipped);
            DefaultTheme.format_placeholder(state, &summary)
        } else {
            DefaultTheme.format_placeholder(state, cursor)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Guided,
    FlagOnly,
}

impl Mode {
    pub fn detect(input_flags: bool, non_interactive: bool) -> Self {
        Self::select(
            input_flags,
            non_interactive,
            [
                io::stdin().is_terminal(),
                io::stdout().is_terminal(),
                io::stderr().is_terminal(),
            ],
            std::env::var_os("CI").as_deref(),
        )
    }

    fn select(
        input_flags: bool,
        non_interactive: bool,
        terminals: [bool; 3],
        ci: Option<&OsStr>,
    ) -> Self {
        let ci_enabled = ci.is_some_and(|value| {
            let value = value.to_string_lossy();
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        });
        if input_flags || non_interactive || !terminals.into_iter().all(|value| value) || ci_enabled
        {
            Self::FlagOnly
        } else {
            Self::Guided
        }
    }
}

fn validate_answer(field: &'static str, value: &str, optional: bool) -> Result<(), InitError> {
    if optional && value.is_empty() {
        Ok(())
    } else {
        validate_metadata_value(field, value)
    }
}

fn optional_input(field: &'static str, label: &str) -> io::Result<Option<String>> {
    let hint = console::Style::new()
        .for_stderr()
        .dim()
        .apply_to("(optional)");
    let value: String = cliclack::input(format!("{label} {hint}"))
        .required(false)
        .validate(move |value: &String| validate_answer(field, value, true))
        .interact()?;
    Ok(if value.is_empty() { None } else { Some(value) })
}

pub fn directory_name(directory: &Path) -> io::Result<String> {
    directory
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|name| validate_metadata_value("name", name).is_ok())
        .map(str::to_owned)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Could not derive a non-blank UTF-8 layer name from directory '{}'. Supply --name explicitly.",
                    directory.display()
                ),
            )
        })
}

pub struct InitInput {
    pub metadata: Metadata,
    pub format: ManifestFormat,
}

pub fn collect_init_input(default_name: &str) -> io::Result<InitInput> {
    // Only guided init uses this rendering adjustment. Submitted empty inputs
    // are optional skips; name always resolves to a validated non-empty value.
    cliclack::set_theme(InitTheme);
    let result = (|| {
        cliclack::intro("Initialise a layer")?;
        cliclack::log::info(
            console::Style::new()
                .for_stderr()
                .dim()
                .apply_to("Enter accepts the default or skips an optional field."),
        )?;
        let name: String = cliclack::input("Name")
            .default_input(default_name)
            .validate(|value: &String| validate_answer("name", value, false))
            .interact()?;
        let author = optional_input("author", "Author")?;
        let description = optional_input("description", "Description")?;
        // The empty-input summary is only for optional metadata, not selections.
        cliclack::reset_theme();
        let mut selector = cliclack::select("Manifest format");
        for format in ManifestFormat::ALL {
            selector = selector.item(format, format.filename(), "");
        }
        let format = selector
            .initial_value(ManifestFormat::default())
            .interact()?;
        Ok(InitInput {
            metadata: Metadata {
                name,
                author,
                description,
            },
            format,
        })
    })();
    cliclack::reset_theme();
    result.map_err(|error: io::Error| {
        // Cliclack renders the cancelled prompt; the CLI reports the reason once.
        io::Error::new(error.kind(), input_failure_message(&error))
    })
}

fn input_failure_message(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::Interrupted {
        "Initialisation cancelled. No files were changed.".to_owned()
    } else {
        format!("Could not collect initialisation input: {error}. No files were changed.")
    }
}

pub fn finish(mode: Mode, manifest: &Path) -> io::Result<()> {
    if mode == Mode::Guided {
        cliclack::outro("Layer initialised.").map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Manifest '{}' was created, but terminal feedback failed: {error}",
                    manifest.display()
                ),
            )
        })?;
    }
    writeln!(
        io::stdout().lock(),
        "Initialised layer: {}",
        manifest.display()
    )
    .map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "Manifest '{}' was created, but reporting its path failed: {error}",
                manifest.display()
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cliclack::{StringCursor, Theme, ThemeState};

    #[test]
    fn submitted_empty_inputs_show_skipped_without_changing_the_value() {
        let cursor = StringCursor::default();
        let rendered = InitTheme.format_placeholder(&ThemeState::Submit, &cursor);
        assert!(console::strip_ansi_codes(&rendered).contains("Skipped"));
        assert!(cursor.is_empty());
    }

    #[test]
    fn unfinished_or_cancelled_inputs_do_not_show_skipped() {
        for state in [
            ThemeState::Active,
            ThemeState::Cancel,
            ThemeState::Error("Invalid".into()),
        ] {
            let cursor = StringCursor::default();
            assert_eq!(
                InitTheme.format_placeholder(&state, &cursor),
                DefaultTheme.format_placeholder(&state, &cursor)
            );
        }
    }

    #[test]
    fn non_empty_defaults_keep_cliclacks_original_rendering() {
        let mut cursor = StringCursor::default();
        cursor.extend("my-layer (default)");
        for state in [ThemeState::Active, ThemeState::Submit] {
            assert_eq!(
                InitTheme.format_placeholder(&state, &cursor),
                DefaultTheme.format_placeholder(&state, &cursor)
            );
        }
    }

    #[test]
    fn default_name_uses_the_last_component_without_normalising_it() {
        assert_eq!(
            directory_name(Path::new("parent/ My Project 配置 🦀 ")).unwrap(),
            " My Project 配置 🦀 "
        );
    }

    #[test]
    fn unusable_directory_names_require_an_explicit_name() {
        for directory in [Path::new("/"), Path::new(""), Path::new("   ")] {
            let error = directory_name(directory).unwrap_err();
            assert!(error.to_string().contains("Supply --name explicitly"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_directory_names_are_not_lossily_converted() {
        use std::os::unix::ffi::OsStrExt;
        assert!(directory_name(Path::new(OsStr::from_bytes(b"parent/\xff"))).is_err());
    }

    #[test]
    fn mode_selection_covers_flags_terminals_overrides_and_ci() {
        for metadata_flags in [false, true] {
            for non_interactive in [false, true] {
                for mask in 0..8 {
                    let terminals = [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0];
                    for (ci, enabled) in [
                        (None, false),
                        (Some(""), false),
                        (Some("0"), false),
                        (Some("false"), false),
                        (Some("FaLsE"), false),
                        (Some("1"), true),
                        (Some("TRUE"), true),
                        (Some("other"), true),
                    ] {
                        let expected =
                            if !metadata_flags && !non_interactive && mask == 7 && !enabled {
                                Mode::Guided
                            } else {
                                Mode::FlagOnly
                            };
                        assert_eq!(
                            Mode::select(
                                metadata_flags,
                                non_interactive,
                                terminals,
                                ci.map(OsStr::new)
                            ),
                            expected
                        );
                    }
                }
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_ci_values_disable_interaction() {
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(
            Mode::select(false, false, [true; 3], Some(OsStr::from_bytes(b"\xff"))),
            Mode::FlagOnly
        );
    }

    #[test]
    fn only_empty_optional_answers_are_skipped() {
        assert!(validate_answer("name", "", false).is_err());
        assert!(validate_answer("author", "", true).is_ok());
        for value in [" ", "\t", "\u{2003}"] {
            assert!(validate_answer("name", value, false).is_err());
            assert!(validate_answer("author", value, true).is_err());
        }
        for value in [" name ", "quoted \"value\"", "配置 🦀"] {
            assert!(validate_answer("name", value, false).is_ok());
            assert!(validate_answer("description", value, true).is_ok());
        }
    }

    #[test]
    fn cancellation_eof_and_io_errors_are_distinguished() {
        let cancelled = input_failure_message(&io::ErrorKind::Interrupted.into());
        assert!(cancelled.contains("cancelled"));
        for kind in [io::ErrorKind::UnexpectedEof, io::ErrorKind::BrokenPipe] {
            let message = input_failure_message(&kind.into());
            assert!(message.contains("Could not collect initialisation input"));
            assert!(message.contains("No files were changed"));
        }
    }
}
