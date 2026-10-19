## MODIFIED Requirements

### Requirement: Write a minimal readable layer manifest in the selected format

A successful initialisation MUST write a UTF-8 layer manifest in the selected format,
ending in a newline. YAML and YML MUST use a readable YAML mapping; TOML MUST use
readable top-level key/value entries with quoted metadata strings and no `[layer]`
wrapper; JSON and JSONC MUST use two-space indented JSON. JSONC output MUST NOT require generated
comments or a header. The object/mapping MUST contain `name`, plus `author` and
`description` only when supplied, for this increment. It MUST NOT add a version,
schema address, dependency field, format field, or other properties.

The emitted root fields MUST appear in `name`, `author`, `description` order,
skipping omitted optional fields, in every supported format. Input flag order
MUST NOT change that output order. This output guarantee MUST NOT make other
field orders invalid when reading existing layer manifests.

Serialisation MUST preserve accepted metadata as strings, including values that
would otherwise be interpreted as YAML booleans, numbers, or nulls. Serialisation
and metadata validation MUST complete before opening the output file.

#### Scenario: Inspect a newly created layer manifest

- **WHEN** initialisation succeeds in any supported format
- **THEN** its fixed filename matches the selected extension and its content is readable in that format with a final newline
- **AND** its only properties are name and any author or description supplied by the developer
- **AND** those properties appear in name, author, description order with absent fields skipped

#### Scenario: A string resembles a YAML typed scalar

- **WHEN** accepted metadata is a string such as `true`, `123`, `null`, or `2026-10-16`
- **THEN** generated YAML uses appropriate quoting or escaping to preserve the original string when decoded

#### Scenario: Minimal TOML output preserves string metadata

- **WHEN** init successfully creates a TOML layer manifest with quotes, backslashes, newlines, or date-like text in accepted metadata
- **THEN** `layer.toml` contains appropriately quoted or escaped top-level strings and decoding recovers the exact original values
- **AND** omitted optional fields remain absent and no `[layer]` wrapper or starter comments are generated

#### Scenario: Input order does not dictate generated field order

- **WHEN** flag-only init supplies description before author and name, or guided init supplies equivalent metadata
- **THEN** both routes emit name, author, description in that order for the selected format
- **AND** supplying only name and description emits those two fields in that order without adding author
