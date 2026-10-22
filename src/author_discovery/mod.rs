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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub name: String,
    /// Complete safe source evidence, also used as the selector's raw hint identity.
    pub hint: String,
    /// Deduplicated source families for the human-facing picker.
    pub summary: String,
    pub eligible: bool,
    pub kind: AuthorProvenanceKind,
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
    // Only admitted records reach grouping: hint() has already checked unsafe
    // GitHub components. Keep display families separate from detailed evidence.
    fn summary(&self) -> String {
        match self {
            Self::Project { .. } => "project metadata".into(),
            Self::GitAuthorEnvironment | Self::Git { .. } => "Git".into(),
            Self::AccountName | Self::AccountUsername | Self::EnvironmentUsername { .. } => {
                "local account".into()
            }
            Self::GithubProfile { host, .. } | Self::GithubOrganization { host, .. } => {
                let family = if matches!(self, Self::GithubProfile { .. }) {
                    "GitHub"
                } else {
                    "GitHub organisation"
                };
                if host.eq_ignore_ascii_case("github.com") {
                    family.into()
                } else {
                    format!("{family} on {host}")
                }
            }
        }
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
    normalize_with(value, &|value| schema::validate_field("author", value))
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

#[derive(Clone)]
pub(super) struct Record {
    name: String,
    source: Source,
    hint: String,
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
    let mut summaries: Vec<Vec<String>> = Vec::new();
    for record in records {
        let eligible = !matches!(record.source, Source::GithubOrganization { .. });
        let index = *indices.entry(record.name.clone()).or_insert_with(|| {
            result.push(Choice {
                name: record.name,
                hint: String::new(),
                summary: String::new(),
                eligible: false,
                kind: AuthorProvenanceKind::Unspecified,
            });
            hints.push(Vec::new());
            summaries.push(Vec::new());
            result.len() - 1
        });
        result[index].eligible |= eligible;
        result[index].kind = result[index].kind.merge(record.source.provenance_kind());
        let summary = record.source.summary();
        if !summaries[index].contains(&summary) {
            summaries[index].push(summary);
        }
        if !hints[index].contains(&record.hint) {
            hints[index].push(record.hint);
        }
    }
    for ((choice, hints), summaries) in result.iter_mut().zip(hints).zip(summaries) {
        choice.hint = join_source_hints(&hints);
        choice.summary = summaries.join(", ");
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

    #[test]
    fn compact_summaries_deduplicate_families_without_losing_detailed_evidence() {
        let context = Context::for_tests();
        let sources = [
            Source::Project {
                file: ProjectFile::PackageJson,
                index: 0,
            },
            Source::Project {
                file: ProjectFile::CargoToml,
                index: 0,
            },
            Source::Project {
                file: ProjectFile::PyprojectToml,
                index: 0,
            },
            Source::GitAuthorEnvironment,
            Source::Git {
                key: GitKey::User,
                scope: GitScope::Global,
                index: 0,
            },
            Source::Git {
                key: GitKey::Author,
                scope: GitScope::Local,
                index: 0,
            },
            Source::GithubProfile {
                login: "alex".into(),
                host: "github.com".into(),
            },
            Source::AccountName,
            Source::AccountUsername,
            Source::EnvironmentUsername {
                variable: "USER",
                index: 0,
            },
            Source::GithubOrganization {
                login: "first-org".into(),
                host: "github.com".into(),
            },
            Source::GithubOrganization {
                login: "second-org".into(),
                host: "github.com".into(),
            },
        ];
        for source in sources.iter().rev().chain(sources.iter()) {
            context.publish(" Team ", source.clone()).unwrap();
        }
        let grouped = context.choices_for_tests().unwrap();
        assert_eq!(grouped.len(), 1);
        let choice = &grouped[0];
        assert_eq!(choice.name, "Team");
        assert_eq!(
            choice.summary,
            "project metadata, Git, GitHub, local account, GitHub organisation"
        );
        assert!(choice.eligible);
        assert_eq!(choice.kind, AuthorProvenanceKind::Mixed);
        for source in sources {
            let hint = source.hint().unwrap();
            assert_eq!(choice.hint.matches(&hint).count(), 1, "{}", choice.hint);
        }
    }

    #[test]
    fn common_identity_summary_stays_short_when_settings_share_a_name() {
        let context = Context::for_tests();
        for source in [
            Source::AccountName,
            Source::GithubProfile {
                login: "alex-morgan".into(),
                host: "github.com".into(),
            },
            Source::Git {
                key: GitKey::User,
                scope: GitScope::Global,
                index: 0,
            },
        ] {
            context.publish("Alex Morgan", source).unwrap();
        }
        let choices = context.choices_for_tests().unwrap();
        assert_eq!(choices[0].summary, "Git, GitHub, local account");
        assert_eq!(
            choices[0].hint,
            "Git global user.name, GitHub profile alex-morgan, and system account name"
        );
    }

    #[test]
    fn summaries_keep_enterprise_and_membership_distinctions_after_safety_checks() {
        for (host, suffix) in [("GITHUB.COM", ""), ("work.example", " on work.example")] {
            let context = Context::for_tests();
            for source in [
                Source::GithubProfile {
                    login: "alex".into(),
                    host: host.into(),
                },
                Source::GithubOrganization {
                    login: "first".into(),
                    host: host.into(),
                },
                Source::GithubOrganization {
                    login: "second".into(),
                    host: host.into(),
                },
                Source::GithubOrganization {
                    login: "unsafe\nlogin".into(),
                    host: host.into(),
                },
            ] {
                context.publish("Same name", source).unwrap();
            }
            let grouped = context.choices_for_tests().unwrap();
            assert_eq!(
                grouped[0].summary,
                format!("GitHub{suffix}, GitHub organisation{suffix}")
            );
            assert!(!grouped[0].hint.contains("unsafe"));
            assert_eq!(grouped[0].kind, AuthorProvenanceKind::Mixed);
        }
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
            },
            Record {
                name: "Team".into(),
                source: Source::AccountName,
                hint: "system account name".into(),
            },
            Record {
                name: "Team".into(),
                source: Source::AccountName,
                hint: "system account name".into(),
            },
            Record {
                name: "Team".into(),
                source: Source::Project {
                    file: ProjectFile::PackageJson,
                    index: 0,
                },
                hint: "package.json author".into(),
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
            })
            .collect(),
        );
        assert_eq!(grouped[0].hint, "same display hint");
        assert_eq!(grouped[0].kind, AuthorProvenanceKind::Mixed);
    }
}
