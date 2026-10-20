## MODIFIED Requirements

### Requirement: Accept explicit non-blank metadata without prompts

Initialisation MUST use flag-only input whenever any input flag is supplied or
interaction is unavailable. The input flags MUST be `--name`, `--author`,
`--description`, and `--format`. This route MUST accept `--name` as an optional
override and `--author` and `--description` as independently optional flags,
without prompting. When name is not supplied, init MUST use the current
directory's final path component unchanged as the name. Guided input MUST offer
the same value as a displayed default accepted by Enter. Init MUST NOT infer other
metadata. Only when no input flags are supplied and interaction is available MUST
init collect input through the guided flow. Flags and answers MUST NOT be combined.

Name MUST satisfy the maintained schema's shared lowercase layer-name definition.
Author and description, when supplied, MUST retain their existing non-blank string
rules. Flags, guided answers, directory defaults, and complete candidates MUST use
that same schema. Init MUST preserve accepted values without silently lowercasing,
trimming, or replacing invalid characters. Init MUST NOT impose a stricter
independent naming rule outside the schema.

Invalid flag values or flags without a value MUST error rather than start a
replacement question. A schema-invalid directory default MUST fail before prompts
or writes with guidance to supply `--name`; a valid explicit name MUST bypass that
default. Absent or interactively skipped optional fields MUST be omitted, not
written as empty strings or `null`. Existing target checks, input routes, and
exclusive creation MUST remain unchanged.

#### Scenario: Only name is supplied

- **WHEN** init receives a schema-valid name flag without author or description in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it creates a layer manifest containing only name without prompts

#### Scenario: One optional field is supplied

- **WHEN** init receives a valid name and either author or description as a non-blank string
- **AND** all other initialisation conditions are satisfied
- **THEN** it records both accepted fields unchanged and omits the other optional field

#### Scenario: All metadata is supplied

- **WHEN** init receives a valid name and non-blank author and description flags
- **THEN** it records those values unchanged
- **AND** asks no metadata questions

#### Scenario: Name override is omitted

- **WHEN** init is flag-only or interaction is unavailable, `--name` is omitted, and the directory's UTF-8 name satisfies the shared name definition
- **AND** other initialisation conditions are satisfied
- **THEN** it uses that directory name unchanged, records supplied optional metadata, and asks no questions

#### Scenario: No usable directory-name default

- **WHEN** no name override is supplied and the directory has no final component, invalid UTF-8, or a name that violates the schema, including uppercase or spaces
- **THEN** init fails before prompts or writes with guidance to supply `--name`
- **AND** does not invent, lowercase, or sanitise a name
- **AND** a valid explicit name bypasses default derivation

#### Scenario: Metadata is blank

- **WHEN** a metadata flag is empty or whitespace-only
- **THEN** init fails with field-specific feedback and no created file or replacement prompt

#### Scenario: Metadata includes format-sensitive characters

- **WHEN** accepted optional metadata includes quotes, backslashes, newlines, whitespace, or non-ASCII characters alongside a valid name
- **THEN** output in the chosen format decodes to those original strings

#### Scenario: Uppercase name is supplied explicitly

- **WHEN** init receives `--name Shared-Config`
- **THEN** it rejects the name without silently converting it to `shared-config`
- **AND** creates no file and opens no replacement prompt

#### Scenario: Invalid guided name can be corrected

- **WHEN** the directory default is valid but a guided answer violates a name constraint
- **THEN** schema-derived prompt feedback explains the correction without referring to flags
- **AND** the developer can edit the answer before any layer manifest is written
