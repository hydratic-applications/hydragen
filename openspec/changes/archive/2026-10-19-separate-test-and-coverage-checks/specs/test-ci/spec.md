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

### Requirement: Terminal input-loss tests establish selected-input readiness

The dedicated-stdin PTY input-loss test SHALL verify a canonical baseline before
child spawn and, after the relevant Name or format-selector frame, observe that
same PTY becoming noncanonical before closing all input master handles. It SHALL
use supported termios observation (`MasterPty::get_termios` and named `ICANON`
flags), without platform-specific bit constants or new Cargo dependencies.
The transition SHALL be treated as evidence of stdin selection, not proof that
a read syscall is blocked or all terminal settings are raw. A visible frame alone
SHALL NOT be treated as selected-input readiness.

Readiness waiting SHALL retain the existing timeout, output-event draining and
child-status checks. Unsupported termios observation, early child exit and timeout
SHALL fail with useful diagnostics rather than skip or extend the test. Helper
tests SHALL cover pending/ready, expired, unsupported and exited-child states.
Finish-timeout changes SHALL add child-status and reader-count diagnostics only,
without changing the completion condition, timeout or child cleanup.

Both injection cases SHALL retain nonzero exit, one correctly styled input-error
diagnostic, empty standard output, no cancellation or success wording, and no
created files or layer manifest. No retries, arbitrary readiness sleeps or input
keystrokes, skipped tests or weakened assertions SHALL mask failures. This
requirement governs test setup only: it SHALL NOT relax production interaction
policy or imply that input loss before stdin selection has been fixed.

#### Scenario: A prompt is visible before input selection

- **WHEN** the target Name or selector frame is visible but the dedicated PTY is still canonical
- **THEN** the test waits for a noncanonical transition within the existing bound
- **AND** it does not disconnect input based on output alone

#### Scenario: Selected input is disconnected

- **WHEN** the dedicated input PTY has transitioned from its verified canonical baseline
- **THEN** the test closes every input master and checks the existing failure/style/no-files assertions
- **AND** it makes no claim that a read syscall was already blocked

#### Scenario: Input readiness cannot be established

- **WHEN** termios observation is unsupported, the child exits early or the deadline expires
- **THEN** the test fails with the relevant state and captured-output diagnostics
- **AND** it does not skip the case, inject readiness input or increase the timeout
