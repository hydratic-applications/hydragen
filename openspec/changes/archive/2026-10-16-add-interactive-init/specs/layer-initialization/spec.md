## MODIFIED Requirements

### Requirement: Accept explicit non-blank metadata without prompts

Initialisation MUST use flag-only input whenever any metadata flag is supplied or
interaction is unavailable. This route MUST accept `--name` as an optional override
and `--author` and `--description` as independently optional flags, without prompting.
When name is not supplied, init MUST use the current directory's final path
component unchanged as the name. Guided input MUST offer the same value as a
displayed default accepted by Enter. Init MUST NOT infer any other metadata.
Only when no metadata flags are supplied and interaction is available MUST init
collect metadata through the guided CLI interaction flow. It MUST NOT combine
flags and prompt answers in one invocation.

Name and every supplied optional value MUST be strings containing at least one
non-whitespace character, matching the layer manifest schema's metadata requirements.
The command MUST validate supplied flags without opening prompts, preserve
accepted values unchanged, and MUST NOT impose additional naming or identity
rules. Invalid flag values or flags written without a value MUST produce an error rather than
a replacement question. Absent or interactively skipped optional fields MUST be
omitted from the layer manifest, not written as empty strings or `null`.

#### Scenario: Only name is supplied

- **WHEN** initialisation receives a non-blank name flag without author or description flags, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and creates a layer manifest containing only name

#### Scenario: One optional field is supplied

- **WHEN** initialisation receives a non-blank name flag and either a non-blank author or description flag, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and records name and the supplied optional field
- **AND** it omits the other optional field

#### Scenario: All metadata is supplied

- **WHEN** initialisation receives non-blank name, author, and description values as flags
- **THEN** it records those values unchanged in the corresponding layer manifest fields
- **AND** it asks no metadata questions

#### Scenario: Name override is omitted

- **WHEN** an invocation is flag-only or interaction is unavailable, and it omits `--name`
- **AND** the current directory's name is a non-blank UTF-8 string and other initialisation conditions are satisfied
- **THEN** the command succeeds using that directory name unchanged
- **AND** it records any supplied author or description, omits unsupplied fields, and asks no questions

#### Scenario: No usable directory-name default

- **WHEN** no name override is supplied and the current directory has no final path component, has a whitespace-only name, or its name is not valid UTF-8
- **THEN** init fails with guidance to supply `--name` explicitly before prompts or writes
- **AND** a valid explicit `--name` bypasses directory-name derivation
- **AND** it does not invent a name or replace invalid characters

#### Scenario: Metadata is blank

- **WHEN** any metadata value supplied as a flag is empty or consists entirely of whitespace
- **THEN** the command exits unsuccessfully and identifies the invalid field
- **AND** it creates no files or replacement prompts

#### Scenario: Metadata includes JSON-sensitive characters

- **WHEN** accepted metadata includes quotes, backslashes, newlines, or non-ASCII characters
- **THEN** the created layer manifest is valid JSON and decoding it recovers the original values

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
- **THEN** it exits successfully with version information and no prompts or project-file changes
