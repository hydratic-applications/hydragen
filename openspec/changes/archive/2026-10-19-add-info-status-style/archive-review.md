# Batch archive review — 2026-10-19

## Scope and order

The user selected and confirmed these six completed changes for one batch archive
on PR #7's existing branch, without merging the PR:

1. `define-cli-visual-identity`
2. `unify-cli-usage-errors`
3. `unify-cli-diagnostics`
4. `compact-single-add-results`
5. `guide-layer-manifest-field-order`
6. `add-info-status-style`

All four planning artifacts are present for each change. Their task counts are
15/15, 16/16, 12/12, 4/4, 12/12, and 4/4 respectively. Each change passed strict
OpenSpec validation before archiving. No selected change needs an incomplete-work
warning or skipped specification sync.

## Overlap resolutions

All overlapping changes are implemented. Their specification updates were merged
in the implementation order above, established from Git history rather than
working-tree modification times.

- **`cli-interaction`:** Apply visual identity, usage errors, CLI-wide diagnostics,
  then info styling. `src/presentation.rs`, `src/interaction.rs`, `src/main.rs`,
  `src/usage_errors.rs`, `src/diagnostic_body.rs`, and `src/diagnostics/` implement
  the shared styles, cliclack theme, parser-first handling and typed error bodies.
  Captured and PTY tests cover each family. The later CLI-wide rules supersede
  the usage delta's historical promise to leave other operational presentation
  unchanged: single failures are direct, and multiple failures use source groups.
- **`layer-initialization`:** Apply visual identity before field order. Init
  completion keeps its filename-only message, guided stderr and flag-only stdout.
  `tests/field_order.rs` covers generated metadata order in all formats; the init
  and interactive-init suites cover presentation, cancellation and write safety.
- **`layer-dependency-addition`:** Apply compact results, field order, then info
  styling. `Presentation::add_success` uses verified outcome cardinality and
  informational all-repeat results. `src/layer_manifest/edit/yaml.rs` and the
  field-order/edit suites verify insertion-only tail placement and comment
  preservation. The add and interactive-add suites cover single/grouped/mixed
  results and no-op storage guarantees. The newest result requirement supersedes
  the earlier green-check treatment for no-ops, not its compact layout.

Ten delta files were synchronised into four main specifications, including the
non-overlapping field-order addition in `layer-manifest`. Existing main-spec
rules for schema-valid directory defaults and argument-only add were retained
rather than replaced by the older visual-identity text. Older result scenarios
that required a counted group for a single result were replaced by the newer
single-versus-grouped contract. The add diagnostic requirement was aligned with
the shared direct-single/source-grouped-multiple rule.

## Archive maintenance

The six directories use the `2026-10-19-` archive prefix. Existing metadata and
historical artifacts are retained; only relative links in the visual-identity
proposal/design need relocation repairs. Incoming links in the design-question
list and brand reference are updated too. Previously archived changes and all
four unselected active records remain untouched. No source code, schema, tests,
foundations, or glossary behaviour changes are needed for this closeout.

## Verification after archiving

- All six directory moves succeeded and preserved their `.openspec.yaml` files.
- Strict validation passed for all seven main specifications.
- All 536 locked Rust tests passed, including the PTY suites.
- All 67 local Markdown links checked in the six archived changes, the
  design-question list and the brand reference resolve, including heading anchors.
- Thirty-four existing archived files are byte-identical to their active
  versions; the remaining two contain only the relative-link repairs above.
- No existing main-spec requirement was removed. The two superseded scenario
  titles were replaced by the explicit terminal/plain and single/grouped cases.
- Previously archived records, unselected active records and runtime assets are
  unchanged. `git diff --check` passed.
