//! Author representation and syntax-only interpretation at explicit input boundaries.
//!
//! Loading never interprets string authors. Discovery can use `recognize` without
//! validating the whole result, retaining a valid name when a contact is invalid.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::layer_manifest::schema;

/// The original author representation. A legacy string is not parsed on load.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Author {
    String(String),
    Object(AuthorObject),
}

impl Author {
    /// The literal string or explicitly declared object name, without interpretation.
    pub fn name(&self) -> &str {
        match self {
            Self::String(name) => name,
            Self::Object(author) => &author.name,
        }
    }
}

impl From<String> for Author {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Author {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<AuthorObject> for Author {
    fn from(value: AuthorObject) -> Self {
        Self::Object(value)
    }
}

/// Known fields are ordered for generation; unknown loaded fields remain data.
/// Construction does not validate: callers validate components or the complete
/// metadata before use. `encode_metadata` always validates before encoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorObject {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl AuthorObject {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: None,
            url: None,
            extra: Map::new(),
        }
    }

    /// Uninterpreted object members retained by loading. Known keys cannot be
    /// inserted here, preventing duplicate or silently overwritten known fields.
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }
}

/// The string syntaxes supported by a source, not content acceptance policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonSyntax {
    Npm,
    Cargo,
}

/// Borrowed, unnormalised components. Recognition says nothing about validity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorParts<'a> {
    pub name: &'a str,
    pub email: Option<&'a str>,
    pub url: Option<&'a str>,
}

impl AuthorParts<'_> {
    /// Strict explicit-input validation. Discovery instead validates fields
    /// independently through `schema::validate_field` and filters bad contacts.
    pub fn validate(&self) -> Result<(), schema::Error> {
        schema::validate_field("author.name", self.name)?;
        if let Some(email) = self.email {
            schema::validate_field("author.email", email)?;
        }
        if let Some(url) = self.url {
            schema::validate_field("author.url", url)?;
        }
        Ok(())
    }

    pub fn to_object(self) -> AuthorObject {
        AuthorObject {
            name: self.name.to_owned(),
            email: self.email.map(str::to_owned),
            url: self.url.map(str::to_owned),
            extra: Map::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonInput<'a> {
    /// No recognised contact syntax. Explicit entry preserves the whole input.
    Literal(&'a str),
    Shorthand(AuthorParts<'a>),
    /// Malformed or ambiguous contact syntax. Explicit entry still treats this
    /// as a literal name; person-string discovery must skip it.
    Unrecognised(&'a str),
}

impl<'a> PersonInput<'a> {
    /// Apply explicit entry's literal fallback, without validating or normalising.
    pub fn explicit_parts(self) -> AuthorParts<'a> {
        match self {
            Self::Shorthand(parts) => parts,
            Self::Literal(name) | Self::Unrecognised(name) => AuthorParts {
                name,
                email: None,
                url: None,
            },
        }
    }
}

/// Recognise a whole npm person string, or Cargo's trailing-email form.
/// This only identifies balanced suffix delimiters and an explicit URI scheme;
/// it deliberately does not implement email or URI content validation.
pub fn recognize(input: &str, syntax: PersonSyntax) -> PersonInput<'_> {
    let mut rest = input;
    let mut url = None;
    if syntax == PersonSyntax::Npm && rest.ends_with(')') {
        // Nested delimiters are ambiguous person syntax, even when the payload
        // might happen to be a valid URI. Do not partially extract them.
        if let Some(start) = rest.rfind('(') {
            let payload = &rest[start + 1..rest.len() - 1];
            if has_scheme(payload) && !payload.contains(['(', ')', '<', '>']) {
                url = Some(payload);
                rest = rest[..start].trim_end_matches(char::is_whitespace);
            }
        }
    }
    let mut email = None;
    if rest.ends_with('>')
        && let Some(start) = rest.rfind('<')
    {
        let payload = &rest[start + 1..rest.len() - 1];
        if !payload.is_empty() && !payload.contains(['<', '>', '(', ')']) {
            email = Some(payload);
            rest = rest[..start].trim_end_matches(char::is_whitespace);
        }
    }
    if email.is_some() || url.is_some() {
        // Reject competing contact delimiters, not punctuation in the name.
        // In npm syntax a scheme-bearing parenthetical before these suffixes
        // would be a reversed or repeated URL; ordinary `(team)`, `@team`, and
        // bare URI text are still part of the declared name. Cargo does not
        // interpret parentheticals as contacts at all.
        if rest.is_empty()
            || rest.contains(['<', '>'])
            || (syntax == PersonSyntax::Npm
                && rest
                    .match_indices('(')
                    .any(|(start, _)| has_scheme(&rest[start + 1..])))
        {
            return PersonInput::Unrecognised(input);
        }
        return PersonInput::Shorthand(AuthorParts {
            name: rest,
            email,
            url,
        });
    }
    if input.contains(['@', '<', '>'])
        || input.contains("://")
        || (syntax == PersonSyntax::Npm && input.contains(['(', ')']))
    {
        PersonInput::Unrecognised(input)
    } else {
        PersonInput::Literal(input)
    }
}

// An interpretation boundary only: format validity remains the schema's job.
fn has_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    scheme.starts_with(|character: char| character.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "+-.".contains(character))
}

/// Interpret explicitly supplied Author text and strictly validate every captured
/// component. Empty optional guided input must be handled as omission by the UI.
pub fn interpret(input: &str) -> Result<AuthorObject, schema::Error> {
    let parts = recognize(input, PersonSyntax::Npm).explicit_parts();
    parts.validate()?;
    Ok(parts.to_object())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn complete_suffixes_are_recognized_without_component_normalization() {
        for (input, name, email, url) in [
            (
                "Alex <alex@example.com>",
                "Alex",
                Some("alex@example.com"),
                None,
            ),
            (
                "Alex (https://alex.example)",
                "Alex",
                None,
                Some("https://alex.example"),
            ),
            (
                "Alex <alex@example.com> (https://alex.example)",
                "Alex",
                Some("alex@example.com"),
                Some("https://alex.example"),
            ),
            (
                "  Zoë O’Connor, Jr. & 水 🦀\t<Alex+work@EXAMPLE.invalid>\u{2003}(mailto:Alex@example.invalid)",
                "  Zoë O’Connor, Jr. & 水 🦀",
                Some("Alex+work@EXAMPLE.invalid"),
                Some("mailto:Alex@example.invalid"),
            ),
            (
                "Alex<alex@example.com>(urn:example:person)",
                "Alex",
                Some("alex@example.com"),
                Some("urn:example:person"),
            ),
        ] {
            let expected = AuthorParts { name, email, url };
            assert_eq!(
                recognize(input, PersonSyntax::Npm),
                PersonInput::Shorthand(expected)
            );
            assert_eq!(interpret(input).unwrap(), expected.to_object());
        }
    }

    #[test]
    fn syntax_only_recognition_does_not_discard_a_name_with_an_invalid_contact() {
        for (input, field, keyword) in [
            ("Alex <bad>", "author.email", "format"),
            ("Alex < alex@example.com >", "author.email", "format"),
            (
                "Alex (https://alex.example/with space)",
                "author.url",
                "format",
            ),
            ("Alex <\u{2003}>", "author.email", "pattern"),
        ] {
            let PersonInput::Shorthand(parts) = recognize(input, PersonSyntax::Npm) else {
                panic!("must recognize balanced syntax, even with invalid content: {input}");
            };
            schema::validate_field("author.name", parts.name).unwrap();
            assert_eq!(parts.name, "Alex");
            let schema::Error::Content(failure) = interpret(input).unwrap_err() else {
                panic!("expected component content failure");
            };
            assert_eq!(
                failure.instance_path,
                format!("/{}", field.replace('.', "/"))
            );
            assert_eq!(failure.keyword, keyword);
            assert!(failure.schema_path.starts_with("/$defs/"));
        }
        let PersonInput::Shorthand(parts) =
            recognize("Alex <bad> (https://alex.invalid)", PersonSyntax::Npm)
        else {
            panic!()
        };
        assert_eq!(parts.email, Some("bad"));
        assert_eq!(parts.url, Some("https://alex.invalid"));
        schema::validate_field("author.url", parts.url.unwrap()).unwrap();
    }

    #[test]
    fn arbitrary_name_punctuation_does_not_hide_a_recognized_contact_suffix() {
        for name in [
            "Alex (team)",
            "  Example @team",
            "Example https://team.invalid",
            "配置 (Research (team)) @example",
        ] {
            for suffix in [
                "<alex@example.com>",
                "(https://alex.invalid)",
                "<alex@example.com> (https://alex.invalid)",
            ] {
                let input = format!("{name}\t{suffix}");
                let PersonInput::Shorthand(parts) = recognize(&input, PersonSyntax::Npm) else {
                    panic!("name punctuation must not suppress suffix recognition: {input}");
                };
                assert_eq!(parts.name, name);
                assert_eq!(
                    parts.email,
                    suffix.starts_with('<').then_some("alex@example.com")
                );
                assert_eq!(
                    parts.url,
                    suffix.ends_with(')').then_some("https://alex.invalid")
                );
                assert_eq!(interpret(&input).unwrap(), parts.to_object());
            }
            for (suffix, path) in [
                ("<not-an-email>", "/author/email"),
                ("(https://alex.invalid/with space)", "/author/url"),
            ] {
                let input = format!("{name} {suffix}");
                let PersonInput::Shorthand(parts) = recognize(&input, PersonSyntax::Npm) else {
                    panic!("invalid contact content must remain visible to discovery: {input}");
                };
                assert_eq!(parts.name, name);
                schema::validate_field("author.name", parts.name).unwrap();
                let schema::Error::Content(failure) = interpret(&input).unwrap_err() else {
                    panic!()
                };
                assert_eq!(failure.instance_path, path);
                assert_eq!(failure.keyword, "format");
                assert!(matches!(failure.kind, schema::ViolationKind::Format { .. }));
            }
            let input = format!("{name} <not-an-email>");
            assert_eq!(
                recognize(&input, PersonSyntax::Cargo),
                PersonInput::Shorthand(AuthorParts {
                    name,
                    email: Some("not-an-email"),
                    url: None,
                })
            );
        }
    }

    #[test]
    fn punctuation_names_do_not_allow_partial_extraction_of_competing_contacts() {
        for name in [
            "Alex (team)",
            "Example @team",
            "Example https://team.invalid",
        ] {
            for suffix in [
                "<a@example.com> <b@example.com>",
                "<bad> <also-bad>",
                "(https://first.invalid) <a@example.com>",
                "(https://first.invalid) <bad>",
                "(https://first.invalid) (https://second.invalid)",
                "<a@example.com> (https://first.invalid) (https://second.invalid)",
                "(https://first.invalid) <a@example.com> (https://second.invalid)",
            ] {
                let input = format!("{name} {suffix}");
                assert_eq!(
                    recognize(&input, PersonSyntax::Npm),
                    PersonInput::Unrecognised(&input)
                );
                assert_eq!(interpret(&input).unwrap(), AuthorObject::new(&input));
            }
        }
    }

    #[test]
    fn literal_and_unrecognized_input_has_exact_explicit_fallback_and_no_partial_parse() {
        for input in [
            "Example team",
            "  配置 🦀  ",
            "line one\nline two",
            "Alex (team)",
            "Alex <alex@example.com",
            "Alex alex@example.com>",
            "Alex <>",
            "Alex <<alex@example.com>>",
            "Alex <a@example.com> trailing",
            "Alex <a@example.com> <b@example.com>",
            "Alex (https://alex.example) trailing",
            "Alex (https://alex.example) <a@example.com>",
            "Alex (https://alex.example) (https://other.example)",
            "Alex (https://alex.example/a(b))",
            "Alex (1http:example)",
            "Alex (relative/path)",
            "Alex <alex@example.com> (team)",
            "Alex <alex@example.com> ",
            "alex@example.com",
            "https://example.com",
        ] {
            let recognized = recognize(input, PersonSyntax::Npm);
            assert!(
                !matches!(recognized, PersonInput::Shorthand(_)),
                "{input:?}"
            );
            let author = interpret(input).unwrap();
            assert_eq!(author, AuthorObject::new(input), "{input:?}");
            assert_eq!(recognized.explicit_parts().name, input);
        }
        assert!(matches!(
            recognize("Example team", PersonSyntax::Npm),
            PersonInput::Literal(_)
        ));
        assert!(matches!(
            recognize("Alex (team)", PersonSyntax::Npm),
            PersonInput::Unrecognised(_)
        ));
        for input in ["", "\u{85}\u{a0}\u{2003}"] {
            let schema::Error::Content(failure) = interpret(input).unwrap_err() else {
                panic!()
            };
            assert_eq!(failure.instance_path, "/author/name");
            assert_eq!(failure.kind, schema::ViolationKind::NonBlank);
        }
    }

    #[test]
    fn cargo_only_interprets_trailing_email_and_never_a_url() {
        let input = "  Example (Research) <bad>";
        assert_eq!(
            recognize(input, PersonSyntax::Cargo),
            PersonInput::Shorthand(AuthorParts {
                name: "  Example (Research)",
                email: Some("bad"),
                url: None,
            })
        );
        assert_eq!(
            recognize("Example (Research)", PersonSyntax::Cargo),
            PersonInput::Literal("Example (Research)")
        );
        assert!(matches!(
            recognize(
                "Alex <a@example.com> (https://example.com)",
                PersonSyntax::Cargo
            ),
            PersonInput::Unrecognised(_)
        ));
        assert_eq!(
            recognize("Alex (https://example.com)", PersonSyntax::Cargo),
            PersonInput::Unrecognised("Alex (https://example.com)")
        );
    }

    #[test]
    fn parts_preserve_contact_payloads_even_when_schema_invalid() {
        let PersonInput::Shorthand(parts) = recognize(
            " Alex\t< a@example.com > (https://example.com/ )",
            PersonSyntax::Npm,
        ) else {
            panic!()
        };
        assert_eq!(parts.name, " Alex");
        assert_eq!(parts.email, Some(" a@example.com "));
        assert_eq!(parts.url, Some("https://example.com/ "));
        assert!(parts.validate().is_err());
    }

    #[test]
    fn representation_and_uninterpreted_members_are_serialized_without_conversion() {
        for value in [
            json!("  Alex <bad>\n arbitrary legacy text  "),
            json!({"name": "  Alex <literal>  ", "email": "a@example.invalid", "url": "mailto:a@example.invalid", "custom": {"nested": [null, false, 7]}, "other": "preserved"}),
        ] {
            schema::validate_value("author", &value).unwrap();
            let author: Author = serde_json::from_value(value.clone()).unwrap();
            match &author {
                Author::String(text) => assert_eq!(value, json!(text)),
                Author::Object(object) => {
                    assert_eq!(object.name, "  Alex <literal>  ");
                    assert_eq!(object.extra().len(), 2);
                    assert_eq!(
                        object.extra()["custom"],
                        json!({"nested": [null, false, 7]})
                    );
                }
            }
            assert_eq!(serde_json::to_value(&author).unwrap(), value);
            assert_eq!(author.clone(), author);
        }
    }
}
