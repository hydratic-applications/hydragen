# CLI interaction specification

## Purpose

Define when the CLI can ask questions, how guided init collects metadata and
selects a layer manifest format, and how shared presentation handles statuses,
errors, cancellation, and input failures without blocking automation or creating
unintended files.

## Requirements

### Requirement: Select an interaction mode without blocking automation

The CLI MUST accept global `--non-interactive` before or after the subcommand.
For init, it MUST select flag-only input whenever any of `--name`, `--author`,
`--description`, or `--format` is explicitly supplied, regardless of terminal/CI
state or value validity. Otherwise, init MUST enable guided input only when all
three standard streams are terminals, the explicit non-interactive flag is absent,
and CI is disabled. A non-empty `CI` other than `0` or `false`, compared without
case sensitivity, MUST disable interaction; unset, empty, `0`, and `false` MUST NOT
disable it by themselves.

When interaction is disabled, the CLI MUST NOT prompt, read answers from standard
input, or open another terminal. Init MUST use supplied flags and documented
defaults: omitted name uses the directory name subject to schema validation;
omitted author and description stay absent; omitted format is YAML (`layer.yaml`).
The inferred format MUST NOT count as an input flag. Prompt eligibility MUST
remain separate from compact status styling and colour preferences.

The initial add command MUST require source arguments and remain prompt-free in
every environment. Missing arguments MUST be a clap usage failure, not a guided
flow. `--non-interactive` MUST remain accepted without changing add's required
arguments or disabling otherwise eligible compact status styling.

#### Scenario: Default terminal interaction

- **WHEN** init runs without input flags, with all three standard streams attached to terminals, no `--non-interactive` flag, and CI mode disabled
- **THEN** it enables guided input, including the final format selector

#### Scenario: Metadata flags select flag-only input

- **WHEN** init receives `--name` in an otherwise eligible interactive terminal, with or without optional input flags
- **THEN** it uses only supplied flags and documented defaults without prompts or guided-session presentation
- **AND** omitted optional metadata fields remain absent from the layer manifest
- **AND** eligible compact status styling does not turn it into a guided session

#### Scenario: Optional metadata flags without a name override

- **WHEN** init receives author or description flags without name, even in an eligible interactive terminal
- **THEN** it uses the schema-valid directory default and supplied metadata
- **AND** it does not ask for name, other metadata, or format

#### Scenario: Format alone selects flag-only input

- **WHEN** init receives `--format jsonc` without metadata flags in an otherwise eligible interactive terminal
- **THEN** it uses the schema-valid directory default, omits author and description, and creates `layer.jsonc` if other conditions are satisfied
- **AND** it asks no questions and uses no guided-session presentation

#### Scenario: Explicitly disable interaction in a terminal

- **WHEN** `--non-interactive` is supplied before or after `init`, even with terminal streams
- **THEN** init does not prompt and uses supplied flags, the directory-name default, and the YAML format default when omitted
- **AND** omitted author and description remain absent
- **AND** the flag does not independently disable eligible compact terminal styling

#### Scenario: A standard stream is redirected or closed

- **WHEN** any of standard input, output, or error is not a terminal
- **THEN** init does not prompt or attempt to read answers from another terminal

#### Scenario: CI has a terminal

- **WHEN** init runs without input flags, CI mode is enabled, including by `CI=1` or `CI=true`, and all standard streams are terminals
- **THEN** init still runs non-interactively
- **AND** it uses the directory-name and YAML defaults without asking questions

#### Scenario: CI is explicitly disabled

- **WHEN** `CI` is unset, empty, `0`, or `false` in any letter case
- **THEN** that value alone does not prevent otherwise eligible terminal interaction

#### Scenario: Add never asks for missing sources

- **WHEN** add runs without source arguments in an interactive terminal or automation
- **THEN** clap reports the missing arguments without prompts, layer manifest discovery, or file changes

#### Scenario: Add accepts the global interaction control

- **WHEN** valid local-source arguments are supplied with `--non-interactive` before or after add
- **THEN** add remains argument-only
- **AND** compact status styling follows the shared presentation policy independently

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

### Requirement: Present concise author sources and quiet provenance labels

Active Author choices MUST retain native radio indicators without added candidate
or action icons. Raw names and actions MUST start at the same display column.
Unfocused candidates MUST append a dimmed source-evidence category: `identity`,
`organisation`, `identity + organisation`, or `project`. These MUST use the existing
evidence aggregation without inferring a person's or company's identity from name
spelling. Project evidence MUST neither erase nor invent identity/membership
evidence. Categories MUST use ` · ` as separator, or ` - ` on basic terminals.

The focused candidate MUST instead append a dimmed parenthesised source summary.
Summaries MUST be derived from accepted structured sources, deduplicating display
families in first-source order: `project metadata`, `Git`, `GitHub`, `local account`,
and `GitHub organisation`. Git settings and author environment sources MUST share
`Git`; project files MUST share `project metadata`; account-name/username and
environment username fallbacks MUST share `local account`. Git keys/scopes,
project filenames and account logins MUST NOT inflate these display summaries.
Non-public GitHub hosts MUST retain `on <host>`; public-host matching MUST remain
case-insensitive, with unsafe login/host components rejected before display.
Families MUST be joined with comma-space, without a final `and`.

Complete detailed hints and source distinctions MUST remain internal for grouping,
ordering, safety and eligibility. This display rule supersedes detailed provenance
wording only in the picker, including the other author-discovery requirements'
references to visible hints. It MUST NOT change discovery, grouping, ordering,
initial selection, typed actions, accepted values, omission or cancellation.
Actions MUST show neither category nor summary. Submitted and cancelled answers
MUST omit renderer-added suffixes. Literal names containing action text, icons,
category words or parentheses MUST remain unchanged candidates. Other selectors
MUST remain unchanged. Disabling colour MUST preserve meaningful text.

The renderer MUST NOT truncate text, insert a detail row, or add expansion input.
Long names and remaining long summaries MUST wrap naturally and clear on redraw.

#### Scenario: Focus switches category to source summary
- **WHEN** a project-and-Git-backed name becomes focused
- **THEN** it shows ` (project metadata, Git)` instead of its category
- **AND** unfocused names show their categories and actions remain plain

#### Scenario: Repeated settings collapse without losing evidence
- **WHEN** the same name comes from multiple Git settings, GitHub profile and local-account sources
- **THEN** its summary is `Git, GitHub, local account`
- **AND** all distinct detailed hints, ordering and eligibility remain unchanged

#### Scenario: Project files share one family
- **WHEN** three supported project files declare the same author
- **THEN** the picker shows one `project metadata` summary family

#### Scenario: Organisation and Enterprise context remain distinct
- **WHEN** profile and membership evidence support one name on an Enterprise host
- **THEN** the summary retains separate GitHub and GitHub organisation families with the host
- **AND** the unfocused category is `identity + organisation`
- **AND** unsafe login/host components remain rejected

#### Scenario: Project metadata does not establish entity type
- **WHEN** only project declarations support a company-like or person-like name
- **THEN** its category is `project`, not an inferred entity type

#### Scenario: Colour-disabled and basic terminals preserve text
- **WHEN** NO_COLOR is set or TERM is dumb
- **THEN** categories and summaries stay readable without emoji or added icon slots
- **AND** basic terminals use the ASCII category separator

#### Scenario: Names and actions remain aligned and literal
- **WHEN** a name contains Skip, parentheses, emoji or a category-like suffix
- **THEN** its complete raw name remains aligned with the plain actions
- **AND** accepting it stores only that unchanged name

#### Scenario: Long rows wrap and clear naturally
- **WHEN** a 40-, 60- or 80-column terminal displays a long name or Enterprise host
- **THEN** all summary text wraps naturally without truncation or extra detail rows
- **AND** moving, submitting or cancelling clears stale summaries and categories

#### Scenario: Completion and other selectors remain unchanged
- **WHEN** Author is accepted, skipped or cancelled, or the format selector opens
- **THEN** completed feedback has no renderer-added categories, hints or icons
- **AND** format labels, layout, input and cancellation retain their existing behaviour

### Requirement: Report known target problems before prompting

Before opening any prompt session, init MUST perform read-only checks for the
target directory, the existing repository-location constraints, and occupied paths
at every supported layer manifest filename. Known failures MUST produce informative
errors without asking questions or changing files. A discovery of duplicates MUST
be warned about once before existing-layer-manifest refusal, without reading contents.
Init MUST NOT read or repair an existing layer manifest or offer an overwrite
confirmation.

The initial check MUST NOT reserve the directory. The final write MUST retain
exclusive creation of its exact chosen target but MUST NOT repeat an all-format
refusal check. A different supported format appearing during the questions MUST
NOT prevent creation solely because it appeared. Duplicates observed after a
successful write MUST follow the non-fatal post-creation warning policy.

#### Scenario: Existing layer manifest before the session

- **WHEN** init finds an existing file, directory, or symlink at any supported layer manifest filename, including malformed content or a dangling symlink
- **THEN** it exits unsuccessfully with an informative message before asking for input
- **AND** all existing entries and file contents remain unchanged

#### Scenario: Invalid repository location

- **WHEN** init is run below a detected repository root or an initial target check fails
- **THEN** it reports the location or filesystem problem before asking questions
- **AND** it creates no files

#### Scenario: The chosen path is occupied during the questions

- **WHEN** another process creates an entry at the selected output filename after the initial checks but before the final write
- **THEN** init exits unsuccessfully rather than replacing that entry
- **AND** the other process's entry remains unchanged

#### Scenario: Another supported format appears during the questions

- **WHEN** the initial check finds no supported layer manifest paths, another supported filename appears during the questions, and the chosen output path remains available
- **THEN** init completes exclusive creation and writing without late all-format refusal
- **AND** it warns about duplicates found in its final observation while preserving success

### Requirement: Cancellation and input failure do not initialise a layer

During guided input, including the format selector, init MUST exit with a nonzero
status and a clear cancellation or failure message on Escape, Ctrl-C, unexpected
end of input, or a prompt I/O failure. Init MUST NOT panic, infer missing answers,
print a success message, or create files on these paths. This guarantee applies
even after some or all metadata has already been answered through prompts.
A later failure to render feedback after a successful write MUST NOT be described
as cancellation with no files changed.

Deliberate cancellation MUST use red cancellation styling when colour is enabled,
identify cancellation rather than an unexpected failure, and report that no files
were changed once. Cliclack's cancelled-field state MUST NOT cause a duplicate
application-level cancellation announcement. Unexpected input or I/O failures
MUST retain their distinct cause and red error treatment when styled.

#### Scenario: Cancel an unfinished session

- **WHEN** the user presses Escape or Ctrl-C while answering any metadata prompt or the format selector
- **THEN** init exits unsuccessfully with clear cancellation feedback and one no-files-changed report
- **AND** no layer manifest or other project file is created
- **AND** it neither prints a success nor reports cancellation as an unexpected operational error

#### Scenario: Input ends or a prompt fails

- **WHEN** input ends unexpectedly or a prompt returns an I/O failure before all input, including format, is resolved
- **THEN** init exits unsuccessfully with an informative message and no success output
- **AND** no layer manifest or other project file is created

#### Scenario: Feedback fails after creation

- **WHEN** the layer manifest write succeeds but subsequent interactive feedback fails
- **THEN** the error explains that the layer manifest was already created rather than claiming that no files changed

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

### Requirement: Establish guided init with the Hydragen session identity

After location, occupied-path, and schema setup checks succeed, guided init MUST
show the Hydragen identity once before its questions. It MUST use the selected
dense 16-column by 8-row braille motif from a maintained implementation asset
outside `openspec/`, without stretching or adding spaces between its characters.
The motif grid MUST have no extra left indent; its two leading blank braille
cells provide the visible two-cell inset. There MUST be one ordinary space
between the motif grid and the text, which starts in column 18.

The title on motif row 3 MUST be bold `Hydragen`, followed by two spaces and a
dimmed `v`-prefixed actual executable version. Row 5 MUST show bold, accented
`INIT`. Row 6 MUST show `Initialise a layer` in the normal foreground without
bold or dimmed styling. The motif MUST use the operation accent. When terminal
width is unknown, cannot accommodate the side-by-side composition, or `TERM` is
`dumb`, the opening MUST use text-only product/version and operation context
without a distorted motif or truncated text.

Flag-only init, help, version, argument-usage failures, and preflight failures
MUST NOT show the branded session opening. A question, validation retry,
cancellation, or completion MUST NOT repeat it. A long-running foreground session
can justify this identity in a future command; this requirement MUST NOT add a
watch command or change the existing input-route policy.

#### Scenario: Guided opening at an accommodating width

- **WHEN** guided init passes its preflight checks and the terminal accommodates the selected composition
- **THEN** its stderr opening uses the selected motif, actual version, exact text positions, and styling roles once
- **AND** it proceeds to the existing guided questions without an extra generic intro

#### Scenario: Narrow or basic terminal

- **WHEN** guided init starts in a terminal too narrow for the side-by-side opening or a basic terminal
- **THEN** it uses a readable text-only product/version and operation opening
- **AND** it neither distorts the motif nor truncates the operation description
- **AND** prompt eligibility and input behaviour remain unchanged

#### Scenario: One-shot or rejected invocation

- **WHEN** init is flag-only, help/version is requested, or invocation/preflight checks reject the command
- **THEN** it does not print the branded opening
- **AND** it retains the relevant compact result or diagnostic

### Requirement: Use cliclack selectively with consistent interaction states

Guided init MUST continue using cliclack for editing, selectors, validation
retries, and cancellation rather than a replacement prompt renderer. Branding,
guided input, and output styling MUST be independent choices. Flag-only init
MUST NOT start a prompt or session frame merely to use a feedback component.

The active question MUST use a bold label and accented marker. Submitted values
and connecting rails MUST recede while remaining readable. Optional annotations
MUST be subdued and submitted skips MUST retain their dimmed italic `Skipped`
treatment. The selected format MUST be accented and other choices readable.

Rejected input MUST use the red-cross error pairing (`✕` where supported), with
a red frame and correction message rather than a red warning triangle.
Cancellation MUST use red cancellation feedback (`■` where supported).
Supported cliclack theme hooks MUST make these presentation adjustments without
changing validation, editing, input preservation, or retry behaviour.

#### Scenario: Correct rejected input in place

- **WHEN** a guided metadata answer is rejected
- **THEN** its marker, frame, and correction feedback use the red error treatment
- **AND** the current answer remains available to edit, without a separate command-level failure announcement
- **AND** correcting the answer continues the guided flow with no early layer manifest creation

#### Scenario: Colour-disabled guided interaction

- **WHEN** guided input is eligible but colour is disabled
- **THEN** labels, correction instructions, and cancellation wording retain their meaning without colour
- **AND** guided eligibility, input validation, and cancellation safety do not change

### Requirement: Present parser failures through the shared error style

Root and subcommand parser failures MUST use one shared presentation route, not
per-command style exceptions. Eligible terminal output MUST use the existing red
`✕ Error:` headline and normal-foreground details without a branded opening or
guided-session frame. Plain output MUST retain meaningful `Error:` wording,
usage, and correction information without ANSI or decorative status icons.

Eligibility MUST follow the shared policy: all standard streams are terminals,
CI is disabled, TERM is not dumb, and no nonempty NO_COLOR or CLICOLOR=0 disable
is present. Explicit disables and CI/redirect gates MUST win over forced colour.
Global `--non-interactive` MUST affect questions, not otherwise eligible styling.
Usage errors MUST use stderr and retain clap's nonzero usage status, ordinarily 2.
Operational errors MUST retain their established status and follow the shared
operational diagnostic requirements below.

#### Scenario: Parser errors share terminal presentation

- **WHEN** root, init, or add rejects a missing argument, unknown option/command, invalid value, or conflicting arguments in an eligible terminal
- **THEN** one red cross error headline is shown with readable normal-foreground details
- **AND** stderr carries the failure without prompts, branding, or a stdout result

#### Scenario: Usage failures in automation

- **WHEN** a parser failure occurs with CI, any redirected standard stream, a dumb terminal, or explicit colour disabling
- **THEN** output remains plain even when colour is forced
- **AND** stderr and the nonzero usage status remain unchanged

#### Scenario: Non-interactive terminal error

- **WHEN** a parser failure occurs in an otherwise eligible terminal with global `--non-interactive`
- **THEN** it uses shared compact error styling without asking questions

### Requirement: Explain invalid invocations using clap's command context

Clap MUST remain responsible for parsing and argument validity. The shared
presentation adapter MUST preserve useful argument names, supplied values,
allowed choices, available suggestions, usage, and help hints from its structured
diagnostics rather than replace every failure with a generic message. It MUST NOT
infer error meanings by matching human-readable parser error text or introduce a
second parser. Unsupported error-context shapes MUST still produce a safe useful
fallback rather than panic or accidentally execute the command.

An add invocation without a source MUST clearly explain that at least one source
is required, show relevant usage, one valid explicit-local-path example, and a
hint for `hydragen add --help`. This MUST remain a prompt-free usage failure,
not a full unsolicited help dump, project check, or implicit empty add operation.

#### Scenario: Add has no source

- **WHEN** add is invoked without sources, including with only a global interaction flag
- **THEN** stderr explains the missing source with usage, an example, and an add-help hint
- **AND** it exits nonzero without asking questions or performing layer discovery

#### Scenario: Invalid format retains correction context

- **WHEN** init receives an unsupported format value
- **THEN** the shared error identifies the option and invalid value and retains the supported choices
- **AND** it does not turn the error into a missing-layer or generic failure

#### Scenario: Unknown option retains a useful suggestion

- **WHEN** clap provides a spelling suggestion for an unknown option or command
- **THEN** the shared error retains that suggestion and relevant usage/help context
- **AND** no operation is executed automatically on the user's behalf

### Requirement: Parse arguments before layer discovery and preserve explicit help

All parser failures MUST be handled before current-directory layer discovery,
layer-manifest loading, schema preparation, prompts, or writes. A missing or
invalid layer manifest MUST NOT replace the argument error or produce a duplicate
warning for a command whose syntax was rejected. Only successful operational
parsing MUST allow layer prerequisites to be checked.

Supported explicit help and version invocations MUST remain successful plain
output on their established stream, with no error headline, status icon, branding,
layer discovery, validator setup, prompts, or writes. Existing width-aware help
and accepted help/version forms MUST remain unchanged. A clap help-on-missing-
input failure MUST NOT be treated as a successful explicit help request.

#### Scenario: Missing source outside a layer

- **WHEN** bare add is run where no enclosing layer manifest exists
- **THEN** the missing-source usage error is reported first without searching for a layer

#### Scenario: Valid syntax outside a layer

- **WHEN** add receives a source argument where no enclosing layer manifest exists
- **THEN** normal operational discovery reports the missing current layer manifest
- **AND** it does not report a missing-source usage error

#### Scenario: Help in a directory with invalid duplicate layer manifests

- **WHEN** supported root or subcommand help or version is requested in a directory with invalid or duplicate layer manifest paths
- **THEN** it succeeds with plain help/version output and no layer warnings or errors
- **AND** no files are changed

### Requirement: Keep untrusted parser diagnostic values inert

Untrusted option, argument, supplied-value, and suggestion/context text MUST use
the shared display-only escaping policy before being placed in diagnostic layout.
Raw terminal controls, carriage returns, line feeds, Unicode separator/bidi
controls, and other characters covered by that policy MUST NOT execute or spoof
diagnostic lines. Application-generated layout, width adaptation, and eligible
status styling MUST remain intact. Escaping MUST NOT alter parsing, accepted
argument values, filesystem input, or stored data.

#### Scenario: Unknown option contains terminal controls

- **WHEN** an invalid option or value includes control characters
- **THEN** its diagnostic shows readable escaped text rather than emitting those raw controls
- **AND** generated usage/help line breaks and the shared error style remain intact

#### Scenario: Ordinary Unicode and literal escapes

- **WHEN** a diagnostic value includes ordinary Unicode or literal backslashes
- **THEN** those characters remain understandable and are not repeatedly escaped
- **AND** valid path arguments still reach operational handling unchanged

### Requirement: Normalize parser diagnostics into one optional-block structure

Parser failures MUST use one shared body renderer, not a bespoke missing-source
body alongside framework-formatted bodies. Structured clap facts MUST populate a
presentation model without introducing another argument grammar or interpreting
human-readable framework messages to identify error meanings or command paths.
The sole command-path envelope exception MUST be a narrow match of clap-generated
`Usage: ` metadata against complete trusted canonical command-path boundaries;
semantic error-prose interpretation and hand-written argv scanning remain forbidden.

The body MUST order a consistently punctuated sentence-case headline, optional
context/correction, relevant usage with at most one useful example, and a final
command-specific help hint. Supporting text MUST use a two-space base indent,
one blank line between blocks, and consistent label capitalization including
`Possible values:`, `Tip:`, `Usage:`, and `Example:`. Choice lists MUST NOT use a
separate square-bracket style. Omitted blocks MUST leave no empty labels or filler.
The shared plain/styled routes MUST preserve the same structure and meaning.

Relevant usage and help MUST refer to the erroring command when its identity is
unambiguously verified. Current root/init/add failures MUST be covered, including
value errors for which clap omits Usage context. Unsupported future graph or
context shapes MUST retain explicit unknown ownership and safe general help,
without inventing a root origin or a scoped example. Command-context recovery MUST NOT change accepted arguments,
execute operations, perform layer discovery, or reclassify which parser failure
occurred. Untrusted fields MUST retain the established display escaping. Unknown
context/fallback failures MUST preserve safe useful information without exposing
framework debug names as the ordinary user interface.

Explicit help MUST retain its own plain documentation layout. Operational
failures MUST retain existing safety/state information and MUST NOT acquire
irrelevant parser usage/examples merely to fill a template.

#### Scenario: Missing source and invalid format share organization

- **WHEN** bare add and init with an invalid format produce different error facts
- **THEN** both use the same section order, indentation, punctuation, and label conventions
- **AND** invalid format shows normal `Possible values:` and `Tip:` lines before usage/example and init-specific help
- **AND** missing source omits unnecessary context while retaining its useful add example and add-specific help

#### Scenario: Command context is retained for value failures

- **WHEN** a parser error lacks framework-generated usage context
- **THEN** current supported root/init/add cases still receive the correct command-specific usage/help
- **AND** a value resembling a command or a later command token cannot redirect the diagnostic context
- **AND** ambiguous or unsupported future cases retain unknown ownership and safe general help rather than inventing an origin

#### Scenario: Future error kinds follow the shared contract

- **WHEN** another command or parser failure is added
- **THEN** it uses the same shared layout and family-level style checks
- **AND** content-specific choices, suggestions, or examples do not introduce another rendering template

### Requirement: Explain a missing current layer directly

When add's ancestor search finds no current layer manifest, CLI output MUST use
`No layer manifest found in this directory or its parents.` as the problem, followed
by the tip `Run this command from an existing layer directory, or run 'hydragen init'
in the directory you want to use as a layer.` and `No files were changed.` in
separate blocks. The tip MUST use the `Tip:` label and two supporting lines, with
the second beginning `'hydragen init'`. Supporting text MUST retain the shared
two-space indent and normal foreground.

This single prerequisite diagnostic MUST omit the absolute invocation path,
counted generic add headline, numbered current-layer group, and separate search
paragraph. It MUST remain a nonzero operational stderr failure, without prompts,
writes, or irrelevant usage/examples. Match structured error facts, not prose.

Search behaviour, parser-first priority, shared plain/styled controls, and other
failures' source groups, actionable paths, warning order, and write-state meaning
MUST remain unchanged. The underlying structured missing-current error MUST retain
its invocation path even though this CLI diagnostic omits it.

#### Scenario: Source supplied outside any layer

- **WHEN** add receives a source but no supported layer manifest exists in the invocation directory or any parent
- **THEN** it reports the concise missing-layer headline, the two recovery options, and one no-files-changed statement
- **AND** neither a path paragraph nor a numbered failure wrapper is printed
- **AND** it exits with the existing operational failure status on stderr without creating files

#### Scenario: Other operational failures still identify the affected source

- **WHEN** a selected layer manifest is invalid or supplied dependencies fail verification
- **THEN** their source groups when multiple failures occur, useful diagnostic paths, warning order, and no-change information remain available

### Requirement: Compose all current CLI errors through shared conventions

Root, init and add diagnostics MUST share concrete sentence-case punctuated
problems, optional useful context/recovery, consistent labels and block spacing,
and truthful operation state. Compact diagnostics MUST use shared structured
composition rather than per-command multiline layout strings or interpreted
English `Display` text. Guided field rejection and cancellation MUST retain
cliclack interaction while following the same clarity and terminology rules.

The compact body MUST use a two-space supporting indent, one additional two-space
level for source-group content, one blank line between blocks, and `Tip:` for
actionable correction. Optional blocks MUST omit filler. Parser usage/example/help
MUST retain the existing parser contract and MUST NOT become required operation
sections. Only an eligible headline MUST use error colour; plain output MUST
preserve structure and meaning without decorative status icons.

#### Scenario: Parser and operational failures are reviewed together

- **WHEN** root/init/add parser, prerequisite, verification, edit and reporting errors are rendered
- **THEN** shared conventions govern labels, spacing, indentation, problem and recovery
- **AND** operational failures do not acquire irrelevant usage/examples/help
- **AND** existing help/version and approved missing-current-layer output remain unchanged

#### Scenario: Field-local rejection remains an interaction

- **WHEN** a guided field answer fails schema validation
- **THEN** the existing field-local cliclack rejection identifies the field and a schema-appropriate correction
- **AND** optional-field skip instructions, retry, no-early-write and cancellation behaviour remain intact

### Requirement: Present single failures directly and batches by source

A single operational failure MUST lead with its concrete problem and identify
the affected source or declaration when necessary, without a generic count or
numbered group. Multiple dependency failures MUST use a concise failure summary
and separate source-labelled groups in the existing failure order. Labels MUST
distinguish supplied sources, existing declarations and the current layer.

Groups MUST NOT use redundant outer numbering or ordinary positional prefixes.
An occurrence position MUST be retained only when needed to distinguish otherwise
ambiguous labels. Every independent failure MUST remain represented. The summary
MUST count failures without claiming they are all new dependency additions.

#### Scenario: One supplied directory has no layer manifest

- **WHEN** add verifies a supplied existing directory `../layer-b` with no supported layer manifest
- **THEN** the headline is `No layer manifest found in '../layer-b'.`
- **AND** the tip offers choosing an existing layer directory or running `hydragen init` in `../layer-b`
- **AND** no generic counted headline, numbered source wrapper or redundant absolute path is printed
- **AND** one no-files-changed result is retained

#### Scenario: Existing and supplied failures form one ordered report

- **WHEN** direct existing declarations and supplied sources independently fail
- **THEN** all failures retain their established order and distinct source roles
- **AND** one concise summary and source groups replace redundant numbering
- **AND** duplicate visible labels remain distinguishable without merging failures

### Requirement: Display concise paths without losing source context

CLI diagnostics MUST prefer original supplied or stored reference spelling when
it unambiguously identifies a dependency source. Resolved directory identity MUST
remain separate from reference spelling. Relative sources MUST remain based on
the selected declaring layer manifest's directory, not invocation cwd.

The CLI MUST explain the declaring base when needed to interpret references,
using existing context only. Formatting MUST NOT add filesystem/network work,
rewrite accepted/stored paths, blindly reduce paths to basenames, or edit opaque
error text to shorten paths. Actionable file/field/line locations, conflict
identities, symlink refusal paths, and exact staging/cleanup paths MUST remain.

#### Scenario: Nested invocation uses the declaring directory

- **WHEN** add is invoked below the declaring directory and reports a relative-source failure
- **THEN** the supplied spelling is retained and the resolution base is made clear
- **AND** lookup, storage and discovery rules remain unchanged

#### Scenario: Aliases and genuine conflicts retain identity

- **WHEN** different spellings or symlink aliases name one resolved directory
- **THEN** presentation does not imply they are distinct identities or change no-op handling
- **AND** a duplicate-existing-declaration diagnostic explains the shared identity without repeating the same resolved path
- **WHEN** a genuine name/source conflict is reported
- **THEN** both relevant references and enough resolved context to distinguish them remain available

#### Scenario: Current and dependency symlinks retain their different policies

- **WHEN** the selected current layer manifest is symlinked
- **THEN** refusal retains its actionable path and regular-file guidance
- **WHEN** a dependency layer manifest is a symlink to a regular file
- **THEN** this presentation change does not introduce a refusal

### Requirement: Tailor recovery to typed error facts

Known typed errors MUST provide a concrete problem, relevant cause/location, and
cause-appropriate recovery where useful. Missing layer manifests, nonexistent
directories, decoding errors, schema content failures, internal schema setup,
permission/I/O errors, name conflicts, unsafe edits and replacement failures MUST
NOT all receive one generic recovery paragraph. Internal setup failures MUST NOT
blame valid input or suggest retrying it as correction.

Adapters MUST preserve useful field/instance paths, parser line/column facts,
flags, schema-derived constraints and underlying causes. Untrusted data and
opaque external/unexpected errors MUST use the existing display-only escaping
boundary without changing accepted arguments or stored data. Meaning MUST NOT be
recovered by matching completed English diagnostic strings.

#### Scenario: Invalid source contents need correction rather than initialisation

- **WHEN** a dependency directory contains a selected layer manifest with invalid syntax or content
- **THEN** its file and useful decoder/schema location or constraint are reported
- **AND** recovery addresses that failure rather than recommending initialisation or a fallback file

#### Scenario: Unknown error payload remains safe and useful

- **WHEN** a failure is opaque or outside the known adapter shapes
- **THEN** a clear generic problem preserves useful escaped cause information
- **AND** controls cannot create terminal actions or spoof diagnostic structure

### Requirement: Preserve truthful mutation and reporting state

Diagnostic state MUST come from typed facts and control flow, not message text.
The CLI MUST distinguish no write, possible partial newly created files, failed
replacement with possible staging leftovers, completed creation/replacement, and
completed work whose established fallible completion path returned a write/flush
error. A generic no-files-changed footer MUST NOT be applied to all failures.
Primary and secondary cleanup causes and exact staging paths MUST remain available
with inspection-before-removal guidance.

Deliberate cancellation MUST retain one cancellation report and nonzero exit;
unexpected interrupted I/O MUST NOT be silently suppressed as cancellation.
Warnings MUST retain deduplication and precede dependent failures. Parsing,
validation, filesystem lifecycle, source order, stream/status and terminal/plain
controls MUST remain unchanged. Completion write/flush errors returned by the
established fallible reporting path MUST remain fatal/stateful. Warning/error
diagnostic delivery MUST retain existing best-effort handling without new
propagation or status changes; warning delivery failure MUST NOT change
completed-write status. Successful output calls MUST NOT imply guaranteed receipt.
No descriptor precheck, terminal reopening or rollback is introduced.

#### Scenario: Creation can leave a partial new file

- **WHEN** an init write fails after exclusive creation
- **THEN** the diagnostic does not claim that no files changed
- **AND** it identifies the affected file and preserves the actual failure

#### Scenario: Replacement and cleanup both fail

- **WHEN** replacement fails and staging cleanup also fails
- **THEN** the selected layer manifest is reported as not replaced
- **AND** both causes and the exact staging path are preserved with safe inspection advice
- **AND** the message does not imply no filesystem changes or unconditional deletion safety

#### Scenario: Feedback fails after a completed write

- **WHEN** creation or replacement completes but the established fallible completion write/flush path returns an error
- **THEN** the command exits nonzero and attempts a diagnostic stating that the layer manifest was already created or updated
- **AND** it retains the delivery cause without claiming cancellation or no changes, even if stderr delivery is unavailable

### Requirement: Make new errors inherit the diagnostic contract

Maintained developer guidance MUST define shared composition, path context,
recovery, state and display-trust rules plus a new-error checklist. New known
typed variants MUST receive a deliberate presentation decision where exhaustive
matching is practical, and opaque failures MUST retain a safe fallback.

Family-level tests MUST distinguish universal style/security rules from parser-
specific sections and retain behavioural/filesystem assertions. Review MUST
include representative plain/captured and bounded actual-terminal output,
deterministic rare-state failures, and related messages together. Wording review
MUST assess clarity and all four DX goals rather than rely only on snapshots.

#### Scenario: A future command introduces an error

- **WHEN** a new error family or known error variant is introduced
- **THEN** it uses shared conventions and supplies its problem, source/context, useful recovery and known state
- **AND** representative family coverage and a wording review accompany it
- **AND** it does not invent a competing command-local layout or alter input validity for presentation

### Requirement: Present neutral outcomes using shared informational styling

Compact presentation MUST support informational messages as a reusable shared
style, using blue `ℹ` in eligible terminals without an automatic `Info:` prefix.
Supporting lines MUST remain in the normal foreground. Plain output MUST preserve
the existing meaningful wording without colour, an icon or a new label.

Info MUST represent neutral observations rather than imply a new change. It MUST
NOT by itself change an operation's exit status or output channel. Actual
affirmative results MUST retain success styling; warning/error styling and guided
interaction MUST remain unchanged. Existing terminal, CI, redirection,
colour-disable-over-force and `--non-interactive` rules MUST govern info too.

#### Scenario: Neutral compact message in a terminal

- **WHEN** an informational result is rendered in an eligible terminal
- **THEN** it uses the shared blue info icon and headline, without `Info:`
- **AND** supporting lines remain uncoloured and unbranded

#### Scenario: Informational output in automation

- **WHEN** informational output is captured, redirected, in CI, or explicitly colour-disabled
- **THEN** its text remains unchanged without ANSI, status icon or added label
- **AND** its established stream and status are unchanged even when colour is forced

### Requirement: Use literally empty compact diagnostic separators

Compact diagnostic block separators MUST contain no spaces or tabs in both plain
and styled routes. Supporting and source-group indentation MUST remain intact on
nonempty lines. Presentation MUST NOT trim or rewrite user data to obtain empty
separators, and regression assertions MUST NOT normalise away whitespace-only gaps.

#### Scenario: A compact diagnostic has multiple blocks

- **WHEN** a parser or operational diagnostic separates context, guidance or operation-state blocks
- **THEN** each separating line is empty rather than indentation-only
- **AND** nonempty content retains the shared hierarchy and escaped display data

### Requirement: Bound completion reporting guarantees to the established fallible path

The CLI MUST retain fatal/stateful handling of completion write/flush errors
returned by the established fallible reporting path, including the actual cause.
Warning/error diagnostic delivery MUST retain its existing best-effort behaviour;
warning delivery failure MUST NOT change completed-write status. This clarification
MUST NOT introduce new propagation or status changes for diagnostic writes.
A successful output call MUST NOT be represented as proof that a recipient
received the report; the runtime can accept an unavailable standard stream
without returning an error.

This boundary MUST NOT introduce descriptor prechecks, another terminal, mutation
preflight refusal, output rollback or suppression of completion errors returned
by the established fallible path. Existing stream assignments, warning meanings
and nonzero error statuses MUST remain unchanged.

#### Scenario: The completion path returns BrokenPipe after an update

- **WHEN** replacement succeeds and the established fallible completion write/flush path returns BrokenPipe
- **THEN** the command fails and attempts a diagnostic retaining the updated state and delivery cause
- **AND** it does not roll back or claim that no files changed

#### Scenario: Completion reporting fails after creation or a no-op

- **WHEN** the established fallible completion write/flush path returns an error after creation or an all-repeat add
- **THEN** the command exits nonzero and attempts a diagnostic retaining the actual cause
- **AND** that diagnostic distinguishes already-created from unchanged state without claiming cancellation

#### Scenario: Warning delivery fails after completed creation

- **WHEN** creation completes and best-effort warning delivery encounters a stderr write error
- **THEN** warning delivery does not change completed-write status or introduce new error propagation
- **AND** any completion-reporting failure retains its separate established fallible handling

#### Scenario: The runtime silently accepts an unavailable stream

- **WHEN** the output API returns success despite the standard stream being unavailable
- **THEN** the CLI makes no guaranteed-delivery or rollback claim
- **AND** it does not add platform-specific descriptor checks or an alternative terminal
