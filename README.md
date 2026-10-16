# Hydragen

Hydragen is a development system for building distinct projects from shared code,
configuration, and project files. Its layer model aims to preserve the connection
to shared sources while keeping project-specific choices clear.

**This is an initial prototype.** The CLI currently implements only
`hydragen init`, which creates a layer's identifying manifest. Dependency
management, inheritance, rendering, and contribution capture are not implemented.
See [FOUNDATIONS.md](FOUNDATIONS.md) for the product goals and
[GLOSSARY.md](GLOSSARY.md) for the layer model's terminology.

## Build and run locally

You need a recent stable Rust toolchain and Cargo, available through
[rustup](https://rustup.rs/). This version was tested with Rust 1.96.0. Building
may download dependencies from crates.io; the built command needs no network,
Git executable, AI service, or external validator.

From this repository's root:

```sh
cargo build --locked
```

The executable is `target/debug/hydragen`. To try it in a disposable directory
without installing it, run these commands from this repository's root:

```sh
HYDRAGEN="$PWD/target/debug/hydragen"
DEMO="$(mktemp -d)"
cd "$DEMO"
"$HYDRAGEN" init --name shared-agent-rules
```

The current directory is the target. Do not run init inside an existing
repository's subdirectory; run it at that repository's root instead.

To make the command available outside the checkout without using its full path,
you can install the local package with Cargo:

```sh
cargo install --path /absolute/path/to/hydragen-2 --locked
```

Replace the path with your checkout's location and ensure Cargo's installation
directory, usually `~/.cargo/bin`, is on your `PATH`.

## Initialise a layer

From the directory that will be the layer's repository root, choose one of two
input routes: **no metadata flags means guided initialisation; any metadata flag
means flag-only initialisation.** A single invocation never combines flags and
prompt answers.

### Guided input

In an interactive terminal:

```sh
hydragen init
```

The guided flow uses [cliclack](https://docs.rs/cliclack/latest/cliclack/index.html)
prompts to ask for name, author, and description in order. The name prompt shows
the current directory's name as its default: press Enter to accept it, or type a
different name. Press Enter without text to skip author or description. Invalid
answers receive inline feedback so you can correct them. No files are created
until input is complete.

The session explains Enter's behaviour once at the start. Author and description
have dimmed `(optional)` labels and empty input areas; only name displays a default.
After an optional field is skipped, its completed prompt shows a dimmed, italic `Skipped`
instead of a blank answer. This is feedback only: the field stays absent from
`layer.json`.

### Flag-only input

Supply metadata flags to run without questions, even in a terminal:

```sh
hydragen init --name shared-agent-rules
```

This writes `layer.json` as indented UTF-8 JSON with a trailing newline:

```json
{
  "name": "shared-agent-rules"
}
```

All metadata flags are optional. Omitted name defaults to the current directory's
name; author and description are independently optional:

```sh
hydragen init --name shared-agent-rules \
  --author "Example team" \
  --description "Coding-agent instructions shared across our projects."
```

Use this longer example in a different directory if you already ran the first
one: init never replaces an existing manifest.

Omitting author or description means omit that field, not ask for it. For example,
`hydragen init --name example` asks no questions and writes only name.
`hydragen init --author "Example team"` also asks no questions: it uses the current
directory's name and records the supplied author. Invalid flag values never
trigger a replacement prompt; a flag written without its value is still an error.

Every supplied value must contain at least one non-whitespace character. Accepted
values are preserved as supplied, including surrounding whitespace. Omitted
optional fields are absent from the manifest, not empty strings or `null`.
An empty optional prompt answer skips the field, but an explicitly empty flag
value or a non-empty whitespace-only answer is invalid. The directory-name default
is preserved unchanged, including spaces and Unicode characters. If the directory
has no usable name (for example, a filesystem root, a whitespace-only name, or a
name that is not valid UTF-8), init reports an error before prompts or writes;
use `--name` to supply a non-blank override. There are no other inferred metadata
values, final confirmation prompts, or layer-type selectors. The resulting
manifest still always contains a non-blank name.

Init creates a **layer**, not a permanently separate kind of project. Under the
layer model, dependencies determine whether a layer is a root or dependent layer.
The initial manifest omits dependencies, and this CLI does not yet manage them.

### Automation and interaction controls

Guided input is available only when standard input, output, and error are all
terminals and continuous integration (CI) mode is disabled. Redirecting or closing
any standard stream disables prompts. When interaction is disabled, init never
reads piped answers or opens a separate terminal to ask for them. A non-empty `CI`
value disables prompts unless it is `0` or `false` (case-insensitive). Unset or
empty `CI` does not disable interaction by itself.

For an explicit guarantee, `--non-interactive` is accepted before or after init:

```sh
hydragen --non-interactive init --name shared-agent-rules
hydragen init --name shared-agent-rules --non-interactive
```

This control is optional for metadata-flag invocations, which are always
prompt-free. It also prevents bare init from asking questions in a terminal:
`hydragen init --non-interactive` writes a manifest using the current directory's
name, without author or description. Bare init in CI or with redirected streams
uses the same default without waiting for input.

## Directory and file safety

- Run at an existing Git working-tree root, or in a directory where you intend
  to create a repository later. Init does not require or initialise Git.
- A `.git` directory or file in the current directory identifies a supported
  repository root. If the nearest such entry is only in an ancestor, init fails
  and identifies that root instead of writing in the wrong directory.
- This guard recognises ordinary clones and worktree-style `.git` files; it does
  not validate Git internals. Bare repositories and layouts selected only through
  Git environment variables are outside this prototype's supported layout.
- Init creates only `layer.json`. It does not scan or classify existing project
  files, change Git metadata, create directories, or alter other files.
- An existing file, directory, or symlink at `layer.json` causes an error. This
  includes malformed manifests and dangling symlinks. Concurrent init attempts
  cannot overwrite a manifest created by the other attempt.
- Known target problems, including an existing manifest or incorrect repository
  location, are checked before guided questions. These read-only checks do not
  reserve the path; exclusive file creation still protects the final write.

## Output and errors

Success exits with status zero and prints the created manifest's path to standard
output. Errors exit with a nonzero status, print a diagnostic to standard error,
and do not print a success message. Guided prompts and session feedback use
standard error; the result line remains on standard output. Flag-only output is
plain text, with no cliclack UI even in a terminal. For example:

```text
Error: '/path/to/project/layer.json' already exists. Initialisation was cancelled; no files were changed.
```

There is no overwrite or force option. Init does not read, validate, repair, or
merge an existing manifest. If an actual write fails, a partial newly created
manifest can remain; the error identifies its path. Inspect that file before
removing it and retrying. Init does not promise recovery from a crash or disk
failure.

Escape or Ctrl-C during a prompt cancels initialisation with a nonzero exit and
no created files. Unexpected input loss or a prompt I/O error also stops without
creating a manifest. If creation succeeded but reporting the result fails, the
error instead explains that the manifest was already created; it does not claim
that no files changed.

For usage, examples, or the version, without prompts or writing files:

```sh
hydragen --help
hydragen init --help
hydragen --version
```

## Development checks

From the checkout's root:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

Tests use isolated temporary directories. Unit tests cover input-mode selection
without changing the test process's environment. Captured-output tests cover the
flag-only route, validation, JSON output, help/version, repository boundaries,
file preservation, concurrency, and execution without Git or terminal input.

Unix-specific tests use a development-only pseudo-terminal harness and `/bin/sh`
to exercise real cliclack prompts, separate output streams, cancellation, input
loss, optional skips, retries, and metadata flags in a terminal. Prompt waits and
child-process waits are bounded. Other Unix tests cover symlinks, output-reporting
failure, repository-inspection errors, and an actual write failure using a
child-process file-size limit. That limit does not affect your shell or other
tests. The built CLI requires neither a shell nor the terminal test harness.

The [manifest specification](openspec/specs/layer-manifest/spec.md) defines the
metadata requirements. Its
[draft JSON Schema](openspec/changes/archive/2026-10-16-define-layer-manifest/layer-manifest.schema.json)
currently checks metadata only, not dependency references or file location.
