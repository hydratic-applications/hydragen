## 1. Separate job responsibilities

- [x] 1.1 Add an independent job with id `tests` and display name `Tests` to the existing workflow, using pinned checkout, no persisted credentials, stable Rust, `cargo test --locked`, and only content-read permission; retain existing events, concurrency, and bounded execution without adding path filters or a dependency on coverage.
- [x] 1.2 Give the separate `coverage` job display name `Coverage report`, retaining its complete instrumented test run, fresh LCOV report, library/CLI validation, pinned tooling, OIDC scope, and fork-upload guard; make report-pipeline failures distinct from tests and Codecov percentage decisions without suppressing instrumented test failures.

## 2. Enforce independent 80% coverage targets

- [x] 2.1 Keep percentage enforcement out of the report job: do not add `--fail-under-lines`, a custom calculator, or another percentage gate. Ensure valid below-target reports still upload, while instrumented-test, report-validation, and upload failures remain failures.
- [x] 2.2 Configure named Codecov project `total-coverage` and patch `changed-lines` entries with targets of `80%`, zero tolerance, and non-informational results; explicitly disable both `default` entries to avoid duplicate assessments, and retain `main`, the token-free shield, account boundaries, and unchanged branch protection.

## 3. Validate results and failure boundaries

- [x] 3.1 Run Actionlint and Codecov YAML validation; verify job independence, read-only test permissions, scoped coverage OIDC, event coverage, upload guards, and configured failure behaviour.
- [x] 3.2 Check that both named Codecov targets are exactly `80%`, with zero tolerance and non-informational results, and that no workflow command or duplicate check independently enforces coverage percentages. Verify a failing normal test fails `Tests` without making coverage percentages part of its result.
- [x] 3.3 Check step ordering and outcomes for valid low coverage, missing/invalid reports, upload failure, cancellation, and fork PRs; confirm low percentages alone do not fail `Coverage report`, valid uploads are retained, and missing service/baseline evidence is never represented as a percentage pass.
- [x] 3.4 Run normal and instrumented suites on Linux, including terminal tests with bounded waits and child cleanup; inspect the same library/CLI report scope without new production exclusions, and run the existing formatting and Clippy checks.

## 4. Document and verify rollout

- [x] 4.1 Revise README Development and `docs/coverage.md` for `Tests`, `Coverage report`, and the two Codecov-only percentage assessments, including 80% targets, no-executable-change patch results, baseline/service and fork limitations, and low-coverage report retention; review from a first-time reader's perspective without redesigning badges or changing product foundations.
- [ ] 4.2 On an authorised push to the existing PR, observe `Tests` and `Coverage report` and confirm named project/patch configuration is accepted; verify Codecov's actual rendered labels against the documented contexts `codecov/project/total-coverage` and `codecov/patch/changed-lines`, with no extra default assessments. Record evidence separately from the original integration's verification; do not create another PR or merge automatically.
- [ ] 4.3 Verify independent Codecov project/patch pass and fail outcomes on eligible reports once a processed base report exists, covering the 80% minimum and high-total/low-patch behaviour where controlled live evidence is available; explicitly leave unavailable evidence pending rather than add unrelated source changes or claim documentation-only diffs prove the changed-line gate.
- [x] 4.4 Confirm and record the specification dependency for later synchronisation: `configure-codecov`'s additions must precede this follow-up's modifications, whose fixed targets supersede the earlier policy. Preserve historical evidence and leave actual synchronisation/archival to that separately requested workflow.
