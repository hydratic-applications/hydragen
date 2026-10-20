use std::cell::RefCell;
use std::ffi::OsStr;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use hydragen::diagnostic_text::display;
use hydragen::layer_manifest::schema;
use hydragen::{
    Author, AuthorObject, InitError, LayerManifestFormat, Metadata, validate_metadata_value,
};

use crate::author_discovery::{AuthorProvenanceKind, Choice, ContactChoice, Preparation};
use crate::diagnostics::{CliIoError, CompletionChannel};
use crate::presentation::{self, Presentation};

#[cfg(all(test, unix))]
#[path = "interaction/author_terminal.rs"]
mod author_tests;
mod author_wait;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuthorAction {
    Candidate(usize),
    EnterAnother,
    Skip,
}

struct AuthorSelectorTheme {
    // Theme hooks receive labels/hints, not selector values. Keep their exact
    // display identities mapped to typed actions; never interpret a name as an action.
    rows: Vec<(String, String, AuthorAction)>,
    kinds: Vec<AuthorProvenanceKind>,
    selected: AuthorAction,
}

impl AuthorSelectorTheme {
    fn kind(&self, action: AuthorAction) -> AuthorProvenanceKind {
        match action {
            AuthorAction::Candidate(index) => self.kinds[index],
            _ => AuthorProvenanceKind::Unspecified,
        }
    }
}

// The full glyph set fixes the slot width, independent of viewport or choices.
fn author_icon_slot_width() -> usize {
    ["👤", "🏢", "◈", "◇", "✏️", "⏭"]
        .into_iter()
        .map(console::measure_text_width)
        .fold(2, usize::max)
}

// Display-only prefix: glyph, right padding to the fixed slot, then two spaces.
// Basic terminals keep that same slot; NO_COLOR does not select ASCII icons.
fn author_icon_prefix(action: AuthorAction, kind: AuthorProvenanceKind, basic: bool) -> String {
    let glyph = match (action, kind, basic) {
        (AuthorAction::EnterAnother, _, false) => "✏️",
        (AuthorAction::EnterAnother, _, true) => "+",
        (AuthorAction::Skip, _, false) => "⏭",
        (AuthorAction::Skip, _, true) => ">",
        (_, AuthorProvenanceKind::Identity, false) => "👤",
        (_, AuthorProvenanceKind::Identity, true) => "U",
        (_, AuthorProvenanceKind::Organisation, false) => "🏢",
        (_, AuthorProvenanceKind::Organisation, true) => "O",
        (_, AuthorProvenanceKind::Mixed, false) => "◈",
        (_, AuthorProvenanceKind::Mixed, true) => "*",
        (_, AuthorProvenanceKind::Unspecified, false) => "◇",
        (_, AuthorProvenanceKind::Unspecified, true) => "?",
    };
    format!(
        "{glyph}{}  ",
        " ".repeat(author_icon_slot_width().saturating_sub(console::measure_text_width(glyph)))
    )
}

type AuthorThemeState = Arc<Mutex<Option<AuthorSelectorTheme>>>;

struct AuthorThemeScope<'a>(&'a AuthorThemeState);

impl Drop for AuthorThemeScope<'_> {
    fn drop(&mut self) {
        *self.0.lock().unwrap() = None;
    }
}

struct InitTheme {
    presentation: Presentation,
    cancelled: Arc<AtomicBool>,
    author: AuthorThemeState,
}

impl cliclack::Theme for InitTheme {
    fn bar_color(&self, state: &cliclack::ThemeState) -> console::Style {
        let style = self.presentation.style();
        match state {
            cliclack::ThemeState::Active => style.cyan(),
            cliclack::ThemeState::Cancel | cliclack::ThemeState::Error(_) => style.red(),
            cliclack::ThemeState::Submit => style.dim(),
        }
    }

    fn state_symbol_color(&self, state: &cliclack::ThemeState) -> console::Style {
        self.bar_color(state)
    }

    fn state_symbol(&self, state: &cliclack::ThemeState) -> String {
        let symbol = match state {
            cliclack::ThemeState::Active => "◆",
            cliclack::ThemeState::Cancel => "■",
            cliclack::ThemeState::Submit => "◇",
            cliclack::ThemeState::Error(_) => "✕",
        };
        self.state_symbol_color(state).apply_to(symbol).to_string()
    }

    fn format_header(&self, state: &cliclack::ThemeState, prompt: &str) -> String {
        if self.hide_author_transition(state) {
            return String::new();
        }
        let label_style = match state {
            cliclack::ThemeState::Active | cliclack::ThemeState::Error(_) => {
                self.presentation.style().bold()
            }
            _ => self.presentation.style(),
        };
        prompt
            .lines()
            .enumerate()
            .map(|(index, line)| {
                let marker = if index == 0 {
                    self.state_symbol(state)
                } else {
                    self.bar_color(state).apply_to("│").to_string()
                };
                format!("{marker}  {}\n", label_style.apply_to(line))
            })
            .collect()
    }

    fn format_intro(&self, title: &str) -> String {
        title.to_owned()
    }

    fn format_outro(&self, message: &str) -> String {
        format!(
            "{}  {message}\n",
            self.presentation.style().dim().apply_to("└")
        )
    }

    fn format_outro_cancel(&self, message: &str) -> String {
        format!(
            "{}\n",
            self.presentation
                .style()
                .red()
                .apply_to(format!("└  {message}"))
        )
    }

    fn format_footer_with_message(&self, state: &cliclack::ThemeState, message: &str) -> String {
        if self.hide_author_transition(state) {
            return String::new();
        }
        let footer = match state {
            cliclack::ThemeState::Active => format!("└  {message}"),
            cliclack::ThemeState::Submit => "│".to_owned(),
            cliclack::ThemeState::Error(error) => format!("└  {error}"),
            cliclack::ThemeState::Cancel => {
                // This observes cancellation, not successful frame delivery. The
                // application reports its reason through an explicit fallible outro.
                self.cancelled.store(true, Ordering::Relaxed);
                return String::new();
            }
        };
        format!("{}\n", self.bar_color(state).apply_to(footer))
    }

    fn radio_symbol(&self, state: &cliclack::ThemeState, selected: bool) -> String {
        match state {
            cliclack::ThemeState::Active if selected => {
                self.presentation.style().cyan().apply_to("●").to_string()
            }
            cliclack::ThemeState::Active => {
                self.presentation.style().dim().apply_to("○").to_string()
            }
            _ => String::new(),
        }
    }

    fn radio_item(
        &self,
        state: &cliclack::ThemeState,
        selected: bool,
        label: &str,
        hint: &str,
    ) -> String {
        if matches!(
            state,
            cliclack::ThemeState::Cancel | cliclack::ThemeState::Submit
        ) && !selected
        {
            return String::new();
        }
        let author_row = {
            let mut context = self.author.lock().unwrap();
            context.as_mut().and_then(|context| {
                let action = context
                    .rows
                    .iter()
                    .find_map(|(row_label, row_hint, action)| {
                        (label == row_label && hint == row_hint).then_some(*action)
                    });
                if selected && let Some(action) = action {
                    context.selected = action;
                }
                action.map(|action| (action, context.kind(action)))
            })
        };
        let author_action = author_row.map(|(action, _)| action);
        if matches!(state, cliclack::ThemeState::Submit) {
            match author_action {
                Some(AuthorAction::Skip) => {
                    return self
                        .presentation
                        .style()
                        .dim()
                        .italic()
                        .apply_to("Skipped")
                        .to_string();
                }
                Some(AuthorAction::EnterAnother) => return String::new(),
                _ => {}
            }
        }
        let style = match state {
            cliclack::ThemeState::Active if selected => self.presentation.style().cyan(),
            cliclack::ThemeState::Submit | cliclack::ThemeState::Cancel => {
                self.presentation.style().dim()
            }
            _ => self.presentation.style(),
        };
        let symbol = self.radio_symbol(state, selected);
        let gap = if symbol.is_empty() { "" } else { " " };
        let icon = if matches!(state, cliclack::ThemeState::Active) {
            author_row.map_or_else(String::new, |(action, kind)| {
                author_icon_prefix(action, kind, self.presentation.basic)
            })
        } else {
            String::new()
        };
        // Author sources appear inline only beside the focused active candidate,
        // never beside actions, unfocused choices, or completed answers.
        let show_hint = author_action.is_none()
            || (matches!(author_action, Some(AuthorAction::Candidate(_)))
                && selected
                && matches!(state, cliclack::ThemeState::Active));
        let hint = if hint.is_empty() || !show_hint {
            String::new()
        } else {
            self.presentation
                .style()
                .dim()
                .apply_to(format!(" ({hint})"))
                .to_string()
        };
        format!(
            "{symbol}{gap}{}{hint}",
            style.apply_to(format!("{icon}{label}"))
        )
    }

    fn format_select_item(
        &self,
        state: &cliclack::ThemeState,
        selected: bool,
        label: &str,
        hint: &str,
    ) -> String {
        if (matches!(
            state,
            cliclack::ThemeState::Cancel | cliclack::ThemeState::Submit
        ) && !selected)
            || self.hide_author_transition(state)
        {
            return String::new();
        }
        // Leave natural terminal wrapping and redraw to cliclack, as for other
        // selectors; do not insert a separate source-detail line.
        format!(
            "{}  {}\n",
            self.bar_color(state).apply_to("│"),
            self.radio_item(state, selected, label, hint)
        )
    }

    fn format_placeholder(
        &self,
        state: &cliclack::ThemeState,
        cursor: &cliclack::StringCursor,
    ) -> String {
        if matches!(state, cliclack::ThemeState::Submit) && cursor.is_empty() {
            let skipped = self
                .presentation
                .style()
                .dim()
                .italic()
                .apply_to("Skipped")
                .to_string();
            format!("{}  {skipped}\n", self.bar_color(state).apply_to("│"))
        } else {
            let style = self.placeholder_style(state);
            let placeholder = match state {
                cliclack::ThemeState::Active | cliclack::ThemeState::Error(_) => {
                    self.cursor_with_style(cursor, &style)
                }
                cliclack::ThemeState::Cancel => String::new(),
                cliclack::ThemeState::Submit => cursor.to_string(),
            };
            placeholder
                .lines()
                .map(|line| {
                    format!(
                        "{}  {}\n",
                        self.bar_color(state).apply_to("│"),
                        style.apply_to(line)
                    )
                })
                .collect()
        }
    }
}

impl InitTheme {
    fn hide_author_transition(&self, state: &cliclack::ThemeState) -> bool {
        matches!(state, cliclack::ThemeState::Submit)
            && self
                .author
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|context| context.selected == AuthorAction::EnterAnother)
    }
}

pub struct GuidedSession {
    presentation: Presentation,
    cancelled: Arc<AtomicBool>,
    author: AuthorThemeState,
}

impl GuidedSession {
    pub fn install(presentation: Presentation) -> Self {
        let cancelled = Arc::new(AtomicBool::new(false));
        let author = Arc::new(Mutex::new(None));
        cliclack::set_theme(InitTheme {
            presentation,
            cancelled: cancelled.clone(),
            author: author.clone(),
        });
        Self {
            presentation,
            cancelled,
            author,
        }
    }
}

impl Drop for GuidedSession {
    fn drop(&mut self) {
        cliclack::reset_theme();
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

    pub(crate) fn select(
        input_flags: bool,
        non_interactive: bool,
        terminals: [bool; 3],
        ci: Option<&OsStr>,
    ) -> Self {
        let ci_enabled = presentation::ci_enabled(ci);
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
    } else if field == "author" {
        hydragen::author::interpret(value)
            .map(|_| ())
            .map_err(InitError::from)
    } else if field.starts_with("author.") {
        schema::validate_field(field, value).map_err(InitError::from)
    } else {
        validate_metadata_value(field, value)
    }
}

fn answer_error_message(error: InitError, optional: bool) -> String {
    match error {
        InitError::InvalidName { guidance } => format!("Name is invalid. {}", display(guidance)),
        InitError::InvalidMetadata { field } if optional => {
            format!(
                "Enter a non-blank {}, or clear the field and press Enter to skip.",
                display(field)
            )
        }
        InitError::InvalidMetadata { field } => format!("Enter a non-blank {}.", display(field)),
        other => other.to_string(),
    }
}

fn optional_input(field: &'static str, label: &str) -> io::Result<Option<String>> {
    let hint = console::Style::new()
        .for_stderr()
        .dim()
        .apply_to("(optional)");
    let value = metadata_input(field, &format!("{label} {hint}"), None, true)?;
    Ok(if value.is_empty() { None } else { Some(value) })
}

// Cliclack's validation callback can only retry. Keep unexpected internal errors
// out of that channel and return them immediately after the submitted input.
#[derive(Clone, Default)]
struct PromptValidation(Rc<RefCell<Option<InitError>>>);

impl PromptValidation {
    fn check(&self, result: Result<(), InitError>, optional: bool) -> Result<(), String> {
        match result {
            Ok(()) => Ok(()),
            Err(error @ (InitError::InvalidMetadata { .. } | InitError::InvalidName { .. })) => {
                Err(answer_error_message(error, optional))
            }
            Err(InitError::Schema(schema::Error::Content(violation))) => {
                let field = match violation.instance_path.as_str() {
                    "/author/name" => "author",
                    "/author/email" => "author email",
                    "/author/url" => "author URL",
                    _ => return Err(violation.to_string()),
                };
                let correction = match &violation.kind {
                    schema::ViolationKind::NonBlank => format!("Enter a non-blank {field}"),
                    schema::ViolationKind::Format { format } if format == "email" => {
                        format!("Enter a valid {field} address, such as alex@example.com")
                    }
                    schema::ViolationKind::Format { format } if format == "uri" => {
                        format!("Enter a valid {field} with an explicit scheme, such as 'https:'")
                    }
                    schema::ViolationKind::Format { format } => {
                        format!("Enter a valid {field} in {} format", display(format))
                    }
                    _ => format!("Correct {field}: {violation}"),
                };
                Err(if optional {
                    format!("{correction}, or clear the field and press Enter to skip.")
                } else {
                    format!("{correction}.")
                })
            }
            Err(error) => {
                *self.0.borrow_mut() = Some(error);
                Ok(())
            }
        }
    }

    fn finish(&self, result: io::Result<String>) -> io::Result<String> {
        match self.0.borrow_mut().take() {
            Some(error) => Err(io::Error::other(error)),
            None => result,
        }
    }
}

fn metadata_input(
    field: &'static str,
    label: &str,
    default: Option<&str>,
    optional: bool,
) -> io::Result<String> {
    let validation = PromptValidation::default();
    let callback = validation.clone();
    let mut input = cliclack::input(label)
        .required(!optional)
        .validate(move |value: &String| {
            callback.check(validate_answer(field, value, optional), optional)
        });
    if let Some(default) = default {
        input = input.default_input(default);
    }
    validation.finish(input.interact())
}

pub fn directory_name(directory: &Path) -> io::Result<String> {
    directory_name_validating(directory, validate_metadata_value)
}

fn directory_name_validating(
    directory: &Path,
    validate: impl FnOnce(&'static str, &str) -> Result<(), InitError>,
) -> io::Result<String> {
    let unusable = || {
        CliIoError::DirectoryName {
            directory: directory.to_owned(),
            source: None,
        }
        .into_io()
    };
    let name = directory
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(unusable)?;
    match validate("name", name) {
        Ok(()) => Ok(name.to_owned()),
        Err(
            error @ (InitError::InvalidMetadata { .. }
            | InitError::InvalidName { .. }
            | InitError::Schema(schema::Error::Content(_))),
        ) => Err(CliIoError::DirectoryName {
            directory: directory.to_owned(),
            source: Some(error),
        }
        .into_io()),
        Err(error) => Err(io::Error::other(error)),
    }
}

pub struct InitInput {
    pub metadata: Metadata,
    pub format: LayerManifestFormat,
}

pub fn collect_init_input(
    default_name: &str,
    session: &GuidedSession,
    preparation: &Preparation,
) -> io::Result<InitInput> {
    collect_init_input_ready(schema::ensure_ready, || {
        collect_ready_input(default_name, session, preparation)
    })
}

fn collect_init_input_ready(
    ready: impl FnOnce() -> Result<(), schema::Error>,
    collect: impl FnOnce() -> io::Result<InitInput>,
) -> io::Result<InitInput> {
    ready().map_err(io::Error::other)?;
    collect()
}

fn collect_ready_input(
    default_name: &str,
    session: &GuidedSession,
    preparation: &Preparation,
) -> io::Result<InitInput> {
    let result = (|| {
        let width = console::Term::stderr()
            .size_checked()
            .map(|(_, width)| usize::from(width));
        cliclack::intro(
            session
                .presentation
                .init_header(width, env!("CARGO_PKG_VERSION")),
        )?;
        cliclack::log::info(console::Style::new().for_stderr().dim().apply_to(
            "Enter accepts the current selection or default; empty optional text skips.",
        ))?;
        let name = metadata_input("name", "Name", Some(default_name), false)?;
        let choices =
            author_wait::snapshot(preparation, &session.cancelled, session.presentation.basic)?;
        let author = collect_author(&choices, session)?;
        let description = optional_input("description", "Description")?;
        let mut selector = cliclack::select("Layer manifest format");
        for format in LayerManifestFormat::ALL {
            selector = selector.item(format, format.extension(), "");
        }
        let format = selector
            .initial_value(LayerManifestFormat::default())
            .interact()?;
        Ok(InitInput {
            metadata: Metadata {
                name,
                author: author.map(Author::Object),
                description,
            },
            format,
        })
    })();
    if result.is_err() {
        // Close before any cancellation/error feedback, including errors at
        // Name. Reporting must not keep optional lookups active in the meantime.
        let _ = preparation.close_and_snapshot();
    }
    finish_input(result, session.cancelled.load(Ordering::Relaxed), || {
        cliclack::outro_cancel("Layer initialisation cancelled. No files were changed.")
    })
}

fn author_initial(choices: &[Choice]) -> AuthorAction {
    choices
        .iter()
        .position(|choice| choice.eligible)
        .map_or(AuthorAction::EnterAnother, AuthorAction::Candidate)
}

fn collect_author(choices: &[Choice], session: &GuidedSession) -> io::Result<Option<AuthorObject>> {
    let (mut author, group) = match select_author(choices, session)? {
        SelectedAuthor::Omitted => return Ok(None),
        SelectedAuthor::Group(index) => {
            (AuthorObject::new(choices[index].name.clone()), Some(index))
        }
        SelectedAuthor::Custom(author) => (author, None),
    };
    if author.email.is_none() {
        author.email = collect_contact(
            "author.email",
            "Author email",
            "Enter another email",
            group.map_or(&[], |index| choices[index].emails.as_slice()),
            session,
        )?;
    }
    if author.url.is_none() {
        author.url = collect_contact(
            "author.url",
            "Author URL",
            "Enter another URL",
            group.map_or(&[], |index| choices[index].urls.as_slice()),
            session,
        )?;
    }
    Ok(Some(author))
}

enum SelectedAuthor {
    Omitted,
    // Snapshot identity survives selection; custom text must never regain it by name.
    Group(usize),
    Custom(AuthorObject),
}

fn custom_author() -> io::Result<SelectedAuthor> {
    optional_input("author", "Author")?.map_or(Ok(SelectedAuthor::Omitted), |value| {
        hydragen::author::interpret(&value)
            .map(SelectedAuthor::Custom)
            .map_err(io::Error::other)
    })
}

fn select_author(choices: &[Choice], session: &GuidedSession) -> io::Result<SelectedAuthor> {
    if choices.is_empty() {
        return custom_author();
    }
    let initial = author_initial(choices);
    let mut rows: Vec<_> = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            (
                choice.name.clone(),
                choice.hint.clone(),
                AuthorAction::Candidate(index),
            )
        })
        .collect();
    rows.push((
        "Enter another author".into(),
        String::new(),
        AuthorAction::EnterAnother,
    ));
    rows.push(("Skip".into(), String::new(), AuthorAction::Skip));
    let mut selector = cliclack::select("Author (optional)")
        .max_rows(7)
        .initial_value(initial);
    for (label, hint, action) in &rows {
        selector = selector.item(*action, label, hint);
    }
    let action = {
        *session.author.lock().unwrap() = Some(AuthorSelectorTheme {
            rows,
            kinds: choices.iter().map(|choice| choice.kind).collect(),
            selected: initial,
        });
        let _scope = AuthorThemeScope(&session.author);
        selector.interact()?
    };
    match action {
        AuthorAction::Candidate(index) => Ok(SelectedAuthor::Group(index)),
        AuthorAction::EnterAnother => custom_author(),
        AuthorAction::Skip => Ok(SelectedAuthor::Omitted),
    }
}

fn collect_contact(
    field: &'static str,
    label: &str,
    custom: &str,
    choices: &[ContactChoice],
    session: &GuidedSession,
) -> io::Result<Option<String>> {
    if choices.is_empty() {
        return optional_input(field, label);
    }
    let initial = AuthorAction::Candidate(0);
    let mut rows: Vec<_> = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            (
                choice.value.clone(),
                choice.hint.clone(),
                AuthorAction::Candidate(index),
            )
        })
        .collect();
    rows.push((custom.into(), String::new(), AuthorAction::EnterAnother));
    rows.push(("Skip".into(), String::new(), AuthorAction::Skip));
    let mut selector = cliclack::select(format!("{label} (optional)"))
        .max_rows(7)
        .initial_value(initial);
    for (label, hint, action) in &rows {
        selector = selector.item(*action, label, hint);
    }
    let action = {
        *session.author.lock().unwrap() = Some(AuthorSelectorTheme {
            rows,
            kinds: choices.iter().map(|choice| choice.kind).collect(),
            selected: initial,
        });
        let _scope = AuthorThemeScope(&session.author);
        selector.interact()?
    };
    match action {
        AuthorAction::Candidate(index) => Ok(Some(choices[index].value.clone())),
        AuthorAction::EnterAnother => optional_input(field, label),
        AuthorAction::Skip => Ok(None),
    }
}

#[derive(Debug)]
struct ReportedCancellation;

impl std::fmt::Display for ReportedCancellation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Layer initialisation cancelled. No files were changed.")
    }
}

impl std::error::Error for ReportedCancellation {}

pub fn is_reported_cancellation(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<io::Error>()
        .and_then(io::Error::get_ref)
        .is_some_and(|cause| cause.is::<ReportedCancellation>())
}

fn finish_input<T>(
    result: io::Result<T>,
    cancelled: bool,
    report: impl FnOnce() -> io::Result<()>,
) -> io::Result<T> {
    result.map_err(|error| {
        // Cliclack's public API cannot distinguish a bare Interrupted from a
        // terminal write of that same kind after rendering Cancel. Do not infer
        // cancellation from Interrupted alone or discard OS/custom error causes.
        if cancelled
            && error.kind() == io::ErrorKind::Interrupted
            && error.raw_os_error().is_none()
            && error.get_ref().is_none()
        {
            return match report() {
                Ok(()) => io::Error::new(io::ErrorKind::Interrupted, ReportedCancellation),
                Err(source) => CliIoError::CancellationFeedback { source }.into_io(),
            };
        }
        CliIoError::Input { source: error }.into_io()
    })
}

pub fn finish(mode: Mode, layer_manifest: &Path, presentation: Presentation) -> io::Result<()> {
    let filename = layer_manifest
        .file_name()
        .unwrap_or(layer_manifest.as_os_str())
        .to_string_lossy();
    let message = presentation.success(&filename);
    if mode == Mode::Guided {
        finish_guided(layer_manifest, &message, |message| cliclack::outro(message))
    } else {
        finish_output(layer_manifest, &message, |message| {
            writeln!(io::stdout().lock(), "{message}")
        })
    }
}

fn finish_guided(
    layer_manifest: &Path,
    message: &str,
    outro: impl FnOnce(&str) -> io::Result<()>,
) -> io::Result<()> {
    outro(message).map_err(|source| {
        CliIoError::InitCompletion {
            path: layer_manifest.to_owned(),
            channel: CompletionChannel::Terminal,
            source,
        }
        .into_io()
    })
}

fn finish_output(
    layer_manifest: &Path,
    message: &str,
    write: impl FnOnce(&str) -> io::Result<()>,
) -> io::Result<()> {
    write(message).map_err(|source| {
        CliIoError::InitCompletion {
            path: layer_manifest.to_owned(),
            channel: CompletionChannel::Output,
            source,
        }
        .into_io()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cliclack::{StringCursor, Theme, ThemeState};

    fn theme() -> InitTheme {
        InitTheme {
            presentation: Presentation {
                decorated: false,
                basic: false,
            },
            cancelled: Arc::new(AtomicBool::new(false)),
            author: Arc::new(Mutex::new(None)),
        }
    }

    #[test]
    fn failed_guided_completion_reports_the_created_layer_manifest_not_cancellation() {
        let layer_manifest = Path::new("/project/shared-rules/layer.jsonc");
        let message = "Layer initialised, layer manifest created at: layer.jsonc";
        for kind in [io::ErrorKind::Interrupted, io::ErrorKind::BrokenPipe] {
            let error = finish_guided(layer_manifest, message, |delivered| {
                assert_eq!(delivered, message);
                Err(io::Error::new(kind, "outro delivery failed"))
            })
            .unwrap_err();
            assert_eq!(error.kind(), kind);
            let text = crate::diagnostics::error_message(&error);
            assert!(text.starts_with("Could not deliver initialisation terminal feedback."));
            assert!(text.contains("Cause: outro delivery failed"));
            assert!(text.contains(
                "Layer manifest '/project/shared-rules/layer.jsonc' was already created"
            ));
            assert_eq!(text, error.to_string());
            assert!(!error.to_string().contains("No files were changed"));
            assert!(!is_reported_cancellation(&error));
        }
    }

    #[test]
    fn guided_completion_failure_escapes_its_path_and_io_detail_only() {
        let raw = "bad\u{1b}[H\r\n\t";
        let escaped = r"bad\x1b[H\r\n\t";
        let path = Path::new(raw).join("layer.json");
        let error = finish_guided(&path, "Completed", |_| {
            Err(io::Error::other(raw.to_owned()))
        })
        .unwrap_err();
        let message = error.to_string();
        assert!(
            !message.chars().any(|ch| ch.is_control() && ch != '\n'),
            "{message:?}"
        );
        assert_eq!(message.matches(escaped).count(), 2, "{message:?}");
        assert!(message.contains("was already created"));
        assert!(message.contains("Could not deliver initialisation terminal feedback"));
        assert!(!message.contains("No files were changed"));
        assert!(!is_reported_cancellation(&error));
    }

    #[test]
    fn flag_completion_failure_retains_created_state_without_adding_a_flush_or_retry() {
        let path = Path::new("/project/layer.yaml");
        let mut writes = 0;
        let error = finish_output(path, "Completed", |message| {
            writes += 1;
            assert_eq!(message, "Completed");
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        })
        .unwrap_err();
        assert_eq!(writes, 1);
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        let context = error
            .get_ref()
            .unwrap()
            .downcast_ref::<CliIoError>()
            .unwrap();
        let CliIoError::InitCompletion {
            path: affected,
            channel,
            source,
        } = context
        else {
            panic!("expected typed completion");
        };
        assert_eq!(affected, path);
        assert_eq!(*channel, CompletionChannel::Output);
        assert_eq!(source.kind(), io::ErrorKind::BrokenPipe);
        assert_eq!(source.raw_os_error(), None);
        let text = crate::diagnostics::error_message(&error);
        assert!(text.starts_with("Could not report initialisation completion."));
        assert!(text.contains("'/project/layer.yaml' was already created"));
        assert!(!text.contains("No files were changed"));
        assert!(!is_reported_cancellation(&error));
    }

    #[test]
    fn flag_completion_failure_preserves_a_native_error_code_without_assuming_its_kind() {
        let directory = tempfile::tempdir().unwrap();
        let source = std::fs::File::open(directory.path().join("missing")).unwrap_err();
        let code = source.raw_os_error();
        let kind = source.kind();
        assert!(code.is_some());
        let error = finish_output(Path::new("/project/layer.yaml"), "Completed", |_| {
            Err(source)
        })
        .unwrap_err();
        assert_eq!(error.kind(), kind);
        let context = error
            .get_ref()
            .unwrap()
            .downcast_ref::<CliIoError>()
            .unwrap();
        let CliIoError::InitCompletion { source, .. } = context else {
            panic!("expected completion context")
        };
        assert_eq!(source.raw_os_error(), code);
        assert_eq!(source.kind(), kind);
    }

    #[test]
    fn input_and_cancellation_feedback_failures_retain_typed_original_causes_and_escape_data() {
        let raw = "failure\x1b[2J\n\r\t\u{202e}";
        for cancelled in [false, true] {
            let error = if cancelled {
                finish_input::<()>(Err(io::ErrorKind::Interrupted.into()), true, || {
                    Err(io::Error::new(io::ErrorKind::BrokenPipe, raw))
                })
            } else {
                finish_input::<()>(
                    Err(io::Error::new(io::ErrorKind::UnexpectedEof, raw)),
                    false,
                    || panic!("not cancelled"),
                )
            }
            .unwrap_err();
            let context = error
                .get_ref()
                .unwrap()
                .downcast_ref::<CliIoError>()
                .unwrap();
            let source = match context {
                CliIoError::Input { source } if !cancelled => source,
                CliIoError::CancellationFeedback { source } if cancelled => source,
                other => panic!("unexpected context: {other:?}"),
            };
            assert_eq!(source.to_string(), raw);
            assert_eq!(source.kind(), error.kind());
            let body = crate::diagnostics::error_message(&error);
            assert!(body.contains(&display(raw).to_string()));
            assert!(!body.contains('\x1b'));
            assert!(!body.contains('\u{202e}'));
            assert_eq!(body.matches("No files were changed.").count(), 1);
            assert!(!is_reported_cancellation(&error));
        }
    }

    #[test]
    fn submitted_empty_inputs_show_skipped_without_changing_the_value() {
        let cursor = StringCursor::default();
        let rendered = theme().format_placeholder(&ThemeState::Submit, &cursor);
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
            assert!(
                !theme()
                    .format_placeholder(&state, &cursor)
                    .contains("Skipped")
            );
        }
    }

    #[test]
    fn non_empty_defaults_keep_the_default_value_visible() {
        let mut cursor = StringCursor::default();
        cursor.extend("my-layer (default)");
        for state in [ThemeState::Active, ThemeState::Submit] {
            assert!(
                console::strip_ansi_codes(&theme().format_placeholder(&state, &cursor))
                    .contains("my-layer (default)")
            );
        }
    }

    #[test]
    fn default_name_uses_the_last_component_without_normalising_it() {
        assert_eq!(
            directory_name(Path::new("parent/my-project.config_2")).unwrap(),
            "my-project.config_2"
        );
    }

    #[test]
    fn unusable_directory_names_require_an_explicit_name() {
        for directory in [
            Path::new("/"),
            Path::new(""),
            Path::new("   "),
            Path::new("\u{85}"),
            Path::new("My-Project"),
            Path::new(" my-project "),
            Path::new("配置"),
            Path::new("\u{feff}"),
        ] {
            let error = directory_name(directory).unwrap_err();
            assert!(error.to_string().contains("Supply --name explicitly"));
        }
    }

    fn setup_failure() -> schema::Error {
        schema::validate_field("unknown", "valid").unwrap_err()
    }

    #[test]
    fn internal_default_validation_errors_do_not_request_a_name_override() {
        let error = directory_name_validating(Path::new("valid-directory"), |field, name| {
            assert_eq!((field, name), ("name", "valid-directory"));
            Err(InitError::from(setup_failure()))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(
            error
                .to_string()
                .contains("Internal layer manifest schema error")
        );
        assert!(!error.to_string().contains("Supply --name"));
        assert!(!error.to_string().contains("non-blank"));
    }

    #[test]
    fn setup_failure_returns_before_the_guided_ui_is_started() {
        let expected = setup_failure();
        let error =
            collect_init_input_ready(|| Err(expected.clone()), || panic!("UI must not start"))
                .err()
                .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(error.to_string(), expected.to_string());
        assert!(!error.to_string().contains("Could not collect"));
    }

    #[test]
    fn only_content_failures_request_inline_retry() {
        let validation = PromptValidation::default();
        let retry = validation
            .check(validate_metadata_value("name", "\u{85}"), false)
            .unwrap_err();
        assert_eq!(retry, "Enter a non-blank name.");
        assert!(validation.finish(Ok("corrected".into())).is_ok());

        let expected = setup_failure();
        assert!(
            validation
                .check(Err(InitError::from(expected.clone())), false)
                .is_ok()
        );
        let error = validation.finish(Ok("not accepted".into())).unwrap_err();
        assert_eq!(error.to_string(), expected.to_string());
        assert!(!error.to_string().contains("non-whitespace"));
    }

    #[test]
    fn nested_author_feedback_uses_structured_rules_and_optional_recovery_not_flags() {
        for (field, value, label, rule) in [
            ("author", "Alex <bad>", "author email", "email address"),
            (
                "author",
                "Alex (https://bad host)",
                "author URL",
                "explicit scheme",
            ),
            ("author.email", "bad", "author email", "email address"),
            (
                "author.url",
                "relative/path",
                "author URL",
                "explicit scheme",
            ),
            ("author.email", " ", "author email", "non-blank"),
            ("author.url", " ", "author URL", "non-blank"),
        ] {
            let message = PromptValidation::default()
                .check(validate_answer(field, value, true), true)
                .unwrap_err();
            for fact in [label, rule, "clear the field and press Enter to skip"] {
                assert!(message.contains(fact), "{message}");
            }
            assert!(!message.contains("--author"));
            if rule != "non-blank" {
                assert!(!message.contains("non-blank"));
            }
        }
        for field in ["author.email", "author.url"] {
            assert!(validate_answer(field, "", true).is_ok());
        }
    }

    #[test]
    fn generic_schema_content_feedback_keeps_its_rule_and_can_retry() {
        let validation = PromptValidation::default();
        let error = schema::validate(&serde_json::json!({"name": 42})).unwrap_err();
        let expected = error.to_string();
        assert_eq!(
            validation
                .check(Err(InitError::from(error)), false)
                .unwrap_err(),
            expected
        );
        assert!(expected.contains("/name (type)"));
        assert!(!expected.contains("non-blank"));
        assert!(validation.finish(Ok("corrected".into())).is_ok());
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
        for value in [" ", "\t", "\u{2003}", "\u{85}"] {
            assert!(validate_answer("name", value, false).is_err());
            assert!(validate_answer("author", value, true).is_err());
        }
        for value in [" name ", "quoted \"value\"", "配置 🦀", "\u{feff}"] {
            assert!(validate_answer("name", value, false).is_err());
            assert!(validate_answer("author", value, true).is_ok());
            assert!(validate_answer("description", value, true).is_ok());
        }
        for value in ["shared-config", "@example/.config", "@example/_config"] {
            assert!(validate_answer("name", value, false).is_ok());
        }
    }

    #[test]
    fn invalid_answers_explain_prompt_recovery_without_changing_flag_errors() {
        for field in ["name", "author", "description"] {
            let optional = field != "name";
            let message = PromptValidation::default()
                .check(validate_answer(field, " \t", optional), optional)
                .unwrap_err();
            assert!(message.contains(field));
            assert!(message.contains("non-blank"));
            assert!(!message.contains("--"));
            assert_eq!(
                message.contains("clear the field and press Enter to skip"),
                optional
            );
            let flag_error = validate_metadata_value(field, " \t")
                .unwrap_err()
                .to_string();
            assert!(flag_error.contains(&format!("--{field}")));
        }
    }

    #[test]
    fn guided_name_policy_errors_explain_correction_without_flags() {
        for name in ["Uppercase", " name ", "_config", &"a".repeat(215)] {
            let retry = PromptValidation::default()
                .check(validate_answer("name", name, false), false)
                .unwrap_err();
            assert!(retry.contains("Name is invalid"));
            assert!(retry.contains("lowercase ASCII"));
            assert!(!retry.contains("--name"));
            assert!(!retry.contains("non-blank"));
            assert!(!retry.contains("skip"));
            let flag = validate_metadata_value("name", name)
                .unwrap_err()
                .to_string();
            assert!(flag.contains("--name"));
        }
    }

    #[test]
    fn unrelated_validation_errors_keep_their_original_diagnostic() {
        let error = InitError::Io {
            action: "inspect layer manifest path",
            path: Path::new("example/layer.json").to_owned(),
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };
        let expected = error.to_string();
        let validation = PromptValidation::default();
        assert!(validation.check(Err(error), true).is_ok());
        assert_eq!(
            validation
                .finish(Ok("not accepted".into()))
                .unwrap_err()
                .to_string(),
            expected
        );
    }

    #[test]
    fn only_observed_bare_cancellation_with_delivered_feedback_is_reported() {
        let mut delivered = false;
        let error = finish_input::<()>(Err(io::ErrorKind::Interrupted.into()), true, || {
            delivered = true;
            Ok(())
        })
        .unwrap_err();
        assert!(delivered);
        assert!(is_reported_cancellation(&error));
        assert_eq!(
            error.to_string(),
            "Layer initialisation cancelled. No files were changed."
        );
        let error = finish_input::<()>(Err(io::ErrorKind::Interrupted.into()), true, || {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "footer failed"))
        })
        .unwrap_err();
        assert!(!is_reported_cancellation(&error));
        assert!(error.to_string().contains("cancellation feedback failed."));
        assert!(error.to_string().contains("Cause: footer failed"));
        assert!(error.to_string().contains("No files were changed"));
    }

    #[test]
    fn unrelated_interruption_eof_and_io_causes_are_not_suppressed() {
        for cancelled in [false, true] {
            let errors = [
                io::Error::new(io::ErrorKind::Interrupted, "custom input failure"),
                io::Error::from_raw_os_error(4),
                io::ErrorKind::UnexpectedEof.into(),
                io::ErrorKind::BrokenPipe.into(),
            ];
            for error in errors {
                let cause = error.to_string();
                let error = finish_input::<()>(Err(error), cancelled, || {
                    panic!("must not report cancellation")
                })
                .unwrap_err();
                assert!(!is_reported_cancellation(&error));
                assert!(error.to_string().contains(&cause));
                assert!(
                    error
                        .to_string()
                        .contains("Could not collect initialisation input")
                );
                assert!(error.to_string().contains("No files were changed"));
            }
        }
        let error = finish_input::<()>(Err(io::ErrorKind::Interrupted.into()), false, || {
            panic!("no cancellation observed")
        })
        .unwrap_err();
        assert!(!is_reported_cancellation(&error));
        assert!(error.to_string().contains("Could not collect"));
    }

    #[test]
    fn theme_uses_its_own_red_cross_and_tracks_cancel_without_claiming_delivery() {
        let mut theme = theme();
        theme.presentation.decorated = true;
        let state = ThemeState::Error("Correct this answer".into());
        let header = theme.format_header(&state, "Author");
        assert!(header.contains("\x1b[31m✕"));
        assert!(!header.contains('▲'));
        let footer = theme.format_footer_with_message(&state, "");
        assert!(footer.contains("\x1b[31m└  Correct this answer"));
        assert!(
            theme
                .format_header(&ThemeState::Active, "Name")
                .contains("\x1b[1mName")
        );
        assert!(
            theme
                .format_footer_with_message(&ThemeState::Cancel, "")
                .is_empty()
        );
        assert!(theme.cancelled.load(Ordering::Relaxed));
        let bare = io::Error::from(io::ErrorKind::Interrupted);
        assert!(!is_reported_cancellation(&bare));
        assert!(
            theme
                .state_symbol(&ThemeState::Cancel)
                .contains("\x1b[31m■")
        );
        assert!(
            theme
                .radio_item(&ThemeState::Active, true, "yaml", "")
                .contains("\x1b[36myaml")
        );
    }

    #[test]
    fn author_defaults_use_eligibility_not_name_or_entity_spelling() {
        let choices = vec![
            Choice {
                name: "Membership".into(),
                hint: "membership".into(),
                eligible: false,
                kind: AuthorProvenanceKind::Organisation,
                emails: Vec::new(),
                urls: Vec::new(),
                records: Vec::new(),
            },
            Choice {
                name: "Declared organisation".into(),
                hint: "project".into(),
                eligible: true,
                kind: AuthorProvenanceKind::Unspecified,
                emails: Vec::new(),
                urls: Vec::new(),
                records: Vec::new(),
            },
        ];
        assert_eq!(author_initial(&choices), AuthorAction::Candidate(1));
        assert_eq!(author_initial(&choices[..1]), AuthorAction::EnterAnother);
        assert_eq!(author_initial(&[]), AuthorAction::EnterAnother);
    }

    #[test]
    fn author_theme_uses_typed_actions_and_resets_without_changing_format_selector() {
        let theme = theme();
        *theme.author.lock().unwrap() = Some(AuthorSelectorTheme {
            rows: vec![
                (
                    "Skip".into(),
                    "package.json author".into(),
                    AuthorAction::Candidate(0),
                ),
                (
                    "Enter another author".into(),
                    "Git local user.name".into(),
                    AuthorAction::Candidate(1),
                ),
                (
                    "Enter another author".into(),
                    String::new(),
                    AuthorAction::EnterAnother,
                ),
                ("Skip".into(), String::new(), AuthorAction::Skip),
            ],
            kinds: vec![
                AuthorProvenanceKind::Unspecified,
                AuthorProvenanceKind::Identity,
            ],
            selected: AuthorAction::Candidate(0),
        });
        let scope = AuthorThemeScope(&theme.author);
        assert_eq!(
            theme.radio_item(&ThemeState::Submit, true, "Skip", "package.json author"),
            "Skip"
        );
        assert_eq!(
            theme.radio_item(
                &ThemeState::Submit,
                true,
                "Enter another author",
                "Git local user.name"
            ),
            "Enter another author"
        );
        assert_eq!(
            theme.radio_item(&ThemeState::Submit, true, "Skip", ""),
            "Skipped"
        );
        assert!(
            !theme
                .radio_item(&ThemeState::Submit, true, "Skip", "package.json author")
                .contains("package.json")
        );
        assert_eq!(
            theme.radio_item(&ThemeState::Active, true, "Skip", "package.json author"),
            "● ◇   Skip (package.json author)"
        );
        drop(scope);
        assert!(theme.author.lock().unwrap().is_none());
        assert_eq!(
            theme.radio_item(&ThemeState::Submit, true, "yaml", ""),
            "yaml"
        );
        assert_eq!(
            theme.radio_item(&ThemeState::Submit, true, "Skip", ""),
            "Skip"
        );
    }

    fn author_theme() -> InitTheme {
        let theme = theme();
        *theme.author.lock().unwrap() = Some(AuthorSelectorTheme {
            rows: vec![
                (
                    "Alex".into(),
                    "package.json author and Git local user.name".into(),
                    AuthorAction::Candidate(0),
                ),
                (
                    "Morgan".into(),
                    "system account name".into(),
                    AuthorAction::Candidate(1),
                ),
                (
                    "No source".into(),
                    String::new(),
                    AuthorAction::Candidate(2),
                ),
                (
                    "Enter another author".into(),
                    String::new(),
                    AuthorAction::EnterAnother,
                ),
                ("Skip".into(), String::new(), AuthorAction::Skip),
            ],
            kinds: vec![
                AuthorProvenanceKind::Identity,
                AuthorProvenanceKind::Identity,
                AuthorProvenanceKind::Unspecified,
            ],
            selected: AuthorAction::Candidate(0),
        });
        theme
    }

    #[test]
    fn only_the_focused_active_author_candidate_has_an_inline_hint() {
        let theme = author_theme();
        let first_hint = "package.json author and Git local user.name";
        let render = |selected, label, hint| {
            theme.format_select_item(&ThemeState::Active, selected, label, hint)
        };
        assert_eq!(
            render(true, "Alex", first_hint),
            "│  ● 👤  Alex (package.json author and Git local user.name)\n"
        );
        assert_eq!(render(false, "Alex", first_hint), "│  ○ 👤  Alex\n");
        assert_eq!(
            render(true, "Morgan", "system account name"),
            "│  ● 👤  Morgan (system account name)\n"
        );
        for (label, icon) in [
            ("Enter another author", "✏️ "),
            ("Skip", "⏭  "),
            ("No source", "◇  "),
        ] {
            assert_eq!(render(true, label, ""), format!("│  ● {icon} {label}\n"));
        }
        theme.author.lock().unwrap().as_mut().unwrap().rows[4].1 = "action hint".into();
        assert_eq!(render(true, "Skip", "action hint"), "│  ● ⏭   Skip\n");
        // Lookup is the exact label/hint pair, not a candidate-like label alone.
        assert_eq!(
            render(true, "Alex", "unmapped hint"),
            "│  ● Alex (unmapped hint)\n"
        );
    }

    #[test]
    fn author_details_never_leak_into_submit_cancel_or_error_rows() {
        let theme = author_theme();
        let hint = "package.json author and Git local user.name";
        for state in [ThemeState::Submit, ThemeState::Cancel] {
            assert_eq!(theme.radio_item(&state, true, "Alex", hint), "Alex");
            assert_eq!(
                theme.format_select_item(&state, true, "Alex", hint),
                "│  Alex\n"
            );
            assert!(
                theme
                    .format_select_item(&state, false, "Alex", hint)
                    .is_empty()
            );
        }
        assert_eq!(
            theme.format_select_item(&ThemeState::Error("Invalid".into()), true, "Alex", hint,),
            "│  Alex\n"
        );
        for label in ["Enter another author", "Skip"] {
            let cancelled = theme.format_select_item(&ThemeState::Cancel, true, label, "");
            assert!(!cancelled.contains('('));
        }
    }

    #[test]
    fn focused_author_details_are_dim_and_other_selectors_keep_their_layout() {
        let mut author = author_theme();
        author.presentation.decorated = true;
        let rendered =
            author.format_select_item(&ThemeState::Active, true, "Morgan", "system account name");
        assert!(rendered.contains("\x1b[2m (system account name)"));
        assert_eq!(
            console::strip_ansi_codes(&rendered),
            "│  ● 👤  Morgan (system account name)\n"
        );

        let ordinary = theme();
        for state in [ThemeState::Active, ThemeState::Submit, ThemeState::Cancel] {
            for selected in [false, true] {
                assert_eq!(
                    console::strip_ansi_codes(
                        &author.format_select_item(&state, selected, "yaml", "")
                    ),
                    ordinary.format_select_item(&state, selected, "yaml", "")
                );
            }
        }
        assert_eq!(
            ordinary.format_select_item(&ThemeState::Active, true, "toml", "ordinary hint",),
            "│  ● toml (ordinary hint)\n"
        );
    }

    #[test]
    fn inline_sources_preserve_complete_text_without_forced_line_breaks() {
        for hint in [
            "package.json author, Cargo.toml package.authors, and Git local user.name".to_owned(),
            "long-token-".repeat(24),
            "界e\u{301}".repeat(60),
        ] {
            let theme = author_theme();
            theme.author.lock().unwrap().as_mut().unwrap().rows[0].1 = hint.clone();
            let rendered = theme.format_select_item(&ThemeState::Active, true, "Alex", &hint);
            assert_eq!(rendered, format!("│  ● 👤  Alex ({hint})\n"));
            assert_eq!(rendered.lines().count(), 1);
        }
    }

    #[test]
    fn all_author_icons_share_one_slot_and_keep_hints_inline() {
        let variants = [
            (
                AuthorAction::Candidate(0),
                AuthorProvenanceKind::Identity,
                "👤",
                "U",
            ),
            (
                AuthorAction::Candidate(0),
                AuthorProvenanceKind::Organisation,
                "🏢",
                "O",
            ),
            (
                AuthorAction::Candidate(0),
                AuthorProvenanceKind::Mixed,
                "◈",
                "*",
            ),
            (
                AuthorAction::Candidate(0),
                AuthorProvenanceKind::Unspecified,
                "◇",
                "?",
            ),
            (
                AuthorAction::EnterAnother,
                AuthorProvenanceKind::Unspecified,
                "✏️",
                "+",
            ),
            (
                AuthorAction::Skip,
                AuthorProvenanceKind::Unspecified,
                "⏭",
                ">",
            ),
        ];
        let slot = author_icon_slot_width();
        assert!(slot >= 2);
        for basic in [false, true] {
            for decorated in [false, true] {
                for (action, kind, unicode, ascii) in variants {
                    let glyph = if basic { ascii } else { unicode };
                    let icon = author_icon_prefix(action, kind, basic);
                    assert!(icon.starts_with(glyph));
                    assert!(
                        icon[glyph.len()..]
                            .chars()
                            .all(|character| character == ' ')
                    );
                    assert_eq!(console::measure_text_width(&icon), slot + 2);
                    assert!(icon.ends_with("  "));

                    let mut theme = author_theme();
                    theme.presentation.basic = basic;
                    theme.presentation.decorated = decorated;
                    let (label, hint) = {
                        let mut context = theme.author.lock().unwrap();
                        let context = context.as_mut().unwrap();
                        context.kinds[0] = kind;
                        let (label, hint, _) = context
                            .rows
                            .iter()
                            .find(|(_, _, row_action)| *row_action == action)
                            .unwrap();
                        (label.clone(), hint.clone())
                    };
                    for selected in [false, true] {
                        let output =
                            theme.format_select_item(&ThemeState::Active, selected, &label, &hint);
                        if !decorated {
                            assert!(!output.contains('\x1b'), "{output:?}");
                        }
                        let plain = console::strip_ansi_codes(&output);
                        let mut lines = plain.lines();
                        let row = lines.next().unwrap();
                        assert!(row.contains(glyph));
                        let (before_name, _) = row.split_once(&label).unwrap();
                        let name_column = console::measure_text_width(before_name);
                        assert_eq!(name_column, 3 + 2 + slot + 2);
                        let expected_hint =
                            if selected && matches!(action, AuthorAction::Candidate(_)) {
                                format!(" ({hint})")
                            } else {
                                String::new()
                            };
                        assert_eq!(
                            row,
                            format!(
                                "│  {} {icon}{label}{expected_hint}",
                                if selected { "●" } else { "○" }
                            )
                        );
                        assert!(lines.next().is_none());
                    }
                    // Rendering decorates neither the raw label/hint identity
                    // nor its accepted or cancelled answer.
                    for state in [ThemeState::Submit, ThemeState::Cancel] {
                        let summary = theme.radio_item(&state, true, &label, &hint);
                        let plain = console::strip_ansi_codes(&summary);
                        let expected = match (&state, action) {
                            (ThemeState::Submit, AuthorAction::Skip) => "Skipped",
                            (ThemeState::Submit, AuthorAction::EnterAnother) => "",
                            _ => label.as_str(),
                        };
                        assert_eq!(plain, expected);
                    }
                    let context = theme.author.lock().unwrap();
                    assert!(
                        context
                            .as_ref()
                            .unwrap()
                            .rows
                            .iter()
                            .any(|row| { row == &(label.clone(), hint.clone(), action) })
                    );
                }
            }
        }
    }
}
