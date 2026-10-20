## MODIFIED Requirements

### Requirement: Actions assess total and changed-line coverage

Within stable-test and coverage reporting scope, Actions SHALL own three quality
policies: `Tests`, `Total coverage` and `Changed-line coverage`. This enumeration
SHALL NOT exclude an additional independent `MSRV` compiler-compatibility check
with its own visible result. The two coverage jobs SHALL be sibling consumers
whose only job dependency is successful `Coverage report` production, not normal
tests, MSRV, one another or Codecov. Each SHALL independently revalidate the
report against expected event/run/repository/revision context and its verified
measured checkout. For positive measured integer line count `N` and integer
covered count `C` with `0 <= C <= N`, each applicable gate SHALL pass exactly when
`5 * C >= 4 * N`. Display rounding SHALL NOT affect policy. A zero total
denominator SHALL be an error. Neither coverage result SHALL compensate for or
suppress the other.

Root Codecov configuration SHALL retain `main` and disable project and patch
statuses with `coverage.status.project: false` and `coverage.status.patch: false`.
Codecov SHALL serve report browsing and the main-specific README shield, not
percentage enforcement. Local gates SHALL NOT depend on Codecov processing,
account entitlement or a processed main coverage baseline. `Coverage report` and
`Codecov main report` SHALL be pipeline-health jobs, not extra quality policies.
Configuration SHALL NOT change branch protection or required-check settings.

#### Scenario: Exactly 80% is covered

- **WHEN** validated applicable counts are 4 covered of 5 measured lines
- **THEN** that coverage gate passes the inclusive 80% minimum

#### Scenario: A below-target value rounds to 80%

- **WHEN** validated counts are 7,999 covered of 10,000 measured lines
- **THEN** that gate fails even if a formatted percentage displays 80%

#### Scenario: Overall coverage passes but changed-line coverage fails

- **WHEN** valid total counts are 85 of 100 and changed-line counts are 7 of 10
- **THEN** `Total coverage` passes and `Changed-line coverage` fails

#### Scenario: Changed-line coverage passes but overall coverage fails

- **WHEN** valid total counts are 75 of 100 and changed-line counts are 9 of 10
- **THEN** `Total coverage` fails and `Changed-line coverage` still runs and passes
- **AND** the valid report remains retained and eligible main uploading is unaffected

#### Scenario: Codecov cannot process reports

- **WHEN** Codecov is unavailable or no processed main baseline exists
- **THEN** valid local total and PR changed-line evidence still determines the Actions gates
- **AND** service failure does not create a replacement percentage or alter those results

#### Scenario: Minimum-compiler compatibility fails independently

- **WHEN** `MSRV` fails but valid coverage evidence is produced
- **THEN** both coverage gates still determine their own results from that evidence
- **AND** the three stable-test/coverage policies do not prevent a separate MSRV result

### Requirement: Coverage setup is reproducible without affecting ordinary use

README Development SHALL include the local LCOV command and link to a focused
reference explaining measurement scope, the three stable-test/coverage policies,
supporting report/upload jobs, exact inclusive 80% policy, N/A, immutable
comparisons, main-only OIDC, fork behaviour, report access and shield
troubleshooting. CI documentation SHALL identify `MSRV` as an additional
independent compiler-compatibility check rather than describe the former three
as the complete CI set. The README, coverage reference, and CI-related entries in
the going-public checklist SHALL agree about this separation without changing
existing status names, gates, upload rules, or required-check settings.
The existing actual token-free main-specific coverage shield URL and appearance
SHALL remain unchanged; no static coverage number or token SHALL replace
unavailable data. Documentation SHALL NOT promise that Codecov's processed
display exactly matches LLVM's local policy scope or that public visibility
guarantees service features.

Ordinary Rust builds, tests and offline CLI use SHALL NOT require coverage tools,
Python helper tests, `diff-cover`, Codecov, Shields.io or upload authentication.
The explicit helper test command SHALL be documented separately. Python policy
assets and tests SHALL live outside `openspec/`; source, build scripts and tests
SHALL NOT load or embed OpenSpec files. `diff-cover` SHALL be pinned to 10.6.0
in an isolated CI environment and remain optional for developer coverage work.
Its explicit TOML configuration SHALL have parser support, including on Python
3.10, where that support is supplied by the optional `toml` dependency extra.

#### Scenario: A developer generates coverage locally

- **WHEN** the documented optional tool installation and LCOV command are followed
- **THEN** a developer can generate the report without uploading or authenticating
- **AND** the reference explains Linux/default-scope limitations and Actions-owned policy

#### Scenario: A developer uses the ordinary workflow

- **WHEN** coverage tools or report services are unavailable
- **THEN** normal Rust build/test commands and CLI use gain no service dependency
- **AND** helper tests remain a separate explicit development/CI command

#### Scenario: Main report or anonymous shield access is unavailable

- **WHEN** a processed main report or token-free shield is not accessible
- **THEN** documentation identifies report access, processing and cache checks separately
- **AND** no fabricated percentage, billing change or visibility change conceals the limitation

#### Scenario: A developer checks CI compatibility and coverage results

- **WHEN** a developer reads the README, coverage reference, or CI checklist entries
- **THEN** stable `Tests`, `MSRV`, and both coverage gates have distinct meanings
- **AND** supporting report/upload jobs are not presented as extra quality policies

#### Scenario: External account setup is incomplete

- **WHEN** private repository access, entitlement, or live service verification is missing
- **THEN** the remaining prerequisite is reported explicitly
- **AND** implementation does not change visibility, billing, or branch protection
  to conceal it
