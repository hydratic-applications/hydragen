# Layer initialisation specification

## Purpose

Define how `hydragen init` creates a layer manifest in the current directory,
including metadata validation, repository-location checks, file safety, and
command outcomes.

## Requirements

### Requirement: Initialise a layer without choosing a layer type

The CLI MUST provide `hydragen init` to create a layer manifest in the current
directory. It MUST NOT require or emit a root/dependent type declaration.
Initialisation MUST omit dependencies from the manifest, without introducing a
permanent restriction on the layer later declaring dependencies.

#### Scenario: Initialise a layer

- **WHEN** a developer runs `hydragen init` with valid metadata in a supported directory without `layer.json`
- **THEN** the command creates `layer.json` in that directory
- **AND** the manifest contains no dependency list or layer-type field
- **AND** success output describes a layer rather than a special root-only project

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
non-whitespace character, matching the manifest schema's metadata requirements.
The command MUST validate supplied flags without opening prompts, preserve
accepted values unchanged, and MUST NOT impose additional naming or identity
rules. Invalid flag values or flags written without a value MUST produce an error rather than
a replacement question. Absent or interactively skipped optional fields MUST be
omitted from the manifest, not written as empty strings or `null`.

#### Scenario: Only name is supplied

- **WHEN** initialisation receives a non-blank name flag without author or description flags, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and creates a manifest containing only name

#### Scenario: One optional field is supplied

- **WHEN** initialisation receives a non-blank name flag and either a non-blank author or description flag, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and records name and the supplied optional field
- **AND** it omits the other optional field

#### Scenario: All metadata is supplied

- **WHEN** initialisation receives non-blank name, author, and description values as flags
- **THEN** it records those values unchanged in the corresponding manifest fields
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
- **THEN** the created manifest is valid JSON and decoding it recovers the original values

### Requirement: Write a minimal readable JSON manifest

A successful initialisation MUST write UTF-8, indented JSON with a trailing
newline. The JSON object MUST contain `name`, plus `author` and `description`
only when supplied, for this increment. It MUST NOT add a version, schema address,
dependency field, or other properties.

#### Scenario: Inspect a newly created manifest

- **WHEN** initialisation succeeds
- **THEN** `layer.json` is readable, indented JSON ending in a newline
- **AND** its only properties are name and any author or description supplied by the developer

### Requirement: Use the current directory with a repository-location guard

Initialisation MUST target the current directory, without redirecting to an
ancestor or creating another directory. For conventional Git layouts, it MUST
find the nearest `.git` directory or file in the current directory or its
ancestors. It MUST permit initialisation when that entry is in the current
directory or no such entry exists. It MUST reject initialisation when that entry
exists only in an ancestor, identifying the repository root. Unexpected errors
checking those entries MUST cause failure without writing a manifest.

This guard covers ordinary Git working trees and layouts using a `.git` file;
it is not validation of Git internals. Bare repositories and layouts selected
only through Git environment variables are outside this increment's supported
layout.

#### Scenario: Initialise before creating a Git repository

- **WHEN** the current directory and its ancestors contain no `.git` directory or file
- **AND** all other initialisation conditions are satisfied
- **THEN** initialisation creates the manifest in the current directory without creating a Git repository

#### Scenario: Initialise at an existing repository root

- **WHEN** the current directory contains a `.git` directory
- **AND** all other initialisation conditions are satisfied
- **THEN** initialisation creates the manifest in that directory

#### Scenario: Initialise at a Git worktree root

- **WHEN** the current directory contains a `.git` file
- **AND** all other initialisation conditions are satisfied
- **THEN** initialisation creates the manifest without modifying the `.git` file

#### Scenario: Refuse a nested repository location

- **WHEN** the nearest `.git` directory or file is in an ancestor of the current directory
- **THEN** initialisation fails with guidance identifying the repository root
- **AND** it creates no manifest in either directory

#### Scenario: Repository-location checks fail

- **WHEN** checking a possible `.git` entry fails for a reason other than the entry being absent
- **THEN** initialisation reports the filesystem failure and creates no files

### Requirement: Preserve existing manifests and project files

Initialisation MUST NOT replace an existing entry at `layer.json`, including a
regular file, directory, or symlink. The no-overwrite guarantee MUST hold if an
entry appears after an initial check but before creation. The command MUST NOT
read, validate, repair, or merge an existing manifest. A successful invocation
MUST create only `layer.json` and MUST leave all existing project files unchanged.

When an entry occupies the manifest path, the diagnostic MUST name the path,
explain that init does not overwrite existing entries, and suggest a safe next
step: inspect the entry or run init in the intended different directory. It MUST
state that this invocation did not change files, without assuming the entry is
a valid layer or implying that Hydragen read its contents. It MUST NOT recommend
blind deletion, offer force/overwrite confirmation, or suggest an unimplemented
command. The occupied path MUST remain intact in the diagnostic.

#### Scenario: A manifest already exists

- **WHEN** `layer.json` already exists, whether its contents are valid JSON or not
- **THEN** initialisation fails and reports that the path already exists, explains the no-overwrite policy, and gives the safe next-step guidance
- **AND** the existing bytes remain unchanged

#### Scenario: A directory or symlink occupies the manifest path

- **WHEN** a directory or symlink, including a dangling symlink, occupies `layer.json`
- **THEN** initialisation fails without replacing the entry or changing a symlink target
- **AND** its diagnostic names the occupied path and gives guidance accurate for an arbitrary entry, without assuming valid manifest contents

#### Scenario: Repeat initialisation

- **WHEN** initialisation is run again in a directory where it previously succeeded
- **THEN** the second invocation fails without changing the manifest or other files
- **AND** it explains the no-overwrite policy and a safe next step without offering to replace the manifest

#### Scenario: Preserve an existing project

- **WHEN** initialisation succeeds in a directory containing source files, configuration, and Git metadata
- **THEN** all pre-existing files remain unchanged
- **AND** no workspace, contribution files, Git metadata, or other project files are created

#### Scenario: Another process creates the manifest first

- **WHEN** another process creates an entry at `layer.json` before initialisation opens its output
- **THEN** initialisation fails rather than overwriting that entry
- **AND** it gives the same occupied-path policy and recovery guidance without claiming that the other process made no changes

### Requirement: Provide predictable command outcomes and help

Successful initialisation MUST exit with status zero and identify the created
manifest on standard output after writing completes. Guided cliclack prompts and
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

- **WHEN** creating or writing the manifest fails
- **THEN** the command exits unsuccessfully and reports the affected path and failure
- **AND** it does not print a success message
- **AND** it does not alter pre-existing project files

#### Scenario: Non-interactive output

- **WHEN** valid init runs with interaction disabled
- **THEN** it reports success and the manifest path only on standard output, with no terminal UI on standard error
- **AND** errors instead use standard error without success output or terminal UI

#### Scenario: Interactive output

- **WHEN** init collects answers in an interactive terminal and successfully creates the manifest
- **THEN** cliclack renders the session on standard error
- **AND** the successful result line and manifest path are reported on standard output after the write

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

### Requirement: Initialisation works without external runtime services

Initialisation MUST NOT require network access, AI services, the Git executable,
or an external schema validator. It MUST NOT execute Git commands or modify a
repository's history, configuration, or staging area.

#### Scenario: Run in an offline non-interactive environment

- **WHEN** valid initialisation is run without terminal input, network access, AI services, or a Git executable
- **AND** all filesystem and location conditions are satisfied
- **THEN** it succeeds using the same command-line interface and manifest format
