## Why

Hydragen has Rust tests but no CI workflow or published coverage reports, so PR
reviewers cannot see which code those tests exercise. With the Codecov GitHub app
already installed, automated reports can make coverage changes visible during
review and show the latest measured `main` coverage in the README.

## What Changes

- Add a GitHub Actions coverage job for pull requests and pushes to `main`, using
  the existing unit, integration, and terminal tests to measure Rust line coverage.
- Upload successful reports to Codecov with authenticated uploads and visible
  failures when report generation or uploading fails.
- Configure project and patch coverage checks against the measured base coverage,
  without inventing a fixed percentage target or changing branch protection.
- Add a live Shields.io coverage badge matching the README's `for-the-badge`
  styling at 22px height and explicitly tracking `main`,
  linked to that branch's Codecov report and integrated into the existing README
  header without reorganising unrelated badges.
- Document local coverage generation and the remaining private-repository service
  setup, including the distinction between upload authentication and badge access.
  Support the current private repository without waiting for its intended later
  publication; keep the badge URL suitable for public access when that happens.

## Capabilities

### New Capabilities

- `code-coverage-ci`: Rust coverage generation, secure Codecov uploads, PR status
  checks, and main-branch report publication.

### Modified Capabilities

- `readme-badges`: Permit and describe a service-backed main-branch coverage shield;
  update the badge layout and source requirements to preserve the current HTML
  header rather than restore an obsolete Markdown layout.

## Impact

- Adds `.github/workflows/coverage.yml` and root `codecov.yml`; updates README.md
  and adds a focused coverage reference under `docs/`.
- Introduces CI-only tooling (`cargo-llvm-cov`, Rust's LLVM tools, and the Codecov
  upload action), not runtime dependencies or changes to Hydragen's CLI.
- Uses GitHub Actions, the installed Codecov app, Codecov repository access, and
  Shields.io. Private coverage/badge access may require account configuration;
  repository visibility and branch-protection changes remain outside scope.
- Keeps ordinary local builds and tests independent of these services. No source,
  test, or build script will read implementation assets from `openspec/`.
