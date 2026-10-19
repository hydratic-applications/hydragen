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
as optional; their input areas MUST start empty rather than containing helper
placeholders. The name prompt's placeholder MUST represent its actual default
value.

The name prompt MUST display the current directory's name as a default and accept
it when the user presses Enter without text. A typed name MUST override that
default. The resolved name MUST be non-blank; a typed whitespace-only name MUST
receive inline validation feedback and another opportunity to answer. Author and
description MUST be optional independently; an empty answer MUST skip the field,
while a non-empty whitespace-only answer MUST be rejected with inline feedback.
Accepted values MUST be preserved unchanged, and skipped fields MUST be absent
from the layer manifest rather than written as empty strings or `null`.
After Enter skips an optional field, the completed prompt MUST show a dimmed,
italic `Skipped` indicator rather than a blank answer. This is presentation only:
it MUST NOT become a default or a stored metadata value, appear before submission,
or mark cancellation or a format choice as a skip.

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

- **WHEN** the user answers the name prompt with a valid name and presses Enter without text at an optional metadata prompt
- **THEN** that field is omitted from the layer manifest
- **AND** its completed prompt shows `Skipped` before the next question or final feedback
- **AND** skipping author does not prevent supplying description, or vice versa

#### Scenario: Accept or override the directory-name default

- **WHEN** the name prompt opens
- **THEN** it shows the current directory's name as the default
- **AND** Enter without text accepts that name unchanged
- **AND** typing a valid name uses that answer instead of the default

#### Scenario: Correct invalid answers

- **WHEN** any typed non-empty metadata answer is whitespace-only
- **THEN** the prompt identifies the field and explains how to supply a non-blank answer without referring to a metadata flag
- **AND** it allows another attempt and creates no file while input remains invalid or incomplete

#### Scenario: Clear an invalid optional answer to skip

- **WHEN** a whitespace-only answer is submitted for Author or Description
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

#### Scenario: Accept the default format

- **WHEN** the user completes the metadata questions and presses Enter on the initial format selection
- **THEN** init creates `layer.yaml` if exclusive creation and writing succeed
- **AND** no format property is added to the layer manifest

#### Scenario: Choose another format without renaming the layer manifest

- **WHEN** the user chooses `yml`, `toml`, `json`, or `jsonc` from the selector
- **THEN** init creates the corresponding fixed `layer.<extension>` filename in that format if creation succeeds
- **AND** the identifying name entered earlier does not alter the filename

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
