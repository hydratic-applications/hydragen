## Why

Hydragen currently requires `layer.json`, even when an author prefers YAML or TOML,
or needs comments in JSON. Supporting familiar formats with one documented
selection rule lets authors choose their editing format without changing the meaning of a layer
or leaving developers unsure which manifest governs it.

## What Changes

- Recognise exactly `layer.yaml`, `layer.yml`, `layer.toml`, `layer.json`, and
  `layer.jsonc` at the layer repository root, in that order of priority. The basename
  remains `layer`; the layer's metadata name does not determine its filename.
- Add shared, read-only discovery and loading APIs, using the same metadata rules
  for every format. Select by filename before parsing; never merge manifests or
  fall back when the selected manifest cannot be loaded or validated.
- Warn once per affected directory per applicable command invocation when multiple
  recognised manifest paths exist. Identify the paths and which takes priority,
  including when loading or initialisation subsequently fails. Help and version
  do not discover manifests or warn.
- **BREAKING:** Change init's default output from `layer.json` to `layer.yaml`.
  Add `--format yaml|yml|toml|json|jsonc`; an explicit format selects flag-only input.
  Add a final format selector to guided init, with `layer.yaml` preselected.
- Refuse initialisation if any recognised manifest path is occupied when init
  starts, without reading or altering its contents. Retain exact-target exclusive
  creation, but do not lock the directory or reject a completed write because a
  different manifest format appeared after the initial check. Warn about
  duplicates observed after a successful write instead.
- Document precedence, format syntax, output, warnings, and the concurrency limits;
  revise current JSON-only terminology while preserving archived decisions.

## Capabilities

### New Capabilities

None. Discovery and loading extend the existing manifest capability.

### Modified Capabilities

- `layer-manifest`: Supported fixed filenames, shared parsing and validation,
  deterministic selection, and duplicate diagnostics.
- `layer-initialization`: YAML default, selectable output format, all-format
  initial checks, exact-target no-overwrite, and post-creation duplicate warnings.
- `cli-interaction`: Format as an input flag, a final guided format selector, and
  all-format checks before prompts.

## Impact

- `src/lib.rs` and a focused new manifest module; library signatures and error
  types may change within this initial prototype. The CLI is the only production
  caller in this repository.
- `src/main.rs`, `src/interaction.rs`, `tests/init.rs`, and
  `tests/interactive_init.rs`, plus new manifest library tests.
- `Cargo.toml` and `Cargo.lock`: add maintained YAML, TOML, and JSONC parsers with only
  the features this change needs; retain Serde and strict `serde_json` support.
- Coordinate with the separate maintained-schema validation plan before editing
  the overlapping manifest module. Reuse its validator if it has landed; this
  change does not migrate the schema or introduce competing validation rules.
- `README.md`, `FOUNDATIONS.md`, `GLOSSARY.md`, `AGENTS.md`, and
  `openspec/design-questions.md` for current behaviour and decisions.
- Existing `layer.json` manifests remain supported unchanged. Scripts that assume
  init creates JSON must supply `--format json`.

## Non-goals

No JSON5, HJSON, INI, XML, or executable configuration support. No new read/validate
CLI command, conversion or migration command, automatic
renaming, manifest merging, custom manifest paths, format override for reading,
schema publishing or relocation, dependency-reference design, rendering, capture,
watching, lock files, or guarantees against duplicates created concurrently under
different filenames. Implementation is tracked in [tasks.md](tasks.md); no
additional CLI commands are included.
