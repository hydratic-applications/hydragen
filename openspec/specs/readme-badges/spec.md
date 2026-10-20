# README badges specification

## Purpose

Define the README's compact badge row, its accessible labels and useful links,
and how live service results are presented without implying unsupported project
guarantees or affecting local builds.

## Requirements

### Requirement: Compact badge row

The README SHALL display nine linked badges immediately below its top-level
Hydragen heading and before the introduction. Its centred HTML header SHALL
contain status, language, minimum Rust version, dependencies, and coverage in that
order as the first group. A line break SHALL place license, repo, stars, and
sponsors in the second group, balancing the rows without fixing their widths. Status,
language, MSRV, coverage, repo, stars, sponsors, and license images SHALL use HTTPS
Shields.io URLs; dependencies SHALL use the existing HTTPS deps.rs URL. All nine
SHALL use `for-the-badge` styling and `height="22"`. Images SHALL NOT use fixed
widths that prevent wrapping on narrow displays. The addition SHALL NOT replace
introductory text or otherwise redesign the header. The source SHALL retain one
linked HTML image per line, with HTML-safe query separators.

#### Scenario: Read the README introduction

- **WHEN** a reader opens the rendered README with badge images available
- **THEN** status, language, minimum Rust version, dependencies, and coverage appear
  in that order in the first group, with license, repo, stars, and sponsors in the second
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

### Requirement: Stable deps.rs URLs with an accepted unknown state

The dependencies image SHALL use
`https://deps.rs/repo/github/hydratic-applications/hydragen/status.svg?style=for-the-badge`.
The image and report URLs SHALL omit `path` because Hydragen's `Cargo.toml` is at
the repository root. The badge SHALL be included even if deps.rs cannot analyze
the private repository and returns `unknown`. Adding it SHALL NOT require a
repository visibility change, credentials, or a successful analysis. Neither its
alternative text nor accompanying prose SHALL claim that dependencies are healthy
or that the badge guarantees security.

#### Scenario: Repository remains private

- **WHEN** deps.rs cannot analyze the private repository and returns an `unknown` badge
- **THEN** the README still includes the live badge and report link
- **AND** this temporary service result does not block adding the badge

#### Scenario: Repository later becomes publicly analyzable

- **WHEN** the repository becomes public and deps.rs successfully analyzes it
- **THEN** the existing badge and report URLs remain appropriate without a README edit
- **AND** the badge displays the service result when service and image caches refresh,
  without a promise of immediate updates

### Requirement: Live stars and account-wide sponsor counts

The stars image SHALL use
`https://img.shields.io/github/stars/hydratic-applications/hydragen?style=for-the-badge&label=stars`.
The sponsors image SHALL use
`https://img.shields.io/github/sponsors/hydratic-applications?style=for-the-badge&label=sponsors`.
Existing presentation-only logo and colour parameters SHALL remain permitted.
These SHALL remain service-backed dynamic images, not hardcoded counts, snapshots,
or locally generated assets. The sponsor count SHALL refer to the owner
`hydratic-applications`, not be described as a repository-specific count.

Unavailable results SHALL NOT be replaced with zero. The badges SHALL be included
even if the private repository is not found by Shields.io or the organization has
no Sponsors listing. This change SHALL NOT require public repository access,
credentials, enabling sponsorship, or a working sponsorship destination.

#### Scenario: Public counts change

- **WHEN** GitHub exposes updated star or sponsor counts to Shields.io
- **THEN** the existing badge URLs can display the updated counts after cache
  refreshes without a README edit
- **AND** zero is displayed only when it is the service's result

#### Scenario: Public data or sponsorship is not available

- **WHEN** the stars integration reports `repo not found` for the private repository
  or the owner's GitHub Sponsors page is unavailable
- **THEN** the README still includes the dynamic badges and their intended links
- **AND** implementation does not substitute a zero, add credentials, change
  visibility, or enable sponsorship to conceal those limitations

### Requirement: License badge is independent of repository visibility

The license image SHALL be a static Shields.io badge stating the exact
`GPL-3.0-or-later` identifier, not a GitHub-backed license detection badge or
an ambiguous `GPL v3` label. Adding or viewing the badge SHALL NOT require
repository publication, repository credentials, or successful remote license
detection. The license text and project application notice SHALL be authoritative
rather than any service's detected label.

#### Scenario: Establish licensing while the repository is private

- **WHEN** the root license, project notice, and Cargo declaration have been added
  but GitHub cannot expose repository licensing to Shields.io
- **THEN** the image URL can still render `license: GPL-3.0-or-later` without
  repository access
- **AND** its relative destination remains the repository's `LICENSE` file
- **AND** no public-visibility change or credential is needed

### Requirement: Live main-branch coverage shield

The coverage image SHALL use the token-free URL
`https://img.shields.io/codecov/c/github/hydratic-applications/hydragen/main?style=for-the-badge`,
with `logo=codecov` for the service's native-colour icon. HTML query separators
SHALL be escaped as `&amp;`; no hardcoded replacement icon SHALL replace the native
logo. The branch SHALL be explicit regardless of which branch contains the
displayed README. The image SHALL remain
service-backed, not a hardcoded percentage or checked-in snapshot. Unavailable or
uninitialised service data SHALL NOT be replaced with zero or fabricated success.
Documentation SHALL explain first-main-report, report-access, and cache-refresh
requirements. Rendering SHALL NOT embed badge-access, upload, API, or repository
credentials, and local README rendering SHALL NOT require secret interpolation.
Neither a visibility nor billing change SHALL be required for local coverage gates.

Documentation SHALL distinguish public GitHub repository visibility from public
Codecov report access. If an earlier private badge-access token exists, removal
from the README SHALL NOT be described as revoking it or removing it from Git
history; documentation SHALL direct maintainers to check remaining consumers and
revoke no-longer-needed tokens separately. OIDC uploading SHALL remain unchanged.

#### Scenario: Read the README from a feature branch

- **WHEN** a feature branch's README is rendered and main coverage is accessible
- **THEN** the shield displays Codecov's latest processed `main` coverage after
  normal cache refresh, not that feature branch's coverage

#### Scenario: Main report or anonymous badge access is unavailable

- **WHEN** Codecov has no processed main report or Shields.io cannot access it
- **THEN** the live URL and useful report link remain without a fabricated number
- **AND** documentation explains remaining access, processing, or cache checks
  without claiming the live numeric badge has been verified
- **AND** no token is added to conceal the unavailable result

#### Scenario: Public repository and report access are verified separately

- **WHEN** the GitHub repository is public
- **THEN** maintainers still verify public Codecov report access and processing
- **AND** the existing token-free branch-specific image and report URLs remain
  appropriate without replacing the integration or changing OIDC uploads
- **AND** any removal of an earlier private badge token is distinguished from
  revocation and Git-history removal

### Requirement: Dynamic MSRV shield reads public main-branch Cargo metadata

The MSRV image SHALL use Shields.io's HTTPS `/badge/dynamic/toml` endpoint,
labelled `Minimum Rust`, with the white Rust logo (`logo=rust`, `logoColor=white`)
so readers need not know the MSRV abbreviation. The source SHALL be
`https://raw.githubusercontent.com/hydratic-applications/hydragen/main/Cargo.toml`
with JSONPath selector `$.package['rust-version']`. It SHALL display the service's
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
