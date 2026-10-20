use std::ffi::{OsStr, OsString};
use std::process::Command;

use hydragen::layer_manifest::schema;
use serde_json::Value;

use super::{
    ContactField, ContactInput, Context, Source, normalize_candidate, normalize_with, process,
    safe_component,
};

const QUERY: &str = "query($after: String) { viewer { login email websiteUrl url organizations(first: 100, after: $after) { nodes { name login email websiteUrl url } pageInfo { hasNextPage endCursor } } } }";

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

/// This is a construction-safety boundary, not a second author-URL validator.
/// Only authority/path components that cannot introduce userinfo, a different
/// host, path traversal, escaping, query, or fragment may be concatenated.
fn fallback_profile_url(host: &str, login: &str) -> Option<String> {
    if login.is_empty()
        || !login
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return None;
    }
    let (hostname, port) = if let Some(rest) = host.strip_prefix('[') {
        let (address, suffix) = rest.split_once(']')?;
        address.parse::<std::net::Ipv6Addr>().ok()?;
        (
            None,
            if suffix.is_empty() {
                None
            } else {
                Some(suffix.strip_prefix(':')?)
            },
        )
    } else {
        let (hostname, port) = host
            .split_once(':')
            .map_or((host, None), |(name, port)| (name, Some(port)));
        (Some(hostname), port)
    };
    if let Some(hostname) = hostname
        && (hostname.is_empty()
            || !hostname.split('.').all(|label| {
                !label.is_empty()
                    && label.starts_with(|c: char| c.is_ascii_alphanumeric())
                    && label.ends_with(|c: char| c.is_ascii_alphanumeric())
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            }))
    {
        return None;
    }
    if let Some(port) = port
        && (port.is_empty()
            || !port.bytes().all(|byte| byte.is_ascii_digit())
            || port.parse::<u16>().ok()? == 0)
    {
        return None;
    }
    Some(format!("https://{host}/{login}"))
}

fn contacts(
    value: &Value,
    website_key: &str,
    profile_key: &str,
    fallback: Option<String>,
    public_email: bool,
) -> Result<Vec<(ContactField, String)>, schema::Error> {
    let mut contacts = Vec::new();
    if public_email && let Some(email) = value.get("email").and_then(Value::as_str) {
        contacts.push((ContactField::Email, email.to_owned()));
    }
    if let Some(website) = value.get(website_key).and_then(Value::as_str) {
        contacts.push((ContactField::Website, website.to_owned()));
    }
    // REST `url` is an API resource, never a profile contact. The caller supplies
    // `html_url` for REST and the documented profile `url` for GraphQL.
    let profile = value
        .get(profile_key)
        .and_then(Value::as_str)
        .map(|value| normalize_with(value, &|value| schema::validate_field("author.url", value)))
        .transpose()?
        .flatten()
        .or(fallback);
    if let Some(profile) = profile {
        contacts.push((ContactField::ProfileUrl, profile));
    }
    Ok(contacts)
}

fn contact_inputs(contacts: &[(ContactField, String)]) -> Vec<ContactInput<'_>> {
    contacts
        .iter()
        .map(|(field, value)| ContactInput {
            field: *field,
            value,
        })
        .collect()
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
    let profile_contacts = contacts(
        &profile,
        "blog",
        "html_url",
        fallback_profile_url(host, login),
        false,
    )?;
    if let Some(name) = usable_name(&profile)?
        && !context.publish_contacts(
            &name,
            Source::GithubProfile {
                login: login.into(),
                host: host.into(),
            },
            &contact_inputs(&profile_contacts),
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
        let viewer_contacts = contacts(viewer, "websiteUrl", "url", None, true)?;
        if !context.enrich_github_profile(host, login, &contact_inputs(&viewer_contacts))? {
            return Ok(());
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
            let org_contacts = contacts(
                organization,
                "websiteUrl",
                "url",
                fallback_profile_url(host, org_login),
                true,
            )?;
            if let Some(name) = usable_name(organization)?
                && !context.publish_contacts(
                    &name,
                    Source::GithubOrganization {
                        login: org_login.into(),
                        host: host.into(),
                    },
                    &contact_inputs(&org_contacts),
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

    fn collect_responses(profile: Value, page: Value) -> (Vec<super::super::Choice>, String) {
        collect_responses_on_host(profile, page, "enterprise.example")
    }

    fn collect_responses_on_host(
        profile: Value,
        page: Value,
        host: &str,
    ) -> (Vec<super::super::Choice>, String) {
        let directory = tempfile::tempdir().unwrap();
        let profile_file = directory.path().join("profile.json");
        let page_file = directory.path().join("page.json");
        let log = directory.path().join("requests");
        fs::write(&profile_file, profile.to_string()).unwrap();
        fs::write(&page_file, page.to_string()).unwrap();
        let gh = fake(
            directory.path(),
            r#"
printf '%s\n' "$*" >> "$LOG"
[ "$1" = api ] && [ "$2" = --hostname ] && [ "$3" = "$HOST" ] || exit 9
case "$4" in
user) /bin/cat "$PROFILE";;
graphql) /bin/cat "$PAGE";;
*) exit 9;;
esac
"#,
        );
        let context = Context::for_tests();
        collect_with(&context, gh.as_os_str(), host, |command| {
            command
                .env_clear()
                .env("LOG", &log)
                .env("PROFILE", &profile_file)
                .env("PAGE", &page_file)
                .env("HOST", host);
        })
        .unwrap();
        (
            context.choices_for_tests().unwrap(),
            fs::read_to_string(log).unwrap(),
        )
    }

    #[test]
    fn public_contacts_reuse_only_existing_requests_and_preserve_website_profile_alternatives() {
        use serde_json::json;
        let (choices, log) = collect_responses(
            json!({"login":"alex", "name":"Alex", "blog":" https://rest-website.invalid ", "html_url":"https://enterprise.example/alex", "url":"https://api.invalid/users/alex", "email":"private-rest-email@example.invalid"}),
            json!({"data":{"viewer":{"login":"alex", "email":" public@example.invalid ", "websiteUrl":"https://graphql-website.invalid", "url":"https://enterprise.example/alex", "organizations":{"nodes":[
                {"login":"team", "name":"Alex", "email":"org@example.invalid", "websiteUrl":"https://team.invalid", "url":"https://enterprise.example/team"},
                {"login":"other", "name":"Other", "email":null, "websiteUrl":null, "url":null}
            ], "pageInfo":{"hasNextPage":false}}}}}),
        );
        assert_eq!(choices.len(), 2);
        let alex = &choices[0];
        assert_eq!(alex.name, "Alex");
        assert_eq!(alex.records.len(), 2);
        assert_eq!(alex.kind, super::super::AuthorProvenanceKind::Mixed);
        assert_eq!(
            alex.emails
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>(),
            ["public@example.invalid", "org@example.invalid"]
        );
        assert_eq!(
            alex.emails[0].kind,
            super::super::AuthorProvenanceKind::Identity
        );
        assert_eq!(
            alex.emails[1].kind,
            super::super::AuthorProvenanceKind::Organisation
        );
        assert_eq!(
            alex.emails[0].hint,
            "GitHub profile alex on enterprise.example email"
        );
        assert_eq!(
            alex.urls
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>(),
            [
                "https://rest-website.invalid",
                "https://graphql-website.invalid",
                "https://enterprise.example/alex",
                "https://team.invalid",
                "https://enterprise.example/team"
            ]
        );
        assert_eq!(choices[1].urls[0].value, "https://enterprise.example/other");
        assert!(choices[1].emails.is_empty());
        assert_eq!(log.lines().count(), 2);
        assert!(log.contains("viewer { login email websiteUrl url"));
        assert!(log.contains("nodes { name login email websiteUrl url }"));
        for forbidden in [
            "user/emails",
            "orgs/",
            "auth ",
            "private-rest-email",
            "rest-website.invalid",
        ] {
            assert!(!log.contains(forbidden), "unexpected request {forbidden}");
        }
        assert!(!format!("{choices:?}").contains("private-rest-email"));
        assert!(!format!("{choices:?}").contains("api.invalid"));
    }

    #[test]
    fn null_wrong_type_invalid_and_scheme_less_contacts_do_not_erase_names_or_other_fields() {
        use serde_json::json;
        for email in [
            Value::Null,
            json!(42),
            json!("bad"),
            json!("bad\n@example.invalid"),
        ] {
            let (choices, log) = collect_responses(
                json!({"login":"alex", "name":"bad\nname", "blog":"scheme-less.example", "html_url":"relative/path", "url":"https://api.invalid/users/alex"}),
                json!({"data":{"viewer":{"login":"alex", "email":email, "websiteUrl":42, "url":null, "organizations":{"nodes":[
                    {"login":"valid", "name":null, "email":"valid@example.invalid", "websiteUrl":"scheme-less.example", "url":"https://enterprise.example/valid"},
                    {"login":"website", "name":"Website", "email":false, "websiteUrl":"https://website.invalid", "url":false},
                    {"login":"bad\nlogin", "name":"Unsafe", "email":"unsafe@example.invalid"}
                ], "pageInfo":{"hasNextPage":false}}}}}),
            );
            assert_eq!(
                choices.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
                ["alex", "Website", "valid"]
            );
            assert!(choices[0].emails.is_empty());
            assert_eq!(choices[0].urls[0].value, "https://enterprise.example/alex");
            assert_eq!(
                choices[1]
                    .urls
                    .iter()
                    .map(|c| c.value.as_str())
                    .collect::<Vec<_>>(),
                [
                    "https://website.invalid",
                    "https://enterprise.example/website"
                ]
            );
            assert_eq!(choices[2].emails[0].value, "valid@example.invalid");
            assert_eq!(choices[2].urls.len(), 1);
            assert_eq!(log.lines().count(), 2);
        }
    }

    #[test]
    fn mismatched_viewer_or_graphql_errors_cannot_attach_contacts_or_memberships() {
        use serde_json::json;
        for page in [
            json!({"data":{"viewer":{"login":"other", "email":"wrong@example.invalid", "websiteUrl":"https://wrong.invalid", "organizations":{"nodes":[{"login":"wrong", "name":"Wrong"}]}}}}),
            json!({"errors":[{"message":"denied"}], "data":{"viewer":{"login":"alex", "email":"error@example.invalid"}}}),
            json!({"data":{"viewer":null}}),
        ] {
            let (choices, log) = collect_responses(
                json!({"login":"alex", "name":"Alex", "blog":"https://early.invalid", "html_url":"https://enterprise.example/alex"}),
                page,
            );
            assert_eq!(choices.len(), 1);
            assert!(choices[0].emails.is_empty());
            assert_eq!(
                choices[0]
                    .urls
                    .iter()
                    .map(|c| c.value.as_str())
                    .collect::<Vec<_>>(),
                ["https://early.invalid", "https://enterprise.example/alex"]
            );
            assert_eq!(log.lines().count(), 2);
        }
    }

    #[test]
    fn fallback_urls_accept_only_safe_authority_and_single_path_components() {
        for host in [
            "github.com",
            "enterprise.example:8443",
            "127.0.0.1",
            "[2001:db8::1]",
            "[::1]:8443",
        ] {
            assert_eq!(
                fallback_profile_url(host, "alex-team_1"),
                Some(format!("https://{host}/alex-team_1"))
            );
        }
        for host in [
            "",
            "evil@example.invalid",
            "github.com@evil.invalid",
            "github.com/path",
            "github.com?query",
            "github.com#fragment",
            "https://github.com",
            "github.com\\@evil.invalid",
            "github.com%2f.evil.invalid",
            "github.com:bad",
            "github.com:0",
            "github.com:65536",
            "github.com:",
            "github..com",
            ".github.com",
            "github.com.",
            " github.com",
            "github.com\n",
            "[::1]@evil.invalid",
            "[::1]:443/path",
            "::1",
        ] {
            assert_eq!(
                fallback_profile_url(host, "alex"),
                None,
                "unsafe authority {host:?}"
            );
        }
        for login in [
            "",
            "../evil",
            ".",
            "..",
            "//evil.invalid",
            "alex/path",
            "alex?query",
            "alex#fragment",
            "alex@evil.invalid",
            "alex%2f..",
            "alex\\path",
            "alex\n",
            "alex name",
            "水",
        ] {
            assert_eq!(
                fallback_profile_url("github.com", login),
                None,
                "unsafe path {login:?}"
            );
        }
    }

    #[test]
    fn unsafe_url_components_omit_only_constructed_contacts_not_display_safe_names() {
        use serde_json::json;
        for (host, login) in [
            ("github.com@evil.invalid", "alex"),
            ("github.com/path", "alex"),
            ("enterprise.example", "../evil"),
            ("enterprise.example", "alex?query"),
            ("enterprise.example", "alex%2f.."),
        ] {
            let (choices, log) = collect_responses_on_host(
                json!({"login":login, "name":"Kept Name", "url":"https://api.invalid/users/alex"}),
                json!({"data":{"viewer":{"login":login, "email":"public@example.invalid", "organizations":{"nodes":[], "pageInfo":{"hasNextPage":false}}}}}),
                host,
            );
            assert_eq!(choices.len(), 1);
            assert_eq!(choices[0].name, "Kept Name");
            assert!(choices[0].urls.is_empty(), "host={host}, login={login}");
            assert_eq!(choices[0].emails[0].value, "public@example.invalid");
            assert_eq!(log.lines().count(), 2);
        }
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
            "printf '%s' '{\"errors\":[{\"message\":\"later denied\"}]}'",
        ] {
            let directory = tempfile::tempdir().unwrap();
            let gh = fake(
                directory.path(),
                &format!(
                    r#"
case "$4" in
user) printf '%s' '{{"login":"alex","name":"Alex","blog":"https://early.invalid","html_url":"https://github.com/alex"}}';;
*) case "$*" in
*after=next*) {later};;
*) printf '%s' '{{"data":{{"viewer":{{"login":"alex","email":"public@example.invalid","organizations":{{"nodes":[{{"login":"team","name":"Team","email":"team@example.invalid","websiteUrl":"https://team.invalid","url":"https://github.com/team"}}],"pageInfo":{{"hasNextPage":true,"endCursor":"next"}}}}}}}}}}';;
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
            let choices = context.choices_for_tests().unwrap();
            assert_eq!(choices[0].emails[0].value, "public@example.invalid");
            assert_eq!(
                choices[0]
                    .urls
                    .iter()
                    .map(|c| c.value.as_str())
                    .collect::<Vec<_>>(),
                ["https://early.invalid", "https://github.com/alex"]
            );
            assert_eq!(choices[1].emails[0].value, "team@example.invalid");
            assert_eq!(
                choices[1]
                    .urls
                    .iter()
                    .map(|c| c.value.as_str())
                    .collect::<Vec<_>>(),
                ["https://team.invalid", "https://github.com/team"]
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
