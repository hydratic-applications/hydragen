## MODIFIED Requirements

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
- **THEN** it passes content validation when its other known fields are valid

#### Scenario: Optional metadata is supplied independently

- **WHEN** a layer manifest contains a valid name and supplies either or both of author and description as non-blank strings
- **THEN** those metadata values pass validation in every supported format
- **AND** their accepted content is retained unchanged

#### Scenario: Supplied metadata is invalid

- **WHEN** name or a supplied author or description is blank, whitespace-only, or not a string, including `null`
- **THEN** it fails validation in every format that can represent the value
- **AND** the loader does not stringify the value or treat explicit null as omission

#### Scenario: Name violates the new naming policy

- **WHEN** a layer manifest uses an uppercase, overlength, or otherwise invalid layer name
- **THEN** field and complete-content validation reject it through the maintained schema
- **AND** no path silently normalises it into a valid name

#### Scenario: Content is not an object

- **WHEN** layer manifest content is a JSON string, number, boolean, array, or `null`
- **THEN** it fails validation against the schema

#### Scenario: Editor schema reference is supplied

- **WHEN** otherwise valid content includes `$schema`
- **THEN** validation accepts only a non-blank string for that property
- **AND** does not retrieve or use it to replace the bundled schema

#### Scenario: Content includes an additional property

- **WHEN** otherwise valid content includes a property outside the schema's defined properties
- **THEN** content validation accepts it without interpreting its value
- **AND** does not claim that the unknown property's behaviour is supported

#### Scenario: Schema document is checked

- **WHEN** the maintained schema is validated against the Draft 2020-12 meta-schema
- **THEN** it is valid for that draft

### Requirement: Parse familiar formats into the same layer manifest data

Each parser MUST produce one object/mapping document for shared content validation.
Accepted strings MUST retain their content without trimming or normalisation.
Unknown fields MUST be accepted and retained as decoded data without implementing
their behaviour. Parsing MUST remain separate from reference classification,
schema validation, and dependency relationship checks. Repeated object/mapping
keys MUST be rejected before a map discards duplicates, including unknown and
nested keys. JSON key comparison MUST use decoded names. Supported data MUST use
string object keys and JSON-compatible values; unsupported complex YAML keys,
TOML native date/time literals, and non-finite numbers MUST NOT be silently
converted to other data.

Strict JSON MUST reject comments, trailing commas, and JSON5 extensions. JSONC
MUST accept line/block comments and trailing commas, but reject unquoted keys,
single-quoted strings, missing commas, and other JSON5-only extensions. Empty or
comment-only input, multiple top-level values, non-object input, invalid syntax,
and invalid UTF-8 MUST fail loading with a path-specific error.

YAML and YML MUST use the same maintained parser and ordinary YAML 1.2 behaviour,
including comments, normal and block strings, standard string tags, and bounded
anchors/aliases. Input MUST contain one mapping document. The loader MUST NOT
implement custom tags, file inclusion, environment substitution, or YAML 1.1
merge-key composition. Parser safety limits MUST NOT be disabled.

TOML MUST use a maintained standard parser and a top-level table without a required
`[layer]` wrapper. Ordinary comments, basic/literal and multiline strings, dotted
keys, tables, inline tables, and arrays of tables MUST be accepted when their
decoded values satisfy the shared data model and content schema. Duplicate keys
and invalid table redefinitions MUST be rejected according to TOML rules; valid
arrays of tables MUST NOT be treated as duplicate declarations.

TOML values MUST retain native types until conversion. The loader MUST NOT
stringify numbers, booleans, or date/time literals, or expose a Serde datetime
wrapper as a user object. Native date/time literals and non-finite numbers MUST
fail with a path-specific unsupported-data diagnostic. Date/time-like string text
MUST be quoted. Omitted optional keys MUST remain absent, and the loader MUST NOT
invent a TOML null syntax. Read-only loading MUST NOT rewrite source files.

#### Scenario: Equivalent metadata across formats

- **WHEN** the same valid name, author, and description are expressed in every supported format
- **THEN** loading produces identical metadata without normalisation
- **AND** optional metadata retains accepted whitespace, Unicode, quotes, and newlines

#### Scenario: JSONC comments and trailing commas

- **WHEN** JSONC contains line/block comments or trailing commas
- **THEN** those constructs are accepted without changing decoded data
- **AND** the same constructs fail strict JSON parsing

#### Scenario: JSONC is not JSON5

- **WHEN** JSONC contains unquoted keys, single-quoted strings, or another disabled extension
- **THEN** loading fails with a diagnostic identifying the selected layer manifest

#### Scenario: Ordinary YAML strings and aliases

- **WHEN** YAML/YML uses comments, a block description, an ordinary string alias, or `!!str`
- **THEN** loading retains normal YAML meaning and applies the shared content schema
- **AND** standard string tags remain strings

#### Scenario: YAML scalar types are not coerced

- **WHEN** a string-valued metadata field is supplied as a YAML boolean, number, null, or empty value
- **THEN** content validation rejects it without string conversion
- **AND** quoting a value permits it only when the resulting string satisfies that field's schema
- **AND** ordinary YAML 1.2 strings such as `yes` and `on` remain strings

#### Scenario: Repeated keys within a layer manifest

- **WHEN** a supported format repeats a decoded key at any depth, including an unknown field or escaped spelling of a JSON key
- **THEN** loading reports a duplicate-key error rather than retaining the first or last value

#### Scenario: Ordinary TOML content

- **WHEN** TOML supplies ordinary top-level metadata, an ordered dependency string array, and supported unknown tables or arrays of tables
- **THEN** loading applies the same shared schema and retains additional decoded data
- **AND** does not require a `[layer]` wrapper or perform dependency filesystem resolution

#### Scenario: TOML typed metadata is not coerced

- **WHEN** TOML supplies a boolean, number, array, table, or native date/time literal for name, author, or description
- **THEN** loading rejects invalid metadata or unsupported data without converting it to a string
- **AND** quoted text is accepted only when it satisfies the field's schema

#### Scenario: TOML native values outside the common data model

- **WHEN** TOML includes native date/time literals or non-finite floats, including in additional nested data
- **THEN** loading reports unsupported data without stringifying it or exposing internal wrappers

#### Scenario: TOML duplicate keys or invalid table redefinitions

- **WHEN** TOML repeats a key or uses conflicting dotted keys or table declarations
- **THEN** loading reports the parser's duplicate or redefinition error

#### Scenario: Additional fields remain uninterpreted

- **WHEN** a valid layer manifest includes supported decoded data in an unknown field
- **THEN** loading retains it without interpreting that field or claiming dependency relationship validity
- **AND** no source file is rewritten

#### Scenario: Unsupported YAML composition or document stream

- **WHEN** YAML includes multiple documents, a merge key, an unsupported custom tag, or an include request
- **THEN** loading rejects it instead of adding composition, inclusion, or fallback behaviour

### Requirement: Schema-backed validation preserves useful feedback and metadata semantics

Schema-backed validation MUST enforce the new name policy and preserve the
non-blank semantics for author and description, including rejection of ASCII-
and Unicode-whitespace-only strings. Accepted values MUST remain unchanged.
Field-specific errors and guided retries MUST derive from structured schema
failures, not duplicated predicates or matching human-readable validator text.
Non-blank, name, and reference failures MUST receive accurate field/content-path
and correction information rather than all being described as blank metadata.

An invalid bundled schema or validator configuration MUST be distinguishable from
invalid user input. It MUST stop before prompts or writes, without panic, retry
prompts, success output, or misleading advice to change valid content. Existing
init interaction selection, output streams, cancellation, and file safety MUST
remain unchanged. Adding this command MUST NOT enable guided add input.

#### Scenario: Whitespace-only metadata includes Unicode characters

- **WHEN** a supplied metadata value contains only whitespace, including ASCII whitespace, NBSP, EM SPACE, or U+0085
- **THEN** schema-backed field and complete-content validation reject it
- **AND** a non-blank author or description remains preserved, including surrounding whitespace
- **AND** names are additionally subject to the stricter name grammar

#### Scenario: A known non-blank constraint fails

- **WHEN** the schema reports a known non-blank failure
- **THEN** feedback identifies the field and need for a non-whitespace character
- **AND** guided init permits correction while supplied-input routes fail without prompting

#### Scenario: A different schema constraint fails

- **WHEN** a name, dependency reference, or other property violates another schema constraint
- **THEN** diagnostics identify its actual field/content path and rule
- **AND** do not mislabel the failure as blank-string input

#### Scenario: Bundled schema cannot be prepared

- **WHEN** the bundled schema is invalid or validator setup fails
- **THEN** init or add reports an internal validation setup failure before prompts or writes
- **AND** does not blame valid content, retry prompts, or report success

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
existing-layer-manifest refusal failure. Init warnings MUST NOT imply existing
contents were loaded. Duplicate warning alone MUST NOT cause a nonzero exit;
commands without duplicates MUST NOT warn. Help, version, and argument-usage
failures MUST NOT discover layer manifests or emit these warnings.

#### Scenario: Duplicate layer manifests followed by a load failure

- **WHEN** discovery finds `layer.yaml` and `layer.json` and loading the selected YAML fails
- **THEN** stderr warns once naming both and selecting YAML before the load error
- **AND** does not fall back to JSON or print success

#### Scenario: Existing duplicates prevent initialisation

- **WHEN** init initially discovers multiple supported layer manifest paths
- **THEN** it warns once and refuses initialisation without reading contents, prompting, or changing files

#### Scenario: Unrelated command output

- **WHEN** help, version, or argument-usage handling runs in a directory with duplicates
- **THEN** it does not inspect layer manifests or emit layer manifest warnings

#### Scenario: One or no supported layer manifest

- **WHEN** discovery finds at most one supported layer manifest path
- **THEN** no duplicate warning is emitted

#### Scenario: Add references a directory repeatedly

- **WHEN** several local references resolve to one directory with duplicate layer manifests
- **THEN** its warning is emitted at most once before any dependent failure
- **AND** follows the same terminal/plain policy as other statuses

## ADDED Requirements

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
