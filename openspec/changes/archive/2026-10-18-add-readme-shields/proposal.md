## Why

The README explains Hydragen's prototype status and Rust implementation only in
prose. A small badge row can make those facts easier to scan, link to a deps.rs
dependency report, and give the introduction a more deliberate visual
presentation without obscuring what the tool currently does.

## What Changes

- Add two linked Shields.io badges immediately below the README title: project
  status (`prototype`) and implementation language (`Rust`).
- Add a deps.rs dependencies badge after them, linked to Hydragen's repository
  report. Include it now, accepting `unknown` while the repository is private;
  the same URLs can be used when the repository becomes public.
- Add a static `repo: GitHub` badge after the dependencies badge, linked to
  `https://github.com/hydratic-applications/hydragen`.
- Append dynamic Shields.io stars and sponsors badges for the repository and its
  owner, `hydratic-applications`, respectively. Accept unavailable live results
  while the repository is private or sponsorship is not configured; never
  substitute a hardcoded zero.
- Use a consistent, compact `flat-square` style, meaningful alternative text,
  and useful link destinations while keeping the introduction and limitations intact.
- Use shortcut image references inside reference-style links for all six badges,
  with all badge image and destination definitions at the bottom of the README.
- Keep essential information in prose so the README remains useful when external
  images cannot load.

## Capabilities

### New Capabilities

- `readme-badges`: Defines the README's badge selection, placement,
  accessibility, and accurate representation of the project.

### Modified Capabilities

None.

## Impact

- Implementation changes only `README.md`; no CLI, library, schema, or dependency
  changes are required.
- Rendered badge images load from `https://img.shields.io/` and `https://deps.rs/`.
  Building and running Hydragen remain offline-capable and independent of both.
- No repository visibility changes, credentials, or deps.rs configuration are
  included. The live dependency result is informational, not a security guarantee.
- Enabling GitHub Sponsors is outside scope. The sponsor count is organization-wide,
  not specific to Hydragen; its link targets the organization's sponsorship page.
- No CI, publication, release, license, coverage, or platform-support claims are
  introduced. Establishing those facilities is outside this change's scope.
