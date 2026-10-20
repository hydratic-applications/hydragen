//! Best-effort read-only author discovery. No terminal or file-creation policy lives here.

mod github;
mod identity;
mod preparation;
mod process;
mod project;

use std::collections::HashMap;

use hydragen::layer_manifest::schema;

use preparation::Context;
pub use preparation::{Preparation, Status};

// Test-only source injection for the binary's terminal driver. Release routing
// has no environment switch and still uses Preparation::start exclusively.
#[cfg(all(test, unix))]
pub(crate) mod testing {
    use super::*;
    use std::io;
    use std::sync::Arc;
    use std::time::Instant;

    pub(crate) struct Publisher<'a>(&'a Context);
    pub(crate) type Lane = Box<dyn FnOnce(&Publisher<'_>) -> Result<(), schema::Error> + Send>;

    impl Publisher<'_> {
        pub(crate) fn active(&self) -> bool {
            self.0.active()
        }

        pub(crate) fn publish(
            &self,
            name: &str,
            membership_only: bool,
        ) -> Result<bool, schema::Error> {
            let source = if membership_only {
                Source::GithubOrganization {
                    login: "fixture-org".into(),
                    host: "github.com".into(),
                }
            } else {
                Source::Project {
                    file: ProjectFile::PackageJson,
                    index: 0,
                }
            };
            self.0.publish(name, source)
        }

        /// Deterministic contact-bearing evidence for controlled terminal tests.
        /// Uses the same validation, ordering, and freeze boundary as collectors.
        pub(crate) fn publish_contacts(
            &self,
            name: &str,
            source: Source,
            contacts: &[ContactInput<'_>],
        ) -> Result<bool, schema::Error> {
            self.0.publish_contacts(name, source, contacts)
        }

        pub(crate) fn enrich_github_profile(
            &self,
            host: &str,
            login: &str,
            contacts: &[ContactInput<'_>],
        ) -> Result<bool, schema::Error> {
            self.0.enrich_github_profile(host, login, contacts)
        }

        #[cfg(unix)]
        pub(crate) fn run(&self, command: std::process::Command) {
            let _ = process::run(command, self.0, None);
        }
    }

    pub(crate) fn start(
        clock: Arc<dyn Fn() -> Instant + Send + Sync>,
        lanes: [Lane; 2],
    ) -> io::Result<Preparation> {
        Preparation::start_lanes(
            clock,
            lanes.map(|lane| -> preparation::Lane {
                Box::new(move |context| lane(&Publisher(context)))
            }),
        )
    }

    #[test]
    fn contact_publisher_and_profile_enrichment_use_real_validation_and_freeze() {
        let context = Context::for_tests();
        let publisher = Publisher(&context);
        publisher.publish("Project", false).unwrap();
        publisher
            .publish_contacts(
                "Profile",
                Source::GithubProfile {
                    host: "fixture.example".into(),
                    login: "alex".into(),
                },
                &[ContactInput {
                    field: ContactField::ProfileUrl,
                    value: " https://fixture.example/alex ",
                }],
            )
            .unwrap();
        publisher
            .enrich_github_profile(
                "fixture.example",
                "alex",
                &[
                    ContactInput {
                        field: ContactField::Email,
                        value: "bad",
                    },
                    ContactInput {
                        field: ContactField::Email,
                        value: " alex@example.invalid ",
                    },
                ],
            )
            .unwrap();
        let frozen = context.choices_for_tests().unwrap();
        assert_eq!(frozen[1].emails[0].value, "alex@example.invalid");
        assert_eq!(frozen[1].urls[0].value, "https://fixture.example/alex");
        context.close_for_tests();
        assert!(
            !publisher
                .enrich_github_profile("fixture.example", "alex", &[])
                .unwrap()
        );
        assert!(
            !publisher
                .publish_contacts("Late", Source::AccountName, &[])
                .unwrap()
        );
        assert_eq!(context.choices_for_tests().unwrap(), frozen);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub name: String,
    pub hint: String,
    pub eligible: bool,
    pub kind: AuthorProvenanceKind,
    /// Independently grouped contacts from this exact-name group only.
    pub emails: Vec<ContactChoice>,
    pub urls: Vec<ContactChoice>,
    /// Retained source records, not metadata to write to a layer manifest.
    pub records: Vec<Record>,
}

/// A contact's provenance describes only sources supplying this contact, not
/// every source supplying the corresponding name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactChoice {
    pub value: String,
    pub hint: String,
    pub kind: AuthorProvenanceKind,
}

/// The declared field supplying a contact. Website precedes profile URL even
/// when later enrichment publishes the website after the profile URL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ContactField {
    Email,
    Url,
    Website,
    ProfileUrl,
}

impl ContactField {
    fn schema_field(self) -> &'static str {
        match self {
            Self::Email => "author.email",
            Self::Url | Self::Website | Self::ProfileUrl => "author.url",
        }
    }
}

/// Unvalidated collector input. Discovery trims and validates each value
/// independently; this is never an explicit-user-input interpretation boundary.
#[derive(Clone, Copy, Debug)]
pub(super) struct ContactInput<'a> {
    pub field: ContactField,
    pub value: &'a str,
}

/// Evidence about a suggestion's sources, not a verified person/entity type.
/// Identity settings may themselves contain a company or team name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorProvenanceKind {
    Identity,
    Organisation,
    Mixed,
    Unspecified,
}

impl AuthorProvenanceKind {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unspecified, kind) | (kind, Self::Unspecified) => kind,
            (Self::Identity, Self::Identity) => Self::Identity,
            (Self::Organisation, Self::Organisation) => Self::Organisation,
            _ => Self::Mixed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ProjectFile {
    PackageJson,
    CargoToml,
    PyprojectToml,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum GitKey {
    Author,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum GitScope {
    Command,
    Worktree,
    Local,
    Global,
    System,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Project {
        file: ProjectFile,
        index: usize,
    },
    GitAuthorEnvironment,
    Git {
        key: GitKey,
        scope: GitScope,
        index: usize,
    },
    GithubProfile {
        login: String,
        host: String,
    },
    AccountName,
    AccountUsername,
    EnvironmentUsername {
        variable: &'static str,
        index: usize,
    },
    GithubOrganization {
        login: String,
        host: String,
    },
}

impl Source {
    fn contact_hint(&self, field: ContactField) -> Option<String> {
        use ContactField::{Email, ProfileUrl, Url, Website};
        let hint = match (self, field) {
            (
                Self::Project {
                    file: ProjectFile::PackageJson,
                    ..
                },
                Email,
            ) => "package.json author.email".to_owned(),
            (
                Self::Project {
                    file: ProjectFile::PackageJson,
                    ..
                },
                Url,
            ) => "package.json author.url".to_owned(),
            (
                Self::Project {
                    file: ProjectFile::CargoToml,
                    ..
                },
                Email,
            ) => "Cargo.toml package.authors email".to_owned(),
            (
                Self::Project {
                    file: ProjectFile::PyprojectToml,
                    ..
                },
                Email,
            ) => "pyproject.toml project.authors.email".to_owned(),
            (Self::GitAuthorEnvironment, Email) => "GIT_AUTHOR_EMAIL".to_owned(),
            (Self::Git { .. }, Email) => {
                let name_hint = self.hint()?;
                format!("{}.email", name_hint.strip_suffix(".name")?)
            }
            (
                Self::GithubProfile { .. } | Self::GithubOrganization { .. },
                Email | Website | ProfileUrl,
            ) => {
                let field = match field {
                    Email => "email",
                    Website => "website",
                    ProfileUrl => "profile URL",
                    Url => unreachable!(),
                };
                format!("{} {field}", self.hint()?)
            }
            // Account names, unrelated project fields, and unsupported Git
            // fields cannot acquire fabricated contact evidence.
            _ => return None,
        };
        Some(hint)
    }

    fn provenance_kind(&self) -> AuthorProvenanceKind {
        match self {
            Self::Project { .. } => AuthorProvenanceKind::Unspecified,
            Self::GithubOrganization { .. } => AuthorProvenanceKind::Organisation,
            Self::GitAuthorEnvironment
            | Self::Git { .. }
            | Self::GithubProfile { .. }
            | Self::AccountName
            | Self::AccountUsername
            | Self::EnvironmentUsername { .. } => AuthorProvenanceKind::Identity,
        }
    }

    fn order(&self) -> (u8, usize, usize, usize) {
        match self {
            Self::Project { file, index } => (0, *file as usize, *index, 0),
            Self::GitAuthorEnvironment => (1, 0, 0, 0),
            Self::Git { key, scope, index } => {
                (2 + *key as u8, *scope as usize, usize::MAX - index, 0)
            }
            Self::GithubProfile { .. } => (4, 0, 0, 0),
            Self::AccountName => (5, 0, 0, 0),
            Self::AccountUsername => (6, 0, 0, 0),
            Self::EnvironmentUsername { index, .. } => (7, *index, 0, 0),
            Self::GithubOrganization { .. } => (8, 0, 0, 0),
        }
    }

    fn hint(&self) -> Option<String> {
        let hint = match self {
            Self::Project { file, .. } => match file {
                ProjectFile::PackageJson => "package.json author",
                ProjectFile::CargoToml => "Cargo.toml package.authors",
                ProjectFile::PyprojectToml => "pyproject.toml project.authors",
            }
            .to_owned(),
            Self::GitAuthorEnvironment => "GIT_AUTHOR_NAME".to_owned(),
            Self::Git { key, scope, .. } => {
                let key = match key {
                    GitKey::Author => "author.name",
                    GitKey::User => "user.name",
                };
                let scope = match scope {
                    GitScope::Command => "command",
                    GitScope::Worktree => "worktree",
                    GitScope::Local => "local",
                    GitScope::Global => "global",
                    GitScope::System => "system",
                };
                format!("Git {scope} {key}")
            }
            Self::GithubProfile { login, host } | Self::GithubOrganization { login, host } => {
                if !safe_component(login) || !safe_component(host) {
                    return None;
                }
                let kind = if matches!(self, Self::GithubProfile { .. }) {
                    "profile"
                } else {
                    "organisation"
                };
                if host.eq_ignore_ascii_case("github.com") {
                    format!("GitHub {kind} {login}")
                } else {
                    format!("GitHub {kind} {login} on {host}")
                }
            }
            Self::AccountName => "system account name".to_owned(),
            Self::AccountUsername => "system account username".to_owned(),
            Self::EnvironmentUsername { variable, .. } => variable.to_string(),
        };
        safe_component(&hint).then_some(hint)
    }
}

pub(super) fn safe_component(value: &str) -> bool {
    !value.trim().is_empty()
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{2028}' | '\u{2029}'))
}

pub(super) fn normalize_candidate(value: &str) -> Result<Option<String>, schema::Error> {
    normalize_with(value, &|value| schema::validate_field("author.name", value))
}

fn normalize_with(
    value: &str,
    validate: &dyn Fn(&str) -> Result<(), schema::Error>,
) -> Result<Option<String>, schema::Error> {
    let value = value.trim();
    if !safe_component(value) {
        return Ok(None);
    }
    match validate(value) {
        Ok(()) => Ok(Some(value.to_owned())),
        Err(schema::Error::Content(_)) => Ok(None),
        Err(error @ schema::Error::Setup { .. }) => Err(error),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    name: String,
    source: Source,
    hint: String,
    contacts: Vec<ContactEvidence>,
    /// Opaque Git origin bytes retained only for association/identity, never hints.
    git_origin: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContactEvidence {
    field: ContactField,
    value: String,
    hint: String,
}

impl Record {
    /// Merge new fields without replacing earlier evidence or duplicating rows.
    /// The caller must match both the exact name and the complete source identity.
    fn merge_contacts(&mut self, contacts: Vec<ContactEvidence>) {
        for contact in contacts {
            if !self.contacts.contains(&contact) {
                self.contacts.push(contact);
            }
        }
        self.contacts.sort_by_key(|contact| contact.field);
    }
}

fn choices(mut records: Vec<Record>) -> Vec<Choice> {
    records.sort_by(|a, b| {
        a.source
            .order()
            .cmp(&b.source.order())
            .then_with(|| match (&a.source, &b.source) {
                (
                    Source::GithubOrganization { login: a_login, .. },
                    Source::GithubOrganization { login: b_login, .. },
                ) => a.name.cmp(&b.name).then(a_login.cmp(b_login)),
                _ => std::cmp::Ordering::Equal,
            })
    });
    let mut indices = HashMap::new();
    let mut result: Vec<Choice> = Vec::new();
    let mut hints: Vec<Vec<String>> = Vec::new();
    for record in records {
        let eligible = !matches!(record.source, Source::GithubOrganization { .. });
        let index = *indices.entry(record.name.clone()).or_insert_with(|| {
            result.push(Choice {
                name: record.name.clone(),
                hint: String::new(),
                eligible: false,
                kind: AuthorProvenanceKind::Unspecified,
                emails: Vec::new(),
                urls: Vec::new(),
                records: Vec::new(),
            });
            hints.push(Vec::new());
            result.len() - 1
        });
        result[index].eligible |= eligible;
        result[index].kind = result[index].kind.merge(record.source.provenance_kind());
        if !hints[index].contains(&record.hint) {
            hints[index].push(record.hint.clone());
        }
        result[index].records.push(record);
    }
    for (choice, hints) in result.iter_mut().zip(hints) {
        choice.hint = join_source_hints(&hints);
        choice.emails = contact_choices(&choice.records, true);
        choice.urls = contact_choices(&choice.records, false);
    }
    result
}

fn contact_choices(records: &[Record], email: bool) -> Vec<ContactChoice> {
    let mut indices = HashMap::new();
    let mut result: Vec<ContactChoice> = Vec::new();
    let mut hints: Vec<Vec<String>> = Vec::new();
    for record in records {
        for contact in &record.contacts {
            if (contact.field == ContactField::Email) != email {
                continue;
            }
            let index = *indices.entry(&contact.value).or_insert_with(|| {
                result.push(ContactChoice {
                    value: contact.value.clone(),
                    hint: String::new(),
                    kind: AuthorProvenanceKind::Unspecified,
                });
                hints.push(Vec::new());
                result.len() - 1
            });
            result[index].kind = result[index].kind.merge(record.source.provenance_kind());
            if !hints[index].contains(&contact.hint) {
                hints[index].push(contact.hint.clone());
            }
        }
    }
    for (choice, hints) in result.iter_mut().zip(hints) {
        choice.hint = join_source_hints(&hints);
    }
    result
}

fn join_source_hints(hints: &[String]) -> String {
    match hints {
        [] => String::new(),
        [hint] => hint.clone(),
        [first, second] => format!("{first} and {second}"),
        [earlier @ .., last] => format!("{}, and {last}", earlier.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contact(field: ContactField, value: &str) -> ContactInput<'_> {
        ContactInput { field, value }
    }

    #[test]
    fn contact_groups_are_independent_source_ordered_and_use_only_actual_evidence() {
        use ContactField::{Email, ProfileUrl, Url, Website};
        let context = Context::for_tests();
        let organisation = Source::GithubOrganization {
            host: "github.com".into(),
            login: "team".into(),
        };
        context
            .publish_contacts(
                " Same ",
                organisation.clone(),
                &[
                    contact(Email, "shared@example.invalid"),
                    contact(Website, "https://team.invalid"),
                    contact(ProfileUrl, "https://github.com/team"),
                ],
            )
            .unwrap();
        context.publish("Same", Source::AccountName).unwrap();
        context
            .publish_contacts(
                "Same",
                Source::Git {
                    key: GitKey::User,
                    scope: GitScope::Local,
                    index: 0,
                },
                &[
                    contact(Email, "identity@example.invalid"),
                    contact(Email, "shared@example.invalid"),
                ],
            )
            .unwrap();
        context
            .publish_contacts(
                "Same",
                Source::Project {
                    file: ProjectFile::PackageJson,
                    index: 0,
                },
                &[
                    contact(Email, " project@example.invalid "),
                    contact(Email, "shared@example.invalid"),
                    contact(Url, "https://project.invalid"),
                ],
            )
            .unwrap();
        context
            .publish_contacts(
                "Another",
                Source::Project {
                    file: ProjectFile::CargoToml,
                    index: 0,
                },
                &[contact(Email, "other@example.invalid")],
            )
            .unwrap();
        // Repeated enrichment is an update to one source record, not a new row.
        context
            .publish_contacts(
                "Same",
                organisation,
                &[contact(Email, "shared@example.invalid")],
            )
            .unwrap();

        let groups = context.choices_for_tests().unwrap();
        let selected = &groups[0]; // The selector can bind this frozen index.
        assert_eq!(selected.name, "Same");
        assert_eq!(selected.kind, AuthorProvenanceKind::Mixed);
        assert_eq!(selected.records.len(), 4);
        assert_eq!(
            selected
                .emails
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>(),
            [
                "project@example.invalid",
                "shared@example.invalid",
                "identity@example.invalid",
            ]
        );
        assert_eq!(selected.emails[0].hint, "package.json author.email");
        assert_eq!(selected.emails[0].kind, AuthorProvenanceKind::Unspecified);
        assert_eq!(
            selected.emails[1].hint,
            "package.json author.email, Git local user.email, and GitHub organisation team email"
        );
        assert_eq!(selected.emails[1].kind, AuthorProvenanceKind::Mixed);
        assert_eq!(selected.emails[2].kind, AuthorProvenanceKind::Identity);
        assert_eq!(
            selected
                .urls
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>(),
            [
                "https://project.invalid",
                "https://team.invalid",
                "https://github.com/team",
            ]
        );
        assert_eq!(selected.urls[0].kind, AuthorProvenanceKind::Unspecified);
        assert_eq!(selected.urls[1].kind, AuthorProvenanceKind::Organisation);
        assert_eq!(selected.urls[1].hint, "GitHub organisation team website");
        assert_eq!(
            selected.urls[2].hint,
            "GitHub organisation team profile URL"
        );
        assert!(
            !selected
                .emails
                .iter()
                .any(|c| c.hint.contains("system account"))
        );
        assert_eq!(groups[1].emails[0].value, "other@example.invalid");
        // Choosing/skipping/customising an email has no operation on the URL pool.
        let urls = selected.urls.clone();
        let _email = selected.emails[2].clone();
        assert_eq!(selected.urls, urls);
    }

    #[test]
    fn contact_equality_is_exact_and_source_records_survive_equal_hints() {
        use ContactField::{Email, Url};
        let context = Context::for_tests();
        for (index, email) in [
            "Alex@example.invalid",
            "alex@example.invalid",
            "Alex@example.invalid",
        ]
        .into_iter()
        .enumerate()
        {
            context
                .publish_contacts(
                    "Alex",
                    Source::Project {
                        file: ProjectFile::CargoToml,
                        index,
                    },
                    &[contact(Email, email)],
                )
                .unwrap();
        }
        context
            .publish_contacts(
                "Alex",
                Source::Project {
                    file: ProjectFile::PackageJson,
                    index: 0,
                },
                &[
                    contact(Url, "https://EXAMPLE.invalid"),
                    contact(Url, "https://example.invalid"),
                    contact(Url, "https://example.invalid/"),
                    contact(Url, "https://example.invalid"),
                ],
            )
            .unwrap();
        context
            .publish_contacts(
                "alex",
                Source::Project {
                    file: ProjectFile::CargoToml,
                    index: 3,
                },
                &[contact(Email, "separate@example.invalid")],
            )
            .unwrap();
        let groups = context.choices_for_tests().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].records.len(), 4);
        assert_eq!(
            groups[0]
                .emails
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>(),
            ["Alex@example.invalid", "alex@example.invalid",]
        );
        assert_eq!(groups[0].emails[0].hint, "Cargo.toml package.authors email");
        assert_eq!(groups[0].urls.len(), 3);
        assert_eq!(groups[1].name, "alex");
        assert_eq!(groups[1].emails[0].value, "separate@example.invalid");
    }

    #[test]
    fn contacts_filter_independently_and_cannot_be_invented_for_name_only_sources() {
        use ContactField::{Email, Url, Website};
        let context = Context::for_tests();
        context
            .publish_contacts(
                " Alex <literal> ",
                Source::Project {
                    file: ProjectFile::PackageJson,
                    index: 0,
                },
                &[
                    contact(Email, "bad"),
                    contact(Email, "a\n@example.invalid"),
                    contact(Email, "a\u{2028}b@example.invalid"),
                    contact(Url, "relative/path"),
                    contact(Url, "https://example.invalid/with space"),
                    contact(Url, "https://example.invalid/\u{1b}[31m"),
                    contact(Url, " mailto:Alex@example.invalid "),
                ],
            )
            .unwrap();
        context
            .publish_contacts(
                "No contacts",
                Source::AccountName,
                &[
                    contact(Email, "invented@example.invalid"),
                    contact(Url, "https://invented.invalid"),
                ],
            )
            .unwrap();
        context
            .publish_contacts(
                "Cargo",
                Source::Project {
                    file: ProjectFile::CargoToml,
                    index: 0,
                },
                &[
                    contact(Email, " cargo@example.invalid "),
                    contact(Url, "https://project-homepage.invalid"),
                ],
            )
            .unwrap();
        context
            .publish_contacts(
                "Github",
                Source::GithubProfile {
                    host: "github.com".into(),
                    login: "alex".into(),
                },
                &[
                    contact(Email, " alex@example.invalid "),
                    contact(Website, "example.invalid"),
                ],
            )
            .unwrap();
        let groups = context.choices_for_tests().unwrap();
        let literal = &groups[0];
        assert_eq!(literal.name, "Alex <literal>");
        assert!(literal.emails.is_empty());
        assert_eq!(literal.urls[0].value, "mailto:Alex@example.invalid");
        assert_eq!(groups[1].emails[0].value, "cargo@example.invalid");
        assert!(groups[1].urls.is_empty());
        assert_eq!(groups[2].emails[0].value, "alex@example.invalid");
        assert!(groups[2].urls.is_empty());
        assert_eq!(groups[3].name, "No contacts");
        assert!(groups[3].emails.is_empty() && groups[3].urls.is_empty());
    }

    #[test]
    fn github_hints_omit_the_public_host_but_keep_enterprise_context_and_safety() {
        for (host, suffix) in [
            ("github.com", ""),
            ("GITHUB.COM", ""),
            ("enterprise.example", " on enterprise.example"),
            (
                "github.com.enterprise.example",
                " on github.com.enterprise.example",
            ),
        ] {
            for (source, kind) in [
                (
                    Source::GithubProfile {
                        login: "alex".into(),
                        host: host.into(),
                    },
                    "profile",
                ),
                (
                    Source::GithubOrganization {
                        login: "alex".into(),
                        host: host.into(),
                    },
                    "organisation",
                ),
            ] {
                assert_eq!(source.hint(), Some(format!("GitHub {kind} alex{suffix}")));
            }
        }
        for (login, host) in [
            ("bad\nlogin", "github.com"),
            ("alex", "github.com\n"),
            ("alex", ""),
        ] {
            assert!(
                Source::GithubProfile {
                    login: login.into(),
                    host: host.into()
                }
                .hint()
                .is_none()
            );
            assert!(
                Source::GithubOrganization {
                    login: login.into(),
                    host: host.into()
                }
                .hint()
                .is_none()
            );
        }
    }

    #[test]
    fn normalization_preserves_spelling_and_rejects_unsafe_text() {
        for value in [
            "",
            " \t ",
            "Alex\nMorgan",
            "a\u{2028}b",
            "a\u{2029}b",
            "a\u{1b}[31m",
        ] {
            assert_eq!(normalize_candidate(value).unwrap(), None);
        }
        for value in ["Alex Morgan", "alex morgan", "Alex  Morgan", "水 🦀"] {
            assert_eq!(
                normalize_candidate(&format!("  {value}  ")).unwrap(),
                Some(value.to_owned())
            );
        }
    }

    #[test]
    fn grouping_is_source_ordered_safe_and_retains_independent_eligibility() {
        let context = Context::for_tests();
        context
            .publish(
                " Team ",
                Source::GithubOrganization {
                    login: "team".into(),
                    host: "github.com".into(),
                },
            )
            .unwrap();
        context
            .publish(
                "Only membership",
                Source::GithubOrganization {
                    login: "org".into(),
                    host: "github.com".into(),
                },
            )
            .unwrap();
        context.publish("Team", Source::AccountName).unwrap();
        context
            .publish(
                "Team",
                Source::Project {
                    file: ProjectFile::CargoToml,
                    index: 0,
                },
            )
            .unwrap();
        context
            .publish(
                "Team",
                Source::Project {
                    file: ProjectFile::PackageJson,
                    index: 0,
                },
            )
            .unwrap();
        context.publish("Team", Source::AccountName).unwrap();
        context
            .publish(
                "Unsafe hint",
                Source::GithubProfile {
                    login: "bad\nlogin".into(),
                    host: "github.com".into(),
                },
            )
            .unwrap();
        let choices = context.choices_for_tests().unwrap();
        assert_eq!(choices.len(), 2);
        assert_eq!(choices[0].name, "Team");
        assert_eq!(
            choices[0].hint,
            "package.json author, Cargo.toml package.authors, system account name, and GitHub organisation team"
        );
        assert!(choices[0].eligible);
        assert!(!choices[1].eligible);
        assert_eq!(choices[0].kind, AuthorProvenanceKind::Mixed);
        assert_eq!(choices[1].kind, AuthorProvenanceKind::Organisation);
    }

    #[test]
    fn source_hint_joining_handles_empty_single_pairs_and_oxford_lists() {
        let hints: Vec<_> = ["first", "second", "third", "fourth"]
            .into_iter()
            .map(String::from)
            .collect();
        for (count, expected) in [
            (0, ""),
            (1, "first"),
            (2, "first and second"),
            (3, "first, second, and third"),
            (4, "first, second, third, and fourth"),
        ] {
            assert_eq!(join_source_hints(&hints[..count]), expected);
        }
    }

    #[test]
    fn joining_keeps_source_text_order_exact_deduplication_and_eligibility() {
        let records = vec![
            Record {
                name: "Team".into(),
                source: Source::GithubOrganization {
                    login: "team".into(),
                    host: "github.com".into(),
                },
                hint: "GitHub organisation team and friends".into(),
                contacts: Vec::new(),
                git_origin: None,
            },
            Record {
                name: "Team".into(),
                source: Source::AccountName,
                hint: "system account name".into(),
                contacts: Vec::new(),
                git_origin: None,
            },
            Record {
                name: "Team".into(),
                source: Source::AccountName,
                hint: "system account name".into(),
                contacts: Vec::new(),
                git_origin: None,
            },
            Record {
                name: "Team".into(),
                source: Source::Project {
                    file: ProjectFile::PackageJson,
                    index: 0,
                },
                hint: "package.json author".into(),
                contacts: Vec::new(),
                git_origin: None,
            },
        ];
        let grouped = choices(records);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].name, "Team");
        assert_eq!(
            grouped[0].hint,
            "package.json author, system account name, and GitHub organisation team and friends"
        );
        assert!(grouped[0].eligible);
    }

    #[test]
    fn provenance_describes_sources_even_when_identity_settings_name_a_company() {
        let sources = [
            (Source::GitAuthorEnvironment, AuthorProvenanceKind::Identity),
            (
                Source::Git {
                    key: GitKey::Author,
                    scope: GitScope::Command,
                    index: 0,
                },
                AuthorProvenanceKind::Identity,
            ),
            (
                Source::Git {
                    key: GitKey::User,
                    scope: GitScope::Local,
                    index: 0,
                },
                AuthorProvenanceKind::Identity,
            ),
            (
                Source::GithubProfile {
                    login: "company".into(),
                    host: "github.com".into(),
                },
                AuthorProvenanceKind::Identity,
            ),
            (Source::AccountName, AuthorProvenanceKind::Identity),
            (Source::AccountUsername, AuthorProvenanceKind::Identity),
            (
                Source::EnvironmentUsername {
                    variable: "USER",
                    index: 0,
                },
                AuthorProvenanceKind::Identity,
            ),
            (
                Source::GithubOrganization {
                    login: "person-like-login".into(),
                    host: "github.com".into(),
                },
                AuthorProvenanceKind::Organisation,
            ),
        ];
        for (source, expected) in sources.into_iter().chain(
            [
                ProjectFile::PackageJson,
                ProjectFile::CargoToml,
                ProjectFile::PyprojectToml,
            ]
            .into_iter()
            .map(|file| {
                (
                    Source::Project { file, index: 0 },
                    AuthorProvenanceKind::Unspecified,
                )
            }),
        ) {
            let context = Context::for_tests();
            for name in [" Team Incorporated ", "Alex Morgan", "👤 literal icon"] {
                assert!(context.publish(name, source.clone()).unwrap());
            }
            let grouped = context.choices_for_tests().unwrap();
            assert_eq!(grouped.len(), 3);
            for name in ["Team Incorporated", "Alex Morgan", "👤 literal icon"] {
                let choice = grouped.iter().find(|choice| choice.name == name).unwrap();
                assert_eq!(choice.name, name);
                assert_eq!(choice.kind, expected);
                assert_eq!(choice.hint, source.hint().unwrap());
            }
        }
    }

    #[test]
    fn project_evidence_neither_erases_nor_invents_known_provenance() {
        fn permutations(sources: &[Source]) -> Vec<Vec<Source>> {
            if sources.is_empty() {
                return vec![vec![]];
            }
            let mut result = Vec::new();
            for index in 0..sources.len() {
                let mut rest = sources.to_vec();
                let first = rest.remove(index);
                for mut tail in permutations(&rest) {
                    tail.insert(0, first.clone());
                    result.push(tail);
                }
            }
            result
        }
        let project = Source::Project {
            file: ProjectFile::PackageJson,
            index: 0,
        };
        let identity = Source::GitAuthorEnvironment;
        let organisation = Source::GithubOrganization {
            login: "team".into(),
            host: "github.com".into(),
        };
        for (sources, kind, eligible) in [
            (
                vec![project.clone()],
                AuthorProvenanceKind::Unspecified,
                true,
            ),
            (
                vec![project.clone(), identity.clone()],
                AuthorProvenanceKind::Identity,
                true,
            ),
            (
                vec![project.clone(), organisation.clone()],
                AuthorProvenanceKind::Organisation,
                true,
            ),
            (
                vec![identity.clone(), organisation.clone()],
                AuthorProvenanceKind::Mixed,
                true,
            ),
            (
                vec![organisation.clone()],
                AuthorProvenanceKind::Organisation,
                false,
            ),
            (
                vec![project, identity, organisation],
                AuthorProvenanceKind::Mixed,
                true,
            ),
        ] {
            let mut baseline = None;
            for ordered in permutations(&sources) {
                let context = Context::for_tests();
                // Repeat every source to exercise idempotence independently of
                // the order and exact-hint deduplication.
                for source in ordered.iter().chain(ordered.iter().rev()) {
                    context.publish("Same name", source.clone()).unwrap();
                }
                let grouped = context.choices_for_tests().unwrap();
                assert_eq!(grouped.len(), 1);
                assert_eq!(grouped[0].name, "Same name");
                assert_eq!(grouped[0].kind, kind);
                assert_eq!(grouped[0].eligible, eligible);
                assert_eq!(baseline.get_or_insert_with(|| grouped.clone()), &grouped);
            }
        }
    }

    #[test]
    fn rejected_source_evidence_does_not_change_provenance() {
        let context = Context::for_tests();
        context.publish("Team", Source::AccountName).unwrap();
        context
            .publish(
                "Team",
                Source::GithubOrganization {
                    login: "unsafe\nlogin".into(),
                    host: "github.com".into(),
                },
            )
            .unwrap();
        assert_eq!(
            context.choices_for_tests().unwrap()[0].kind,
            AuthorProvenanceKind::Identity
        );
    }

    #[test]
    fn provenance_merge_is_idempotent_commutative_and_associative() {
        let kinds = [
            AuthorProvenanceKind::Unspecified,
            AuthorProvenanceKind::Identity,
            AuthorProvenanceKind::Organisation,
            AuthorProvenanceKind::Mixed,
        ];
        for first in kinds {
            assert_eq!(first.merge(first), first);
            assert_eq!(first.merge(AuthorProvenanceKind::Unspecified), first);
            for second in kinds {
                assert_eq!(first.merge(second), second.merge(first));
                for third in kinds {
                    assert_eq!(
                        first.merge(second).merge(third),
                        first.merge(second.merge(third))
                    );
                }
            }
        }
        assert_eq!(
            AuthorProvenanceKind::Identity.merge(AuthorProvenanceKind::Organisation),
            AuthorProvenanceKind::Mixed
        );
    }

    #[test]
    fn hint_deduplication_does_not_discard_provenance_evidence() {
        let grouped = choices(
            [
                Source::AccountName,
                Source::GithubOrganization {
                    login: "team".into(),
                    host: "github.com".into(),
                },
            ]
            .into_iter()
            .map(|source| Record {
                name: "Team".into(),
                source,
                hint: "same display hint".into(),
                contacts: Vec::new(),
                git_origin: None,
            })
            .collect(),
        );
        assert_eq!(grouped[0].hint, "same display hint");
        assert_eq!(grouped[0].kind, AuthorProvenanceKind::Mixed);
    }
}
