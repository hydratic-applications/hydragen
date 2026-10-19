# Layer initialisation specification

## Purpose

Define how `hydragen init` creates a layer manifest in the current directory,
including format selection, metadata validation, repository-location checks,
file safety, and command outcomes.

## Requirements

### Requirement: Initialise a layer without choosing a layer type

The CLI MUST provide `hydragen init` to create a layer manifest in the current
directory. It MUST NOT require or emit a root/dependent type declaration.
Initialisation MUST omit dependencies from the layer manifest, without introducing a
permanent restriction on the layer later declaring dependencies. The basename
MUST remain `layer`, independent of the layer's identifying name.

#### Scenario: Initialise a layer

- **WHEN** a developer runs `hydragen init` with valid input in a supported directory with no occupied supported layer manifest paths at its initial check
- **THEN** the command creates the layer manifest with the chosen format's fixed filename in that directory
- **AND** the layer manifest contains no dependency list or layer-type field
- **AND** success output describes a layer rather than a special root-only project

### Requirement: Accept explicit non-blank metadata without prompts

Initialisation MUST use flag-only input whenever any input flag is supplied or
interaction is unavailable. The input flags MUST be `--name`, `--author`,
`--description`, and `--format`. This route MUST accept `--name` as an optional
override and `--author` and `--description` as independently optional flags,
without prompting. When name is not supplied, init MUST use the current
directory's final path component unchanged as the name. Guided input MUST offer
the same value as a displayed default accepted by Enter. Init MUST NOT infer any
other metadata. Only when no input flags are supplied and interaction is available
MUST init collect input through the guided CLI interaction flow. It MUST NOT
combine flags and prompt answers in one invocation.

Name MUST satisfy the maintained schema's shared lowercase layer-name definition.
Author and description, when supplied, MUST retain their existing non-blank string
rules. Flags, guided answers, directory defaults, and complete candidates MUST use
that same schema. Init MUST preserve accepted values without silently lowercasing,
trimming, or replacing invalid characters. Init MUST NOT impose a stricter
independent naming rule outside the schema.

Invalid flag values or flags without a value MUST error rather than start a
replacement question. A schema-invalid directory default MUST fail before prompts
or writes with guidance to supply `--name`; a valid explicit name MUST bypass that
default. Absent or interactively skipped optional fields MUST be omitted, not
written as empty strings or `null`. Existing target checks, input routes, and
exclusive creation MUST remain unchanged.

#### Scenario: Only name is supplied

- **WHEN** initialisation receives a schema-valid name flag without author or description flags, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and creates a layer manifest containing only name without prompts

#### Scenario: One optional field is supplied

- **WHEN** initialisation receives a valid name flag and either a non-blank author or description flag, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and records name and the supplied optional field unchanged
- **AND** it omits the other optional field

#### Scenario: All metadata is supplied

- **WHEN** initialisation receives a valid name and non-blank author and description values as flags
- **THEN** it records those values unchanged in the corresponding layer manifest fields
- **AND** it asks no metadata questions

#### Scenario: Name override is omitted

- **WHEN** an invocation is flag-only or interaction is unavailable, and it omits `--name`
- **AND** the current directory's UTF-8 name satisfies the shared name definition and other initialisation conditions are satisfied
- **THEN** the command succeeds using that directory name unchanged
- **AND** it records any supplied author or description, omits unsupplied fields, and asks no questions

#### Scenario: No usable directory-name default

- **WHEN** no name override is supplied and the current directory has no final path component, has a schema-invalid name including uppercase or spaces, or its name is not valid UTF-8
- **THEN** init fails with guidance to supply `--name` explicitly before prompts or writes
- **AND** a valid explicit `--name` bypasses directory-name derivation
- **AND** it does not invent, lowercase, or sanitise a name

#### Scenario: Metadata is blank

- **WHEN** any metadata value supplied as a flag is empty or consists entirely of whitespace
- **THEN** the command exits unsuccessfully and identifies the invalid field
- **AND** it creates no files or replacement prompts

#### Scenario: Metadata includes format-sensitive characters

- **WHEN** accepted optional metadata includes quotes, backslashes, newlines, whitespace, or non-ASCII characters alongside a valid name
- **THEN** the created layer manifest is valid in the chosen format and decoding it recovers the original strings

#### Scenario: Uppercase name is supplied explicitly

- **WHEN** init receives `--name Shared-Config`
- **THEN** it rejects the name without silently converting it to `shared-config`
- **AND** creates no file and opens no replacement prompt

#### Scenario: Invalid guided name can be corrected

- **WHEN** the directory default is valid but a guided answer violates a name constraint
- **THEN** schema-derived prompt feedback explains the correction without referring to flags
- **AND** the developer can edit the answer before any layer manifest is written

### Requirement: Write a minimal readable layer manifest in the selected format

A successful initialisation MUST write a UTF-8 layer manifest in the selected format,
ending in a newline. YAML and YML MUST use a readable YAML mapping; TOML MUST use
readable top-level key/value entries with quoted metadata strings and no `[layer]`
wrapper; JSON and JSONC MUST use two-space indented JSON. JSONC output MUST NOT require generated
comments or a header. The object/mapping MUST contain `name`, plus `author` and
`description` only when supplied, for this increment. It MUST NOT add a version,
schema address, dependency field, format field, or other properties.

The emitted root fields MUST appear in `name`, `author`, `description` order,
skipping omitted optional fields, in every supported format. Input flag order
MUST NOT change that output order. This output guarantee MUST NOT make other
field orders invalid when reading existing layer manifests.

Serialisation MUST preserve accepted metadata as strings, including values that
would otherwise be interpreted as YAML booleans, numbers, or nulls. Serialisation
and metadata validation MUST complete before opening the output file.

#### Scenario: Inspect a newly created layer manifest

- **WHEN** initialisation succeeds in any supported format
- **THEN** its fixed filename matches the selected extension and its content is readable in that format with a final newline
- **AND** its only properties are name and any author or description supplied by the developer
- **AND** those properties appear in name, author, description order with absent fields skipped

#### Scenario: A string resembles a YAML typed scalar

- **WHEN** accepted metadata is a string such as `true`, `123`, `null`, or `2026-10-16`
- **THEN** generated YAML uses appropriate quoting or escaping to preserve the original string when decoded

#### Scenario: Minimal TOML output preserves string metadata

- **WHEN** init successfully creates a TOML layer manifest with quotes, backslashes, newlines, or date-like text in accepted metadata
- **THEN** `layer.toml` contains appropriately quoted or escaped top-level strings and decoding recovers the exact original values
- **AND** omitted optional fields remain absent and no `[layer]` wrapper or starter comments are generated

#### Scenario: Input order does not dictate generated field order

- **WHEN** flag-only init supplies description before author and name, or guided init supplies equivalent metadata
- **THEN** both routes emit name, author, description in that order for the selected format
- **AND** supplying only name and description emits those two fields in that order without adding author

### Requirement: Use the current directory with a repository-location guard

Initialisation MUST target the current directory, without redirecting to an
ancestor or creating another directory. For conventional Git layouts, it MUST
find the nearest `.git` directory or file in the current directory or its
ancestors. It MUST permit initialisation when that entry is in the current
directory or no such entry exists. It MUST reject initialisation when that entry
exists only in an ancestor, identifying the repository root. Unexpected errors
checking those entries MUST cause failure without writing a layer manifest.

This guard covers ordinary Git working trees and layouts using a `.git` file;
it is not validation of Git internals. Bare repositories and layouts selected
only through Git environment variables are outside this increment's supported
layout.

#### Scenario: Initialise before creating a Git repository

- **WHEN** the current directory and its ancestors contain no `.git` directory or file
- **AND** all other initialisation conditions are satisfied
- **THEN** initialisation creates the layer manifest in the current directory without creating a Git repository

#### Scenario: Initialise at an existing repository root

- **WHEN** the current directory contains a `.git` directory
- **AND** all other initialisation conditions are satisfied
- **THEN** initialisation creates the layer manifest in that directory

#### Scenario: Initialise at a Git worktree root

- **WHEN** the current directory contains a `.git` file
- **AND** all other initialisation conditions are satisfied
- **THEN** initialisation creates the layer manifest without modifying the `.git` file

#### Scenario: Refuse a nested repository location

- **WHEN** the nearest `.git` directory or file is in an ancestor of the current directory
- **THEN** initialisation fails with guidance identifying the repository root
- **AND** it creates no layer manifest in either directory

#### Scenario: Repository-location checks fail

- **WHEN** checking a possible `.git` entry fails for a reason other than the entry being absent
- **THEN** initialisation reports the filesystem failure and creates no files

### Requirement: Preserve existing layer manifests and project files

At its initial target check, init MUST reject an occupied path at any supported
layer manifest filename, regardless of the requested output format or content validity.
This includes regular files, directories, and symlinks, including dangling
symlinks. The command MUST NOT read, validate, repair, merge, rename, or replace an
existing layer manifest. An initial duplicate discovery MUST be reported according to
the layer manifest warning policy before refusal.

Creation MUST remain exclusive for the exact chosen output path, including if an
entry appears after the initial check. The initial check MUST NOT reserve the
directory or create a lock. A different supported filename appearing after that
check MUST NOT by itself prevent creation or turn a successful write into an
error. Init MUST NOT repeat an all-format refusal check at the final write.
A successful invocation MUST create only its selected layer manifest and MUST leave
all pre-existing project files unchanged.

When an entry occupies the layer manifest path, the CLI diagnostic MUST name its
actual filename and identify the current directory as its location, without
repeating the absolute directory path. Its headline MUST read `This directory
already has a layer manifest (layer.<extension>).`, substituting the occupied
filename even when another format was requested. One supporting line MUST read
`Initialisation creates a new layer manifest; it won’t overwrite an existing one.`
The refusal MUST contain only these two lines, without separate generic inspection,
different-directory, or no-files-changed advice. Initialisation MUST still leave
existing entries unchanged. Naming the entry's layer manifest role MUST NOT imply
that its contents were read or validated; the same refusal applies to malformed
files, directories, and symlinks. It MUST NOT recommend blind deletion, offer
force/overwrite confirmation, or suggest an unimplemented command. Other errors
and the library's diagnostic path context MUST remain unchanged.

#### Scenario: A layer manifest already exists

- **WHEN** any supported layer manifest path is occupied when init performs its initial check, whether its contents are valid or not
- **THEN** initialisation fails with a two-line refusal identifying the existing layer manifest and its filename and explaining that init creates rather than overwrites a layer manifest, even if another format was requested
- **AND** the existing entry and bytes remain unchanged

#### Scenario: A directory or symlink occupies a layer manifest path

- **WHEN** a directory or symlink, including a dangling symlink, occupies any supported layer manifest path at the initial check
- **THEN** initialisation fails without replacing the entry or changing a symlink target
- **AND** its diagnostic names the occupied filename and current directory with guidance accurate for an arbitrary entry, without assuming valid layer manifest contents

#### Scenario: Repeat initialisation

- **WHEN** initialisation is run again in a directory where it previously succeeded
- **THEN** the second invocation fails without changing the layer manifest or other files, even if it requests a different format
- **AND** it explains that initialisation creates rather than overwrites a layer manifest, without offering replacement or appending generic recovery advice

#### Scenario: Preserve an existing project

- **WHEN** initialisation succeeds in a directory containing source files, configuration, and Git metadata
- **THEN** all pre-existing files remain unchanged
- **AND** no workspace, contribution files, Git metadata, lock files, or other project files are created

#### Scenario: Another process creates the exact target first

- **WHEN** another process creates an entry at the chosen layer manifest path before init opens its output
- **THEN** initialisation fails rather than overwriting that entry
- **AND** it does not retry with another format
- **AND** it gives the same two-line refusal without claiming that the other process made no changes

#### Scenario: Another format appears after the initial check

- **WHEN** init's initial check finds no occupied supported layer manifest paths
- **AND** a different supported layer manifest filename appears before the final write
- **AND** exclusive creation and writing of the chosen target otherwise succeed
- **THEN** initialisation succeeds without deleting or modifying either layer manifest
- **AND** its final observation reports duplicates if both paths are then observed

### Requirement: Provide predictable command outcomes and help

Successful initialisation MUST exit with status zero after writing completes and
report exactly one success message: `Layer initialised, layer manifest created at:
layer.<extension>`, using the actual created layer manifest filename without
repeating the absolute directory. Guided init MUST report completion through its cliclack
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
be respected without enabling or disabling questions. Existing layer manifest and
filesystem error details MUST NOT be discarded to simplify the presentation.

Usage, validation, location, input, cancellation, and create/write failures MUST
exit with a nonzero status and actionable diagnostics on standard error, without
claiming success. A duplicate warning alone MUST NOT cause failure. Top-level and
init help and version output MUST remain available without writing project files,
requiring metadata, discovering layer manifests, or opening a prompt session.

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
- **AND** it creates no project files, discovers no layer manifests, and opens no prompts
- **AND** init help explains the directory-name default, optional metadata, format choices, YAML default, and the non-interactive override
- **AND** it explains that any input flag, including format alone, selects flag-only input without follow-up questions
- **AND** it includes interactive and flag-based examples

#### Scenario: A filesystem write fails

- **WHEN** creating or writing the layer manifest fails
- **THEN** the command exits unsuccessfully and reports the affected path and failure
- **AND** it does not print a success message
- **AND** it does not alter pre-existing project files

#### Scenario: Non-interactive terminal output

- **WHEN** valid flag-only init runs with its result attached to an eligible terminal
- **THEN** stdout contains one green-check success naming the created layer manifest file
- **AND** there is no branded opening, prompt, or session frame
- **AND** warnings and errors use compact styled stderr diagnostics without changing their meaning or exit-status rules

#### Scenario: Captured or unattended output

- **WHEN** init runs in CI or emits output to a non-terminal stream
- **THEN** the affected output remains plain text even under forced-colour settings
- **AND** flag-only success names the created layer manifest on stdout while diagnostics use stderr
- **AND** there are no status icons, prompts, branded openings, or terminal-control sequences

#### Scenario: Interactive output

- **WHEN** init collects answers in an interactive terminal and successfully creates the layer manifest
- **THEN** cliclack renders the session and single success message on stderr
- **AND** the success names the created layer manifest filename rather than the absolute directory
- **AND** stdout has no duplicate success or layer-manifest-path result

#### Scenario: Every selected format identifies the file actually created

- **WHEN** successful init uses any supported layer manifest format
- **THEN** the success message names that format's actual fixed layer manifest filename
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
- **THEN** it exits successfully with version information and no layer manifest discovery, prompts, or project-file changes

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

### Requirement: Initialisation works without external runtime services

Initialisation MUST NOT require network access, AI services, the Git executable,
or an external schema validator. It MUST NOT execute Git commands or modify a
repository's history, configuration, or staging area.

#### Scenario: Run in an offline non-interactive environment

- **WHEN** valid initialisation is run without terminal input, network access, AI services, or a Git executable
- **AND** all filesystem and location conditions are satisfied
- **THEN** it succeeds using the same command-line interface and layer manifest format

### Requirement: Select a supported initialisation format

Init MUST accept `--format` with exactly the lowercase values `yaml`, `yml`,
`toml`, `json`, and `jsonc`, corresponding to the fixed supported filenames. Flag-only
execution MUST default omitted format to `yaml`. Guided execution MUST offer the
same default in its final selector. Format MUST NOT become layer manifest metadata or
allow arbitrary paths, basenames, or extensions. A format flag MUST select only
the newly created file's format, not override precedence for existing layer manifests.

#### Scenario: Default output format

- **WHEN** init runs flag-only without an explicit format and all other conditions are satisfied
- **THEN** it creates `layer.yaml` containing YAML

#### Scenario: Explicit output format

- **WHEN** init receives any supported format flag and all other conditions are satisfied
- **THEN** it creates the matching fixed filename with content in that format and no other layer manifest

#### Scenario: TOML-only flag invocation

- **WHEN** init receives `--format toml` without metadata flags and all other initialisation conditions are satisfied
- **THEN** it creates `layer.toml` using the directory-name default, without questions
- **AND** author and description are omitted

#### Scenario: Invalid or missing format value

- **WHEN** `--format` has no value or has an unsupported value
- **THEN** clap reports a usage error without questions, writes, or layer manifest discovery

### Requirement: Observe duplicate layer manifests after successful creation

After a successful layer manifest write, init MUST make a best-effort read-only
observation of the supported paths. If multiple paths are observed, it MUST emit
one duplicate warning identifying the current selection according to layer manifest
priority and MUST still report success for the file actually created. The warning
MUST NOT claim to prove another process ran. Init MUST NOT roll back, delete, or
rewrite either layer manifest or create a lock to prevent this situation.

If the final observation cannot complete, init MUST warn that the layer manifest was
created but duplicate checking failed, without changing creation's successful
status. Failures to deliver terminal feedback or the result stream MUST retain
the existing reporting-error semantics and MUST NOT claim no files were changed.
The observation MUST NOT promise detection of files created after it ran.

#### Scenario: Created file is not the highest-priority layer manifest

- **WHEN** init creates `layer.json` after an initially empty check and its final observation also finds `layer.yaml`
- **THEN** it warns that `layer.yaml` takes priority
- **AND** it exits successfully and reports that it created `layer.json`
- **AND** both layer manifests remain unchanged by the observation

#### Scenario: Final observation fails

- **WHEN** the selected layer manifest is successfully created and written but final duplicate checking encounters an inspection error
- **THEN** init warns about the failed check and reports the created path with successful status
- **AND** it does not report cancellation, remove the file, or claim no files changed

#### Scenario: Concurrent initialisation selects different formats

- **WHEN** two init operations both initially observe no supported layer manifest paths and later create different supported filenames
- **THEN** both creations can succeed without locks or late all-format rejection
- **AND** each operation warns about whatever duplicates its own final observation sees
