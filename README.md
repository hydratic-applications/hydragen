# Hydragen

Hydragen is a development system for building distinct projects from shared code,
configuration, and project files. Its layer model aims to preserve the connection
to shared sources while keeping project-specific choices clear.

**This is an initial prototype.** The CLI currently implements only
`hydragen init`, which creates a layer's identifying manifest. Dependency
management, inheritance, rendering, and contribution capture are not implemented.
The library also provides shared read-only manifest discovery and loading; there
is no CLI read or validate command.
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
input routes: **no input flags means guided initialisation when interaction is
available; any input flag means flag-only initialisation.** The input flags are
`--name`, `--author`, `--description`, and `--format`. A single invocation never
combines flags and prompt answers. New manifests default to YAML (`layer.yaml`).

### Guided input

In an interactive terminal:

```sh
hydragen init
```

The guided flow uses [cliclack](https://docs.rs/cliclack/latest/cliclack/index.html)
prompts to ask Name, Author, Description, then Manifest format. The name prompt
shows the current directory's name as its default: press Enter to accept it, or
type a different name. Press Enter without text to skip author or description. Invalid
answers receive inline feedback so you can correct them. The final selector lists
`yaml`, `yml`, `toml`, `json`, and `jsonc`, in that order, with `yaml` initially
selected. Press Enter to accept YAML or choose another format. Each choice creates
the corresponding fixed filename, such as `layer.json` for `json`. No files are
created until all input, including format, is complete. The identifying name does
not change the fixed filename.

The session explains Enter's behaviour once at the start. Author and description
have dimmed `(optional)` labels and empty input areas; only the name text prompt
displays a default. After an optional field is skipped, its completed prompt
shows a dimmed, italic `Skipped` instead of a blank answer. This is feedback
only: the field stays absent from the created manifest. A selected format is
never shown as `Skipped`.

### Flag-only input

Supply any input flag to run without questions, even in a terminal:

```sh
hydragen init --name shared-agent-rules
```

This writes `layer.yaml` as a readable UTF-8 YAML mapping with a trailing newline:

```yaml
name: shared-agent-rules
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

Use `--format yaml|yml|toml|json|jsonc` to choose the output syntax and fixed
filename. For example, in a fresh directory:

```sh
hydragen init --format toml
```

Format alone asks no questions: this creates `layer.toml` using the current
directory's name, without author or description. The accepted format values are
exactly those lowercase extensions, not paths or arbitrary filenames. An invalid
or missing format value is a usage error, not an invitation to prompt. Omitting
format defaults to `yaml`; an inferred default does not count as an input flag.
Format is not stored as metadata and cannot override selection of an existing
manifest.

**Compatibility change:** init previously defaulted to `layer.json`. Scripts
needing JSON must now request it explicitly:

```sh
hydragen init --name shared-agent-rules --format json
```

Existing layers with a single `layer.json` need no changes or migration.

Every supplied metadata value must contain at least one non-whitespace character.
Accepted values are preserved as supplied, including surrounding whitespace. Omitted
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

This control is optional for input-flag invocations, which are always
prompt-free. It also prevents bare init from asking questions in a terminal:
`hydragen init --non-interactive` writes `layer.yaml` using the current directory's
name, without author or description. Bare init in CI or with redirected streams
uses the same name and YAML defaults without waiting for input.

## Manifest formats and reading

A manifest belongs at the layer's repository root. Its identifying `name` is
unrelated to its filename: a layer named `shared-agent-rules` still uses a fixed
`layer` filename, not `shared-agent-rules.yaml`.

The supported filenames and selection order, from highest to lowest priority, are:

| Priority | Filename | Syntax | Init flag |
| --- | --- | --- | --- |
| 1 (default) | `layer.yaml` | YAML | `--format yaml` |
| 2 | `layer.yml` | YAML | `--format yml` |
| 3 | `layer.toml` | TOML | `--format toml` |
| 4 | `layer.json` | Strict JSON | `--format json` |
| 5 | `layer.jsonc` | JSON with comments | `--format jsonc` |

In full: **`layer.yaml` > `layer.yml` > `layer.toml` > `layer.json` > `layer.jsonc`.**
This is Hydragen's convention, not a general rule shared by configuration tools.
There is no custom filename, configurable precedence, read-time format override,
or automatic migration or conversion. JSON5, HJSON, INI, XML, and executable
configuration are not supported.

### Output and accepted syntax

Init writes only `name` and supplied `author` and `description`, in that order.
All five outputs are UTF-8 with a final newline and preserve the exact accepted
strings, including whitespace, Unicode, and newlines. There are no starter
comments, headers, schema URLs, dependencies, versions, format properties, or
other extra fields.

- **YAML/YML:** Both extensions use the same parser and a readable block mapping
  on output, as in the default example above. Reading accepts one YAML 1.2 mapping
  document, ordinary comments, quoted and plain strings, block strings, standard
  `!!str` tags, and anchors/aliases within parser safety limits. YAML 1.1 merge
  keys, custom tags, file inclusion, and environment substitution are not
  supported. Metadata such as `name: true`, `name: 123`, or `name: null` is not a
  string and fails validation; quote it to make it string data. An empty YAML
  value is null, not an omitted field. YAML 1.2 plain `yes` and `on` are strings.
  Init quotes or escapes ambiguous strings as needed to preserve their meaning.

  YAML integers must fit a signed 64-bit value when negative or an unsigned
  64-bit value when non-negative. This applies to every integer spelling the
  parser recognizes, including underscores, uppercase radix prefixes, binary
  notation, and signed radix forms. In-range spellings keep their numeric meaning;
  overflowing spellings are rejected in keys and values at any depth, never
  treated as strings. Quote numeric-looking text or use `!!str` to keep it as
  a string.

  Binary-tagged keys and values (`!!binary`) are rejected before their bytes can
  be converted into UTF-8 text, even when those bytes would form valid text.
  The same rejection applies when the binary tag is written as an explicit URI
  or through a `%TAG` directive. Quoted text that merely contains `!!binary`
  remains ordinary text.

  **Parser corrections:** Overflowing extension integer spellings previously
  could fall back to strings, and binary-tagged data could become strings when
  its bytes were valid UTF-8. Both behaviours are now rejected at the parser
  boundary, including within additional fields, arrays, and mapping keys. These
  are explicit input-acceptance corrections, separate from schema validation.
- **TOML:** Metadata is declared as top-level quoted strings, with no `[layer]`
  wrapper. For example, supplying all three fields with `--format toml` produces:

  ```toml
  name = "shared-agent-rules"
  author = "Example team"
  description = "Coding-agent instructions shared across our projects."
  ```

  Reading accepts ordinary `#` comments, basic, literal, and multiline strings,
  dotted keys, tables, inline tables, and arrays of tables for supported data.
  The parser rejects duplicate keys and invalid table redefinitions; valid arrays
  of tables remain valid. TOML has no null syntax, and omitted keys stay absent.
  Native date/time literals and non-finite numbers are outside the shared
  manifest data model and are rejected, even in additional fields. Quote
  date/time-like text to keep it as a string. Numbers, booleans, and other types
  are never converted into strings to make metadata pass validation.
- **JSON/JSONC:** Both outputs use two-space indented JSON. For example, requesting
  either format with only `--name shared-agent-rules` produces:

  ```json
  {
    "name": "shared-agent-rules"
  }
  ```

  This is also valid JSONC; `.jsonc` needs no generated comments. When reading,
  strict `.json` rejects comments and trailing commas. `.jsonc` accepts line and
  block comments and trailing commas, but not JSON5 features such as unquoted
  keys, single-quoted strings, or missing commas.

### Shared content schema

The maintained [manifest schema](schemas/layer-manifest.schema.json) uses JSON
Schema Draft 2020-12. It validates the JSON-compatible data decoded from **all five
formats: YAML, YML, TOML, JSON, and JSONC**, not just JSON source files. The same
schema definitions govern flags, guided metadata answers, directory-name defaults,
and complete candidate manifests before writing. The library reader validates the
entire decoded document before extracting metadata.

The schema requires a non-blank string `name`; `author` and `description` are
independently optional but must be non-blank strings when supplied. Explicit null
is not omission. An optional `$schema` property must also be a non-blank string.
Non-blank means containing a character outside Unicode's `White_Space` set:
U+0085 counts as whitespace, but U+FEFF does not. The maintained schema spells
out this set rather than relying on the regex shorthand `\S`, preserving the
CLI's existing whitespace semantics. Validation does not trim, normalize, or
coerce values into strings.

**Loader correction:** `$schema` values were previously ignored by the reader.
They are now checked against the existing non-blank-string rule in every format:
blank strings and non-string values, such as numbers or null where the format
supports them, fail loading. This is a parsed-content check, not type coercion;
the separate YAML parser corrections are described above.

This is a **metadata-only** schema. It permits additional properties
(`additionalProperties: true`) without interpreting their meanings; passing
validation does not establish support for dependency declarations, versions, or
inheritance. The schema sees parsed data, not the original scalar spellings or
tags. Format parsers determine source types, accepted syntax, duplicate-key
handling, and which values can be represented. Schema validation does not decide
which filename governs a layer, verify repository location, or make file creation
safe; those remain separate checks.

The schema is bundled into the built CLI and used offline. Running the executable
needs no source checkout, schema sidecar file, external validator, or schema fetch.
There is no runtime schema-selection option, and a manifest's `$schema` value
never selects another validator or triggers retrieval. Changing the maintained
schema requires rebuilding the application to change its bundled rules.

Depending on your editor and its extensions, you may be able to associate a
manifest with this same local schema file through editor configuration. This
change does not publish a public schema URL, configure automatic associations,
provide a schema-export command, or promise editor support for every format. Init
does not add a `$schema` property to generated manifests.

### Shared library reader

The library separates discovery, loading, and content validation. Discovery
inspects only the five supported paths in a supplied directory, without reading
contents, searching ancestors or nested directories, or changing files. It
selects the highest-priority occupied name, including when that entry is a
directory or dangling symlink. Unexpected path-inspection errors are errors, not
evidence that a path is absent.

Loading reads **only the selected manifest**, using its filename's parser. A
symlink to a regular file is readable; a dangling symlink, directory, or other
non-regular target produces a path-specific error instead of waiting for special
file input. A selected file that disappears, cannot be read, or has invalid
syntax or metadata fails loading. There is no fallback to lower-priority content,
and manifests are never merged. With no supported path, loading reports a missing
manifest. Discovery remains available independently, so a caller can report
duplicates before a later loading error.

Loaded documents must be one object or mapping, with string keys and
JSON-compatible values: objects, arrays, strings, booleans, finite numbers, and
null. Repeated keys at any object depth,
including unknown fields or escaped spellings of the same JSON key, are errors
rather than first-value or last-value wins. Complex YAML keys, non-finite values,
empty or comment-only documents, multiple documents or top-level values, and
invalid UTF-8 are rejected.

Additional fields are retained as decoded data but are not interpreted; accepting
them does not validate dependency declarations. Loading never rewrites a file or
promises comment preservation for a future editing feature. This is a shared
library capability, not a new CLI command. Init uses discovery for safety and
warnings but does not read existing manifest contents.

## Directory and file safety

- Run at an existing Git working-tree root, or in a directory where you intend
  to create a repository later. Init does not require or initialise Git.
- A `.git` directory or file in the current directory identifies a supported
  repository root. If the nearest such entry is only in an ancestor, init fails
  and identifies that root instead of writing in the wrong directory.
- This guard recognises ordinary clones and worktree-style `.git` files; it does
  not validate Git internals. Bare repositories and layouts selected only through
  Git environment variables are outside this prototype's supported layout.
- Init creates only the selected manifest. Apart from the repository-location
  checks and inspecting supported manifest paths, it does not scan or classify
  existing project files, change Git metadata, create directories, or alter other
  files.
- At the initial check, an existing entry at **any** supported manifest filename
  causes an error, regardless of the requested output format. This includes
  malformed files, directories, symlinks, and dangling symlinks. Init does not
  read their contents to decide whether it can proceed.
- Known target problems, including an existing manifest or incorrect repository
  location, are checked before guided questions. These read-only checks do not
  reserve the directory. Metadata validation and serialisation complete before
  opening the target, and exclusive creation (`create_new`) protects the exact
  chosen path at the final write. A competing file, directory, or symlink at that
  path causes an error without overwrite, retry, or a switch to another format.

There are no lock files or final all-format refusal checks. A different supported
filename appearing after the initial check, including during guided questions,
does not prevent creation. Two init operations choosing the same filename cannot
overwrite each other; operations choosing different filenames can both succeed
if both initially observed no supported entries.

After a completed write, init makes a best-effort discovery of the supported
paths. It warns about observed duplicates without rollback or changing success,
even if a higher-priority manifest now governs the layer instead of the file it
just created. If this observation fails, init warns that the manifest was created
but duplicate checking failed; the inspection failure is non-fatal. Actual write
and result-reporting failures remain errors, as described below.

These checks are snapshots, not a promise to prevent or detect all concurrent
changes: another process can create a manifest after the final observation. A
duplicate warning describes observed entries; it does not prove another process
ran. These are ordinary local filesystem safeguards, not a hostile-filesystem
sandbox or a guarantee against every path-replacement race or network filesystem
behaviour.

## Output and errors

Success exits with status zero and prints the created manifest's path to standard
output. Errors exit with a nonzero status and print a diagnostic to standard
error rather than claiming success. Guided prompts and session feedback use
standard error; the result line remains on standard output. Flag-only output is
plain text, with no cliclack UI even in a terminal. For example:

```text
Error: '/path/to/project/layer.yaml' already exists. Initialisation was cancelled; no files were changed.
```

### Duplicate warnings

When discovery finds multiple supported names, init warns once per directory per
invocation on standard error, listing every detected filename in priority order
and identifying the selection. It reports this before refusing an initially
occupied target, without claiming an existing file was loaded. For example:

```text
Warning: Multiple layer manifests found in '/path/to/project': layer.yaml, layer.json. layer.yaml takes priority (priority: layer.yaml > layer.yml > layer.toml > layer.json > layer.jsonc). Manifests are not merged; keep only the intended manifest.
```

The same selection rule applies to the shared library reader and must be used by
future manifest-dependent commands, with duplicate diagnostics before any
dependent load or validation error. There is no warning for zero or one supported
entry. A warning alone does not change exit status; init still fails when its
initial check finds any occupied supported name.

If duplicates are observed only after successful creation, the warning is
followed by normal success output naming the file **actually created**. For
example, creating `layer.json` can succeed while warning that a newly observed
`layer.yaml` takes priority. Neither file is removed, renamed, or rewritten.
Deliberately keep only the manifest you intend to govern the layer; duplicates
are not combined or automatically repaired.

### Failures and cancellation

There is no overwrite or force option. Init does not read, validate, repair, or
merge an existing manifest. If an actual write fails, a partial newly created
manifest can remain; the error identifies its path. Inspect that file before
removing it and retrying. Init does not promise recovery from a crash or disk
failure.

Escape or Ctrl-C during any prompt, including the final format selector after
metadata is complete, cancels initialisation with a nonzero exit and no created
files. Unexpected input loss or a prompt I/O error also stops without creating a
manifest. If creation succeeded but reporting the result or terminal feedback
fails, the error instead explains that the manifest was already created; it does
not claim that no files changed or roll back that creation.

For usage, examples, or the version, without prompts, manifest discovery or
warnings, or writing files:

```sh
hydragen --help
hydragen init --help
hydragen --version
```

Argument-usage errors likewise do not scan manifests.

## Development checks

From the checkout's root:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
```

Tests use isolated temporary directories. Unit tests cover input-mode selection
without changing the test process's environment. Captured-output tests cover the
flag-only route, validation, all five output formats, help/version/usage,
repository boundaries, file preservation, duplicate diagnostics, concurrency,
and execution without Git or terminal input. Manifest tests cover shared
discovery, selected-only loading without fallback, parser contracts, and exact
string preservation, with independent output checks as well as round-trips.

For schema changes, check behaviour rather than just the schema's structure:
validate the schema against Draft 2020-12, exercise independently authored valid
and invalid data, compare field and complete-document validation, and validate
actual init output from all five formats with the production validator. Include
Unicode whitespace, `$schema` values, and additional properties while retaining
independent assertions for exact output, omissions, errors, and filesystem effects.
For the YAML parser corrections, check numeric spellings at range boundaries,
overflow rejection, binary-tag rejection for keys and values (including tag URIs
and directives), and exact preservation of quoted and `!!str` text in both YAML
extensions.

Also verify fresh builds and tests from a disposable source copy without
`openspec/`, schema inclusion in the source package, and execution with only the
built binary and no schema sidecar. These are verification requirements, not a
record that those checks have been run.

Unix-specific tests use a development-only pseudo-terminal harness and `/bin/sh`
to exercise real cliclack prompts, separate output streams, cancellation, input
loss, optional skips, retries, input flags in a terminal, and the final format
selector. Prompt waits and child-process waits are bounded. Deterministic
lifecycle tests exercise exact-target collisions and different-format creation
after the initial check, including duplicates and failed final observations.
Other Unix tests cover symlinks, output-reporting failure, repository-inspection
errors, and an actual write failure using a child-process file-size limit. That
limit does not affect your shell or other tests. The built CLI requires neither
a shell nor the terminal test harness.

The [manifest specification](openspec/specs/layer-manifest/spec.md) records the
metadata requirements; format, selection, and reader decisions are in the
[manifest-formats change](openspec/changes/support-layer-manifest-formats/design.md).
The [maintained-schema design](openspec/changes/use-maintained-manifest-schema/design.md)
explains the shared validation boundary. OpenSpec documents are planning and
decision references, not build, test, or runtime inputs. Archived schemas remain
historical snapshots and are not synchronized with the maintained schema.
