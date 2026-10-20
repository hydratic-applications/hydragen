## ADDED Requirements

### Requirement: Authoritative minimum supported Rust version

The root `Cargo.toml` SHALL declare `[package].rust-version` as a full stable
compiler version including its patch component. This value SHALL be the
authoritative minimum supported Rust version (MSRV) for building and testing
Hydragen with the checked-in lockfile. The initial value SHALL be verified with
successful locked builds and tests before compatibility is claimed. Dependency
metadata or edition selection alone SHALL NOT count as verification.

#### Scenario: Establish the initial baseline

- **WHEN** a maintainer introduces the MSRV declaration
- **THEN** the selected compiler has successfully built and tested the checkout
  with `--locked`, without bypassing Rust version checks
- **AND** the passing version and verification evidence are recorded

#### Scenario: A candidate cannot be verified

- **WHEN** toolchain installation or other infrastructure fails before verification
- **THEN** the candidate remains unverified
- **AND** the failure is not treated as proof of compiler incompatibility or as
  successful support verification

### Requirement: Verify the baseline independently in CI

CI SHALL check the declared MSRV on pull requests, main-branch pushes, and manual
runs, in a separate job named `MSRV` with its own visible result. The MSRV job
SHALL select the exact toolchain from the checked-out
`Cargo.toml`, run `cargo build --locked` and `cargo test --locked` using that
toolchain, and fail on either error. It SHALL have bounded execution, read-only
repository permissions, and no dependency on repository secrets. Existing
stable-compiler tests and coverage checks SHALL remain independent and retain
their toolchains, status names, coverage gates, and upload policy.
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
- **THEN** `Tests` and `MSRV` display their respective results independently
- **AND** neither job depends on or suppresses the other

#### Scenario: Accept the Linux compatibility evidence before merge

- **WHEN** implementation is ready for merge
- **THEN** the recorded Linux `MSRV` run identifies the implementation revision,
  shows the declared compiler was used, and passes its locked build and full tests
- **AND** a workflow syntax check or macOS result alone does not satisfy this check

### Requirement: Validate and pass the compiler version as data

The helper SHALL validate the parsed Cargo string before emitting an MSRV to
Actions outputs, environment values, or toolchain arguments. It SHALL accept
only canonical ASCII numeric `major.minor.patch`, with no leading zeroes except
for a zero component.
It SHALL reject channel aliases, prereleases, custom toolchain names, options,
whitespace/newlines, and shell metacharacters. Invalid input SHALL fail before a
version is emitted. Workflows SHALL pass the validated value as quoted data, not
interpolated shell source or an `eval` expression. Explicit Cargo toolchain
selection SHALL take precedence over runner defaults.

#### Scenario: Cargo metadata contains an unsupported toolchain string

- **WHEN** `rust-version` is not a canonical numeric stable version, including a
  channel alias, prerelease, option-like value, newline, or shell expression
- **THEN** validation fails with an actionable diagnostic before emitting the
  version or invoking a toolchain installer or Cargo

#### Scenario: A valid compiler version crosses the Actions step boundary

- **WHEN** a validated declaration is consumed by the toolchain and test steps
- **THEN** it is passed as quoted data without shell evaluation
- **AND** the compiler selected for the build and tests is the exact declaration

### Requirement: Keep README version claims consistent offline

A local consistency check SHALL read the authoritative Cargo value and verify
that the README's checkout-specific minimum-version prose agrees with it. It
SHALL also validate the dynamic MSRV badge's intended endpoint, public main-branch
source, Cargo field selector, and main-scoped alternative text, without comparing
the live main value against the local checkout. The MSRV CI job SHALL run the
check and its regression tests. Missing, malformed, or inconsistent declarations
or claims SHALL produce a nonzero result and an actionable diagnostic. The check
SHALL NOT fetch badge images or depend on remote repository access. Its implementation and tests SHALL
live outside `openspec/` and SHALL NOT read OpenSpec files.

#### Scenario: Maintainer changes only the Cargo declaration

- **WHEN** `rust-version` changes without updating the README's MSRV claims
- **THEN** the consistency check fails and identifies stale local build prose
- **AND** it directs the maintainer to align them with `Cargo.toml`
- **AND** no update to the dynamic image's version message is required

#### Scenario: A required claim is absent or malformed

- **WHEN** the Cargo declaration, dynamic badge configuration, alternative text,
  or minimum-version sentence is absent or malformed
- **THEN** the consistency check fails with a diagnostic rather than passing
  because no version could be found

#### Scenario: Check consistency without network access

- **WHEN** matching local metadata and README claims are checked offline
- **THEN** the check succeeds without contacting Shields.io or GitHub

### Requirement: Explain and maintain the support baseline

The README's existing build guidance SHALL identify the minimum compiler and
explain MSRV on first use. It SHALL recommend current stable for ordinary
development and distinguish the baseline from an exact compiler pin and from
requirements to run an already-built executable. It SHALL NOT imply that every
intermediate compiler or every platform is tested. Changes to the baseline SHALL
be deliberate responses to code or dependency needs, updating metadata and
checked documentation together, not automatic consequences of a new stable release.

#### Scenario: A developer reads the build requirements

- **WHEN** a developer reads the build section without loading badge images
- **THEN** the minimum supported compiler and its meaning are understandable
- **AND** a newer stable compiler is not described as unsupported merely because
  its version differs from the MSRV

#### Scenario: A newer stable compiler is released

- **WHEN** the current stable Rust version advances
- **THEN** stable CI can use it without automatically changing the declared MSRV

#### Scenario: A change requires a newer compiler

- **WHEN** a maintainer deliberately raises the minimum for code or dependencies
- **THEN** the same reviewed change updates Cargo metadata and README claims
- **AND** CI verifies the new baseline
