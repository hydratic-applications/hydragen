use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use hydragen::layer_manifest::schema;

use super::{
    Choice, ContactEvidence, ContactInput, Record, Source, choices, github, identity,
    normalize_with, project,
};

const LIFETIME: Duration = Duration::from_secs(15);
const AUTHOR_WAIT: Duration = Duration::from_millis(1500);
pub(super) const CLEANUP: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pending,
    Ready,
    Expired,
    Fatal,
}

type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;
type Validator = Arc<dyn Fn(&'static str, &str) -> Result<(), schema::Error> + Send + Sync>;
pub(super) type Lane = Box<dyn FnOnce(&Context) -> Result<(), schema::Error> + Send>;

struct State {
    closed: bool,
    lifetime: Instant,
    author_deadline: Option<Instant>,
    completed: usize,
    processes: usize,
    records: Vec<Record>,
    fatal: Option<schema::Error>,
}

#[derive(Clone)]
pub(super) struct Context {
    state: Arc<Mutex<State>>,
    clock: Clock,
    validate: Validator,
}

impl Context {
    fn new(clock: Clock, validate: Validator) -> Self {
        let lifetime = clock() + LIFETIME;
        Self {
            state: Arc::new(Mutex::new(State {
                closed: false,
                lifetime,
                author_deadline: None,
                completed: 0,
                processes: 0,
                records: Vec::new(),
                fatal: None,
            })),
            clock,
            validate,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // No user callbacks, schema validation, or OS calls run under this lock.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn deadline(state: &State) -> Instant {
        state
            .author_deadline
            .map_or(state.lifetime, |author| author.min(state.lifetime))
    }

    fn is_active(&self, state: &State) -> bool {
        !state.closed && state.fatal.is_none() && (self.clock)() < Self::deadline(state)
    }

    pub(super) fn active(&self) -> bool {
        self.is_active(&self.lock())
    }
    pub(super) fn now(&self) -> Instant {
        (self.clock)()
    }
    #[cfg(test)]
    pub(super) fn effective_deadline(&self) -> Instant {
        Self::deadline(&self.lock())
    }

    pub(super) fn publish(&self, name: &str, source: Source) -> Result<bool, schema::Error> {
        self.publish_contacts(name, source, &[])
    }

    /// The bool reports whether publication is still active, not whether an
    /// invalid field was accepted. Existing name-only collectors use `publish`.
    pub(super) fn publish_contacts(
        &self,
        name: &str,
        source: Source,
        contacts: &[ContactInput<'_>],
    ) -> Result<bool, schema::Error> {
        self.publish_with_origin(name, source, contacts, None)
    }

    pub(super) fn publish_git_contacts(
        &self,
        name: &str,
        source: Source,
        origin: &[u8],
        contacts: &[ContactInput<'_>],
    ) -> Result<bool, schema::Error> {
        self.publish_with_origin(name, source, contacts, Some(origin.to_vec()))
    }

    fn publish_with_origin(
        &self,
        name: &str,
        source: Source,
        contacts: &[ContactInput<'_>],
        git_origin: Option<Vec<u8>>,
    ) -> Result<bool, schema::Error> {
        if !self.active() {
            return Ok(false);
        }
        let normalized: Result<Option<Record>, schema::Error> = (|| {
            let name = normalize_with(name, &|value| (self.validate)("author.name", value))?;
            let contacts = self.normalize_contacts(&source, contacts)?;
            Ok(name.zip(source.hint()).map(|(name, hint)| Record {
                name,
                source,
                hint,
                contacts,
                git_origin,
            }))
        })();
        let mut state = self.lock();
        let record = match normalized {
            Ok(record) => record,
            Err(error) => {
                state.fatal.get_or_insert_with(|| error.clone());
                return Err(error);
            }
        };
        if !self.is_active(&state) {
            return Ok(false);
        }
        if let Some(record) = record {
            if let Some(existing) = state.records.iter_mut().find(|existing| {
                existing.name == record.name
                    && existing.source == record.source
                    && existing.git_origin == record.git_origin
            }) {
                existing.merge_contacts(record.contacts);
            } else {
                state.records.push(record);
            }
        }
        Ok(true)
    }

    fn normalize_contacts(
        &self,
        source: &Source,
        contacts: &[ContactInput<'_>],
    ) -> Result<Vec<ContactEvidence>, schema::Error> {
        let mut result = Vec::new();
        for input in contacts {
            let Some(hint) = source.contact_hint(input.field) else {
                continue;
            };
            if let Some(value) = normalize_with(input.value, &|value| {
                (self.validate)(input.field.schema_field(), value)
            })? {
                let contact = ContactEvidence {
                    field: input.field,
                    value,
                    hint,
                };
                if !result.contains(&contact) {
                    result.push(contact);
                }
            }
        }
        result.sort_by_key(|contact| contact.field);
        Ok(result)
    }

    /// Enrich only an already published profile with this exact host/login.
    /// Missing or mismatched identities introduce no row or alias. Earlier
    /// contacts survive absent, invalid, or repeated later contact fields.
    pub(super) fn enrich_github_profile(
        &self,
        host: &str,
        login: &str,
        contacts: &[ContactInput<'_>],
    ) -> Result<bool, schema::Error> {
        if !self.active() {
            return Ok(false);
        }
        let source = Source::GithubProfile {
            host: host.to_owned(),
            login: login.to_owned(),
        };
        let normalized = self.normalize_contacts(&source, contacts);
        let mut state = self.lock();
        let contacts = match normalized {
            Ok(contacts) => contacts,
            Err(error) => {
                state.fatal.get_or_insert_with(|| error.clone());
                return Err(error);
            }
        };
        if !self.is_active(&state) {
            return Ok(false);
        }
        for record in &mut state.records {
            if record.source == source {
                record.merge_contacts(contacts.clone());
            }
        }
        Ok(true)
    }

    fn complete(&self, result: Result<(), schema::Error>) {
        let mut state = self.lock();
        if let Err(error) = result {
            state.fatal.get_or_insert(error);
        }
        state.completed += 1;
    }

    pub(super) fn reserve_process(&self) -> bool {
        let mut state = self.lock();
        if !self.is_active(&state) || state.processes >= 2 {
            return false;
        }
        state.processes += 1;
        true
    }

    pub(super) fn release_process(&self) {
        self.lock().processes -= 1;
    }

    #[cfg(test)]
    pub(super) fn for_tests() -> Self {
        schema::ensure_ready().unwrap();
        Self::new(Arc::new(Instant::now), Arc::new(schema::validate_field))
    }

    #[cfg(test)]
    pub(super) fn close_for_tests(&self) {
        self.lock().closed = true;
    }

    #[cfg(test)]
    pub(super) fn with_clock_for_tests(clock: Clock) -> Self {
        schema::ensure_ready().unwrap();
        Self::new(clock, Arc::new(schema::validate_field))
    }

    #[cfg(test)]
    pub(super) fn choices_for_tests(&self) -> Result<Vec<Choice>, schema::Error> {
        let state = self.lock();
        if let Some(error) = &state.fatal {
            return Err(error.clone());
        }
        Ok(choices(state.records.clone()))
    }
}

pub struct Preparation {
    context: Context,
}

impl Preparation {
    pub fn start(directory: &Path) -> io::Result<Self> {
        schema::ensure_ready().map_err(io::Error::other)?;
        let directory = directory.to_owned();
        let local: Lane = Box::new(move |context| {
            project::collect(&directory, context)?;
            if context.active() {
                identity::collect(&directory, context)?;
            }
            Ok(())
        });
        Self::start_lanes(Arc::new(Instant::now), [local, Box::new(github::collect)])
    }

    pub(super) fn start_lanes(clock: Clock, lanes: [Lane; 2]) -> io::Result<Self> {
        schema::ensure_ready().map_err(io::Error::other)?;
        let context = Context::new(clock, Arc::new(schema::validate_field));
        let preparation = Self { context };
        for (index, lane) in lanes.into_iter().enumerate() {
            let context = preparation.context.clone();
            thread::Builder::new()
                .name(format!("author-source-{index}"))
                .spawn(move || {
                    let result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lane(&context)))
                            .unwrap_or_else(|_| {
                                Err(schema::Error::Setup {
                                    message: "Author discovery worker failed internally".into(),
                                    detail: None,
                                })
                            });
                    // Fatal must be visible before readiness.
                    context.complete(result);
                })?;
        }
        Ok(preparation)
    }

    pub fn enter_author(&self) {
        let mut state = self.context.lock();
        if state.completed < 2 && self.context.is_active(&state) && state.author_deadline.is_none()
        {
            state.author_deadline = Some(self.context.now() + AUTHOR_WAIT);
        }
    }

    pub fn status(&self) -> Status {
        let state = self.context.lock();
        if state.fatal.is_some() {
            Status::Fatal
        } else if state.completed == 2 {
            Status::Ready
        } else if !self.context.is_active(&state) {
            Status::Expired
        } else {
            Status::Pending
        }
    }

    pub fn close_and_snapshot(&self) -> io::Result<Vec<Choice>> {
        let mut state = self.context.lock();
        state.closed = true;
        if let Some(error) = &state.fatal {
            return Err(io::Error::other(error.clone()));
        }
        let records = std::mem::take(&mut state.records);
        drop(state);
        // Freeze does not await subprocess or source cleanup.
        Ok(choices(records))
    }

    pub fn cancel(&self) {
        self.context.lock().closed = true;
        let until = Instant::now() + CLEANUP;
        while self.context.lock().processes != 0 && Instant::now() < until {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

impl Drop for Preparation {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::author_discovery::ContactField;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn controlled() -> (Preparation, Arc<AtomicU64>) {
        let start = Instant::now();
        let elapsed = Arc::new(AtomicU64::new(0));
        let time = elapsed.clone();
        let context = Context::new(
            Arc::new(move || start + Duration::from_millis(time.load(Ordering::SeqCst))),
            Arc::new(schema::validate_field),
        );
        (Preparation { context }, elapsed)
    }

    #[test]
    fn profile_enrichment_merges_matching_identity_without_losing_early_contacts() {
        use ContactField::{Email, ProfileUrl, Website};
        let (preparation, time) = controlled();
        let context = &preparation.context;
        let profile = Source::GithubProfile {
            host: "fixture.example".into(),
            login: "alex".into(),
        };
        context
            .publish_contacts(
                "Alex",
                profile.clone(),
                &[ContactInput {
                    field: ProfileUrl,
                    value: "https://fixture.example/alex",
                }],
            )
            .unwrap();
        time.store(3000, Ordering::SeqCst);
        preparation.enter_author();
        let deadline = context.effective_deadline();
        for (host, login) in [
            ("another.example", "alex"),
            ("fixture.example", "someone-else"),
        ] {
            context
                .enrich_github_profile(
                    host,
                    login,
                    &[ContactInput {
                        field: Email,
                        value: "wrong@example.invalid",
                    }],
                )
                .unwrap();
        }
        let update = [
            ContactInput {
                field: Email,
                value: " alex@example.invalid ",
            },
            ContactInput {
                field: Website,
                value: "https://alex.invalid",
            },
            ContactInput {
                field: ProfileUrl,
                value: "https://fixture.example/alex",
            },
        ];
        context
            .enrich_github_profile("fixture.example", "alex", &update)
            .unwrap();
        context
            .enrich_github_profile("fixture.example", "alex", &update)
            .unwrap();
        context
            .publish_contacts(
                "Alex",
                profile,
                &[
                    ContactInput {
                        field: Email,
                        value: "bad",
                    },
                    ContactInput {
                        field: Website,
                        value: "relative/path",
                    },
                ],
            )
            .unwrap();
        assert_eq!(context.effective_deadline(), deadline);
        time.store(4500, Ordering::SeqCst);
        assert_eq!(preparation.status(), Status::Expired);
        assert!(
            !context
                .enrich_github_profile(
                    "fixture.example",
                    "alex",
                    &[ContactInput {
                        field: Email,
                        value: "late@example.invalid"
                    },]
                )
                .unwrap()
        );
        let frozen = preparation.close_and_snapshot().unwrap();
        assert_eq!(frozen.len(), 1);
        assert_eq!(frozen[0].records.len(), 1);
        assert_eq!(frozen[0].emails.len(), 1);
        assert_eq!(frozen[0].emails[0].value, "alex@example.invalid");
        assert_eq!(
            frozen[0]
                .urls
                .iter()
                .map(|c| c.value.as_str())
                .collect::<Vec<_>>(),
            ["https://alex.invalid", "https://fixture.example/alex",]
        );
        assert_eq!(
            frozen[0].emails[0].hint,
            "GitHub profile alex on fixture.example email"
        );
        assert!(
            !context
                .enrich_github_profile("fixture.example", "alex", &update)
                .unwrap()
        );
        assert_eq!(frozen[0].emails.len(), 1);
    }

    #[test]
    fn contact_schema_setup_failures_are_fatal_in_publication_and_enrichment() {
        for enrichment in [false, true] {
            let (mut preparation, _) = controlled();
            let source = Source::GithubProfile {
                host: "fixture.example".into(),
                login: "alex".into(),
            };
            preparation.context.publish("Alex", source.clone()).unwrap();
            let error = schema::Error::Setup {
                message: "contact schema unavailable".into(),
                detail: None,
            };
            let injected = error.clone();
            preparation.context.validate = Arc::new(move |field, value| {
                if field == "author.email" {
                    Err(injected.clone())
                } else {
                    schema::validate_field(field, value)
                }
            });
            let contacts = [ContactInput {
                field: ContactField::Email,
                value: "alex@example.invalid",
            }];
            let result = if enrichment {
                preparation
                    .context
                    .enrich_github_profile("fixture.example", "alex", &contacts)
            } else {
                preparation
                    .context
                    .publish_contacts("Alex", source, &contacts)
            };
            assert_eq!(result, Err(error));
            assert_eq!(preparation.status(), Status::Fatal);
            assert!(preparation.close_and_snapshot().is_err());
        }
    }

    #[test]
    fn close_racing_contact_enrichment_keeps_the_entire_earlier_snapshot() {
        use std::sync::mpsc;
        let (started, validation_started) = mpsc::channel();
        let (release, validation_release) = mpsc::channel();
        let (finished, complete) = mpsc::channel();
        let (mut preparation, _) = controlled();
        preparation
            .context
            .publish_contacts(
                "Alex",
                Source::GithubProfile {
                    host: "fixture.example".into(),
                    login: "alex".into(),
                },
                &[ContactInput {
                    field: ContactField::ProfileUrl,
                    value: "https://fixture.example/alex",
                }],
            )
            .unwrap();
        let validation_release = Mutex::new(validation_release);
        preparation.context.validate = Arc::new(move |field, value| {
            if field == "author.email" {
                started.send(()).unwrap();
                validation_release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap();
            }
            schema::validate_field(field, value)
        });
        let context = preparation.context.clone();
        let worker = thread::spawn(move || {
            finished
                .send(context.enrich_github_profile(
                    "fixture.example",
                    "alex",
                    &[ContactInput {
                        field: ContactField::Email,
                        value: "late@example.invalid",
                    }],
                ))
                .unwrap();
        });
        validation_started
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        let frozen = preparation.close_and_snapshot().unwrap();
        assert!(frozen[0].emails.is_empty());
        assert_eq!(frozen[0].urls[0].value, "https://fixture.example/alex");
        release.send(()).unwrap();
        assert_eq!(
            complete.recv_timeout(Duration::from_secs(2)).unwrap(),
            Ok(false)
        );
        worker.join().unwrap();
        assert!(preparation.context.choices_for_tests().unwrap().is_empty());
        assert!(frozen[0].emails.is_empty());
    }

    #[test]
    fn fresh_author_budget_does_not_restart_and_freeze_blocks_late_records() {
        let (preparation, time) = controlled();
        time.store(3000, Ordering::SeqCst);
        assert_eq!(preparation.status(), Status::Pending);
        preparation.enter_author();
        preparation
            .context
            .publish("early", Source::AccountName)
            .unwrap();
        time.store(4400, Ordering::SeqCst);
        preparation.enter_author();
        assert_eq!(preparation.status(), Status::Pending);
        time.store(4500, Ordering::SeqCst);
        assert_eq!(preparation.status(), Status::Expired);
        assert!(
            !preparation
                .context
                .publish("late", Source::AccountName)
                .unwrap()
        );
        assert_eq!(preparation.close_and_snapshot().unwrap()[0].name, "early");
        assert!(
            !preparation
                .context
                .publish("later", Source::AccountName)
                .unwrap()
        );
    }

    #[test]
    fn lifetime_and_ready_results_never_start_an_extra_wait() {
        let (preparation, time) = controlled();
        time.store(15000, Ordering::SeqCst);
        preparation.enter_author();
        assert!(preparation.context.lock().author_deadline.is_none());
        assert_eq!(preparation.status(), Status::Expired);
        let (ready, _) = controlled();
        ready.context.complete(Ok(()));
        ready.context.complete(Ok(()));
        ready.enter_author();
        assert_eq!(ready.status(), Status::Ready);
        assert!(ready.context.lock().author_deadline.is_none());
    }

    #[test]
    fn raced_setup_error_is_not_flattened_to_ready_or_empty() {
        let (mut preparation, _) = controlled();
        let error = schema::Error::Setup {
            message: "injected".into(),
            detail: None,
        };
        let injected = error.clone();
        preparation.context.validate = Arc::new(move |_, _| Err(injected.clone()));
        assert_eq!(preparation.status(), Status::Pending);
        assert_eq!(
            preparation.context.publish("valid", Source::AccountName),
            Err(error)
        );
        preparation.context.complete(Ok(()));
        preparation.context.complete(Ok(()));
        assert_eq!(preparation.status(), Status::Fatal);
        assert!(preparation.close_and_snapshot().is_err());
    }

    #[test]
    fn invalid_input_returns_active_and_reservations_are_bounded() {
        let (preparation, _) = controlled();
        assert!(
            preparation
                .context
                .publish("\n", Source::AccountName)
                .unwrap()
        );
        assert!(preparation.context.reserve_process());
        assert!(preparation.context.reserve_process());
        assert!(!preparation.context.reserve_process());
        preparation.context.release_process();
        preparation.context.release_process();
        preparation.cancel();
        assert!(!preparation.context.reserve_process());
    }

    #[test]
    fn both_lanes_progress_independently_and_completion_retains_partial_results() {
        use std::sync::mpsc;
        let (local_release, wait_local) = mpsc::channel();
        let (local_started, local_ready) = mpsc::channel();
        let (remote_published, remote_ready) = mpsc::channel();
        let local: Lane = Box::new(move |context| {
            local_started.send(()).unwrap();
            wait_local.recv_timeout(Duration::from_secs(2)).unwrap();
            context.publish(
                "project",
                Source::Project {
                    file: super::super::ProjectFile::PackageJson,
                    index: 0,
                },
            )?;
            Ok(())
        });
        let remote: Lane = Box::new(move |context| {
            context.publish(
                "remote",
                Source::GithubProfile {
                    login: "login".into(),
                    host: "github.com".into(),
                },
            )?;
            remote_published.send(()).unwrap();
            Ok(())
        });
        let preparation =
            Preparation::start_lanes(Arc::new(Instant::now), [local, remote]).unwrap();
        local_ready.recv_timeout(Duration::from_secs(2)).unwrap();
        remote_ready.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(preparation.status(), Status::Pending);
        assert_eq!(
            preparation.context.choices_for_tests().unwrap()[0].name,
            "remote"
        );
        local_release.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while preparation.status() == Status::Pending && Instant::now() < until {
            thread::yield_now();
        }
        assert_eq!(preparation.status(), Status::Ready);
        assert_eq!(
            preparation
                .close_and_snapshot()
                .unwrap()
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            ["project", "remote"]
        );
    }

    #[test]
    fn close_racing_validation_excludes_late_record_and_keeps_foreground_unblocked() {
        use std::sync::mpsc;
        let (started, validation_started) = mpsc::channel();
        let (release, validation_release) = mpsc::channel();
        let (finished, complete) = mpsc::channel();
        let (mut preparation, _) = controlled();
        let validation_release = Mutex::new(validation_release);
        preparation.context.validate = Arc::new(move |_, _| {
            started.send(()).unwrap();
            validation_release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            Ok(())
        });
        let context = preparation.context.clone();
        let worker = thread::spawn(move || {
            finished
                .send(context.publish("late", Source::AccountName))
                .unwrap();
        });
        validation_started
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        assert!(preparation.close_and_snapshot().unwrap().is_empty());
        release.send(()).unwrap();
        assert_eq!(
            complete.recv_timeout(Duration::from_secs(2)).unwrap(),
            Ok(false)
        );
        // Completion is acknowledged before any test join.
        while !worker.is_finished() {
            thread::yield_now();
        }
        worker.join().unwrap();
        assert!(preparation.context.choices_for_tests().unwrap().is_empty());
    }

    #[test]
    fn author_wait_is_clamped_to_original_lifetime_and_pages_cannot_extend_it() {
        let (preparation, time) = controlled();
        time.store(14900, Ordering::SeqCst);
        preparation.enter_author();
        time.store(14950, Ordering::SeqCst);
        preparation
            .context
            .publish("page", Source::AccountName)
            .unwrap();
        preparation.enter_author();
        time.store(15000, Ordering::SeqCst);
        assert_eq!(preparation.status(), Status::Expired);
        assert_eq!(preparation.close_and_snapshot().unwrap()[0].name, "page");
    }

    #[test]
    fn drop_does_not_join_an_uninterruptible_source_and_late_completion_is_closed() {
        use std::sync::mpsc;
        let (started, source_started) = mpsc::channel();
        let (release, source_release) = mpsc::channel();
        let (finished, source_finished) = mpsc::channel();
        let preparation = Preparation::start_lanes(
            Arc::new(Instant::now),
            [
                Box::new(move |context| {
                    started.send(()).unwrap();
                    source_release.recv_timeout(Duration::from_secs(2)).unwrap();
                    finished
                        .send(context.publish("late account", Source::AccountName)?)
                        .unwrap();
                    Ok(())
                }),
                Box::new(|_| Ok(())),
            ],
        )
        .unwrap();
        source_started.recv_timeout(Duration::from_secs(2)).unwrap();
        let before = Instant::now();
        drop(preparation);
        assert!(before.elapsed() < Duration::from_millis(100));
        release.send(()).unwrap();
        assert!(
            !source_finished
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
        );
    }

    #[cfg(unix)]
    mod running_processes {
        use super::*;
        use crate::author_discovery::process;
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        use std::path::PathBuf;
        use std::process::Command;
        use std::sync::mpsc;

        struct Fixture {
            _directory: tempfile::TempDir,
            executable: PathBuf,
            pid: PathBuf,
            requests: PathBuf,
        }

        impl Fixture {
            fn new(script: &str) -> Self {
                let directory = tempfile::tempdir().unwrap();
                let executable = directory.path().join("fake-source");
                fs::write(&executable, format!("#!/bin/sh\n{script}\n")).unwrap();
                fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
                Self {
                    executable,
                    pid: directory.path().join("started-pid"),
                    requests: directory.path().join("requests"),
                    _directory: directory,
                }
            }

            fn started_pid(&self) -> libc::pid_t {
                // The child writes its PID only once it has actually started.
                // This is a bounded explicit fixture signal, not a lookup delay.
                let until = Instant::now() + Duration::from_secs(2);
                loop {
                    if let Ok(pid) = fs::read_to_string(&self.pid)
                        && let Ok(pid) = pid.trim().parse()
                    {
                        return pid;
                    }
                    assert!(Instant::now() < until, "source did not signal startup");
                    thread::yield_now();
                }
            }

            fn assert_reaped(pid: libc::pid_t) {
                let mut status = 0;
                // SAFETY: pid identifies this fixture's child; status is writable.
                assert_eq!(
                    unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
                    -1
                );
                assert_eq!(
                    io::Error::last_os_error().raw_os_error(),
                    Some(libc::ECHILD)
                );
                // SAFETY: signal zero only checks existence, sending no signal.
                assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
                assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
            }
        }

        fn clock() -> (Clock, Arc<AtomicU64>, Instant) {
            let base = Instant::now();
            let milliseconds = Arc::new(AtomicU64::new(0));
            let elapsed = milliseconds.clone();
            (
                Arc::new(move || base + Duration::from_millis(elapsed.load(Ordering::SeqCst))),
                milliseconds,
                base,
            )
        }

        #[test]
        fn author_cutoff_preserves_profile_and_first_page_and_reaps_stalled_later_page() {
            let fixture = Fixture::new(
                r#"
[ "$GH_PROMPT_DISABLED" = 1 ] || exit 9
[ "$1 $2 $3" = 'api --hostname fixture.example' ] || exit 9
printf '%s\n' "$*" >> "$REQUESTS"
case "$4" in
user) printf 'HTTP/2.0 200 OK\r\nX-Oauth-Scopes: read:user\r\n\r\n'; printf '%s' '{"login":"alex","name":"Alex","blog":"https://early.invalid","html_url":"https://fixture.example/alex","email":"private@example.invalid"}';;
graphql) case "$*" in
*after=next*)
  printf '%s\n' "$PRIVATE_STDERR_SENTINEL" >&2
  printf '%s\n' "$$" > "$PID_FILE"
  exec /bin/sleep 30;;
*) printf '%s' '{"data":{"viewer":{"login":"alex","email":"public@example.invalid","organizations":{"nodes":[{"login":"team","name":"Team","email":"team@example.invalid","websiteUrl":"https://team.invalid","url":"https://fixture.example/team"}],"pageInfo":{"hasNextPage":true,"endCursor":"next"}}}}}';;
esac;;
*) exit 9;;
esac
"#,
            );
            let executable = fixture.executable.clone();
            let pid_file = fixture.pid.clone();
            let requests = fixture.requests.clone();
            let (clock, elapsed, base) = clock();
            let (done, completion) = mpsc::channel();
            let (local_done, local_ready) = mpsc::channel();
            let preparation = Preparation::start_lanes(
                clock,
                [
                    Box::new(move |context| {
                        context.publish(
                            "Local project",
                            Source::Project {
                                file: super::super::super::ProjectFile::PackageJson,
                                index: 0,
                            },
                        )?;
                        local_done.send(()).unwrap();
                        Ok(())
                    }),
                    Box::new(move |context| {
                        let result = github::collect_for_tests(
                            context,
                            executable.as_os_str(),
                            "fixture.example",
                            |command| {
                                command
                                    .env_clear()
                                    .env("GH_PROMPT_DISABLED", "1")
                                    .env("GIT_TERMINAL_PROMPT", "0")
                                    .env("PID_FILE", &pid_file)
                                    .env("REQUESTS", &requests)
                                    .env(
                                        "PRIVATE_STDERR_SENTINEL",
                                        "private-error-not-for-the-terminal",
                                    );
                            },
                        );
                        done.send(result.clone()).unwrap();
                        result
                    }),
                ],
            )
            .unwrap();
            let pid = fixture.started_pid();
            local_ready.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_eq!(
                fs::read_to_string(&fixture.requests)
                    .unwrap()
                    .lines()
                    .count(),
                3
            );
            elapsed.store(3000, Ordering::SeqCst);
            assert_eq!(preparation.status(), Status::Pending);
            assert!(completion.try_recv().is_err());
            preparation.enter_author();
            assert_eq!(
                preparation.context.effective_deadline(),
                base + Duration::from_millis(4500)
            );
            elapsed.store(4499, Ordering::SeqCst);
            preparation.enter_author();
            assert_eq!(preparation.status(), Status::Pending);
            elapsed.store(4500, Ordering::SeqCst);
            let cutoff = Instant::now();
            completion
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap();
            assert!(cutoff.elapsed() < Duration::from_millis(500));
            assert!(
                !preparation.context.lock().closed,
                "effective deadline must stop gh even before foreground freeze"
            );
            let frozen = preparation.close_and_snapshot().unwrap();
            assert_eq!(
                frozen
                    .iter()
                    .map(|choice| choice.name.as_str())
                    .collect::<Vec<_>>(),
                ["Local project", "Alex", "Team"]
            );
            assert!(!frozen[2].eligible);
            assert_eq!(frozen[1].records.len(), 1);
            assert_eq!(frozen[1].emails[0].value, "public@example.invalid");
            assert_eq!(
                frozen[1]
                    .urls
                    .iter()
                    .map(|c| c.value.as_str())
                    .collect::<Vec<_>>(),
                ["https://early.invalid", "https://fixture.example/alex"]
            );
            assert_eq!(frozen[2].emails[0].value, "team@example.invalid");
            assert_eq!(
                frozen[2]
                    .urls
                    .iter()
                    .map(|c| c.value.as_str())
                    .collect::<Vec<_>>(),
                ["https://team.invalid", "https://fixture.example/team"]
            );
            assert!(!format!("{frozen:?}").contains("private@example.invalid"));
            assert!(!format!("{frozen:?}").contains("private-error"));
            Fixture::assert_reaped(pid);
            assert_eq!(
                fs::read_to_string(&fixture.requests)
                    .unwrap()
                    .lines()
                    .count(),
                3
            );
            assert!(
                !preparation
                    .context
                    .publish("late page", Source::AccountName)
                    .unwrap()
            );
            assert_eq!(frozen.len(), 3);
            assert!(preparation.context.choices_for_tests().unwrap().is_empty());
        }

        #[test]
        fn preparation_cancel_drop_and_lifetime_stop_owned_processes_and_reject_late_publication() {
            #[derive(Clone, Copy, Debug)]
            enum Stop {
                Cancel,
                Drop,
                Lifetime,
            }
            for stop in [Stop::Cancel, Stop::Drop, Stop::Lifetime] {
                let fixture =
                    Fixture::new("printf '%s\\n' \"$$\" > \"$PID_FILE\"; exec /bin/sleep 30");
                let executable = fixture.executable.clone();
                let pid_file = fixture.pid.clone();
                let (clock, elapsed, _) = clock();
                let (done, completion) = mpsc::channel();
                let mut preparation = Some(
                    Preparation::start_lanes(
                        clock,
                        [
                            Box::new(move |context| {
                                context.publish_contacts(
                                    "early",
                                    Source::GitAuthorEnvironment,
                                    &[ContactInput {
                                        field: ContactField::Email,
                                        value: "early@example.invalid",
                                    }],
                                )?;
                                let mut command = Command::new("/bin/sh");
                                command.arg(executable);
                                command.env_clear().env("PID_FILE", &pid_file);
                                let result = process::run(command, context, None);
                                let late = context.publish("late", Source::AccountName)?;
                                done.send((result, late)).unwrap();
                                Ok(())
                            }),
                            Box::new(|_| Ok(())),
                        ],
                    )
                    .unwrap(),
                );
                let context = preparation.as_ref().unwrap().context.clone();
                let pid = fixture.started_pid();
                assert_eq!(context.lock().processes, 1);
                let before = Instant::now();
                let mut completed_at_lifetime = None;
                let frozen = match stop {
                    Stop::Cancel => {
                        preparation.as_ref().unwrap().cancel();
                        preparation.as_ref().unwrap().close_and_snapshot().unwrap()
                    }
                    Stop::Drop => {
                        drop(preparation.take());
                        context.choices_for_tests().unwrap()
                    }
                    Stop::Lifetime => {
                        elapsed.store(15000, Ordering::SeqCst);
                        assert!(!context.active());
                        completed_at_lifetime =
                            Some(completion.recv_timeout(Duration::from_secs(2)).unwrap());
                        assert!(
                            !context.lock().closed,
                            "lifetime must stop the process without foreground closure"
                        );
                        preparation.as_ref().unwrap().close_and_snapshot().unwrap()
                    }
                };
                assert!(
                    before.elapsed() < Duration::from_millis(500),
                    "{stop:?} blocked foreground cleanup"
                );
                let (result, late) = completed_at_lifetime
                    .unwrap_or_else(|| completion.recv_timeout(Duration::from_secs(2)).unwrap());
                assert_eq!(result, Err(process::Failure::Unavailable));
                assert!(!late, "{stop:?} accepted a late result");
                assert_eq!(
                    frozen
                        .iter()
                        .map(|choice| choice.name.as_str())
                        .collect::<Vec<_>>(),
                    ["early"]
                );
                assert_eq!(frozen[0].emails[0].value, "early@example.invalid");
                assert!(!context.publish("even later", Source::AccountName).unwrap());
                assert_eq!(context.lock().processes, 0);
                Fixture::assert_reaped(pid);
            }
        }
    }
}
