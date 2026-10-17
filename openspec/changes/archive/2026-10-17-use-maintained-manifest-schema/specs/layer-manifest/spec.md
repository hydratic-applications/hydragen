## MODIFIED Requirements

### Requirement: A schema describes metadata requirements

The layer manifest format MUST have a maintained JSON Schema at
`schemas/layer-manifest.schema.json`, using JSON Schema Draft 2020-12. The schema
MUST require an object with name and permit optional author and description, with
non-blank string validation for each supplied metadata field. Optional metadata
MUST be independently omittable, not represented by blank strings or `null`.

The same metadata rules MUST apply after parsing every supported layer manifest format.
The CLI MUST NOT require an external runtime schema validator or add a schema
property to generated layer manifests.

The schema MUST permit an optional `$schema` property containing a non-blank
string and MUST permit additional properties without interpreting them. Validation
MUST cover metadata only; it MUST NOT establish dependency reference syntax, how
versioning works or whether it belongs in the layer manifest, or how layers combine.
Schema validation MUST NOT substitute for filename, repository-location, or safe
file-creation checks.

#### Scenario: Missing name

- **WHEN** a layer manifest omits name
- **THEN** it fails validation against the schema in every supported format

#### Scenario: Optional metadata is omitted

- **WHEN** a layer manifest contains a valid name and omits author and description
- **THEN** it passes metadata validation against the schema in every supported format

#### Scenario: Optional metadata is supplied independently

- **WHEN** a layer manifest contains a valid name and supplies either or both of author and description as non-blank strings
- **THEN** it passes metadata validation against the schema in every supported format

#### Scenario: Supplied metadata is invalid

- **WHEN** name or a supplied author or description is blank, whitespace-only, or not a string, including `null`
- **THEN** it fails validation against the schema in every format that can represent the supplied value
- **AND** the loader does not turn a non-string value into a string or treat explicit null as omission

#### Scenario: Content is not an object

- **WHEN** layer manifest content is a JSON string, number, boolean, array, or `null`
- **THEN** it fails validation against the schema

#### Scenario: Editor schema reference is supplied

- **WHEN** otherwise valid metadata includes a `$schema` property
- **THEN** metadata validation accepts a non-blank string value for that property
- **AND** rejects blank, whitespace-only, or non-string values
- **AND** validation does not retrieve the referenced address or path

#### Scenario: Content includes an additional property

- **WHEN** otherwise valid metadata includes a property outside the schema's defined properties
- **THEN** it passes metadata validation without interpreting that property's value
- **AND** this result does not establish that the property or any dependency declaration is supported

#### Scenario: Schema document is checked

- **WHEN** the maintained schema is validated against the Draft 2020-12 meta-schema
- **THEN** it is a valid JSON Schema document for that draft

## ADDED Requirements

### Requirement: YAML decoding preserves type boundaries before schema validation

YAML and YML decoding MUST reject integer overflow for every numeric spelling
recognized by the selected parser, including underscores, uppercase radix
prefixes, binary, and signed radix forms, rather than fall back to strings.
Existing recognized in-range spellings MUST retain their numeric types. Checks
MUST apply to keys and values at any depth and retain alias/tag handling and work
limits. Ordinary quoted or explicitly string-tagged numeric-looking text MUST
remain strings.

Binary-tagged YAML keys and values MUST be rejected, including recognized
directive/URI spellings, rather than converted to UTF-8 strings. These checks MUST
occur at the parsing boundary before JSON Schema validation; the schema MUST NOT
attempt to reconstruct source types from converted strings.

#### Scenario: Extension integer spelling overflows

- **WHEN** a recognized YAML integer spelling with underscores, an uppercase or binary prefix, or a signed radix form exceeds the supported integer range
- **THEN** decoding fails in metadata, additional data, nested arrays/objects, and key positions instead of accepting a fallback string
- **AND** an in-range spelling remains numeric and a quoted or explicitly string-tagged control remains text

#### Scenario: Binary tag is supplied

- **WHEN** YAML content includes a binary-tagged value or key, directly, through a recognized tag directive, or as an anchored scalar
- **THEN** decoding rejects it before schema validation whether its decoded bytes would be valid UTF-8 or not
- **AND** an ordinary string or explicitly string-tagged value remains unaffected

### Requirement: Executable inputs are independent of OpenSpec artifacts

Hydragen source code, tests, and build scripts MUST NOT load or embed files from
`openspec/`. Assets required to build, test, or run Hydragen MUST live in maintained
project locations outside OpenSpec. The maintained layer manifest schema MUST be
included in the source package so a packaged build does not depend on an asset
outside that package.

Archived schemas MUST remain historical snapshots, not synchronized validation
sources or inputs to normal builds and tests. Documentation links to OpenSpec
specifications and decisions MUST remain permitted.

#### Scenario: Build and test without planning artifacts

- **WHEN** the source and required maintained assets are copied without `openspec/` or existing build outputs
- **AND** the Rust toolchain and package dependencies are available
- **THEN** Hydragen builds and its tests pass using a fresh build output directory
- **AND** no build or test attempts to read planning artifacts or archived schemas

#### Scenario: Build the packaged source

- **WHEN** the Hydragen source package is inspected and built independently of the original checkout
- **THEN** it includes `schemas/layer-manifest.schema.json` and builds successfully

#### Scenario: Archive snapshot differs from maintained schema

- **WHEN** a future intentional change updates the maintained schema without updating an archived schema
- **THEN** executable validation and tests use the maintained schema
- **AND** they do not require equality with or read the archived schema

### Requirement: Init content validation uses the maintained schema

The CLI and initialisation library MUST use the maintained schema to determine
metadata-content validity rather than independently reproduce its constraints in
Rust. This MUST apply to supplied flags, resolved guided answers, directory-name
defaults, and the complete candidate layer manifest before file creation.

Individual field validation MUST use the corresponding definitions from the same
maintained schema. Validating a field MUST NOT require fabricating valid values
for unrelated fields. An unknown field selector or missing schema definition
MUST produce an explicit failure rather than unconstrained acceptance.

Initialisation MUST validate the complete candidate content before filesystem
access in the library creation operation, including for direct library callers.
The content encoded in the selected format MUST be the content that passed complete validation. Input
interpretation and filesystem checks MUST remain separate: empty optional prompt
answers mean omission, directory-name selection resolves a default, and exclusive
creation protects the output path independently of content validity.

#### Scenario: Metadata is supplied through flags

- **WHEN** flag-only init receives metadata values and resolves any omitted name from the directory
- **THEN** it validates the resulting content using the maintained schema without opening prompts
- **AND** schema-invalid content produces a nonzero exit and no created files

#### Scenario: A guided answer is checked before completion

- **WHEN** a guided metadata prompt receives a candidate value
- **THEN** it applies the corresponding maintained schema definition to the resolved supplied value
- **AND** schema-invalid user input receives inline feedback and another opportunity to answer
- **AND** no layer manifest is created while guided input is invalid or incomplete

#### Scenario: Optional input is omitted or explicitly supplied empty

- **WHEN** an optional guided answer is empty
- **THEN** the field is omitted from the candidate layer manifest
- **AND** an explicitly empty metadata flag is instead treated as a supplied value and rejected by the schema
- **AND** a non-empty whitespace-only optional answer is rejected rather than omitted

#### Scenario: Name comes from the directory default

- **WHEN** init derives a usable UTF-8 name from the directory's final path component
- **THEN** it validates that name using the maintained name definition without normalization
- **AND** a schema-invalid default fails before prompts or writes with guidance to supply `--name`
- **AND** a valid explicit name bypasses default derivation but still receives schema validation

#### Scenario: Direct library creation receives invalid metadata

- **WHEN** a caller invokes the library initialisation operation with schema-invalid metadata
- **THEN** it returns the content validation failure before inspecting or creating filesystem entries
- **AND** no file is created

#### Scenario: Complete candidate needs validation beyond field feedback

- **WHEN** initialisation has collected its metadata and is ready to create the layer manifest
- **THEN** it validates the complete candidate against the maintained schema, even if individual fields were already validated
- **AND** it writes only after complete validation succeeds, preserving accepted strings, optional-field omission, the selected format's established output style, and a trailing newline
- **AND** existing target, repository-location, and exclusive-creation checks still apply

#### Scenario: A metadata constraint changes in the maintained schema

- **WHEN** a constraint on an existing metadata property changes in the maintained schema and the application is rebuilt
- **THEN** field checks and complete-content checks enforce that constraint from the updated schema
- **AND** no independently maintained Rust predicate is needed to enforce the same content rule

#### Scenario: Field validation target is unavailable

- **WHEN** field validation is requested for an unknown field or a required property definition cannot be resolved
- **THEN** it fails explicitly instead of treating the value as valid

### Requirement: Layer manifest content validation is reusable independently of file loading

The library MUST provide a reusable operation that validates parsed JSON content
against the maintained schema independently of init's typed metadata writer.
This operation MUST NOT require a terminal, a layer manifest file, or a supported
repository location, and MUST NOT create files. It MUST report content failures
with sufficient field or content-path and rule information for callers to explain
the problem. This requirement MUST NOT introduce existing-layer-manifest loading or a
new CLI command. The existing layer manifest loader MUST apply this validator to the
entire decoded document for every supported format before extracting identifying
metadata, and MUST retain additional data unchanged. A layer manifest's `$schema` value
MUST NOT select an alternative validator or trigger retrieval. Existing format
parsing policies MUST remain separate from content validation.

#### Scenario: Validate parsed content directly

- **WHEN** a library caller submits a parsed JSON value without a layer manifest file or terminal
- **THEN** the validator reports whether it satisfies the maintained schema without filesystem effects
- **AND** it rejects invalid shapes, including missing name, wrong metadata types, and non-object content

#### Scenario: Load content from any supported format

- **WHEN** the selected YAML, YML, TOML, JSON, or JSONC layer manifest decodes successfully
- **THEN** the loader validates the entire decoded document with the same maintained schema before extracting metadata
- **AND** it rejects invalid `$schema` values, including blank strings and representable non-string values
- **AND** valid additional nested data remains unchanged in the loaded document

#### Scenario: Selected layer manifest fails schema validation

- **WHEN** discovery finds a higher-priority layer manifest with schema-invalid content and a lower-priority valid layer manifest
- **THEN** loading fails on the selected higher-priority path without falling back
- **AND** the original discovery remains available for duplicate diagnostics

### Requirement: Schema validation is bundled and offline

The CLI MUST use the maintained schema bundled with its build, without reading
the source checkout, requiring a schema sidecar, invoking an external validator,
or retrieving schema resources over a network or from external files. Prepared
validators or their initialization failure MUST be cached within the process,
not reconstructed for each answer or validation call. Help and version output
MUST remain available without preparing validation.

#### Scenario: Run with only the executable

- **WHEN** init runs with only the executable available, without a source checkout, schema sidecar, network access, or external validator
- **AND** valid inputs and existing directory/file safety conditions are satisfied
- **THEN** it creates the same valid layer manifest using the schema bundled with that executable

#### Scenario: Bundled schema requires an unavailable resource

- **WHEN** preparing the bundled schema requires an external schema resource that is not bundled in memory
- **THEN** initialization fails without attempting network or external-file retrieval
- **AND** init exits nonzero before prompts or writes with an internal-schema diagnostic

#### Scenario: Several validations occur in one process

- **WHEN** a process validates multiple answers or candidate layer manifests
- **THEN** it reuses prepared validators rather than repeatedly parsing and compiling the schema
- **AND** an initialization failure is retained rather than repeatedly retried for user answers

### Requirement: Schema-backed validation preserves useful feedback and metadata semantics

Schema-backed validation MUST preserve current non-blank metadata semantics,
including rejection of ASCII- and Unicode-whitespace-only strings and preservation
of accepted values without normalization. Friendly field-specific errors and
inline retries MUST be derived from schema failures, not duplicated validity
checks or matching the validator's human-readable error text. Other constraint
failures MUST receive an accurate field/path and rule explanation instead of being
mislabelled as blank-string errors.

An invalid bundled schema or validator configuration MUST be distinguishable from
invalid user input. Such internal failures MUST cause a nonzero exit before
prompts or writes, without panic, retry prompts, success output, or misleading
guidance to change valid metadata. Existing interaction selection, output-stream
separation, cancellation, and file-safety requirements MUST remain unchanged.

#### Scenario: Whitespace-only metadata includes Unicode characters

- **WHEN** a supplied metadata value consists only of whitespace, including ASCII whitespace, NBSP, EM SPACE, or U+0085
- **THEN** schema-backed validation rejects it consistently across fields and complete-content checks
- **AND** a value containing a non-whitespace character remains accepted and preserved unchanged, including surrounding whitespace

#### Scenario: A known non-blank constraint fails

- **WHEN** the schema reports that a supplied metadata string violates its non-blank constraint
- **THEN** CLI feedback identifies the field and explains the need for a non-whitespace character
- **AND** guided input allows correction while flag-only execution fails without a replacement question

#### Scenario: A different schema constraint fails

- **WHEN** validation reports a rule violation other than the known non-blank constraint
- **THEN** the diagnostic reflects the failed rule and affected field or content path
- **AND** it does not incorrectly describe every failure as blank metadata

#### Scenario: Bundled schema cannot be prepared

- **WHEN** the bundled schema is invalid or validator configuration fails
- **THEN** init reports an internal validation setup failure and exits nonzero before prompts or writes
- **AND** it does not panic, report an unusable directory-name default, or ask the user to retry metadata
