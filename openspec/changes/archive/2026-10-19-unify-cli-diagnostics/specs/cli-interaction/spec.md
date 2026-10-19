## ADDED Requirements

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
completed work whose reporting failed. A generic no-files-changed footer MUST NOT
be applied to all failures. Primary and secondary cleanup causes and exact
staging paths MUST remain available with inspection-before-removal guidance.

Deliberate cancellation MUST retain one cancellation report and nonzero exit;
unexpected interrupted I/O MUST NOT be silently suppressed as cancellation.
Warnings MUST retain deduplication and precede dependent failures. Parsing,
validation, filesystem lifecycle, source order, stream/status and terminal/plain
controls MUST remain unchanged.

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

- **WHEN** creation or replacement completes but completion delivery fails
- **THEN** the diagnostic states that the layer manifest was already created or updated
- **AND** it reports the delivery cause without claiming cancellation or no changes

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
