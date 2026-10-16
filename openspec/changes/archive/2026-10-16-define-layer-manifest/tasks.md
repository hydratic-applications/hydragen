## 1. Review the metadata schema

- [x] 1.1 Check `layer-manifest.schema.json` against the specification: only name is required; author and description are independently optional; each supplied metadata value must be a non-blank string. Preserve the metadata-only scope without adding naming rules, dependency reference syntax, or a required version. Keep the design clear about which editor-support and extensibility choices remain drafts.

## 2. Validate the schema and examples

- [x] 2.1 Validate the schema itself against JSON Schema Draft 2020-12. Check that name-only input, name with either optional field, and all three fields pass; missing name, blank or whitespace-only values, `null`, and other non-string metadata values fail. Validate the JSON examples in the design and confirm that allowing additional properties does not imply dependency validation.
- [x] 2.2 Record a repeatable validation command and its results in this change's design document, including the validator used. Do not add application code, CLI commands, or a runtime validation dependency to complete this design-and-schema change.

## 3. Check documentation consistency and readiness

- [x] 3.1 Review the proposal, design, specification, schema descriptions, glossary, design-question notes, and `add-layer-init` artifacts for consistent metadata requirements. Confirm that `layer.json` belongs at the repository root, while the schema checks contents rather than file location. Keep dependency sources, references, versioning, and inheritance mechanics open. Read any revised text as a first-time developer and check glossary cross-references and alignment with the foundations.
- [x] 3.2 Run `openspec validate define-layer-manifest --strict` and `git diff --check`, resolve any issues within this change's scope, and confirm that the completed work remains limited to design, schema, and documentation. Leave CLI implementation to `add-layer-init`.
