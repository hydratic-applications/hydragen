## Context

Guided init currently asks Name, Author, Description, then Layer manifest format.
Only Name has a default. Author and Description start empty, and Enter skips them.
`src/main.rs` selects guided or flag-only input before collecting values and checks
the target and schema before prompting. `src/interaction.rs` uses cliclack and
shared schema-derived validation; skipped values become `None`.

This change addresses the guided-input part of design question 9 and the
responsiveness concern in question 11. It does not settle broader agent interfaces
or performance targets. It supports the foundations' DX, efficiency, and
performance values: reduce repetitive typing while preserving explicit choices,
offline operation, and fast automation. The responsiveness refinement also extends
the existing DX and performance values in `FOUNDATIONS.md`: prepare likely next
steps safely, distinguish background lifetime from visible waiting, and keep
unavoidable waits understandable and cancellable. The supplied init example is
integrated into its everyday-development scenario; `AGENTS.md` carries the general
implementation/review guidance. These are explicit refinements of existing values,
not a ninth value or a requirement to make every command asynchronous. Common
background-preparation terminology is explained here, so no glossary addition is
needed.

## Goals / Non-Goals

**Goals:**

- Offer distinct names from project author declarations, local identity, and
  optional GitHub sources, with project-declared authors first.
- Make choosing a name, entering another, and omitting author discoverable.
- Keep unavailable identity sources silent and non-fatal.
- Overlap discovery with Name input so optional lookup does not delay the first
  useful prompt, and show progress only for an actual wait at Author.
- Preserve existing flag-only defaults, schema validation, and write safety.

**Non-Goals:**

- Require Git, `gh`, GitHub authentication, or network access for init to work.
- Initiate login, request additional permissions, switch GitHub accounts, enumerate
  saved accounts/hosts, or contact providers other than the selected GitHub host.
- Infer identity from email addresses, repository owners, commit history, or remote
  URLs; an author can also be a team rather than the current account holder.
- Change Git or account settings, cache names persistently, introduce new flags,
  or populate author outside guided init.
- Change metadata definitions, supported formats, or repository boundaries.
- Read npm author preferences, `DEBFULLNAME`, generic `NAME`, or Git committer
  identity in this increment.
- Discover contributors, maintainers, copyright owners, parent/child/dependency
  project authors, workspace-inherited values, or dynamically generated metadata.
- Execute project configuration, package managers, or build tools to find authors.

## Decisions

### Discover only for an eligible guided session

Add a small author-discovery module, separate from terminal presentation and layer
manifest filesystem logic. Start it only on the guided branch, after target
preflight, schema readiness, and directory-name default validation, passing the
actual init directory. Return a session-owned preparation handle that publishes
validated candidates incrementally and reports completion, rather than awaiting
all discovery before opening Name. At the Author boundary, obtain one ordered
snapshot of names and their contributing sources. Distinguish project declarations, developer-identity sources,
and GitHub membership sources so default selection respects project attribution
without treating membership as authorship. Do not discover for help, version,
usage errors, preflight failures, input-flag execution, CI, redirected streams,
or `--non-interactive`.

Automatically filling flag-only metadata was considered, but would silently change
existing automation according to the machine's identity. An explicit `--author`
continues to be used unchanged and validated normally; omission remains omission.

### Prepare author choices while Name is being answered

Start the read-only work before displaying Name, without waiting for its results.
This is preparation for a likely next step, not optimistic writing or assuming
that the developer will accept a suggestion. Name input, including validation
retries, continues normally while author sources run. Do not delay Name until
`gh`, Git, or file lookup finishes, and do not restart discovery when Name is
corrected or when Author is reached.

Use a small, bounded worker arrangement: one local lane for project/Git/account
sources and one optional GitHub lane, so network latency does not hold up local
results and local work does not postpone the network request. Publish usable
records as each source or page completes, with stable source priority recorded
independently of completion order. Workers do not read terminal input, render
spinners/prompts, print errors, create project files, or modify identity settings.
The foreground session alone owns terminal presentation and input.

Keep two separate clocks for this increment:

- **Preparation lifetime: 15 seconds from discovery start.** This bounds optional
  background work even if the developer stays on Name for a long time. All GitHub
  calls/pages share this lifetime; there is no separate 1.5-second GitHub timer
  running during Name input. Git retains its independent 500 ms source guard.
- **Additional Author wait: at most 1,500 ms from entering the Author step.** Time
  spent preparing while Name is active does not consume this allowance. If work
  is already complete or has reached its lifetime limit, use the available
  candidates immediately without a loading indicator. Otherwise continue only
  until completion, the preparation deadline, or this Author deadline, whichever
  comes first. New sources, pages, and partial results never reset a deadline.

For example, if discovery finishes after three seconds while the developer spends
four seconds on Name, Author has no lookup wait. If Name is accepted after 200 ms
and discovery is still running, Author may wait up to another 1,500 ms, not just
the remainder of a timer started at launch. An unfinished attempt is not restarted
after spending 15 seconds on Name; its already-collected names remain usable.

While there is an actual Author wait, use `cliclack::spinner()` on standard error
with the concise message `Finding author suggestions…`. Do not show it alongside
Name or add startup/background status messages. Clear the indicator on every
exit before opening the selector/text prompt or reporting cancellation/failure;
do not leave a synthetic success/warning line claiming optional lookup was complete.
Do not add a minimum display time or delay ready results to show an animation.
Keep the meaning understandable without colour and use normal text feedback when
the terminal cannot animate.

Keep Escape and Ctrl-C cancellation available during the wait, with the same
nonzero/no-files-changed outcome as cancellation at a prompt. The spinner is a
renderer, not an input handler: its `cancel` method does not establish keyboard
cancellation. Add a foreground cancellation-aware wait using supported console/
signal facilities without replacing cliclack's Name/Author prompt handling or
running competing terminal readers. If the existing console API cannot provide
a bounded key read, use focused cross-platform event polling rather than leave
an uncancellable input-reader thread behind. Restore terminal state before the
next cliclack prompt on every exit. Deliberate cancellation still receives one
report, never a generic duplicate or a fallback Author prompt. Unexpected wait/
terminal failures remain errors rather than ordinary unavailable-name results.

At completion or either deadline, close collection, cancel unneeded work, and
freeze/group the candidates already available. Use this stable snapshot for the
Author selector or shared empty input; ignore late arrivals rather than inserting
options, reordering choices, changing defaults, or reopening a prompt. Keep source
priority intact even when a lower-priority source completed first. Source failures
remain silent and previously accepted names remain usable.

The preparation handle owns cancellation and child-process cleanup for every
exit, including Name cancellation, prompt I/O failure, timeout, and session drop.
Terminate and reap owned subprocesses; do not keep paging or initiate new lookups
after the handle is closed. Cleanup must not add an unbounded join to Author or
cancellation. Do not assume that arbitrary platform account or filesystem calls
can be interrupted: isolate such calls from the foreground wait, bound worker
count, and prevent late completion from publishing or starting further work. This
is a limitation to assess when choosing the account API, not a reason to block
the CLI forever. Cache schema setup before launching work so known internal
schema failures still stop before Name; do not disguise unexpected internal errors
as source unavailability.

Sources: [cliclack spinner](https://docs.rs/cliclack/0.5.6/cliclack/#spinner)
and [progress lifecycle methods](https://docs.rs/cliclack/0.5.6/cliclack/struct.ProgressBar.html).

### Use focused platform helpers without adding an async framework

Use `whoami` 2.1's fallible `realname` and `username` APIs with its desktop `std`
feature, not an invented unknown-name fallback. These identify the effective OS
account, which can differ from the login account under `sudo`. OS directory
services can block, so this lookup stays in the isolated local worker; it never
holds the foreground wait open after cutoff.

Use `process-wrap` 10's standard-library process-group support on Unix and Job
Object support on Windows to contain ordinary child processes and descendants.
Use its nonblocking `start_kill`/`try_wait` controls, not its blocking `kill` or
`wait_with_output` helpers. Keep child ownership in the runner, reserve launches
under the short shared state lock, and perform OS spawn outside that lock. A
launch finishing after closure immediately enters cleanup and cannot publish.
Capture stdout and stderr concurrently with separate 1 MiB limits plus one
overflow-detection byte per stream. Never join an unfinished capture reader;
terminate the group/job and allow at most 100 ms for runner cleanup, then abort
that source lane rather than accumulate another request's capture helpers. Freeze
does not wait for this cleanup interval. Deliberately escaped Unix process groups
and stalled kernel operations are not a promise of absolute process-tree control.

Use `crossterm` 0.29's timed event polling only during the foreground loading wait.
Restore raw mode explicitly on every exit, with a best-effort drop guard. On
Windows save and restore the exact console mode through target-specific
`windows-sys` APIs; merely toggling generic raw-mode flags can change previous
settings. Native cliclack still owns the actual metadata prompts and selectors.
Unix non-symlink file-open flags use target-specific `libc` constants.

These dependencies offer focused synchronous APIs without introducing Tokio or
another asynchronous runtime. Their MIT license alternatives are compatible
with Hydragen's GPL license; preserve the existing release-time dependency notice
review rather than treating this choice as a complete compiled-release audit.
See the [whoami APIs](https://docs.rs/whoami/2.1.3/whoami/),
[process-wrap standard frontend](https://docs.rs/process-wrap/10.0.1/process_wrap/std/),
and [crossterm event ownership rules](https://docs.rs/crossterm/0.29.0/crossterm/event/).

### Order project and identity sources

Collect all usable candidates; finding a name does not stop discovery. Presentation
order, including the optional GitHub sources described below, is:

1. Directly declared authors in the init directory's `package.json`, `Cargo.toml`,
   and `pyproject.toml`, in that fixed file order and each field's declaration order.
   These share the highest priority tier; file order is a stable tie-break, not a
   claim that one ecosystem's author metadata is inherently more authoritative.
2. `GIT_AUTHOR_NAME` from the process environment.
3. Git `author.name` values applicable to the init directory.
4. Git `user.name` values applicable to the init directory.
5. The active GitHub account's display name, falling back to its login.
6. The current local account's display/full name.
7. The current local account's login username.
8. If the account API cannot supply a usable login username, platform-appropriate
   environment usernames: `LOGNAME` then `USER` on Unix, or `USERNAME` on Windows.
9. Accessible GitHub organisation names, sorted by normalized name and then login.

### Read directly declared authors from project files

Read only the three fixed filenames in the actual init directory, once each, after
preflight and schema readiness and only in guided mode. Parse JSON/TOML directly
with maintained parsers; this is not a request to validate the entire project for
its ecosystem. Limit each read to 1 MiB, accept only regular files without following
symlinks, and skip missing, unreadable, oversized, invalidly encoded, malformed, or
unsupported input silently. Do not recurse, search parents, follow workspace/file
references, inspect dependencies, or execute `npm`, `cargo`, Python, build backends,
or executable configuration such as `setup.py`. No project file is modified.

Extract only these declared fields:

- `package.json`: `author` as either an object with a string `name`, or npm's
  documented person string with a name and optional `<email>` and `(url)` parts.
  Extract only the name. Do not consult `contributors`, `maintainers`, or `.npmrc`.
- `Cargo.toml`: each string in `[package].authors`, accepting Cargo's optional
  trailing `<email>` notation and extracting only the name. Cargo now deprecates
  this field, but existing declarations remain useful read-only suggestions.
  Skip workspace-inherited author tables; do not consult `[workspace.package]` or
  resolve `package.workspace`, even if some workspace data is in the same file.
- `pyproject.toml`: each string `name` in `[project].authors` entries. Skip
  email-only entries, `[project].maintainers`, and tool-specific formats such as
  `[tool.poetry].authors`. Read any directly listed authors without asking a build
  backend to resolve or extend dynamic metadata.

For person strings, recognize the documented optional contact syntax rather than
guessing names from email addresses or indiscriminately splitting punctuation.
Skip ambiguous/malformed shorthand instead of copying contact details into an
author suggestion. Object/table `name` fields are explicit names, not person
shorthand. Normalize and validate extracted names through the shared candidate
path. Invalid array entries are skipped independently; a malformed file contributes
nothing but does not erase other files' or sources' names.

Every declared author is a separate candidate; do not join an array into one
string or automatically add all authors to the layer manifest. Combine equal names
across files and other sources, with hints such as `package.json author` and
`Cargo.toml package.authors`. Keep differing names as choices rather than treating
disagreement as an error. A project author may be a person, team, or organisation:
all are eligible for initial selection because this source explicitly credits
authorship. Do not try to infer entity type from the spelling of a name.

This supersedes the initial preference for the current developer's identity. The
project's existing attribution is stronger context when adding a layer manifest to
an established project, even when the person running init is a different maintainer.
The developer still confirms a choice and can enter another author or skip it.

Sources: [npm author forms](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#people-fields-author-contributors),
[Cargo authors](https://doc.rust-lang.org/cargo/reference/manifest.html#the-authors-field),
and [Python project authors](https://packaging.python.org/en/latest/specifications/pyproject-toml/#authors-maintainers).

### Read Git and local account names

Read Git values through Git itself in the init directory, without a shell. Use
its all-values output with scope/origin information and include handling, rather
than asking only for one effective value. Respect applicable conditional includes
and enabled worktree configuration. Keep distinct names from lower-priority scopes
as alternatives: a repository work name must not hide a global personal name from
the developer. Rank Git values by normal scope precedence (command, worktree,
local, global, system), with later definitions first within a scope. Do not scan
inactive conditional includes or parse Git configuration files ourselves. Outside
a repository, global/system values can still be found. No email is required.
Configuration precedence orders choices; it does not silently decide attribution.

Use a focused cross-platform account API or library that reads the current account
without interactive prompts or application-initiated network requests. Collect
both its real/display name and its login username when usable, even when Git has
already supplied names. Environment username values are only an account-username
fallback, not a source of additional alternatives when that API succeeded.
Do not use a hostname as an author. Select the concrete library during
implementation after checking supported platforms, licensing, and local lookup
behaviour; avoid a large dependency solely for this fallback.

Parsing Git configuration manually or using `git var GIT_AUTHOR_IDENT` was
rejected: the former duplicates Git semantics, and the latter may require email
or invent an identity we do not want.

### Add optional GitHub candidates through existing gh authentication

When `gh` is available, attempt read-only requests using the credentials it would
normally use for one host. Honour a non-empty `GH_HOST`; otherwise use `github.com`.
Pass that host explicitly on each request, rather than inferring it from repository
remotes or probing every saved host. Let `gh` resolve its active account or existing
token environment; do not read credential files, extract tokens, switch accounts,
or run login/permission-refresh commands. Disable `gh` prompting and disconnect
stdin, capturing output without displaying authentication or API errors.

First use `gh api user` for that host to obtain the authenticated user's `name` and
`login`. A usable display name supplies the personal candidate; otherwise try the
login. Identify the actual returned account and host in the source hint, because
the GitHub profile may differ from the Git author. Ignore unrelated response
fields such as email or employer. If authentication/profile lookup fails, retain
local candidates and skip GitHub organisation lookup.

After a successful profile lookup, use GraphQL, GitHub's query API, through
`gh api graphql` to read the authenticated viewer's organisation `name` and `login`,
plus the viewer login and pagination information. Request at most 100 organisations per
page. This obtains display names without a separate REST detail request for every
organisation. Page only while the preparation handle's effective deadline allows;
never use uncontrolled auto-pagination.
Retain usable complete records from successful pages if a later page fails or
times out. Ignore responses attributed to a different viewer than the profile
lookup, rather than switching identity mid-discovery.

Use each organisation's usable display name, otherwise its login, as one candidate.
Hints identify it as a GitHub organisation, with its login and host. Membership
only suggests a possible author; it does not establish that the organisation owns
or authored this layer. Never use the profile's free-text company field to infer
organisation membership. Missing permissions, policy/single-sign-on restrictions, token limitations,
empty results, and API errors do not trigger permission requests or warnings and
must not be described as proof that the user belongs to no organisations.

This explicitly replaces the initial local-only discovery boundary: guided init
may now make optional authenticated network requests. It still works offline and
without `gh`; flag-only execution does not even attempt tool/account lookup. Do
not add a login question, permission tutorial, or failure message to the prompts.
Existing custom entry and Skip remain available regardless of GitHub results.

Sources: [gh api](https://cli.github.com/manual/gh_api),
[authenticated user profile](https://docs.github.com/en/rest/users/users#get-the-authenticated-user),
[organisation access limitations](https://docs.github.com/en/rest/orgs/orgs#list-organizations-for-the-authenticated-user),
and [GraphQL viewer and organisation fields](https://docs.github.com/en/graphql/reference/objects#user).

### Group equal names and retain their sources

After normalizing and validating candidates, group names by exact string equality.
For example, ` Alex Morgan ` and `Alex Morgan` become one option after trimming.
Do not case-fold names, collapse internal whitespace, or rewrite spelling; different
accepted strings remain distinct choices. Preserve each group's first position in
source-priority order and append its distinct contributing sources in that order.

Use the name as the option label and compact source labels as the hint, for example
`Alex Morgan` with `Git local user.name, system account name`. Distinguish Git keys
and scopes and label environment sources by their variable names. Repeated
occurrences of the same source label do not duplicate the hint. Do not expose full
configuration paths or unrelated account details in these hints. Source information
explains the suggestion but is not stored in the layer manifest.

Apply the same grouping across project, GitHub, and local identity sources. Include
project filename and field in the source hints. Retain GitHub account/host
and organisation/login labels in the hints, but never credentials or raw API
responses. Validate hint components as safe single-line display text as well as
validating candidate names. A group with a project declaration or developer-identity
source remains eligible for initial selection even if a GitHub organisation has
the identical name. Only groups supported solely by GitHub membership are
ineligible as the initial author; a project-declared organisation is not such a group.

This replaces the initial first-match approach: users can choose among all found
names without seeing several rows that would write exactly the same author.

### Treat discovery as bounded, fallible input

Trim surrounding whitespace from discovered candidates, then reject empty values,
invalid text encoding, and control characters (including embedded line breaks and
terminal escape sequences). Validate each remaining candidate with the maintained
schema's author definition. A content failure discards only that candidate; an
internal schema failure remains fatal rather than being disguised as no identity.
Do not trim or otherwise transform author text explicitly entered by the user.

Missing executables, absent keys, unreadable/malformed project files or configuration,
failed account lookups, GitHub failures, invalid candidates, and timeouts produce no user-facing
warning. Continue collecting other sources and retain previously found names, or
return an empty collection. Do not invent a fallback such as `Unknown`, an email,
or a hostname.

Give Git subprocess lookup a shared 500 ms execution budget, not 500 ms per
candidate. Bound captured output, disconnect child stdin, capture stderr rather
than displaying it, and kill and reap a timed-out child. When the Git budget is
exhausted, skip remaining Git commands and try the account fallback. Use injectable
lookup boundaries and a deadline so ordering, failure, and timeouts can be tested
without relying on the developer's account or Git setup. Platform account APIs
can have platform-dependent latency; avoid repeated lookup and verify their local
behaviour rather than claiming this budget bounds all OS account services.

An unbounded `Command::output` call would be simpler but could stall guided init
on a misbehaving Git executable. A timeout warning would draw attention to an
optional convenience rather than help the developer initialise their layer.

All `gh` invocations share the preparation handle's 15-second lifetime and, once
Author is reached, its additional 1,500 ms wait deadline. This replaces the former
1,500 ms budget measured from GitHub lookup start. Bound output capture, terminate
and reap children at the effective deadline or cancellation, and retain earlier
validated candidates. No request or page resets either deadline. Do not add
persistent response caching or retries. The foreground allowance bounds time
awaiting suggestions rather than pretending all platform account/filesystem calls
are interruptible.

### Show a selector only when there are names to choose from

After preparation is complete or its wait ends, if at least one distinct name is
in the frozen snapshot, use cliclack's native single-choice
selector labelled `Author (optional)`. List every grouped name with its source
hint, then `Enter another author`, then `Skip`. Initially select the first group
with a project declaration or developer-identity source (Git identity, GitHub
profile, or local account/username). This selects a project author first when
available, including a project-declared organisation. If only GitHub-membership
suggestions exist, initially select `Enter another author`, not an inferred author.
Enter on a name accepts it without another confirmation or text prompt.
Author remains one string: multiple names cannot be selected together.

`Enter another author` opens an empty optional Author text input. It has no
discovered default, placeholder name, or prefilled editable text: those names are
already available in the selector and this action requests a different value.
Typing supplies an author; Enter without text omits it. Either submission proceeds
to Description, without returning to the selector or restoring a discovered name.
`Skip` directly omits author and moves to Description without opening a text input
or asking for confirmation. Use typed action values, not name strings, so a
discovered name such as `Skip` cannot be confused with the omission action.

If no names are found, bypass the selector completely and open that same empty
optional text input through the same collection/validation function used by
`Enter another author`, not a separately implemented copy. There is no benefit in
asking someone to choose between entering a name and skipping when the optional
input already supports both. Both omission routes produce `None`, never an empty
string or action label in the layer manifest.

Keep the existing short session guidance: Enter accepts defaults or skips optional
fields. The selector itself exposes its actions; do not add special skip tokens,
undocumented keys, or a tutorial to compensate for an ambiguous text field.
Name keeps its directory-name placeholder/default with typing-to-override, not
editable prefilling. Description remains an empty optional input. Empty author
text and Description can still be cleared and submitted to skip after invalid
whitespace-only input; existing correction guidance remains accurate.

The completed author step shows the accepted name or the existing dimmed, italic
`Skipped` indicator, including for the selector's Skip action. Source hints and
action labels are not metadata. Guidance and hints must remain clear without
colour. Use native selection scrolling for long lists rather than dropping names.

Editable prefilling was rejected because it differs from Name's default behaviour.
A placeholder default alone cannot express both Enter-to-accept and Enter-to-skip.
A special input such as `-` makes omission depend on remembering syntax and adds
prompt noise. The native selector makes all actions visible; its tradeoff is one
additional step only when entering a custom author after names were discovered.

### Preserve agreed main contracts during integration

Integration with main `328644ec72012e11ae01ffbf7213694ebd40e7be` applies
already-agreed constraints, not a new author-suggestion product design. The
[shared name policy](../../specs/layer-manifest/spec.md) restricts layer names to
the maintained schema's lowercase definition. Directory defaults must pass that
definition before prompts or author preparation; failure gives `--name` override
guidance rather than trimming, lowercasing, or sanitising the directory name.
A valid explicit name bypasses default derivation and selects flag-only input,
with no discovery. A rejected guided name stays editable with schema-derived
correction, without restarting author preparation.

This layer-name restriction does not apply to author or description: explicit
accepted strings retain Unicode, uppercase and surrounding whitespace unchanged.
Only discovered candidates are trimmed and checked for safe display before the
existing author-schema validation and exact grouping. Project-first ordering,
membership eligibility, native selection, preparation deadlines and the accepted
#115 limitation remain unchanged.

Main's [shared CLI diagnostic contract](../../specs/cli-interaction/spec.md) also
remains in force. Parsing outcomes precede current-directory resolution, layer
discovery, schema preparation, prompts and writes. Explicit help/version remain
plain and successful; parser and operational failures use eligible compact
styling or plain output, structured causes, shared block layout with literally
empty separators, established streams/statuses and truthful write/reporting state.
Guided rejection and cancellation retain their cliclack interaction variants.
Do not restore the former plain-usage-error exception or apply Name's constraints
to optional metadata while reconciling these flows. Main's argument-only local
dependency addition and its source-preserving write rules remain independent of
guided author discovery.

The [initialisation runtime-service requirement](../../specs/layer-initialization/spec.md#requirement-initialisation-works-without-external-runtime-services)
previously prohibited all Git commands. This change explicitly supersedes that
blanket prohibition only for the optional read-only Git configuration queries
already selected for eligible guided author discovery. It also records the
already-selected optional GitHub requests through existing `gh` authentication.
Neither becomes a required runtime service: offline/no-Git operation, flag-only
and early-exit isolation, and optional author entry/omission remain intact. Git
initialisation, repository mutation, other Git commands, project-tool execution,
login, permission requests and account switching remain forbidden. The focused
`layer-initialization` delta reconciles the specifications; it does not introduce
a new discovery source or redesign the agreed interaction.

The validation recorded for `734ce99` is historical evidence for the earlier
integration with main `4a82827`, not a test result for this newer integration.
Repeat runtime validation and independent review against the resolved candidate;
record those results separately in [verification.md](verification.md).

### Keep terminal-test sources visible to the pinned coverage exporter

PR #15's first Linux coverage run reached report preparation but found no LCOV
record for `src/interaction/author_tests.rs`. The pinned cargo-llvm-cov 0.9.1
exporter filters that test-style filename even though the module's instrumented
tests ran. The [CI diagnosis](verification.md#pr-ci-coverage-export-failure-and-approved-compatibility-fix)
records the source evidence and distinguishes this from a failed percentage gate.

Rename only the source file to `src/interaction/author_terminal.rs`, preserving its
body, test functions, deferred assertions and Rust module name `author_tests`
through `#[path]`. Keep it under `src`. Add a compatibility guard tied to the
workflow's pinned exporter version and its default filename filters; this is not
a source exemption. Do not change coverage settings, exclusions or either 80%
threshold. Including these terminal-test helper lines in measurement changes the
measured scope, not production test coverage or the strength of its evidence.
Local validation of the fix based on `7545817` passed: locked build, formatting,
all-target lint, 673 Rust tests with 0 failures and 3 ignored entries, and 42
coverage-policy tests after the exact rename was staged for the index guard.
The focused code review was accepted conditionally on full results and updated
documentation; final publication review and a fresh hosted run proving the new
LCOV source record and actual unchanged 80% gate results remain pending.

### Keep validation and file creation unchanged

Use the same field validation and full-candidate schema validation as existing
guided input. Do not introduce a separate author field schema or load
implementation assets from OpenSpec. Preserve prompt order, format selection,
cancellation, exclusive target creation, and duplicate reporting. Discovery reads
project authors, local identity, and optionally GitHub data; it changes no identity
settings or project files and stores no credentials or source-membership metadata.

Tests must cover source ordering and failures independently, plus real Git
configuration with an isolated home/configuration environment. Update terminal
fixtures to use deterministic discovered names and choose the explicit Skip action
rather than assuming the Author step is always an empty text input. Exercise one
name, multiple names, grouped sources, selection, custom entry, direct skip,
empty/retry/clear, no-name text entry, no-colour, and cancellation paths in actual
terminal sessions with bounded waits and child cleanup. Captured-output tests must prove
that flag-only execution never invokes discovery and still omits absent author.

Use fake `gh` executables/fixtures for automated tests, not the developer's real
credentials, account, memberships, or network. Cover profile/login fallback,
organisation display-name/login fallback, partial and paginated results, permission
and network failures, the shared timeout, explicit host/account attribution,
cross-source grouping, and membership-only initial selection. Terminal tests
must continue to work deterministically without installed or authenticated `gh`.

Test preparation using an injected monotonic clock and controlled source-completion
signals rather than relying on long sleeps. Cover instant-ready, completion during
Name, fast Name plus residual wait, slow Name, lifetime expiry before Author,
partial results, deadline races, frozen choices, and no timer reset on new pages
or Name retries. Real-terminal tests must prove Name appears and remains usable
before delayed source completion, spinner visibility only during Author waiting,
colour-disabled feedback, cancellation while Name or the spinner is active, and
child cleanup with no later output or file creation. Review actual time visibly
waiting in addition to total command duration; passing concurrency tests alone
does not establish an enjoyable or responsive experience.

Use maintained fixtures outside OpenSpec for all supported project-author forms,
including shorthand contacts, arrays, invalid entries, conflicting files, duplicate
names, malformed/oversized files, non-regular entries, and unsupported inheritance
or dynamic forms. Assert project-first ordering and eligibility of project-declared
organisations, no traversal/tool execution, and no project-author reads on any
flag-only or early-exit path. Verify all pre-existing file contents are unchanged.

## Risks / Trade-offs

- **A configured identity belongs to the wrong person or team** → Show names and
  sources before writing, make omission explicit, and never infer identity in
  automation.
- **More sources create more choices** → Group identical names, rank project
  declarations before identity sources, use compact hints, and keep environment
  usernames as a fallback only when account username lookup fails.
- **Host settings make tests nondeterministic** → Inject account sources for unit
  tests and isolate Git environment/configuration in process and terminal tests.
- **Git or platform lookup is slow** → Bound subprocess execution/output, perform
  discovery once, and review local account API behaviour before adoption.
- **GitHub lookup adds optional network latency** → Start it during Name input,
  separate the preparation lifetime from the short Author wait, avoid per-organisation
  detail requests/retries, and retain local-only use.
- **Concurrent work disrupts prompts or changes choices** → Only the foreground
  renders/reads input; freeze one snapshot before Author and ignore late results.
- **A stalled worker delays cancellation** → Own and terminate subprocesses, bound
  worker count, avoid unbounded foreground joins, and test cancellation during
  earlier input as well as the visible wait.
- **Private memberships appear in terminal suggestions** → Read only what existing
  credentials permit, label organisations clearly, and never persist membership
  details; only an explicitly selected author name goes into the layer manifest.
- **Membership is mistaken for project authorship** → GitHub-membership-only
  alternatives are never the initial selection. An independent project author
  declaration or developer-identity source makes the same value eligible.
- **Project author data is stale or names a previous maintainer** → Label it as
  project metadata rather than the current user's identity, preserve other choices,
  and require the developer to accept, replace, or skip it.
- **Conditional selection conflicts with current text-only wording** → Replace
  that rule explicitly in the delta spec and update README, help, and AGENTS
  guidance during implementation, rather than leaving contradictory instructions.

### Accept the native selector limitation pending an upstream fix

Implementation review on 2026-10-19 found that cliclack 0.5.6 can initially hide
the selected option when `initial_value` points below the first `max_rows` options.
Hydragen uses a seven-row window. With seven or more distinct names supported
only by GitHub membership and no project-declared or developer-identity group,
the initially selected `Enter another author` action has a zero-based index of
at least seven, outside the initial visible indices zero to six. Moving Up can
also select a name that remains off-screen, and accepting that name can omit its
label from the completed prompt. The selected value is preserved, including the
author stored in the layer manifest; the defect affects selection visibility and
submitted feedback, not metadata correctness. See the
[initial cursor setup](https://github.com/fadeevab/cliclack/blob/bf5e861ba89ab4163912f0e383cfb06d60b98970/src/select.rs#L99-L116)
and [viewport-limited rendering](https://github.com/fadeevab/cliclack/blob/bf5e861ba89ab4163912f0e383cfb06d60b98970/src/select.rs#L159-L192).

In production, whenever any project-declared or developer-identity group exists,
the source order places the first eligible group at index zero, within the normal
initial window. This does not rule out the membership-only case or resolve its
missing selection and feedback.

Hydragen's maintainer accepts landing the useful author-suggestion feature with
this acknowledged limitation instead of blocking the entire increment. This
supersedes the earlier decision to wait for an upstream fix before landing.
Project-first suggestions, optional entry and omission, and bounded preparation
remain useful while the display defect remains outstanding. Keep the agreed
option order, membership-only default, and native scrolling unchanged. Do not
vendor, fork, or locally patch cliclack, add a runtime warning, or introduce a
product workaround. The upstream follow-up is
[cliclack issue #115](https://github.com/fadeevab/cliclack/issues/115).

Hydragen tracks adoption and regression verification in
[issue #14](https://github.com/hydratic-applications/hydragen/issues/14).
As of 2026-10-20, [upstream PR #116](https://github.com/fadeevab/cliclack/pull/116)
proposes a fix for #115 and is open, not merged. Its existence does not resolve
the limitation in Hydragen's unmodified dependency or complete the deferred tasks.

Temporarily ignore only these two regression tests in
`src/interaction/author_terminal.rs` (Rust module `author_tests`), with an explicit
ignore reason linking issue #115 and identifying the upstream initial-selection/
feedback defect:

- `long_membership_only_selector_visibly_selects_custom_and_enter_opens_empty_input`
- `moving_up_from_long_membership_custom_visibly_selects_and_summarises_the_name`

Keep their original assertions so they continue to demonstrate the known failures
against the unmodified dependency. Run them separately with:

```sh
cargo test --locked --bin hydragen long_membership -- --ignored
```

These two temporary exclusions are separate from the existing ignored child-only
terminal driver, which is invoked by the terminal tests. Report the exclusions
and the deferred tests' results explicitly; a passing default suite does not mean
all scenarios pass. Tasks 3.3 and 4.5 remain unchecked for their upstream-dependent
portions. After adopting an upstream fix, remove both temporary ignore markers,
pass both regressions, and repeat full validation before closing that follow-up.
Landing with this exception does not establish full specification conformance or
authorise archiving the change.

## Migration Plan

No layer manifest migration is needed. Implement and test the guided-only lookup,
then update existing usage explanations without expanding the README into an
identity-resolution reference. Explain optional authenticated GitHub network access
and continued offline/no-`gh` operation in the existing guidance. Record the scoped
question-9 resolution alongside this proposal. If the feature is reverted, restore empty Author input and its
guidance; layer manifests already written with an accepted author remain valid.
The general foundations/guidance refinement is independently meaningful and does
not need reverting if this particular discovery implementation is removed.

## Open Questions

No product decision remains open. Landing is accepted with the native selector
limitation recorded above; the upstream-dependent portions of tasks 3.3 and 4.5
remain outstanding, not a whole-feature landing blocker. The intended interaction
and specification requirements remain unchanged, and the exception does not mean
all scenarios pass or the change is ready to archive. Focused platform dependencies
are selected above, with untested target behaviour still requiring explicit
qualification. Changing automation defaults, scanning multiple GitHub accounts/hosts,
adding other identity providers, and initiating authentication remain outside this
change.
