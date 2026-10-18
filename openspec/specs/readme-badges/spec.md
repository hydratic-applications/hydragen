# README badges specification

## Purpose

Define the README's compact badge row, its accessible labels and useful links,
and how live service results are presented without implying unsupported project
guarantees or affecting local builds.

## Requirements

### Requirement: Compact badge row

The README SHALL display seven linked badges immediately below its top-level
Hydragen heading and before the introduction: static `status: prototype` and
`language: Rust` badges served over HTTPS by `img.shields.io`, followed by a live
dependencies badge served over HTTPS by `deps.rs`, and a static
`repo: hydratic-applications/hydragen`
badge served over HTTPS by `img.shields.io`, followed by dynamic stars and sponsors
badges served over HTTPS by `img.shields.io`, then a static
`license: GPL-3.0-or-later` badge served over HTTPS by `img.shields.io`.
All seven SHALL use the compact `flat-square` style with square corners and no
gradients. The layout SHALL use ordinary Markdown without a fixed width so the
badges can wrap on narrow displays.

#### Scenario: Read the README introduction

- **WHEN** a reader opens the rendered README with badge images available
- **THEN** the prototype-status, Rust-language, dependencies, repo, stars,
  sponsors, and license badges appear below the title, in that order, before
  the introduction
- **AND** all seven use `flat-square` styling with square corners and no gradients,
  without replacing any introductory text

#### Scenario: Read on a narrow display

- **WHEN** the rendered README is viewed at a phone-sized width
- **THEN** the badges fit or wrap without requiring horizontal scrolling for the
  badge row

### Requirement: Accessible labels and useful navigation

The status badge SHALL have alternative text `Status Badge` and a Markdown
link to `#hydragen`. The language badge SHALL have alternative text
`Rust Badge` and a Markdown link to `#build-and-run-locally`. The dependencies
badge SHALL have alternative text `Deps Badge` and link to
`https://deps.rs/repo/github/hydratic-applications/hydragen`. The repo badge SHALL
have alternative text `Repo Badge` and link to
`https://github.com/hydratic-applications/hydragen`. The stars badge SHALL have
alternative text `Stars Badge` and link to
`https://github.com/hydratic-applications/hydragen/stargazers`. The sponsors badge
SHALL have alternative text `Sponsors Badge` and link to
`https://github.com/sponsors/hydratic-applications`. The license badge SHALL have
alternative text `License Badge` and link to the relative path `LICENSE`.
The README SHALL retain its textual prototype warning, current limitations,
and Rust build guidance independently of the badges, and SHALL identify
`GPL-3.0-or-later` in its License section independently of the badge image.

#### Scenario: Follow badge links

- **WHEN** a reader follows a badge's link
- **THEN** the status badge navigates to the introduction and the language badge
  navigates to the local build instructions
- **AND** the dependencies badge navigates to Hydragen's deps.rs repository report
- **AND** the repo badge navigates to Hydragen's GitHub repository
- **AND** the stars and sponsors badges link to the repository's stargazers and
  the repository owner's GitHub Sponsors page, respectively
- **AND** the license badge opens the repository's `LICENSE` file rather than
  a badge SVG or third-party license summary

#### Scenario: External images are unavailable

- **WHEN** badge images cannot load or a reader uses a nonvisual reading tool
- **THEN** descriptive alternative text identifies each badge's meaning
- **AND** the README's prose still explains the prototype's limitations and how
  to build it with Rust
- **AND** its License section still identifies `GPL-3.0-or-later` and links to
  the local license text

### Requirement: Shortcut badge references with definitions at the bottom

The badge paragraph SHALL use `[![Status Badge]][Status]`,
`[![Rust Badge]][Rust]`, `[![Deps Badge]][Deps]`, `[![Repo Badge]][Repo]`,
`[![Stars Badge]][Stars]`, `[![Sponsors Badge]][Sponsors]`, and
`[![License Badge]][License]`, with one linked image per source line in the same
paragraph. All fourteen image and destination reference definitions SHALL appear
together at the bottom of the README, after its existing content. The
`Deps Badge` definition SHALL identify the SVG URL and the `Deps` definition
SHALL identify the deps.rs report page.
The `Repo Badge` definition SHALL use
`https://img.shields.io/badge/repo-hydratic--applications%2Fhydragen-181717?style=flat-square&logo=github&logoColor=white`
and the `Repo` definition SHALL identify the GitHub repository page.
The `License Badge` definition SHALL use
`https://img.shields.io/badge/license-GPL--3.0--or--later-blue?style=flat-square`
and the `License` definition SHALL identify `LICENSE`.

#### Scenario: Inspect the README source

- **WHEN** a maintainer reads the README Markdown
- **THEN** the badge paragraph contains shortcut image references wrapped in
  reference-style links, without inline URLs or repeated explicit image labels
- **AND** all badge image and destination definitions are grouped at the bottom

#### Scenario: Render the dependency reference link

- **WHEN** a Markdown renderer resolves `[![Deps Badge]][Deps]`
- **THEN** it displays the SVG identified by `Deps Badge` as an image linked to
  the report page identified by `Deps`, rather than to the SVG itself

#### Scenario: Render the repo reference link

- **WHEN** a Markdown renderer resolves `[![Repo Badge]][Repo]`
- **THEN** it displays the static `repo: hydratic-applications/hydragen` image linked to
  `https://github.com/hydratic-applications/hydragen`, rather than to the badge SVG

#### Scenario: Render the license reference link

- **WHEN** a Markdown renderer resolves `[![License Badge]][License]`
- **THEN** it displays the static `license: GPL-3.0-or-later` image linked to
  the local `LICENSE` file

### Requirement: Badges do not imply unsupported project guarantees

The README SHALL include only prototype-status, Rust-language, deps.rs
dependencies, GitHub repo, dynamic stars, dynamic sponsors, and the established
`GPL-3.0-or-later` license badges. It SHALL NOT add badges claiming a CI result,
coverage level, published version, minimum supported Rust version, or platform
support. The license claim SHALL agree with the project application notice and
Cargo package metadata. Badge rendering SHALL NOT introduce any network
requirement for building, testing, or running Hydragen.

#### Scenario: Review badge claims

- **WHEN** a maintainer reviews the updated README
- **THEN** the badges describe the project's prototype status and Rust
  implementation, display deps.rs's dependency result, link to the GitHub
  repository, stargazers, and owner's sponsorship page, and identify the
  established GPLv3-or-later license without implying other unestablished
  guarantees or infrastructure

#### Scenario: Use Hydragen without badge service access

- **WHEN** Shields.io or deps.rs is unavailable
- **THEN** the badge addition does not affect building, testing, or running
  Hydragen because no implementation or build process fetches the images

### Requirement: Stable deps.rs URLs with an accepted unknown state

The dependencies image SHALL use
`https://deps.rs/repo/github/hydratic-applications/hydragen/status.svg?style=flat-square`.
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
`https://img.shields.io/github/stars/hydratic-applications/hydragen?style=flat-square&label=stars`.
The sponsors image SHALL use
`https://img.shields.io/github/sponsors/hydratic-applications?style=flat-square&label=sponsors`.
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
