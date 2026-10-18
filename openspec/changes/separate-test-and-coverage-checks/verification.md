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

## Pending

At the local-verification checkpoint, progress is **10/12 tasks complete**. Tasks
4.2–4.3 retain separate live-verification requirements. The maintainer has now
authorised committing and pushing this follow-up to `swift-otter` and PR #8.
Live check names must be observed after that push; independent Codecov pass/fail
outcomes need eligible reports and a processed base report. No merge, publication,
billing, membership, or required-check changes are authorised by this rollout.
