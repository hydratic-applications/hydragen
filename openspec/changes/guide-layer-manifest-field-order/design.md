## Context

The reported sequence is `hydragen init`, followed by `hydragen add ../shared`.
Init already serialises its typed metadata in `name`, `author`, `description`
order. The block-YAML missing-field editor instead inserts `dependencies` before
the first key and its leading comments. Its tests deliberately expect that output.
Flow YAML and JSON/JSONC append missing fields at the end of the root collection;
TOML inserts after root assignments and before explicit table sections.

Add already promises insertion-only edits: existing bytes stay in order, comments
remain attached to their existing fields, and the candidate is reparsed and
schema-validated. A decoded-data comparison proves that only the intended reference
append occurred. Field placement must fit those guarantees, not override them.

### What npm demonstrates

Primary-source review on 2026-10-19 distinguishes several behaviours rather than
assuming every npm command applies one canonical order:

- [`@npmcli/package-json`'s `save`][npm-save] uses a preferred-order sorter only
  when the caller requests `sort`. Otherwise it serialises the existing object
  order and preserves detected indentation and line endings.
- [Its sorter][npm-sort] puts identity and descriptive fields before dependency
  groups, but calls its order arbitrary and retains additional fields.
- [Its dependency updater][npm-deps] copies the original object and updates the
  dependency groups in place. Existing top-level positions are retained; a newly
  introduced group is appended. It alphabetises entries *inside* dependency
  objects, not every top-level property.
- [`init-package-json`][npm-init] builds familiar defaults in a defined sequence
  and, in the reviewed source, requests sorted saving on its confirmed interactive
  path but not its `yes` path. This is evidence for familiar generated layouts,
  not a claim that npm always normalises field order.
- `package.json` uses JSON, whose [objects are unordered and arrays ordered][json].
  Some nested structures have separate meaning: for example, Node's conditional
  [`exports` key order][node-exports] matters. Neither a top-level presentation
  convention nor npm's dependency-object sorting licenses arbitrary recursive
  sorting in Hydragen.

The useful precedent is **guide what the tool writes without rejecting a user's
otherwise valid layout**. Hydragen retains stronger source-preservation guarantees
than npm's general object reserialisation and keeps dependency references ordered.

## Goals / Non-Goals

**Goals:**

- Make newly generated layer manifests and the first dependency addition read
  consistently, with identity before relationships.
- Define field order as a presentation convention, separate from content validity.
- Preserve manually chosen order, comments, additional fields, and dependency-list
  order across YAML, YML, TOML, JSON, and JSONC.
- Fix the block-YAML insertion boundary without changing operational safety or
  adding significant work to ordinary commands.

**Non-Goals:**

- A formatter, sort command, lint mode, ordering warnings, strict-order option,
  configuration setting, or automatic migration of existing files.
- Sorting dependency references, nested object keys, or unknown properties.
- Generating `$schema`, dependencies, a version, or other new metadata in init.
- Changing schema validity, filename priority, dependency resolution, validation
  traversal, rendering precedence, CLI feedback, or atomic replacement.

## Decisions

### Guide field order without enforcing it on input

Use this preferred order in maintained usage examples, skipping absent fields:

1. `$schema`, if supplied by the developer.
2. `name`.
3. `author`.
4. `description`.
5. `dependencies`.

Keep `author` before `description` to match existing init output and guided input;
copying npm's exact metadata sequence would create churn without helping this
workflow. Additional fields have no required relative position. A focused example
can omit fields or demonstrate another accepted layout when that is its purpose.

All otherwise valid field orders remain accepted without placement warnings,
including `dependencies` before `name`. Do not add order checks to the maintained
content schema or treat schema property enumeration as a writer policy. Format
syntax still applies: a TOML assignment after a table header belongs to that
table, regardless of its spelling.

**Alternatives:** Rejecting unconventional order or emitting warnings would create
friction for harmless differences. Automatically normalising on every mutation
would violate source preservation and enlarge diffs. A new explicit formatting
workflow could be considered separately if requested, but is unnecessary here.

### Keep init's established metadata order explicit

Retain typed struct serialisation through `encode_metadata`, which already avoids
the generic JSON map's enumeration order. Guarantee `name`, then supplied `author`,
then supplied `description` in every format, with omitted values still omitted.
Do not add a generic ordering framework or a runtime dependency to restate three
fields. Use ordered-source assertions to prevent a future map-based serializer
from silently changing this behaviour.

**Alternative:** Alphabetical output would place author before name and would not
address the scanability goal. Changing all generated metadata to npm's exact order
would unnecessarily depart from the established Hydragen layout.

### Append a missing dependency field without moving existing fields

When `dependencies` is absent, place it after all existing root entries, including
`$schema` and unknown fields, rather than finding a canonical slot among them.
For TOML, append within the root assignment area before explicit `[table]` or
`[[array-of-tables]]` sections, retaining comments attached to those sections.
This is an intentional syntax accommodation, not permission to insert a nested
dependency declaration.

When `dependencies` already exists, including an empty list or one placed before
`name`, extend it where it is. Keep existing reference order and append new
references in argument order. All-repeat plans still bypass editing and staging.

For the ordinary init-to-add workflow, the result is therefore `name`, supplied
`author`, supplied `description`, then `dependencies`. For hand-authored files,
preserving their layout takes priority over achieving the documented ideal.

**Alternatives:** Inserting immediately after the last known metadata field would
require new comment-boundary handling between existing fields, give unknown fields
an implicit placement rule, and differ from existing JSON and TOML behaviour.
Prepending preserves bytes but causes the reported inconsistent layout.

### Change the YAML tail boundary rather than reserialising

Replace the first-key boundary in block-YAML `absent_block_insertion` with a
parser/scanner-backed boundary after the complete final root value and its
attached comments. Keep the existing flow-root path unless regression tests reveal
a contract mismatch. Preserve the current spelling/style of inserted reference
literals; changing inline arrays to block sequences is not part of this change.

The block-mapping boundary must:

- Account for the final value's whole subtree, including nested collections,
  aliases, multiline quoted strings, and literal/folded block strings, rather than
  searching for the last colon or assuming a scalar's last token ends its value.
- Keep inline comments and more-indented own-line comments with the preceding
  field/subtree. Place the new field before remaining root-level own-line footer
  comments and before `...`; comments outside the document remain outside it.
- Preserve leading comments, directives, BOM, mapping/key tags and anchors,
  indentation, line-ending style, and whether the file has a final newline.
- Preserve block-scalar decoded values, including the trailing-newline behaviour
  controlled by `|`, `|-`, `|+`, and their folded counterparts. Do not insert text
  into their content or consume their meaningful trailing blank lines.
- Stay within existing bounded parsing/scanning. If a specific construct cannot
  safely support tail insertion, return a specific preparation error before
  staging; do not silently fall back to prepending or broad reserialisation, and
  do not blanket-refuse ordinary comments or block strings.

The exact candidate still passes the shared decoder, maintained schema, and
intended-append proof in `src/layer_manifest/edit.rs` before any write. This is
especially important where YAML collection-end positions and block-string
boundaries do not coincide with safe insertion points.

**Alternative:** A text-only EOF append can land outside the YAML document, change
block-scalar content, or misattach a footer. A general reformatter would discard
the source guarantees to solve a small placement issue.

### Keep the policy focused and document the reported workflow

This resolves the field-placement subset of design questions 2 (layer manifest
content) and 10 (everyday developer experience), without settling broader content,
rendering, registry, or formatting questions. Link this proposal from the shared
question list as a proposed resolution; promote the wording when implemented.

During implementation, add concise guidance to the existing README layer manifest
section: the preferred order is a convention, arbitrary valid order works, and add
does not reorder existing fields. Align `AGENTS.md` and maintained usage examples;
do not rewrite archived artifacts or reorder test fixtures intentionally exercising
alternate layouts.

The foundations' DX and Efficiency values already support useful consistency
without unnecessary uniformity. Extend the existing returning-to-projects scenario
with the concrete init-then-add example during implementation, rather than adding
a new guiding value or a technical ordering rule. No glossary change is needed:
no new product term is introduced.

## Risks / Trade-offs

- **Appending after a YAML block string can change its value** → Use structural
  ranges and chomping-sensitive tests, and retain the shared decoded-change proof.
- **Comment intent is not fully encoded by YAML syntax** → Keep established
  attachment rules, state the root-footer convention, and assert exact surrounding
  fragments as well as decoded data.
- **Preferred order will not be universal** → Explicitly prioritise existing
  layouts; this increment improves defaults rather than promising normalisation.
- **Tests currently require dependencies-first YAML** → Change those expected
  outputs while retaining all their preservation cases, and add placement tests
  so the change cannot pass on decoded equality alone.
- **Different formats have different scope/delimiter rules** → Verify equivalent
  root-level meaning and placement in all five formats instead of imposing one
  physical text-edit algorithm.

## Migration Plan

No data migration or compatibility flag is needed. Existing field order remains
valid; files already containing dependencies are not reordered. The behavioural
change applies only when a missing field is introduced. Implement the focused
editor change with regression tests and aligned guidance. Reverting it would
restore old placement on later missing-field insertions without invalidating any
files created under this policy.

## Open Questions

None block this increment. Whether to offer an explicit formatter and where
future layer manifest fields belong remain separate decisions; no command or
placeholder configuration is reserved here.

[npm-save]: https://github.com/npm/cli/blob/ae98c62586b6fea44114f72e249e246a2981de70/node_modules/%40npmcli/package-json/lib/index.js
[npm-sort]: https://github.com/npm/cli/blob/ae98c62586b6fea44114f72e249e246a2981de70/node_modules/%40npmcli/package-json/lib/sort.js
[npm-deps]: https://github.com/npm/cli/blob/ae98c62586b6fea44114f72e249e246a2981de70/node_modules/%40npmcli/package-json/lib/update-dependencies.js
[npm-init]: https://github.com/npm/init-package-json/blob/70ef43b305cc32fc9eadbf12e75ae6c3a2780208/lib/init-package-json.js
[json]: https://www.rfc-editor.org/rfc/rfc8259#section-4
[node-exports]: https://nodejs.org/api/packages.html#conditional-exports
