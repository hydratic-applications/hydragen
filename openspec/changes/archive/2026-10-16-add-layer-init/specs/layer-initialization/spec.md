## ADDED Requirements

### Requirement: Initialise a layer without choosing a layer type

The CLI MUST provide `hydragen init` to create a layer manifest in the current
directory. It MUST NOT require or emit a root/dependent type declaration.
Initialisation MUST omit dependencies from the layer manifest, without introducing a
permanent restriction on the layer later declaring dependencies.

#### Scenario: Initialise a layer

- **WHEN** a developer runs `hydragen init` with valid metadata in a supported directory without `layer.json`
- **THEN** the command creates `layer.json` in that directory
- **AND** the layer manifest contains no dependency list or layer-type field
- **AND** success output describes a layer rather than a special root-only project

### Requirement: Accept explicit non-blank metadata without prompts

Initialisation MUST require `--name` and accept `--author` and `--description`
as independently optional flags. Name and every supplied optional value MUST
be strings containing at least one non-whitespace character, matching the layer manifest
schema's metadata requirements. The command MUST preserve accepted values as
supplied, MUST NOT impose additional naming or identity rules, and MUST NOT prompt
or infer missing metadata. Absent optional fields MUST be omitted from the
layer manifest, not written as empty strings or `null`.

#### Scenario: Only name is supplied

- **WHEN** initialisation receives a non-blank name without author or description
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and creates a layer manifest containing only name

#### Scenario: One optional field is supplied

- **WHEN** initialisation receives a non-blank name and either a non-blank author or description
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and records name and the supplied optional field
- **AND** it omits the other optional field

#### Scenario: All metadata is supplied

- **WHEN** initialisation receives non-blank name, author, and description values
- **THEN** it records those values unchanged in the corresponding layer manifest fields

#### Scenario: Required metadata is missing

- **WHEN** an invocation omits `--name`
- **THEN** the command exits unsuccessfully and identifies the missing argument with usage guidance
- **AND** it does not create a layer manifest or wait for interactive input

#### Scenario: Metadata is blank

- **WHEN** any supplied metadata value is empty or consists entirely of whitespace
- **THEN** the command exits unsuccessfully and identifies the invalid field
- **AND** it creates no files

#### Scenario: Metadata includes JSON-sensitive characters

- **WHEN** accepted metadata includes quotes, backslashes, newlines, or non-ASCII characters
- **THEN** the created layer manifest is valid JSON and decoding it recovers the original values

### Requirement: Write a minimal readable JSON layer manifest

A successful initialisation MUST write UTF-8, indented JSON with a trailing
newline. The JSON object MUST contain `name`, plus `author` and `description`
only when supplied, for this increment. It MUST NOT add a version, schema address,
dependency field, or other properties.

#### Scenario: Inspect a newly created layer manifest

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

Initialisation MUST NOT replace an existing entry at `layer.json`, including a
regular file, directory, or symlink. The no-overwrite guarantee MUST hold if an
entry appears after an initial check but before creation. The command MUST NOT
read, validate, repair, or merge an existing layer manifest. A successful invocation
MUST create only `layer.json` and MUST leave all existing project files unchanged.

#### Scenario: A layer manifest already exists

- **WHEN** `layer.json` already exists, whether its contents are valid JSON or not
- **THEN** initialisation fails and reports that the path already exists
- **AND** the existing bytes remain unchanged

#### Scenario: A directory or symlink occupies the layer manifest path

- **WHEN** a directory or symlink, including a dangling symlink, occupies `layer.json`
- **THEN** initialisation fails without replacing the entry or changing a symlink target

#### Scenario: Repeat initialisation

- **WHEN** initialisation is run again in a directory where it previously succeeded
- **THEN** the second invocation fails without changing the layer manifest or other files

#### Scenario: Preserve an existing project

- **WHEN** initialisation succeeds in a directory containing source files, configuration, and Git metadata
- **THEN** all pre-existing files remain unchanged
- **AND** no workspace, contribution files, Git metadata, or other project files are created

#### Scenario: Another process creates the layer manifest first

- **WHEN** another process creates an entry at `layer.json` before initialisation opens its output
- **THEN** initialisation fails rather than overwriting that entry

### Requirement: Provide predictable command outcomes and help

Successful initialisation MUST exit with status zero and identify the created
layer manifest on standard output after writing completes. Usage, validation, location,
and filesystem failures MUST exit with a nonzero status and actionable
diagnostics on standard error, without claiming success. Top-level and init help
MUST be available without writing project files or requiring metadata.

#### Scenario: Request help

- **WHEN** a developer runs `hydragen --help` or `hydragen init --help`
- **THEN** the command exits successfully and displays relevant usage
- **AND** it creates no project files
- **AND** init help identifies name as required and author and description as optional
- **AND** it includes name-only and optional-metadata examples

#### Scenario: A filesystem write fails

- **WHEN** creating or writing the layer manifest fails
- **THEN** the command exits unsuccessfully and reports the affected path and failure
- **AND** it does not print a success message
- **AND** it does not alter pre-existing project files

### Requirement: Initialisation works without external runtime services

Initialisation MUST NOT require network access, AI services, the Git executable,
or an external schema validator. It MUST NOT execute Git commands or modify a
repository's history, configuration, or staging area.

#### Scenario: Run in an offline non-interactive environment

- **WHEN** valid initialisation is run without terminal input, network access, AI services, or a Git executable
- **AND** all filesystem and location conditions are satisfied
- **THEN** it succeeds using the same command-line interface and layer manifest format
