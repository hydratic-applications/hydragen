## ADDED Requirements

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
- **AND** correcting the answer continues the guided flow with no early manifest creation

#### Scenario: Colour-disabled guided interaction

- **WHEN** guided input is eligible but colour is disabled
- **THEN** labels, correction instructions, and cancellation wording retain their meaning without colour
- **AND** guided eligibility, input validation, and cancellation safety do not change

## MODIFIED Requirements

### Requirement: Select an interaction mode without blocking automation

The CLI MUST accept a global `--non-interactive` flag before or after the
subcommand. For init, it MUST select flag-only initialisation whenever any of
`--name`, `--author`, `--description`, or `--format` is explicitly supplied,
regardless of terminal or CI state and even when a supplied value is invalid.
Otherwise, it MUST enable guided interaction by default only when standard input,
output, and error are all terminals, the explicit non-interactive flag is absent,
and CI mode is disabled. A non-empty `CI` value other than `0` or `false`, compared
without case sensitivity, MUST disable interaction; unset, empty, `0`, and `false`
values MUST NOT disable it by themselves.

When interaction is disabled, the CLI MUST NOT run prompts, read answers from
standard input, or open another terminal for input. It MUST use supplied flags
and documented defaults; omitted name MUST default to the current directory's
name, omitted author and description MUST remain absent, and omitted format MUST
default to YAML with filename `layer.yaml`. An inferred format default MUST NOT
count as an explicitly supplied input flag. Prompt eligibility MUST be separate
from compact status styling and MUST NOT be enabled by colour preferences.

#### Scenario: Default terminal interaction

- **WHEN** init runs without input flags, with all three standard streams attached to terminals, no `--non-interactive` flag, and CI mode disabled
- **THEN** it enables guided input, including the final format selector

#### Scenario: Metadata flags select flag-only input

- **WHEN** init receives `--name` in an otherwise eligible interactive terminal, with or without optional input flags
- **THEN** it uses only supplied flags and documented defaults without prompts or guided session presentation
- **AND** omitted optional metadata fields remain absent from the manifest
- **AND** eligible compact status styling does not turn it into a guided session

#### Scenario: Optional metadata flags without a name override

- **WHEN** init receives author or description flags without name, even in an eligible interactive terminal
- **THEN** it uses the current directory's name and supplied metadata
- **AND** it does not ask for name, other metadata, or format

#### Scenario: Format alone selects flag-only input

- **WHEN** init receives `--format jsonc` without metadata flags in an otherwise eligible interactive terminal
- **THEN** it uses the directory's name, omits author and description, and creates `layer.jsonc` if other conditions are satisfied
- **AND** it asks no questions and uses no guided session presentation

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

### Requirement: Collect metadata through a guided cliclack session

Guided init MUST use cliclack to collect name, author, description, and manifest
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
from the manifest rather than written as empty strings or `null`.
After Enter skips an optional field, the completed prompt MUST show a dimmed,
italic `Skipped` indicator rather than a blank answer. This is presentation only:
it MUST NOT become a default or a stored metadata value, appear before submission,
or mark cancellation or a format choice as a skip.

The final Manifest format question MUST be a selector with choices labelled by
the extensions `yaml`, `yml`, `toml`, `json`, and `jsonc`, without a leading dot or
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
- **THEN** cliclack asks for name, author, description, and manifest format in order
- **AND** only after valid answers, optional skips, and a format choice does init create the manifest

#### Scenario: Skip optional fields

- **WHEN** the user answers the name prompt with a valid name and presses Enter without text at an optional metadata prompt
- **THEN** that field is omitted from the manifest
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
- **AND** the manifest omits the skipped field after the remaining input is complete

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
- **AND** no format property is added to the manifest

#### Scenario: Choose another format without renaming the manifest

- **WHEN** the user chooses `yml`, `toml`, `json`, or `jsonc` from the selector
- **THEN** init creates the corresponding fixed `layer.<extension>` filename in that format if creation succeeds
- **AND** the identifying name entered earlier does not alter the filename

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
- **AND** no manifest or other project file is created
- **AND** it neither prints a success nor reports cancellation as an unexpected operational error

#### Scenario: Input ends or a prompt fails

- **WHEN** input ends unexpectedly or a prompt returns an I/O failure before all input, including format, is resolved
- **THEN** init exits unsuccessfully with an informative message and no success output
- **AND** no manifest or other project file is created

#### Scenario: Feedback fails after creation

- **WHEN** the manifest write succeeds but subsequent interactive feedback fails
- **THEN** the error explains that the manifest was already created rather than claiming that no files changed
