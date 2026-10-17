## Why

Hydragen's tests currently load a manifest schema from an archived OpenSpec change,
while the CLI independently implements its validation rules in Rust. A maintained
schema outside planning history should govern CLI validation, eliminating that
build/test dependency and giving future editor tooling the same definition of
valid manifest content.

## What Changes

- Establish `schemas/layer-manifest.schema.json` as the maintained manifest schema.
  Preserve the existing metadata-only rules and unknown-property handling; keep
  the archived schema unchanged as a historical snapshot, not a second maintained
  source.
- Bundle the maintained schema into the CLI and use it for runtime validation
  without network access, a schema sidecar file, or an external validator.
- Replace independently coded metadata constraints with shared schema-backed
  validation for flags, guided answers, directory-name defaults, and complete
  manifest content before file creation. Apply the same validator to complete
  documents decoded by the existing YAML/YML/TOML/JSON/JSONC loader before
  extracting metadata, without adding a manifest-reading command.
- Preserve friendly field-specific diagnostics, inline prompt retries, defaults,
  optional-field omission, accepted string contents, and existing interaction and
  filesystem-safety guarantees. Keep input collection, error presentation,
  repository-location checks, and safe file creation separate from schema rules.
- Replace structural-only schema assertions with real validation tests, including
  actual init output, invalid JSON content, and Unicode whitespace compatibility.
  Verify that the project builds and tests without `openspec/`, and that the
  executable runs without the source checkout or a separate schema file.
- Update the README to identify the maintained schema, explain its validation
  scope, and distinguish the shared asset from future editor publishing.
- Add guidance to `AGENTS.md` prohibiting source code, tests, and build scripts
  from loading OpenSpec artifacts. Required implementation assets must live in
  maintained project locations; manifest-content validation must use the schema
  rather than independently reproduce its constraints.
- Apply two parser corrections explicitly authorized during merge assessment:
  preserve recognized in-range YAML numeric extensions but reject their overflow
  instead of falling back to strings, and reject binary-tagged YAML keys/values
  rather than silently convert their bytes to text. These checks precede schema
  validation; they are not schema constraints.

The completed `support-layer-manifest-formats` work has been merged first. Preserve
its YAML default, format priority, serializers, and init lifecycle; retain parser
policies except the two explicitly authorized type-boundary corrections above.
JSON Schema validates their shared parsed data, not the source file's text syntax.

This increment does not add further manifest formats, schema generation,
new loading workflows, new commands, a public schema URL, generated `$schema`
properties, dependency-reference syntax, or schema-versioning policy. Schema
validation continues to cover metadata contents, not dependency semantics, file
location, or filesystem safety.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `layer-manifest`: Establish a maintained schema outside OpenSpec as the source
  used for CLI manifest-content validation, including init's individual input
  paths and complete output. Require offline, standalone use without changing the
  current metadata rules or claiming broader manifest validation.

## Impact

- **Code:** Introduce shared schema-validation logic in `src/manifest/schema.rs`
  and update `src/manifest.rs`, `src/lib.rs`,
  `src/main.rs`, and `src/interaction.rs` to use it. Remove the archived-schema
  include and the independent non-blank predicate.
- **Dependencies:** Add a Rust JSON Schema validator as a runtime dependency with
  unnecessary external-retrieval features disabled; update `Cargo.lock`.
- **Tests:** Add schema and reusable-validator coverage, extend captured-output
  init tests and existing bounded terminal tests, and verify standalone builds,
  packaged schema inclusion, and runtime operation.
- **Documentation:** Update `README.md` and `AGENTS.md`. Keep archived decisions
  and schemas intact. No change to the product foundations or glossary meaning
  is expected.
- **Compatibility:** Preserve command syntax, generated JSON, validation outcomes,
  friendly feedback, and no-overwrite behaviour. Investigate regex/Unicode
  differences before accepting the validator; do not silently change which
  metadata strings are valid. Preserve each non-JSON serializer too. One deliberate
  correction is that the loader will now reject invalid `$schema` values (such as
  blank strings, numbers, or `null`) that its previous metadata-only extraction
  did not check; this enforces the existing schema rule across every format.
