## MODIFIED Requirements

### Requirement: Preserve existing manifests and project files

At its initial target check, init MUST reject an occupied path at any supported
manifest filename, regardless of the requested output format or content validity.
This includes regular files, directories, and symlinks, including dangling
symlinks. The command MUST NOT read, validate, repair, merge, rename, or replace an
existing manifest. An initial duplicate discovery MUST be reported according to
the manifest warning policy before refusal.

Creation MUST remain exclusive for the exact chosen output path, including if an
entry appears after the initial check. The initial check MUST NOT reserve the
directory or create a lock. A different supported filename appearing after that
check MUST NOT by itself prevent creation or turn a successful write into an
error. Init MUST NOT repeat an all-format refusal check at the final write.
A successful invocation MUST create only its selected manifest and MUST leave
all pre-existing project files unchanged.

When an entry occupies the manifest path, the CLI diagnostic MUST name its actual
filename and identify the current directory as its location, without repeating
the absolute directory path. Its headline MUST read `layer.<extension> already
exists in the current directory.`, substituting the occupied filename even when
another format was requested. It MUST explain that init does not overwrite
existing entries and suggest inspecting the entry or running init in the intended
different directory. It MUST state that this invocation did not change files,
without assuming the entry is a valid layer or implying that Hydragen read its
contents. It MUST NOT recommend blind deletion, offer force/overwrite confirmation,
or suggest an unimplemented command. Other errors and the library's diagnostic
path context MUST remain unchanged.

#### Scenario: A manifest already exists

- **WHEN** any supported manifest path is occupied when init performs its initial check, whether its contents are valid or not
- **THEN** initialisation fails and names the occupied filename in the current directory, explains the no-overwrite policy, and gives safe next-step guidance, even if another format was requested
- **AND** the existing entry and bytes remain unchanged

#### Scenario: A directory or symlink occupies a manifest path

- **WHEN** a directory or symlink, including a dangling symlink, occupies any supported manifest path at the initial check
- **THEN** initialisation fails without replacing the entry or changing a symlink target
- **AND** its diagnostic names the occupied filename and current directory with guidance accurate for an arbitrary entry, without assuming valid manifest contents

#### Scenario: Repeat initialisation

- **WHEN** initialisation is run again in a directory where it previously succeeded
- **THEN** the second invocation fails without changing the manifest or other files, even if it requests a different format
- **AND** it explains the no-overwrite policy and a safe next step without offering to replace the manifest

#### Scenario: Preserve an existing project

- **WHEN** initialisation succeeds in a directory containing source files, configuration, and Git metadata
- **THEN** all pre-existing files remain unchanged
- **AND** no workspace, contribution files, Git metadata, lock files, or other project files are created

#### Scenario: Another process creates the exact target first

- **WHEN** another process creates an entry at the chosen manifest path before init opens its output
- **THEN** initialisation fails rather than overwriting that entry
- **AND** it does not retry with another format
- **AND** it gives the same concise occupied-filename diagnostic, policy, and recovery guidance without claiming that the other process made no changes

#### Scenario: Another format appears after the initial check

- **WHEN** init's initial check finds no occupied supported manifest paths
- **AND** a different supported manifest filename appears before the final write
- **AND** exclusive creation and writing of the chosen target otherwise succeed
- **THEN** initialisation succeeds without deleting or modifying either manifest
- **AND** its final observation reports duplicates if both paths are then observed

### Requirement: Provide predictable command outcomes and help

Successful initialisation MUST exit with status zero after writing completes and
report exactly one success message: `Layer initialised, manifest created at:
layer.<extension>`, using the actual created manifest filename without repeating
the absolute directory. Guided init MUST report completion through its cliclack
session on standard error, with no separate success/path line on standard output.
Flag-only init MUST report its result on standard output without a session header
or prompt frame. Prompts, warnings, and errors MUST use standard error.

Operational terminal feedback MUST use a green checkmark (`✓`) for success,
a yellow triangle (`▲`) with `Warning:` for warnings, and a red cross (`✕`) with
`Error:` for failures where styling is enabled. The success filename MUST be bold.
Only diagnostic headlines MUST receive severity colouring; supporting details
and recovery guidance MUST remain readable in the normal foreground.

Output styling MUST be independent of input mode. Flag-only output to an eligible
terminal MUST use compact styled feedback without starting guided input.
Decorative statuses MUST require all standard streams to be terminals, CI to be
disabled, and `TERM` not to be `dumb`. A non-empty `NO_COLOR` or `CLICOLOR=0`
MUST disable decorative status styling even if colour is forced. A global
`--non-interactive` flag MUST disable questions without independently disabling
otherwise eligible compact styling.
Captured and unattended output MUST retain the same meaning as plain text,
without ANSI decoration, status icons, banners, prompts, or spinners even when a
forced-colour environment variable is supplied. Colour-disabled preferences MUST
be respected without enabling or disabling questions. Existing manifest and
filesystem error details MUST NOT be discarded to simplify the presentation.

Usage, validation, location, input, cancellation, and create/write failures MUST
exit with a nonzero status and actionable diagnostics on standard error, without
claiming success. A duplicate warning alone MUST NOT cause failure. Top-level and
init help and version output MUST remain available without writing project files,
requiring metadata, discovering manifests, or opening a prompt session.

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
- **AND** it creates no project files, discovers no manifests, and opens no prompts
- **AND** init help explains the directory-name default, optional metadata, format choices, YAML default, and the non-interactive override
- **AND** it explains that any input flag, including format alone, selects flag-only input without follow-up questions
- **AND** it includes interactive and flag-based examples

#### Scenario: A filesystem write fails

- **WHEN** creating or writing the manifest fails
- **THEN** the command exits unsuccessfully and reports the affected path and failure
- **AND** it does not print a success message
- **AND** it does not alter pre-existing project files

#### Scenario: Non-interactive terminal output

- **WHEN** valid flag-only init runs with its result attached to an eligible terminal
- **THEN** stdout contains one green-check success naming the created manifest file
- **AND** there is no branded opening, prompt, or session frame
- **AND** warnings and errors use compact styled stderr diagnostics without changing their meaning or exit-status rules

#### Scenario: Captured or unattended output

- **WHEN** init runs in CI or emits output to a non-terminal stream
- **THEN** the affected output remains plain text even under forced-colour settings
- **AND** flag-only success names the created manifest on stdout while diagnostics use stderr
- **AND** there are no status icons, prompts, branded openings, or terminal-control sequences

#### Scenario: Interactive output

- **WHEN** init collects answers in an interactive terminal and successfully creates the manifest
- **THEN** cliclack renders the session and single success message on stderr
- **AND** the success names the created manifest filename rather than the absolute directory
- **AND** stdout has no duplicate success or manifest-path result

#### Scenario: Every selected format identifies the file actually created

- **WHEN** successful init uses any supported manifest format
- **THEN** the success message names that format's actual fixed manifest filename
- **AND** it neither hardcodes layer.yaml for other formats nor echoes an absolute directory

#### Scenario: Colour-disabled feedback

- **WHEN** the developer disables colour for otherwise eligible terminal output
- **THEN** feedback remains understandable without coloured text
- **AND** input-route selection, output streams, status meanings, and filesystem outcomes remain unchanged

#### Scenario: Mixed terminal and redirected streams

- **WHEN** any standard stream is redirected while another remains a terminal
- **THEN** compact status output on both output streams is plain even if colour is forced
- **AND** no questions or branded opening appear

#### Scenario: Explicit disable wins over forced colour

- **WHEN** an otherwise eligible terminal invocation sets non-empty NO_COLOR or CLICOLOR=0 together with forced colour
- **THEN** its status output has no decorative styling or status icons
- **AND** its input-route and stream choices are unaffected

#### Scenario: Request version

- **WHEN** a developer runs `hydragen --version`
- **THEN** it exits successfully with version information and no manifest discovery, prompts, or project-file changes

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
