## MODIFIED Requirements

### Requirement: Manifest has a fixed filename and location

Every layer's manifest MUST use one of exactly five supported filenames at the
root of the layer's repository: `layer.yaml`, `layer.yml`, `layer.toml`,
`layer.json`, or `layer.jsonc`. The basename MUST remain `layer`; the identifying
metadata name MUST NOT change the filename. YAML MUST be used for `.yaml` and
`.yml`, TOML for `.toml`, strict JSON for `.json`, and JSONC for `.jsonc`. These
rules apply to both root and dependent layers.

When multiple supported names are occupied, selection MUST use this order from
highest to lowest priority: `layer.yaml`, `layer.yml`, `layer.toml`, `layer.json`,
`layer.jsonc`. Selection MUST depend on the filename, not content validity, modification time,
or filesystem enumeration order. Manifests MUST NOT be merged. Documentation
MUST state the complete priority order explicitly.

#### Scenario: Supported manifest at the repository root

- **WHEN** a layer declares its manifest under any one supported filename at the repository root
- **THEN** that file satisfies the manifest filename and location requirement
- **AND** its extension determines the parser

#### Scenario: Multiple supported names are occupied

- **WHEN** two or more supported manifest paths are occupied
- **THEN** the highest-priority occupied name is selected
- **AND** lower-priority manifests are not merged with it or read for their contents

#### Scenario: Filename is independent of identifying name

- **WHEN** a layer's identifying metadata name is `shared-agent-rules`
- **THEN** its manifest still uses a supported `layer` filename
- **AND** `shared-agent-rules.yaml` is not an alternative recognised manifest name

#### Scenario: Different filename or nested location

- **WHEN** manifest content exists only under an unsupported filename or in a subdirectory
- **THEN** it does not satisfy the layer's manifest requirement

#### Scenario: TOML priority between YAML and JSON

- **WHEN** `layer.toml`, `layer.json`, and `layer.jsonc` are present without either YAML filename
- **THEN** discovery selects `layer.toml` without reading the lower-priority manifests
- **AND** either YAML filename would take priority over `layer.toml` if present

### Requirement: Manifest declares layer dependencies

The selected layer manifest MUST list the other layers that the layer depends
on, regardless of its supported format. Omitting the dependency list and
providing an empty list MUST both mean that the layer has no dependencies and
is therefore a root layer. The dependency field name and reference format remain
open; this change MUST NOT introduce or validate a dependency reference syntax.

#### Scenario: Layer depends on other layers

- **WHEN** a layer depends on other layers
- **THEN** those dependencies are listed in its selected manifest

#### Scenario: Omitted dependency list

- **WHEN** a layer's selected manifest omits the dependency list
- **THEN** the layer has no dependencies and is a root layer

#### Scenario: Empty dependency list

- **WHEN** a layer's selected manifest includes an empty dependency list
- **THEN** the layer has no dependencies and is a root layer, with the same outcome as omitting the list

### Requirement: A schema describes metadata requirements

The manifest data model MUST have a JSON Schema requiring name and permitting
optional author and description, with non-blank string validation for each
supplied field. The same metadata rules MUST apply after parsing any supported
format. The initial draft covers metadata only; it does not establish dependency
reference syntax, how versioning works or whether it belongs in the manifest,
or how layers combine. This change MUST NOT require an external runtime schema
validator or a schema property in generated manifests.

#### Scenario: Missing name

- **WHEN** a manifest omits name
- **THEN** it fails metadata validation in every supported format

#### Scenario: Optional metadata is omitted

- **WHEN** a manifest contains a valid name and omits author and description
- **THEN** it passes metadata validation in every supported format

#### Scenario: Optional metadata is supplied independently

- **WHEN** a manifest contains a valid name and supplies either or both of author and description as non-blank strings
- **THEN** it passes metadata validation in every supported format

#### Scenario: Supplied metadata is invalid

- **WHEN** name or a supplied author or description is blank, whitespace-only, or not a string, including `null`
- **THEN** it fails metadata validation in every supported format
- **AND** the loader does not turn a non-string value into a string or treat an explicit null as omission

## ADDED Requirements

### Requirement: Share read-only discovery and loading

The library MUST provide shared discovery and loading for all supported manifest
formats. Discovery MUST inspect only supported paths in the supplied directory,
without reading manifest contents, searching ancestors, or changing files. It
MUST return ordered candidates and selected-path information independently of
subsequent content loading, so duplicate information remains available on errors.
No new CLI command is required solely to expose this library capability.

Occupied supported paths MUST include regular files, directories, and symlinks,
including dangling symlinks. An unexpected path-inspection error MUST be reported,
not interpreted as absence. Loading MUST read only the selected candidate. If
none exists, loading MUST report a missing-manifest error identifying the directory
and recognised filenames. Discovery with no candidates MUST be representable
without error for initialisation checks.

Loading a selected symlink to a regular file MUST be supported using the parser
of the manifest filename. A dangling symlink, directory, or non-regular selected
target MUST produce a path-specific load error rather than silently selecting
another candidate or intentionally waiting for input from a special file.

#### Scenario: Discover without reading contents

- **WHEN** discovery examines a supplied directory with supported manifest entries
- **THEN** it reports their ordered filenames and the highest-priority selection without reading any manifest contents
- **AND** it leaves that directory and all entries unchanged

#### Scenario: Load failure retains the discovery result

- **WHEN** discovery finds multiple candidates and loading the selected candidate fails
- **THEN** the caller still has the discovered paths and selection needed to report a duplicate warning

#### Scenario: Selected manifest is invalid or disappears

- **WHEN** the selected candidate has invalid syntax or metadata, is unreadable, or disappears before loading
- **AND** a valid lower-priority manifest is present
- **THEN** loading fails on the selected path without fallback or reading the lower-priority contents

#### Scenario: No manifest exists

- **WHEN** no supported manifest path is occupied in the supplied directory
- **THEN** discovery returns no selected candidate
- **AND** a request to load a manifest reports the missing manifest and supported filenames

#### Scenario: Occupied higher-priority path is not a readable manifest

- **WHEN** a directory or dangling symlink occupies `layer.yaml` and a valid `layer.json` exists
- **THEN** discovery selects `layer.yaml` and reports both paths
- **AND** loading fails on `layer.yaml` without fallback

#### Scenario: Selected manifest is a readable symlink

- **WHEN** `layer.jsonc` is selected and resolves through a symlink to a regular file
- **THEN** loading reads that target as JSONC without modifying the link or target

### Requirement: Parse familiar formats into the same manifest data

Each parser MUST produce one object/mapping document for shared metadata
validation. Accepted string values MUST retain their content without trimming or
normalisation. Additional fields MUST be accepted and retained as decoded data,
without implying dependency validation or implementing their behaviour.
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
their decoded values satisfy the shared manifest data model. Duplicate keys and
invalid table redefinitions MUST be rejected according to TOML rules; valid
arrays of tables MUST NOT be treated as duplicate declarations.

TOML values MUST retain their native types until conversion to the shared data
model. The loader MUST NOT stringify numbers, booleans, or date/time literals,
or expose a Serde datetime wrapper as a user-supplied object. Native date/time
literals and non-finite numbers MUST fail with a path-specific unsupported-data
diagnostic; date/time-like text MUST be quoted to be a manifest string. Omitted
optional keys MUST remain absent. The loader MUST NOT invent a TOML null syntax.

#### Scenario: Equivalent metadata across formats

- **WHEN** the same name, author, and description are expressed as valid strings in each supported format
- **THEN** loading produces identical metadata, preserving whitespace, Unicode, quotes, and newlines

#### Scenario: JSONC comments and trailing commas

- **WHEN** a JSONC manifest contains line comments, block comments, or trailing commas
- **THEN** those constructs are accepted without changing its decoded data
- **AND** the same constructs in a JSON manifest fail parsing

#### Scenario: JSONC is not JSON5

- **WHEN** a JSONC manifest contains an unquoted key, single-quoted string, or another disabled permissive extension
- **THEN** loading fails with a diagnostic identifying the selected manifest

#### Scenario: Ordinary YAML strings and aliases

- **WHEN** a YAML or YML manifest uses comments, a block description string, or an ordinary alias to a string anchor
- **THEN** loading accepts them with their normal YAML meaning and the same metadata validation
- **AND** an explicitly tagged `!!str` value is still treated as a string

#### Scenario: YAML scalar types are not coerced

- **WHEN** YAML metadata contains an unquoted boolean, number, null, or empty value
- **THEN** metadata validation rejects the non-string value
- **AND** quoting a non-blank scalar as a string permits it
- **AND** ordinary YAML 1.2 plain strings such as `yes` or `on` remain strings

#### Scenario: Repeated keys within a manifest

- **WHEN** a supported manifest repeats a key at any object/mapping depth, including an unknown field or an escaped spelling of the same JSON key
- **THEN** loading reports a duplicate-key error rather than taking the first or last value

#### Scenario: Ordinary TOML content

- **WHEN** a TOML manifest declares top-level string name, author, or description using ordinary comment and string syntax, with supported additional tables or arrays of tables
- **THEN** loading validates the same metadata and retains the additional decoded data
- **AND** it does not require a `[layer]` wrapper or invent dependency semantics

#### Scenario: TOML typed metadata is not coerced

- **WHEN** TOML supplies a boolean, number, array, table, or native date/time literal as name, author, or description
- **THEN** loading rejects the invalid metadata or unsupported data without converting it to a string
- **AND** a quoted non-blank string with the same textual content can pass metadata validation

#### Scenario: TOML native values outside the common data model

- **WHEN** a TOML manifest contains a native date/time literal or non-finite float, including in an additional field
- **THEN** loading reports unsupported data at the selected path rather than converting it to a string, null, or internal wrapper object

#### Scenario: TOML duplicate keys or invalid table redefinitions

- **WHEN** TOML defines the same decoded key twice or uses conflicting dotted keys or table declarations
- **THEN** loading reports the parser's duplicate or redefinition error rather than discarding a value

#### Scenario: Additional fields remain uninterpreted

- **WHEN** a valid manifest includes an additional field with supported decoded data
- **THEN** loading preserves that data without interpreting it or claiming its dependency declarations are valid
- **AND** no source file is rewritten

#### Scenario: Unsupported YAML composition or document stream

- **WHEN** YAML input contains multiple documents, a merge key, an unsupported custom tag, or an include request
- **THEN** loading rejects it rather than adding composition, inclusion, or fallback behaviour

### Requirement: Warn about multiple manifest paths on applicable commands

A CLI command that touches or depends on a layer manifest MUST use shared
discovery and report discovered duplicate supported paths. It MUST emit at most
one duplicate warning per affected directory per invocation, on standard error,
identifying the directory, all discovered filenames in priority order, and which
takes priority. The warning MUST communicate that one manifest is selected, not
that their contents are merged. Non-interactive warnings MUST be plain text.

Warnings MUST remain available before a subsequent load, validation, or
existing-manifest refusal error. Initialisation warnings MUST NOT claim an
existing manifest was loaded. A duplicate warning alone MUST NOT cause a nonzero
exit status. Commands with no duplicates MUST NOT emit a duplicate warning.
Help, version, and argument-usage failures MUST NOT discover manifests or emit
manifest warnings. This increment MUST integrate the convention into init;
future commands using manifests MUST adopt it when introduced.

#### Scenario: Duplicate manifests followed by a load failure

- **WHEN** an applicable command discovers `layer.yaml` and `layer.json` and loading `layer.yaml` then fails
- **THEN** stderr includes one duplicate warning naming both and selecting `layer.yaml`, before the load error
- **AND** the command does not fall back to JSON or print a successful result

#### Scenario: Existing duplicates prevent initialisation

- **WHEN** init discovers multiple supported manifest paths at its initial check
- **THEN** it warns once which takes priority and then reports that initialisation cannot proceed
- **AND** it reads no manifest content, opens no prompts, and changes no files

#### Scenario: Unrelated command output

- **WHEN** help, version, or argument-usage handling runs in a directory containing duplicate manifests
- **THEN** it neither inspects manifests nor emits manifest warnings

#### Scenario: One or no supported manifest

- **WHEN** discovery finds at most one supported manifest path
- **THEN** the CLI emits no duplicate warning
