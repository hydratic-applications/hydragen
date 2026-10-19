## MODIFIED Requirements

### Requirement: Compact badge row

The README SHALL display nine linked badges immediately below its top-level
Hydragen heading and before the introduction. Its centred HTML header SHALL
contain status, language, MSRV, dependencies, coverage, and license in that order.
A line break SHALL retain repo, stars, and sponsors as the second group. Status,
language, MSRV, coverage, repo, stars, sponsors, and license images SHALL use HTTPS
Shields.io URLs; dependencies SHALL use the existing HTTPS deps.rs URL. All nine
SHALL use `for-the-badge` styling and `height="22"`. Images SHALL NOT use fixed
widths that prevent wrapping on narrow displays. The addition SHALL NOT replace
introductory text or otherwise redesign the header.

#### Scenario: Read the README introduction

- **WHEN** a reader opens the rendered README with badge images available
- **THEN** status, language, MSRV, dependencies, coverage, and license appear in
  that order in the first group, with repo, stars, and sponsors in the second
- **AND** all nine use matching `for-the-badge` styling at 22px height

#### Scenario: Read on a narrow display

- **WHEN** the rendered README is viewed at a phone-sized width
- **THEN** badges fit or wrap without requiring horizontal scrolling for the header

### Requirement: Accessible labels and useful navigation

The existing badges SHALL retain descriptive alternative text and their current
destinations: status to `#hydragen`, language to `https://rust-lang.org/`,
dependencies to `https://deps.rs/repo/github/hydratic-applications/hydragen`,
coverage to `https://app.codecov.io/gh/hydratic-applications/hydragen/tree/main`,
repo to `https://github.com/hydratic-applications/hydragen`, stars to
`https://github.com/hydratic-applications/hydragen/stargazers`, sponsors to
`https://github.com/sponsors/hydratic-applications`, and license to `LICENSE`.
Coverage SHALL retain alternative text `Code coverage on main`. The MSRV badge
SHALL link to `#build-and-run-locally` and have alternative text
`Minimum supported Rust version on main`, without a hardcoded version. The README
SHALL retain textual prototype limitations, checkout-specific minimum-version
build guidance, and its exact `GPL-3.0-or-later` License section independently of
images.

#### Scenario: Follow badge links

- **WHEN** a reader follows a badge link
- **THEN** the MSRV badge navigates to the build requirements, not a badge SVG
- **AND** the existing eight badges retain their useful destinations, including
  coverage to the main report and license to the local license file

#### Scenario: External images are unavailable

- **WHEN** images cannot load or a reader uses a nonvisual reading tool
- **THEN** alternative text identifies each badge's meaning, including the
  main-branch scope of the minimum compiler and coverage badges
- **AND** the prose still explains prototype limitations, build requirements,
  and licensing

### Requirement: Badges do not imply unsupported project guarantees

The README SHALL include only prototype-status, Rust-language, verified MSRV,
deps.rs dependencies, live Codecov main-branch coverage, GitHub repo, dynamic
stars, dynamic sponsors, and established `GPL-3.0-or-later` license badges. MSRV
SHALL express the source-build support baseline declared in main's `Cargo.toml`,
not an exact compiler pin, a required runtime installation, a CI pass result, or proof
that every compiler and platform is tested. Coverage SHALL remain a measured
service result, not a fixed percentage or guarantee of correctness, security,
complete testing, or platform support. The README SHALL NOT add badges claiming
a CI pass result, published version, or platform support. License SHALL agree
with the project notice and Cargo metadata. Badge rendering SHALL NOT introduce
a network requirement for ordinary local building, testing, or running Hydragen.

#### Scenario: Review badge claims

- **WHEN** a maintainer reviews the updated README
- **THEN** the MSRV badge reads main's Cargo declaration and build guidance
  distinguishes that scope from the local checkout's verified minimum
- **AND** coverage retains only the service-backed main-branch measurement
- **AND** the other badges retain their established meanings without new guarantees

#### Scenario: Use Hydragen without badge service access

- **WHEN** Shields.io, Codecov, or deps.rs is unavailable
- **THEN** ordinary builds, tests, and CLI use remain unaffected because those
  processes do not fetch badge images

## ADDED Requirements

### Requirement: Dynamic MSRV shield reads public main-branch Cargo metadata

The MSRV image SHALL use Shields.io's HTTPS `/badge/dynamic/toml` endpoint,
labelled `MSRV`, with the source
`https://raw.githubusercontent.com/hydratic-applications/hydragen/main/Cargo.toml`
and JSONPath selector `$.package['rust-version']`. It SHALL display the service's
result for that Cargo field without embedding a version literal or credentials.
Query parameters SHALL be URL-encoded and HTML separators escaped correctly.
Documentation SHALL make the main-branch scope and cache delay clear. CI SHALL
validate the configured endpoint, source, selector, and alternative text locally
without comparing main's live result to an unmerged checkout's version.

#### Scenario: The main-branch declaration changes

- **WHEN** a verified change to `rust-version` is merged to `main`
- **THEN** the existing shield URL can display the new declaration after service
  and image-proxy cache refreshes, without editing a badge version literal
- **AND** the image does not require a token or a published crate

#### Scenario: Read a checkout with a different baseline from main

- **WHEN** a branch or historical checkout has a different MSRV from current main
- **THEN** the dynamic badge still reports main's minimum
- **AND** alternative text identifies that scope and local build prose and
  `Cargo.toml` identify the checkout's own requirement
- **AND** CI does not reject the branch merely because main's value differs

#### Scenario: The declaration has not reached main or the service is unavailable

- **WHEN** main has no `rust-version` yet or the image service cannot read it
- **THEN** the real dynamic URL is retained without inventing a numeric result
- **AND** local Cargo metadata and build prose remain sufficient to find the
  checkout's requirement, with no image request needed for builds or CI checks

#### Scenario: The dynamic query points to the wrong source or field

- **WHEN** the README's MSRV image uses a different repository, branch, file,
  selector, or a static version message instead of the configured dynamic query
- **THEN** local validation fails with a diagnostic identifying the mismatch
