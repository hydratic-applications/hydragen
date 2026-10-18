## Why

The current coverage job combines test execution, report generation, and uploading,
so its result does not clearly distinguish test correctness from coverage policy.
Separate test results and explicit 80% targets for both overall and changed-line
coverage will make PR feedback easier to understand and act on.

## What Changes

- Provide distinct GitHub Actions jobs for normal tests and coverage-report
  generation/uploading, keeping test results independent of Codecov availability.
- Give the Actions checks clear display names, `Tests` and `Coverage report`.
  Use descriptive Codecov contexts, `codecov/project/total-coverage` and
  `codecov/patch/changed-lines`, instead of unexplained default names.
- Configure separate Codecov project and patch assessments with fixed 80%
  targets, rather than targets based on previous coverage. Both are independent
  requirements: passing overall coverage does not excuse poorly covered new code.
- Use Codecov as the sole enforcer of both coverage percentages, with no duplicate
  local percentage gate; continue uploading valid reports below the target so
  developers can inspect them.
- Preserve existing event coverage, terminal tests, upload security, private
  repository support, and the token-free main-branch README shield.
- Update the development documentation to explain the separate checks, exact
  thresholds, and missing-base or no-changed-executable-lines cases.

## Capabilities

### New Capabilities

- `test-ci`: A standalone, read-only test check whose result is independent of
  coverage percentages and Codecov service availability.

### Modified Capabilities

- `code-coverage-ci`: Name the supporting report job clearly and set both
  Codecov project and changed-line targets to 80%, without local percentage
  enforcement. Its baseline is introduced by
  the pending `configure-codecov` change on this same branch and PR; synchronise
  that baseline before this follow-up's delta specification.

## Impact

- Revises `.github/workflows/coverage.yml`, `codecov.yml`, README Development,
  and `docs/coverage.md`; no CLI, Cargo dependencies, or badge appearance changes.
- Continues on `swift-otter` and PR #8 as requested. Depends on the existing
  `configure-codecov` implementation, not a second Codecov integration.
- Adds a normal test run alongside the instrumented coverage run. This modest
  duplication keeps check responsibilities clear without passing compiled
  binaries or raw profiling data between jobs.
- Does not change branch protection, billing, repository visibility, or the
  existing policy of not uploading reports from fork PRs.
