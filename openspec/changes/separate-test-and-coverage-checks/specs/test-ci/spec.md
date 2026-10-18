## ADDED Requirements

### Requirement: Tests have an independent CI result

The repository SHALL provide a bounded GitHub Actions job with id `tests` and
explicit check display name `Tests`
for pull requests, pushes to `main`, and manual dispatch, without path filters.
It SHALL use stable Rust and the locked dependency versions to run the normal
unit, integration, Unix terminal, and documentation tests. Any test failure SHALL
fail this job. The job SHALL have only repository-content read permission, with
no Codecov upload, OIDC issuance, coverage target, or dependency on the coverage
job. Existing pinned-checkout and credential-persistence safeguards SHALL remain.

#### Scenario: A test fails

- **WHEN** any test in the normal test command fails
- **THEN** the `tests` check fails independently of measured coverage
- **AND** no failing or terminal test is skipped to produce a passing result

#### Scenario: Tests pass but coverage is too low

- **WHEN** all normal tests pass and overall coverage is below 80%
- **THEN** `Tests` succeeds while Codecov's total-coverage assessment fails when
  it can assess that report
- **AND** low coverage alone does not fail the supporting `Coverage report` job

#### Scenario: Codecov or coverage tooling fails

- **WHEN** coverage generation, authentication, uploading, or Codecov processing fails
- **THEN** that failure does not prevent the normal test job from running or
  change its test-based result

#### Scenario: A fork or documentation-only PR is opened

- **WHEN** a fork or documentation-only PR triggers CI
- **THEN** the normal test job runs without needing coverage-service credentials
  or OIDC permissions
