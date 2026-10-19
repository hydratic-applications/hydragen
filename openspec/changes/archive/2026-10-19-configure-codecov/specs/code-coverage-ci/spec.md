## ADDED Requirements

### Requirement: Coverage runs for pull requests and main

The repository SHALL provide a bounded GitHub Actions `coverage` job for
`pull_request` events, pushes to `main`, and manual dispatch, without path filters.
It SHALL generate a fresh LCOV line-coverage report using stable Rust,
`llvm-tools-preview`, and versioned `cargo-llvm-cov`, respecting Cargo.lock and
running all workspace test targets on Linux, including unit, CLI subprocess, and
Unix pseudo-terminal tests. It SHALL NOT introduce a platform support claim,
nightly-only coverage requirement, or runtime dependency.

#### Scenario: A pull request is opened or updated

- **WHEN** a PR is opened, reopened, or updated, including a documentation-only PR
- **THEN** the coverage job runs the existing test targets with instrumentation
- **AND** success produces a nonempty LCOV report with source records for the
  library modules and `src/main.rs`, including hits from CLI subprocess execution

#### Scenario: Main receives a commit

- **WHEN** a commit is pushed to `main`
- **THEN** the coverage job generates a report for that main-branch commit
- **AND** the report is eligible for upload to establish or refresh main coverage

#### Scenario: Tests fail or report generation fails

- **WHEN** any test, instrumentation, or report validation step fails
- **THEN** the coverage job fails and does not upload a misleading successful report
- **AND** tests are not silently skipped or production modules excluded to hide failure

### Requirement: Authenticated uploads have explicit failure and trust boundaries

Successful same-repository PR, main push, and authorised manual runs SHALL upload
only the generated LCOV report to Codecov using GitHub OIDC. The job SHALL request
only read access to repository contents and job-scoped OIDC token issuance.
Actions SHALL use reviewed full commit SHA pins. Upload failures SHALL fail the
job. The workflow SHALL NOT execute PR code through `pull_request_target`, expose
stored credentials to fork code, or grant repository/PR/status write permissions.
Fork PRs SHALL run coverage generation but skip uploads with an explanatory job
summary. Upload secrets SHALL NOT appear in tracked files, badge URLs, or logs.

#### Scenario: A same-repository report is ready

- **WHEN** tests and LCOV validation succeed for an eligible run
- **THEN** the Codecov action uses OIDC and uploads only the explicit report path
- **AND** automatic report searching is disabled

#### Scenario: Authentication or uploading fails

- **WHEN** Codecov rejects authentication or the report cannot be uploaded
- **THEN** the Actions coverage job fails visibly rather than ignore the error
- **AND** documentation identifies repository access and OIDC as setup checks

#### Scenario: A fork submits a pull request

- **WHEN** the PR head repository differs from its base repository
- **THEN** the job still runs instrumented tests and generates local coverage
- **AND** it skips uploading, explains why in the job summary, and does not claim
  Codecov assessment succeeded

### Requirement: Codecov assesses project and changed-line coverage

Root Codecov configuration SHALL identify `main` as the branch and enable project
and patch default statuses with `target: auto`, `threshold: 0`, and
`informational: false`. It SHALL NOT impose an absolute coverage percentage or
change GitHub branch protection. Documentation SHALL distinguish Actions
test/report/upload results from asynchronous Codecov coverage assessments and
explain that meaningful base comparisons need a processed base report.

#### Scenario: Codecov processes a PR with base coverage available

- **WHEN** Codecov has processed both a same-repository PR report and its base report
- **THEN** its app publishes project and patch assessments using automatic targets
- **AND** the configuration does not force those assessments into always-pass mode

#### Scenario: The integration has no main baseline yet

- **WHEN** the first PR report is processed before a base report exists
- **THEN** the setup documentation explains how the first main push establishes it
- **AND** missing base evidence is not presented as full coverage or completed
  verification of comparative checks

### Requirement: Coverage setup is reproducible without affecting ordinary use

The README Development section SHALL include a local coverage command and link to
a focused reference explaining tooling prerequisites, measurement scope, OIDC,
private Codecov access, fork limitations, status semantics, badge access, and
baseline troubleshooting. Ordinary builds, tests, and CLI use SHALL NOT require
Codecov, Shields.io, upload authentication, or coverage tools. Implementation
assets and checks SHALL NOT load or embed files from `openspec/`.

#### Scenario: A developer generates coverage locally

- **WHEN** a developer follows the documented tooling and coverage commands
- **THEN** they can produce the same LCOV report without uploading or authenticating
- **AND** the reference explains Linux line coverage and stable-tooling limitations

#### Scenario: A developer uses the existing local workflow

- **WHEN** Codecov and Shields.io are unavailable or coverage tools are not installed
- **THEN** ordinary build, test, and CLI commands have no new service dependency

#### Scenario: External account setup is incomplete

- **WHEN** private repository access, entitlement, or live service verification is missing
- **THEN** the remaining prerequisite is reported explicitly
- **AND** implementation does not change visibility, billing, or branch protection
  to conceal it
