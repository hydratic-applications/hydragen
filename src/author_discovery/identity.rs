use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use hydragen::layer_manifest::schema;

use super::{
    ContactField, ContactInput, Context, GitKey, GitScope, Source, normalize_candidate, process,
};

pub(super) fn collect(directory: &Path, context: &Context) -> Result<(), schema::Error> {
    collect_with(
        context,
        |context| {
            let mut command = Command::new("git");
            command.current_dir(directory);
            collect_git(command, context)
        },
        || whoami::realname().ok(),
        || whoami::username().ok(),
        |variable| std::env::var(variable).ok(),
    )
}

fn collect_with(
    context: &Context,
    git: impl FnOnce(&Context) -> Result<bool, schema::Error>,
    realname: impl FnOnce() -> Option<String>,
    username: impl FnOnce() -> Option<String>,
    environment: impl Fn(&'static str) -> Option<String>,
) -> Result<(), schema::Error> {
    if !context.active() {
        return Ok(());
    }
    if let Some(name) = environment("GIT_AUTHOR_NAME") {
        let email = environment("GIT_AUTHOR_EMAIL");
        let contacts: Vec<_> = email
            .as_deref()
            .map(|value| ContactInput {
                field: ContactField::Email,
                value,
            })
            .into_iter()
            .collect();
        if !context.publish_contacts(&name, Source::GitAuthorEnvironment, &contacts)? {
            return Ok(());
        }
    }
    if !git(context)? {
        return Ok(());
    }
    collect_account(context, realname, username, environment)
}

fn collect_git(mut command: Command, context: &Context) -> Result<bool, schema::Error> {
    // The Git source guard starts here, not while project files were being read.
    let deadline = context.now() + Duration::from_millis(500);
    git_arguments(&mut command);
    command.env("GIT_TERMINAL_PROMPT", "0");
    match process::run(command, context, Some(deadline)) {
        Ok(output) => publish_git(&output, context)?,
        Err(process::Failure::Abandoned) => return Ok(false),
        Err(process::Failure::Unavailable) => {}
    }
    Ok(context.active())
}

fn git_arguments(command: &mut Command) {
    command.args([
        "config",
        "--null",
        "--show-scope",
        "--show-origin",
        "--includes",
        "--get-regexp",
        "^(author|user)\\.(name|email)$",
    ]);
}

fn publish_git(output: &[u8], context: &Context) -> Result<(), schema::Error> {
    // NUL framing protects values containing newlines. Git emits each matching
    // scope\0origin\0key\nvalue\0; a bad value does not poison another record.
    struct Name {
        value: String,
        key: GitKey,
        scope: GitScope,
        origin: Vec<u8>,
        index: usize,
    }
    #[derive(Default)]
    struct Bucket {
        names: BTreeSet<String>,
        emails: Vec<String>,
        latest_name: Option<usize>,
    }
    let mut buckets: BTreeMap<(GitKey, GitScope, Vec<u8>), Bucket> = BTreeMap::new();
    let mut names = Vec::new();
    let mut fields = output.split(|byte| *byte == 0);
    let mut index = 0;
    while let (Some(scope), Some(origin), Some(record)) =
        (fields.next(), fields.next(), fields.next())
    {
        let definition = index;
        index += 1;
        let scope = match scope {
            b"command" => GitScope::Command,
            b"worktree" => GitScope::Worktree,
            b"local" => GitScope::Local,
            b"global" => GitScope::Global,
            b"system" => GitScope::System,
            _ => continue,
        };
        let Some(newline) = record.iter().position(|byte| *byte == b'\n') else {
            continue;
        };
        let (key, is_name) = match &record[..newline] {
            b"author.name" => (GitKey::Author, true),
            b"user.name" => (GitKey::User, true),
            b"author.email" => (GitKey::Author, false),
            b"user.email" => (GitKey::User, false),
            _ => continue,
        };
        let Ok(value) = std::str::from_utf8(&record[newline + 1..]) else {
            continue;
        };
        if !context.active() {
            break;
        }
        let bucket = buckets.entry((key, scope, origin.to_vec())).or_default();
        if is_name {
            if let Some(value) = normalize_candidate(value)? {
                bucket.names.insert(value.clone());
                bucket.latest_name = Some(definition);
                names.push(Name {
                    value,
                    key,
                    scope,
                    origin: origin.to_vec(),
                    index: definition,
                });
            }
        } else {
            bucket.emails.push(value.to_owned());
        }
    }
    for name in names {
        let bucket = &buckets[&(name.key, name.scope, name.origin.clone())];
        let contacts: Vec<_> = if bucket.names.len() == 1
            && !name.origin.is_empty()
            && bucket.latest_name == Some(name.index)
        {
            // Preserve Git's later-definition-first preference inside a bucket.
            // Attach once to its latest usable name declaration: repeating the
            // same name must not multiply the bucket's contact storage/work.
            bucket
                .emails
                .iter()
                .rev()
                .map(|value| ContactInput {
                    field: ContactField::Email,
                    value,
                })
                .collect()
        } else {
            Vec::new()
        };
        if !context.publish_git_contacts(
            &name.value,
            Source::Git {
                key: name.key,
                scope: name.scope,
                index: name.index,
            },
            &name.origin,
            &contacts,
        )? {
            break;
        }
    }
    Ok(())
}

fn collect_account(
    context: &Context,
    realname: impl FnOnce() -> Option<String>,
    username: impl FnOnce() -> Option<String>,
    environment: impl Fn(&'static str) -> Option<String>,
) -> Result<(), schema::Error> {
    if !context.active() {
        return Ok(());
    }
    if let Some(name) = realname()
        && !context.publish(&name, Source::AccountName)?
    {
        return Ok(());
    }
    if !context.active() {
        return Ok(());
    }
    if let Some(username) = username()
        && let Some(username) = normalize_candidate(&username)?
    {
        context.publish(&username, Source::AccountUsername)?;
        return Ok(());
    }
    if !context.active() {
        return Ok(());
    }
    #[cfg(unix)]
    let variables = ["LOGNAME", "USER"].as_slice();
    #[cfg(windows)]
    let variables = ["USERNAME"].as_slice();
    #[cfg(not(any(unix, windows)))]
    let variables: &[&'static str] = &[];
    for (index, variable) in variables.iter().enumerate() {
        if let Some(name) = environment(variable)
            && !context.publish(&name, Source::EnvironmentUsername { variable, index })?
        {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{self, Read, Seek};
    use std::process::{Child, ExitStatus, Output, Stdio};
    use std::time::Instant;

    // Configuration semantics must not depend on the production runner's 500ms
    // lookup or 100ms capture-cleanup budgets. Files avoid pipe/capture-thread
    // scheduling altogether; the test still bounds execution and reaps failures.
    fn fixture_output(mut command: Command, timeout: Duration) -> io::Result<Output> {
        let mut stdout = tempfile::tempfile()?;
        let mut stderr = tempfile::tempfile()?;
        let mut child = command
            .stdin(Stdio::null())
            .stdout(stdout.try_clone()?)
            .stderr(stderr.try_clone()?)
            .spawn()?;
        let status = wait_fixture_child(&mut child, timeout)?;
        stdout.rewind()?;
        stderr.rewind()?;
        let mut output = Output {
            status,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        stdout.read_to_end(&mut output.stdout)?;
        stderr.read_to_end(&mut output.stderr)?;
        Ok(output)
    }

    fn wait_fixture_child(child: &mut Child, timeout: Duration) -> io::Result<ExitStatus> {
        let deadline = Instant::now() + timeout;
        let result = (|| loop {
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Git fixture command exceeded its test execution allowance",
                ));
            }
            std::thread::sleep(Duration::from_millis(5));
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }

    fn git_fixture_output(command: Command) -> Vec<u8> {
        let output = fixture_output(command, Duration::from_secs(10)).unwrap();
        assert!(output.status.success(), "Git fixture failed: {output:?}");
        output.stdout
    }

    #[test]
    fn author_environment_is_collected_before_all_git_and_account_alternatives() {
        use std::cell::Cell;
        let context = Context::for_tests();
        context
            .publish(
                "Project team",
                Source::Project {
                    file: super::super::ProjectFile::PackageJson,
                    index: 0,
                },
            )
            .unwrap();
        let git_called = Cell::new(false);
        let realname_called = Cell::new(false);
        let username_called = Cell::new(false);
        let environment_reads = Cell::new(0);
        collect_with(
            &context,
            |context| {
                git_called.set(true);
                publish_git(b"local\0file:fixture\0author.name\nConfigured author\0local\0file:fixture\0user.name\nConfigured user\0local\0file:fixture\0user.name\n Alex \0", context)?;
                Ok(context.active())
            },
            || { realname_called.set(true); Some("Alex".into()) },
            || { username_called.set(true); Some("login".into()) },
            |variable| {
                assert!(matches!(variable, "GIT_AUTHOR_NAME" | "GIT_AUTHOR_EMAIL"), "usable account username must prevent username-environment reads");
                environment_reads.set(environment_reads.get() + 1);
                Some(if variable == "GIT_AUTHOR_NAME" { " Alex " } else { "alex@example.invalid" }.into())
            },
        ).unwrap();
        assert!(git_called.get() && realname_called.get() && username_called.get());
        assert_eq!(environment_reads.get(), 2);
        let choices = context.choices_for_tests().unwrap();
        assert_eq!(
            choices
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Project team",
                "Alex",
                "Configured author",
                "Configured user",
                "login"
            ]
        );
        assert_eq!(
            choices[1].hint,
            "GIT_AUTHOR_NAME, Git local user.name, and system account name"
        );
        assert_eq!(choices[2].hint, "Git local author.name");
        assert_eq!(choices[4].hint, "system account username");
        assert!(choices.iter().all(|choice| choice.eligible));
        assert_eq!(choices[1].emails[0].value, "alex@example.invalid");
        assert_eq!(choices[1].emails[0].hint, "GIT_AUTHOR_EMAIL");
    }

    #[test]
    fn usable_account_username_never_consults_environment() {
        let context = Context::for_tests();
        context
            .publish("Git", Source::GitAuthorEnvironment)
            .unwrap();
        collect_account(
            &context,
            || Some("Full name".into()),
            || Some("login".into()),
            |_| panic!("environment must not be read"),
        )
        .unwrap();
        assert_eq!(
            context
                .choices_for_tests()
                .unwrap()
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            ["Git", "Full name", "login"]
        );
    }

    #[test]
    fn invalid_username_uses_ordered_environment_fallback_even_with_realname() {
        let context = Context::for_tests();
        collect_account(
            &context,
            || Some("Full name".into()),
            || Some("bad\nlogin".into()),
            |variable| Some(variable.to_owned()),
        )
        .unwrap();
        let names: Vec<_> = context
            .choices_for_tests()
            .unwrap()
            .into_iter()
            .map(|choice| choice.name)
            .collect();
        #[cfg(unix)]
        assert_eq!(names, ["Full name", "LOGNAME", "USER"]);
        #[cfg(windows)]
        assert_eq!(names, ["Full name", "USERNAME"]);
    }

    #[test]
    fn missing_account_never_manufactures_a_name() {
        let context = Context::for_tests();
        collect_account(&context, || None, || None, |_| None).unwrap();
        assert!(context.choices_for_tests().unwrap().is_empty());
    }

    #[test]
    fn nul_records_skip_bad_values_independently_and_reverse_definitions() {
        let context = Context::for_tests();
        publish_git(b"global\0file:/secret/path\0user.name\nold\0global\0file:x\0user.name\nnew\0local\0file:x\0author.name\nbad\nname\0local\0file:x\0author.name\nwork\0command\0command line:\0user.name\n\xff\0", &context).unwrap();
        let choices = context.choices_for_tests().unwrap();
        assert_eq!(
            choices
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            ["work", "new", "old"]
        );
        assert_eq!(choices[1].hint, "Git global user.name");
    }

    #[test]
    fn git_contacts_require_one_distinct_usable_name_in_the_same_origin_scope_and_family() {
        let context = Context::for_tests();
        let mut output = Vec::new();
        for (scope, origin, key, value) in [
            ("global", "file:/private/global", "user.name", "Shared"),
            (
                "global",
                "file:/private/global",
                "user.email",
                "global@example.invalid",
            ),
            ("local", "file:/private/local", "user.name", " Shared "),
            ("local", "file:/private/local", "user.name", "Shared"),
            ("local", "file:/private/local", "user.name", "bad\nname"),
            (
                "local",
                "file:/private/local",
                "user.email",
                "older@example.invalid",
            ),
            (
                "local",
                "file:/private/local",
                "user.email",
                "latest@example.invalid",
            ),
            ("local", "file:/private/local", "user.email", "not-an-email"),
            (
                "local",
                "file:/private/ambiguous",
                "user.name",
                "Ambiguous A",
            ),
            (
                "local",
                "file:/private/ambiguous",
                "user.name",
                "Ambiguous B",
            ),
            (
                "local",
                "file:/private/ambiguous",
                "user.email",
                "ambiguous@example.invalid",
            ),
            (
                "local",
                "file:/private/split-name",
                "user.name",
                "Split origin",
            ),
            (
                "local",
                "file:/private/split-email",
                "user.email",
                "split@example.invalid",
            ),
            ("global", "file:/private/scope", "user.name", "Split scope"),
            (
                "local",
                "file:/private/scope",
                "user.email",
                "scope@example.invalid",
            ),
            (
                "local",
                "file:/private/family",
                "author.name",
                "Split family",
            ),
            (
                "local",
                "file:/private/family",
                "user.email",
                "family@example.invalid",
            ),
            (
                "local",
                "file:/private/author",
                "author.name",
                "Author identity",
            ),
            (
                "local",
                "file:/private/author",
                "author.email",
                "author@example.invalid",
            ),
            ("local", "", "user.name", "Missing origin"),
            ("local", "", "user.email", "missing-origin@example.invalid"),
        ] {
            output.extend_from_slice(format!("{scope}\0{origin}\0{key}\n{value}\0").as_bytes());
        }
        publish_git(&output, &context).unwrap();
        let choices = context.choices_for_tests().unwrap();
        let shared = choices.iter().find(|c| c.name == "Shared").unwrap();
        assert_eq!(
            shared
                .emails
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>(),
            [
                "latest@example.invalid",
                "older@example.invalid",
                "global@example.invalid"
            ]
        );
        assert_eq!(shared.emails[0].hint, "Git local user.email");
        assert!(
            shared
                .records
                .iter()
                .all(|record| record.git_origin.is_some())
        );
        assert!(choices.iter().all(|c| !c.hint.contains("/private/")
            && c.emails.iter().all(|e| !e.hint.contains("/private/"))));
        for name in [
            "Ambiguous A",
            "Ambiguous B",
            "Split origin",
            "Split scope",
            "Split family",
            "Missing origin",
        ] {
            assert!(
                choices
                    .iter()
                    .find(|c| c.name == name)
                    .unwrap()
                    .emails
                    .is_empty(),
                "{name}"
            );
        }
        assert_eq!(
            choices
                .iter()
                .find(|c| c.name == "Author identity")
                .unwrap()
                .emails[0]
                .value,
            "author@example.invalid"
        );
    }

    #[test]
    fn environment_contacts_never_pair_with_git_account_or_unrelated_variables() {
        use std::cell::RefCell;
        for (name, email) in [
            (None, Some("email-only@example.invalid")),
            (Some("Same"), None),
            (Some("Same"), Some("invalid")),
            (Some("Same"), Some(" env@example.invalid ")),
            (Some("bad\nname"), Some("orphan@example.invalid")),
        ] {
            let context = Context::for_tests();
            let reads = RefCell::new(Vec::new());
            collect_with(&context, |context| {
                publish_git(b"local\0file:fixture\0user.name\nSame\0local\0file:fixture\0user.email\ngit@example.invalid\0", context)?;
                Ok(true)
            }, || Some("Account".into()), || Some("account-login".into()), |variable| {
                reads.borrow_mut().push(variable);
                match variable {
                    "GIT_AUTHOR_NAME" => name.map(str::to_owned),
                    "GIT_AUTHOR_EMAIL" => email.map(str::to_owned),
                    _ => panic!("unexpected identity variable {variable}"),
                }
            }).unwrap();
            assert_eq!(
                &*reads.borrow(),
                if name.is_some() {
                    &["GIT_AUTHOR_NAME", "GIT_AUTHOR_EMAIL"][..]
                } else {
                    &["GIT_AUTHOR_NAME"][..]
                }
            );
            let choices = context.choices_for_tests().unwrap();
            let shared = choices.iter().find(|c| c.name == "Same").unwrap();
            let emails: Vec<_> = shared.emails.iter().map(|c| c.value.as_str()).collect();
            assert_eq!(
                emails,
                if email == Some(" env@example.invalid ") {
                    vec!["env@example.invalid", "git@example.invalid"]
                } else {
                    vec!["git@example.invalid"]
                }
            );
            assert!(
                choices
                    .iter()
                    .find(|c| c.name == "Account")
                    .unwrap()
                    .emails
                    .is_empty()
            );
        }
    }

    fn isolated_git(directory: &Path, home: &Path, global: &Path) -> Command {
        let mut command = Command::new("git");
        command
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", home)
            .env("XDG_CONFIG_HOME", home.join("xdg"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", global)
            .current_dir(directory);
        command
    }

    #[test]
    fn real_git_honours_scopes_includes_conditions_worktrees_and_command_values() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir(&repo).unwrap();
        let repo = repo.canonicalize().unwrap();
        let global = root.path().join("global");
        let include = root.path().join("included");
        let conditional = root.path().join("conditional");
        fs::write(
            &include,
            "[user]\n name = Included\n email = included@example.invalid\n",
        )
        .unwrap();
        fs::write(
            &conditional,
            "[author]\n name = Conditional\n email = conditional@example.invalid\n",
        )
        .unwrap();
        fs::write(&global, format!("[user]\n name = Global old\n name = Global new\n email = ambiguous@example.invalid\n[include]\n path = {}\n[includeIf \"gitdir:{}\"]\n path = {}\n[includeIf \"gitdir:/never/hydragen/\"]\n path = /nonexistent/inactive\n", include.display(), repo.join(".git").display(), conditional.display())).unwrap();
        let mut command = isolated_git(&repo, root.path(), &global);
        command.args(["init", "--quiet"]);
        git_fixture_output(command);
        fs::write(repo.join(".git/config"), "[core]\n repositoryformatversion = 0\n[extensions]\n worktreeConfig = true\n[user]\n name = Local\n email = local@example.invalid\n[author]\n name = Local author\n email = author@example.invalid\n").unwrap();
        fs::write(
            repo.join(".git/config.worktree"),
            "[user]\n name = Worktree\n email = worktree@example.invalid\n",
        )
        .unwrap();
        let mut command = isolated_git(&repo, root.path(), &global);
        command
            .env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", "user.name")
            .env("GIT_CONFIG_VALUE_0", "Command")
            .env("GIT_CONFIG_KEY_1", "user.email")
            .env("GIT_CONFIG_VALUE_1", "command@example.invalid");
        git_arguments(&mut command);
        let output = git_fixture_output(command);
        let context = Context::for_tests();
        publish_git(&output, &context).unwrap();
        let choices = context.choices_for_tests().unwrap();
        assert_eq!(
            choices
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Local author",
                "Conditional",
                "Command",
                "Worktree",
                "Local",
                "Included",
                "Global new",
                "Global old"
            ]
        );
        assert_eq!(choices[2].hint, "Git command user.name");
        assert_eq!(choices[3].hint, "Git worktree user.name");
        for (index, expected) in [
            "author@example.invalid",
            "conditional@example.invalid",
            "command@example.invalid",
            "worktree@example.invalid",
            "local@example.invalid",
            "included@example.invalid",
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(choices[index].emails[0].value, expected);
        }
        assert!(choices[6].emails.is_empty() && choices[7].emails.is_empty());
        let mut command = isolated_git(root.path(), root.path(), &global);
        git_arguments(&mut command);
        let output = git_fixture_output(command);
        let outside = Context::for_tests();
        publish_git(&output, &outside).unwrap();
        assert_eq!(
            outside
                .choices_for_tests()
                .unwrap()
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            ["Included", "Global new", "Global old"]
        );
    }

    #[test]
    fn real_git_does_not_pair_effective_contacts_across_included_files_or_key_families() {
        let root = tempfile::tempdir().unwrap();
        let global = root.path().join("global");
        let included = root.path().join("included");
        fs::write(&global, format!("[user]\n name = Split user\n[author]\n email = parent-author@example.invalid\n[include]\n path = {}\n", included.display())).unwrap();
        fs::write(
            &included,
            "[user]\n email = included-user@example.invalid\n[author]\n name = Split author\n",
        )
        .unwrap();
        let mut command = isolated_git(root.path(), root.path(), &global);
        git_arguments(&mut command);
        let output = git_fixture_output(command);
        let context = Context::for_tests();
        publish_git(&output, &context).unwrap();
        let choices = context.choices_for_tests().unwrap();
        assert_eq!(
            choices.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["Split author", "Split user"]
        );
        assert!(choices.iter().all(|c| c.emails.is_empty()));
        assert_ne!(
            choices[0].records[0].git_origin,
            choices[1].records[0].git_origin
        );
    }

    #[cfg(unix)]
    #[test]
    fn fixture_output_captures_both_streams_without_pipe_capacity_or_stdin_waits() {
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "read answer && exit 9; i=0; while [ \"$i\" -lt 16384 ]; do printf 'stdout\\n'; printf 'stderr\\n' >&2; i=$((i + 1)); done",
        ]);
        let output = fixture_output(command, Duration::from_secs(10)).unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"stdout\n".repeat(16384));
        assert_eq!(output.stderr, b"stderr\n".repeat(16384));
    }

    #[cfg(unix)]
    #[test]
    fn fixture_output_timeout_kills_and_reaps_the_command() {
        let mut child = Command::new("/bin/sleep")
            .arg("30")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let pid = child.id() as libc::pid_t;
        // Expire immediately after spawn rather than racing a child-written PID
        // marker against a short real-time deadline on a busy runner.
        let error = wait_fixture_child(&mut child, Duration::ZERO).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        let mut status = 0;
        // SAFETY: pid identifies the fixture child and status is writable.
        assert_eq!(
            unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
        // SAFETY: signal zero only checks whether the fixture child still exists.
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
    }

    #[cfg(unix)]
    #[test]
    fn git_guard_expiry_preserves_account_fallback_without_expiring_preparation() {
        use std::sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
            mpsc,
        };
        use std::time::Instant;
        let directory = tempfile::tempdir().unwrap();
        let log = directory.path().join("started");
        let base = Instant::now();
        let elapsed = Arc::new(AtomicU64::new(0));
        let clock = elapsed.clone();
        let context = Context::with_clock_for_tests(Arc::new(move || {
            base + Duration::from_millis(clock.load(Ordering::SeqCst))
        }));
        let worker_context = context.clone();
        let worker_log = log.clone();
        let (done, completed) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", "printf started > \"$LOG\"; exec /bin/sleep 30", "--"]);
            command.env("LOG", worker_log);
            let result = (|| {
                assert!(collect_git(command, &worker_context)?);
                collect_account(
                    &worker_context,
                    || Some("Account".into()),
                    || Some("login".into()),
                    |_| panic!("usable login"),
                )
            })();
            done.send(result).unwrap();
        });
        let until = Instant::now() + Duration::from_secs(2);
        while !log.exists() && Instant::now() < until {
            std::thread::yield_now();
        }
        assert!(log.exists());
        elapsed.store(500, Ordering::SeqCst);
        completed
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert!(context.active());
        assert_eq!(
            context
                .choices_for_tests()
                .unwrap()
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            ["Account", "login"]
        );
        while !worker.is_finished() {
            std::thread::yield_now();
        }
        worker.join().unwrap();
    }
}
