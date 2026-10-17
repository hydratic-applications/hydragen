## Context

The repository contains foundations, a glossary, and OpenSpec planning, but no
application code. The `define-layer-manifest` change establishes `layer.json` at
the repository root, with a required name and optional author and description.
All supplied metadata values must be non-blank strings; dependency reference
syntax remains open.

This increment implements only initialisation. It does not yet let a developer
inherit files or edit a rendered layer result. That boundary keeps the first
implementation reviewable while providing a real command to try.

## Goals / Non-Goals

**Goals:**

- Create a layer manifest safely in a new or existing project directory.
- Use the same command and layer manifest shape for any layer, without a type selector.
- Work without interactive input, network access, AI, or an installed Git command.
- Establish a small Rust application with focused tests and build instructions.

**Non-Goals:**

- Add, retrieve, validate, or resolve dependency references.
- Render workspaces or capture additions, modifications, or deletions.
- Introduce an inspection command or a general existing-layer-manifest validator.
- Create Git repositories, change Git configuration, stage files, or commit.
- Provide prompts, inferred metadata, a target-directory argument, a force flag,
  publishing, installers, or agent-specific interfaces.

## Decisions

### Initialise a layer without a type declaration

The command creates identifying metadata only. It omits the dependency list,
which already means no dependencies under the agreed layer model. It emits no
`type`, `root`, or extensibility field and does not describe the result as a
special root-only project in help or success output.

Dependencies determine whether the layer is a root or dependent layer. A future
dependency-management command can add them without recreating or converting the
layer. This increment does not select the dependency field name or format.

An explicit root/dependent selector would duplicate information from dependencies
and incorrectly suggest that initialisation makes a permanent category choice.

### Require explicit metadata through flags

The smallest valid invocation is:

```sh
hydragen init --name shared-agent-rules
```

Author and description can be supplied independently:

```sh
hydragen init --name shared-agent-rules \
  --author "Example team" \
  --description "Coding-agent instructions shared across our projects."
```

Only `--name` is required. `--author` and `--description` are optional, and each can
be supplied without the other. Every supplied value must contain at least one
non-whitespace character; otherwise, the command fails before creating files.
Omit optional fields that were not supplied rather than serialising empty strings
or `null`. Preserve accepted values as supplied rather than silently changing
their spelling or whitespace. Do not add naming, uniqueness, contact-information,
or length rules.

CLI validation must match the metadata requirements in the
`define-layer-manifest` schema: a required non-blank name, and non-blank optional
strings when present. Test name-only input, each optional field independently,
all fields together, and invalid supplied values. This does not turn the draft
schema into a complete validator for dependency declarations or introduce a
general existing-layer-manifest validation command.

Explicit flags avoid introducing prompt handling, Git identity lookup, or
directory-name defaults. Developers, scripts, and agents can use the same
interface. Running `hydragen init` without `--name` reports the missing argument
and usage; it does not wait for input or require the optional fields.

### Use the current directory and respect repository boundaries

The current directory is the intended root of the layer's repository. Do not
search upward for a layer manifest and initialise a different directory, and do not
create missing parent directories.

Initialisation does not require Git to have been initialised yet. If no enclosing
Git working tree is detected, write the layer manifest in the current directory; the
developer can later initialise a repository there.

To avoid violating the agreed layer manifest location inside an existing repository,
use a small filesystem guard: starting at the current directory, find the
nearest `.git` directory or file. If it is in the current directory, proceed. If
it is only in an ancestor, fail and identify that repository root. Recognising
both a directory and a file accommodates ordinary clones and Git worktrees.
If checking a path fails for a reason other than absence, report the error
rather than assume there is no repository.

This is a guard against accidental nested initialisation, not a Git repository
validator. Bare repositories and repository layouts selected only through Git
environment variables are outside this increment's supported layout. Do not
invoke Git, parse its history, or create `.git` entries. Requiring an existing
repository would add an unnecessary prerequisite; accepting nested locations
silently would conflict with the layer manifest's agreed repository-root location.

### Write only the layer manifest and never replace an existing entry

On success, write readable, indented UTF-8 JSON with a trailing newline. The
layer manifest contains `name`, plus `author` and `description` only when supplied.
Do not emit a dependency list, version, or `$schema` address: references and
versioning are unresolved, and there is no published schema address to use.

Validate metadata and serialise it before opening the output. Create
`layer.json` with exclusive creation, so a concurrent initialisation cannot
overwrite it between an existence check and the write. An existing regular
file, directory, or symlink at that path causes failure without replacement.
Do not read or attempt to repair an existing layer manifest, even if it is malformed.

No other project files are written. In particular, do not create a workspace,
contribution directory, or project `.gitignore`. Existing code and configuration
are left in place; scanning and classifying those files is not part of init.

Report success on standard output only after the write completes, identifying
the created layer manifest. Report usage, location, validation, and filesystem failures
on standard error with a nonzero exit status. Do not fix exact diagnostic wording
or distinct numeric error codes in this increment. A write failure can leave a
partial newly created layer manifest; report its path so the developer can inspect it.
Do not promise crash-atomic file creation or remove paths that could belong to
another process.

### Start with one Rust package and tests at the command boundary

Use one Cargo package at the repository root, producing the `hydragen` binary.
Use `clap` for argument parsing and help, and `serde` with `serde_json` for metadata
serialisation. Commit `Cargo.lock` for reproducible application builds. Ignore
Cargo's build output without broadly ignoring project content.

Keep argument handling separate enough from validation and file creation to test
them directly, but do not add a multi-crate architecture, plugin system, async
runtime, full JSON Schema engine, or dependency resolver.

Use isolated temporary directories for command-level tests. Cover ordinary
directories, repository roots, nested directories, and `.git` files as well as
directories, without invoking Git. Test metadata validation directly and parse
generated JSON rather than relying only on text snapshots. Include an existing
file preservation test and a repeated-init test.

## Risks / Trade-offs

- **Initialisation does not demonstrate reuse.** → Describe its limited scope
  honestly; rendering and contribution capture are a later increment, not implied
  capabilities of this command.
- **An explicit name is more typing than an inferred value or prompt.** → Show
  both name-only and optional-metadata examples in help and the README. Revisit
  convenience once real use identifies friction rather than adding input modes now.
- **Filesystem repository detection covers conventional layouts only.** → State
  the supported layouts and test `.git` files, nested paths, and inaccessible
  checks. Broader Git integration is separate work.
- **The layer manifest schema remains a draft.** → Match its metadata rules without
  copying it into a second runtime schema or claiming to validate dependencies.
- **A failed write can leave a partial layer manifest.** → Report the failure and path;
  refuse to overwrite it on retry. Stronger transactional writes can be evaluated
  if this limitation causes practical problems.

## Migration Plan

There is no existing application or persisted CLI state to migrate. Build and try
the command locally before considering packaging or distribution. Removing the
newly created layer manifest undoes initialisation; no other project content changes.
Do not add a separate rollback command.

## Verification

Checked on 2026-10-16 on macOS with Rust and Cargo 1.96.0:

- `cargo build`, `cargo fmt --check`, and
  `cargo clippy --all-targets -- -D warnings` passed.
- The README's build command and both initialisation examples were checked in
  disposable directories. Repeating init preserved the existing layer manifest and
  reported the expected error.
- `cargo test --locked` passed all 24 tests: six library tests and 18 command-level
  tests. Unix-specific tests exercise real repository-inspection and write
  failures, and concurrent initialisation preserves the winning layer manifest.
- A separate comparison against the archived layer manifest schema, using Python's
  `jsonschema` 4.26.0 Draft 2020-12 validator, agreed with the CLI on 16 sets of
  metadata supplied as command-line arguments. These covered optional-field combinations,
  missing name, blank values, whitespace, escaping, and Unicode. Non-string JSON
  values cannot be supplied through the string-valued CLI flags.

The Rust tests also check the schema's required fields and metadata constraints
without introducing a runtime schema engine. The implementation uses exclusive
file creation rather than an existence check followed by an overwriting write.

## Open Questions

No unresolved decision blocks this increment. Dependency sources and versions,
the workspace layout, patch structure, capture timing, updates, and purpose-built
agent interfaces remain open in `openspec/design-questions.md`. None is settled
by creating identifying metadata.

The foundations already describe layers by their dependencies, rather than a type
chosen at creation, and need no revision. The glossary's layer and layer manifest
definitions reflect that only name is required metadata; author and description
are optional.
