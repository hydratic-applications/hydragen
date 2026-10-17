## Context

The prototype implements only `hydragen init`. In `src/lib.rs`, `Metadata`
serialises to pretty JSON, `preflight` checks the repository location and the
single `layer.json` path, and `initialize` repeats that check before exclusive
creation. `src/main.rs` chooses between metadata flags and guided input.
`src/interaction.rs` contains the interaction policy and cliclack presentation.
There is no existing layer manifest reader or layer-manifest-dependent command to extend.

`tests/init.rs` exercises captured CLI output, filesystem safety, and same-path
concurrency. `tests/interactive_init.rs` has a bounded pseudo-terminal harness
with child cleanup. Both assume JSON output and a three-question guided flow.

The foundations emphasise familiar editing, low mental overhead, and clear source
selection. Supporting established formats serves those values; a deterministic
choice and visible duplicates avoid hiding which layer manifest governs a layer.
Open design questions 3 and 9 are affected: the JSON-only filename decision and
the list of flags/questions selecting each input route. Dependency syntax,
versioning, and broader workspace layout are not affected.

## Goals / Non-Goals

**Goals:**

- Support five fixed filenames with equivalent metadata meaning.
- Make selection predictable and duplicates visible, including on failures.
- Supply a tested shared reader now without inventing a new CLI command.
- Default new layers to YAML and keep complete guided and flag-only routes.
- Preserve no-overwrite, cancellation, output-stream, and repository-boundary
  guarantees while following the agreed, deliberately limited concurrency policy.

**Non-goals:**

- Conversion, repair, merging layer manifests, custom filenames, read-format overrides,
  automatic migration, or comment-preserving edits of existing layer manifests.
- Other formats such as JSON5, HJSON, INI, XML, and executable configuration.
- Dependency syntax, fetching, rendering, capture, watching, lock files, or
  serialising concurrent initialisation across different filenames.
- Moving or publishing the archived draft schema, completing the separate todo
  about source tests referencing that schema, or adding a runtime schema engine.
- Changing current product documentation or main specs during this planning-only
  session. Those edits are implementation tasks; archives remain historical.

## Decisions

### Fixed filenames and an explicit priority order

Use one library-owned format enum with five choices, default, filename mapping,
and an ordered list:

1. `yaml` → `layer.yaml` (default and highest priority)
2. `yml` → `layer.yml`
3. `toml` → `layer.toml`
4. `json` → `layer.json`
5. `jsonc` → `layer.jsonc`

`yaml` and `yml` are separate filename choices using the same YAML implementation.
The enum does not let users supply a path or change `layer`. A layer manifest's `name`
field is identifying metadata, unrelated to its filename.

There is no universal cross-format precedence. Docker Compose explicitly prefers
`compose.yaml` over `compose.yml`; Prettier documents a different order with JSON
before explicitly named YAML configs. Hydragen chooses its own clear convention:
default YAML first, both YAML extensions together, comment-friendly TOML next,
then the pre-existing strict JSON filename before the commented alternative. TOML
is an established configuration format in Cargo and Python's `pyproject.toml`;
it adds a useful familiar choice without an executable configuration runtime.
Its placement preserves the relative order of every previously agreed filename.
Do not claim this complete order is prescribed by another tool or by a language
standard.

**Alternatives rejected:** JSON first solely for historical continuity; JSONC
first solely because comments are useful; filesystem enumeration order; modification
time; merging; configurable precedence; or an interactive choice on duplicates.
These would disconnect the default from precedence or make governing input harder
to predict. Existing JSON-only layers are unaffected.

### Separate discovery, loading, validation, and presentation

Add a focused `src/layer_manifest.rs` module, exported through `src/lib.rs`. Keep init's
repository-boundary and creation logic in the existing library unless its size
justifies a small init module; avoid unrelated refactoring.

Use these conceptual interfaces; exact Rust names can follow surrounding style:

- **Format:** fixed enum, priority list, filename, and default. Keep clap's parsing
  adapter in the binary rather than making the library depend on CLI presentation.
- **Discovery:** inspect the five paths in one explicitly supplied directory and
  return ordered candidates, the selected candidate if any, and duplicate data.
  No content reads, logging, upward search, or project changes.
- **Loading:** accept a discovery/selected candidate and read only that candidate,
  using its filename's parser. Return the path, format, validated metadata, and
  decoded document including uninterpreted additional fields.
- **Validation:** require an object/mapping, inspect known field types and
  presence, then reuse common non-blank validation without normalising strings.
  Use a layer manifest/metadata error that does not describe a read failure as an init
  flag error; init can wrap or format it for its own input route.
- **CLI diagnostics:** render discovery data before calling loading or refusing
  existing layer manifests. Discovery must survive a later error; a success-only loader
  result is insufficient. Library functions never write to terminal streams.

Discovery operates on the supplied layer root only; it does not settle how future
commands locate a layer from a subdirectory. Keep init's existing canonicalisation
and Git-location check. Future layer-manifest-dependent commands must discover, report
duplicates, then load through this shared API, not invent local filename searches.
No global scan before clap help/version handling is added.

**Alternative rejected:** one function that returns only successfully loaded
metadata, because it loses duplicate information on parser and validation failures.
A generic command framework or multi-layer warning registry is unnecessary while
init is the sole command; ownership of the discovery result provides once-only
reporting now, and establishes the convention for future commands.

### Count occupied names, not just valid readable files

Use `symlink_metadata` as the existing preflight does, so a dangling symlink or a
directory is not mistaken for an absent path. All occupied supported names count
for initialisation refusal and selection; do not silently skip an unusable higher
priority entry. Ignore unrelated filenames and nested layer manifests.

For read-only loading, a symlink resolving to a regular file is readable, while
its layer manifest filename still determines format. Report a selected dangling symlink,
directory, or other non-regular target as a path-specific error. Check target kind
before reading so a FIFO does not make ordinary layer manifest loading wait for input.
These are normal local filesystem checks, not a hostile-filesystem sandbox or a
promise to prevent every race involving a path replaced between inspection and
opening. Do not add case-folding or accept extra spellings beyond the host
filesystem's ordinary lookup behaviour.

An unexpected discovery inspection error fails the initial/read operation rather
than treating that name as missing. A missing selected file at load time is also
an error, not a reason to select a lower-priority file. Lower-priority file contents
are never read or validated.

### Use maintained parsers, not hand-written syntax processing

Retain `serde_json` for strict JSON and for JSON/JSONC output. Recommended new
dependencies, based on documentation reviewed on 2026-10-16, with TOML added on
2026-10-17:

- `serde-saphyr` 2.x, with default features disabled and `serialize` and
  `deserialize` enabled, for YAML input/output. Its documented Rust minimum is
  1.89, below the repository's documented tested toolchain, 1.96.
- `jsonc-parser` 0.34.x for JSONC input. Use its AST interface if needed to retain
  repeated object properties long enough to reject duplicates before converting
  to a map. Do not enable editing/CST or faster non-DoS-resistant hashing features.
- `toml` 1.1.x for TOML input/output, using its `std`, `serde`, `parse`, and
  `display` features without editing, unbounded parsing, or alternative hashing
  features. Version `1.1.8+spec-1.1.0` was non-yanked with Rust minimum 1.85 when
  checked on 2026-10-17; the documented API exposes typed tables/values and Serde
  input/output. It targets TOML 1.1 while accepting ordinary TOML 1.0 documents.

These are planned dependency choices, not a claim that a compatibility spike has
already passed. Resolve exact versions and record them in `Cargo.lock` during
implementation. First run focused parser contract tests; if the documented
behaviour does not hold, report the gap rather than silently relaxing the spec.

Input contracts:

- `.json` stays strict JSON: comments and trailing commas are errors.
- `.jsonc` supports line/block comments and trailing commas. Explicitly disable
  all unrelated permissive parser options, including single quotes, unquoted
  names, missing commas, hexadecimal/non-finite numbers, and extended escapes.
  JSONC libraries' defaults can be substantially broader than JSONC.
- YAML follows ordinary YAML 1.2 data behaviour and uses the same parser for both
  extensions: one mapping document, comments, normal quoted/plain/block strings,
  and ordinary anchors/aliases. No bespoke tokenizer or indentation convention.
  Reject YAML 1.1 merge-key composition and unsupported custom tags; do not enable
  include, interpolation, robotics, or other domain extensions. Standard `!!str`
  remains valid; an explicit string is still a string.
- TOML follows the maintained parser's standard TOML document rules, with metadata
  at the top level, not under a `[layer]` wrapper. Accept `#` comments, basic and
  literal strings, multiline strings, dotted keys, tables, inline tables, and
  arrays of tables for otherwise supported data. Let the parser enforce duplicate
  key and invalid table-redefinition rules; do not confuse a valid sequence of
  array-of-table entries with duplicate layer manifest keys.
- Parse TOML into its native typed table/value representation before converting
  to shared decoded data. Do not silently stringify numbers, booleans, or native
  date/time values to make them pass string metadata validation. The layer manifest's
  shared JSON-compatible data model has no native date/time type: reject such
  literals with guidance to use a quoted string, rather than allowing Serde's
  internal datetime representation to leak into the layer manifest. Likewise reject
  non-finite numbers. TOML has no null value syntax; omitted optional keys remain
  absent and are not synthesised as null. These are layer manifest data constraints,
  not a hand-written replacement for TOML syntax parsing.
- Decode values before metadata type validation. A YAML `name: true`, `name: 123`,
  `name: null`, or empty value must not become a valid string merely because the
  Rust destination type is `String`. YAML 1.2 `yes`/`on` are strings, not booleans.
  Configure YAML accordingly and verify against its generic value interface or
  strict type options; do not rely on type-driven coercion defaults.
- Reject repeated mapping/object keys, including repeated unknown fields and
  nested objects, before a generic map can discard them. Check decoded key names
  so escaped spellings of the same JSON key are duplicates. For strict JSON, use
  a duplicate-checking Serde map visitor rather than plain `Value` deserialisation.
- Keep the decoded data compatible with JSON-style layer manifest data: string-keyed
  objects, arrays, strings, booleans, finite numbers, and null for uninterpreted
  fields. Reject complex YAML keys, TOML native date/time literals, and non-finite
  numbers instead of silently changing their meaning. Known metadata fields
  impose stricter string rules.
- Preserve parser recursion/alias budgets. No recursive textual substitutions,
  regex comment stripping, tag-driven object construction, or external services.

The shared validator requires non-blank string `name`; optional `author` and
`description` must each be non-blank strings if present. Omitted is different from
explicit null. Additional fields are accepted and retained as data, but not
interpreted or validated as dependencies. Loading does not rewrite the source or
promise comment preservation in an eventual editing feature.

#### YAML integer overflow compatibility guard

Implementation evidence on 2026-10-17 exposed a gap in `serde-saphyr` 2.0.0:
some implicitly numeric hexadecimal/octal scalars become strings when conversion
overflows. A value such as `name: 0x10000000000000000` can therefore bypass the
non-string metadata rule; numeric mapping keys and nested values are affected too.
The typed-string strict option only moves the boundary and is not a robust fix.

The authorised evaluation of maintained alternatives found the same class of
fallback in `yaml_serde` 0.10.7, `serde_norway` 0.9.42, and `saphyr` 0.2.2.
The first two rejected the 2^64 case but accepted 2^128 and larger hexadecimal/
octal integers as strings; the third failed from 2^64. Native parsed values were
tested in eight metadata/value/key positions before any integration. Quoted and
explicit `!!str` controls remained valid. None was accepted as a replacement;
this bounded evaluation does not claim a complete compatibility test of them.

After reviewing that evidence, the user explicitly authorised one narrow
exception to avoiding local scalar classification: inspect the maintained YAML
parser's scalar events and apply YAML's existing integer syntax rules to reject
unsupported integer overflow. Keep `serde-saphyr` and its normal parsing and
serialization. This guard MUST NOT implement a YAML tokenizer, strip comments,
invent new scalar syntax, change type semantics, or reject numeric-looking
quoted strings and explicit string tags. Respect explicit numeric tags even
when a scalar is quoted. All normal syntax, quoting, tags, document structure,
aliases, and parser limits remain the library's responsibility.

Add regressions for both extensions, each metadata field, nested additional
values, arrays, mapping keys, signed/boundary/very-large forms, and corresponding
quoted/explicit-string positive controls. The guard is compatibility hardening,
not a relaxation of the layer manifest contract. Its extra parser-event inspection is
limited to YAML reads; init still never parses existing layer manifest contents.

Evaluation sources: [yaml_serde 0.10.7](https://crates.io/api/v1/crates/yaml_serde/0.10.7),
[serde_norway 0.9.42](https://crates.io/api/v1/crates/serde_norway/0.9.42), and
[saphyr 0.2.2](https://crates.io/api/v1/crates/saphyr/0.2.2). The standalone probe
is retained in the approved temporary workspace as
`hydragen-yaml-candidate-probe/`; rerun it with `cargo run --quiet --locked`.

#### Coordinate with the maintained-schema plan

A separate saved project plan, **Make the maintained layer manifest schema drive CLI
validation**, proposes a maintained `schemas/layer-manifest.schema.json`, an
embedded validator, and `src/layer_manifest.rs`. That implementation is not present in
this checkout. Its ownership/validation work and this format work overlap in the
same module and init callers; they must not establish competing validation rules.

Before implementation, recheck the checkout and that plan. If its validator has
landed, extend the existing module and pass every decoded format's complete data
through the maintained schema validator. Keep schema-backed field feedback for
prompts/defaults, including any existing `$schema` property constraints. Parser
syntax, duplicate-key detection, filename discovery, and safe writes remain
separate responsibilities; they do not replace schema validation. Do not restore
a handwritten validity predicate alongside the maintained schema.

If both changes are scheduled, prefer landing the maintained-schema work first
and then building format support on its reusable parsed-value validator. This is
a coordination recommendation, not an added product prerequisite or permission
to implement the other plan here. If formats land first, retain the current
shared validation call point so the schema work can replace its implementation
without duplicating format-specific rules. Resolve module/signature conflicts
against the actual implementation rather than replacing either module wholesale.

**Alternatives rejected:** archived/deprecated YAML dependencies without review;
JSON5 as a substitute for JSONC; letting each parser enforce a different metadata
contract; or deserialising directly into `Option<String>` without guarding null.
This format change introduces no schema engine of its own. Schema ownership and
runtime integration belong to the maintained-schema work above; reuse its
validator when present. Publishing a schema address remains out of scope.

### Keep format selection within the existing input-route policy

Expose `--format yaml|yml|toml|json|jsonc` as an init-only value flag. The CLI stores its
presence separately from the resolved YAML default, for example as an `Option`:
an inferred default must not make bare init look like an explicit flag invocation.
Use clap value validation with lowercase extension choices and no path aliases.

Any of `--name`, `--author`, `--description`, or explicit `--format` selects
flag-only execution. Rename the internal `metadata_flags` concept to `input_flags`
where it controls the route. Keep terminal, CI, and `--non-interactive` behaviour.
`hydragen init --format jsonc` uses the directory-name default, omits author and
description, and asks nothing. An invalid or missing format value is a clap error,
not an invitation to prompt. `--format` is not a read-time precedence override.

Bare eligible terminal init asks Name, Author, Description, then Layer manifest format.
Use a cliclack select with choices in priority order and `layer.yaml` preselected;
each label includes its exact filename. Enter accepts it. Keep all answers in
memory until every question completes. Escape, Ctrl-C, or input failure at the
new selector follows the existing no-files/nonzero cancellation path.

Return metadata and format as separate init inputs, not as a new layer manifest field.
Update the existing optional-field presentation carefully: the custom skipped
placeholder theme must not mislabel a format selection as `Skipped`.

### Write minimal, format-appropriate output

Serialise and validate before opening the target. YAML/YML use a readable block
mapping with safe quoting/escaping; TOML uses readable top-level key/value entries
with quoted string values and no `[layer]` wrapper; JSON/JSONC use the existing
two-space pretty JSON style. All output is UTF-8 with a final newline, and decoding recovers accepted
metadata exactly, including surrounding whitespace, Unicode, and newlines.
For YAML, test strings resembling booleans, nulls, numbers, dates, and comment
syntax so output remains string data when decoded by ordinary YAML tools.
For TOML, use its serializer rather than concatenating strings; test basic/literal
and multiline cases, escaped newlines, backslashes, and date-like string content.
Keep its generated output compatible with ordinary TOML 1.0 tools where possible;
these flat string fields need no newer syntax. For example:

```toml
name = "shared-agent-rules"
author = "Example team"
description = "Shared coding-agent instructions."
```

JSON output is also valid JSONC, so `.jsonc` needs no generated header or starter
comments. Init emits only `name` and supplied author/description, in the existing
field order; it adds no schema URL, dependencies, format property, or version.
Use the real created path in success output, which remains on stdout.

### Check once for permission to initialise; keep exact-target exclusive creation

Replace the current repeated preflight pattern with distinct preparation,
creation, and observation steps:

1. **Inspect/prepare at command start:** canonicalise the directory, keep repository
   boundary checks, and discover all five paths. Make duplicate data available to
   the CLI, then refuse any occupied name before guided input or writes. No
   existing layer manifest content is read. This check does not reserve the directory.
2. **Create using that prepared target:** after flags/defaults or all guided
   answers are complete, serialise the requested format and open only its fixed
   target with `create_new(true)`. Do not repeat the all-format refusal check.
   This preparation/creation split must be explicit in the library API so a
   convenience wrapper does not accidentally reintroduce final rejection.
3. **Observe after writing:** rediscover the five names without parsing them.
   If multiple exist, warn which takes priority now and still report success for
   the file actually created, even when that file has lower priority. No rollback,
   delete, rename, repair, or change to another process's file.

A competing entry at the exact chosen filename still causes an error through
exclusive creation, including a directory or dangling symlink. If this happens,
best-effort rediscovery can supply a duplicate warning before that error; it must
not retry or replace the target. Same-format races therefore retain one successful
creator, but different formats can both succeed if both started from an empty
inspection. A different format appearing during guided questions is also allowed.

Post-write observation is best effort. An inspection failure after creation emits
a warning identifying that duplicate checking failed, preserves the successful
result, and does not falsely claim no files changed. As today, actual write
failures can leave a partial newly created file; failures to report a created
result retain the existing explicit reporting-error semantics.

**Alternatives rejected:** lock files, late all-format rejection, cleanup/rollback
of a successfully created layer manifest, and guarantees that a post-write observation
can detect files created after it. The user chose low overhead and observation
over cross-format serialisation; a warning describes observed state, not proof of
another running process.

### Warn once, before a dependent failure, with truthful wording

Use plain stderr diagnostics for duplicate warnings, including flag-only and CI
execution. Do not add a logging framework or a generic startup layer manifest scan.
Use the same message builder for init and future reader command callers. Include
the directory, every occupied filename in priority order, the selected filename,
and the complete priority order (or an equally clear explanation of selection).

Example reader warning:

```text
Warning: Multiple layer manifests found in '/project': layer.yaml, layer.json. Using layer.yaml (priority: layer.yaml > layer.yml > layer.toml > layer.json > layer.jsonc).
```

For an initial init refusal, use wording such as `layer.yaml takes priority`
rather than claiming init loaded an existing file. For post-creation duplicates,
identify the chosen layer manifest under the same rule, followed by the ordinary
success line naming the file init actually created. The warning itself never
changes exit status. Explain that duplicates are not merged and should be reduced
to the one intended layer manifest; do not automatically remove them.

The initial existing-layer-manifest path reports once and stops. A successful creation
started with no candidates, so only its final discovery can produce the duplicate
warning. Avoid re-rendering the same discovery through nested helpers. Future
commands touching multiple layers should report once for each affected directory
within their command context; no unused global cache is needed in this increment.

Help/version and clap usage failures do not inspect project layer manifests. Known
repository-location failures may stop before discovery; do not scan unrelated
directories merely to emit a warning.

### Update current documentation without rewriting history

- README: lead with YAML examples and default, document every exact filename and
  priority, `--format`, guided selector, shared validation, strict JSON versus
  JSONC, a TOML example and its top-level string fields, duplicate warning examples,
  and concurrency limits. State that only init
  exists and that loading is currently a library capability, not a CLI command.
- Foundations: replace the JSON-only statement in the layer model with the shared
  layer manifest concept and supported fixed filenames. Keep parsing details in the
  specifications/README; preserve the eight guiding values and unrelated material.
- Glossary: update Layer, Root layer, and Layer manifest, using links to the existing
  Layer manifest heading rather than inventing a new term for each format.
- AGENTS: extend input flags to include `--format`, record the YAML default and
  separate routes, and clarify initial checks versus exclusive final creation.
- Design questions: mark the filename and input-route decisions as superseded by
  this change, with links to the relevant decision headings. Preserve unrelated
  open questions and historical context. Update at implementation time so the
  current docs do not prematurely claim an unimplemented feature exists.
- Main specs: apply these deltas through the normal OpenSpec sync/archive workflow
  once implementation is verified; do not hand-edit archived changes.

No foundational philosophy change is needed. These are targeted factual updates,
not a reason to add a new guiding value or redesign unrelated product choices.
The archived JSON Schema still describes the same required/optional metadata;
label links to it as a historical metadata-only draft rather than extending code's
dependency on that path. Do not silently complete the separate schema-location
todo as part of this work.

## Implementation sequence and verification

The detailed checklist is in [tasks.md](tasks.md). Implement in four reviewable
increments, each with the tests for the behaviour it introduces:

1. **Layer manifest core:** format mapping, discovery, error/data contracts, parsers,
   validation, and output codecs. Prove parser policy with focused tests before
   integrating CLI changes.
2. **Flag-only init and filesystem lifecycle:** explicit format/default handling,
   inspect/create split, duplicate warning rendering, and post-write observation.
3. **Guided init:** final selector and update the real-terminal test flows without
   weakening their waits, assertions, or child cleanup.
4. **Documentation and verification:** current docs, decision links, complete
   regression checks, and OpenSpec validation. No implementation begins until
   separately requested.

Verification must cover:

- All 32 filename-presence subsets, not only adjacent priority pairs; no content
  reads during discovery; no parsing of lower-priority candidates.
- All output formats and optional-field combinations, exact value round-trips,
  YAML string ambiguities, absent versus null, wrong types, empty/non-object
  documents, repeated keys at every object depth, and invalid UTF-8 input.
- JSONC comments/trailing commas and rejection of permissive JSON5 extensions;
  strict JSON rejection of the same extensions; ordinary YAML comments, block
  strings, standard tags, and bounded anchors, plus stated unsupported cases.
- TOML comment/string forms and exact output round-trips; top-level metadata;
  ordinary tables, dotted keys, and arrays of tables for extra data; duplicate
  keys/invalid table redefinitions; typed metadata rejection; native date/time and
  non-finite numeric rejection without hidden coercion or datetime wrapper objects.
- High-priority syntax/read/validation failures with valid lower-priority files:
  error on the chosen path, no fallback, duplicate data still available to render.
- Occupied names of every supported extension and kind; selected readable
  symlinks; dangling symlinks and non-regular targets; repository-boundary and
  inspection errors, with no new runtime services or Git subprocesses.
- CLI defaults and all format values, missing/invalid flag values, format-only
  terminal invocations, CI/redirected streams, help/version, stderr/stdout
  separation, and no warning for a single layer manifest.
- Guided default and non-default selections, cancellation at the format selector,
  no early writes, existing entries before questions, and alternate formats
  appearing during the questions without causing late refusal.
- Same-target concurrency retaining exclusive creation, and deterministically
  interleaved different-format creation from two prepared targets, with no lock
  files. Test observation at a known point rather than hoping scheduler timing
  makes a warning appear in two unconstrained processes.
- Post-write scan failure, exact-target collision reporting, real write failure,
  and result-reporting failure without incorrect no-files-changed claims.

Keep selected existing JSON-focused tests explicit with `--format json` where
their purpose is JSON escaping/formatting; move default-output tests to YAML.
Do not let shared loader assertions replace all independent output validation,
which could hide a matching reader/writer bug. Use parser-level expected values
and a few exact output fixtures alongside end-to-end loader round-trips.

Required final commands: `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, and
`openspec validate support-layer-manifest-formats --strict`. Include bounded
real-terminal sessions and an independent review of revised documentation from
the perspective of a developer who has not read this conversation.

## Risks / Trade-offs

- **Parser defaults differ** → Explicit options, typed-value validation, duplicate
  detection before map insertion, and adversarial/negative format fixtures.
- **The new default changes scripts** → Mark the compatibility change clearly and
  show `--format json`; retain existing JSON files without migration.
- **Discovery information can disappear on errors** → Keep discovery separate from
  loading/refusal and render it before the dependent operation.
- **Init currently rechecks too late for the agreed policy** → Introduce the
  prepared-target split and a deterministic test of another format appearing
  during prompts. Do not simply expand the existing repeated preflight loop.
- **Duplicate observations are snapshots** → Document that other processes can
  create files after the final check, and never imply the warning proves a race
  or that no warning guarantees exactly one layer manifest forever.
- **Comment-aware formats invite expectations about editing** → Keep this change
  read-only for existing files, with no promise to rewrite or preserve comments
  in a future mutation feature.
- **Overengineering early infrastructure** → Limit new abstractions to format,
  discovery, loading/validation, creation lifecycle, and a small diagnostic helper.
  No future-command implementations, global cache, or logging subsystem.

## Migration Plan

Existing single-file `layer.json` repositories need no changes. New init defaults
to `layer.yaml`; scripts needing JSON opt in with `--format json`. Projects with
multiple supported paths get a deterministic choice and a warning, not automatic
migration. Removing unwanted duplicate files is a deliberate user action.

No data migration or destructive rollback is required. The implementation should
land with updated help/docs and tests together; if a regression is discovered,
stop using the affected build rather than converting or deleting layer manifests
automatically. Never create a JSON duplicate merely to support an older build.

## Remaining assumptions and missing implementation evidence

No unresolved product question blocks the agreed increment. The following are
explicit implementation assumptions or checks, not established test results:

- Ordinary local filesystem lookup and reading a symlink to a regular file are
  supported; hostile filesystem replacement and network filesystem guarantees
  remain outside the prototype's safety contract.
- Post-write duplicate-check failures are non-fatal observation warnings; actual
  write and result-reporting errors retain their existing failure semantics.
- Exact parser versions, their duplicate/type/null behaviour, transitive build
  compatibility, and how best to disable optional features need verification in
  the first implementation increment. No parser spike or Cargo regression suite
  was run during this planning-only work.
- The separately saved maintained-schema plan may land before implementation.
  Re-read its delivered validator/module and reuse it; this proposal neither
  supersedes that plan nor authorises implementing it.
- Future commands must adopt the shared discovery/warning convention when they
  are introduced. Their command semantics, root search, dependency traversal, and
  long-running warning refresh behaviour remain separate design work.

## Implementation verification — 2026-10-17

The four implementation phases were completed against the requirements above.
The parser compatibility checks that were pending at planning time now have
runtime evidence:

- Resolved dependencies: `serde-saphyr` 2.0.0, `jsonc-parser` 0.34.0, and
  `toml` 1.1.8+spec-1.1.0, with the intended limited feature sets in Cargo files.
- The user-approved [YAML overflow guard](#yaml-integer-overflow-compatibility-guard)
  closes the investigated implicit-number-to-string fallback. The dependency was
  retained after bounded evaluation of alternatives. No layer manifest rule was relaxed.
- JSONC requires additional validation of parser-provided original string-token
  ranges and whitespace gaps using `serde_json`: disabling extension switches
  alone still accepted raw control characters and non-JSON whitespace. This uses
  the maintained parser's tokens, not a new tokenizer or comment-stripping pass.
- `cargo test --locked`: **122 passed**, comprising 11 library, 15 binary,
  33 captured init, 7 lifecycle, 17 real-terminal init, 31 layer manifest, and
  8 YAML integer regression tests. This includes all 32 discovery subsets,
  independent output expectations, selected-only errors, exact-target collisions,
  different-format interleavings, post-write observation failures, and selector
  cancellation/input loss with bounded waits and child cleanup.
- `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, and
  strict validation of this OpenSpec change passed. Independent phase reviews
  accepted core parsing, creation/diagnostics, guided input, and documentation.
- Ten bounded standalone CLI smoke checks passed using an isolated binary copy:
  five exact output fixtures, duplicate priority/refusal, and help/version/usage
  discovery bypass. Documentation was reconciled with runtime examples; all
  195 local documentation links/fragments resolve, the eight guiding values are
  unchanged, and only relevant design questions 3 and 9 were revised.

The documented limitations remain intentional: duplicate observations are
snapshots without locks, filesystem checks are not a hostile-filesystem sandbox,
and unsupported native data is rejected rather than silently converted. Schema
ownership/runtime-schema work remains separate and was not implemented here.
The separately captured `add-validate-command` proposal is future planning only;
no validate/read command was added. Main-spec sync and archive remain separate
workflow actions, not side effects of this implementation.

## Sources

Primary documentation reviewed on 2026-10-16, with TOML sources added on
2026-10-17; these sources inform the choices,
but do not establish a universal priority order or prove our implementation:

- [Docker Compose file preference](https://docs.docker.com/compose/intro/compose-application-model/#the-compose-file).
- [Prettier configuration precedence](https://prettier.io/docs/configuration).
- [JSONC syntax and optional trailing commas](https://jsonc.org/), and
  [Deno's comments/trailing-commas convention](https://docs.deno.com/runtime/fundamentals/configuration/#denojson).
- [YAML 1.2.2 specification](https://yaml.org/spec/1.2.2/).
- [TOML 1.1 specification](https://toml.io/en/v1.1.0),
  [Cargo manifest format](https://doc.rust-lang.org/cargo/reference/manifest.html),
  and [Python's pyproject.toml](https://packaging.python.org/en/latest/guides/writing-pyproject-toml/).
- [toml crate documentation](https://docs.rs/toml/1.1.8+spec-1.1.0/toml/)
  and [published version/toolchain metadata](https://crates.io/api/v1/crates/toml/1.1.8+spec-1.1.0).
- [serde-saphyr 2.0.0 documentation](https://docs.rs/serde-saphyr/2.0.0/serde_saphyr/)
  and [parser options](https://docs.rs/serde-saphyr/2.0.0/serde_saphyr/options/struct.Options.html).
- [jsonc-parser 0.34.0 documentation](https://docs.rs/jsonc-parser/0.34.0/jsonc_parser/)
  and its [ordered object-property AST](https://docs.rs/jsonc-parser/0.34.0/jsonc_parser/ast/struct.Object.html).
