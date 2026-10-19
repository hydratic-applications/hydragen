# Actions-owned coverage gates verification

## Scope and status

This record covers the revised Actions-owned `Tests`, `Total coverage` and
`Changed-line coverage` design. The older [verification record](verification.md)
describes the superseded Codecov-only policy and does not establish the new
gates' outcomes.

The coverage implementation has local and hosted PR policy evidence. Diagnosis
of the retained initial terminal-test timeout confirmed a matching Linux input
selection race. The test-only synchronisation increment has completed local
validation, independent review and fresh hosted verification of the reviewed commit.
Production fallback before input selection remains a separate unfixed runtime
gap. Main OIDC uploading, processed main reports and anonymous access remain
unverified; PR #8 remains open and the repository private at this review.

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

## Initial fresh Rust test and measurement-scope verification

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

## Corrected local validation and independent approval (before incorporating main)

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

## Hosted PR rollout and unresolved terminal-test reliability

### Initial hosted failure retained

The first ordinary push, `7c6d65b`, triggered
[run 39193711392](https://github.com/hydratic-applications/hydragen/actions/runs/39193711392).
`Tests` failed because
`losing_terminal_input_reports_failure_without_creating_a_layer_manifest` timed
out waiting for child exit at the Name prompt in `Session::finish`, which has a
10-second limit. Independently, the report producer succeeded, total coverage
passed at **96.11%**, and changed-line coverage reported valid N/A. The main-only
uploader correctly skipped the PR event. This failure is retained as evidence;
it was not replaced by a successful retry of the same revision.

At this stage the intermittent terminal-test failure was unresolved and read-only
diagnosis was pending. The later green run on a different revision did not prove
the timeout fixed. The matching mechanism and subsequent test-only increment are
recorded below; completing policy rollout does not close their validation or the
separate production fallback issue.

### PR head after incorporating main (before the harness increment)

Main revision `c99e24e497772d5da78088ee2f65557534f71374` was merged into the PR
branch, producing head `9e3cc759d6bd9024194c1b50d8084a2b98f29487`. This incorporated
both source contributions without a conflict; it did **not** merge PR #8 into
main. The combined candidate received independent review. Fresh local formatting,
Clippy, macOS normal tests, Linux normal/instrumented tests and all **39 Python
tests** passed. Each Rust suite passed **187 tests**; the fresh canonical report
contained the same nine production sources and **2,374/2,469 covered lines
(96.15%)**. These results belong to the combined revision, not the earlier
186-test source snapshot recorded above, and do not explain the initial timeout.

The separate [run 39194042983](https://github.com/hydratic-applications/hydragen/actions/runs/39194042983)
on this head, attempt **1**, succeeded:

- [Tests](https://github.com/hydratic-applications/hydragen/actions/runs/39194042983/job/117872802591)
  passed all **187 Rust tests**, with none ignored or filtered out, completed the
  zero-doctest phase, and passed all **39 Python tests**.
- [Coverage report](https://github.com/hydratic-applications/hydragen/actions/runs/39194042983/job/117872802391)
  succeeded and retained the canonical report and run-bound manifest.
- [Total coverage](https://github.com/hydratic-applications/hydragen/actions/runs/39194042983/job/117873143610)
  passed at **2,374/2,469 (96.15%)**.
- [Changed-line coverage](https://github.com/hydratic-applications/hydragen/actions/runs/39194042983/job/117873143549)
  succeeded with explicit **N/A**, after validating the immutable base/head
  comparison and finding no applicable measured head lines.
- `Codecov main report` was correctly **skipped** because this was a PR event,
  not an eligible main push or manual-main run.

### Retained evidence and policy reconciliation

The named artifact
`coverage-39194042983-1-9e3cc759d6bd9024194c1b50d8084a2b98f29487` was downloaded and
rechecked with the actual `total` and `changed` consumers. Independently supplied
expectations were repository `hydratic-applications/hydragen`, run
`39194042983`, attempt `1`, event `pull_request`, head `9e3cc759d6bd9024194c1b50d8084a2b98f29487`
and base `c99e24e497772d5da78088ee2f65557534f71374`. Identity, report digest,
source hashes, counts and comparison validation passed; the repeated results
were total **2,374/2,469 PASS** and changed-line **valid N/A**.

The hosted results match the independent-consumer design and controlled local
fixtures for exact 80%, below-target failure, opposite total/changed outcomes and
valid N/A. Neither hosted PR run demonstrates a positive-denominator changed-line
threshold pass or failure: those arithmetic/failure boundaries are supported by
the controlled local tests, not by interpreting N/A as 100% coverage. No OIDC
issuance, actual Codecov upload, processed main report or anonymous-access proof
is established by these PR runs or artifact checks.

## Terminal input-selection harness increment

### Diagnosis and scope

A controlled Linux reproduction confirmed a mechanism matching the retained
timeout. With locked cliclack 0.5.6 and console 0.16.6, a Name frame can be visible
before console selects an input terminal. The test's standard input (stdin) is
redirected to a dedicated pseudo-terminal (PTY), while a separate controlling
terminal stays alive for output. Disconnecting the input master before console
checks whether stdin is a terminal (`isatty(stdin)`) can make
that check fail on Linux; console then opens `/dev/tty` and keeps waiting on the
surviving terminal. The original hosted transcript cannot establish this as its
only possible cause. macOS hangup behaviour differs, so a successful macOS batch
alone cannot establish Linux reliability.

The implemented change is confined to `tests/interactive_init.rs`. It verifies a
canonical, line-buffered baseline for the dedicated input PTY before spawn, waits
for the Name or format-selector frame, then waits for that PTY's observed
noncanonical transition before closing every input master. Supported
`MasterPty::get_termios` observation and the named `ICANON` flag avoid platform
bit constants and new Cargo dependencies. This establishes stdin selection, not
proof of a blocked read system call or a fully raw terminal configuration.

Readiness checks keep the existing timeout and output-event draining, explicitly
failing for unsupported termios, early child exit or deadline expiry. Two helper
tests cover those decisions. `Session::finish` adds child-status and reader-count
timeout diagnostics only; its timeout, exit condition and cleanup remain unchanged.
Both injection cases retain their nonzero exit, single styled error, empty stdout,
no cancellation/success wording and no-created-files/layer-manifest assertions.
No readiness keystrokes, arbitrary sleeps, retries or skipped tests were added.

Production input loss **before** stdin selection can still trigger the fallback
to the controlling terminal. That runtime gap is not fixed by this harness-only
increment and remains an open follow-up under the existing interaction policy.
There are no production Rust, Cargo dependency, workflow or coverage-policy changes.

### Confirmed local validation and independent review

- The actual test-source diff was reviewed for the documented scope and unchanged
  Name/selector assertions. Initial source inspection established task 6.1;
  completed validation and independent review now establish task 6.2.
- macOS Clippy passed. The full macOS normal suite passed all **189 Rust tests**,
  with none ignored or filtered out, and completed its zero-doctest phase. The
  increase from 187 is the two test-only readiness helpers, not production code.
  Results were corroborated from the maintained local validation logs.
- Full normal Linux and instrumented Linux suites each passed all **189 Rust
  tests**, with no failures, ignored tests or filtered tests. The normal run also
  completed its zero-doctest phase. Both new readiness-helper diagnostic tests ran
  in the full suites. These results were corroborated from the Linux validation log.
- The three fixed stress batches—macOS, normal Linux and instrumented Linux—each
  passed **100 repetitions**, exercising both injection cases per repetition,
  with **zero failures**. All **300 repetitions / 600 case executions** passed in
  their first fixed batches; there was no retry-until-green procedure or CI retry
  policy. The driver stops on any nonzero result or timeout.
- The independent reviewer read all 300 distinct repetition logs, matched the
  normal/instrumented stress executable paths to the full-suite binaries and
  checked **300 nonempty LLVM profile files** from instrumented stress execution.
  Review confirmed the existing error/style/stdout/no-cancellation/no-files
  assertions were unchanged in both injection cases.
- Formatting and the existing **39 Python tests** passed in the orchestrator's
  checks. Fresh coverage retained all nine production sources at canonical
  **2,374/2,469 covered lines (96.15%)**. Actual report parsing, `prepare`, `total`,
  `changed` and the inline uploader's local data fixture passed. This is local
  validation, not OIDC issuance or a service upload; no source or coverage-policy
  change is part of the harness increment.
- Independent review approved the exact harness source and completed evidence
  for commit and ordinary-push validation, with no material evidence gaps. At
  that stage the correction was uncommitted on HEAD `9e3cc75` and had not been
  pushed. Its earlier green run did not contain or validate the correction;
  the required fresh hosted verification is recorded below.

These results validate the test-harness correction locally, not a production
runtime fix. Input loss before stdin selection can still enter the `/dev/tty`
fallback and remains a separate unfixed follow-up. Independent approval alone does not
establish hosted success, main uploading, publication readiness or merge clearance.

The design and `test-ci` requirements now record this test-setup distinction
without broadening the production contract. Revised text was reviewed from a
first-time reader's perspective. Documentation/specification validation remains
separate from actual hosted verification and the open production runtime gap.

### Fresh hosted verification of the harness correction

Reviewed commit `4d44910396dd5b6538b32b259f5137b3a680a2d1` was pushed normally.
[Run 39195433082](https://github.com/hydratic-applications/hydragen/actions/runs/39195433082),
attempt **1**, succeeded on that exact head against immutable base
`c99e24e497772d5da78088ee2f65557534f71374`:

- [Tests](https://github.com/hydratic-applications/hydragen/actions/runs/39195433082/job/117877075523)
  passed **189 Rust tests**, none ignored or filtered out, the zero-doctest phase
  and all **39 Python tests**.
- [Coverage report](https://github.com/hydratic-applications/hydragen/actions/runs/39195433082/job/117877074725)
  passed its instrumented suite and retained valid canonical report data.
- [Total coverage](https://github.com/hydratic-applications/hydragen/actions/runs/39195433082/job/117877496107)
  passed at **2,374/2,469 (96.15%)**;
  [Changed-line coverage](https://github.com/hydratic-applications/hydragen/actions/runs/39195433082/job/117877496186)
  succeeded with explicit valid **N/A** for that immutable comparison.
- [Codecov main report](https://github.com/hydratic-applications/hydragen/actions/runs/39195433082/job/117877497211)
  was correctly skipped for the PR event.

The retained artifact
`coverage-39195433082-1-4d44910396dd5b6538b32b259f5137b3a680a2d1` was downloaded and
rechecked with the actual `total` and `changed` consumers at the matching checkout.
Independently expected repository, run/attempt, head and base matched; source
hashes, report digest, canonical counts and valid N/A were revalidated.

This completes local and hosted validation of the scoped test-harness correction,
not proof of every future timing interleaving. The initial failure remains
recorded, and the production pre-selection `/dev/tty` fallback is still unfixed.
Changed-line N/A is not a numerical threshold demonstration. No actual main OIDC
upload, processed main report or anonymous-access/shield proof follows from this
PR run; PR #8 remains unmerged and the repository private at this review.

## Active progress and pending evidence

The coverage-policy rollout reached **19/20 tasks complete** before adding the
three focused harness tasks. Tasks 5.1 and 5.2 retain their actual hosted check,
artifact and local-fixture reconciliation evidence; that earlier green revision
does not prove this later harness change works.

Active progress is now **22/23 tasks complete**. Task **6.3** is complete on the
fresh hosted run and retained-artifact checks above. Only **5.3** remains pending:
eligible main OIDC uploading, processed report, anonymous access and token-free
main shield verification after separate publication-readiness and merge
prerequisites. PR #8 has not been merged into main, the repository remains
private, and publication readiness is not established.
The production pre-selection fallback remains a separate open runtime issue,
not an implicitly completed task or an exception to the interaction policy.
