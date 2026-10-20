## ADDED Requirements

### Requirement: Coverage workflow is limited to coverage responsibilities

`.github/workflows/coverage.yml` SHALL retain workflow name `Coverage` and contain
the `coverage`, `total`, `changed`, and `upload` jobs with display names
`Generate coverage report`, `Check total coverage`, `Check changed-line coverage`, and
`Upload main coverage to Codecov`. Normal `tests` and `msrv` jobs SHALL live in
the separate testing workflow. Instrumented tests SHALL remain part of report
production and SHALL NOT replace the normal testing result.

Coverage SHALL retain pull-request, main-push, and manual-dispatch events without
path filters, its existing coverage-prefixed PR/ref cancellation group, immutable
revision context, reviewed action pins, timeouts, commands, artifact identities,
thresholds, and N/A policy. Both gates and the uploader SHALL depend only on
successful `coverage` production within that same workflow run. No cross-workflow
artifact transfer or dependency on testing SHALL be introduced.

Only the existing eligible main uploader SHALL have `id-token: write`. All other
jobs across both workflows SHALL remain repository-content read-only. The
uploader SHALL retain its data-only validation, no-checkout/no-repository-code
boundary, and main-only OIDC policy. Regression checks SHALL cover both workflow
files without weakening existing artifact, privilege, or action-pin assertions.

The producer and uploader display names SHALL identify generation and upload as
distinct stages of the same report, not separate coverage measurements. Job ids
and artifact identities SHALL remain unchanged. The uploader SHALL consume the
producer's validated report without running tests or generating fresh coverage.

#### Scenario: A contributor inspects coverage results

- **WHEN** a PR triggers coverage
- **THEN** `Coverage` contains report production, total and changed-line assessment,
  and the skipped main-only upload job, not normal-test or minimum-compiler jobs
- **AND** both coverage gates consume only that run's validated report

#### Scenario: Testing fails while valid coverage evidence exists

- **WHEN** normal tests or MSRV fail but report production succeeds
- **THEN** coverage gates still assess their validated evidence independently
- **AND** eligible main uploading still depends only on report production

#### Scenario: A valid main report is below target after the split

- **WHEN** a main run produces a valid report below an applicable threshold
- **THEN** the producer succeeds, the failing coverage gate reports its own failure,
  and the existing isolated uploader remains eligible
- **AND** no testing result or cross-workflow artifact is required for uploading

#### Scenario: A maintainer distinguishes report generation from upload

- **WHEN** an eligible main run generates and uploads coverage
- **THEN** `Generate coverage report` produces the report once and
  `Upload main coverage to Codecov` sends that same validated report to Codecov
- **AND** uploading does not rerun tests, remeasure coverage, enforce another
  percentage gate, or imply successful Codecov processing or public report access

## MODIFIED Requirements

### Requirement: Coverage runs for pull requests and main

The repository SHALL provide a bounded read-only GitHub Actions job with id
`coverage` and display name `Generate coverage report` for `pull_request` events,
pushes to `main`, and manual dispatch, without path filters. It SHALL run
independently of normal tests and generate fresh LCOV once on Linux using stable Rust, matching
`llvm-tools-preview`, and `cargo-llvm-cov` 0.9.1 with
`--locked --workspace --all-targets`. Existing unit, CLI subprocess and Unix
pseudo-terminal targets SHALL remain included with bounded waits and child
cleanup. The default measurement scope, including inline unit-test code, SHALL
remain unchanged, without new production-source exclusions, nightly coverage,
platform guarantees, or runtime dependencies.

For a PR, the measured checkout SHALL be the immutable event head SHA, not a
synthetic merge commit. The comparison SHALL run from the merge base of the
immutable event base SHA and that head to the same head. Required history SHALL
be obtained without persisted checkout credentials. Missing revisions, an
unavailable merge base, or checkout/source identity mismatches SHALL be errors,
not not-applicable results or reasons to compare against a moving default branch.

#### Scenario: A pull request is opened or updated

- **WHEN** any PR, including a fork or documentation-only PR, triggers coverage
- **THEN** the producer measures its immutable head with the existing test targets
- **AND** success produces valid coverage for the tracked production sources,
  including executed library and CLI source lines
- **AND** the recorded comparison uses the immutable event base and measured head

#### Scenario: Main receives a commit

- **WHEN** a commit is pushed to `main`
- **THEN** the producer generates and retains a report for that main commit
- **AND** the report is eligible for the separate main-only upload job

#### Scenario: Required comparison history is unavailable

- **WHEN** a PR's immutable base or head cannot be resolved or has no usable merge base
- **THEN** coverage evidence preparation fails with a revision diagnostic
- **AND** no moving branch, default merge checkout or N/A substitutes for the comparison

#### Scenario: Instrumented tests or report generation fail

- **WHEN** any instrumented test, instrumentation, or validation step fails
- **THEN** the producer fails and dependent assessments do not claim success
- **AND** the independent normal-test result remains unchanged
- **AND** no test skip, stale profile, or source exclusion conceals the failure

### Requirement: Actions assess total and changed-line coverage

Within stable-test and coverage reporting scope, Actions SHALL own three quality
policies: `Run tests on stable Rust`, `Check total coverage`, and
`Check changed-line coverage`. This enumeration SHALL NOT exclude an additional
independent `Run tests on minimum supported Rust` compiler-compatibility result
with its own visible result. The two coverage jobs SHALL be sibling consumers
whose only job dependency is successful `Generate coverage report` production,
not normal tests, MSRV, one another or Codecov. Each SHALL independently revalidate the
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
account entitlement or a processed main coverage baseline.
`Generate coverage report` and `Upload main coverage to Codecov` SHALL be
pipeline-health jobs, not extra quality policies. Configuration SHALL NOT change
branch protection or required-check settings.

#### Scenario: Exactly 80% is covered

- **WHEN** validated applicable counts are 4 covered of 5 measured lines
- **THEN** that coverage gate passes the inclusive 80% minimum

#### Scenario: A below-target value rounds to 80%

- **WHEN** validated counts are 7,999 covered of 10,000 measured lines
- **THEN** that gate fails even if a formatted percentage displays 80%

#### Scenario: Overall coverage passes but changed-line coverage fails

- **WHEN** valid total counts are 85 of 100 and changed-line counts are 7 of 10
- **THEN** `Check total coverage` passes and `Check changed-line coverage` fails

#### Scenario: Changed-line coverage passes but overall coverage fails

- **WHEN** valid total counts are 75 of 100 and changed-line counts are 9 of 10
- **THEN** `Check total coverage` fails and `Check changed-line coverage` still runs and passes
- **AND** the valid report remains retained and eligible main uploading is unaffected

#### Scenario: Codecov cannot process reports

- **WHEN** Codecov is unavailable or no processed main baseline exists
- **THEN** valid local total and PR changed-line evidence still determines the Actions gates
- **AND** service failure does not create a replacement percentage or alter those results

#### Scenario: Minimum-compiler compatibility fails independently

- **WHEN** `Run tests on minimum supported Rust` fails but valid coverage evidence is produced
- **THEN** both coverage gates still determine their own results from that evidence
- **AND** the three stable-test/coverage policies do not prevent a separate MSRV result

### Requirement: Authenticated uploads have explicit failure and trust boundaries

Only a push to `main` or manual dispatch on `main` SHALL upload a validated report
to Codecov using GitHub OIDC in a separate job depending only on successful report
production. No PR SHALL upload, including same-repository, bot and fork PRs. Only
the upload job SHALL have `id-token: write`; all other jobs SHALL be read-only.
The workflow SHALL NOT grant repository/PR/status write permission, execute PR
code through `pull_request_target`, introduce a privileged `workflow_run` bridge,
or expose stored credentials to contributed code. Actions SHALL use reviewed full
commit SHA pins; secrets SHALL NOT appear in files, URLs or logs.

The uploader SHALL use a fresh runner without checkout, Cargo, repository scripts,
build caches or executable artifact consumption. It SHALL download only an
explicitly named current-run report artifact to a dedicated directory and treat
it as data. A workflow-inline validator SHALL check allowed filenames, regular
file types, sizes, schema, independently expected repository/run/attempt/event/
commit context, inventory metadata and report digest before upload. It SHALL
accept only canonical line-only LCOV with summaries equal to its unique line
records, not raw LLVM grouped summaries or non-line metadata. It SHALL
reject unexpected files, symlinks, stale context or altered contents. The official
pinned uploader SHALL use OIDC, explicit file/commit/branch/slug, disabled report
search, enabled integrity checks, and `plugins: noop`.

Root Codecov configuration SHALL set `codecov.disable_default_path_fixes: true`
to preserve the validated repository-relative LCOV source paths during processing.
An uploader-side file inventory from the temporary upload directory SHALL NOT
rewrite or discard those paths merely because that directory has no production
source checkout. This SHALL retain the isolated data-only upload and strict
artifact validation boundaries.

Authentication/upload failures SHALL fail only the upload job, not rewrite the
test or coverage gate results. A valid below-target report SHALL remain eligible
because neither gate is an upload dependency.

#### Scenario: A validated main report is ready

- **WHEN** an eligible main run produces a valid report, even below 80%
- **THEN** the isolated uploader validates that run's named data artifact and uses OIDC
- **AND** it uploads only the explicit report for the expected commit, main branch and repository
- **AND** it neither checks out nor executes repository or artifact code

#### Scenario: Uploader inventory contains unrelated upload files

- **WHEN** validated LCOV contains repository-relative `src/` paths but the isolated
  uploader's file inventory contains only upload files rather than production sources
- **THEN** Codecov's default path rewriting is disabled and the validated source paths are preserved
- **AND** no source is discarded or remapped to that unrelated inventory, and no checkout is added

#### Scenario: Authentication or uploading fails

- **WHEN** Codecov authentication or uploading fails after valid production
- **THEN** the upload job fails visibly with recovery guidance
- **AND** `Run tests on stable Rust`, `Check total coverage`, and
  `Check changed-line coverage` retain independent results
- **AND** no stored token, broadened permission or ignored error hides the failure

#### Scenario: A pull request produces coverage

- **WHEN** any same-repository, bot or fork PR produces a valid report
- **THEN** all applicable local Actions gates run and the report is retained
- **AND** uploading is skipped with an explanation, without OIDC issuance to PR jobs

#### Scenario: Artifact identity or contents do not match

- **WHEN** an upload artifact has unexpected data, a wrong run/attempt/event/commit,
  an invalid digest, a symlink or an invalid schema
- **THEN** validation fails before the Codecov action is invoked
- **AND** no downloaded content is executed to validate or repair it

### Requirement: Changed-line applicability requires valid immutable diff evidence

For PR events, changed-line assessment SHALL use pinned `diff-cover` 10.6.0 with
an explicit `--diff-file` describing the recorded merge-base-to-measured-head
comparison and JSON output. The adapter SHALL validate integer counts and per-file
line sets against LCOV, the actual diff and changed source paths before exact
threshold arithmetic, explicitly checking root and per-source JSON object shapes.
Git comparison SHALL force textual hunks with `--text` and disable external diff
programs and text conversion, so Git attributes cannot suppress source changes.
It SHALL NOT rely on rounded display/`--fail-under`, moving
defaults, or tool configuration that changes refs or excludes sources. Missing
coverage or path mismatches SHALL fail, not become N/A.

Only a valid PR comparison with zero applicable measured executable lines SHALL
produce successful explicit N/A, without a numerical 100%. Deleted lines SHALL
not create measured head lines; added and renamed paths SHALL resolve to the head.
Main pushes and manual non-PR runs SHALL report changed-line N/A by event policy
after report validation, without inventing a PR comparison.

#### Scenario: A valid PR has no applicable executable changes

- **WHEN** report, inventory, refs, diff and path validation succeed and no changed measured lines apply
- **THEN** `Check changed-line coverage` succeeds with explicit not-applicable wording
- **AND** it reports no fabricated numerical 100% or numerical 0% failure

#### Scenario: An apparently empty comparison has missing evidence

- **WHEN** a report, ref, expected source or changed-source mapping is missing or invalid
- **THEN** changed-line assessment fails rather than claiming N/A

#### Scenario: Git attributes mark changed source as binary

- **WHEN** a tracked source has changed measured lines and attributes such as `-diff`
- **THEN** the immutable comparison still supplies textual source hunks
- **AND** uncovered changed lines are assessed, not silently reported as N/A

#### Scenario: A main or manual run has no PR event

- **WHEN** validated report data belongs to a main push or manual non-PR run
- **THEN** changed-line policy reports N/A because the event has no PR comparison
- **AND** total coverage still applies independently

### Requirement: Report production does not enforce coverage percentages

`Generate coverage report` SHALL produce, validate and retain evidence without
either percentage decision or upload privilege. Low coverage alone SHALL NOT fail the
producer or discard valid data. Instrumentation, test, report generation,
validation and artifact-retention failures SHALL remain failures. A producer
success SHALL mean valid retained report data, not passage of coverage policies
or successful Codecov uploading.

#### Scenario: A valid report is below a target

- **WHEN** production and retention succeed but total or changed-line coverage is below 80%
- **THEN** `Generate coverage report` succeeds and both consumers can independently assess it
- **AND** the report remains inspectable regardless of either gate's result

#### Scenario: Evidence cannot be produced or retained

- **WHEN** generation, validation or artifact retention fails
- **THEN** the producer is unsuccessful and dependent assessments do not claim a pass
- **AND** independent normal tests still run

### Requirement: Coverage setup is reproducible without affecting ordinary use

README Development SHALL include the local LCOV command and link to a focused
reference explaining measurement scope, the separate `Testing` and `Coverage`
workflows, the stable-test and compiler-compatibility results, supporting
report/upload jobs, exact inclusive 80% policy, N/A, immutable comparisons,
main-only OIDC, fork behaviour, report access and shield troubleshooting. CI
documentation SHALL identify `Run tests on stable Rust` and
`Run tests on minimum supported Rust` as independent executions of the Rust test
suite on different compilers in `Testing`, with `Check total coverage`,
`Check changed-line coverage`, and their supporting report/upload jobs in
`Coverage`. It SHALL explain that each workflow
supports manual dispatch separately; dispatching one does not run the other.
The README, coverage reference, and CI-related entries in the going-public
checklist SHALL agree about this separation and use all six action-led display
names without changing job ids, gates, upload rules, or required-check settings.
The focused reference SHALL explain that report generation runs once and the
gates and eligible uploader reuse the same report;
it SHALL distinguish generating a report from uploading it to Codecov.

The focused reference SHALL identify that the `tests` and `msrv` jobs moved out of
`Coverage` and all six job ids remain stable. It SHALL map the old/new labels:
`Tests` to `Run tests on stable Rust`, `MSRV` to `Run tests on minimum supported Rust`,
`Coverage report` to `Generate coverage report`, `Total coverage` to
`Check total coverage`, `Changed-line coverage` to `Check changed-line coverage`,
and `Codecov main report` to `Upload main coverage to Codecov`.
It SHALL direct maintainers to review external workflow/path references and actual
check identities before changing branch-protection or ruleset configuration.
Documentation SHALL NOT promise universal compatibility of external references
or automatically change those settings as part of the workflow split and renaming.

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
- **THEN** `Run tests on stable Rust` and `Run tests on minimum supported Rust` are
  identified with `Testing`, and `Check total coverage` and
  `Check changed-line coverage` with `Coverage`, with distinct meanings
- **AND** supporting report/upload jobs are not presented as extra quality policies
- **AND** their action-based names distinguish generating one report from uploading it

#### Scenario: A maintainer follows manual-run guidance

- **WHEN** a maintainer wants both compiler-test and coverage results manually
- **THEN** the focused reference explains how to select each workflow separately
- **AND** it does not claim a Coverage dispatch also runs the normal-test or minimum-compiler jobs

#### Scenario: A maintainer reviews existing required-check references

- **WHEN** a repository rule or integration refers to the former workflow arrangement
- **THEN** the migration guidance directs review of actual emitted check names and
  workflow paths rather than assuming every context changes or remains compatible
- **AND** this change does not automatically alter branch protection or repository rules

#### Scenario: External account setup is incomplete

- **WHEN** private repository access, entitlement, or live service verification is missing
- **THEN** the remaining prerequisite is reported explicitly
- **AND** implementation does not change visibility, billing, or branch protection
  to conceal it
