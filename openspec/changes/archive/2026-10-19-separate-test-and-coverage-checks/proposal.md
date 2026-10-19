## Why

Test correctness, coverage policy, and report publishing answer different questions.
GitHub Actions should give every PR, including fork PRs, independent test and
coverage results without depending on a coverage service's availability or account
features. Report generation must preserve useful evidence even below the target.

## What Changes

- Provide three Actions-owned quality checks: `Tests`, `Total coverage`, and
  `Changed-line coverage`. Require at least 80% for each applicable coverage gate,
  using exact integer counts rather than rounded percentages.
- Keep `Coverage report` as a read-only producer of fresh Linux coverage and
  validated metadata. Validate raw LLVM output, then retain canonical line-only
  LCOV counting every unique measured source line once. The two percentage
  consumers depend only on that producer,
  not on each other, normal tests, or Codecov. Retain valid below-target reports.
- Measure the immutable PR head and compare its changed lines against the merge
  base of that head and the event's immutable base SHA. Bind report evidence to
  repository, run/attempt, event, revisions, digest, and tracked production sources.
- Use pinned `diff-cover` for diff analysis with validated integer JSON results;
  apply the inclusive threshold in the repository's adapter. Explicitly report
  valid zero-applicable-line and non-PR results as not applicable, not 100%.
- Disable Codecov project and patch statuses. Keep Codecov for report browsing and
  the existing token-free main-branch README shield, not percentage enforcement.
- Isolate OIDC uploading in a separate main-push/manual-main job that consumes
  validated same-run report data without checking out or executing repository code.
  Do not upload from any PR, including same-repository and bot PRs.
- Revise development documentation and the publication checklist for these trust
  boundaries, failure modes, and independent checks.

## Capabilities

### New Capabilities

- `test-ci`: A standalone, read-only normal Rust and coverage-helper test check,
  independent of coverage percentages and report publishing.

### Modified Capabilities

- `code-coverage-ci`: Actions-owned total and changed-line gates, run-bound report
  artifacts, and isolated main-only authenticated uploading replace service-owned
  policy and PR uploading. The pending `configure-codecov` change introduces the
  baseline: synchronise its additions before this follow-up's modifications when
  separately requested. Do not modify archived assets or reinterpret historical
  verification as evidence for the revised gates.

## Impact

- Revises `.github/workflows/coverage.yml`, `codecov.yml`, README Development,
  `docs/coverage.md`, and `docs/going-public-checklist.md`; adds maintained Python
  policy helpers and tests outside `openspec/`.
- Preserves stable Rust, `cargo-llvm-cov` 0.9.1, the workspace/all-targets Linux
  measurement scope, terminal tests, and inline unit-test contributions. Adds no
  production-source exclusions or Rust/runtime dependencies. `diff-cover` 10.6.0
  is an isolated CI/developer tool, not a CLI dependency.
- Continues on the existing branch and PR. Normal and instrumented test runs
  remain separate; consumers share only report data, not executables or profiles.
- Does not change repository visibility, billing, required checks, branch
  protection, normal offline CLI use, or the existing README shield URL/appearance.
  Publication remains subject to its separate readiness checks.
