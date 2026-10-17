## MODIFIED Requirements

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
