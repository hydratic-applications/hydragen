//! The single ErrorKind-to-presentation-facts adapter. clap still decides validity,
//! supplies choices/suggestions, and owns all argument parsing.

use clap::error::{ContextKind as K, ContextValue as V, ErrorKind as E};

use super::{
    Diagnostic,
    catalog::{Catalog, Origin},
};

pub(super) fn adapt(
    error: &clap::Error,
    catalog: &Catalog<'_>,
    origin: Origin,
) -> Option<Diagnostic> {
    let mut fields = Fields {
        error,
        used: Vec::new(),
        valid: true,
    };
    let mut details = Vec::new();
    let mut recovery = None;
    let summary = match error.kind() {
        E::ArgumentConflict => {
            let (noun, invalid) = if let Some(invalid) = fields.string(K::InvalidArg) {
                ("argument", invalid)
            } else {
                ("subcommand", fields.string(K::InvalidSubcommand)?)
            };
            match fields.value(K::PriorArg)? {
                V::String(prior) if prior == invalid => {
                    format!("The {noun} '{invalid}' cannot be used multiple times.")
                }
                V::String(prior) => {
                    format!("The {noun} '{invalid}' cannot be used with '{prior}'.")
                }
                V::Strings(prior) if !prior.is_empty() => {
                    details.push(format!("Conflicting arguments: {}", quoted(prior)));
                    format!("The {noun} '{invalid}' conflicts with other arguments.")
                }
                V::None => format!("The {noun} '{invalid}' cannot be used with other arguments."),
                _ => return None,
            }
        }
        E::NoEquals => format!(
            "An equals sign is required for '{}'.",
            fields.string(K::InvalidArg)?
        ),
        E::InvalidValue => {
            let argument = fields.string(K::InvalidArg)?;
            let value = fields.string(K::InvalidValue)?;
            let values = fields.strings(K::ValidValue)?;
            if !values.is_empty() {
                details.push(format!("Possible values: {}", values.join(", ")));
            }
            recovery = catalog.recovery(origin, error.kind(), argument);
            if value.is_empty() {
                format!("A value is required for '{argument}'.")
            } else {
                format!("Invalid value '{value}' for '{argument}'.")
            }
        }
        E::InvalidSubcommand => format!(
            "Unrecognized subcommand '{}'.",
            fields.string(K::InvalidSubcommand)?
        ),
        E::MissingRequiredArgument => {
            let missing = fields.strings(K::InvalidArg)?;
            if missing.is_empty() {
                return None;
            }
            if let [argument] = missing {
                recovery = catalog.recovery(origin, error.kind(), argument);
            }
            match recovery.and_then(|content| content.summary) {
                Some(summary) => summary.to_owned(),
                None => {
                    details.push(format!("Required arguments: {}", missing.join(", ")));
                    "Required arguments are missing.".to_owned()
                }
            }
        }
        E::MissingSubcommand => {
            let parent = fields.string(K::InvalidSubcommand)?;
            let commands = fields.strings(K::ValidSubcommand)?;
            if !commands.is_empty() {
                details.push(format!("Commands: {}", commands.join(", ")));
            }
            format!("'{parent}' requires a subcommand.")
        }
        E::InvalidUtf8 => "Invalid UTF-8 was detected in an argument.".to_owned(),
        E::TooManyValues => {
            let argument = fields.string(K::InvalidArg)?;
            let value = fields.string(K::InvalidValue)?;
            format!("Unexpected value '{value}' for '{argument}'; no more values were expected.")
        }
        E::TooFewValues => {
            let argument = fields.string(K::InvalidArg)?;
            let minimum = fields.number(K::MinValues)?;
            let actual = fields.number(K::ActualNumValues)?;
            format!(
                "At least {minimum} {} required for '{argument}'; {actual} {} provided.",
                value_verb(minimum, "required"),
                value_verb(actual, "provided")
            )
        }
        E::ValueValidation => {
            let argument = fields.string(K::InvalidArg)?;
            let value = fields.string(K::InvalidValue)?;
            format!("Invalid value '{value}' for '{argument}'.")
        }
        E::WrongNumberOfValues => {
            let argument = fields.string(K::InvalidArg)?;
            let expected = fields.number(K::ExpectedNumValues)?;
            let actual = fields.number(K::ActualNumValues)?;
            format!(
                "Expected {expected} {} for '{argument}', but {actual} {} provided.",
                if expected == 1 { "value" } else { "values" },
                value_verb(actual, "provided")
            )
        }
        E::UnknownArgument => format!("Unexpected argument '{}'.", fields.string(K::InvalidArg)?),
        E::DisplayHelpOnMissingArgumentOrSubcommand => {
            let commands = catalog.commands(origin)?;
            if commands.is_empty() {
                "Arguments are required.".to_owned()
            } else {
                details.push(format!("Commands: {}", commands.join(", ")));
                "A command is required.".to_owned()
            }
        }
        _ => return None,
    };

    for kind in [
        K::SuggestedCommand,
        K::SuggestedSubcommand,
        K::SuggestedArg,
        K::SuggestedValue,
    ] {
        match fields.value(kind) {
            Some(V::String(value)) => details.push(format!("Tip: Did you mean '{value}'?")),
            Some(V::Strings(values)) if !values.is_empty() => {
                details.push(format!("Tip: Did you mean {}?", quoted(values)))
            }
            Some(V::Strings(_)) | None => {}
            _ => return None,
        }
    }
    if let Some(value) = fields.value(K::Suggested) {
        let V::StyledStrs(suggestions) = value else {
            return None;
        };
        for suggestion in suggestions {
            // The encoded leaf has already been escaped; do not case-convert raw
            // user text in copied separator/cross-command suggestions.
            let suggestion = suggestion.ansi().to_string();
            if !suggestion.is_empty() {
                let punctuation = if suggestion.ends_with(['.', '?', '!']) {
                    ""
                } else {
                    "."
                };
                details.push(format!("Tip: {suggestion}{punctuation}"));
            }
        }
    }
    fields.value(K::Usage);
    if !fields.complete() {
        return None;
    }
    let mut model = Diagnostic::new(summary, catalog, origin);
    model.details = details;
    model.example = recovery.map(|content| content.example);
    Some(model)
}

fn quoted(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("'{value}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn value_verb(count: isize, purpose: &str) -> &'static str {
    match (count == 1, purpose) {
        (true, "required") => "value is",
        (false, "required") => "values are",
        (true, _) => "value was",
        (false, _) => "values were",
    }
}

// Track consumed keys so even a known context with an unexpected meaning cannot
// silently disappear. Typed extraction rejects unexpected value shapes.
struct Fields<'a> {
    error: &'a clap::Error,
    used: Vec<K>,
    valid: bool,
}

impl<'a> Fields<'a> {
    fn value(&mut self, kind: K) -> Option<&'a V> {
        self.used.push(kind);
        self.error.get(kind)
    }
    fn string(&mut self, kind: K) -> Option<&'a str> {
        match self.value(kind)? {
            V::String(value) => Some(value),
            _ => {
                self.valid = false;
                None
            }
        }
    }
    fn strings(&mut self, kind: K) -> Option<&'a [String]> {
        match self.value(kind)? {
            V::Strings(values) => Some(values),
            _ => None,
        }
    }
    fn number(&mut self, kind: K) -> Option<isize> {
        match self.value(kind)? {
            V::Number(value) if *value >= 0 => Some(*value),
            _ => None,
        }
    }
    fn complete(&self) -> bool {
        self.valid
            && self
                .error
                .context()
                .all(|(kind, _)| self.used.contains(&kind))
    }
}
