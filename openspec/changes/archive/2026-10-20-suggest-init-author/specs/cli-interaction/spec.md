## ADDED Requirements

### Requirement: Discover guided author names

Guided init MUST start best-effort author discovery once in the background after
successful target preflight, schema readiness, and directory-name default
validation, before Name and without waiting for discovery to complete. It MUST
collect usable candidates from supported
project author declarations, `GIT_AUTHOR_NAME`, applicable Git `author.name` and
`user.name` values, the current local account's display/full name, and its login
username. It MUST NOT stop after the first usable candidate. If the account API
cannot supply a usable login
username, it MUST also collect usable platform-appropriate environment usernames:
`LOGNAME` then `USER` on Unix, or `USERNAME` on Windows. Otherwise it MUST NOT add
environment username alternatives.

Git values MUST be read for the init directory using Git's own configuration and
include handling, with their scopes recorded. Distinct names from applicable
repository-local, global, and other active scopes MUST remain available even when
another scope overrides them. Inactive conditional includes MUST NOT be scanned.
The lookup MUST work without a repository or email address. Presentation order
MUST put project-declared authors before `GIT_AUTHOR_NAME`, Git `author.name`, Git
`user.name`, optional GitHub profile name, account display/full name, account
username, environment username fallbacks, then optional GitHub organisation names.
Project files MUST be ordered `package.json`, `Cargo.toml`, `pyproject.toml`, with
entries in their declaration order. This MUST be a stable tie-break within the
project tier, not a reason to hide other project names. Organisation names MUST
be sorted by normalized name and then login. Within each Git key, normal scope
precedence MUST determine ordering, with later definitions first within a scope.

Discovered candidates MUST have surrounding whitespace trimmed. Empty values,
invalid text encoding, and control characters, including embedded line breaks and
terminal escape sequences, MUST be rejected as discovery input. Remaining
candidates MUST satisfy the maintained layer manifest schema's author definition.
These discovery checks MUST NOT change the schema or transform explicit user
answers. Content validation failures MUST discard only that candidate; internal
schema setup failures MUST remain fatal before prompting or writing.

Names MUST be grouped by exact equality after trimming, with one option per
distinct name. Grouping MUST NOT change case, internal whitespace, or spelling.
Each group MUST retain its first position in presentation order and all distinct
contributing sources in that order. Hints MUST identify project filename/field,
the Git key and scope, account-name/username source, environment variable, or GitHub identity/host as
appropriate. Source records MUST distinguish project declarations,
developer-identity sources, and GitHub membership sources without guessing whether
a name belongs to a person or organisation from its spelling.
Hints MUST be safe single-line display text and MUST
NOT expose full configuration paths or unrelated account details. Source hints
MUST NOT become author metadata.

Discovery MUST NOT require Git, `gh`, authentication, or network access to complete
init. It MUST NOT initiate authentication, read prompt answers, or change identity
settings or project files. Remote requests MUST be limited to the optional GitHub
source specified below; local Git and account lookup MUST remain local. Discovery MUST
NOT infer names from email addresses, hostnames, commit history, or remote URLs.
Unavailable sources, failed reads, malformed configuration, invalid candidates,
and timeouts MUST be silent and non-fatal and MUST NOT discard names from successful
sources. If all sources are unusable, the result MUST contain no names. Discovery
MUST NOT read npm author preferences, `DEBFULLNAME`, generic `NAME`, or Git committer
identity in this increment. Git subprocesses MUST share a 500 ms execution budget, use
bounded output capture and disconnected stdin, and be terminated and reaped on
timeout. Exhausting that budget MUST skip remaining Git subprocesses without
preventing the local-account fallback.

Flag-only execution MUST NOT perform author discovery. This includes any input
flag, explicit non-interactive mode, CI mode, or any non-terminal standard stream.
Omitted author MUST remain absent on that route, and explicit `--author` MUST
retain its existing validation and value-preservation behaviour. Help, version,
argument-usage failures, and failed target preflight MUST NOT perform discovery.

#### Scenario: Git author environment name leads the identity tier

- **WHEN** guided init has no usable project-declared author but has a usable `GIT_AUTHOR_NAME` and other usable identity sources
- **THEN** the trimmed environment author name is the first grouped option
- **AND** discovery also collects usable names from the other sources

#### Scenario: Repository and global names remain available

- **WHEN** `GIT_AUTHOR_NAME` and effective `author.name` are absent, and the repository and global configuration supply different usable `user.name` values
- **THEN** guided init offers both names, with the repository name before the global name and their scopes identified
- **AND** applicable conditional includes and enabled worktree settings are read through Git, without scanning inactive includes

#### Scenario: Author-specific Git configuration wins without requiring email

- **WHEN** no usable `GIT_AUTHOR_NAME` exists and effective `author.name` and `user.name` differ, with no configured email
- **THEN** guided init offers both names with `author.name` before `user.name`

#### Scenario: Global configuration works before Git setup

- **WHEN** init runs outside a repository and global Git `user.name` is usable
- **THEN** guided init includes that name without creating a Git repository

#### Scenario: Collect account alternatives even when Git supplied a name

- **WHEN** Git supplies a name and both the account display/full name and login username are usable
- **THEN** discovery collects all of them, with account values following Git values
- **AND** identical values are grouped rather than repeated

#### Scenario: Environment usernames are a last account fallback

- **WHEN** the account API cannot provide a usable login username
- **THEN** discovery collects usable platform-appropriate environment username values after the other sources
- **AND** a usable account display/full name does not prevent this username fallback

#### Scenario: A successful account username lookup avoids environment alternatives

- **WHEN** the account API provides a usable login username and environment username variables differ
- **THEN** the account username is included and environment usernames are not consulted

#### Scenario: The same name from multiple sources is one choice

- **WHEN** Git local `user.name` supplies ` Alex Morgan ` and the account display name supplies `Alex Morgan`
- **THEN** the selector contains one `Alex Morgan` option whose hint identifies both sources
- **AND** that option retains its Git-priority position and repeated source labels are not duplicated

#### Scenario: Distinct spellings are preserved

- **WHEN** sources supply `Alex Morgan`, `alex morgan`, and `Alex  Morgan`
- **THEN** those remain separate options rather than silently rewriting the names

#### Scenario: Reject unsafe or invalid discovered values

- **WHEN** a discovery candidate is blank after trimming, has invalid encoding or control characters, or fails author content validation
- **THEN** it is not displayed or accepted as a suggestion
- **AND** discovery continues with other candidates without a warning

#### Scenario: Git lookup fails or stalls

- **WHEN** Git is missing, configuration cannot be read, or a Git lookup exceeds the shared execution budget
- **THEN** discovery silently tries the local-account fallback
- **AND** a timed-out Git child is terminated and reaped without reading terminal input
- **AND** any names already found remain available

#### Scenario: No usable identity is available

- **WHEN** all discovery sources are absent, fail, or yield unusable names
- **THEN** the Author text prompt starts empty, with no author selector, warning, or invented author
- **AND** Enter omits author

#### Scenario: Automation does not discover or infer an author

- **WHEN** init selects flag-only execution despite project author files, discoverable local identity, and available authenticated GitHub access
- **THEN** it performs no author lookup and asks no identity questions
- **AND** it does not read project files to discover authors
- **AND** it does not probe for `gh`, inspect its authentication, or make network requests
- **AND** absent `--author` remains absent from the layer manifest
- **AND** a supplied `--author` is validated and preserved without consulting local identity

#### Scenario: Non-executing and rejected invocations do not look up identity

- **WHEN** help, version, an argument-usage failure, or a failed initial target check stops init
- **THEN** no author discovery occurs

### Requirement: Prepare author suggestions during earlier input and bound visible waiting

Eligible guided init MUST start one read-only preparation attempt before displaying
Name, after interaction selection, target preflight, schema readiness, and
directory-name default validation. It MUST NOT await optional discovery before
displaying Name. Preparation MUST continue while Name is answered or retried,
without restarting work or consuming the additional Author-step wait allowance.
Local and optional GitHub source work MUST be able to progress independently;
neither lane MUST wait for the other before beginning. Resource use MUST be
bounded rather than spawning an unbounded worker for every candidate.

Preparation MUST have a 15-second lifetime from its start. Owned external source
operations MUST respect that deadline even if Name remains active. Separately,
entering the Author step MUST start a maximum 1,500 ms additional wait allowance
when preparation is still pending. Work MUST stop being awaited at completion,
the preparation deadline, or the Author deadline, whichever occurs first. Time
before Author MUST NOT count against its additional wait allowance. Requests,
pages, partial results, and Name retries MUST NOT reset either deadline. An
attempt already complete or expired before Author MUST be used immediately,
without a new lookup, artificial delay, or loading indicator. Git's 500 ms source
guard MUST remain independent and MUST NOT extend either effective deadline.

During an actual Author wait, the foreground MUST use a native cliclack loading
indicator on standard error with `Finding author suggestions…`. Its meaning MUST
remain clear without colour or animation. No indicator or background status
messages MUST compete with the active Name prompt. No minimum display duration
MUST delay ready results. The indicator MUST be cleared before displaying Author
or reporting cancellation/failure, without a synthetic completion success or
timeout warning for optional discovery.

Escape and Ctrl-C during this wait MUST cancel the whole guided invocation with
the established nonzero/no-files-changed outcome and one cancellation report,
not skip to Author or emit a duplicate generic error. Cancellation handling MUST
be provided by the foreground wait; drawing/cancelling a spinner MUST NOT be
assumed to detect keyboard cancellation. Only one foreground owner MUST read
terminal input or render prompts/progress at a time. Workers MUST NOT read
terminal answers, print output, render indicators, or change project/identity
files. Unexpected wait/terminal failures MUST remain errors rather than be
treated as unavailable names.

Usable candidates MUST be published incrementally and retained independently of
source completion order. At completion or either deadline, the foreground MUST
close collection and freeze one snapshot before opening Author. Grouping and
source priority MUST apply to this snapshot, not to worker completion order.
Late results MUST NOT add/reorder choices, change the initial selection, reopen
prompts, or alter an accepted/skipped author. Empty snapshots MUST use the shared
optional Author text prompt; partial snapshots MUST retain every already
validated name without waiting for a timed-out source.

The preparation handle MUST cancel unneeded work, prevent new calls/pages, and
terminate/reap owned subprocesses on either deadline, Name cancellation, wait
cancellation, prompt failure, or session exit. Worker shutdown MUST NOT introduce
an unbounded foreground join. Non-interruptible local platform calls MUST be
isolated from the foreground wait with bounded worker count and closed result
publication after cancellation/cutoff; their eventual completion MUST NOT restart
work, print output, or affect prompts. Internal schema setup MUST be checked
before workers or Name, preserving the existing fail-before-prompt guarantee.
Flag-only and early-exit paths MUST start no preparation, timers, or workers.

#### Scenario: Results are ready while Name is being answered

- **WHEN** discovery completes while the developer is answering Name
- **THEN** Name input is not blocked or accompanied by a spinner
- **AND** Author uses the ready snapshot immediately with no loading indicator or extra delay

#### Scenario: Fast Name entry leaves a short residual wait

- **WHEN** the developer reaches Author 200 ms after preparation starts and discovery remains pending
- **THEN** a cliclack loading indicator appears for up to an additional 1,500 ms, unless completion or the lifetime limit occurs first
- **AND** the earlier 200 ms is not deducted from that visible-wait allowance

#### Scenario: Longer background preparation is useful rather than prematurely timed out

- **WHEN** discovery completes after three seconds and the developer spends four seconds answering Name
- **THEN** those results remain available at Author without having been discarded by a 1,500 ms launch-time GitHub timer
- **AND** Author opens without a lookup wait

#### Scenario: The preparation lifetime expires before Author

- **WHEN** preparation remains unfinished for 15 seconds while Name is still active
- **THEN** owned subprocesses are stopped, already validated results are retained, and Name remains usable
- **AND** reaching Author later uses those results immediately without restarting discovery or waiting again

#### Scenario: Partial results survive a visible-wait timeout

- **WHEN** the Author allowance expires while one source remains pending and other sources have supplied names
- **THEN** the indicator is cleared and the frozen partial choices are shown without a warning
- **AND** pending work is cancelled rather than extending the wait

#### Scenario: Source completion races with the cutoff

- **WHEN** a source finishes around the snapshot cutoff
- **THEN** its candidate is included only if published before collection closes
- **AND** any later result cannot change the displayed choices or default

#### Scenario: Name validation retry does not restart preparation

- **WHEN** the developer corrects an invalid Name answer
- **THEN** the same preparation attempt continues within its original lifetime
- **AND** no duplicate lookup or timer reset occurs

#### Scenario: Cancel Name while lookups are active

- **WHEN** the developer cancels at Name with background source operations running
- **THEN** the preparation handle closes, owned subprocesses are terminated/reaped, and no Author prompt, late output, or file creation occurs
- **AND** cancellation retains its single established report and nonzero exit

#### Scenario: Cancel during the visible loading indicator

- **WHEN** the developer presses Escape or Ctrl-C while waiting at Author
- **THEN** the indicator is cleared, pending work is cancelled, and init reports cancellation once and exits nonzero
- **AND** no author fallback prompt or layer manifest is created

#### Scenario: Disabled colour or basic terminal feedback

- **WHEN** a guided Author wait occurs without colour or animation support
- **THEN** the loading message still explains the wait and cancellation remains available
- **AND** the same deadlines, snapshot rules, and cleanup apply

### Requirement: Read project-declared authors before identity suggestions

Guided discovery MUST read directly declared authors from the init directory's
`package.json`, `Cargo.toml`, and `pyproject.toml`, once each, after successful target
preflight and schema readiness. These candidates MUST precede all machine/account
identity and GitHub membership suggestions. All supported files present MUST be
considered; differing declarations MUST remain choices, not cause an error.

Supported author forms MUST be limited to:

- `package.json` `author`: an object with a string `name`, or npm's documented
  person string with a name and optional `<email>` and `(url)` components.
- `Cargo.toml` `[package].authors`: each string in the authors array, with an
  optional trailing `<email>` component.
- `pyproject.toml` `[project].authors`: the string `name` of each author entry.

Documented person-string contact components MUST be removed when extracting the
declared name. Explicit object/table name fields MUST NOT be interpreted as
person-string shorthand. Missing/blank names, email-only entries, malformed or
ambiguous shorthand, and unsupported field types MUST contribute no candidate.
Names MUST NOT be guessed from email addresses. Each accepted author entry MUST
be a separate candidate, never concatenated with other entries. Extracted names
MUST use the same normalization, maintained author-schema validation, and grouping
as every other discovery source. Invalid array entries MUST be skipped independently
without discarding valid entries. Source hints MUST identify the filename and field.

Reads MUST be limited to regular non-symlink files at those exact paths in the
init directory, with a maximum of 1 MiB per file. Missing, unreadable, oversized,
invalidly encoded, or syntactically malformed files MUST be skipped silently
without changing any existing file or discarding candidates from other sources.
Discovery MUST parse the files as data directly, without requiring their package
managers or validating the entire ecosystem project. It MUST NOT execute project
configuration, build backends, package managers, or build tools; search parent,
child, or dependency projects; follow symlinks or workspace references; or resolve
dynamically generated author values.

Discovery MUST NOT read contributors, maintainers, copyright notices, npm author
preferences, `[workspace.package].authors`, inherited Cargo author tables, or
tool-specific Python author fields in this increment. Directly listed Python
author entries MUST remain usable even if a backend could extend them dynamically;
discovery MUST NOT run that backend. Existing deprecated Cargo authors declarations
MUST remain usable as suggestions without updating or warning about the project.

A project-declared author MUST be eligible for initial selection regardless of
whether it names a person, team, or organisation. GitHub membership alone MUST
NOT override that project's attribution. Grouping a project name with GitHub
membership or identity sources MUST retain its project-priority position and
eligibility. The developer MUST still select a single name, enter another, or skip;
discovery MUST NOT silently write every declared author. Flag-only execution and
early-exit paths MUST perform no project-author discovery reads.

#### Scenario: Existing project author precedes current developer identity

- **WHEN** `package.json` declares `Example team` and Git/system/GitHub profile sources name the current developer
- **THEN** `Example team` is listed first and initially selected, with its project source identified
- **AND** the current developer's names remain available as alternatives

#### Scenario: npm author object and shorthand extract names

- **WHEN** a supported `package.json` author is an object with `name` or a person string such as `Alex Morgan <alex@example.com> (https://example.com)`
- **THEN** discovery extracts the declared name without email or URL
- **AND** missing names and email-only author data are not converted into names

#### Scenario: Cargo and Python author arrays offer separate choices

- **WHEN** `[package].authors` or `[project].authors` directly lists several usable names
- **THEN** each distinct extracted name is offered individually in declaration order within its file
- **AND** invalid entries are skipped, valid entries remain, and names are not joined into one author

#### Scenario: Multiple project files disagree

- **WHEN** `package.json`, `Cargo.toml`, and `pyproject.toml` declare different usable authors
- **THEN** all are offered before identity suggestions, in the stable project-file order
- **AND** no declaration is hidden or reported as a conflict

#### Scenario: A project name also appears in other sources

- **WHEN** the same normalized name appears in two project files, Git, and GitHub
- **THEN** one option identifies all contributing sources and retains its earliest project position

#### Scenario: A project explicitly credits an organisation

- **WHEN** a project author field declares an organisation name also returned by GitHub membership lookup
- **THEN** the grouped name remains eligible for initial selection because the project explicitly credits it
- **AND** it is not treated as a membership-only suggestion

#### Scenario: Malformed or unsafe project entries are best-effort failures

- **WHEN** one project file is missing, unreadable, oversized, malformed, invalidly encoded, a symlink, or not a regular file
- **THEN** that file contributes no author and produces no warning
- **AND** other project files and identity sources are still considered without modifying existing files

#### Scenario: Inheritance and unrelated attribution do not trigger broader discovery

- **WHEN** author data is only present in Cargo workspace inheritance, Python dynamic/tool-specific metadata, contributors/maintainers, or parent/child/dependency projects
- **THEN** discovery does not use that data or search/execute anything to resolve it
- **AND** explicitly listed supported author entries remain usable

#### Scenario: Project-author discovery never invokes project tools

- **WHEN** guided init reads project author metadata
- **THEN** no package manager, build backend, project hook, or executable configuration is run
- **AND** absent package-manager executables do not prevent direct parsing or init

### Requirement: Discover optional GitHub names through existing gh authentication

Guided discovery MUST attempt optional read-only GitHub lookup through `gh` when
the executable is available, using only its existing authentication. It MUST use
one host: a non-empty `GH_HOST` when supplied, otherwise `github.com`, passed
explicitly to each request. It MUST use the account/token selected by `gh` for
that host, without enumerating other accounts or hosts, switching accounts,
extracting credentials, inferring a host from repository remotes, or launching
login or permission-refresh commands. Prompts MUST be disabled and stdin
disconnected for child processes.

A successful authenticated-user lookup MUST contribute the profile's usable
display name, falling back to its usable login when the display name is absent or
invalid. Source hints MUST identify the actual account login and host. After a
successful profile lookup, discovery MUST attempt accessible organisation names
for that same authenticated viewer, preferring each organisation's usable display
name and otherwise its usable login. If profile authentication fails, organisation
lookup MUST be skipped. Responses for a different viewer MUST be ignored rather
than mixed into the first account's choices. The free-text profile company field
MUST NOT be treated as an organisation membership.

Organisation lookup MUST fetch names/logins in batches of at most 100 per page,
not issue a separate detail request per organisation. Further pages MUST be
requested only while the shared budget remains. It MUST retain usable complete
records from successful pages when a later page fails or times out. Profile and
organisation candidates MUST use the same normalization, content validation, and
cross-source grouping as local names. Hints MUST distinguish GitHub profile names
from GitHub organisation names and identify organisation logins and host; tokens,
raw API responses, email, and unrelated profile data MUST NOT be displayed or
stored as author metadata.

All `gh` subprocesses and organisation pages MUST share the preparation attempt's
15-second lifetime and, once Author is reached, its additional 1,500 ms wait
deadline. A separate 1,500 ms timer MUST NOT expire GitHub lookup while Name is
being answered. No request or page MUST reset these deadlines. Captured output
MUST be bounded. Timeouts/cancellation MUST terminate and reap the child, with no retry or
persistent response caching. Missing `gh`, missing/expired credentials, offline
operation, API errors, rate limits, insufficient permissions, and token/SSO policy
limitations MUST be silent and non-fatal. They MUST NOT discard usable local or
previously validated GitHub candidates or trigger authentication/permission
questions. An empty organisation result MUST NOT be reported as proof that the
user has no memberships. Flag-only execution MUST NOT attempt any of this lookup.

A group supported solely by GitHub membership MUST NOT be the initial author
selection. Membership MUST NOT be treated as evidence of this layer's authorship
or ownership.
If a grouped value also has a project declaration or developer-identity source,
it MUST remain eligible for initial selection on that independent basis. This
includes project-declared organisations. Only a developer-selected name
MUST be written to the layer manifest; account/organisation membership records
and source hints MUST NOT be persisted there.

#### Scenario: GitHub profile and Git author differ

- **WHEN** the active GitHub profile has a usable name different from the local Git author
- **THEN** both names are offered, with Git before GitHub and each source identified
- **AND** no Git setting or GitHub profile is changed

#### Scenario: GitHub profile name falls back to login

- **WHEN** the authenticated profile has no usable display name but has a usable login
- **THEN** the login is offered as the GitHub personal candidate with account and host identified

#### Scenario: GitHub and local names are grouped together

- **WHEN** the same normalized name is supplied by Git and the authenticated GitHub profile
- **THEN** it appears once with both source hints, retaining its Git-priority position

#### Scenario: Accessible organisations are explicit alternatives

- **WHEN** organisation lookup returns accessible memberships
- **THEN** each usable display name, or login fallback, is offered after project and identity choices with an organisation/login/host hint
- **AND** the profile's free-text company value is not added as another organisation

#### Scenario: Only GitHub membership names are available

- **WHEN** discovery finds usable GitHub organisation names but no project-declared or developer-identity candidate
- **THEN** the selector still lists those organisation names, Enter another author, and Skip
- **AND** Enter another author is initially selected, not an organisation

#### Scenario: A name has both personal and organisation sources

- **WHEN** a GitHub organisation name is identical after normalization to a developer-identity candidate
- **THEN** they are one choice with both sources identified
- **AND** that choice remains eligible for initial selection because of its developer-identity source

#### Scenario: One host and credential selection are honoured

- **WHEN** `GH_HOST` selects a host and `gh` has multiple configured accounts or hosts
- **THEN** requests explicitly target only that host using its currently selected credentials
- **AND** hints identify the returned profile rather than assuming the Git author or querying other accounts

#### Scenario: Default host is explicit

- **WHEN** `GH_HOST` is unset or empty
- **THEN** optional requests explicitly target `github.com` without inferring another host from Git remotes

#### Scenario: Viewer identity changes during lookup

- **WHEN** an organisation response identifies a viewer different from the earlier profile response
- **THEN** that organisation response is ignored rather than mixing accounts
- **AND** earlier valid candidates are preserved

#### Scenario: Missing gh, authentication, or connectivity

- **WHEN** `gh` is unavailable, unauthenticated, has expired credentials, or cannot reach GitHub
- **THEN** init silently retains local choices or uses the shared empty Author text prompt when none exist
- **AND** no login, permission request, warning, or authentication retry occurs

#### Scenario: Organisation access is restricted

- **WHEN** profile lookup succeeds but organisation lookup fails, has insufficient permissions, or returns no memberships
- **THEN** the profile and local choices remain usable without requesting extra permissions
- **AND** no claim is made that the user belongs to no organisations

#### Scenario: A later organisation page fails

- **WHEN** earlier organisation pages supplied usable names and a later page fails or exceeds the shared deadline
- **THEN** earlier names remain available and paging stops without retries
- **AND** the budget is not reset for each page and no per-organisation detail requests are made

#### Scenario: A gh process stalls

- **WHEN** a `gh` process reaches the effective preparation or Author-wait deadline
- **THEN** it is terminated and reaped, no further GitHub processes run, and earlier usable candidates remain available

#### Scenario: Select an organisation without persisting its membership data

- **WHEN** the developer selects a GitHub organisation name and completes guided init
- **THEN** author is that selected name only, without its login/host hint or membership metadata
- **AND** the next prompt after selection is Description

## MODIFIED Requirements

### Requirement: Collect metadata through a guided cliclack session

Guided init MUST use cliclack to collect name, author, description, and layer manifest
format in that order, without input flags. Init MUST NOT combine input flags and
prompt answers within one invocation or require a final confirmation.

Before the first prompt, the session MUST explain once that Enter accepts the
default or skips an optional field. Optional metadata prompts MUST be labelled
as optional. Author and Description text inputs MUST start empty, without helper
placeholders, discovered defaults, or editable prefilling. The name prompt's
placeholder MUST represent its actual default value.

After any bounded preparation wait and indicator cleanup, if the frozen snapshot
contains at least one distinct name, the Author step MUST use
a native cliclack single-choice selector. It MUST list every grouped name with
its source hint, then `Enter another author`, then `Skip`. The first group with a
project declaration or developer-identity source MUST be initially selected,
including a project-declared organisation. If only GitHub-membership groups exist,
`Enter another author` MUST be initially selected instead. Enter MUST accept the
current selection. Choosing a name MUST set
author to that name without opening another author prompt. Multiple author names
MUST NOT be selected or combined. Long lists MUST remain navigable without
discarding names.

Choosing `Skip` MUST omit author and proceed directly to Description, without
opening a text input or asking for confirmation. Choosing `Enter another author`
MUST open the same empty optional Author text prompt used when discovery finds
no names. An empty answer there MUST also omit author and proceed to Description,
without restoring any discovered name or returning to the selector. If discovery
finds no names, init MUST bypass the Author selector entirely and open that shared
text prompt directly. Action identity MUST be separate from candidate name text.
Omission MUST NOT depend on special input tokens or undocumented key combinations.

Before prompts or author preparation, the directory-name default MUST satisfy the
maintained schema's shared lowercase layer-name definition. An invalid default
MUST fail with guidance to supply a valid `--name`, without silently trimming,
lowercasing, or sanitising it. A valid explicit `--name` MUST bypass default
derivation and select flag-only execution rather than author discovery.

The name prompt MUST display the schema-valid current directory's name as a
default and accept it when the user presses Enter without text. A typed name MUST
override that default and satisfy the same schema definition without silent
conversion. Any schema-invalid typed name, including a whitespace-only answer,
MUST receive field-specific inline correction feedback and another opportunity
to answer. Author and description MUST remain arbitrary non-blank strings,
including Unicode and surrounding whitespace, not subject to layer-name syntax.
They MUST be optional independently. An empty text answer MUST skip the
optional field, while a non-empty whitespace-only answer MUST be rejected with
inline feedback. Accepted answers MUST be preserved unchanged, and skipped fields
MUST be absent from the layer manifest rather than written as empty strings or
`null`. The completed author step MUST show the accepted name, not its source
hints or action labels. After an optional field is skipped through an empty
answer or the Author selector's Skip action, its completed prompt MUST show a dimmed,
italic `Skipped` indicator rather than a blank answer. This is presentation only:
it MUST NOT become a default or a stored metadata value, appear before submission,
or mark an accepted name, cancellation, or a format choice as a skip.

The final `Layer manifest format` question MUST be a selector with choices labelled
by the extensions `yaml`, `yml`, `toml`, `json`, and `jsonc`, without a leading dot or
`layer.` prefix, in priority order. `yaml` MUST be initially selected and accepted
by Enter. The completed prompt MUST also show the extension alone.
The selector MUST NOT allow arbitrary filenames or rename the fixed basename.
Format MUST be kept separate from metadata. No file MUST be created until all
guided answers, including format, are complete.

Guided validation feedback MUST identify the current field and explain how to
correct the answer in prompt terms, without referring to metadata flags. For an
optional field, it MUST also explain that clearing the field and pressing Enter
skips it. Guided and flag-only input MUST use the same metadata validation rules;
adapting the message MUST NOT change validation or stored values. Flag-only
validation diagnostics MUST continue to identify the relevant flag without
starting prompts. The meaning of guided feedback MUST remain clear without colour.

Prompt-specific wording MUST be derived from structured schema failures, not a
second validity check or matching human-readable validator messages. The known
non-blank failure MUST receive non-blank correction guidance; other schema rule
failures MUST retain accurate field/path and rule information. Internal schema
setup failures MUST stop init before prompts or writes, without asking the user
to retry valid metadata or mislabelling them as invalid answers.

#### Scenario: Initialise through prompts

- **WHEN** an eligible terminal user runs `hydragen init` without input flags
- **THEN** cliclack asks for name, author, description, and layer manifest format in order
- **AND** only after valid answers, optional skips, and a format choice does init create the layer manifest

#### Scenario: Skip optional fields

- **WHEN** the user answers the name prompt with a valid name and submits an empty optional text answer or chooses Skip in the Author selector
- **THEN** that field is omitted from the layer manifest
- **AND** its completed prompt shows `Skipped` before the next question or final feedback
- **AND** skipping author does not prevent supplying description, or vice versa

#### Scenario: Accept or override the directory-name default

- **WHEN** the name prompt opens
- **THEN** it shows the current directory's name as the default
- **AND** Enter without text accepts that name unchanged
- **AND** typing a valid name uses that answer instead of the default

#### Scenario: One discovered name still offers visible alternatives

- **WHEN** exactly one distinct name eligible for initial selection is found and no other distinct names are found
- **THEN** the selector offers that name, Enter another author, and Skip
- **AND** the discovered name is initially selected

#### Scenario: Invalid directory default stops before author discovery

- **WHEN** guided init would otherwise be eligible but the directory-name default violates the shared name definition, including uppercase or spaces
- **THEN** init fails before prompts, author preparation, or writes with guidance to supply a valid `--name`
- **AND** it does not lowercase, trim, sanitise, or replace the directory name with a discovered author

#### Scenario: Reject an invalid guided name without changing it

- **WHEN** the directory default is valid but the developer types a name such as `Shared-Config` or `café`
- **THEN** schema-derived inline feedback explains how to correct Name without referring to flags
- **AND** the answer remains available for editing rather than being silently converted, no file is written, and author preparation is not restarted

#### Scenario: Preserve arbitrary accepted author and description

- **WHEN** author or description contains Unicode, uppercase, or surrounding whitespace and satisfies its non-blank schema definition alongside a valid layer name
- **THEN** the accepted optional metadata is preserved unchanged rather than subjected to layer-name restrictions
- **AND** normalization of discovered author candidates does not transform explicit author or description input

#### Scenario: Accept the initially selected author

- **WHEN** the user presses Enter on the initially selected author name
- **THEN** that name becomes author, the completed step shows the accepted name rather than `Skipped`, and the next prompt is Description

#### Scenario: Choose a different discovered name

- **WHEN** several distinct names are listed and the user chooses a non-initial name
- **THEN** only the selected name becomes author and the next prompt is Description
- **AND** neither source hints nor other discovered names are stored as author

#### Scenario: Skip directly from the selector

- **WHEN** the user chooses Skip in the Author selector
- **THEN** author is omitted, the completed step shows `Skipped`, and the next prompt is Description
- **AND** no author text input or confirmation is shown

#### Scenario: Enter another author through the shared text prompt

- **WHEN** the user chooses Enter another author and types `Example team` into the next prompt
- **THEN** the same empty optional Author input used for no discovered names collects that value
- **AND** the next prompt is Description and the eventual layer manifest records `Example team`

#### Scenario: Empty custom author still skips

- **WHEN** the user chooses Enter another author and presses Enter without text in the shared Author input
- **THEN** author is omitted, the completed input shows `Skipped`, and the next prompt is Description
- **AND** no discovered name is reapplied and the selector is not shown again

#### Scenario: No names means direct optional text entry

- **WHEN** no usable names are discovered
- **THEN** no Author selector appears and the shared empty optional Author input opens directly
- **AND** typing a valid name supplies author, while Enter without text omits it
- **AND** either submission proceeds to Description

#### Scenario: Candidate names cannot trigger selector actions

- **WHEN** a usable discovered name is literally `Skip` or `Enter another author` and the user selects that name option
- **THEN** it is stored as the selected author name rather than interpreted as an action

#### Scenario: Correct invalid answers

- **WHEN** any typed non-empty metadata answer is whitespace-only
- **THEN** the prompt identifies the field and explains how to supply a non-blank answer without referring to a metadata flag
- **AND** it allows another attempt and creates no file while input remains invalid or incomplete

#### Scenario: Clear an invalid optional answer to skip

- **WHEN** a whitespace-only answer is submitted for the shared Author text input or Description
- **THEN** its feedback explains both supplying a non-blank answer and clearing the field then pressing Enter to skip
- **AND** clearing the input and pressing Enter skips only that field and shows its submitted-skip feedback
- **AND** the layer manifest omits the skipped field after the remaining input is complete

#### Scenario: Flag-only validation retains flag context

- **WHEN** a metadata flag is supplied with an empty or whitespace-only value
- **THEN** its diagnostic identifies that flag and the invocation fails without asking a replacement question
- **AND** it uses compact terminal error styling or plain captured/unattended output as appropriate
- **AND** no files are created

#### Scenario: Guided validation without colour

- **WHEN** guided input is available but terminal colour is disabled and a non-empty whitespace-only answer is submitted
- **THEN** the field and correction or optional-skip instructions remain understandable from the text
- **AND** validation, retry, and no-early-write behaviour are unchanged

#### Scenario: Author choices without colour

- **WHEN** guided input is available without terminal colour and author names are discovered
- **THEN** the optional label, names, grouped source hints, selection, Enter another author, and Skip remain understandable without colour

#### Scenario: Accept the default format

- **WHEN** the user completes the metadata questions and presses Enter on the initial format selection
- **THEN** init creates `layer.yaml` if exclusive creation and writing succeed
- **AND** no format property is added to the layer manifest

#### Scenario: Choose another format without renaming the layer manifest

- **WHEN** the user chooses `yml`, `toml`, `json`, or `jsonc` from the selector
- **THEN** init creates the corresponding fixed `layer.<extension>` filename in that format if creation succeeds
- **AND** the identifying name entered earlier does not alter the filename

#### Scenario: Cancel during either author input route

- **WHEN** the user cancels in the Author selector, the shared Author text input, or a later unfinished prompt
- **THEN** init exits unsuccessfully with the existing cancellation feedback and no layer manifest is created
