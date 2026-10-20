use std::ffi::{OsStr, OsString};
use std::process::Command;

use hydragen::layer_manifest::schema;
use serde_json::Value;

use super::{Context, Source, normalize_candidate, process, safe_component};

const QUERY: &str = "query($after: String) { viewer { login organizations(first: 100, after: $after) { nodes { name login } pageInfo { hasNextPage endCursor } } } }";

pub(super) fn collect(context: &Context) -> Result<(), schema::Error> {
    let Some(host) = selected_host(std::env::var_os("GH_HOST")) else {
        return Ok(());
    };
    collect_with(context, OsStr::new("gh"), &host, |_| {})
}

fn selected_host(value: Option<OsString>) -> Option<String> {
    match value {
        None => Some("github.com".into()),
        Some(host) if host.is_empty() => Some("github.com".into()),
        // An unusable explicit host must not silently send requests elsewhere.
        Some(host) => host.into_string().ok(),
    }
}

fn request(
    context: &Context,
    executable: &OsStr,
    host: &str,
    arguments: &[&str],
    configure: &impl Fn(&mut Command),
) -> Option<Value> {
    if !context.active() {
        return None;
    }
    let mut command = Command::new(executable);
    // Inherit gh's selected authentication. Never inspect credentials or ask gh
    // to enumerate/switch accounts. Explicit host prevents repository inference.
    command.args(["api", "--hostname", host]);
    command.args(arguments);
    command
        .env("GH_PROMPT_DISABLED", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    configure(&mut command);
    let output = process::run(command, context, None).ok()?;
    serde_json::from_slice(&output).ok()
}

fn usable_name(value: &Value) -> Result<Option<String>, schema::Error> {
    if let Some(name) = value.get("name").and_then(Value::as_str)
        && let Some(name) = normalize_candidate(name)?
    {
        return Ok(Some(name));
    }
    match value.get("login").and_then(Value::as_str) {
        Some(login) => normalize_candidate(login),
        None => Ok(None),
    }
}

fn collect_with(
    context: &Context,
    executable: &OsStr,
    host: &str,
    configure: impl Fn(&mut Command),
) -> Result<(), schema::Error> {
    if !safe_component(host) {
        return Ok(());
    }
    let Some(profile) = request(context, executable, host, &["user"], &configure) else {
        return Ok(());
    };
    let Some(login) = profile
        .get("login")
        .and_then(Value::as_str)
        .filter(|login| safe_component(login))
    else {
        return Ok(());
    };
    if let Some(name) = usable_name(&profile)?
        && !context.publish(
            &name,
            Source::GithubProfile {
                login: login.into(),
                host: host.into(),
            },
        )?
    {
        return Ok(());
    }
    let mut cursor: Option<String> = None;
    while context.active() {
        let mut arguments = vec!["graphql", "-f", "query", "-F", "after=null"];
        let query = format!("query={QUERY}");
        arguments[2] = &query;
        let after;
        if let Some(cursor) = &cursor {
            after = format!("after={cursor}");
            arguments[4] = &after;
            // Use raw -f for cursor strings; typed -F interprets numeric cursors.
            arguments[3] = "-f";
        }
        let Some(page) = request(context, executable, host, &arguments, &configure) else {
            break;
        };
        if page
            .get("errors")
            .is_some_and(|errors| !errors.as_array().is_some_and(Vec::is_empty))
        {
            break;
        }
        let Some(viewer) = page.pointer("/data/viewer") else {
            break;
        };
        if viewer.get("login").and_then(Value::as_str) != Some(login) {
            break;
        }
        let Some(organizations) = viewer.get("organizations") else {
            break;
        };
        let Some(nodes) = organizations.get("nodes").and_then(Value::as_array) else {
            break;
        };
        // Request and accept no more than 100 entries per page.
        if nodes.len() > 100 {
            break;
        }
        for organization in nodes {
            let Some(org_login) = organization
                .get("login")
                .and_then(Value::as_str)
                .filter(|login| safe_component(login))
            else {
                continue;
            };
            if let Some(name) = usable_name(organization)?
                && !context.publish(
                    &name,
                    Source::GithubOrganization {
                        login: org_login.into(),
                        host: host.into(),
                    },
                )?
            {
                return Ok(());
            }
        }
        let Some(info) = organizations.get("pageInfo") else {
            break;
        };
        if info.get("hasNextPage").and_then(Value::as_bool) != Some(true) {
            break;
        }
        let Some(next) = info
            .get("endCursor")
            .and_then(Value::as_str)
            .filter(|cursor| !cursor.is_empty())
        else {
            break;
        };
        // A broken server repeating its cursor must not create an endless loop.
        if cursor.as_deref() == Some(next) {
            break;
        }
        cursor = Some(next.to_owned());
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn collect_for_tests(
    context: &Context,
    executable: &OsStr,
    host: &str,
    configure: impl Fn(&mut Command),
) -> Result<(), schema::Error> {
    collect_with(context, executable, host, configure)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    };
    use std::time::{Duration, Instant};

    fn fake(directory: &Path, script: &str) -> std::path::PathBuf {
        let path = directory.join("fake-gh");
        fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    #[test]
    fn fake_gh_explicit_host_selected_auth_fallback_paging_and_grouping() {
        let directory = tempfile::tempdir().unwrap();
        let log = directory.path().join("log");
        let gh = fake(
            directory.path(),
            r#"
printf '%s\n' "$*" >> "$LOG"
[ "$GH_PROMPT_DISABLED" = 1 ] || exit 9
[ "$SELECTED_TEST_CREDENTIAL" = inherited ] || exit 9
[ "$1 $2 $3" = 'api --hostname enterprise.example' ] || exit 9
case "$4" in
user) printf '%s' '{"login":"alex","name":"bad\nname","company":"Not an org"}';;
graphql)
case "$*" in
*after=next*) printf '%s' '{"data":{"viewer":{"login":"alex","organizations":{"nodes":[{"login":"a","name":"Alpha"},{"login":"team","name":"Team"}],"pageInfo":{"hasNextPage":false}}}}}';;
*) printf '%s' '{"data":{"viewer":{"login":"alex","organizations":{"nodes":[{"login":"z","name":"Zulu"},{"login":"fallback","name":"bad\u2028name"}],"pageInfo":{"hasNextPage":true,"endCursor":"next"}}}}}';;
esac;;
*) exit 9;;
esac
"#,
        );
        let context = Context::for_tests();
        context
            .publish(
                "Team",
                Source::Project {
                    file: super::super::ProjectFile::PackageJson,
                    index: 0,
                },
            )
            .unwrap();
        collect_with(&context, gh.as_os_str(), "enterprise.example", |command| {
            command
                .env_clear()
                .env("LOG", &log)
                .env("SELECTED_TEST_CREDENTIAL", "inherited")
                .env("GH_PROMPT_DISABLED", "1");
        })
        .unwrap();
        let choices = context.choices_for_tests().unwrap();
        assert_eq!(
            choices
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            ["Team", "alex", "Alpha", "Zulu", "fallback"]
        );
        assert!(choices[0].eligible);
        assert!(
            choices[0]
                .hint
                .contains("GitHub organisation team on enterprise.example")
        );
        assert_eq!(choices[1].hint, "GitHub profile alex on enterprise.example");
        assert!(!choices[2].eligible);
        let log = fs::read_to_string(log).unwrap();
        assert_eq!(log.lines().count(), 3);
        assert!(log.contains("organizations(first: 100"));
        assert!(!log.contains("orgs/"));
    }

    #[test]
    fn later_failure_or_viewer_change_retains_prior_complete_records() {
        for later in [
            "exit 4",
            "printf '%s' '{\"data\":{\"viewer\":{\"login\":\"other\",\"organizations\":{\"nodes\":[{\"login\":\"wrong\",\"name\":\"Wrong\"}],\"pageInfo\":{\"hasNextPage\":false}}}}}'",
        ] {
            let directory = tempfile::tempdir().unwrap();
            let gh = fake(
                directory.path(),
                &format!(
                    r#"
case "$4" in
user) printf '%s' '{{"login":"alex","name":"Alex"}}';;
*) case "$*" in
*after=next*) {later};;
*) printf '%s' '{{"data":{{"viewer":{{"login":"alex","organizations":{{"nodes":[{{"login":"team","name":"Team"}}],"pageInfo":{{"hasNextPage":true,"endCursor":"next"}}}}}}}}}}';;
esac;; esac
"#
                ),
            );
            let context = Context::for_tests();
            collect_with(&context, gh.as_os_str(), "github.com", |_| {}).unwrap();
            assert_eq!(
                context
                    .choices_for_tests()
                    .unwrap()
                    .iter()
                    .map(|choice| choice.name.as_str())
                    .collect::<Vec<_>>(),
                ["Alex", "Team"]
            );
        }
    }

    #[test]
    fn failed_profile_skips_organizations_and_unsafe_hints_never_launch() {
        let directory = tempfile::tempdir().unwrap();
        let log = directory.path().join("log");
        let gh = fake(
            directory.path(),
            "printf '%s\\n' \"$*\" >> \"$LOG\"; exit 1",
        );
        let context = Context::for_tests();
        collect_with(&context, gh.as_os_str(), "github.com", |command| {
            command.env("LOG", &log);
        })
        .unwrap();
        assert_eq!(fs::read_to_string(&log).unwrap().lines().count(), 1);
        assert!(context.choices_for_tests().unwrap().is_empty());
        collect_with(&context, gh.as_os_str(), "bad\nhost", |command| {
            command.env("LOG", &log);
        })
        .unwrap();
        assert_eq!(fs::read_to_string(&log).unwrap().lines().count(), 1);
    }

    #[test]
    fn effective_deadline_stops_a_stalled_gh_without_starting_next_page() {
        let directory = tempfile::tempdir().unwrap();
        let log = directory.path().join("started");
        let gh = fake(
            directory.path(),
            "printf started > \"$LOG\"; exec /bin/sleep 30",
        );
        let base = Instant::now();
        let elapsed = Arc::new(AtomicU64::new(0));
        let clock = elapsed.clone();
        let context = Context::with_clock_for_tests(Arc::new(move || {
            base + Duration::from_millis(clock.load(Ordering::SeqCst))
        }));
        let owned = context.clone();
        let (done, completion) = mpsc::channel();
        let process_log = log.clone();
        let runner = std::thread::spawn(move || {
            done.send(collect_with(
                &owned,
                gh.as_os_str(),
                "github.com",
                |command| {
                    command.env("LOG", &process_log);
                },
            ))
            .unwrap();
        });
        let start = Instant::now();
        while !log.exists() && start.elapsed() < Duration::from_secs(2) {
            std::thread::yield_now();
        }
        assert!(log.exists());
        // Name may take over 1.5 seconds; that must not kill GitHub early.
        elapsed.store(3000, Ordering::SeqCst);
        assert!(context.active());
        assert!(completion.try_recv().is_err());
        elapsed.store(15000, Ordering::SeqCst);
        completion
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        while !runner.is_finished() {
            std::thread::yield_now();
        }
        runner.join().unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn host_selection_is_explicit_and_does_not_scan_saved_hosts() {
        assert_eq!(selected_host(None).as_deref(), Some("github.com"));
        assert_eq!(
            selected_host(Some(OsString::new())).as_deref(),
            Some("github.com")
        );
        assert_eq!(
            selected_host(Some("enterprise.example".into())).as_deref(),
            Some("enterprise.example")
        );
        use std::os::unix::ffi::OsStringExt;
        assert_eq!(selected_host(Some(OsString::from_vec(vec![0xff]))), None);
    }
}
