# Layer initialisation specification

## Purpose

Define how `hydragen init` creates a layer manifest in the current directory,
including format selection, metadata validation, repository-location checks,
file safety, and command outcomes.

## Requirements

### Requirement: Initialise a layer without choosing a layer type

The CLI MUST provide `hydragen init` to create a layer manifest in the current
directory. It MUST NOT require or emit a root/dependent type declaration.
Initialisation MUST omit dependencies from the manifest, without introducing a
permanent restriction on the layer later declaring dependencies. The basename
MUST remain `layer`, independent of the layer's identifying name.

#### Scenario: Initialise a layer

- **WHEN** a developer runs `hydragen init` with valid input in a supported directory with no occupied supported manifest paths at its initial check
- **THEN** the command creates the manifest with the chosen format's fixed filename in that directory
- **AND** the manifest contains no dependency list or layer-type field
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

Name and every supplied optional metadata value MUST be strings containing at
least one non-whitespace character, matching the manifest metadata requirements.
The command MUST validate supplied flags without opening prompts, preserve
accepted values unchanged, and MUST NOT impose additional naming or identity
rules. Invalid flag values or flags written without a value MUST produce an error
rather than a replacement question. Absent or interactively skipped optional
fields MUST be omitted from the manifest, not written as empty strings or `null`.

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

#### Scenario: Metadata includes format-sensitive characters

- **WHEN** accepted metadata includes quotes, backslashes, newlines, whitespace, or non-ASCII characters
- **THEN** the created manifest is valid in the chosen format and decoding it recovers the original strings

### Requirement: Write a minimal readable manifest in the selected format

A successful initialisation MUST write a UTF-8 manifest in the selected format,
ending in a newline. YAML and YML MUST use a readable YAML mapping; TOML MUST use
readable top-level key/value entries with quoted metadata strings and no `[layer]`
wrapper; JSON and JSONC MUST use two-space indented JSON. JSONC output MUST NOT require generated
comments or a header. The object/mapping MUST contain `name`, plus `author` and
`description` only when supplied, for this increment. It MUST NOT add a version,
schema address, dependency field, format field, or other properties.

Serialisation MUST preserve accepted metadata as strings, including values that
would otherwise be interpreted as YAML booleans, numbers, or nulls. Serialisation
and metadata validation MUST complete before opening the output file.

#### Scenario: Inspect a newly created manifest

- **WHEN** initialisation succeeds in any supported format
- **THEN** its fixed filename matches the selected extension and its content is readable in that format with a final newline
- **AND** its only properties are name and any author or description supplied by the developer

#### Scenario: A string resembles a YAML typed scalar

- **WHEN** accepted metadata is a string such as `true`, `123`, `null`, or `2026-10-16`
- **THEN** generated YAML uses appropriate quoting or escaping to preserve the original string when decoded

#### Scenario: Minimal TOML output preserves string metadata

- **WHEN** init successfully creates a TOML manifest with quotes, backslashes, newlines, or date-like text in accepted metadata
- **THEN** `layer.toml` contains appropriately quoted or escaped top-level strings and decoding recovers the exact original values
- **AND** omitted optional fields remain absent and no `[layer]` wrapper or starter comments are generated

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

#### Scenario: A manifest already exists

- **WHEN** any supported manifest path is occupied when init performs its initial check, whether its contents are valid or not
- **THEN** initialisation fails and identifies the existing path, even if another format was requested
- **AND** the existing entry and bytes remain unchanged

#### Scenario: A directory or symlink occupies a manifest path

- **WHEN** a directory or symlink, including a dangling symlink, occupies any supported manifest path at the initial check
- **THEN** initialisation fails without replacing the entry or changing a symlink target

#### Scenario: Repeat initialisation

- **WHEN** initialisation is run again in a directory where it previously succeeded
- **THEN** the second invocation fails without changing the manifest or other files, even if it requests a different format

#### Scenario: Preserve an existing project

- **WHEN** initialisation succeeds in a directory containing source files, configuration, and Git metadata
- **THEN** all pre-existing files remain unchanged
- **AND** no workspace, contribution files, Git metadata, lock files, or other project files are created

#### Scenario: Another process creates the exact target first

- **WHEN** another process creates an entry at the chosen manifest path before init opens its output
- **THEN** initialisation fails rather than overwriting that entry
- **AND** it does not retry with another format

#### Scenario: Another format appears after the initial check

- **WHEN** init's initial check finds no occupied supported manifest paths
- **AND** a different supported manifest filename appears before the final write
- **AND** exclusive creation and writing of the chosen target otherwise succeed
- **THEN** initialisation succeeds without deleting or modifying either manifest
- **AND** its final observation reports duplicates if both paths are then observed

### Requirement: Provide predictable command outcomes and help

Successful initialisation MUST exit with status zero and identify the manifest
actually created on standard output after writing completes. Guided cliclack
prompts and session feedback MUST use standard error, not the result stream.
Warnings MUST also use standard error. For flag-only input or any other invocation
where interaction is disabled, success, warning, and error output MUST remain
plain text without cliclack banners, prompts, spinners, or terminal-control
sequences.

Usage, validation, location, input, cancellation, and create/write failures MUST
exit with a nonzero status and actionable diagnostics on standard error, without
claiming success. A duplicate warning alone MUST NOT cause failure. Top-level and
init help and version output MUST remain available without writing project files,
requiring metadata, discovering manifests, or opening a prompt session.

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

#### Scenario: Non-interactive output

- **WHEN** valid init runs with interaction disabled
- **THEN** it reports success and the created manifest path only on standard output, with no terminal UI on standard error
- **AND** any warnings use plain standard error without changing a successful result
- **AND** errors instead use standard error without success output or terminal UI

#### Scenario: Interactive output

- **WHEN** init collects answers in an interactive terminal and successfully creates the manifest
- **THEN** cliclack renders the session on standard error
- **AND** the successful result line and actual manifest path are reported on standard output after the write

#### Scenario: Request version

- **WHEN** a developer runs `hydragen --version`
- **THEN** it exits successfully with version information and no manifest discovery, prompts, or project-file changes

### Requirement: Initialisation works without external runtime services

Initialisation MUST NOT require network access, AI services, the Git executable,
or an external schema validator. It MUST NOT execute Git commands or modify a
repository's history, configuration, or staging area.

#### Scenario: Run in an offline non-interactive environment

- **WHEN** valid initialisation is run without terminal input, network access, AI services, or a Git executable
- **AND** all filesystem and location conditions are satisfied
- **THEN** it succeeds using the same command-line interface and manifest format

### Requirement: Select a supported initialisation format

Init MUST accept `--format` with exactly the lowercase values `yaml`, `yml`,
`toml`, `json`, and `jsonc`, corresponding to the fixed supported filenames. Flag-only
execution MUST default omitted format to `yaml`. Guided execution MUST offer the
same default in its final selector. Format MUST NOT become manifest metadata or
allow arbitrary paths, basenames, or extensions. A format flag MUST select only
the newly created file's format, not override precedence for existing manifests.

#### Scenario: Default output format

- **WHEN** init runs flag-only without an explicit format and all other conditions are satisfied
- **THEN** it creates `layer.yaml` containing YAML

#### Scenario: Explicit output format

- **WHEN** init receives any supported format flag and all other conditions are satisfied
- **THEN** it creates the matching fixed filename with content in that format and no other manifest

#### Scenario: TOML-only flag invocation

- **WHEN** init receives `--format toml` without metadata flags and all other initialisation conditions are satisfied
- **THEN** it creates `layer.toml` using the directory-name default, without questions
- **AND** author and description are omitted

#### Scenario: Invalid or missing format value

- **WHEN** `--format` has no value or has an unsupported value
- **THEN** clap reports a usage error without questions, writes, or manifest discovery

### Requirement: Observe duplicate manifests after successful creation

After a successful manifest write, init MUST make a best-effort read-only
observation of the supported paths. If multiple paths are observed, it MUST emit
one duplicate warning identifying the current selection according to manifest
priority and MUST still report success for the file actually created. The warning
MUST NOT claim to prove another process ran. Init MUST NOT roll back, delete, or
rewrite either manifest or create a lock to prevent this situation.

If the final observation cannot complete, init MUST warn that the manifest was
created but duplicate checking failed, without changing creation's successful
status. Failures to deliver terminal feedback or the result stream MUST retain
the existing reporting-error semantics and MUST NOT claim no files were changed.
The observation MUST NOT promise detection of files created after it ran.

#### Scenario: Created file is not the highest-priority manifest

- **WHEN** init creates `layer.json` after an initially empty check and its final observation also finds `layer.yaml`
- **THEN** it warns that `layer.yaml` takes priority
- **AND** it exits successfully and reports that it created `layer.json`
- **AND** both manifests remain unchanged by the observation

#### Scenario: Final observation fails

- **WHEN** the selected manifest is successfully created and written but final duplicate checking encounters an inspection error
- **THEN** init warns about the failed check and reports the created path with successful status
- **AND** it does not report cancellation, remove the file, or claim no files changed

#### Scenario: Concurrent initialisation selects different formats

- **WHEN** two init operations both initially observe no supported manifest paths and later create different supported filenames
- **THEN** both creations can succeed without locks or late all-format rejection
- **AND** each operation warns about whatever duplicates its own final observation sees
