# Unavailable-input coverage verification

## Status and scope

Implementation and local validation are complete. Full final-source macOS and
Linux stable suites passed. Instrumented Linux and Linux minimum-version checks
passed before a test-name-only clarification, whose exact equivalence was
verified. Independent code and evidence review found no remaining blocker and
closed both initial findings. Remote PR checks remain separate from this local
evidence.

The change adds no-controlling-terminal test coverage and a connected-input
control. It does not fix the separate production case where console selects a
surviving controlling terminal after stdin disconnects. Frame-triggered closure
is not guaranteed before-selection injection.

The existing selected-input regression, including current main's width and
colour cases, remains part of the required validation. Its noncanonical-input
observation establishes selection, not a blocked read syscall.

## Baseline refresh

The investigation started at `4a8282776c605df51be3040647da7243f1d4da59`. Before
implementation continued, main had advanced by 34 commits, including changes to
the terminal harness, diagnostics and minimum supported Rust version.

With user approval, this isolated branch was fast-forwarded to
`bc8418ab8c1cca0ac27c9e2e21221411ed0ff086`, then this session's changes were
reapplied. The Cargo conflict was resolved by preserving main's package settings
and production dependencies and adding only the planned Unix development
dependency. Other branches and worktrees were not modified.

`cargo metadata --locked --no-deps --format-version 1` succeeded after the refresh.
It identified Rust 1.89.0 as the minimum and `libc` as a development dependency
restricted to `cfg(unix)`. This metadata check is not a build or test result.

## Validation scope

The checks below record commands, platform/toolchain identities, source identity,
exit statuses, test counts, cleanup observations and retained log paths for:

- Formatting and locked all-targets Clippy with warnings denied.
- Full locked macOS and Linux test suites, including the new launcher control,
  loss cases and exceptional cleanup checks.
- Instrumented Linux execution using the existing coverage command and policy.
- Minimum-compiler compatibility under current main's Rust 1.89.0 policy.
- Independent code and evidence review, followed by affected revalidation.

Earlier scratch experiments and the old 190-test macOS baseline are historical
design evidence only. They must not be substituted for these maintained-suite
results on the refreshed source.

## Frozen candidate and focused checks

Validation used an uncommitted candidate based on `bc8418a`. Its source identity is retained
in `target/unavailable-input-validation/frozen-candidate.json`, including:

- `tests/interactive_init.rs`: SHA-256
  `d8080c8c60a3645d3daa0ec59c2a793e612bd3ad66dffd04ba63ff83bc8ef52e`.
- `tests/support/unavailable_input.rs`: SHA-256
  `cb30d87286dc3f8f039f915fafa63a72fe719b89309cd4dc014c207d25e394d3`.

The preservation audit confirmed unchanged production, schema and policy files;
unchanged locked package versions, sources and checksums; only the Unix test
dependency declaration and root lockfile entry added; and byte-preserved
selected-input regression and pure readiness functions. The implementation lane
also checked the existing key/frame helpers and blocking output-reader backend.

Focused macOS checks used `target/unavailable-input-implementation` as the Cargo
target directory and passed formatting, locked all-targets Clippy with warnings
denied, eight cleanup/setup tests, three terminal-input test functions and four
readiness-helper tests. The terminal functions included the existing 16-combination
input-loss regression. The incomplete-cleanup test exercised the actual separate
10-second deadline. The cancellation test intentionally caught a panic to verify
that reader shutdown cannot satisfy normal output completion.

Complete logs are retained under
`target/unavailable-input-implementation/logs/14-format-check-final.log` through
`19-frozen-candidate-and-preservation.log`. These focused checks are not substitutes
for full-suite results.

The new support module is 878 lines, including its exceptional-path tests. That
exceeds the early estimate made before the detailed cleanup requirements were
added. Independent review assessed proportionality and found the isolated reader,
ownership and shutdown support justified by those requirements, without a broad
migration of the existing harness. This remains a substantial testing maintenance
commitment, not a very small launcher addition.

## Initial independent implementation review

A fresh reviewer found no blocking implementation defect and independently ran
the existing compiled candidate on macOS: eight cleanup/helper tests, three
terminal-input functions and four readiness tests passed. These were executable
reruns, not a fresh build or full-suite validation. All 31 frozen source hashes
matched, and source comparison confirmed the existing selected-input function
and blocking-reader backend were preserved.

The reviewer requested two non-blocking accuracy corrections: name the main
support asset and its maintenance cost in the proposal, and rename the guarded
launch-drop test so it does not imply partial reader-startup failure injection.
The proposal is corrected. After all frozen-candidate validation commands
completed, the test was renamed to
`dropping_a_guarded_launch_reaps_the_child_and_stops_started_readers`.
The actual partial-startup ownership path was inspected but not fault-injected.

Review logs are retained under `target/unavailable-input-review/`. A broad
process-absence check was inconclusive because another session was running an
unrelated diagnostic test. The reviewer did not signal or otherwise alter it.

## Full macOS validation

On Apple Silicon macOS 26.5.2 (build 25F84), Homebrew Rust/Cargo 1.96.0 ran:

- `cargo fmt --check`: exit 0.
- `cargo clippy --locked --all-targets -- -D warnings`: exit 0.
- `cargo test --locked`: exit 0, 582 passed across 22 test executables; zero
  failed, ignored, measured or filtered out. The doc-test phase also succeeded
  with zero doc tests. The full command took 83.127 seconds.

The terminal suite ran all 45 tests, including eight exceptional helper tests,
both new test functions and all 16 configurations of the existing selected-input
regression. Ordinary guided use, cancellation, non-interactive routes, styling
and schema validation remained in the full run.

The source audit matched the frozen identity before and after. Each command ran
once, with no timeout or retry. The external supervisor tracked exact owned
descendants independently of process groups and confirmed zero owned processes
remaining, without sending cleanup signals. It did not alter unrelated processes.

Complete logs, commands, environment, source audits and cleanup observations are
retained under `target/unavailable-input-validation/macos/`. Its `pty/` directory
contains 2,556 actual terminal evidence files, including the new failure cases,
connected control and original input-loss configurations. These results precede
the test-name-only review correction described above.

## Full Linux validation

All project commands ran as UID/GID 10001:10001 in Debian 12 Linux aarch64,
with the checkout mounted read-only. Rust/Cargo 1.99.1, Clippy 0.1.99,
cargo-llvm-cov 0.9.1 and matching LLVM 23.1.1 were used, with a separate exact
Rust/Cargo 1.89.0 toolchain for the minimum supported version.

Each command passed once:

- `cargo +1.99.1 test --locked -- --show-output`: 582 passed.
- `cargo +1.99.1 clippy --locked --all-targets -- -D warnings`: exit 0.
- `cargo +1.99.1 llvm-cov --locked --workspace --all-targets --lcov --output-path /evidence/runner/validation/coverage/raw.lcov -- --show-output`:
  582 passed and a fresh LCOV report generated.
- `cargo +1.89.0 build --locked`: exit 0.
- `cargo +1.89.0 test --locked -- --show-output`: 582 passed.

All three full test runs had zero failures, ignored tests or filtered tests.
They each executed all eight launcher/cleanup tests, both new terminal test
functions and the existing 16-configuration selected-input regression. Normal
and minimum-version commands also completed their zero-test doc-test phase;
the instrumented all-targets command does not run doc tests.

Native read/create permission-denial probes succeeded, so the relevant
permission cases were genuinely exercised. The only `SKIP` message was the
intentional synthetic unsupported-probe control, not an unsupported environment.
Runs used default unoptimised Cargo profiles with debug information, disabled
incremental compilation and distinct stable, instrumented and minimum-version
target directories. No normal-operation deadline was extended, command retried,
test skipped or source changed. Before/after inventories matched the frozen
candidate. The unchanged Python subset also matched its earlier 71-test evidence.

The raw LCOV report is retained at
`target/unavailable-input-validation/linux/runner/validation/coverage/raw.lcov`:
537,779 bytes, SHA-256
`2aacb11f998a1be9d16d6ad946bf24a851f4eb3839ac9927ab25d0ee2c6d16c0`.
Its 24 source records match every production Rust file. Default exporter scope
excludes test-source records, so execution of the new helper is established by
test logs, not a claim of helper line coverage. Container-absolute paths are
unchanged. No policy consumer, threshold judgement or service upload was run.

Per-batch and final `/proc` scans found no remaining runner child processes.
The exact owned container was removed after evidence collection. An auxiliary
display-only `docker top` command rejected this backend's unsupported `uid`
column; its failure was retained without retry. This did not invalidate the
independent process scans or constitute a Rust test failure.

Exact commands, times, environments, source inventories, process observations
and actual terminal transcripts are retained under
`target/unavailable-input-validation/linux/runner/validation/`, indexed by
`VALIDATION.md` and `summary.json`. These results precede the test-name correction.

## Review correction and final source identity

The only code edit after the complete frozen-candidate runs renamed the guarded
launch-drop test. Its body and all other source files are unchanged. The final
helper SHA-256 is
`9ef9601a2ed715ddce44225f9bf0637d328f0f3dcc5af1d0405d218b17d79b3f`.
`target/unavailable-input-validation/final-candidate.json` records the final
source identity and repeated preservation checks.

Final-source macOS formatting, locked all-targets Clippy and full locked tests
passed: 582 tests, zero failed, ignored or filtered out, plus the successful
zero-test doc-test phase. All eight helpers, both new terminal functions and the
original 16 configurations executed. The full command took 52.858 seconds.
No timeout or cleanup signals were needed, and zero owned processes remained.
Before/after snapshots matched `final-candidate.json`. Reversing only the test
identifier reproduced the original frozen helper hash exactly, proving no body
change. Fresh logs and terminal transcripts are under
`target/unavailable-input-validation/macos-review-fix/`; earlier logs are intact.

Final-source Linux stable tests and locked all-targets Clippy also passed. The
full command ran 582 tests, with zero failed, ignored or filtered, plus the
successful zero-test doc-test phase. All eight helpers, both new functions and
the original 16 configurations executed; native permission-denial cases ran
without capability skips. All 31 source hashes matched, and reversing the one
identifier again reproduced the earlier helper exactly. Process scans found no
runner descendants, and the exact review-fix container was removed. Fresh logs,
source proofs, commands and terminal transcripts are under
`target/unavailable-input-validation/linux/review-fix/`.

The earlier instrumented and minimum-version results remain explicitly tied to
the pre-rename source; they are not presented as literal runs of the final byte
sequence. Independent exact identifier-only equivalence supports carrying those
results forward without suggesting a literal rerun.

## Final independent review

The reviewer checked raw logs, command/result records, source inventories,
cleanup records and retained artifact hashes, rather than relying only on
summaries. Both initial findings are closed. Final macOS/Linux results, the
pre-rename instrumented/minimum-version runs, Python subset preservation and
coverage-scope limitations are supported by the retained evidence. No new
implementation or evidence blocker remained, and no conditional Linux gate
remained open.

Review notes and the evidence audit are retained under
`target/unavailable-input-review/final/`. The reviewer requested final status
wording and platform-limit reconciliation, followed by strict OpenSpec and diff
checks. Those documentation corrections do not change the tested source.

## Platform and evidence limits

Local maintained-suite validation covers Apple Silicon macOS and Linux aarch64.
Linux x86-64 runner execution is left to the repository's unchanged PR checks;
Windows terminal behaviour is not covered by this Unix-only addition.
Raw local evidence lives under ignored `target/` paths and is not included in
the PR. The maintained record preserves commands, source identities, results and
limitations, rather than claiming those local files are remotely available.

## Product and documentation review

This is testing infrastructure, not a change to product behaviour or terminology.
The foundations' DX and efficiency values still support preserving trustworthy
failure diagnostics and file safety without a dependency fork. No foundation or
glossary revision is warranted. The production input-source question remains
explicitly deferred in question 14, and existing CLI interaction rules are not
weakened.

The proposal and design now acknowledge the larger isolated support module and
why the cleanup requirements expanded the initial estimate. The test-ci delta
retains the distinction between frame visibility and selected-input readiness,
and explicitly permits only the narrow shared lifecycle changes needed here.
No archived evidence has been rewritten.

The revised proposal, design, test-ci delta and verification record were reread
for first-time-reader clarity. `openspec validate cover-unavailable-terminal-input
--strict`, the local Markdown destination check (68 destinations across six
documents) and `git diff --check` all passed after the final reconciliation.

## Python helper checks

The unchanged Python helpers were checked independently while the Rust candidate
was still being implemented. In the prepared Linux aarch64 container, Python
3.11.2 with isolated `diff-cover[toml]==10.6.0` ran each suite once:

- `python -m unittest discover -s tests/coverage -v`: 47 tests, exit 0.
- `python -m unittest discover -s tests/rust_version -v`: 24 tests, exit 0.

The checkout was mounted read-only. Both commands used the isolated Python
environment, explicit `DIFF_COVER`, disabled bytecode writes and an owned temporary
directory. Neither timed out or required a retry. Before/after inventories of
`scripts/`, `tests/coverage/` and `tests/rust_version/` matched, with combined
SHA-256 `e086ee6c6af3a3264786a38c76291a42991ce6d17ae4a60b42be95e821a502c8`.

Logs, exact commands and per-file hashes are retained under ignored
`target/unavailable-input-validation/linux/python-helper-validation/`. These are
71 passing Python helper tests, not independent Rust-suite results. The unchanged
subset was verified again during both Linux validation stages, so these results
carry forward to the final implementation without a rerun.
