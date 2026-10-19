## Historical completion record (superseded design)

The earlier Codecov-only increment reached **10/12 tasks complete**. Its checked
work established separate normal tests and instrumented reporting, configured
named Codecov 80% targets, checked workflow/failure boundaries, ran Linux/macOS
suites and formatting/Clippy checks, documented that policy, and recorded the
specification synchronisation order (former tasks 1.1–3.4, 4.1 and 4.4).

Former tasks 4.2 and 4.3 remained incomplete: actual named Codecov assessment
labels and independent live project/patch pass/fail evidence were not verified.
Those service-policy tasks are superseded, not completed by the revised design.
See [historical verification](verification.md). Its earlier results do not
validate the Actions gates below. Active progress counts only the new checklist.

## 1. Revise the active contract and documentation

- [x] 1.1 Replace the active proposal/design/delta specifications with three Actions quality policies, exact inclusive arithmetic, immutable comparisons, complete run-bound report evidence, explicit N/A, and isolated main-only authenticated uploading; record the original-additions-before-follow-up-modifications dependency without synchronising or archiving.
- [x] 1.2 Revise README Development, the focused coverage reference and publication checklist for Actions-owned policy and report-only Codecov; preserve the actual shield and offline runtime behaviour, and review the wording for a first-time reader.

## 2. Implement maintained evidence and policy helpers

- [x] 2.1 Add report preparation/validation outside `openspec/`: validate raw LLVM syntax/required fields/summary pair bounds without false grouped-detail equality; emit canonical line-only LCOV preserving every unique measured line, with recomputed summaries, immutable-head complete source inventory/library/CLI execution, deliberate duplicate handling, valid paths and independently verifiable run/event/repository/revision/digest metadata. Consumers and uploader accept strict canonical evidence only.
- [x] 2.2 Add a total consumer that revalidates artifact/context and applies `5 * covered >= 4 * measured` with a positive denominator, useful counts and no display-rounding policy.
- [x] 2.3 Add a changed-line consumer using pinned diff-cover 10.6.0, explicit immutable merge-base-to-head diff, validated JSON integer counts/line sets and source mapping, exact arithmetic and valid PR/non-PR N/A semantics.
- [x] 2.4 Add stdlib regression fixtures/tests for exact/above/below 80% (including rounded-to-80), independent gate outcomes, valid N/A, missing/malformed data, raw LLVM grouped summaries/instantiations, missing/invalid summary pairs, canonical-only tamper rejection, JSON object shapes, Git-attribute binary suppression, invalid counts/duplicates, path/source omissions, stale identity/digest, unavailable refs, additions, renames and deletions; include real pinned diff-cover integration using temporary Git fixtures.

## 3. Wire independent workflow jobs and trust boundaries

- [x] 3.1 Keep `Tests` independent/read-only with stable Rust, no persisted checkout credentials, `cargo test --locked` and the explicit Python helper test command; retain events, bounded waits, cleanup and concurrency without path filters.
- [x] 3.2 Make `Coverage report` a read-only producer with no OIDC or percentage gate; measure immutable PR heads with required history, retain normalized LCOV and run-bound metadata once, and preserve existing cargo-llvm-cov 0.9.1 Linux/default scope without exclusions.
- [x] 3.3 Add sibling `Total coverage` and `Changed-line coverage` jobs depending only on valid production; independently validate artifact context and keep low-total, normal-test or service outcomes from suppressing changed-line assessment or report retention.
- [x] 3.4 Isolate uploading to a fresh main-push/manual-main job depending only on the producer, with job-only OIDC, no checkout/Cargo/repository scripts/caches/executable artifacts, explicitly named current-run data, inline strict validation and explicit pinned-uploader inputs/search disabled/integrity checks/plugins noop; forbid all PR uploads and privileged event bridges.
- [x] 3.5 Disable Codecov project and patch statuses while retaining main report browsing and the actual token-free shield; preserve all non-upload read-only permissions and leave repository settings, billing, visibility and required checks unchanged.

## 4. Validate the exact candidate

- [x] 4.1 Run Python helper regressions, real pinned diff-cover integration and the actual production report through preparation and consumers; verify raw-to-canonical preservation/counts, strict canonical rejection, exact thresholds, sibling outcomes, valid N/A, forced-text source diffs and all malformed/missing/path/source/ref/run/digest failure boundaries.
- [x] 4.2 Run Actionlint and Codecov YAML validation; inspect actual workflow dependencies, immutable checkout/fetch, permissions, artifact trust, data-only uploader and below-target retention/upload failure behaviour.
- [x] 4.3 Run formatting, Clippy, normal macOS/Linux and instrumented Linux suites with bounded terminal cleanup; verify unchanged production coverage scope and dynamically complete tracked source records.
- [x] 4.4 Reconcile documentation with the final helper CLI and workflow contract; reread from a first-time reader's perspective and record only actual validation evidence.
- [x] 4.5 Run strict OpenSpec validation and diff checks for the revised documentation/specifications; keep historical verification clearly separate and new technical verification truthful.
- [x] 4.6 Obtain independent review of the exact candidate's arithmetic, diff analysis, artifact/data trust and documentation; resolve findings and rerun affected checks before marking implementation complete.

## 5. Verify hosted rollout separately

- [ ] 5.1 On an authorised ordinary push to the existing PR, observe `Tests`, `Total coverage` and `Changed-line coverage`, plus retained report data and supporting pipeline jobs; verify the measured head/run and record real N/A only when justified.
- [ ] 5.2 Reconcile hosted results with controlled local pass/fail fixtures and independent gates; do not claim a documentation-only PR proves live positive-denominator changed-line threshold behaviour or introduce unrelated production changes for evidence.
- [ ] 5.3 After separately cleared publication/merge and an eligible main run, verify the isolated OIDC upload for the actual main revision, processed report, anonymous report access and the unchanged token-free main shield after cache refresh; leave this pending until those prerequisites occur.
