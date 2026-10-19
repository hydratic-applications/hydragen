# Actions-owned coverage gates verification

## Scope and status

This record covers the revised Actions-owned `Tests`, `Total coverage` and
`Changed-line coverage` design. The older [verification record](verification.md)
describes the superseded Codecov-only policy and does not establish the new
gates' outcomes.

The corrected local implementation and independent review are now complete.
Hosted execution, actual OIDC uploading, processed main reports and public access
remain unverified; local completion is not publication or merge clearance.

The documentation/specification work revises the active proposal, design and
requirements, including the original authenticated-upload requirement. README
Development and the focused coverage reference describe exact inclusive 80%
policy, independent consumers, immutable PR evidence, explicit N/A, retained
reports and isolated main-only uploading. The publication checklist expects local
Actions gates, separate main OIDC verification, anonymous access and the existing
token-free shield. No production behaviour or service-setting result is claimed
from these edits.

## Documentation review

- Read the revised text as a first-time contributor: distinguish the three
  quality policies from report/upload pipeline jobs; explain LCOV, measured
  executable lines, the merge base and OIDC before relying on those concepts.
- Keep the local LCOV command and scope unchanged. Document the separate Python
  helper test invocation and isolated real diff-cover dependency. Reconcile the
  helper's shared context flags, subcommands, exit codes and data files without
  adding an exhaustive CI tutorial to README Development.
- Require the original `configure-codecov` ADDED baseline to be synchronised
  before this follow-up's MODIFIED requirements when separately requested. No
  synchronisation, archival, repository setting or publication action is part of
  this documentation work.

## Validation evidence

- `openspec validate separate-test-and-coverage-checks --strict` passed for the
  revised proposal/design/delta contract.
- `git diff --check` passed. README changes are confined to Development; the
  header and actual main-specific shield URL/appearance are unchanged.
- The pinned official diff-cover 10.6.0 README was checked for LCOV support,
  `--diff-file`, JSON output and rounded display behaviour. This is documentation
  evidence, not execution of the policy adapter or integration tests.

## Fresh Rust test and measurement-scope verification

The following fresh runs validate the unchanged Rust test infrastructure and
coverage measurement scope, not the new Python helpers, percentage policy,
workflow trust boundaries, hosted checks or Codecov integration.

- `cargo fmt --check` and macOS `cargo clippy --all-targets -- -D warnings`
  passed. macOS `cargo test --locked` passed all **186 tests**, with none ignored
  or filtered out; its documentation-test phase completed with zero doctests.
  Logs are retained under ignored `target/actions-gates-macos/` as `clippy.log`
  and `normal-tests.log`.
- Fresh Linux normal and instrumented runs each passed all **186 tests**, with
  none ignored or filtered out. The normal run also completed its zero-doctest
  phase. Instrumentation used `cargo-llvm-cov` 0.9.1 with
  `--locked --workspace --all-targets`, Rust 1.99.1 and matching LLVM 23.1.1 tools
  in the exact image
  `rust@sha256:a24e292b4af7bdee351a31d3a6379972a5652f0ea22a1b6d6883324ac4f5097c`.
  This was local Linux aarch64 execution, not hosted x86-64 validation.
- Linux source was mounted read-only. The container used `--init` for child
  reaping, a 30-minute outer bound and cleanup limited to the exact owned
  container; terminal tests retained their bounded waits and child cleanup.
  Normal and instrumented execution completed in 99.3 seconds. The log is
  `target/actions-gates-linux/normal-and-instrumented.log`.
- Fresh `target/actions-gates-linux/lcov.info` contains nine production-source
  records matching every currently tracked Rust file under `src/`, checked
  dynamically rather than against a fixed expected file count. Summed raw LLVM
  grouped line-summary counts (`LF`/`LH`) are **2,527** measured and **2,413**
  covered (**95.49%**). These are LLVM's grouped summaries, not the final
  distinct-source-line policy counts. No production exclusions were added. These
  source/scope checks are not execution of the new report-preparation or gate helper.

## Initial helper and workflow contract reconciliation

Read `scripts/coverage_gate.py`, `.github/workflows/coverage.yml`,
`tests/coverage/test_coverage_gate.py` and `codecov.yml` to reconcile documentation
with the initial implementation. This subsection records source inspection before
corrective validation, not test execution or independent approval. Final local
validation is recorded below.

- Documented `prepare --report RAW --output NEW_DIR`, `total --artifact DIR`,
  `changed --artifact DIR` and its optional `--diff-cover` executable. All three
  require event/head/base/repository/run ID/run attempt; preparation creates a new
  directory, and non-PR base equals head. The intended exit contract is 0 for
  pass/explicit N/A, 1 for threshold failure and 2 for invalid evidence/input.
- Corrected the data description: exactly `lcov.info` and `manifest.json`, with
  run/revision/digest metadata and source content digests/file lengths in the
  manifest, and coverage counts validated from LCOV rather than duplicated
  aggregate manifest fields. Source completeness is not weakened.
- Matched the supporting upload name `Codecov main report`; the three quality
  names remain `Tests`, `Total coverage` and `Changed-line coverage`. Source
  inspection shows sibling consumers and main-only upload depend only on the
  producer, and Codecov project/patch statuses are disabled. Hosted outcomes
  remain unverified.
- Matched the combined `Tests` job's isolated pinned diff-cover dependency and
  real integration tests. Ordinary Rust test commands remain unchanged. Added
  focused-reference guidance for full history, allowed source paths, artifact
  size limits and explicit failures, without extending the README tutorial.
- Checked the pinned upstream [package metadata](https://github.com/Bachmann1234/diff_cover/blob/v10.6.0/pyproject.toml)
  and [configuration parser](https://github.com/Bachmann1234/diff_cover/blob/v10.6.0/diff_cover/config_parser.py):
  Python 3.10 needs the optional `toml` extra for the adapter's explicit TOML
  configuration; Python 3.11+ has a standard-library parser. The reference's
  isolated installation includes the extra. Initial workflow inspection found
  plain diff-cover installations. The corrective plan explicitly installs
  `'diff-cover[toml]==10.6.0'` in both workflow environments; compatibility
  verification of that correction was pending at this stage.
- Initial source inspection also found an error-contract edge case: non-object
  diff-cover JSON, or a non-object per-source statistics entry, can reach `.get`
  without a shape check and raise an uncaught `AttributeError`, rather than the
  intended diagnostic/exit 2. This required helper validation and regression
  verification; it was not a documented threshold failure.

The revised reference and specifications were reread from a first-time reader's
perspective. No new helper execution, workflow lint, hosted or main/service
verification is claimed by this reconciliation.

## Production-report compatibility finding and corrective design (resolved locally)

Exercising the initial producer against the actual fresh production report
failed before artifact creation: its raw summary/detail equality check rejected
valid LLVM output. All nine source records were present; this was not evidence
of malformed instrumentation or missing production coverage.

The pinned LLVM 23.1.1 [LCOV exporter](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageExporterLcov.cpp)
emits detailed `DA` records from merged source-file coverage, but `LF`/`LH` and
function summaries come from file function-group summaries. [Report construction](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageReport.cpp)
and [summary aggregation](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageSummaryInfo.h)
take maxima within instantiation groups and add groups that can overlap in source
lines. This is not simply repeated addition of every function instantiation.
Enumerated function details likewise need not equal grouped function totals.

For the same report, distinct validated source-line records contain **2,348
covered of 2,443 measured lines (96.11%)**, compared with the preserved raw grouped
summary **2,413/2,527 (95.49%)** above. Both describe the same nine files and test
execution. The difference is counting method, not improved tests, removed lines
or new exclusions. At this stage, the distinct-line counts identified the intended
policy input; corrected pipeline validation is recorded below.

The approved corrective design keeps strict raw syntax, required fields and
summary pairs, nonnegative counts, covered/found pair bounds, duplicates,
inventory, line coordinates and library/CLI execution checks. It removes only
unsupported raw grouped-summary/detail comparisons. The producer then emits
canonical line-only `SF`/`DA`/recomputed `LF`/`LH`/`end_of_record` data, preserving
every unique validated line and its execution count. Consumers and the uploader
must reject noncanonical fields and summaries inconsistent with those records.
The CLI, manifest schema and two-file artifact stay unchanged; no additional
LLVM JSON export or segment-to-line engine is needed.

Independent candidate review also found that `.gitattributes` can mark a source
as `-diff`, suppressing textual hunks and allowing false changed-line N/A. The
corrective comparison must force `--text` while retaining disabled external diff
and text-conversion filters, with an uncovered-source regression. Explicit
diff-cover JSON root/per-source object checks and Python 3.10 TOML parser support
are part of the same correction. These findings requested implementation
revisions; the findings and design alone did not establish successful fixes.

Corrected-helper regressions, actual-production preservation and consumer/uploader
checks, forced-text diff validation and renewed approval were required after
these findings. Their completion is recorded next. The compatibility analysis
alone did not establish hosted checks, main uploading or service outcomes.

## Corrected local validation and independent approval

- The orchestrator and independent reviewer each reran all **39 Python tests**
  successfully, including the real pinned diff-cover CLI, raw/canonical report
  cases, exact thresholds, source/diff/context failures and the actual inline
  workflow upload validator. Local execution used **Python 3.14.6**; Python 3.10
  runtime compatibility was not executed. Both workflow installations now request
  `'diff-cover[toml]==10.6.0'`, supplying its required configuration parser.
- Each independently processed the fresh nine-source production report. Raw
  validation is used only for preparation, followed by strict canonical
  revalidation. Every source, line coordinate and execution count was preserved:
  **2,443 measured lines, 2,348 covered (96.11%)**. No source or measured line was
  removed. Canonical output contains only `SF`, `DA`, computed `LF`/`LH` and
  `end_of_record`; consumers and the uploader reject raw/non-line fields or
  inconsistent canonical summaries.
- `prepare`, `total` and `changed` passed local production-data exercises for PR,
  push and manual event contexts. Total passed at **2,348/2,443**; changed-line
  results were valid explicit N/A for the actual comparison and non-PR event
  policies. The extracted inline validator accepted push/manual artifacts against
  independently supplied expectations. These are local fixtures, not hosted
  events, OIDC issuance or actual uploads.
- The real CLI regression with a tracked `-diff` source attribute now reports
  **FAIL, 0/1 covered, exit 1**, not N/A, because comparison forces `--text` while
  disabling external diff and text conversion. Malformed JSON root/per-source
  object shapes now produce actionable validation errors with **exit 2**.
- The raw production report remained unchanged, with SHA-256
  `9b99a25135c0380cd81f386f35e70249465d8049795da526878884d099f3e998`.
  Its grouped **2,413/2,527 (95.49%)** summary remains preserved above and is not
  substituted for canonical policy counts.
- Actionlint passed, the Codecov configuration API returned `Valid!`, and strict
  OpenSpec validation and diff checks passed. Workflow dependency, permission,
  immutable identity, artifact-data and failure-boundary checks were reviewed;
  actual service authentication/upload failure injection was not performed.
- The independent reviewer approved the corrected local candidate with no
  material correctness or security findings remaining within the accepted scope.
  Review covered arithmetic, diff analysis, source completeness, canonical data,
  main-only OIDC isolation and documentation. This approval does not establish
  hosted results, main publishing, anonymous access or merge readiness.

Final documentation reconciliation rechecked the actual corrected helper and
workflow: subcommands, shared context options, exit contract, manifest keys and
two-file artifact remain as documented. No reference/design mismatch required
additional changes. The revised evidence was reread for first-time-reader clarity;
the existing Rust-suite/scope results above are not evidence for a different
source revision or a later main-branch run.

## Active progress and pending evidence

Active progress is **17/20 tasks complete**. Implementation, local validation,
documentation reconciliation and independent local review (sections 1–4) are
complete. Only **5.1–5.3** remain pending: observe the actual revised PR checks and
retained report, reconcile hosted results against controlled fixtures, then
verify an eligible main OIDC upload, processed report, anonymous access and the
unchanged token-free shield when their separate prerequisites are met. No hosted
execution or actual upload has occurred for this revised implementation. Local
results and earlier successful uploads do not establish those remaining outcomes.
