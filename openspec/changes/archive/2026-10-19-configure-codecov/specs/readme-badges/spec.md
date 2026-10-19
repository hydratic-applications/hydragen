## MODIFIED Requirements

### Requirement: Compact badge row

The README SHALL display eight linked badges immediately below its top-level
Hydragen heading and before the introduction. Its centred HTML header SHALL
retain status, language, dependencies, and license in their existing relative
order, adding coverage between dependencies and license. A line break SHALL
retain repo, stars, and sponsors as the second group. Status, language, coverage,
repo, stars, sponsors, and license images SHALL use HTTPS Shields.io URLs; the
dependencies image SHALL use the existing HTTPS deps.rs URL. All eight SHALL
use `for-the-badge` styling and `height="22"` to match the header. Images SHALL NOT
use fixed widths that prevent wrapping on narrow displays.

#### Scenario: Read the README introduction

- **WHEN** a reader opens the rendered README with badge images available
- **THEN** status, language, dependencies, coverage, and license appear in that
  order in the first group, with repo, stars, and sponsors in the second group
- **AND** all eight use `for-the-badge` styling at 22px height without replacing introductory text

#### Scenario: Read on a narrow display

- **WHEN** the rendered README is viewed at a phone-sized width
- **THEN** badges fit or wrap without requiring horizontal scrolling for the header

### Requirement: Accessible labels and useful navigation

The existing badges SHALL retain descriptive alternative text and their current
destinations: status to `#hydragen`, language to `https://rust-lang.org/`,
dependencies to `https://deps.rs/repo/github/hydratic-applications/hydragen`,
repo to `https://github.com/hydratic-applications/hydragen`, stars to
`https://github.com/hydratic-applications/hydragen/stargazers`, sponsors to
`https://github.com/sponsors/hydratic-applications`, and license to `LICENSE`.
The coverage badge SHALL have alternative text `Code coverage on main` and link
to `https://app.codecov.io/gh/hydratic-applications/hydragen/tree/main`.
The README SHALL retain its textual prototype warning, current limitations, Rust
build guidance, and exact `GPL-3.0-or-later` License section independently of images.

#### Scenario: Follow badge links

- **WHEN** a reader follows a badge link
- **THEN** the existing seven badges retain their useful destinations
- **AND** coverage opens the Codecov main-branch report rather than a badge SVG
- **AND** license opens the repository's `LICENSE` rather than a third-party summary

#### Scenario: External images are unavailable

- **WHEN** images cannot load or a reader uses a nonvisual reading tool
- **THEN** descriptive alternative text identifies each badge's meaning, including
  that coverage tracks `main`
- **AND** the prose still explains prototype limitations, local builds, and licensing

### Requirement: Badges do not imply unsupported project guarantees

The README SHALL include only prototype-status, Rust-language, deps.rs
dependencies, live Codecov main-branch coverage, GitHub repo, dynamic stars,
dynamic sponsors, and the established `GPL-3.0-or-later` license badges. Coverage
SHALL present a measured service result, not a fixed percentage or a guarantee of
correctness, security, complete testing, or platform support. The README SHALL
NOT add badges claiming a CI pass result, published version, minimum supported
Rust version, or platform support. License SHALL agree with the project notice
and Cargo metadata. Badge rendering SHALL NOT introduce a network requirement
for ordinary local building, testing, or running Hydragen.

#### Scenario: Review badge claims

- **WHEN** a maintainer reviews the updated README
- **THEN** coverage adds only the latest service-backed main-branch measurement
- **AND** other badges retain their established meanings without new guarantees

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

## REMOVED Requirements

### Requirement: Shortcut badge references with definitions at the bottom

**Reason**: The current README uses a centred HTML header, not the historical
Markdown reference paragraph. Adding coverage must preserve that header rather
than require an unrelated source/layout rewrite.

**Migration**: Retain one linked HTML image per source line, using existing badge
URLs and destinations. Use the updated compact-header and navigation requirements
for order, wrapping, and accessible text; escape HTML query separators correctly.

## ADDED Requirements

### Requirement: Live main-branch coverage shield

The coverage image SHALL use
`https://img.shields.io/codecov/c/github/hydratic-applications/hydragen/main?style=for-the-badge`,
with `logo=codecov` for the service's native-colour icon and optionally with an
approved dedicated Codecov badge-access token. HTML query separators SHALL be
escaped as `&amp;`; no hardcoded replacement icon SHALL replace the native logo.
The branch SHALL be explicit regardless of which branch contains the displayed README.
The image SHALL remain service-backed, not a hardcoded percentage or checked-in
snapshot. Unavailable or uninitialised service data SHALL NOT be replaced with
zero or a fabricated success. Documentation SHALL explain first-main-report and
cache-refresh requirements.

For a private repository, implementation SHALL request maintainer approval before
embedding a dedicated badge-access token and explain that URL readers, Shields.io,
and image proxies can see it. It SHALL NOT embed an upload token, API token, or
GitHub credential. Without approved badge access, the token-free URL SHALL remain
and the unavailable result and remaining setup SHALL be explicit; numerical
private-badge verification SHALL remain incomplete. No visibility change SHALL
be required. Local README rendering SHALL NOT require secret interpolation.
Documentation SHALL treat later publication as a separate maintainer decision,
not a prerequisite for private CI. It SHALL explain verifying public Codecov
report access, retaining the same branch-specific image and report URLs, and
removing and revoking any no-longer-needed badge token after publication. It SHALL
NOT imply that removing a token from the README removes it from Git history.

#### Scenario: Read the README from a feature branch

- **WHEN** a feature branch's README is rendered and main coverage is accessible
- **THEN** the shield displays Codecov's latest processed `main` coverage after
  normal cache refresh, not that feature branch's coverage

#### Scenario: No main report or approved private badge access exists

- **WHEN** Codecov has no processed main report or Shields.io cannot access it
- **THEN** the live URL and useful report link remain without a fabricated number
- **AND** documentation explains the remaining setup without claiming the live
  numeric badge has been verified

#### Scenario: Private badge access is approved

- **WHEN** the maintainer approves including Codecov's dedicated badge-access token
- **THEN** only that token is added to the image URL with HTML-safe query separators
- **AND** no upload or repository credential is included or requested for rendering

#### Scenario: The maintainer later makes the repository public

- **WHEN** GitHub repository visibility and Codecov report access become public
- **THEN** the existing token-free branch-specific image URL and report link remain
  appropriate without replacing them with a different badge integration
- **AND** documentation explains removing any private badge token from the current
  README and revoking it if no longer needed, while retaining OIDC uploads
