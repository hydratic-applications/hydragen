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
- Use `Coverage` for PRs and read-only manual checks, containing report generation
  and the two coverage gates, with no upload job or skipped upload result.
- Add `Coverage on main` for pushes to `main` and manual runs, containing the same
  three jobs plus `Upload main coverage to Codecov`, guarded to main only. Share
  generation/check steps through local composite actions rather than duplicate
  their logic, preserving job labels, same-run artifacts, and the trust boundary.
- Use responsibility headings for workflows and action-led names for all six
  jobs: `Run` executes tests, `Check` evaluates a stated condition, `Generate`
  produces a report, and `Upload` transfers it. Rename display labels only, keeping
  every job id stable. Generation and upload use one report, not duplicate measurement.
- Keep Testing enabled for PRs, main pushes, and manual dispatch; route coverage
  automatically by event to the PR or main entry point, without path filters.
  Each entry point has an independent cancellation group and manual dispatch.
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
- `code-coverage-ci`: Separate PR/main coverage entry points so uploading is absent
  from PR results, share read-only measurement/check steps, and name generation,
  threshold checks, and upload by their actions.

## Impact

- Adds `.github/workflows/tests.yml` and moves the two testing jobs out of
  `.github/workflows/coverage.yml`.
- Adds `.github/workflows/coverage-main.yml` for main publication and shared
  composite actions under `.github/actions/coverage-report/` and
  `.github/actions/coverage-check/`. PR/main runs each measure once and consume
  their own report; neither workflow dispatches the other or transfers artifacts
  across runs.
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
