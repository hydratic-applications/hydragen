# Independent CI checks verification

## Implementation status — 2026-10-19

- Added independent Actions jobs `Tests` and `Coverage report`. The test job has
  read-only permissions and no Codecov/OIDC dependency. The report job retains
  existing instrumentation, validation, upload safeguards, and fork skipping.
- Configured named Codecov assessments `total-coverage` and `changed-lines`, each
  with an 80% target, zero tolerance, and non-informational results. Disabled the
  default entries; no local percentage gate was added.
- Updated README Development and `docs/coverage.md` to distinguish normal tests,
  report-pipeline health, and Codecov-only percentage decisions. Reviewed the
  revised text for a first-time reader, retaining setup, scope, fork/private
  limitations, baseline guidance, and token-free shield behaviour. No product
  decision, foundation change, or new glossary term was needed.
- `actionlint .github/workflows/coverage.yml` passed. Codecov's configuration
  validator returned `Valid!` and confirmed both named 80% targets and disabled
  defaults. `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` also passed.

## Initial test failure and focused repair

- Ran the normal `cargo test --locked` suite in a disposable Linux container,
  using Rust 1.99.1 on `aarch64-unknown-linux-gnu` and the same pinned Bookworm
  image as the previous increment. Source was mounted read-only; artifacts are
  under ignored `target/separate-checks-linux/`. Execution had a 30-minute outer
  timeout with container cleanup and child reaping.
- The normal run failed
  `losing_terminal_input_reports_failure_without_creating_a_layer_manifest`:
  `tests/interactive_init.rs:1656` asserted `!session.finish().success()`.
  The terminal-test binary reported 24 passes and one failure. Earlier binaries
  passed; Cargo stopped before later binaries and the subsequent instrumented run.
- This is the same existing test observed failing intermittently in the original
  Codecov change's macOS validation. At the initial Linux failure, no Rust source
  or test had been changed or skipped. The new job separation did not establish
  the cause of the failure.
- The maintainer authorised investigation and a focused fix. Inspection of the
  installed `portable-pty` 0.9.0 source found that `UnixMasterWriter::drop` sends
  a newline followed by EOT before closing its descriptor. The test used that
  writer to simulate disconnection: it unintentionally submitted input first.
- Before the repair, 30 isolated macOS retries passed, but Linux reproduction
  failed on attempt 17 with a timeout. Its transcript showed the untouched Name
  prompt being submitted and Author opening during input loss. This supports the
  injected-newline explanation and preserves evidence of the original failure.
- Replaced only this test's writer with a plain owned duplicate of the input
  terminal's master descriptor. It sends no keystrokes on drop. The existing
  failure/no-files-created assertions remain at both the name prompt and format
  selector; the exit-status assertion now includes captured diagnostics. No
  production code, Cargo dependency, coverage exclusion, test skip, or retry
  mechanism was added to CI. See the focused repair decision in `design.md`.

## Configuration and failure-boundary checks

- Actionlint passed after the job split; Codecov's hosted YAML validator again
  returned `Valid!`, with only `total-coverage` and `changed-lines` enabled at
  80%, zero tolerance, and non-informational results.
- Parsed the actual workflow and Codecov YAML using local PyYAML, verifying
  event coverage, no path filters or job dependencies, explicit display names,
  bounded execution, stable Rust, pinned actions, no persisted checkout
  credentials, test-only read permission, and report-only OIDC scope.
- Confirmed the normal-test command is exactly `cargo test --locked`, with no
  uploader or percentage check. Executed that shell command with a temporary
  fake `cargo` returning 101 and verified the same failing exit code propagates;
  no application source was intentionally broken for this check.
- Executed the actual LCOV-validation shell step against temporary fixtures:
  a valid report below 80% with library/CLI hits passed unchanged; missing,
  empty, malformed-count, missing-CLI, and unexecuted-library reports failed.
  Fixture data was not uploaded and no local percentage gate was introduced.
- Reviewed Actions step ordering and default success conditions: generation and
  validation must pass before upload; failed/cancelled work cannot force a later
  upload through `always()` or `continue-on-error`. Upload errors are fatal via
  `fail_ci_if_error: true`; authentication/search safeguards remain unchanged.
  This is configuration/fixture evidence, not a live forced upload-error test.
- Same-repository PRs, main pushes, and manual runs retain the eligible upload
  route; fork PRs retain the mutually exclusive skip summary. No local fallback,
  privileged PR event, custom status wrapper, or duplicated percentage check was
  added. Documentation explicitly leaves unavailable service/base assessments
  unverified rather than treating report-pipeline success as a percentage pass.

## Specification synchronisation order

`configure-codecov` introduces `code-coverage-ci`, which is not yet in the main
specifications. Synchronise its additions first, then this change's modifications
and `test-ci` addition. The named fixed 80% targets supersede the original
automatic-target policy. Original verification remains historical; no actual
specification synchronisation or archival was performed.

## Corrected test and coverage results

- The repaired input-loss test passed **100 consecutive runs on Linux and 100
  on macOS**, exercising both the name prompt and format selector in each run.
  These repetition loops were investigation-only, not retries added to CI.
- On Linux, `cargo test --locked` and
  `cargo llvm-cov --locked --workspace --all-targets --lcov --output-path /artifacts/lcov.info`
  each passed all **186 tests**, with none ignored or filtered out. The normal
  run also completed the documentation-test phase (currently zero doctests).
- A final full `cargo test --locked` run on macOS also passed all 186 tests,
  with none ignored or filtered out, and completed the documentation-test phase.
- Linux used Rust 1.99.1, matching LLVM tools (LLVM 23.1.1), and
  cargo-llvm-cov 0.9.1 in the pinned image
  `rust@sha256:a24e292b4af7bdee351a31d3a6379972a5652f0ea22a1b6d6883324ac4f5097c`.
  The local image is `aarch64-unknown-linux-gnu`, not a claim that the hosted
  `ubuntu-latest` x86-64 rollout has already been observed. Source was mounted
  read-only, with a 30-minute bound, container init/child reaping, and timeout
  cleanup. Terminal tests retain their own bounded waits and cleanup.
- The fresh LCOV report at ignored `target/separate-checks-linux/lcov.info`
  contains nine production-source records, including executed `src/lib.rs`,
  `src/main.rs`, and layer manifest modules. It records 2,413/2,527 measured lines
  hit (95.49%). This is local LLVM evidence, not a Codecov assessment, a new
  percentage gate, or a replacement for the original hosted 96.11% observation.
  No new production exclusions were added.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, Actionlint,
  `git diff --check`, and strict OpenSpec validation passed after the repair.

## First hosted split-job run and Unicode correction repair

- Committed and pushed the split-job implementation as `6626103` on
  `swift-otter`, updating PR #8. Hosted run
  [39177774792](https://github.com/hydratic-applications/hydragen/actions/runs/39177774792)
  displayed the expected Actions names: `Tests` passed, but `Coverage report`
  failed in `unicode_schema_answers_reject_next_line_whitespace_and_accept_bom_without_normalizing`
  at the then-current `tests/interactive_init.rs:1152`. The name retained U+0085
  before U+FEFF instead of containing U+FEFF alone. The input-loss test passed.
- GitHub's job-step data confirmed report validation and Codecov uploading were
  skipped after that instrumented failure. The commit's check-runs contained
  only the two Actions jobs and its status list was empty: no new Codecov
  assessment was published for that commit. This was not an 80% threshold failure.
- Codecov's PR page displayed its onboarding message: comparisons become
  available after merging coverage to the default branch. That is separate from
  the failed upload pipeline. The page's older `CI Passed` label is not evidence
  that the latest GitHub run passed. No settings or account changes were made.
- Inspected the installed prompt dependencies: `console` restores the previous
  terminal mode after each key read, before cliclack draws/validates the next
  prompt frame. A scripted Backspace during a canonical-input gap can be
  consumed by the terminal, leaving the application's prior input intact.
- A controlled pseudo-terminal experiment reproduced byte loss: after reading
  U+0085 in raw mode, restoring the default canonical mode and sending
  Backspace/U+FEFF/Enter delivered only U+FEFF/newline. With a noncanonical
  baseline the Backspace was preserved. The original full test also passed 200
  isolated Linux retries; repetition alone did not reproduce the hosted timing.
- Set `stty -icanon -echo -isig min 1 time 0` in the test session's existing
  bounded shell child before executing the CLI. Setup failure exits nonzero.
  Output newline handling and all Unicode expectations remain unchanged. There
  are no added sleeps, retries, skips, production changes, or Cargo dependencies.
- With that setup, the unchanged Unicode correction test passed 200 consecutive
  normal Linux runs, 200 instrumented Linux runs, and 100 macOS runs. Normal
  Linux/macOS and instrumented Linux full suites each passed all 186 tests,
  with none ignored or filtered out. Normal documentation-test phases completed
  with zero doctests. Linux used the same pinned aarch64 image and matching tools
  as above, with read-only source and a 30-minute container/cleanup bound.
- Fresh `target/separate-checks-linux/unicode-fixed-lcov.info` retained executed
  library/CLI source records and 2,413/2,527 measured lines hit (95.49%), without
  new exclusions. Formatting, Clippy, diff checks, and strict OpenSpec validation
  passed. Updated `docs/coverage.md` to explain why the two service assessments
  may be absent when report production/uploading fails; reviewed that explanation
  for a first-time reader.
- The maintainer authorised committing/pushing this focused repair and observing
  hosted CI and Codecov feedback. These local results alone did not establish
  the repair's hosted outcome. Configuration and trust boundaries remain unchanged.
- Pushed the repair as `a6bd453`; hosted run
  [39178427489](https://github.com/hydratic-applications/hydragen/actions/runs/39178427489)
  was started for that commit. The working tree was clean immediately after
  the push, before adding these subsequent rollout observations.
- Read-only inspection of Codecov's
  [configuration manager](https://app.codecov.io/gh/hydratic-applications/hydragen/config)
  listed `Coverage reports` as configured, but `Project coverage` under
  `Available with Pro Plan`. Its YAML settings showed default branch `main`.
  [Codecov's pricing page](https://about.codecov.io/pricing/) also distinguishes
  patch coverage/status access from project coverage. This is an account-feature
  warning to verify, not proof that uploading is blocked or that a paid plan
  must be purchased. No trial, purchase, or configuration change was made.
  Browsable aggregate coverage from earlier uploads does not establish that the
  named overall-coverage status can be published on this account. Documentation
  now makes that distinction explicit.

## Hosted repair outcome

- Run [39178427489](https://github.com/hydratic-applications/hydragen/actions/runs/39178427489)
  passed on `a6bd453`: `Tests` completed in 1 minute 31 seconds and
  `Coverage report` in 1 minute 26 seconds. Both previously failing terminal
  tests passed under instrumentation. All 186 instrumented tests passed;
  library/CLI report validation and OIDC uploading also succeeded.
- The uploader used head SHA `a6bd4532c8242e092a16f0de35984421545abe9c`, found the
  intended LCOV file, verified CLI integrity, and reported successful storage
  upload and queueing. The Codecov commit page subsequently displayed one merged
  `github-actions` upload linked to the coverage job, confirming report processing
  rather than merely upload acceptance.
- The [processed commit's file explorer](https://app.codecov.io/gh/hydratic-applications/hydragen/commit/a6bd4532c8242e092a16f0de35984421545abe9c/tree)
  showed 2,348/2,443 tracked lines covered (96.11%). That aggregate report is
  available without any account or billing change. It is not evidence that the
  overall 80% status was published or enforced.
- Opened `view YAML file` on that commit. Codecov's effective YAML contained
  `patch.changed-lines` and `project.total-coverage`, both with target 80.0,
  threshold 0.0, and `informational: false`, and both `default: false` entries.
  The new configuration is therefore accepted for the uploaded commit, not
  just syntactically valid at the standalone validator.
- GitHub still showed only the two successful Actions checks after processing;
  the commit-status API returned an empty `statuses` list. No live named Codecov
  assessment or extra default assessment was observed. Codecov's commit page
  showed no changed covered files, while its PR page retained the first-upload
  onboarding/missing-default-branch comparison limitation. The earlier account
  feature warning remains unresolved; do not claim that a main merge alone
  guarantees all four checks, or that project reporting definitely requires a
  purchase based only on that warning.
- No merge, trial, purchase, visibility change, alternate local gate, fabricated
  success status, or required-check change was used to conceal the remaining
  service-verification gap. The terminal-test/report-pipeline failure is fixed;
  baseline establishment and actual Codecov status availability are follow-up
  prerequisites, not source failures in this successful run.

## Pending

Progress remains **10/12 tasks complete**. The Actions display names have been
observed and Codecov accepted/processed the named configuration, but task 4.2
still needs its actual rendered assessment names. Task 4.3 needs independent
Codecov pass/fail outcomes with eligible reports, a processed base report, and
confirmed account feature availability. No merge, publication, billing,
membership, or required-check changes are authorised by this rollout.
