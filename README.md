<h1 align="center"><a name="hydragen"></a>Hydragen</h1>

<p align="center">
  <a href="#hydragen"><img src="https://img.shields.io/badge/status-prototype-orange?style=for-the-badge" alt="Status: prototype" height="20"></a>
  <a href="#build-and-run-locally"><img src="https://img.shields.io/badge/language-Rust-D34516?style=for-the-badge&amp;logo=rust&amp;logoColor=white" alt="Language: Rust" height="20"></a>
  <a href="https://deps.rs/repo/github/hydratic-applications/hydragen"><img src="https://deps.rs/repo/github/hydratic-applications/hydragen/status.svg?style=for-the-badge" alt="Dependency status" height="20"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue?style=for-the-badge" alt="License: GPL-3.0-or-later" height="20"></a>
  <br>
  <a href="https://github.com/hydratic-applications/hydragen"><img src="https://img.shields.io/badge/GitHub-hydratic--applications%2Fhydragen-blue?style=for-the-badge&amp;logo=github&amp;logoColor=white" alt="GitHub: hydratic-applications/hydragen" height="20"></a>
  <a href="https://github.com/hydratic-applications/hydragen/stargazers"><img src="https://img.shields.io/github/stars/hydratic-applications/hydragen?style=for-the-badge&amp;label=stars" alt="GitHub stars" height="20"></a>
  <a href="https://github.com/sponsors/hydratic-applications"><img src="https://img.shields.io/github/sponsors/hydratic-applications?style=for-the-badge&amp;label=sponsors&amp;logo=data%3Aimage%2Fsvg%2Bxml%3Bbase64%2CPHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0xMiAyMSAzIDEyQy0zIDYgNi0zIDEyIDVjNi04IDE1IDEgOSA3WiIvPjwvc3ZnPg%3D%3D" alt="GitHub sponsors" height="20"></a>
</p>

Hydragen is a development system for building distinct projects from shared code,
configuration, and project files. Its building block is a **layer**: a project or
reusable set of files that other layers can depend on. The goal is to keep shared
content connected to its sources while making project-specific changes clear.

**This is an initial prototype.** Use `hydragen init` to create a layer manifest
identifying a layer, and `hydragen add` to record verified local-path dependencies.
Add declares relationships only: it does not fetch or copy files, render a
workspace, or capture contributions. Publishing and registry access are not
implemented.

See [FOUNDATIONS.md](FOUNDATIONS.md) for the product goals and
[GLOSSARY.md](GLOSSARY.md) for the layer model's terminology.
The [name and icon](docs/brand.md) express the project's identity and guiding values.

## Build and run locally

You need a recent stable Rust toolchain and Cargo, available through
[rustup](https://rustup.rs/). This version was tested with Rust 1.96.0.

From this repository's root:

```sh
cargo build --locked
```

To try the built command in a disposable directory without installing it:

```sh
HYDRAGEN="$PWD/target/debug/hydragen"
DEMO="$(mktemp -d)"
cd "$DEMO"
"$HYDRAGEN" init --name shared-agent-rules
```

Alternatively, install it from your checkout's root:

```sh
cargo install --path . --locked
```

Ensure Cargo's installation directory, usually `~/.cargo/bin`, is on your `PATH`.
The built CLI works offline and does not require Git or an AI service.

## Initialise a layer

Run init in the directory that will be the layer's repository root. It can be an
existing Git repository or a directory where you intend to create one later;
Hydragen does not initialise Git. Inside an existing repository, run it at the
root, not in a subdirectory.

### Guided setup

In an interactive terminal:

```sh
hydragen init
```

The prompts ask for a name, optional author and description, then a layer manifest
format. Press Enter to use the directory's name, skip either optional field,
or accept the default format, YAML (`layer.yaml`). Escape or Ctrl-C cancels
without creating a file.

To use the directory-name default, it must satisfy the
[layer-name rules](#the-layer-manifest). If it contains uppercase letters or
spaces, for example, init stops before prompting; supply a valid override such as
`hydragen init --name my-project`. Hydragen does not rename the directory or
silently convert the name.

### Using flags

Supply any of `--name`, `--author`, `--description`, or `--format` to run without
questions. Omitted name defaults to the directory's name, author and description
are left out, and format defaults to YAML.

```sh
hydragen init --name shared-agent-rules \
  --author "Example team" \
  --description "Coding-agent instructions shared across our projects."
```

This creates `layer.yaml`:

```yaml
name: shared-agent-rules
author: Example team
description: Coding-agent instructions shared across our projects.
```

To choose another format in a fresh directory, use `--format` with `yaml`, `yml`,
`toml`, `json`, or `jsonc`. For example, `hydragen init --format toml` creates
`layer.toml` using the directory's name, without asking questions.

### Scripts and CI

Use `--non-interactive` to explicitly disable prompts:

```sh
hydragen init --non-interactive
```

Without input flags, this uses the directory's name and YAML format, omitting
author and description. Prompts are also disabled in CI or when any standard
stream is not a terminal; init does not read piped answers.

Init confirms the created layer manifest filename once. Guided feedback uses
standard error; flag-only success uses standard output.

## Add local dependencies

Suppose `my-project`, `shared-agent-rules`, and `shared-config` are sibling
directories, each already initialised as a layer. From `my-project`, run:

```sh
hydragen add ../shared-agent-rules ../shared-config
```

For a layer manifest initially containing only `name: my-project`, the resulting
`layer.yaml` declares the following ordered dependencies (formatting may differ):

```yaml
name: my-project
dependencies:
  - ../shared-agent-rules
  - ../shared-config
```

Use explicit paths: `./shared-config`, `../shared-config`, or a native absolute
path, not a bare directory name. Path spelling is stored unchanged; directory
names can contain spaces or uppercase even though declared layer names cannot.

You can run add from a nested directory. It searches upward to the filesystem
root for the nearest directory with a supported layer manifest filename, crossing
Git boundaries if necessary. An invalid selection fails there rather than falling
back to a parent layer. **Relative paths resolve from the selected layer
manifest's directory, not your current directory.** Running the example from
`my-project/src` therefore still selects the two siblings of `my-project`.

Before any update, add validates the current layer, all existing direct local
dependencies, and the whole requested batch. Each target directory must have its
own valid layer manifest; its parents and dependency chain are not searched.
Dependency names must be unique and cannot equal the current layer's name. Broken
existing dependencies or any batch error prevent all additions.

New references append in argument order without reordering existing entries.
Re-adding the same named local directory is harmless, even with a different path
spelling: the existing reference stays unchanged. A name claimed by a different
directory or an existing published dependency is a conflict, not an implicit
replacement. Repeated new sources are added only once.

Success names the destination layer and requested dependencies. One dependency
gets a single sentence: `Added shared-config to my-project.` or
`No changes needed: shared-config is already a dependency of my-project.` Multiple
dependencies use separate counted `Added` and `Already present` groups, omitting
empty groups and the layer manifest path. `Already present` means declared before
this invocation; an all-repeat batch reports no changes and does not rewrite the
file. Add always requires arguments and never prompts, uses the network, or
changes Git.

## Output and automation

Add success uses standard output. Warnings and errors from either command use
standard error, and errors return a nonzero exit status. Global
`--non-interactive` is accepted before or after either subcommand.

Terminal status messages use colour when appropriate. CI, redirected execution,
or `NO_COLOR=1` keep results plain; `--non-interactive` disables questions without
disabling eligible terminal styling.

## The layer manifest

The layer manifest lives at the layer's repository root. It requires a `name` and
can include optional `author`, `description`, and an ordered `dependencies` list.
An omitted or empty dependency list means the layer has no dependencies.

**Layer names must be lowercase.** Use lowercase ASCII letters, digits, `.`, `_`,
and `-`, optionally as `@scope/name`, with at most 214 characters in the complete
name. Unscoped names and scopes cannot start with `.` or `_`; the name after a
scope can. Whitespace, uppercase, and other characters are rejected, never silently
rewritten. These rules apply to init flags, answers, directory defaults, and every
layer manifest loader; existing layer manifests with invalid names need deliberate
correction. A layer's declared name is separate from its directory name and the
fixed layer manifest filename.

Author and description remain arbitrary strings containing at least one
non-whitespace character, preserved unchanged. Omit unsupplied optional fields
rather than setting them to empty strings or `null`.

Supported formats are **YAML** (`layer.yaml` or `layer.yml`), **TOML**
(`layer.toml`), **JSON** (`layer.json`), and **JSON with comments** (`layer.jsonc`).
The filename is fixed by the format, not by the layer's name.

If more than one layer manifest is present, the priority order is:

**`layer.yaml` > `layer.yml` > `layer.toml` > `layer.json` > `layer.jsonc`**

Hydragen warns about duplicates. Layer manifests are not merged, and an invalid
higher-priority layer manifest does not cause fallback to another file. Keep only
the layer manifest you intend to use.

Existing published declarations such as `shared-config@1.2.0` or
`@example/shared-config@1.2.0-beta.1+build.7` are accepted for offline syntax and
name checks only, not verified existence. They require complete, strict
[Semantic Versioning (SemVer) 2.0.0](https://semver.org/spec/v2.0.0.html) versions,
including optional prerelease and build identifiers; ranges, tags such as `latest`,
and implicit versions are not supported. Local paths have no version, and no
lockfile is generated. Adding published dependencies, publishing, and
registry-backed guided search/version selection are deferred. Future published
add must reject a different version of an already-declared layer, not overwrite
it; a separate explicit upgrade/version-change workflow has not been chosen.

There is no CLI read or validate command yet. For detailed content rules, see the
[layer manifest schema](schemas/layer-manifest.schema.json) and
[dependency declaration specification](openspec/specs/layer-manifest/spec.md).

## Existing files and failures

Init creates only the layer manifest and leaves other project files unchanged. It
refuses to run if any supported layer manifest filename is already occupied,
including by a directory or symlink. It never overwrites an existing entry; there
is no force option.

If a write fails, a partial new layer manifest may remain. Inspect the path
reported in the error before retrying. If creation succeeds but reporting the
result fails, the error tells you the layer manifest was already created.

Add edits only the selected current layer manifest, which must be a regular file,
not a symlink. It preserves the format, comments, and unrelated source text and
data, or refuses an edit it cannot perform safely. Dependency files are read only.
Updates use a hidden staging file beside the layer manifest and atomic replacement,
preserving file permissions. Handled failures before replacement leave the original
intact; cleanup failures identify any staging file left behind. If replacement
succeeds but reporting fails, the error says the layer manifest was already updated.

Abrupt termination can leave a hidden staging file. Atomic replacement does not
guarantee survival of every power failure or preserve all filesystem metadata.
There is no lock or concurrent-edit check: simultaneous writers can overwrite
each other's updates. Avoid editing the layer manifest concurrently with add.
See the [addition and safety specification](openspec/specs/layer-dependency-addition/spec.md)
for the detailed contract.

## Help

```sh
hydragen --help
hydragen init --help
hydragen add --help
hydragen --version
```

## Development

From the checkout's root:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

Product requirements and design decisions live in [OpenSpec](openspec/); those
documents are not build or runtime inputs.

## License

Hydragen is licensed under **GNU GPL version 3 or later** (`GPL-3.0-or-later`).

Copyright (C) 2026 Leyton Blackler

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program in [LICENSE](LICENSE). If not, see
<https://www.gnu.org/licenses/>.

Using Hydragen does not, by itself, require your independently authored projects
or layers to use the GPL. Third-party content retains its applicable terms;
see [third-party notices](THIRD_PARTY_NOTICES.md) for material included here.
