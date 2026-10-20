## Why

All CI jobs currently appear under the `Coverage` workflow, even when they check
ordinary test correctness or the minimum supported Rust version (MSRV). Separate
workflow headings and action-led job names should make each result's purpose
clear, distinguishing running tests, checking a policy, generating data, and
uploading it without changing what CI verifies or coupling testing to coverage.

## What Changes

- Introduce a `Testing` workflow containing the existing independent `tests` and
  `msrv` jobs, labelled `Run tests on stable Rust` and
  `Run tests on minimum supported Rust`. Retain job ids, commands, and compiler policies.
- Limit `Coverage` to `Generate coverage report`, `Check total coverage`,
  `Check changed-line coverage`, and `Upload main coverage to Codecov`, preserving
  the current artifact and trust boundaries.
- Use responsibility headings for workflows and action-led names for all six
  jobs: `Run` executes tests, `Check` evaluates a stated condition, `Generate`
  produces a report, and `Upload` transfers it. Rename display labels only, keeping
  every job id stable. Generation and upload use one report, not duplicate measurement.
- Keep both workflows enabled for pull requests, pushes to `main`, and manual
  dispatch, without path filters. Give each its own cancellation group.
- Preserve immutable PR-head checkouts, read-only testing, bounded execution,
  pinned actions, coverage thresholds, and main-only OIDC uploading.
- Update workflow regression tests and current CI documentation to reflect the
  separation, including that manual dispatch runs the selected workflow only.
- Document the workflow-name migration and review existing required-check
  references without changing branch protection or repository rules automatically.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `test-ci`: Place ordinary stable tests in a dedicated testing workflow,
  independently triggered and cancelled from coverage, with explicit action-led
  job labels.
- `rust-version-support`: Group minimum-compiler verification with testing, not
  coverage, use a parallel test-execution label that identifies the compiler,
  and retain its separate job and compiler-support contract.
- `code-coverage-ci`: Reserve the Coverage workflow for coverage responsibilities,
  name report generation, threshold checks, and upload by their actions, and align documentation
  with the two-workflow arrangement.

## Impact

- Adds `.github/workflows/tests.yml` and moves the two testing jobs out of
  `.github/workflows/coverage.yml`.
- Updates workflow assertions in `tests/coverage/test_coverage_gate.py` and
  `tests/rust_version/test_check_rust_version.py` without adding runtime dependencies
  or loading OpenSpec files from implementation tests.
- Revises existing explanations in `README.md`, `docs/coverage.md`, and
  `docs/going-public-checklist.md` rather than adding a separate CI tutorial.
- Changes workflow grouping for testing and display names for all six jobs.
  Integrations referencing old workflow names, paths, or check labels need review;
  required-check settings are not changed automatically.
- No CLI, Rust source, Cargo dependency, MSRV baseline, badge, coverage-policy,
  repository-permission, or publication changes.
