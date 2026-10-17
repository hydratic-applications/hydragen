## Context

After merging `support-layer-manifest-formats`, the CLI implements `hydragen init`
with YAML/YML/TOML/JSON/JSONC output and YAML as its default. The library also has
read-only manifest discovery/loading. `Metadata` in `src/lib.rs`
represents its output, while `validate_metadata_value` independently enforces the
manifest's non-blank-string rule using Rust's whitespace predicate. Flags,
guided answers, and directory-name defaults call that predicate. Before writing,
the library validates metadata and uses the selected format's serializer.

`src/manifest.rs` already owns discovery, loading, and codecs. Its decoders produce
complete JSON-compatible `serde_json::Value` documents under explicit parser
policies. Loading currently validates only extracted identifying fields, so it
does not enforce the existing optional `$schema` rule. Schema integration must
validate the entire decoded document before extraction, preserving additional
fields in `LoadedManifest.document`.

A unit test in the same module embeds a schema from an archived OpenSpec change.
It checks a few schema properties but does not validate generated content. This
makes test compilation depend on planning history without making that schema
govern runtime behaviour. The README also presents the archived schema as the
current asset.

The [proposal](proposal.md) establishes the replacement: one maintained schema
outside OpenSpec, used by the CLI and available for future editor tooling.
Developers should be able to change a content constraint in that schema without
also implementing an equivalent predicate in Rust.

This supports the foundations' DRY, developer-experience, and performance values.
The existing init contract remains important: helpful errors, separate guided
and flag-only input, no files before validation completes, and exclusive creation
even after read-only target checks. The schema cannot replace repository-location
or filesystem-safety checks.

## Goals / Non-Goals

**Goals:**

- Maintain the current metadata schema at
  `schemas/layer-manifest.schema.json`, independently of OpenSpec history.
- Make schema validation determine content validity for every init input route
  and the complete manifest before writing.
- Provide reusable validation of JSON-compatible parsed content from all supported
  formats, including the existing loader, without coupling it to a
  command, terminal, or filesystem path.
- Embed the schema and cache compilation so ordinary use is offline and does not
  repeatedly prepare validators.
- Preserve current validity outcomes, output, friendly feedback, and file safety.
- Exercise the schema and its actual CLI use in tests, including standalone builds
  and runtime operation without OpenSpec artifacts.
- Document the ownership boundary in the README and `AGENTS.md`.

**Non-Goals:**

- Adding a new loading workflow, repairing existing files, or adding a CLI command.
- Further format support or parser-policy redesign, dependency-reference syntax,
  versioning, stricter unknown
  fields, or broader manifest semantics.
- Schema generation, generated Rust types, or generated CLI arguments/prompts.
- Publishing a public schema URL, automatic editor association, a schema-export
  command, or adding `$schema` to init output.
- Runtime schema overrides or remote/file reference retrieval.
- Rewriting archives, changing package publication policy, or adding CI machinery.

## Decisions

### 1. Maintain the schema as an implementation asset, not an archive dependency

Create `schemas/layer-manifest.schema.json` from the archived schema without
changing its rules, updating descriptive text to cover all supported manifest
formats rather than only `layer.json`: Draft 2020-12, an object with required non-blank string `name`,
independently optional non-blank string `author` and `description`, the existing
optional `$schema` property, and permitted additional properties.

The new file is the sole maintained schema asset. Keep the archive unchanged as
a historical snapshot; do not synchronize it or compare it with the maintained
file in tests. The maintained schema still describes metadata only. Acceptance
of additional properties does not establish dependency syntax or validate those
properties' meanings.

The README will link to the maintained file and explain its scope. Editor tooling
can use that same file through appropriate local configuration, but this change
does not promise a distribution address or automatic integration.

**Alternatives:** Moving the dependency to the current OpenSpec specifications
would still couple executable inputs to planning documents. A test fixture would
not establish a runtime source of truth. Generating a schema from the current
writer types would add machinery and would need separate treatment of the
non-blank rule and properties the writer does not emit.

#### Unicode compatibility correction authorized on 2026-10-17

Runtime tests with `jsonschema` 0.58.6 confirmed that both supported regex engines
translate `\S` using ECMA-262 whitespace: they accept U+0085 alone and reject
U+FEFF alone, the reverse of existing Rust metadata validation. Engine selection
does not change that translation. The user authorized an explicit Unicode
whitespace character class in the maintained schema to preserve existing CLI
semantics, rather than changing accepted metadata or adding a parallel predicate.

The class represents Unicode White_Space: U+0009–000D, U+0020, U+0085, U+00A0,
U+1680, U+2000–200A, U+2028–2029, U+202F, U+205F, and U+3000. A valid string
contains a character outside that set; U+FEFF is not in the set. Keep that definition
in the schema only and leave the archive untouched. Shared schema definitions
can name the non-blank rule for truthful diagnostics without copying its regex
into Rust. Test both engines and field/whole-object paths.

#### Parser compatibility corrections authorized on 2026-10-17

Runtime characterization of the merged YAML decoder reproduced two behaviours
outside what schema validation can repair after parsing. Extension integer
spellings (underscores, uppercase prefixes, binary, and signed radix forms) can
fall back to strings on overflow, including as keys. Standard `!!binary` tags can
decode UTF-8 bytes into string values or keys. The user explicitly authorized
these bounded parser corrections as part of merge assessment:

- Preserve currently recognized in-range integer spellings, but reject overflow
  consistently for every recognized spelling and key/value position. Quoted or
  explicitly string-tagged content must remain text.
- Reject binary-tagged values and keys, including recognized tag-URI/directive
  spellings, instead of silently converting bytes to text.

Implement these checks in the parser boundary, not JSON Schema. Retain parser
budgets, duplicate detection, explicit numeric tags, aliases, and writer string
round-trips. These are explicit exceptions to the otherwise preserved parser
policy; document the tightened input acceptance and regression evidence.

### 2. Embed an offline validator and compile it once per process

Add a focused `src/manifest/schema.rs` submodule alongside the existing decoders.
It embeds the maintained file using
`include_str!`, parses it, and prepares the validators on first validation use.
Use standard-library lazy initialization to cache either the prepared validator
set or an initialization failure. Do not parse or compile per field, answer, or
validation call. Help and version output do not require validator initialization.

Use the Rust `jsonschema` crate as a normal runtime dependency with default
features disabled. Start with version requirement `0.58.6`, commit resolution in
`Cargo.lock`, and use explicit offline configuration with Draft 2020-12 support.
Do not enable HTTP, external-file retrieval, TLS, async resolution, or macros.
The standard draft declaration must not trigger a network request. References
required by the schema must resolve from the bundled in-memory schema resources;
unexpected external references fail initialization rather than being fetched.

Planning evidence checked on 2026-10-16: crates.io reports 0.58.6 as non-yanked,
with Rust 1.85.0 as its minimum; the checkout uses Rust 1.96.0. Its documentation
supports offline construction and draft-specific meta-validation. Verify the
selected dependency and feature graph during implementation.

Sources: [crate release](https://crates.io/crates/jsonschema/0.58.6) and
[validator documentation](https://docs.rs/jsonschema/0.58.6/jsonschema/).

**Alternatives:** Loading a sidecar schema would complicate standalone installs
and permit a binary's validation rules to change with local files. Fetching a
schema would add availability, security, and startup costs. An external validator
would violate the existing runtime-service boundary. A handwritten subset of JSON
Schema would recreate a second validation implementation.

### 3. Share the schema between complete-content and field validation

The manifest module exposes a reusable operation accepting
`&serde_json::Value`, returning success or an owned validation error. It does not
read files or depend on init's typed writer. The existing loader uses it immediately
after decoding and before metadata extraction, including shapes that `Metadata`
cannot construct. The manifest's `$schema` value is validated as content, never
used to fetch or select another validator. Keep the original validated document,
including additional properties. Parser-specific syntax, duplicate-key, type,
number-range, and unsupported-value policies remain in the decoders.

Init also needs field validation while a complete object does not yet exist.
Prepare field validators that reference the maintained document's property
definitions through an in-memory schema registry. Use an internal resource
identifier for this registration; it is not a published schema URL and is not
written into manifests. Reference wrappers contain no independently maintained
type, regex, or length constraints.

Keep the complete-object validator and known-field validators in one cached set.
Known-field selection is application wiring, not a second source of content rules.
Missing property targets and unknown field selectors must fail explicitly rather
than validating unconstrained content. Complete-object validation remains the
final authority for required properties and any future cross-field constraints.

**Alternatives:** Independently written per-field schemas would duplicate rules.
Validating incomplete objects would incorrectly report missing fields. Filling
those objects with placeholder metadata would depend on assumptions about what
values remain valid as the schema evolves.

### 4. Integrate at existing validation boundaries and preserve the write contract

Replace the implementation of `validate_metadata_value` with schema-backed field
validation, retaining the shared entry point where practical. Make
`Metadata::validate` validate the complete serialized metadata object. The final
serialization path encodes the same candidate content that passed validation,
preserving field omission, accepted strings, and each format's established output
style (including two-space indentation for JSON and a trailing newline).
Route `validate_document` through whole-document schema validation before extracting
`Metadata`; preserve its existing error categories where applicable with structured
fallback/schema-setup errors for new cases. Avoid recursion between encoders and
`Metadata::validate`.

Flags validate through the shared object validator. Guided answers and derived
directory names use the field validator, followed by complete validation before
writing. Prepare schema validation before opening prompts so an internal failure
cannot become an endless retry. Retain known-target checks before guided input.

Empty optional answers still mean omission; an explicitly empty flag remains a
supplied invalid value. Directory-name selection and UTF-8 conversion remain
application concerns. Change the existing directory-name failure handling so
invalid content produces override guidance, while schema initialization failures
remain internal failures instead of being hidden by `.is_ok()`.

`initialize` must continue validating before filesystem access for direct library
callers. Exclusive creation, existing-entry detection, repository guards, and
partial-write diagnostics remain intact. A schema validator never establishes
that creating a path is safe.

**Alternatives:** Validating only prompt answers or only the command entry point
would leave other callers able to bypass the contract. Validating after opening
the output would permit invalid content to create a file. Moving filesystem
checks into schema validation would confuse content validity with target safety.

### 5. Translate schema failures without reimplementing their predicates

Represent invalid content separately from schema initialization/configuration
failure. Retain enough structured information for a content path, failed keyword,
and fallback explanation. Extend `InitError` narrowly to preserve these distinctions
and useful error sources; do not build a general validation framework.

The CLI retains field-specific wording and inline retries by translating the
schema's reported failure, not checking the input again. Preserve current
non-blank wording for the corresponding violations. Other schema failures need
an accurate path/rule diagnostic, rather than being labelled as blank-string
errors. Do not match the validator library's human-readable error strings.

To recognize that reviewed rule without copying its regex into Rust, the named
schema definition carries an optional
`x-hydragen-nonblank-pattern-sha256` annotation. Compare it with a SHA-256 digest
of the decoded pattern once during setup, using the `sha2` crate. The digest is
only a diagnostic approval token: missing or mismatched annotations fall back to
the actual pattern diagnostic and never change validity or fail initialization.
Update the token only after reviewing that a changed rule still means non-blank,
not automatically with arbitrary pattern changes. Test replacement of the named
pattern itself so a different rule cannot inherit misleading non-blank wording.

An invalid bundled schema is an internal error: report it clearly with a nonzero
exit before prompts or writes. Do not panic, blame user metadata, or ask the user
to retry the same value. Terminal formatting stays outside manifest validation.

**Alternatives:** Raw validator output would unnecessarily change established
prompt feedback. Keeping handwritten checks solely for friendly messages could
let those checks disagree with the schema. Treating all failures as invalid
metadata would hide defects in the bundled implementation.

### 6. Verify behaviour and independence, not the schema's internal layout

Replace the current structural schema test with behavioural coverage. Validate
the maintained schema itself against Draft 2020-12, exercise accepted/rejected
content, and validate actual init output using the production validator. Expected
results remain independently authored; tests must not derive expectations from
the schema or scrape OpenSpec examples.

Cover all five loader formats and output codecs, required name, independent optional fields, all metadata, non-object inputs,
missing fields, wrong types including `null`, blank strings, whitespace, Unicode,
escaping, current `$schema` handling, and permitted unknown properties. Verify
field and whole-object validation agree on field constraints. Retain independent
assertions for exact output, omission, and filesystem effects. Add cross-format
invalid `$schema` cases (representable wrong types in TOML), complete additional-data
preservation, and selected-only schema failure with no fallback to another format.
Characterize numeric-extension overflow and binary-tag handling in YAML separately;
do not silently change those policies during schema integration.

Extend the existing captured-output and bounded pseudo-terminal tests rather
than introducing another terminal harness. Keep retries, defaults, skips,
cancellation, separate output streams, and no-file-on-invalid-input coverage.

Verify builds and tests in a disposable source copy without `openspec/` or old
build outputs, using a fresh target directory. Check package inclusion of the
maintained schema and that the packaged crate builds. Separately run only the
binary without a source checkout or schema sidecar. Dependency downloads during
Cargo setup are distinct from runtime schema retrieval; repeat relevant tests
offline once dependencies are available.

**Alternatives:** Retargeting the old structural test would retain weak coverage.
Archive equality tests would make history an executable input again. Removing
OpenSpec from the real checkout for verification would unnecessarily disturb the
working tree. A blanket source-string ban would also reject legitimate explanatory
references without proving standalone operation.

### 7. Record the implementation-asset boundary in project guidance

Add a concise `Schemas and implementation assets` section to `AGENTS.md`:

- Source code, tests, and build scripts must not load or embed `openspec/` files.
  Required assets belong in maintained project locations; documentation may link
  to specifications and decisions.
- Manifest-content validity comes from the maintained schema, not duplicated
  predicates across flags, prompts, defaults, or output validation.
- Friendly presentation, input interpretation, repository-location checks, and
  safe file creation remain separate responsibilities.
- Archived schemas are historical snapshots, not synchronized assets or normal
  build/test inputs.

This is maintenance guidance, not a new product value. Review the foundations
after implementation, but no foundations or glossary change is expected. The open
questions about source references, versioning, workspace layout, and future agent
interfaces remain open; this change does not resolve them indirectly.

## Risks / Trade-offs

- **Regex engines may disagree about Unicode whitespace** → Compare the selected
  validator with current behaviour, including ASCII whitespace, NBSP, EM SPACE,
  U+0085, and mixed visible text. Evaluate supported regex configuration if needed.
  Do not silently weaken rules, remove failing cases, or retain a parallel Rust
  predicate. Escalate any unresolved semantic conflict before completion.
- **A runtime validator adds dependencies, build time, and binary/startup cost** →
  Disable unnecessary features, compile once, inspect the feature graph, and
  smoke-check release help, flag-only init, and guided responsiveness. This change
  does not claim performance measurements already exist.
- **Field validation can miss whole-object constraints** → Always validate the
  complete candidate before writing. Field feedback is an early check, not a
  replacement for final validation.
- **Error translation can obscure new constraints or internal failures** → Use
  structured failure information, preserve a truthful fallback, and test internal
  failure handling separately from retryable user input.
- **The schema and Rust writer/UI still require coordinated changes** → The schema
  governs validity but does not generate field support. Test actual output and
  document that adding a property may require type and interaction work.
- **Unknown properties can hide mistakes or imply unsupported capabilities** →
  Preserve the existing metadata-only policy and explain its limits. Tightening it
  requires a separate decision.

## Migration Plan

There is no file-format migration: command syntax, YAML default, and each selected
format's output remain unchanged. The loader deliberately tightens validation of
`$schema`: it previously ignored that property's value, but now enforces its
existing non-blank-string schema rule. Document this correction and test it for
every representable format.
Introduce the maintained asset, runtime validator, input integration, tests, and
documentation together in a focused implementation increment. Do not leave the
CLI and tests using different maintained schema sources between completed steps.

Run formatting, Clippy, locked tests, a locked release build, diff checks, strict
OpenSpec validation, packaged build verification, standalone-source checks, and
real-terminal exercises. Review documentation from a first-time reader's
perspective. Keep the originating todo open until implementation and verification
are complete.

If validation compatibility or dependency behaviour blocks the change, stop and
revise this design before release. Any rollback should revert only this change's
implementation, leaving user manifests, archived history, and unrelated work
untouched; it must not make the archive an executable dependency again.

## Open Questions

No further product choices are required for this increment. Implementation must
verify dependency features, regex/Unicode compatibility, standalone/package
behaviour, and acceptable responsiveness. These are verification gates, not
permission to change the agreed rules silently.

Public schema hosting and version selection, editor automation, alternative
manifest formats, existing-manifest workflows, and dependency semantics remain
outside this change.

## Implementation verification — 2026-10-17

The format branch merged without conflicts, and the schema, all-format adapters,
and authorized parser corrections passed independent review. The full suite has
161 passing tests; formatting, Clippy, release build, strict OpenSpec validation,
fresh-source tests without OpenSpec, and packaged-source verification passed.

Executable-only runs passed for all five formats, including an actual network-denied
macOS sandbox run. Rebuilt malformed and externally referenced schemas failed
before writes, with help/version still available. The live HTTP probe observed
zero retrieval requests. The existing real-terminal harness also passed against
the isolated executable. Schema dependency retrieval features remain disabled.

Release observations: 7,181,456-byte executable; ten local samples had help/init
medians of 2.271/4.271 ms, with one 445.815 ms first help launch. These are local
process observations without a pre-change baseline, not performance guarantees.
See [verification.md](verification.md) for commands, findings, evidence, and limits.
