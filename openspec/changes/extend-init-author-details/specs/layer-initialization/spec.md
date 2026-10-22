## MODIFIED Requirements

### Requirement: Accept explicit non-blank metadata without prompts

Initialisation MUST use flag-only input whenever any input flag is supplied or
interaction is unavailable. The input flags MUST be `--name`, `--author`,
`--author-email`, `--author-url`, `--description`, and `--format`. This route MUST
accept `--name` as an optional override and author and description as independently
optional, without prompting. Contact flags MUST require `--author`. When name is
not supplied, init MUST use the current directory's final path component unchanged
as the layer name. Guided input MUST offer the same value as a displayed default
accepted by Enter. Init MUST NOT infer unsupplied metadata on the flag-only route.
Only when no input flags are supplied and interaction is available MUST init
collect input through the guided CLI interaction flow. It MUST NOT combine flags
and prompt answers in one invocation.

Layer name MUST satisfy the maintained schema's shared lowercase layer-name
definition. Author input MUST become an object with a schema-valid non-blank
name and independently optional email and URL. `--author` MUST accept a literal
name or recognised npm shorthand through the explicit author-input requirement.
Description MUST retain its existing non-blank string rule. Flags, guided answers,
directory defaults, and complete candidates MUST use the same maintained schema.
Init MUST preserve accepted values without silently lowercasing, trimming, or
replacing invalid characters, apart from removing recognised shorthand syntax.
Init MUST NOT impose an independent content grammar outside the schema.

Invalid flag values or flags without a value MUST error rather than start a
replacement question. Contact flags without `--author` MUST be clap usage failures
before operational dispatch. A schema-invalid directory default MUST fail before
prompts or writes with guidance to supply `--name`; a valid explicit name MUST
bypass that default. Absent or interactively skipped optional fields MUST be
omitted, not written as empty strings or `null`. Existing target checks, input
routes, and exclusive creation MUST remain unchanged.

#### Scenario: Only name is supplied

- **WHEN** initialisation receives a schema-valid name flag without author or description flags, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and creates a layer manifest containing only name without prompts

#### Scenario: One optional field is supplied

- **WHEN** initialisation receives a valid name flag and either a non-blank author name or description flag, in any terminal or CI state
- **AND** all other initialisation conditions are satisfied
- **THEN** it succeeds and records the supplied author as an object or description as a string, preserving accepted component values
- **AND** it omits the other optional field

#### Scenario: All metadata is supplied

- **WHEN** initialisation receives a valid layer name, author, optional contacts, and description as flags
- **THEN** it records the accepted author components in an author object and description unchanged
- **AND** it asks no metadata questions and performs no author discovery

#### Scenario: Name override is omitted

- **WHEN** an invocation is flag-only or interaction is unavailable, and it omits `--name`
- **AND** the current directory's UTF-8 name satisfies the shared name definition and other initialisation conditions are satisfied
- **THEN** the command succeeds using that directory name unchanged
- **AND** it records any supplied author or description, omits unsupplied fields, and asks no questions

#### Scenario: No usable directory-name default

- **WHEN** no name override is supplied and the current directory has no final path component, has a schema-invalid name including uppercase or spaces, or its name is not valid UTF-8
- **THEN** init fails with guidance to supply `--name` explicitly before prompts or writes
- **AND** a valid explicit `--name` bypasses directory-name derivation
- **AND** it does not invent, lowercase, or sanitise a name

#### Scenario: Metadata is blank

- **WHEN** any metadata value supplied as a flag is empty or consists entirely of whitespace
- **THEN** the command exits unsuccessfully and identifies the invalid field and flag
- **AND** it creates no files or replacement prompts

#### Scenario: Metadata includes format-sensitive characters

- **WHEN** accepted optional metadata includes quotes, backslashes, newlines, whitespace, or non-ASCII characters alongside a valid layer name
- **THEN** the created layer manifest is valid in the chosen format and decoding it recovers the original accepted component strings

#### Scenario: Uppercase name is supplied explicitly

- **WHEN** init receives `--name Shared-Config`
- **THEN** it rejects the name without silently converting it to `shared-config`
- **AND** creates no file and opens no replacement prompt

#### Scenario: Invalid guided name can be corrected

- **WHEN** the directory default is valid but a guided answer violates a name constraint
- **THEN** schema-derived prompt feedback explains the correction without referring to flags
- **AND** the developer can edit the answer before any layer manifest is written

### Requirement: Write a minimal readable layer manifest in the selected format

A successful initialisation MUST write a UTF-8 layer manifest in the selected
format, ending in a newline. YAML and YML MUST use a readable YAML mapping with
a nested mapping for author. TOML MUST use readable top-level key/value entries
with quoted strings and a root inline table for author, without a `[layer]` wrapper.
JSON and JSONC MUST use two-space indented JSON with a nested author object. JSONC
output MUST NOT require generated comments or a header. The object/mapping MUST
contain `name`, plus `author` and `description` only when supplied. Author MUST
use the object form even when only its name is supplied, without asking which
representation to write. Its only generated fields MUST be `name` and supplied
`email` and `url`. Init MUST NOT add a version, schema address, dependency field,
format field, or other properties.

The emitted root fields MUST appear in `name`, `author`, `description` order,
skipping omitted optional fields, in every supported format. Generated author
fields MUST use `name`, `email`, `url` order, skipping omissions. Input flag order
MUST NOT change those output orders. These output guarantees MUST NOT make other
field orders invalid when reading existing layer manifests.

Serialisation MUST preserve accepted component strings, including values that
would otherwise be interpreted as YAML booleans, numbers, or nulls. Serialisation
and complete metadata validation MUST finish before opening the output file.

#### Scenario: Inspect a newly created layer manifest

- **WHEN** initialisation succeeds in any supported format
- **THEN** its fixed filename matches the selected extension and its content is readable in that format with a final newline
- **AND** its only root properties are name and any author or description supplied by the developer
- **AND** those properties appear in name, author, description order with absent fields skipped
- **AND** a supplied author is an object with its supplied known fields in name, email, url order

#### Scenario: A string resembles a YAML typed scalar

- **WHEN** accepted metadata is a string such as `true`, `123`, `null`, or `2026-10-16`
- **THEN** generated YAML uses appropriate quoting or escaping to preserve the original string when decoded, including within author

#### Scenario: Minimal TOML output preserves string metadata

- **WHEN** init successfully creates a TOML layer manifest with quotes, backslashes, newlines, or date-like text in accepted metadata
- **THEN** `layer.toml` quotes or escapes the strings appropriately, using a root inline table for author, and decoding recovers the exact accepted values
- **AND** description remains a root string, omitted optional fields remain absent, and no `[layer]` wrapper or starter comments are generated

#### Scenario: Input order does not dictate generated field order

- **WHEN** flag-only init supplies description before author and name, or guided init supplies equivalent metadata
- **THEN** both routes emit name, author, description in that order for the selected format
- **AND** supplying only name and description emits those two fields in that order without adding author

## ADDED Requirements

### Requirement: Interpret explicit author shorthand without inferring missing contacts

Custom Author text and `--author` MUST recognise full, unambiguous npm shorthand
with a name and `<email>`, `(URL)`, or both in that order. URL suffix detection
MUST require an explicit URI scheme. Recognised components MUST use their shared
schema definitions; invalid captured components MUST report their component
failure rather than silently become literal name text. Input not recognised as
shorthand MUST be treated as the literal author name and validated through the
name definition. Empty guided input MUST mean author omission; explicitly empty
flags MUST remain invalid. Parsed author objects MUST always have a valid name.

Interpretation MUST remove only recognised contact delimiters and the separating
whitespace immediately before the suffix, preserving name and contact payloads
without other normalisation. Name-only input MUST remain unchanged. Explicit
object names, Git/account name values, and selected discovery labels MUST NOT be
reinterpreted as shorthand. File loading MUST retain the original string/object
representation instead of applying init's input conversion.

`--author-email` and `--author-url` MUST supply explicit contacts without discovery.
If shorthand and a contact flag supply the same field, exact equal valid values
MUST be accepted once; differing values MUST fail before writes with both input
sources identified. Every explicit value MUST be validated before equality
deduplication. Init MUST NOT silently select an override or start questions.

#### Scenario: Name and email shorthand

- **WHEN** explicit Author input is `Alex <alex@example.com>`
- **THEN** init constructs name `Alex` and email `alex@example.com`
- **AND** guided init skips Author email and asks an empty optional Author URL input

#### Scenario: Name and URL shorthand

- **WHEN** explicit Author input is `Alex (https://alex.example)`
- **THEN** init constructs name `Alex` and URL `https://alex.example`
- **AND** guided init asks an empty optional Author email input and skips Author URL

#### Scenario: Complete shorthand

- **WHEN** explicit Author input is `Alex <alex@example.com> (https://alex.example)`
- **THEN** init constructs the three author fields and guided init goes directly to Description
- **AND** no author-representation question or duplicate contact prompt appears, while the final layer manifest format selector remains

#### Scenario: Literal custom name

- **WHEN** explicit Author input is `Example team` or `Alex (team)`
- **THEN** it becomes the literal name unchanged
- **AND** guided contact inputs have no suggestions or defaults

#### Scenario: A detected contact violates its schema

- **WHEN** recognised shorthand contains a schema-invalid email or URI
- **THEN** guided input receives useful component-specific retry feedback and flag-only input fails with `--author` context
- **AND** no file is written and the value does not silently become an accepted literal name

#### Scenario: Explicit contact flags agree or conflict

- **WHEN** `--author 'Alex <alex@example.com>'` is supplied with `--author-email alex@example.com`
- **THEN** init records that email once without prompts or discovery
- **AND** using a different or invalid explicit email fails before writing, identifying the relevant flags and cause

#### Scenario: Contact flag has no author

- **WHEN** a contact flag is supplied without `--author`
- **THEN** clap reports a usage failure before current-directory resolution, discovery, schema setup, prompts, or writes
- **AND** a directory-name default cannot substitute for the missing author name
