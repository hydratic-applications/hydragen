## MODIFIED Requirements

### Requirement: Coverage runs for pull requests and main

The repository SHALL provide a bounded GitHub Actions job with id `coverage` and
explicit check display name `Coverage report` for
`pull_request` events, pushes to `main`, and manual dispatch, without path filters.
It SHALL run independently of the normal `tests` job and generate a fresh LCOV
line-coverage report using stable Rust, `llvm-tools-preview`, and versioned
`cargo-llvm-cov`, respecting Cargo.lock and running all workspace test targets on
Linux, including unit, CLI subprocess, and Unix pseudo-terminal tests. It SHALL
NOT introduce a platform support claim, nightly-only coverage requirement, or
runtime dependency.

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

- **WHEN** any instrumented test, instrumentation, or report validation step fails
- **THEN** the coverage job fails and does not upload a misleading successful report
- **AND** tests are not silently skipped or production modules excluded to hide failure
- **AND** its failure does not replace the independent normal-test result

### Requirement: Codecov assesses project and changed-line coverage

Root Codecov configuration SHALL identify `main` as the branch and enable
separate project and patch statuses with `target: 80%`, `threshold: 0`,
and `informational: false`. Project SHALL assess overall line coverage; patch
SHALL independently assess added or modified executable lines. Each minimum
SHALL be 80%, not an automatic baseline-relative target or an 80-percentage-point
regression allowance. Neither check SHALL compensate for failure of the other.
Configuration SHALL NOT change GitHub branch protection. Documentation SHALL
distinguish Actions results from asynchronous Codecov assessments and explain
missing-base and no-applicable-executable-line states without fabricating success.
The project entry SHALL be named `total-coverage` and the patch entry
`changed-lines`, with both sections' `default` entries disabled. Their documented
status contexts SHALL be `codecov/project/total-coverage` and
`codecov/patch/changed-lines`; implementation SHALL verify the actual rendered
names and SHALL NOT add duplicate patch calculations or a privileged wrapper
solely to hide the Codecov prefix. Codecov SHALL be the sole enforcer of the two
coverage percentages; no Actions job SHALL add a duplicate local percentage gate.

#### Scenario: Codecov processes a PR with base coverage available

- **WHEN** Codecov has processed both a code-changing same-repository PR report
  and its base report
- **THEN** its app assesses overall and changed-line coverage against independent
  80% targets, passing at or above the target and failing below it
- **AND** neither assessment is forced into always-pass mode

#### Scenario: A developer reads the PR checks

- **WHEN** Actions and Codecov publish the configured checks
- **THEN** the Actions display names are `Tests` and `Coverage report`
- **AND** Codecov's named project and patch contexts identify `total-coverage`
  and `changed-lines`, rather than publishing extra bare default assessments
- **AND** no additional custom check repeats the changed-line assessment merely
  to replace Codecov's provider prefix

#### Scenario: Overall coverage passes but changed-line coverage fails

- **WHEN** overall coverage is 85% and added or modified executable lines are 70% covered
- **THEN** project meets its target while patch fails its own target

#### Scenario: Changed-line coverage passes but overall coverage fails

- **WHEN** added or modified executable lines are 90% covered but overall coverage is 75%
- **THEN** patch meets its target while project fails its own target

#### Scenario: The integration has no main baseline yet

- **WHEN** the first PR report is processed before a base report exists
- **THEN** setup documentation explains how the first main push establishes it
- **AND** missing base evidence is not presented as full coverage or a completed
  patch comparison
- **AND** a successful report job is not presented as a substitute percentage pass

#### Scenario: No executable lines were added or modified

- **WHEN** Codecov finds no added or modified executable lines to assess
- **THEN** its non-applicable, neutral, or skipped patch result is accepted
- **AND** the absence of applicable lines is not labelled a numerical 0% failure
  or a fabricated 100% result

### Requirement: Coverage setup is reproducible without affecting ordinary use

The README Development section SHALL include a local coverage command and link
to a focused reference explaining tooling prerequisites, measurement scope,
separate test, report-production, and Codecov assessment responsibilities, both
80% targets, OIDC, private Codecov access, fork limitations, badge access, and baseline
troubleshooting. Ordinary builds, tests, and CLI use SHALL NOT require Codecov,
Shields.io, upload authentication, or coverage tools. Implementation assets and
checks SHALL NOT load or embed files from `openspec/`.

#### Scenario: A developer generates coverage locally

- **WHEN** a developer follows the documented tooling and coverage commands
- **THEN** they can produce the same LCOV report without uploading or authenticating
- **AND** the reference explains that CI's percentage decisions come from Codecov,
  not a second local gate
- **AND** the reference explains Linux line coverage and stable-tooling limitations

#### Scenario: A developer uses the existing local workflow

- **WHEN** Codecov and Shields.io are unavailable or coverage tools are not installed
- **THEN** ordinary build, test, and CLI commands have no new service dependency
- **AND** normal tests are not subject to a coverage percentage threshold

#### Scenario: External account setup is incomplete

- **WHEN** private repository access, entitlement, or live service verification is missing
- **THEN** the remaining prerequisite is reported explicitly
- **AND** implementation does not change visibility, billing, or branch protection
  to conceal it

## ADDED Requirements

### Requirement: Report production does not enforce coverage percentages

The Actions `Coverage report` job SHALL generate and validate fresh coverage data
and attempt eligible uploads without enforcing either 80% target locally. Low
coverage alone SHALL NOT fail that job or prevent a valid report from being
uploaded. Instrumented tests, report validation, authentication, and upload
failures SHALL remain job failures. A successful job SHALL signify successful
report production and eligible uploading, not passage of Codecov's assessments.
Fork PRs SHALL retain their upload-skip policy without a local percentage fallback.

#### Scenario: A valid report is below the target

- **WHEN** instrumented tests, report validation, and uploading succeed but total
  or changed-line coverage is below 80%
- **THEN** `Coverage report` succeeds and preserves the report for inspection
- **AND** the applicable Codecov percentage check independently fails its target

#### Scenario: Percentage assessment is unavailable

- **WHEN** report production succeeds but Codecov processing or required base data
  is unavailable
- **THEN** no duplicate local percentage check is introduced
- **AND** the unavailable percentage result is not replaced by the report job's success

#### Scenario: Uploading fails after valid report generation

- **WHEN** a valid report is generated but Codecov uploading fails
- **THEN** `Coverage report` fails rather than suppress the upload error
- **AND** the test-only job retains its independent test result

#### Scenario: A fork has low coverage

- **WHEN** a fork PR produces a valid report below 80%
- **THEN** low coverage alone does not fail report production and uploading remains skipped
- **AND** neither Codecov percentage assessment is claimed to have run
