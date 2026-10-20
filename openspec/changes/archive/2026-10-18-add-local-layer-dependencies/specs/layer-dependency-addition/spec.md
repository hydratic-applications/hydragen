## ADDED Requirements

### Requirement: Add local dependencies through a batch-capable argument-only command

The CLI MUST provide `hydragen add <source> [<source> ...]`, requiring at least one
source argument and supporting batches. It MUST only declare verified local
dependencies in the current layer manifest. It MUST NOT prompt, fetch or copy
dependency content, render a workspace, capture contributions, mutate Git,
generate a lockfile, or implement publishing or registry lookup.

A supplied published reference MUST produce a clear unsupported-feature error
rather than be added without verification. Missing arguments and help/version
handling MUST use clap without discovering layer manifests or changing files.
The global non-interactive control MUST remain accepted.

#### Scenario: Add multiple local sources

- **WHEN** a developer supplies two valid local paths and all current-layer, existing-dependency, and batch checks succeed
- **THEN** add records the new references through one successful layer manifest update
- **AND** leaves dependency sources, project content, Git, and workspace state unchanged

#### Scenario: Published addition is deferred

- **WHEN** a batch includes `shared-config@1.2.0`
- **THEN** add explains that published-layer addition is not supported yet
- **AND** does not contact a registry or change the layer manifest

#### Scenario: Help and missing arguments do not inspect layers

- **WHEN** add help, version handling, or an argument-usage failure is processed
- **THEN** the CLI does not discover layer manifests, warn about duplicates, or prompt

### Requirement: Discover the nearest enclosing layer without a Git boundary

Add MUST inspect the invocation directory and then its parents through the
filesystem root using shared single-directory discovery. It MUST stop at the
nearest directory with any occupied supported layer manifest filename and use
the established priority. An invalid selection MUST fail there, not trigger a
lower-priority file or parent-directory fallback. An unexpected inspection error
MUST fail rather than be treated as absence. Absence throughout the search MUST
produce an actionable missing-layer-manifest error.

Git roots MUST NOT stop this search. Add MUST NOT change init's current-directory
repository guard or make shared single-directory discovery search ancestors.
Known symlinked current layer manifests MUST be refused before editing rather
than replaced or followed for writing.

#### Scenario: Nested invocation selects an enclosing layer

- **WHEN** add runs below a directory with a supported layer manifest and no nearer directory has an occupied supported name
- **THEN** it selects that enclosing layer, including across an intervening Git boundary

#### Scenario: Nearest layer is invalid

- **WHEN** the nearest matching directory has an invalid selected layer manifest and a parent has a valid one
- **THEN** add fails on the nearest selection without editing the parent

#### Scenario: An occupied selected name is not a readable regular file

- **WHEN** a directory, dangling symlink, or other non-regular entry takes priority over a valid lower-priority layer manifest
- **THEN** add fails on the selected entry without fallback or waiting on special-file input

#### Scenario: Current layer manifest is a symlink

- **WHEN** the selected current layer manifest is a symlink to a regular file
- **THEN** add refuses editing without replacing the link or changing its target

### Requirement: Resolve local sources from the declaring layer directory

Relative command arguments and stored local references MUST resolve from the
selected current layer manifest's directory, not the invocation directory. Add
MUST preserve accepted supplied path strings for storage, including relative or
absolute form, redundant components, spaces, and spelling. Internal resolution
MUST NOT imply rewriting the stored reference.

Source classification MUST be syntax-based. Add MUST support explicit native
relative and absolute forms, including standalone `.` and `..`, and diagnose
recognisable foreign or ambiguous path forms without treating them as published
references or translating them. Bare names MUST NOT be inferred as local paths.
The CLI MUST NOT expand literal home or environment syntax; input already
expanded by a shell is simply the received argument.

#### Scenario: Relative arguments have a stable base

- **WHEN** the current layer manifest is `/work/app/layer.yaml`, invocation is from `/work/app/src`, and the supplied path is `../shared-config`
- **THEN** add verifies `/work/shared-config`
- **AND** stores `../shared-config` unchanged

#### Scenario: Absolute references remain absolute

- **WHEN** an explicit absolute local path passes verification
- **THEN** add retains that reference rather than converting it to a relative path

#### Scenario: Missing local source does not become a registry source

- **WHEN** an explicit path does not resolve to a supported local target
- **THEN** add reports a path failure without registry fallback

#### Scenario: Bare source matches a directory by coincidence

- **WHEN** add receives bare `shared-config` and a directory with that name exists
- **THEN** it explains the explicit local-path requirement instead of inspecting that directory as the source

#### Scenario: Unsupported platform path

- **WHEN** a source uses a recognisable path form that the host cannot resolve or an ambiguous drive-relative form
- **THEN** add reports a path-specific unsupported-form error rather than a published-layer lookup

### Requirement: Share bounded read-only direct-dependency verification

The library MUST expose reusable, read-only checks for the prerequisites used by
add, independently of terminal rendering and mutation. Add MUST load and validate
the current layer manifest, every existing direct local dependency, and each
supplied local target before editing. Each target MUST be a directory with a
supported layer manifest in that directory. Target verification MUST use shared
discovery, loading, parsing, and the maintained schema; filename presence alone
MUST NOT be sufficient, and target ancestors MUST NOT be searched.

Existing published declarations MUST be validated offline for name/version syntax
and included in name checks without claiming existence. The checks MUST NOT
traverse targets' dependency chains or implement a validate CLI. Loading a target
MUST still validate its own declaration syntax. Structured results MUST preserve
source/content paths, relationship failures, and discovery information needed for
warnings, including when loading fails.

#### Scenario: Target has no layer manifest

- **WHEN** a supplied directory has no supported layer manifest but its parent does
- **THEN** verification fails for that target without borrowing the parent's layer manifest

#### Scenario: Target layer manifest is malformed or schema-invalid

- **WHEN** the selected target layer manifest cannot be read, parsed, or validated
- **THEN** add reports the target-specific failure without modifying the current layer manifest

#### Scenario: Existing local dependency is broken

- **WHEN** an already-declared direct local dependency is missing or invalid
- **THEN** add reports the failure and makes no update
- **AND** does not infer a name or bypass uniqueness checking to add other sources

#### Scenario: Existing published declaration does not require network access

- **WHEN** the current layer manifest includes a syntactically valid exact published reference
- **THEN** verification uses its declared name without network access
- **AND** does not claim that the published layer exists

#### Scenario: Direct verification does not resolve a dependency chain

- **WHEN** a target's layer manifest has valid content but its own local dependency is unavailable
- **THEN** add does not follow that target's dependency to verify its existence
- **AND** does not claim that the full dependency graph is valid

#### Scenario: Other operations reuse verification

- **WHEN** a library caller requests the direct-relationship checks without a terminal or edit
- **THEN** it receives the same structured validity results without writes or presentation side effects

### Requirement: Enforce unique declared names without implicit replacement

The current layer's name MUST be reserved. All existing direct dependency names
MUST be unique across local and published references. Names MUST come from loaded
local layer manifests or validated published-reference syntax, not supplied aliases
or path basenames. Name validity MUST use the shared lowercase policy, not a second
normalisation scheme. Existing duplicate declarations MUST fail even when their
references identify the same source.

For supplied local sources, matching a declared name from the same resolved
directory MUST be a harmless repeat. A different path spelling or directory
symlink identifying that directory MUST NOT replace the stored reference.
Matching names from different directories, collisions with published references,
and names equal to the current layer's name MUST fail the entire batch without
source replacement. Repeats within a new batch MUST retain the first occurrence.

#### Scenario: Re-add an existing local source

- **WHEN** a supplied source has the same name and resolved directory as an existing local dependency
- **THEN** it is already present and its original stored reference remains unchanged

#### Scenario: Alternate paths identify the same local directory

- **WHEN** relative, absolute, or directory-symlink references resolve to the same named local directory
- **THEN** an existing declaration is not duplicated or rewritten

#### Scenario: Two directories declare the same name

- **WHEN** existing or supplied local sources have the same name but resolve to different directories
- **THEN** add rejects the batch with a conflict diagnostic and changes no declarations

#### Scenario: Current layer name is claimed by a dependency

- **WHEN** a dependency source declares the current layer's name, including a different directory
- **THEN** verification rejects the self-name conflict

#### Scenario: Existing declarations already conflict

- **WHEN** existing entries repeat a dependency name, including local/published collisions or different published versions
- **THEN** add identifies the conflicting declarations and refuses to edit
- **AND** does not silently remove or replace an entry

#### Scenario: New local source collides with an existing published name

- **WHEN** a new local target declares the name of an existing published dependency
- **THEN** add rejects the source switch rather than replacing the published reference

### Requirement: Verify the entire batch before appending in order

Add MUST establish existing-layer validity and verify the complete supplied batch
before a write. Independent source failures MUST be collected and reported in
argument order where meaningful. A prerequisite failure MUST NOT be followed by
speculative checks that require unavailable information. Any verification error
MUST prevent all additions.

The successful edit plan MUST append only genuinely new references in
first-appearance argument order and preserve all existing array entries and order.
An all-repeat batch MUST succeed without rewriting or staging a layer manifest.

#### Scenario: One member of a batch fails

- **WHEN** two supplied sources verify and a third is invalid
- **THEN** no reference from the batch is saved

#### Scenario: Independent failures are collected

- **WHEN** one supplied target is missing and another has malformed content
- **THEN** the CLI reports both failures in argument order rather than only the first
- **AND** does not update the current layer manifest

#### Scenario: New and repeated arguments preserve intended order

- **WHEN** a batch requests new source A, new source B, then A again
- **THEN** the edit appends A and B in that order, once each
- **AND** A is classified only as added, not already present

#### Scenario: All requested dependencies were present before invocation

- **WHEN** every requested source is a harmless repeat and all checks pass
- **THEN** add succeeds without rewriting the file, altering its formatting, or creating a staging file

### Requirement: Preserve comments and unrelated content when editing

Add MUST retain the selected layer manifest format and preserve comments in
YAML/YML, TOML, and JSONC, including existing dependency comments. It MUST preserve
metadata and unknown decoded data, and source text outside the edited dependency
declaration. It MUST NOT sort existing declarations, convert formats, broadly
reserialise the document, or discard comments as a fallback.

Before writing, the prepared source MUST be parsed with the established format
rules, validated with the maintained schema, and checked to preserve decoded
content except for the intended dependency append. If an edit cannot safely
preserve comments or unrelated meanings, it MUST fail with a specific explanation
without changing the original. This refusal MUST NOT replace ordinary editing
support for standard arrays and comments in every supported format.

#### Scenario: Comments survive a dependency append

- **WHEN** a supported commented layer manifest contains metadata, dependency comments, and unrelated custom fields
- **THEN** adding a valid source preserves those comments, unrelated source text, and decoded data
- **AND** adds the reference without changing format

#### Scenario: Dependencies are absent or empty

- **WHEN** a valid supported layer manifest omits dependencies or has an empty dependency array
- **THEN** add inserts or extends the declaration without removing unrelated text or comments

#### Scenario: Alias editing would change unrelated meaning

- **WHEN** a YAML anchor/alias arrangement cannot be edited without affecting unrelated decoded data or losing comments
- **THEN** add explains the unsupported edit and leaves the source unchanged
- **AND** does not use generic reserialisation as a fallback

#### Scenario: Prepared output fails validation

- **WHEN** the planned edited source fails parsing, schema validity, or intended-change verification
- **THEN** the update stops before opening a staging file or changing the layer manifest

### Requirement: Replace the layer manifest through hidden sibling-file staging

For actual additions, add MUST write a prepared candidate to a hidden temporary
file beside the selected layer manifest, preserve the original file permissions,
synchronise the staged file, and atomically replace the original. It MUST NOT
truncate the original in place or fall back to non-atomic copying. The original
MUST remain intact on handled failures before successful replacement.

The command MUST NOT announce staging during normal operation. It MUST attempt
cleanup on success and every handled failure, and accurately report a cleanup
failure if cleanup is impossible. Abrupt termination can leave a hidden staging
file in the repository; unconditional cleanup or power-loss durability MUST NOT
be promised. The replacement MUST NOT be described as preserving every filesystem
metadata attribute beyond the specified permissions.

This increment MUST NOT introduce locking, stale-content checks, or automatic
concurrent-edit merges. Concurrent writers can lose updates despite atomic
replacement. Documentation MUST state that limitation rather than claim full
transactional safety against other processes.

#### Scenario: Prepared update completes

- **WHEN** staging, permission preservation, synchronisation, and replacement succeed
- **THEN** the selected layer manifest contains the complete prepared update
- **AND** no staging file remains or is announced in normal output

#### Scenario: Staging or replacement fails

- **WHEN** a handled error occurs before replacement succeeds
- **THEN** the original layer manifest remains unchanged
- **AND** cleanup is attempted, with any cleanup failure reported accurately

#### Scenario: Process terminates before cleanup

- **WHEN** abrupt termination prevents normal cleanup
- **THEN** a hidden sibling staging file can remain as a documented limitation
- **AND** no requirement claims that it cannot persist in the repository

### Requirement: Report actual additions separately from pre-existing dependencies

Successful add MUST exit zero and use one compact stdout result naming the
destination layer. It MUST list requested dependency names separately under
`Added` and `Already present`, with counts, omitting empty groups. Each dependency
MUST appear once in its outcome group; each group MUST preserve first-appearance
argument order. `Already present` MUST mean present before this invocation, not
a later repeat of a newly added source. Unrequested existing dependencies MUST
NOT be included. All-repeat execution MUST report no changes needed and list
the requested already-present dependencies.

The layer manifest path MUST NOT appear by default in success feedback. Add MUST
NOT show a full branded opening, prompt/session frame, or per-source success
messages. Eligible terminal statuses MUST use shared styling, and plain output
MUST retain meaningful text without ANSI or status icons. Warnings and errors
MUST use stderr. CI, redirected streams, colour disables, and `--non-interactive`
MUST follow the shared presentation policy independently of prompt eligibility.

#### Scenario: Mixed additions and harmless repeats

- **WHEN** two requested sources are new and one was already declared
- **THEN** success names the destination layer and lists two names under `Added (2)` and one under `Already present (1)`
- **AND** does not print the layer manifest path or unrelated dependency names

#### Scenario: Only one outcome group is populated

- **WHEN** all requested dependencies are new or all were already present
- **THEN** only the relevant nonempty group is shown
- **AND** an all-repeat result describes no changes rather than a completed update

#### Scenario: Plain automation output

- **WHEN** add runs with redirected streams or CI enabled
- **THEN** stdout retains the destination name, group labels, counts, and dependency names without ANSI or decorative status icons
- **AND** warnings/errors remain on stderr

### Requirement: Present actionable grouped errors without misreporting writes

Usage, discovery, validation, edit-preparation, and write failures MUST exit
nonzero with accurate stderr diagnostics. Independent batch verification errors
MUST use a readable command-level headline and source-specific groups in argument
order, explaining causes and correction. Diagnostic paths MUST remain available
where needed; source-resolution errors MUST make the declaring-directory base
clear when relevant.

Duplicate layer manifest warnings MUST use shared discovery results and appear
at most once per affected directory per invocation, before a dependent failure,
even when several references resolve to that directory. Warning alone MUST NOT
cause failure.

Feedback MUST distinguish failures before replacement from failures reporting a
completed replacement. It MUST NOT claim success before replacement or claim no
files changed after a completed update or a known cleanup failure. If completion
reporting fails after replacement, it MUST explain that the layer manifest was
already updated. Diagnostics MUST NOT recommend unimplemented upgrade or registry
commands as though they exist.

#### Scenario: Verification failure has no update

- **WHEN** independent source checks fail before staging
- **THEN** stderr groups the failures with guidance and accurately reports no changes
- **AND** stdout contains no success result

#### Scenario: Repeated target discovery has duplicates

- **WHEN** several references identify one directory with multiple layer manifests
- **THEN** stderr warns once for that directory, naming candidates and selection in priority order
- **AND** the warning precedes any failure depending on its selected layer manifest

#### Scenario: Completion output fails after replacement

- **WHEN** replacement completes but writing completion feedback fails
- **THEN** the resulting error explains that the layer manifest was already updated
- **AND** does not report cancellation or no files changed
