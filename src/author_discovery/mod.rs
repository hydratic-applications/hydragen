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
    pub hint: String,
    pub eligible: bool,
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
            Self::GithubProfile { login, host } => {
                if !safe_component(login) || !safe_component(host) {
                    return None;
                }
                format!("GitHub profile {login} on {host}")
            }
            Self::GithubOrganization { login, host } => {
                if !safe_component(login) || !safe_component(host) {
                    return None;
                }
                format!("GitHub organisation {login} on {host}")
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
    for record in records {
        let eligible = !matches!(record.source, Source::GithubOrganization { .. });
        let index = *indices.entry(record.name.clone()).or_insert_with(|| {
            result.push(Choice {
                name: record.name,
                hint: String::new(),
                eligible: false,
            });
            hints.push(Vec::new());
            result.len() - 1
        });
        result[index].eligible |= eligible;
        if !hints[index].contains(&record.hint) {
            hints[index].push(record.hint);
        }
    }
    for (choice, hints) in result.iter_mut().zip(hints) {
        choice.hint = hints.join(", ");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "package.json author, Cargo.toml package.authors, system account name, GitHub organisation team on github.com"
        );
        assert!(choices[0].eligible);
        assert!(!choices[1].eligible);
    }
}
