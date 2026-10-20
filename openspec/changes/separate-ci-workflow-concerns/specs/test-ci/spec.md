## ADDED Requirements

### Requirement: Correctness checks use a dedicated testing workflow

The repository SHALL provide `.github/workflows/tests.yml` with workflow name
`Testing`, containing the existing `tests` and `msrv` jobs with display names
`Run tests on stable Rust` and `Run tests on minimum supported Rust`.
The `tests` job SHALL retain stable Rust, `cargo test --locked`,
and its explicit Python coverage-helper regressions using isolated pinned
`diff-cover[toml]==10.6.0`. The `msrv` job SHALL retain the separate compiler-support
contract. Neither job SHALL remain in the `Coverage` workflow, and the testing
workflow SHALL NOT produce coverage artifacts, enforce percentage thresholds,
upload reports, or request OIDC credentials.

Both jobs SHALL retain Ubuntu execution, 30-minute timeouts, repository-content
read-only permissions, full reviewed action SHA pins, and checkout without
persisted credentials. Testing checkout SHALL select the immutable PR event head
SHA for PRs and `github.sha` otherwise, using testing-specific context rather than
coverage artifact context. The jobs SHALL NOT depend on each other or on coverage.

#### Scenario: A contributor inspects test results

- **WHEN** a PR triggers CI
- **THEN** the `Testing` workflow presents separate `Run tests on stable Rust`
  and `Run tests on minimum supported Rust` results
- **AND** ordinary test correctness and minimum-compiler compatibility are not
  grouped under `Coverage`

#### Scenario: Tests run on a fork's immutable head

- **WHEN** a fork PR triggers the testing workflow
- **THEN** both testing jobs check out the immutable event head rather than a
  synthetic merge commit, without persisted credentials or privileged permissions
- **AND** neither job needs coverage artifacts, coverage-service access, or secrets

#### Scenario: Existing helper regressions run after the move

- **WHEN** the `Run tests on stable Rust` job executes in the testing workflow
- **THEN** it runs the full existing Rust test command and Python coverage-helper suite
- **AND** helper regression failures still fail that job without applying a coverage percentage

### Requirement: CI job labels state their actions and subjects

Workflow names SHALL group responsibility as `Testing` and `Coverage`. Each job's
display name SHALL state its primary action and subject rather than only a topic,
result noun, or unexplained abbreviation. The testing jobs SHALL be named
`Run tests on stable Rust` and `Run tests on minimum supported Rust`. Coverage jobs
SHALL be named `Generate coverage report`, `Check total coverage`,
`Check changed-line coverage`, and `Upload main coverage to Codecov`.

Both compiler jobs SHALL use parallel test-execution wording, identifying their
compiler rather than implying different operations through different verbs.
These names SHALL distinguish test execution, coverage-threshold checks,
report generation, and upload without changing job ids, exit/failure semantics,
test commands, or validation policy. Names SHALL NOT embed a compiler-version
literal or imply that generating/uploading a report enforces a percentage gate.
Workflow regression tests SHALL assert the exact labels and corresponding job ids.

#### Scenario: A developer reads a standalone CI result

- **WHEN** a developer sees a job's display name outside its workflow grouping
- **THEN** the name identifies both the action and subject, including stable Rust,
  minimum supported Rust, the coverage scope, or the main-only Codecov upload
- **AND** understanding the minimum-compiler label does not require knowing the
  MSRV abbreviation

#### Scenario: A developer compares compiler test results

- **WHEN** a developer reads the two jobs under `Testing`
- **THEN** both names identify running tests, with stable Rust and minimum
  supported Rust as the difference
- **AND** the minimum-version job retains its supporting build and metadata checks
  without being labelled as a different kind of test operation

#### Scenario: Renaming checks does not change their outcomes

- **WHEN** a job runs with its new action-led display name
- **THEN** it retains its existing id, substantive commands, failure rules, and permissions
- **AND** generating or uploading coverage is not represented as another threshold check

### Requirement: Testing and coverage have independent workflow lifecycles

The testing workflow SHALL trigger on `pull_request`, pushes to `main`, and
`workflow_dispatch`, without path filters. It SHALL use
`testing-${{ github.event.pull_request.number || github.ref }}` as its concurrency
group with `cancel-in-progress: true`, distinct from coverage's existing group.
Neither workflow SHALL trigger or depend on the other through an event bridge
or a reusable-workflow call. Manually dispatching a workflow SHALL run only the
selected responsibility.

#### Scenario: Documentation-only changes trigger both responsibilities

- **WHEN** a documentation-only PR is opened or updated
- **THEN** both testing and coverage workflows are eligible to run without path filtering
- **AND** neither workflow waits for the other's result

#### Scenario: A newer testing run supersedes an older run

- **WHEN** a newer testing run starts for the same PR or ref
- **THEN** it can cancel older testing work through its own concurrency group
- **AND** its cancellation group does not cancel coverage work for that PR or ref

#### Scenario: A maintainer dispatches one workflow

- **WHEN** a maintainer manually dispatches `Testing` or `Coverage`
- **THEN** that workflow runs its own jobs without dispatching the other workflow
- **AND** running both responsibilities manually requires selecting each workflow

## MODIFIED Requirements

### Requirement: Tests have an independent CI result

The repository SHALL provide a bounded GitHub Actions job with id `tests` and
explicit check display name `Run tests on stable Rust`
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
- **THEN** `Run tests on stable Rust` succeeds while Actions' `Check total coverage`
  fails on valid below-target evidence
- **AND** low coverage alone does not fail the supporting `Generate coverage report` job

#### Scenario: A coverage consumer fails

- **WHEN** total or changed-line policy or its artifact validation fails
- **THEN** `Run tests on stable Rust` still runs and its result depends only on
  the normal Rust and helper tests
- **AND** the coverage consumers do not depend on the normal-test result

#### Scenario: Codecov or coverage tooling fails

- **WHEN** coverage generation, authentication, uploading, or Codecov processing fails
- **THEN** that failure does not prevent the normal test job from running or
  change its test-based result

#### Scenario: A fork or documentation-only PR is opened

- **WHEN** a fork or documentation-only PR triggers CI
- **THEN** the normal test job runs without needing coverage-service credentials
  or OIDC permissions
