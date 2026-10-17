## MODIFIED Requirements

### Requirement: Preserve existing layer manifests and project files

Initialisation MUST NOT replace an existing entry at `layer.json`, including a
regular file, directory, or symlink. The no-overwrite guarantee MUST hold if an
entry appears after an initial check but before creation. The command MUST NOT
read, validate, repair, or merge an existing layer manifest. A successful invocation
MUST create only `layer.json` and MUST leave all existing project files unchanged.

When an entry occupies the layer manifest path, the diagnostic MUST name the path,
explain that init does not overwrite existing entries, and suggest a safe next
step: inspect the entry or run init in the intended different directory. It MUST
state that this invocation did not change files, without assuming the entry is
a valid layer or implying that Hydragen read its contents. It MUST NOT recommend
blind deletion, offer force/overwrite confirmation, or suggest an unimplemented
command. The occupied path MUST remain intact in the diagnostic.

#### Scenario: A layer manifest already exists

- **WHEN** `layer.json` already exists, whether its contents are valid JSON or not
- **THEN** initialisation fails and reports that the path already exists, explains the no-overwrite policy, and gives the safe next-step guidance
- **AND** the existing bytes remain unchanged

#### Scenario: A directory or symlink occupies the layer manifest path

- **WHEN** a directory or symlink, including a dangling symlink, occupies `layer.json`
- **THEN** initialisation fails without replacing the entry or changing a symlink target
- **AND** its diagnostic names the occupied path and gives guidance accurate for an arbitrary entry, without assuming valid layer manifest contents

#### Scenario: Repeat initialisation

- **WHEN** initialisation is run again in a directory where it previously succeeded
- **THEN** the second invocation fails without changing the layer manifest or other files
- **AND** it explains the no-overwrite policy and a safe next step without offering to replace the layer manifest

#### Scenario: Preserve an existing project

- **WHEN** initialisation succeeds in a directory containing source files, configuration, and Git metadata
- **THEN** all pre-existing files remain unchanged
- **AND** no workspace, contribution files, Git metadata, or other project files are created

#### Scenario: Another process creates the layer manifest first

- **WHEN** another process creates an entry at `layer.json` before initialisation opens its output
- **THEN** initialisation fails rather than overwriting that entry
- **AND** it gives the same occupied-path policy and recovery guidance without claiming that the other process made no changes

### Requirement: Provide predictable command outcomes and help

Successful initialisation MUST exit with status zero and identify the created
layer manifest on standard output after writing completes. Guided cliclack prompts and
session feedback MUST use standard error, not the result stream. For flag-only
input or any other invocation where interaction is disabled, success and error
output MUST remain plain text without cliclack banners, prompts, spinners, or
terminal-control sequences.

Usage, validation, location, input, cancellation, and filesystem failures MUST
exit with a nonzero status and actionable diagnostics on standard error, without
claiming success. Top-level and init help and version output MUST remain available
without writing project files, requiring metadata, or opening a prompt session.

Root and init help MUST adapt ordinary explanatory prose and option descriptions
to the detected terminal width, using word-boundary wrapping and readable
continuation hierarchy rather than depending on terminal autowrap. Init option
descriptions MUST start on a following line so their hierarchy remains clear at
the representative 40-, 80-, and 100-column review widths. Help MUST remain plain
and preserve all meaningful usage, defaults, input-route rules, and examples.
Examples MUST retain argument boundaries and quoting so they can be copied as
valid shell commands, with deliberate continuations when needed. A token too
long to fit without splitting MUST remain intact rather than be truncated or
changed. The review widths MUST NOT be treated as a general minimum-width promise.

Help MUST cap normal wrapping at 100 columns while respecting narrower detected
widths. When terminal width cannot be determined, help MUST use a 100-column
fallback layout, without asking questions or opening another terminal. Width
adaptation MUST NOT alter the command's meaning, exit behaviour, or output channel.

#### Scenario: Request help

- **WHEN** a developer runs `hydragen --help` or `hydragen init --help`
- **THEN** the command exits successfully and displays relevant usage
- **AND** it creates no project files and opens no prompts
- **AND** init help explains the directory-name default and optional name override, optional author and description, and the non-interactive override
- **AND** it explains that any metadata flag selects flag-only input without follow-up questions
- **AND** it includes interactive and flag-based examples

#### Scenario: A filesystem write fails

- **WHEN** creating or writing the layer manifest fails
- **THEN** the command exits unsuccessfully and reports the affected path and failure
- **AND** it does not print a success message
- **AND** it does not alter pre-existing project files

#### Scenario: Non-interactive output

- **WHEN** valid init runs with interaction disabled
- **THEN** it reports success and the layer manifest path only on standard output, with no terminal UI on standard error
- **AND** errors instead use standard error without success output or terminal UI

#### Scenario: Interactive output

- **WHEN** init collects answers in an interactive terminal and successfully creates the layer manifest
- **THEN** cliclack renders the session on standard error
- **AND** the successful result line and layer manifest path are reported on standard output after the write

#### Scenario: Request version

- **WHEN** a developer runs `hydragen --version`
- **THEN** the command exits successfully with version information and no prompts or project-file changes

#### Scenario: Help at representative terminal widths

- **WHEN** root or init help is requested with `-h` or `--help` at detected widths of 40, 80, or 100 columns
- **THEN** ordinary prose and descriptions wrap at word boundaries with readable hierarchy, preserving all content
- **AND** init descriptions follow their option names on subsequent lines
- **AND** deliberate example continuations retain valid shell argument boundaries and quoting
- **AND** output is plain stdout, status is zero, and no prompts or files are created

#### Scenario: Captured help without a detectable width

- **WHEN** help is captured and no terminal width can be determined
- **THEN** it uses the 100-column fallback layout and preserves the same usage information
- **AND** it does not read answers or open another terminal and remains plain stdout with status zero

#### Scenario: A help token exceeds the available width

- **WHEN** a help token cannot fit at the reviewed width without splitting
- **THEN** the full token is preserved even if it exceeds that width
- **AND** surrounding prose still wraps normally without treating an entire long description as an unbreakable token
