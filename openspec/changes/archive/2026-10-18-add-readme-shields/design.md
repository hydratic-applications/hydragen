## Context

`README.md` opens with a title, a short introduction, and an explicit prototype
warning. `Cargo.toml` identifies a Rust crate, but the repository has no declared
license or GitHub Actions workflow. The badge selection and visual treatment need
to be explicit before implementation; adding conventional badges indiscriminately
could imply capabilities or infrastructure that do not exist.

This is a documentation-only increment. It supports the foundations' emphasis on
clear presentation and visibility without changing product behavior. None of the
open questions in `openspec/design-questions.md` needs resolution here. No new
Hydragen terminology or changes to the foundations or glossary are needed.

## Goals / Non-Goals

**Goals:**

- Make prototype status and implementation language visible at a glance.
- Link to live dependency information from deps.rs, accepting `unknown` while
  the repository is private without requiring a later README edit for public access.
- Add a small, consistent badge row without displacing the introduction.
- Provide a direct link to the GitHub repository, including from copied or locally
  rendered versions of the README.
- Display live repository stars and the repository owner's sponsor count without
  maintaining numbers in the README.
- Preserve useful text and navigation when images are unavailable.

**Non-Goals:**

- Add CI, licensing, releases, package publication, coverage reporting, or badges
  claiming any of them.
- Display a Rust minimum supported version: the README's tested version is not a
  declared minimum supported version.
- Add metrics beyond stars and sponsors, generated badge assets, scripts, or build
  dependencies.
- Redesign or expand the rest of the README.
- Change repository visibility, supply private-repository credentials to deps.rs,
  or fix dependency findings as part of adding the badge.
- Enable GitHub Sponsors, change account settings, or create a funding configuration.

## Decisions

### Use three static Shields.io badges and three live badges

Use `status: prototype` and `language: Rust`, in that order. Both describe facts
already established by the README and source. Static badges avoid depending on a
release registry or workflow that has not been configured.

Append a dependencies badge served directly by deps.rs and linked to its report
for `hydratic-applications/hydragen`. deps.rs compares Cargo dependencies with
crates.io versions and RustSec advisories; it provides useful dependency
information without requiring publication of Hydragen or a CI workflow.

Include the live badge now even though the repository is private. Checked on
2026-10-18: the report returned HTTP 400 with “Failed to analyze repository,”
while the badge returned HTTP 200 and an SVG reading `dependencies: unknown`.
This is an accepted temporary state, not a release blocker. Once public access
allows analysis, the same URLs should report the result after service and image
caches refresh; this is not a guarantee of immediate analysis or service uptime.

After the dependencies badge, add a static `repo: GitHub` badge with the GitHub
logo, linked to `https://github.com/hydratic-applications/hydragen`. This provides
repository navigation without relying on GitHub API access or implying any
popularity metric. It works as an image while the repository is private; the
destination still requires GitHub permission until the repository is public.

Append dynamic `stars` and `sponsors` badges using Shields.io's
[GitHub Repo stars](https://shields.io/badges/git-hub-repo-stars) and
[GitHub Sponsors](https://shields.io/badges/git-hub-sponsors) integrations. Stars
measure `hydratic-applications/hydragen`; sponsors count the repository owner's
account, `hydratic-applications`, not this repository alone. Link stars to the
repository's stargazers and sponsors to the organization's GitHub Sponsors page.
Use the service integrations, not static numbers or a custom JSON endpoint.

Checked on 2026-10-18: the stars SVG read `stars: repo not found` while the
repository was private; the sponsors SVG read `sponsors: 0`. GitHub reported no
Sponsors listing for the organization. Include both badges now, accepting service
errors and the not-yet-available sponsorship destination. Making the repository
public or enabling sponsorship remains a separate user decision. The same URLs
are intended to serve future public data without a README edit, subject to service
availability and cache refreshes. Never replace unavailable data with a fabricated zero.

Alternatives: dynamic build, version, and license badges are familiar but lack
authoritative sources here. Additional metrics such as downloads remain outside
scope. Retaining prose alone would not satisfy the request for badges, while
hardcoded stars or sponsors would become stale rather than meet the dynamic requirement.

### Keep presentation compact and native to Markdown

Place the linked images directly below `# Hydragen`, separated from the existing
introduction by a blank line. Write one linked image per source line, in the same
Markdown paragraph, so they appear side by side where space allows and can wrap
naturally on narrow screens. Use explicit `style=flat-square` on all six images,
no custom HTML, and no fixed layout widths.

Use these image URLs:

- `https://img.shields.io/badge/status-prototype-orange?style=flat-square`
- `https://img.shields.io/badge/language-Rust-DEA584?style=flat-square&logo=rust&logoColor=white`
- `https://deps.rs/repo/github/hydratic-applications/hydragen/status.svg?style=flat-square`
- `https://img.shields.io/badge/repo-GitHub-181717?style=flat-square&logo=github&logoColor=white`
- `https://img.shields.io/github/stars/hydratic-applications/hydragen?style=flat-square&label=stars`
- `https://img.shields.io/github/sponsors/hydratic-applications?style=flat-square&label=sponsors`

The first, second, and fourth follow the [Shields.io static badge API](https://shields.io/badges/static-badge).
The third uses the [deps.rs repository badge API](https://github.com/deps-rs/deps.rs#badges),
which supports the same style name. Omit Ratatui's `path=ratatui` parameter:
Hydragen's `Cargo.toml` is at the repository root, not in a package subdirectory.
The fifth and sixth use the GitHub integrations above with explicit labels and
style, overriding the stars badge's default social style.
The requested `flat-square` style uses square corners and solid fills without
gradients, rather than the default rounded treatment. The shared style keeps the
row cohesive; orange distinguishes prototype maturity,
and the Rust badge uses a recognizable language label and logo. Text, rather than
color alone, conveys each fact. Large `for-the-badge` styling or a custom banner
would compete with the concise introduction.

### Use shortcut image references and keep definitions at the bottom

Use the requested shortcut image-reference syntax inside reference-style links
for all six badges. Keep this paragraph below the title:

```markdown
[![Status Badge]][Status]
[![Rust Badge]][Rust]
[![Deps Badge]][Deps]
[![Repo Badge]][Repo]
[![Stars Badge]][Stars]
[![Sponsors Badge]][Sponsors]
```

The image reference labels also become the alternative text: `Status Badge`,
`Rust Badge`, `Deps Badge`, `Repo Badge`, `Stars Badge`, and `Sponsors Badge`.
The dependency label identifies the
badge without hardcoding a passing result. Retain the prose explaining prototype status and
the Rust toolchain independently of the images.

Place all twelve reference definitions together at the very bottom of the README,
after the existing content:

```markdown
[Status Badge]: https://img.shields.io/badge/status-prototype-orange?style=flat-square
[Status]: #hydragen
[Rust Badge]: https://img.shields.io/badge/language-Rust-DEA584?style=flat-square&logo=rust&logoColor=white
[Rust]: #build-and-run-locally
[Deps Badge]: https://deps.rs/repo/github/hydratic-applications/hydragen/status.svg?style=flat-square
[Deps]: https://deps.rs/repo/github/hydratic-applications/hydragen
[Repo Badge]: https://img.shields.io/badge/repo-GitHub-181717?style=flat-square&logo=github&logoColor=white
[Repo]: https://github.com/hydratic-applications/hydragen
[Stars Badge]: https://img.shields.io/github/stars/hydratic-applications/hydragen?style=flat-square&label=stars
[Stars]: https://github.com/hydratic-applications/hydragen/stargazers
[Sponsors Badge]: https://img.shields.io/github/sponsors/hydratic-applications?style=flat-square&label=sponsors
[Sponsors]: https://github.com/sponsors/hydratic-applications
```

The status and Rust links lead to the introduction and build instructions using
existing anchors. The dependency image links to the deps.rs report, not the SVG.
The repo image links to the GitHub repository, not the badge image.
The deps.rs report and badge use the same repository and default root path; neither URL
needs to change solely because the repository becomes public. Keeping URLs out
of the badge paragraph makes the README source easier to scan and maintain than
inline URLs or repeated explicit image labels.

Use Markdown links around the images rather than Shields.io's `link` parameter:
the service documents that parameter as ineffective for Markdown images. Preserve
the existing prototype warning and Rust toolchain guidance; badges supplement
those explanations rather than replace them.

## Risks / Trade-offs

- External images can be blocked or either badge service can be unavailable → descriptive
  alternative text and the existing prose preserve all essential information.
- Static status can become stale → review the badge when changing the prototype
  warning; do not introduce automation for static labels.
- The deps.rs report currently fails for the private repository → accept the
  service's `unknown` badge without claiming healthy or audited dependencies.
  Do not add credentials or change visibility to make the badge work.
- Dependency findings or cached results can change independently of this checkout
  → link to the service's report, not a fixed claim of freshness or security.
- Stars may show `repo not found` while the repository is private, and the sponsor
  destination is unavailable before a Sponsors listing exists → accept these
  temporary limitations; do not supply tokens, enable sponsorship, change visibility,
  or present unavailable data as zero. An organization-wide sponsor count must not
  be described as funding specifically for Hydragen.
- Badge contrast or wrapping can disappoint in a real Markdown renderer → inspect
  the rendered introduction in light and dark themes at desktop and narrow widths,
  adjusting colors if needed without changing the labels or compact style.
- Rendering remote images requires a network request, possibly through a renderer's
  image proxy → keep this limited to README presentation; builds, tests, and CLI
  execution must not fetch badges.

## Migration Plan

Add the badge paragraph to `README.md`, check its image and navigation targets,
and review the rendered introduction. No migration, deployment configuration, or
Rust test changes are needed. Rollback is removal of the badge paragraph.

## Open Questions

None blocking this increment. Public repository access is a future user decision,
not a prerequisite for adding the deps.rs badge. This change does not commit to
creating data sources for any additional badges.
