## ADDED Requirements

### Requirement: Minimum-compiler compatibility is grouped with testing

The `msrv` job SHALL run in the `Testing` workflow with display name
`Run tests on minimum supported Rust`, at `.github/workflows/tests.yml`, not in
`Coverage`. Its move SHALL retain the exact toolchain selection from validated
`Cargo.toml` metadata, actual
compiler/Cargo version logging, locked build and full test commands, offline
README/badge consistency checks, helper regressions, and existing security and
execution bounds. It SHALL NOT introduce a duplicated compiler-version literal,
coverage tools, report generation, or dependencies on stable tests or coverage.

Workflow regression tests SHALL inspect the new testing workflow and continue
executing its actual validated-version handoff shell code to verify that invalid
metadata cannot emit toolchain output. Implementation tests SHALL remain outside
`openspec/` and SHALL NOT read OpenSpec artifacts.

#### Scenario: Stable tests pass but minimum-compiler tests fail

- **WHEN** current stable passes but the declared compiler cannot build or pass tests
- **THEN** `Testing` reports `Run tests on stable Rust` and
  `Run tests on minimum supported Rust` independently, with the latter failing
- **AND** coverage remains separately scheduled and assessed in `Coverage`

#### Scenario: MSRV metadata is invalid after the workflow move

- **WHEN** the MSRV handoff encounters invalid compiler metadata or inconsistent README claims
- **THEN** it fails before emitting a version or installing a compiler
- **AND** regressions verify the handoff extracted from `tests.yml`, not obsolete coverage YAML

#### Scenario: Coverage tooling or uploading fails

- **WHEN** coverage generation, a coverage gate, or Codecov upload fails
- **THEN** the minimum-compiler job still runs with its existing compiler-support checks
- **AND** its result is reported under testing rather than treated as a coverage failure

## MODIFIED Requirements

### Requirement: Verify the baseline independently in CI

CI SHALL check the declared MSRV on pull requests, main-branch pushes, and manual
runs, in a separate job named `Run tests on minimum supported Rust` with its own
visible result. The MSRV job SHALL select the exact toolchain from the checked-out
`Cargo.toml`, run `cargo build --locked` and `cargo test --locked` using that
toolchain, and fail on either error. It SHALL have bounded execution, read-only
repository permissions, and no dependency on repository secrets. Existing
stable-compiler tests and coverage checks SHALL remain independent and retain
their toolchains, coverage gates, and upload policy. Display names SHALL follow
the action-led naming contract in the test and coverage specifications, while
all existing job ids remain unchanged.
The job SHALL log actual compiler and Cargo versions. Before merge, a successful
Linux run for the implementation revision SHALL be observed and recorded with
its run URL, commit, toolchain versions, and locked build/full-test results.

#### Scenario: The runner default is newer than the baseline

- **WHEN** an MSRV job runs on a runner whose default Rust compiler is newer
- **THEN** the build and full test suite still use the declared baseline
- **AND** selecting the runner default cannot conceal an MSRV incompatibility

#### Scenario: Source or dependencies exceed the baseline

- **WHEN** the locked checkout cannot build or pass tests with the declared compiler
- **THEN** the MSRV job fails rather than choosing a newer compiler or changing
  dependency resolution to make the job pass
- **AND** success on current stable does not substitute for baseline success

#### Scenario: Verify minimum support without changing coverage

- **WHEN** the MSRV job is added
- **THEN** existing stable tests and coverage checks continue independently
- **AND** coverage tools are not required to work on the older compiler, coverage
  is not measured twice, and coverage does not depend on MSRV success

#### Scenario: Stable and minimum-compiler results differ

- **WHEN** tests pass on current stable but fail on the declared minimum, or the
  reverse
- **THEN** `Run tests on stable Rust` and `Run tests on minimum supported Rust`
  display their respective results independently
- **AND** neither job depends on or suppresses the other

#### Scenario: Accept the Linux compatibility evidence before merge

- **WHEN** implementation is ready for merge
- **THEN** the recorded Linux `Run tests on minimum supported Rust` run identifies the implementation revision,
  shows the declared compiler was used, and passes its locked build and full tests
- **AND** a workflow syntax check or macOS result alone does not satisfy this check
