## ADDED Requirements

### Requirement: Tests have an independent CI result

The repository SHALL provide a bounded GitHub Actions job with id `tests` and
explicit check display name `Tests`
for pull requests, pushes to `main`, and manual dispatch, without path filters.
It SHALL use stable Rust and the locked dependency versions to run the normal
unit, integration, Unix terminal, and documentation tests with `cargo test --locked`
and the Python helper tests with
`python3 -m unittest discover -s tests/coverage -v`. Any test failure SHALL
fail this job. The job SHALL have only repository-content read permission, with
no Codecov upload, OIDC issuance, coverage target, or dependency on the coverage
job. Existing pinned-checkout and credential-persistence safeguards SHALL remain.
The Python suite SHALL use isolated pinned diff-cover 10.6.0 for real diff
integration; this dependency SHALL NOT be added to ordinary Rust test commands.

#### Scenario: A test fails

- **WHEN** any normal Rust or Python helper test fails
- **THEN** the `tests` check fails independently of measured coverage
- **AND** no failing or terminal test is skipped to produce a passing result

#### Scenario: Tests pass but coverage is too low

- **WHEN** all normal tests pass and overall coverage is below 80%
- **THEN** `Tests` succeeds while Actions' `Total coverage` fails on valid below-target evidence
- **AND** low coverage alone does not fail the supporting `Coverage report` job

#### Scenario: A coverage consumer fails

- **WHEN** total or changed-line policy or its artifact validation fails
- **THEN** `Tests` still runs and its result depends only on the normal Rust and helper tests
- **AND** the coverage consumers do not depend on the normal-test result

#### Scenario: Codecov or coverage tooling fails

- **WHEN** coverage generation, authentication, uploading, or Codecov processing fails
- **THEN** that failure does not prevent the normal test job from running or
  change its test-based result

#### Scenario: A fork or documentation-only PR is opened

- **WHEN** a fork or documentation-only PR triggers CI
- **THEN** the normal test job runs without needing coverage-service credentials
  or OIDC permissions
