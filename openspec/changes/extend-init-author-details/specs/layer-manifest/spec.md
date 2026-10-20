## MODIFIED Requirements

### Requirement: Every layer has identifying metadata

Every layer MUST have a layer manifest declaring its name. The name MUST satisfy
the shared layer-name requirements. Author and description MUST be optional
independently of each other. Description, when supplied, MUST be a string
containing at least one non-whitespace character. Author MUST be a non-blank
string or an object with a required non-blank `name` and independently optional
`email` and `url`, according to the author representation requirement. Accepted
values MUST be preserved without normalisation. These rules MUST apply to both
root and dependent layers, whether intended primarily for reuse, as working
projects, or both.

#### Scenario: Root layer metadata

- **WHEN** a root layer is defined
- **THEN** its layer manifest declares a valid layer name
- **AND** author and description can each be omitted

#### Scenario: Dependent layer metadata

- **WHEN** a dependent layer is defined
- **THEN** its layer manifest declares a valid layer name
- **AND** author and description can each be omitted

### Requirement: A schema describes metadata requirements

The layer manifest format MUST have a maintained JSON Schema at
`schemas/layer-manifest.schema.json`, using JSON Schema Draft 2020-12. The schema
MUST require an object with a valid layer name, permit optional author in its
string/object forms and optional description with its non-blank string rule,
and define the optional ordered dependency array and source-reference syntax.
Optional metadata MUST be independently omittable, not represented by blank
strings or `null`.

Shared definitions in that schema MUST determine name, author, nested author
fields, and reference content validity, including exact published versions.
Field checks, parsed-content checks, every format loader, init input/defaults,
and complete candidates before writing MUST use those definitions rather than
duplicate constraints in independent predicates or separate schemas.

The same content rules MUST apply after parsing every supported format. The CLI
MUST NOT require an external runtime validator or add a schema property to
generated layer manifests. The schema MUST permit an optional `$schema` property
containing a non-blank string and MUST permit unknown properties without
interpreting them. A supplied `$schema` MUST NOT select another validator or cause
retrieval. Unknown-field acceptance MUST NOT make an invalid known `dependencies`
or author field acceptable.

Schema validation MUST NOT perform filesystem or network access, establish
cross-layer name uniqueness, choose registry versions, define how layers combine,
or replace filename, repository-location, or safe-write checks.

#### Scenario: Missing name

- **WHEN** a layer manifest omits name
- **THEN** it fails validation against the schema in every supported format

#### Scenario: Optional metadata is omitted

- **WHEN** a layer manifest contains a valid name and omits author and description
- **THEN** it passes content validation in every supported format when its other known fields are valid

#### Scenario: Optional metadata is supplied independently

- **WHEN** a layer manifest contains a valid name and supplies a valid string/object author, a non-blank description, or both
- **THEN** those metadata values pass validation in every supported format
- **AND** their accepted content and author representation are retained unchanged

#### Scenario: Supplied metadata is invalid

- **WHEN** name or a supplied description is blank, whitespace-only, or not a string, including `null`, or author violates its string/object definition
- **THEN** it fails validation against the schema in every format that can represent the supplied value
- **AND** the loader does not coerce invalid data or treat explicit null as omission

#### Scenario: Name violates the new naming policy

- **WHEN** a layer manifest uses an uppercase, overlength, or otherwise invalid layer name
- **THEN** field and complete-content validation reject it through the maintained schema
- **AND** no path silently normalises it into a valid name

#### Scenario: Content is not an object

- **WHEN** layer manifest content is a JSON string, number, boolean, array, or `null`
- **THEN** it fails validation against the schema

#### Scenario: Editor schema reference is supplied

- **WHEN** otherwise valid content includes a `$schema` property
- **THEN** content validation accepts a non-blank string value for that property
- **AND** rejects blank, whitespace-only, or non-string values
- **AND** validation does not retrieve the referenced address or path or use it to replace the bundled schema

#### Scenario: Content includes an additional property

- **WHEN** otherwise valid content includes a property outside the schema's defined properties
- **THEN** content validation accepts it without interpreting that property's value
- **AND** this result does not establish that the unknown property's behaviour is supported

#### Scenario: Schema document is checked

- **WHEN** the maintained schema is validated against the Draft 2020-12 meta-schema
- **THEN** it is a valid JSON Schema document for that draft

### Requirement: Parse familiar formats into the same layer manifest data

Each parser MUST produce one object/mapping document for shared content
validation. Accepted string values MUST retain their content without trimming or
normalisation. Additional fields MUST be accepted and retained as decoded data,
without implementing their behaviour. Parsing MUST remain separate from reference
classification, schema validation, and dependency relationship checks.
Repeated object/mapping keys MUST be rejected before a map discards duplicates,
including duplicate unknown fields and nested keys. JSON key comparison MUST use
decoded names. Supported data MUST use string object keys and JSON-compatible
values; unsupported complex YAML keys, TOML native date/time literals, and
non-finite numeric values MUST NOT be silently converted to other data.

Strict JSON MUST reject comments, trailing commas, and JSON5 extensions. JSONC
MUST accept line and block comments and trailing commas, but MUST reject unquoted
keys, single-quoted strings, missing commas, and other JSON5-only extensions.
Empty or comment-only input, multiple top-level values, non-object input, invalid
syntax, and invalid UTF-8 MUST fail loading with a path-specific error.

YAML and YML MUST use the same maintained parser and ordinary YAML 1.2 data
behaviour, including comments, normal strings, block strings, standard string
tags, and bounded anchors/aliases. YAML input MUST contain one mapping document.
The loader MUST NOT implement custom tags, file inclusion, environment
substitution, or YAML 1.1 merge-key composition. Parser safety limits MUST NOT be
disabled to accept unbounded alias expansion or nesting.

TOML MUST use a maintained standard TOML parser and a top-level table without a
required `[layer]` wrapper. It MUST accept ordinary comments, basic/literal and
multiline strings, dotted keys, tables, inline tables, and arrays of tables when
their decoded values satisfy the shared layer manifest data model and content
schema. Duplicate keys and invalid table redefinitions MUST be rejected according to TOML rules; valid
arrays of tables MUST NOT be treated as duplicate declarations.

TOML values MUST retain their native types until conversion to the shared data
model. The loader MUST NOT stringify numbers, booleans, or date/time literals,
or expose a Serde datetime wrapper as a user-supplied object. Native date/time
literals and non-finite numbers MUST fail with a path-specific unsupported-data
diagnostic; date/time-like text MUST be quoted to be a layer manifest string. Omitted
optional keys MUST remain absent. The loader MUST NOT invent a TOML null syntax.
Read-only loading MUST NOT rewrite source files. Author objects MUST be supported
as YAML mappings, JSON/JSONC objects, and TOML inline tables, dotted keys, or
explicit `[author]` tables; arrays of authors MUST NOT become a single author.

#### Scenario: Equivalent metadata across formats

- **WHEN** the same valid name, string/object author, and description are expressed in each supported format
- **THEN** loading produces identical metadata without normalisation
- **AND** accepted string whitespace, Unicode, quotes, newlines, author representation, and additional author data are retained

#### Scenario: JSONC comments and trailing commas

- **WHEN** a JSONC layer manifest contains line comments, block comments, or trailing commas
- **THEN** those constructs are accepted without changing its decoded data
- **AND** the same constructs in a JSON layer manifest fail parsing

#### Scenario: JSONC is not JSON5

- **WHEN** a JSONC layer manifest contains an unquoted key, single-quoted string, or another disabled permissive extension
- **THEN** loading fails with a diagnostic identifying the selected layer manifest

#### Scenario: Ordinary YAML strings and aliases

- **WHEN** a YAML or YML layer manifest uses comments, a block description string, or an ordinary alias to a string anchor
- **THEN** loading accepts them with their normal YAML meaning and the shared content schema
- **AND** an explicitly tagged `!!str` value is still treated as a string

#### Scenario: YAML scalar types are not coerced

- **WHEN** YAML metadata contains an unquoted boolean, number, null, or empty value where its schema requires a string or author object
- **THEN** content validation rejects the invalid value without string conversion
- **AND** quoting a value permits it only when the resulting string satisfies that field's schema
- **AND** ordinary YAML 1.2 plain strings such as `yes` or `on` remain strings

#### Scenario: Repeated keys within a layer manifest

- **WHEN** a supported layer manifest repeats a key at any object/mapping depth, including an unknown field, author member, or an escaped spelling of the same JSON key
- **THEN** loading reports a duplicate-key error rather than taking the first or last value

#### Scenario: Ordinary TOML content

- **WHEN** TOML supplies ordinary top-level metadata, an ordered dependency string array, and supported unknown tables or arrays of tables
- **THEN** loading applies the same shared schema and retains additional decoded data
- **AND** it does not require a `[layer]` wrapper or perform dependency filesystem resolution

#### Scenario: TOML typed metadata is not coerced

- **WHEN** TOML supplies a boolean, number, array, or native date/time literal as name, author, or description, or a table as name or description
- **THEN** loading rejects the invalid metadata or unsupported data without converting it to a string
- **AND** an author table is accepted only when it satisfies the author-object schema
- **AND** quoted text is accepted only when it satisfies the field's schema

#### Scenario: TOML native values outside the common data model

- **WHEN** a TOML layer manifest contains a native date/time literal or non-finite float, including in an additional field
- **THEN** loading reports unsupported data at the selected path rather than converting it to a string, null, or internal wrapper object

#### Scenario: TOML duplicate keys or invalid table redefinitions

- **WHEN** TOML defines the same decoded key twice or uses conflicting dotted keys or table declarations
- **THEN** loading reports the parser's duplicate or redefinition error rather than discarding a value

#### Scenario: Additional fields remain uninterpreted

- **WHEN** a valid layer manifest includes an additional field with supported decoded data
- **THEN** loading preserves that data without interpreting the unknown field or claiming dependency relationship validity
- **AND** no source file is rewritten

#### Scenario: Unsupported YAML composition or document stream

- **WHEN** YAML input contains multiple documents, a merge key, an unsupported custom tag, or an include request
- **THEN** loading rejects it rather than adding composition, inclusion, or fallback behaviour

### Requirement: Schema-backed validation preserves useful feedback and metadata semantics

Schema-backed validation MUST enforce the shared name policy and preserve the
non-blank semantics for author strings, author-object names, and description,
including rejection of ASCII- and Unicode-whitespace-only strings. Author-object
contacts MUST use their maintained non-blank and format definitions. Accepted
values MUST remain unchanged. Field-specific errors and guided retries MUST derive
from structured schema failures, not duplicated predicates or matching
human-readable validator text. Non-blank, name, contact-format, and reference
failures MUST receive accurate field/content-path and correction information
rather than all being described as blank metadata. Author union validation MUST
retain the useful nested failure rather than only an opaque union error.

An invalid bundled schema or validator configuration MUST be distinguishable from
invalid user input. Such internal failures MUST cause a nonzero exit before
prompts or writes, without panic, retry prompts, success output, or misleading
guidance to change valid metadata. Existing interaction selection, output-stream
separation, cancellation, and file-safety requirements MUST remain unchanged.
Adding this command MUST NOT enable guided add input.

#### Scenario: Whitespace-only metadata includes Unicode characters

- **WHEN** a supplied metadata string consists only of whitespace, including ASCII whitespace, NBSP, EM SPACE, or U+0085
- **THEN** schema-backed validation rejects it consistently across fields and complete-content checks
- **AND** a non-blank author name or description remains accepted and preserved unchanged, including surrounding whitespace
- **AND** layer names and author contacts additionally satisfy their own schema rules

#### Scenario: A known non-blank constraint fails

- **WHEN** the schema reports that a supplied metadata string violates its non-blank constraint
- **THEN** CLI feedback identifies the field and explains the need for a non-whitespace character
- **AND** guided init allows correction while supplied-input routes fail without a replacement question

#### Scenario: A different schema constraint fails

- **WHEN** a name, dependency reference, author contact, or other property violates a schema constraint other than the known non-blank constraint
- **THEN** the diagnostic reflects the failed rule and affected field or content path
- **AND** it does not incorrectly describe every failure as blank metadata

#### Scenario: Bundled schema cannot be prepared

- **WHEN** the bundled schema is invalid or validator configuration fails
- **THEN** init or add reports an internal validation setup failure and exits nonzero before prompts or writes
- **AND** it does not panic, report an unusable directory-name default, or ask the user to retry metadata

## ADDED Requirements

### Requirement: Author supports familiar string and object representations

The maintained schema MUST accept author as an existing non-blank string or an
object requiring a non-blank `name`. `email` and `url` MUST be independently
optional non-blank strings. Email MUST satisfy JSON Schema `email` format; URL
MUST satisfy `uri` format as an absolute URI. The runtime validator MUST enable
format assertions offline without checking reachability or ownership. Objects
without a name, explicit nulls, wrong known-field types, blank known fields, and
invalid contact formats MUST fail validation. Unknown author-object properties
MUST be accepted and retained without interpretation, not excuse invalid known
fields. Duplicate keys at any depth MUST remain decoder errors.

String authors MUST retain their existing non-blank validity and exact decoded
content, including name-only values and npm shorthand with optional `<email>`
and `(URL)` parts. Loading MUST NOT tighten their grammar, extract contacts,
rewrite their representation, or require migration. A string not interpreted as
shorthand MUST remain a literal author name. Author metadata MUST NOT imply a
verified identity or restrict authors to individuals rather than organisations.

Shared field validation MUST expose author as a whole and each nested known
field through definitions in `schemas/layer-manifest.schema.json`. Nested
validation MUST NOT require fabricating unrelated values. Unknown selectors,
missing definitions, or invalid schema configuration MUST fail explicitly before
guided interaction. All formats, input routes, discovered fields, library callers,
and complete write candidates MUST use these same definitions.

#### Scenario: Supported npm-style representations

- **WHEN** otherwise valid content supplies `Alex`, `Alex <alex@example.com> (https://alex.example)`, or an author object with name and either or both contacts
- **THEN** the schema accepts the supported representation in all five formats
- **AND** loading preserves that representation and its decoded content without rewriting source

#### Scenario: Name is required on an author object

- **WHEN** author is `{}`, an email-only object, a URL-only object, or an object with a blank/non-string name
- **THEN** validation fails with a useful author/name path and required, type, or non-blank cause
- **AND** the same rule applies to direct library validation and complete init candidates

#### Scenario: Optional contacts are absent or invalid

- **WHEN** an author object has a valid name and omits either contact
- **THEN** omission is valid without creating an empty or null property
- **AND** supplying a blank, null, non-string, malformed email, or relative URL fails its corresponding definition

#### Scenario: Nested validation uses the maintained schema

- **WHEN** a caller validates an author name, email, or URL independently
- **THEN** validation uses its maintained nested definition with an accurate `/author/name`, `/author/email`, or `/author/url` instance path
- **AND** content-policy changes in that schema affect nested and complete validation together

#### Scenario: Add preserves an existing author representation

- **WHEN** add appends dependencies to a layer manifest with a string or object author, including comments and extra nested author data
- **THEN** the append leaves author source bytes, representation, decoded values, and associated comments unchanged
- **AND** candidate verification still proves only the intended dependency append changed decoded data

#### Scenario: Contact format assertions are effective but offline

- **WHEN** object author email is `not-an-email` or URL is `relative/path`
- **THEN** both field and complete validation reject the corresponding value without network access
- **AND** valid syntactic contacts are not rejected merely because a mailbox or website is unreachable
