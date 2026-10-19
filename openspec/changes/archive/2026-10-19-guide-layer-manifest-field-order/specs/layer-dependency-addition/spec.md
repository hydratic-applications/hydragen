## MODIFIED Requirements

### Requirement: Preserve comments and unrelated content when editing

Add MUST retain the selected layer manifest format and preserve comments in
YAML/YML, TOML, and JSONC, including existing dependency comments. It MUST preserve
metadata and unknown decoded data, and source text outside the edited dependency
declaration. It MUST NOT sort existing declarations, convert formats, broadly
reserialise the document, or discard comments as a fallback.

When `dependencies` is missing, add MUST introduce it after all existing root
entries without moving any of them, including unknown fields and `$schema`.
For TOML it MUST insert a root assignment after existing root assignments and
before explicit table or array-of-tables sections, without changing their scope
or separating leading comments from those sections. For flow YAML and JSON/JSONC,
the new field MUST be the last member of the root mapping/object before its
closing delimiter. For block YAML/YML, it MUST follow the complete final root
value and its attached comments, before root-level own-line footer comments and
any document-end marker. Comments outside the YAML document MUST remain outside
it. Existing inline and more-indented own-line comments MUST remain attached to
their original field or subtree.

When `dependencies` already exists, including an empty array or a field before
`name`, add MUST extend it in its existing position. It MUST preserve existing
root-field and dependency-reference order rather than normalising the layout.
Existing all-repeat plans MUST still bypass editing and staging.

Edits MUST retain existing source bytes in order, leading comments, indentation,
line-ending style, and final-newline presence. YAML directives, BOM, mapping/key
tags and anchors, block-scalar content including meaningful trailing newlines,
and document markers MUST retain their existing meaning and attachment. New
separator or line-break text MUST be inserted only where needed for the edit;
existing unrelated bytes MUST NOT be rewritten.

Before writing, the prepared source MUST be parsed with the established format
rules, validated with the maintained schema, and checked to preserve decoded
content except for the intended dependency append. If an edit cannot safely
preserve comments or unrelated meanings, it MUST fail with a specific explanation
without changing the original. This refusal MUST NOT replace ordinary editing
support for standard arrays, comments, nested data, and YAML block strings in every
supported format. Add MUST NOT fall back to prepending a missing dependency field
when safe tail insertion cannot be established.

#### Scenario: Comments survive a dependency append

- **WHEN** a supported commented layer manifest contains metadata, dependency comments, and unrelated custom fields
- **THEN** adding a valid source preserves those comments, unrelated source text, and decoded data
- **AND** adds the reference without changing format

#### Scenario: Dependencies are absent or empty

- **WHEN** a valid supported layer manifest omits dependencies or has an empty dependency array
- **THEN** add inserts or extends the declaration without removing unrelated text or comments
- **AND** a missing field is placed after existing root entries while an empty declaration is extended in place

#### Scenario: Alias editing would change unrelated meaning

- **WHEN** a YAML anchor/alias arrangement cannot be edited without affecting unrelated decoded data or losing comments
- **THEN** add explains the unsupported edit and leaves the source unchanged
- **AND** does not use generic reserialisation as a fallback

#### Scenario: Prepared output fails validation

- **WHEN** the planned edited source fails parsing, schema validity, or intended-change verification
- **THEN** the update stops before opening a staging file or changing the layer manifest

#### Scenario: Add the first dependency after initialisation

- **WHEN** init creates a layer manifest with name and any supplied author or description in any supported format and add then declares its first dependency
- **THEN** the file retains name, author, description order with omitted fields skipped, followed by dependencies
- **AND** accepted metadata values and existing source bytes remain unchanged

#### Scenario: Existing nonpreferred field order remains intact

- **WHEN** an otherwise valid layer manifest places dependencies before name and supplies description before author
- **THEN** add extends dependencies in place without moving any root field or warning about order
- **AND** existing references retain their order and genuinely new references append in argument order

#### Scenario: Unknown fields are not moved for ideal placement

- **WHEN** a layer manifest without dependencies has `$schema` and unknown fields interleaved with metadata
- **THEN** add places the new field after existing root entries without moving or interpreting those fields
- **AND** TOML explicit sections still follow the new root assignment with their existing leading comments

#### Scenario: YAML final value has nested data and attached comments

- **WHEN** a YAML or YML layer manifest without dependencies ends its root data with a nested collection and has inline or more-indented trailing comments followed by a root-level footer and a document-end marker
- **THEN** add inserts dependencies after the complete final value and its attached comments but before the footer and document-end marker
- **AND** leading comments and comments outside the document keep their original placement and attachment

#### Scenario: YAML final value is a block string

- **WHEN** a YAML or YML layer manifest without dependencies ends with an ordinary literal or folded block string, including strip, clip, or keep trailing-newline behaviour
- **THEN** add inserts dependencies after that value without changing its decoded string or meaningful blank lines
- **AND** a specific source arrangement that cannot safely support insertion fails before staging with an explanation rather than changing content or prepending the field

#### Scenario: Existing source conventions survive missing-field insertion

- **WHEN** a supported editable layer manifest without dependencies uses a supported line-ending style, Unicode text, and either includes or omits a final newline
- **THEN** add preserves those conventions while introducing the field after existing root entries
- **AND** YAML BOM, directives, indentation, explicit keys, tags, and anchors remain intact where supplied
