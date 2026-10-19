## ADDED Requirements

### Requirement: Field order is a presentation convention rather than a validity rule

Hydragen MUST accept otherwise valid layer manifests regardless of the order of
root properties in YAML, YML, TOML, JSON, and JSONC. It MUST NOT reject, warn,
change exit status, or rewrite a file solely because its fields use a different
order. This rule MUST NOT relax format syntax, duplicate-key checks, content
validation, or dependency-relationship checks.

Maintained general-usage examples MUST present supplied known fields in the
preferred order `$schema`, `name`, `author`, `description`, `dependencies`, skipping
absent fields. Examples specifically demonstrating accepted alternative layouts
MUST remain permitted. Additional properties MUST have no prescribed placement.
This convention MUST NOT require inserting optional fields, retrieving `$schema`,
or sorting nested objects or dependency arrays. Existing init field omission and
add's source-preserving placement rules MUST take precedence over normalising an
existing file to this preferred order.

#### Scenario: Dependencies precede identifying metadata

- **WHEN** an otherwise valid layer manifest declares dependencies before name
- **THEN** shared loading and content validation accept it in every supported format
- **AND** commands emit no diagnostic solely about field order

#### Scenario: Optional and additional properties use another order

- **WHEN** an otherwise valid layer manifest places description before author or interleaves additional properties and a supplied `$schema`
- **THEN** loading preserves its decoded values and source without order-specific diagnostics or rewriting

#### Scenario: Preferred layout is shown to developers

- **WHEN** a maintained general-usage example shows all currently defined root fields
- **THEN** it presents `$schema`, name, author, description, and dependencies in that order
- **AND** explains that field placement is a convention rather than a validity condition

#### Scenario: Ordering flexibility does not bypass content or syntax rules

- **WHEN** a layer manifest has repeated keys, invalid known-field content, or invalid format syntax
- **THEN** existing checks still reject it regardless of field order
- **AND** a TOML key assigned within a named table is not interpreted as a root key merely because its name matches one
