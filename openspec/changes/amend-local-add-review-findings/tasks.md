## 1. Accepted baseline and review contract

- [x] 1.1 Record accepted R1 integration: cancellation commit c3b3c8a, ordinary merge 3ea1fcb with inspected main 6ccf96b, independent review and 544 Rust/39 Python baseline; distinguish the unresolved reporting-test transient from completed integration.
- [x] 1.2 Complete focused proposal/design/deltas/dispositions and validate this change strictly before implementation lanes start; retain R14's no-refactor and R15's qualified remedy.

## 2. YAML safety, layout and source oracles

- [x] 2.1 Add failing-first source-sensitive EOF regressions for both YAML extensions: missing fields, existing dependency tails, nested mappings/sequences, terminal explicit scalar keys with implicit null values, chomping modes, safe controls, excess indentation, all-space/empty keep bodies, header comments/properties, absent indentation hints and relevant line endings.
- [x] 2.2 Implement the bounded scanner-tail source-facts guard; retain demonstrated-safe acceptance, original bytes and no staging on refusal/no-op, shared schema and intended decoded-change verification; pause on ambiguous/disagreeing facts rather than broaden rejection.
- [x] 2.3 Add failing-first exact multiline flow layout regressions for own-line entries/close, footer/inline/more-indented comments, later commas, empty lists, root mappings, batches, CRLF and final unrelated flow values.
- [x] 2.4 Separate closing-boundary discovery from entry planning and implement bounded comma/item insertions without reformatting compact collections or moving missing-field boundaries.
- [x] 2.5 Strengthen exact-output/permitted-insertion oracles and negative comment/literal/unrelated-field corruption controls across affected YAML, JSONC and TOML cases; retain one-insertion proofs only where valid.
- [x] 2.6 Obtain independent YAML phase review and record failing-first/targeted results and any acceptance limitations before integration advances.

## 3. Accurate shared diagnostics

- [x] 3.1 Emit literally empty block separators, remove five masking normalisers and tighten unnormalised family assertions without trimming nonempty data.
- [x] 3.2 Correct non-regular-entry and non-string metadata causes, retaining paths/no fallback, current/dependency symlink distinctions, conditional optional omission advice and the public NotString variant shape.
- [x] 3.3 Review captured and scheduled bounded actual-terminal diagnostic families at normal/narrow widths, including plain/styled structure, escaping, streams/status and mutation state; obtain independent phase review.

## 4. Reliable environment-sensitive fixtures

- [x] 4.1 Pin copied-help-example width, retain deliberate width tests and rerun with inherited COLUMNS=30.
- [x] 4.2 Probe actual read/create denial for all three affected Unix fixtures before state snapshots; update all five consumers for ready/unsupported-with-reason/setup-error, restore permissions/remove probe artifacts on every path and retain unaffected/Windows coverage.
- [x] 4.3 Capture visible --show-output unsupported evidence and real unprivileged denial results; keep unexpected errors/success after verified setup fatal and make no unexecuted root-runtime claim.
- [x] 4.4 Establish deterministic bounded closed-stdout and broken-pipe setup where supported, investigate the initial baseline transient and use portable ErrorKind simulation; retain fatal/stateful returned completion errors and unchanged best-effort warning/error delivery.
- [x] 4.5 Obtain independent fixture/reporting phase review with targeted evidence; keep PTY-heavy execution scheduled sequentially.

## 5. Focused current documentation and dispositions

- [x] 5.1 Correct only the README's actual generated YAML walkthrough, concise existing colour controls, completion-reporting limitation and hard-link/open-handle safety explanation.
- [x] 5.2 Add concrete Unreleased name/Rust source compatibility and integrated bare-version notes without an API stability, release or publication promise.
- [x] 5.3 Align docs/cli-diagnostics.md and replacement module documentation with literal separators, established fallible completion handling, existing best-effort diagnostic delivery and atomic path-replacement consequences.
- [x] 5.4 Link the bounded preservation/reporting rationale from the relevant design-question refinements; retain unrelated open questions, archived changes and main specs unchanged unless the parent explicitly invokes approved alignment.
- [x] 5.5 Complete first-time-reader documentation review, foundations/glossary relevance assessment and concrete R1–R15 file/evidence dispositions; obtain independent documentation review.

## 6. Final validation and approved delivery

- [x] 6.1 Run full locked Rust tests, formatting, all-target/all-feature warning-denied clippy and incoming Python coverage-helper tests; preserve coverage gates and distinguish inherited/integration/amendment failures.
- [x] 6.2 Validate current specs and this change strictly; check local links/whitespace and complete scheduled captured/bounded terminal family review with child cleanup.
- [x] 6.3 Verify source packaging and fresh offline build/tests without OpenSpec or old build outputs; report only executed platform/runtime/coverage evidence.
- [x] 6.4 Obtain independent final review, resolve confirmed findings and finish measured dispositions/remaining limitations without claiming the separate input-reader race is fixed.
- [x] 6.5 Parent completes approved focused commits, ordinary push and concise PR #7 description refresh; do not merge the PR, autoarchive, publish a release or install globally.

## 7. Authorised current-main integration after amendment delivery

- [x] 7.1 Integrate inspected main 4a82827 by ordinary merge, retaining branch-required YAML comment support/runtime dependencies and incoming dependency/schema/coverage records; review the merge against both parents.
- [x] 7.2 Renew schema fingerprint, validation/classification and offline-boundary checks, full locked Rust/Python validation, formatting/clippy/strict specs, packaging and fresh offline independence against the merged tree.
- [x] 7.3 Obtain independent merged-result/final evidence review; update the current integration boundary and limitations without rewriting prior measured observations.
- [ ] 7.4 Commit, ordinary-push and refresh PR #7, verify matching remote head and actual mergeability/check state; keep the PR open and amendment unarchived.
