## Why

Initialising a layer and then adding its first dependency currently puts
`dependencies` before `name` in ordinary YAML layer manifests. Familiar field
placement should make files easier to scan across projects without turning a
presentation preference into a validity rule or rewriting developers' layouts.

## What Changes

- Establish a preferred order for documented layer manifest examples:
  `$schema` when present, `name`, `author`, `description`, then `dependencies`.
  Optional fields remain optional; init still generates only its existing metadata.
- Guarantee that init emits supplied metadata in `name`, `author`, `description`
  order across all five supported filenames.
- When add introduces a missing `dependencies` field, insert it after existing
  root fields rather than before them. Preserve TOML's root-table scope by inserting
  before explicit table sections. Do not move unknown fields to achieve an ideal
  order.
- Continue accepting otherwise valid layer manifests in any field order, without
  ordering warnings. Extend existing dependency lists where they already are,
  without sorting fields or dependency references.
- Retain insertion-only edits, comments and their established attachment, unrelated
  source bytes, decoded-data verification, and the existing safe-write lifecycle.
- Record the npm comparison: preferred output ordering is separate from accepted
  input, and sorting entries inside npm dependency objects is not a model for
  Hydragen's ordered reference list.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `layer-manifest`: Define advisory field order and explicitly accept any field
  order without diagnostics solely about placement.
- `layer-initialization`: Guarantee stable metadata order in generated files.
- `layer-dependency-addition`: Place newly introduced dependencies after existing
  root fields while preserving existing layouts and format-specific safety.

## Impact

- Focused implementation change in `src/layer_manifest/edit/yaml.rs`, especially
  missing-field insertion for block mappings; JSON/JSONC, flow YAML, and TOML
  editors need regression coverage for the shared placement contract.
- Lock in init's existing ordered struct serialisation in
  `src/layer_manifest.rs` and cover init-to-add workflows for every format.
- Update maintained examples and concise user guidance, with implementation
  guidance aligned in `AGENTS.md` and the decision linked from
  `openspec/design-questions.md`.
- No schema validity changes, new CLI options, formatter, dependencies, network
  operations, or migration of existing files. Previously generated files remain
  valid and are not automatically reordered.
