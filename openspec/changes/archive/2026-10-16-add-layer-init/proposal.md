## Why

Hydragen has product foundations and a draft layer manifest format, but no runnable
tool. Initialising a layer is a small first developer action that makes those
decisions usable without designing inheritance, workspaces, or dependency
management at the same time.

## What Changes

- Introduce a Rust CLI with a `hydragen init` command that creates `layer.json`
  in the current directory using a required name and optional author and description.
- Validate name and any supplied optional metadata as non-blank strings, matching
  the layer manifest schema; omit optional fields when they are not supplied.
- Initialise a layer, not a separate category of project. Whether a layer has
  dependencies determines whether it is a root or dependent layer; there is no
  type flag or permanent classification at initialisation.
- Omit dependencies from the initial layer manifest. Adding and resolving dependencies
  remains future work, rather than a restriction on what the layer can become.
- Preserve existing files, refuse to replace an existing layer manifest, and provide
  clear help, success messages, and errors without requiring interactive input.
- Allow initialisation before a Git repository exists, while guarding against
  accidentally placing the layer manifest below an existing repository's root.
- Add focused tests and local build and usage instructions.

## Capabilities

### New Capabilities

- `layer-initialization`: Create a layer manifest safely through `hydragen init`,
  with explicit metadata, predictable command-line behaviour, and no dependency
  management or Git mutations.

### Modified Capabilities

None. This change uses the existing
[layer-manifest requirements](../../../specs/layer-manifest/spec.md) without changing
them: required name, optional author and description, non-blank string validation,
and the agreed layer manifest location.

## Impact

- Add the first Rust package and `hydragen` executable to this repository.
- Introduce command-line parsing and JSON serialisation dependencies, a Cargo
  lockfile, and tests. No runtime network access or AI service is required.
- Add developer documentation for building, testing, and trying the command.
- Record the initialisation decisions in this change and link the relevant
  partial resolutions from `openspec/design-questions.md`.
- Do not implement rendering, contribution capture, dependency references,
  patches, deletion markers, publishing, or agent-specific integrations.
