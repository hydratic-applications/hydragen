# CLI interaction specification

## Purpose

Define when the CLI can ask questions, how guided init collects and presents
metadata, and how target errors, cancellation, and input failures are handled
without blocking automation or creating unintended files.

## Requirements

### Requirement: Select an interaction mode without blocking automation

The CLI MUST accept a global `--non-interactive` flag before or after the
subcommand. For init, it MUST select flag-only initialisation whenever any of
`--name`, `--author`, or `--description` is supplied, regardless of terminal or CI
state and even when a supplied value is invalid. Otherwise, it MUST enable guided
interaction by default only when standard input, output, and error are all
terminals, the explicit flag is absent, and CI mode is disabled. A non-empty `CI`
value other than `0` or `false`, compared without case sensitivity, MUST disable
interaction; unset, empty, `0`, and `false` values MUST NOT disable it by themselves.

When interaction is disabled, the CLI MUST NOT run prompts, read answers from
standard input, or open another terminal for input. It MUST use supplied flags
and documented defaults; omitted name MUST default to the current directory's
name, while omitted author and description MUST remain absent.

#### Scenario: Default terminal interaction

- **WHEN** init runs without metadata flags, with all three standard streams attached to terminals, no `--non-interactive` flag, and CI mode disabled
- **THEN** it enables interactive metadata collection

#### Scenario: Metadata flags select flag-only input

- **WHEN** init receives `--name` in an otherwise eligible interactive terminal, with or without optional metadata flags
- **THEN** it uses only supplied flags without prompts or cliclack presentation
- **AND** omitted optional fields remain absent from the manifest

#### Scenario: Optional flags without a name override

- **WHEN** init receives author or description flags without name, even in an eligible interactive terminal
- **THEN** it uses the current directory's name and the supplied metadata
- **AND** it does not ask for name or other metadata

#### Scenario: Explicitly disable interaction in a terminal

- **WHEN** `--non-interactive` is supplied before or after `init`, even with terminal streams
- **THEN** init does not prompt and uses supplied flags and the directory-name default
- **AND** omitted author and description remain absent

#### Scenario: A standard stream is redirected or closed

- **WHEN** any of standard input, output, or error is not a terminal
- **THEN** init does not prompt or attempt to read answers from another terminal

#### Scenario: CI has a terminal

- **WHEN** init runs without metadata flags, CI mode is enabled, including by `CI=1` or `CI=true`, and all standard streams are terminals
- **THEN** init still runs non-interactively
- **AND** it uses the current directory's name without asking questions

#### Scenario: CI is explicitly disabled

- **WHEN** `CI` is unset, empty, `0`, or `false` in any letter case
- **THEN** that value alone does not prevent otherwise eligible terminal interaction

### Requirement: Collect metadata through a guided cliclack session

Guided init MUST use cliclack to collect name, author, and description in that
order, without metadata flags. Init MUST NOT combine metadata flags and prompt
answers within one invocation or require a final confirmation.

Before the first prompt, the session MUST explain once that Enter accepts the
default or skips an optional field. Optional prompts MUST be labelled as optional;
their input areas MUST start empty rather than containing helper placeholders.
The name prompt's placeholder MUST represent its actual default value.

The name prompt MUST display the current directory's name as a default and accept
it when the user presses Enter without text. A typed name MUST override that
default. The resolved name MUST be non-blank; a typed whitespace-only name MUST
receive inline validation feedback and another opportunity to answer. Author and
description MUST be optional independently; an empty answer MUST skip the field,
while a non-empty whitespace-only answer MUST be rejected with inline feedback. Accepted
values MUST be preserved unchanged, and skipped fields MUST be absent from the
manifest rather than written as empty strings or `null`.
After Enter skips an optional field, the completed prompt MUST show a dimmed, italic
`Skipped` indicator rather than a blank answer. This is presentation only: it
MUST NOT become a default or a stored metadata value, appear before submission,
or mark cancellation as a skip.

#### Scenario: Initialise through prompts

- **WHEN** an eligible terminal user runs `hydragen init` without metadata flags
- **THEN** cliclack asks for name, author, and description in order
- **AND** only after valid answers or optional skips does init create the manifest

#### Scenario: Skip optional fields

- **WHEN** the user answers the name prompt with a valid name and presses Enter without text at an optional prompt
- **THEN** that field is omitted from the manifest
- **AND** its completed prompt shows `Skipped` before the next question or final feedback
- **AND** skipping author does not prevent supplying description, or vice versa

#### Scenario: Accept or override the directory-name default

- **WHEN** the name prompt opens
- **THEN** it shows the current directory's name as the default
- **AND** Enter without text accepts that name unchanged
- **AND** typing a valid name uses that answer instead of the default

#### Scenario: Correct invalid answers

- **WHEN** any typed non-empty answer is whitespace-only
- **THEN** the prompt explains the validation problem and allows another attempt
- **AND** no file is created while answers remain invalid or incomplete

### Requirement: Report known target problems before prompting

Before opening any prompt session, init MUST perform read-only checks for the
target directory, the existing repository-location constraints, and an existing
entry at `layer.json`. Known failures MUST produce informative errors without
asking questions or changing files. Init MUST NOT read or repair an existing
manifest or offer an overwrite confirmation. The final write MUST retain
exclusive creation even after the checks pass.

#### Scenario: Existing manifest before the session

- **WHEN** init finds an existing file, directory, or symlink at `layer.json`, including a malformed manifest or dangling symlink
- **THEN** it exits unsuccessfully with an informative message before asking for metadata
- **AND** all existing entries and file contents remain unchanged

#### Scenario: Invalid repository location

- **WHEN** init is run below a detected repository root or a target check fails
- **THEN** it reports the location or filesystem problem before asking questions
- **AND** it creates no files

#### Scenario: A manifest appears during the questions

- **WHEN** another process creates `layer.json` after the initial checks but before the final write
- **THEN** init exits unsuccessfully rather than replacing that entry
- **AND** the other process's file remains unchanged

### Requirement: Cancellation and input failure do not initialise a layer

During metadata collection, init MUST exit with a nonzero status and a clear
cancellation or failure message on Escape, Ctrl-C, unexpected end of input, or a
prompt I/O failure. Init MUST NOT panic, infer missing answers, print a success
message, or create files on these paths. This guarantee applies even after some
metadata has already been answered through prompts. A later failure to render
feedback after a successful write MUST NOT be described as cancellation with no
files changed.

#### Scenario: Cancel an unfinished session

- **WHEN** the user presses Escape or Ctrl-C while answering any metadata prompt
- **THEN** init exits unsuccessfully with a clear cancellation message
- **AND** no manifest or other project file is created

#### Scenario: Input ends or a prompt fails

- **WHEN** input ends unexpectedly or a prompt returns an I/O failure before all metadata is resolved
- **THEN** init exits unsuccessfully with an informative message and no success output
- **AND** no manifest or other project file is created

#### Scenario: Feedback fails after creation

- **WHEN** the manifest write succeeds but subsequent interactive feedback fails
- **THEN** any error explains that the manifest was created rather than claiming no files changed
