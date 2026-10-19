# Layer manifest specification

## Purpose

Define a layer's identifying metadata, supported layer manifest filenames and formats,
read-only discovery and loading, duplicate warnings, dependency declaration
requirements, and shared validation through the maintained content schema.

## Requirements

### Requirement: Every layer has identifying metadata

Every layer MUST have a layer manifest declaring its name. The name MUST satisfy
the shared layer-name requirements. Author and description MUST be optional
independently of each other and, when supplied, MUST be strings containing at
least one non-whitespace character. Accepted values MUST be preserved without
normalisation. These rules MUST apply to both root and dependent layers, whether
intended primarily for reuse, as working projects, or both.

#### Scenario: Root layer metadata

- **WHEN** a root layer is defined
- **THEN** its layer manifest declares a valid layer name
- **AND** author and description can each be omitted

#### Scenario: Dependent layer metadata

- **WHEN** a dependent layer is defined
- **THEN** its layer manifest declares a valid layer name
- **AND** author and description can each be omitted

### Requirement: Layer manifest has a fixed filename and location

Every layer manifest MUST use one of exactly five supported filenames at the
root of the layer's repository: `layer.yaml`, `layer.yml`, `layer.toml`,
`layer.json`, or `layer.jsonc`. The basename MUST remain `layer`; the identifying
metadata name MUST NOT change the filename. YAML MUST be used for `.yaml` and
`.yml`, TOML for `.toml`, strict JSON for `.json`, and JSONC for `.jsonc`. These
rules apply to both root and dependent layers.

When multiple supported names are occupied, selection MUST use this order from
highest to lowest priority: `layer.yaml`, `layer.yml`, `layer.toml`, `layer.json`,
`layer.jsonc`. Selection MUST depend on the filename, not content validity, modification time,
or filesystem enumeration order. Layer manifests MUST NOT be merged. Documentation
MUST state the complete priority order explicitly.

#### Scenario: Supported layer manifest at the repository root

- **WHEN** a layer declares its layer manifest under any one supported filename at the repository root
- **THEN** that file satisfies the layer manifest filename and location requirement
- **AND** its extension determines the parser

#### Scenario: Multiple supported names are occupied

- **WHEN** two or more supported layer manifest paths are occupied
- **THEN** the highest-priority occupied name is selected
- **AND** lower-priority layer manifests are not merged with it or read for their contents

#### Scenario: Filename is independent of identifying name

- **WHEN** a layer's identifying metadata name is `shared-agent-rules`
- **THEN** its layer manifest still uses a supported `layer` filename
- **AND** `shared-agent-rules.yaml` is not an alternative recognised layer manifest name

#### Scenario: Different filename or nested location

- **WHEN** layer manifest content exists only under an unsupported filename or in a subdirectory
- **THEN** it does not satisfy the layer manifest requirement

#### Scenario: TOML priority between YAML and JSON

- **WHEN** `layer.toml`, `layer.json`, and `layer.jsonc` are present without either YAML filename
- **THEN** discovery selects `layer.toml` without reading the lower-priority layer manifests
- **AND** either YAML filename would take priority over `layer.toml` if present

### Requirement: Layer manifest declares layer dependencies

The selected layer manifest MUST declare dependencies in an optional
`dependencies` array of source-reference strings in every supported format. An
omitted field and an empty array MUST both mean no dependencies and therefore a
root layer. Other shapes, including maps, nested tuples, objects, and `null`, MUST
fail content validation. Local and published references MUST use the defined
source-reference syntax without separate name or version properties.

Array order MUST be retained. Defining ordered declarations MUST NOT itself
establish rendering precedence. The content schema MUST NOT attempt filesystem
lookup or substitute for cross-layer name-uniqueness checks.

#### Scenario: Layer depends on other layers

- **WHEN** a layer declares `dependencies: ["../shared-agent-rules", "shared-config@1.2.0"]`
- **THEN** content validation accepts the ordered array of syntactically valid references
- **AND** does not claim either target exists or has been retrieved

#### Scenario: Omitted dependency list

- **WHEN** a layer's selected layer manifest omits `dependencies`
- **THEN** the layer has no dependencies and is a root layer

#### Scenario: Empty dependency list

- **WHEN** a layer's selected layer manifest includes `dependencies: []`
- **THEN** the layer has no dependencies and is a root layer, with the same outcome as omitting the field

#### Scenario: Dependency entries do not duplicate names in keys

- **WHEN** a layer manifest supplies a name-keyed dependency map, source objects, nested name/version arrays, or non-string entries
- **THEN** content validation rejects the declaration with the affected content path

### Requirement: A schema describes metadata requirements

The layer manifest format MUST have a maintained JSON Schema at
`schemas/layer-manifest.schema.json`, using JSON Schema Draft 2020-12. The schema
MUST require an object with a valid layer name, permit optional author and
description with the existing non-blank string rules, and define the optional
ordered dependency array and source-reference syntax. Optional metadata MUST be
independently omittable, not represented by blank strings or `null`.

Shared definitions in that schema MUST determine name and reference content
validity, including exact published versions. Field checks, parsed-content checks,
every format loader, init input/defaults, and complete candidates before writing
MUST use those definitions rather than duplicate constraints in independent
predicates or separate schemas.

The same content rules MUST apply after parsing every supported format. The CLI
MUST NOT require an external runtime validator or add a schema property to
generated layer manifests. The schema MUST permit an optional `$schema` property
containing a non-blank string and MUST permit unknown properties without
interpreting them. A supplied `$schema` MUST NOT select another validator or cause
retrieval. Unknown-field acceptance MUST NOT make an invalid known `dependencies`
field acceptable.

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

- **WHEN** a layer manifest contains a valid name and supplies either or both of author and description as non-blank strings
- **THEN** those metadata values pass validation in every supported format
- **AND** their accepted content is retained unchanged

#### Scenario: Supplied metadata is invalid

- **WHEN** name or a supplied author or description is blank, whitespace-only, or not a string, including `null`
- **THEN** it fails validation against the schema in every format that can represent the supplied value
- **AND** the loader does not turn a non-string value into a string or treat explicit null as omission

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

### Requirement: Share read-only discovery and loading

The library MUST provide shared discovery and loading for all supported layer manifest
formats. Discovery MUST inspect only supported paths in the supplied directory,
without reading layer manifest contents, searching ancestors, or changing files. It
MUST return ordered candidates and selected-path information independently of
subsequent content loading, so duplicate information remains available on errors.
No new CLI command is required solely to expose this library capability.

Occupied supported paths MUST include regular files, directories, and symlinks,
including dangling symlinks. An unexpected path-inspection error MUST be reported,
not interpreted as absence. Loading MUST read only the selected candidate. If
none exists, loading MUST report a missing-layer-manifest error identifying the directory
and recognised filenames. Discovery with no candidates MUST be representable
without error for initialisation checks.

Loading a selected symlink to a regular file MUST be supported using the parser
of the layer manifest filename. A dangling symlink, directory, or non-regular selected
target MUST produce a path-specific load error rather than silently selecting
another candidate or intentionally waiting for input from a special file.

#### Scenario: Discover without reading contents

- **WHEN** discovery examines a supplied directory with supported layer manifest entries
- **THEN** it reports their ordered filenames and the highest-priority selection without reading any layer manifest contents
- **AND** it leaves that directory and all entries unchanged

#### Scenario: Load failure retains the discovery result

- **WHEN** discovery finds multiple candidates and loading the selected candidate fails
- **THEN** the caller still has the discovered paths and selection needed to report a duplicate warning

#### Scenario: Selected layer manifest is invalid or disappears

- **WHEN** the selected candidate has invalid syntax or metadata, is unreadable, or disappears before loading
- **AND** a valid lower-priority layer manifest is present
- **THEN** loading fails on the selected path without fallback or reading the lower-priority contents

#### Scenario: No layer manifest exists

- **WHEN** no supported layer manifest path is occupied in the supplied directory
- **THEN** discovery returns no selected candidate
- **AND** a request to load a layer manifest reports the missing layer manifest and supported filenames

#### Scenario: Occupied higher-priority path is not a readable layer manifest

- **WHEN** a directory or dangling symlink occupies `layer.yaml` and a valid `layer.json` exists
- **THEN** discovery selects `layer.yaml` and reports both paths
- **AND** loading fails on `layer.yaml` without fallback

#### Scenario: Selected layer manifest is a readable symlink

- **WHEN** `layer.jsonc` is selected and resolves through a symlink to a regular file
- **THEN** loading reads that target as JSONC without modifying the link or target

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
Read-only loading MUST NOT rewrite source files.

#### Scenario: Equivalent metadata across formats

- **WHEN** the same valid name, author, and description are expressed in each supported format
- **THEN** loading produces identical metadata without normalisation
- **AND** optional metadata retains accepted whitespace, Unicode, quotes, and newlines

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

- **WHEN** YAML metadata contains an unquoted boolean, number, null, or empty value
- **THEN** content validation rejects the non-string value without string conversion
- **AND** quoting a value permits it only when the resulting string satisfies that field's schema
- **AND** ordinary YAML 1.2 plain strings such as `yes` or `on` remain strings

#### Scenario: Repeated keys within a layer manifest

- **WHEN** a supported layer manifest repeats a key at any object/mapping depth, including an unknown field or an escaped spelling of the same JSON key
- **THEN** loading reports a duplicate-key error rather than taking the first or last value

#### Scenario: Ordinary TOML content

- **WHEN** TOML supplies ordinary top-level metadata, an ordered dependency string array, and supported unknown tables or arrays of tables
- **THEN** loading applies the same shared schema and retains additional decoded data
- **AND** it does not require a `[layer]` wrapper or perform dependency filesystem resolution

#### Scenario: TOML typed metadata is not coerced

- **WHEN** TOML supplies a boolean, number, array, table, or native date/time literal as name, author, or description
- **THEN** loading rejects the invalid metadata or unsupported data without converting it to a string
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

### Requirement: Warn about multiple layer manifest paths on applicable commands

Applicable commands MUST report duplicate layer manifest paths.
A command that touches or depends on a layer manifest, including init and add,
MUST use shared discovery and report duplicate supported paths. It MUST emit
at most one warning per affected directory per invocation on stderr, naming the
directory, all discovered filenames in priority order, and the selected filename.
Repeated references to one directory MUST NOT multiply its warning. The message
MUST explain selection, not merging. Warnings MUST follow the shared compact
status-styling policy; CI and redirected execution MUST remain plain, while
`--non-interactive` alone MUST NOT disable otherwise eligible styling.

Warnings MUST remain available before a subsequent load, validation, or
existing-layer-manifest refusal error. Initialisation warnings MUST NOT claim an
existing layer manifest was loaded. A duplicate warning alone MUST NOT cause a nonzero
exit status. Commands with no duplicates MUST NOT emit a duplicate warning.
Help, version, and argument-usage failures MUST NOT discover layer manifests or emit
layer manifest warnings.

#### Scenario: Duplicate layer manifests followed by a load failure

- **WHEN** an applicable command discovers `layer.yaml` and `layer.json` and loading `layer.yaml` then fails
- **THEN** stderr includes one duplicate warning naming both and selecting `layer.yaml`, before the load error
- **AND** the command does not fall back to JSON or print a successful result

#### Scenario: Existing duplicates prevent initialisation

- **WHEN** init discovers multiple supported layer manifest paths at its initial check
- **THEN** it warns once which takes priority and then reports that initialisation cannot proceed
- **AND** it reads no layer manifest content, opens no prompts, and changes no files

#### Scenario: Unrelated command output

- **WHEN** help, version, or argument-usage handling runs in a directory containing duplicate layer manifests
- **THEN** it neither inspects layer manifests nor emits layer manifest warnings

#### Scenario: One or no supported layer manifest

- **WHEN** discovery finds at most one supported layer manifest path
- **THEN** the CLI emits no duplicate warning

#### Scenario: Add references a directory repeatedly

- **WHEN** several local references resolve to one directory with duplicate layer manifests
- **THEN** its warning is emitted at most once before any dependent failure
- **AND** follows the same terminal/plain policy as other statuses

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
The content encoded in the selected format MUST be the content that passed
complete validation. Input interpretation and filesystem checks MUST remain
separate: empty optional prompt answers mean omission, directory-name selection
resolves a default, and exclusive creation protects the output path independently
of content validity.

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
the problem. The reusable validation operation MUST NOT itself load layer manifest files
or introduce a new CLI command. The existing layer manifest loader MUST apply this
validator to the entire decoded document for every supported format before extracting identifying
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

Schema-backed validation MUST enforce the shared name policy and preserve the
non-blank semantics for author and description, including rejection of ASCII-
and Unicode-whitespace-only strings. Accepted values MUST remain unchanged.
Field-specific errors and guided retries MUST derive from structured schema
failures, not duplicated predicates or matching human-readable validator text.
Non-blank, name, and reference failures MUST receive accurate field/content-path
and correction information rather than all being described as blank metadata.

An invalid bundled schema or validator configuration MUST be distinguishable from
invalid user input. Such internal failures MUST cause a nonzero exit before
prompts or writes, without panic, retry prompts, success output, or misleading
guidance to change valid metadata. Existing interaction selection, output-stream
separation, cancellation, and file-safety requirements MUST remain unchanged.
Adding this command MUST NOT enable guided add input.

#### Scenario: Whitespace-only metadata includes Unicode characters

- **WHEN** a supplied metadata value consists only of whitespace, including ASCII whitespace, NBSP, EM SPACE, or U+0085
- **THEN** schema-backed validation rejects it consistently across fields and complete-content checks
- **AND** a non-blank author or description remains accepted and preserved unchanged, including surrounding whitespace
- **AND** names are additionally subject to the stricter name grammar

#### Scenario: A known non-blank constraint fails

- **WHEN** the schema reports that a supplied metadata string violates its non-blank constraint
- **THEN** CLI feedback identifies the field and explains the need for a non-whitespace character
- **AND** guided init allows correction while supplied-input routes fail without a replacement question

#### Scenario: A different schema constraint fails

- **WHEN** a name, dependency reference, or other property violates a schema constraint other than the known non-blank constraint
- **THEN** the diagnostic reflects the failed rule and affected field or content path
- **AND** it does not incorrectly describe every failure as blank metadata

#### Scenario: Bundled schema cannot be prepared

- **WHEN** the bundled schema is invalid or validator configuration fails
- **THEN** init or add reports an internal validation setup failure and exits nonzero before prompts or writes
- **AND** it does not panic, report an unusable directory-name default, or ask the user to retry metadata

### Requirement: Layer names follow an explicit lowercase naming policy

A layer name MUST be a nonempty string using lowercase ASCII letters, digits,
`.`, `_`, and `-`, optionally structured as `@scope/name`. Both scoped components
MUST be nonempty. The complete name, including scope and separators, MUST have at
most 214 characters. An unscoped name and a scope component MUST NOT begin with
`.` or `_`; a scoped package component MUST be allowed to begin with either.
Other characters, whitespace, and uppercase MUST be rejected without trimming,
case conversion, or other normalisation. Node-specific reserved-name exclusions
MUST NOT be introduced. These rules MUST apply to every declared layer name and
published-reference name, independently of registry availability or ownership.

#### Scenario: Valid names and scopes

- **WHEN** names include `shared-config`, `@example/shared-config`, `@example/.config`, `@example/_config`, or `node_modules`
- **THEN** they pass name validation without implying publication availability

#### Scenario: Invalid names are not rewritten

- **WHEN** a name includes uppercase, surrounding spaces, Unicode, an empty scoped component, a forbidden leading character, or exceeds the full-name length limit
- **THEN** validation rejects it with corrective guidance
- **AND** does not lowercase, trim, shorten, or otherwise rewrite it

### Requirement: Dependency reference syntax distinguishes local paths from exact published versions

Reference syntax MUST be defined through the maintained schema. Explicit relative
path prefixes, standalone `.` and `..`, and explicit absolute path forms MUST
identify local references. Their strings MUST have no separate version. Bare
directory names MUST NOT be inferred as local, even if such a directory exists.
Path contents MUST NOT be subjected to layer-name restrictions, and references
MUST NOT be trimmed or rewritten. Native filesystem interpretation and unsupported
platform-path diagnostics MUST remain separate from content validity.

Published references MUST be `name@version` or `@scope/name@version`, using the
shared name rules and one complete strict SemVer 2.0.0 version. Prerelease and
build metadata MUST be accepted and retained. Partial versions, forbidden leading
zeroes, `v` prefixes, surrounding whitespace, ranges, tags, empty versions, and
implicit versions MUST fail validation. Git, alias, `file:`, and `link:` reference
protocols MUST NOT be introduced.

Reference content validation MUST be offline. It MUST NOT claim a source exists,
select a version, retrieve content, or generate a lockfile.

#### Scenario: Local paths remain local regardless of existence

- **WHEN** a reference is `./shared-config`, `../shared-agent-rules`, or an explicit absolute path
- **THEN** it is classified as a path without testing directory existence
- **AND** spaces or uppercase in directory names are preserved rather than subjected to the layer-name grammar

#### Scenario: Bare names are not local paths

- **WHEN** a reference is bare `shared-config`, even with a matching local directory
- **THEN** it fails the required explicit-path-or-exact-published-reference syntax
- **AND** does not acquire a different meaning based on the filesystem

#### Scenario: Exact published versions

- **WHEN** references are `shared-config@1.2.0`, `shared-config@1.2.0-beta.1`, or `@example/shared-config@1.2.0+build.7`
- **THEN** they pass content validation without registry access
- **AND** their complete identifiers remain unchanged

#### Scenario: Unsupported published selectors

- **WHEN** references include `shared-config@^1.2.0`, `shared-config@latest`, `shared-config@1.2`, `shared-config@v1.2.0`, `shared-config@01.2.0`, or `shared-config@`
- **THEN** content validation rejects them instead of resolving or normalising a selector

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
