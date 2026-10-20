## Context

Before this change, `.github/workflows/coverage.yml` was the only CI workflow. It contained
six independently reported jobs: `Tests`, `MSRV`, `Coverage report`, `Total
coverage`, `Changed-line coverage`, and `Codecov main report`. The first two are
already independent of coverage in their job dependencies, but GitHub groups
them under `Coverage`. A failing minimum-compiler test therefore appears to be a
coverage concern even though it does not measure coverage.

The existing specifications already separate ordinary testing, compiler support,
and coverage policy. The implementation should expose that same distinction.
Tests also inspect the workflow source: coverage tests extract the inline upload
validator and check job wiring, while Rust-version tests extract the MSRV handoff
shell code. Moving YAML without updating these tests would break them or leave
the new workflow unprotected.

This is a focused maintenance change aligned with the foundations' DX,
efficiency, and visibility values. It does not change Hydragen's product model,
so neither `FOUNDATIONS.md` nor `GLOSSARY.md` needs revision. Question 8 in
`openspec/design-questions.md` provides the existing compiler-support decision;
its baseline and verification obligations remain unchanged. No unresolved product
question needs to be settled or newly recorded for this reorganisation.

## Goals / Non-Goals

**Goals:**

- Make workflow headings describe responsibilities and job names state their
  actions, distinguishing running tests, checking conditions, generating a report,
  and uploading that report.
- Keep stable correctness and MSRV compatibility together as independent testing
  results, separate from coverage measurement and policy.
- Preserve existing commands, events, revision selection, timeouts, action pins,
  permissions, thresholds, and artifact/upload safety.
- Keep regression tests and current documentation accurate after the move.
- Omit the main-only upload job entirely from PR results, rather than show it as
  skipped, without duplicating measurement/check implementation.

**Non-Goals:**

- Add formatting, linting, security scanning, new platforms, a matrix, an aggregate
  CI gate, or new required checks.
- Change Rust versions, test targets, coverage tools, helper-test placement by
  subject matter, coverage scope, thresholds, badges, or Codecov configuration.
- Add cross-workflow artifacts, event bridges, caches, repository/ruleset changes,
  or a dependency between Testing and coverage.
- Implement the change while proposing it, publish a branch, or dispatch remote CI
  without the maintainer's authorisation.

## Decisions

### Split by responsibility into Testing and Coverage

Add `.github/workflows/tests.yml` with workflow name `Testing`. Move the existing
`tests` and `msrv` jobs there. Use `.github/workflows/coverage.yml` named `Coverage`
for PR/read-only checks, with `coverage`, `total`, and `changed`. Add
`.github/workflows/coverage-main.yml` named `Coverage on main` for main pushes and
manual dispatch, containing those jobs plus `upload`. Keep the existing job ids
within their entry points and the action-led labels below.

`Testing / Run tests on stable Rust` means ordinary stable-compiler correctness
and helper regression tests; `Testing / Run tests on minimum supported Rust` means
the declared minimum compiler builds and passes its tests. These labels
describe workflow grouping, not a promise about the exact status-context strings
returned by every GitHub API or integration.

Renaming the existing workflow to `CI` would remove the misleading heading but
leave all concerns in one generic bucket. A workflow per job would fragment two
closely related compiler-test results. `Testing` and `Coverage` remain concise
responsibility headings; the job names beneath them describe the actual actions.

The existing Python coverage-helper regressions stay with `Run tests on stable Rust`:
they test helper correctness, not the measured percentage. Preserve their isolated
`diff-cover[toml]==10.6.0` setup. MSRV helper regressions stay with the minimum-Rust
check. The instrumented test run necessarily stays in the coverage producer; it is not a
replacement for the normal test job or a new correctness gate.

### Use action-led job names rather than ambiguous result nouns

Use these exact display names, keeping the existing job ids:

- `tests`: `Run tests on stable Rust` (formerly `Tests`).
- `msrv`: `Run tests on minimum supported Rust` (formerly `MSRV`).
- `coverage`: `Generate coverage report` (formerly `Coverage report`).
- `total`: `Check total coverage` (formerly `Total coverage`).
- `changed`: `Check changed-line coverage` (formerly `Changed-line coverage`).
- `upload`: `Upload main coverage to Codecov` (formerly `Codecov main report`).

Lead with the actual operation and name its subject. Both testing jobs run the
full Rust test suite, so both use `Run tests on ...`; their labels distinguish
current stable Rust from the declared minimum supported Rust. The minimum-version
job also retains its explicit locked build and metadata checks, but those
supporting steps do not make it a different kind of test result.

`Check` describes evaluating coverage against a threshold; `Generate` describes
producing reusable report data; `Upload` describes transferring it to the named
service. These verbs identify the primary responsibility, not every supporting
step: generation still validates its output, and successful test execution still
establishes a passing correctness result. Use parallel wording for the same
operation under different conditions rather than suggesting a distinction with
different verbs.

Spell out minimum supported Rust in the visible label rather than requiring
readers to know the MSRV abbreviation. Keep the exact compiler version in Cargo
metadata, not in a potentially stale display label. Coverage gate descriptions
continue to explain the exact inclusive 80% requirement; do not repeat the numeric
policy in names that would need separate maintenance.

Retaining noun-only names would preserve old labels but not address the user's
ambiguity. Using `Check minimum Rust compatibility` only for the minimum-version
job would obscure that both compiler jobs execute tests. Prefixing every job
with `Check` would wrongly suggest that generating
or uploading the report is another coverage threshold. Using only verbs such as
`Check` would omit the subject needed to understand a standalone failure result.

This naming decision supersedes the earlier plan to preserve testing and
coverage-gate labels. Preserve all six job ids and substantive requirements, but
review external references for all six changed display names.

### Keep report generation and upload as distinct actions on one report

Rename the `coverage` job from `Coverage report` to `Generate coverage report`,
and the `upload` job from `Codecov main report` to `Upload main coverage to Codecov`.
The producer runs instrumented tests once, validates and normalizes their coverage
data, and retains one report artifact. Both gates and the uploader consume that
same artifact; the uploader does not rerun tests or generate another measurement.
No additional job, report format, or artifact is introduced by the rename.

The producer runs on all existing events. The uploader runs only on an eligible
push or manual run on `main`, validating and sending that report to Codecov for
hosted browsing and the README badge. The names therefore expose the action,
destination, and main-only restriction rather than making both jobs sound like
different reports. The two percentage gates still enforce policy locally;
uploading is not another coverage gate.

Keeping separate jobs is intentional, not redundant: report generation executes
repository code without upload privileges, while the isolated uploader receives
only data and alone can request OIDC credentials. Merging them would weaken that
trust boundary. `Publish coverage` was considered, but `Upload main coverage to
Codecov` states the concrete operation without implying that Codecov has finished
processing it or that public report access is guaranteed.

Delta specifications update existing check-name references while preserving all
substantive pipeline requirements. Keep job ids and artifact names stable; use
display-name-aware regressions and align current documentation with the new labels.

### Keep triggers, immutable checkout, and cancellation independent

Testing declares `pull_request`, pushes to `main`, and `workflow_dispatch`.
Coverage declares `pull_request` and `workflow_dispatch`; Coverage on main declares
pushes to `main` and `workflow_dispatch`. None has path filters or triggers/waits
for another workflow. A PR schedules no upload workflow, not even a skipped job.
Manual dispatch runs only the chosen entry point. Select Coverage for read-only
checks, or Coverage on main on the `main` branch for uploading; a non-main manual
dispatch of the latter may run read-only checks but cannot upload. Do not add
`workflow_run`, `workflow_call`, or `pull_request_target`.

Use a testing-only environment value such as `TEST_HEAD` for
`${{ github.event.pull_request.head.sha || github.sha }}` and check out that value
in both testing jobs. Do not carry `COVERAGE_*` variables or artifact identities
into testing. Keep `persist-credentials: false`, full reviewed action pins,
`ubuntu-latest`, each existing 30-minute job bound, and explicit compiler selection.

Retain `coverage-<PR number or ref>` for Coverage, use `coverage-main-<ref>` for
Coverage on main, and `testing-<PR number or ref>` for Testing, all with
`cancel-in-progress: true`. A newer run cancels older work in its own entry point,
never another responsibility or a distinct manual entry point.

### Declare event-specific jobs and share read-only steps

The user approved this refinement after observing the initial PR's skipped upload
check. GitHub still creates a skipped result for a job guarded by `if:` in a
PR-triggered workflow. To omit it, the PR's declared job graph must exclude it.
This decision supersedes the original single-coverage-workflow arrangement and
its assumption that a skipped upload result is acceptable.

Keep the report and gate jobs concrete in both coverage entry points, with the
same display names, runner, timeouts, read-only permissions, immutable checkout,
and direct `needs: coverage` edges. Extract the producer's post-checkout steps to
`.github/actions/coverage-report/action.yml` and the consumers' post-checkout steps
to `.github/actions/coverage-check/action.yml`. These are local composite actions:
reusable groups of steps that execute inside the caller's existing job, not new
jobs, privileged workflows, or cross-run artifact consumers.

The check action accepts only `total` or `changed` as its mode. Validate that input
before artifact work, pass it as quoted environment data to the existing CLI,
and install isolated pinned diff-cover only for `changed`. Do not interpolate it
as shell source. Both entry points use the same two action definitions from their
immutable checkout, retaining one authoritative measurement/check implementation.
Small job declarations and expected event context remain explicit in each entry
point; regressions enforce their parity rather than duplicate business logic.

Composite actions are preferred over nested reusable workflows here because they
preserve the exact check labels without caller/callee prefixes and the direct
producer-to-gate/uploader dependency graph. A reusable workflow containing both
producer and gates would make a caller's uploader depend on the aggregate gate
outcome unless more infrastructure were added. Duplicating the entire pipeline
would avoid new abstractions but create two implementations to maintain.

The main uploader stays inline in `coverage-main.yml`, with its existing guard,
fresh data-only runner, validator and OIDC configuration. It never checks out or
calls a local composite action. The PR entry point and shared actions contain no
uploader or OIDC capability. No `workflow_run` bridge, stored token, second
measurement, or cross-workflow artifact handoff is introduced.

### Preserve the coverage pipeline and privilege boundary intact

Moving the first two jobs out must not alter report production, normalization,
artifact names or metadata, immutable comparison refs, exact inclusive 80% gates,
or changed-line N/A policy. The two gates and uploader still depend only on
`coverage`, within the same workflow and run. No cross-workflow artifact transfer
or reconstruction of report identity is needed.

Only the eligible main uploader retains `id-token: write`, with no checkout or
execution of repository/artifact code. All testing jobs retain content-read-only
permissions and need no secrets or coverage service. Upload or coverage failures
cannot suppress either compiler's test job, and test failures cannot suppress
coverage production or its eligible consumers.

### Update workflow regressions without weakening them

Point the Rust-version workflow tests at `tests.yml`, including extraction and
execution of the actual validated-version handoff. Extract job bodies by their
boundaries rather than relying on a following coverage job that no longer exists.

Extend workflow assertions across all three entry points and both shared actions:
exact workflow/job membership and names, event routing without filters, distinct
cancellation groups, immutable testing checkouts, bounds, action pins, permissions,
and no dependency or event bridge between workflows. Retain coverage artifact,
consumer dependency, main-only upload, and inline-validator regression coverage.
The shared check action defines pinned diff-cover installation once for changed-line
assessment, while Testing installs it once for helper regressions. Verify the
PR graph omits upload, both entry points call the same actions, the uploader
depends only on the producer, and the extracted actual validator and shared
check shell retain valid/invalid evidence and independent-outcome regressions.

Keep these checks in maintained tests outside `openspec/`. Use existing testing
facilities rather than add a YAML parser dependency solely for this move, and run
`actionlint` on all workflow entry points for syntax and expression validation.

### Document current organisation and treat external references as a migration

Revise the existing README Development explanation briefly, and use
`docs/coverage.md` for workflow membership, check meanings, independent manual
dispatch, and a short migration note. Update CI-related checklist entries without
rewriting publication history or adding a CI badge.

Preserving all job ids limits internal wiring changes, but all six display names,
testing workflow-qualified links, and required-workflow rules can refer to the
old arrangement. Review actual emitted check names and relevant
branch-protection/ruleset references,
when access is available. Record any needed maintainer action; do not rename,
remove, or weaken required checks automatically, and do not assert that every
required-check configuration necessarily changes.

## Risks / Trade-offs

- **Cross-workflow cancellation** → Distinct testing/coverage group prefixes plus
  regression assertions prevent one workflow cancelling the other.
- **Lost or duplicated checks after the move** → Assert exact membership, stable
  job ids, and the intended display names; review all entry points with actionlint.
- **Tests still assume the monolithic workflow** → Update both helper suites,
  preserve actual-shell/inline-validator execution tests, and rerun them.
- **Duplicated event declarations can drift** → Test all entry points' events and
  filters together; test parity of the thin coverage entry points and the shared
  composite action calls.
- **Separate manual runs require two dispatches** → Document this explicitly;
  independence is more valuable than introducing an orchestration workflow here.
- **Remote check references may be stale** → Inspect observed check identities and
  report required maintainer follow-up before merge; never change rules silently.

## Migration Plan

1. Move the testing jobs, establish independent entry points and cancellation,
   use the six action-led labels, and extract shared read-only steps. The uploader
   is declared only by the main entry point, retaining its same-run dependency.
2. Update the two helper-test suites and current CI documentation together.
3. Run actionlint on all workflows, the coverage-policy and Rust-version Python
   suites (with isolated pinned diff-cover), the offline Rust-version checker,
   and `cargo test --locked`. Record commands and outcomes in this change's
   implementation verification record.
4. On an authorised ordinary PR push, inspect both workflow runs for the same
   immutable PR head and record their URLs, workflow/job membership, compiler
   results, independent coverage results, and the absence of any upload check or
   Coverage on main run. Inspect shared-step execution and artifact identity. Do not treat a legitimately failing
   coverage gate as evidence that testing failed. Do not create failing commits,
   change repository settings, or dispatch uploads merely to exercise this move.
5. Review applicable external workflow/check references with the maintainer. If
   access or live runs are unavailable, record that evidence gap rather than claim
   live verification. Rollback, if necessary, restores the two jobs and their
   tests/docs to the single workflow and the former job display labels
   without changing code or coverage policy; coordinate external references
   separately if they were updated. Main publication cannot be observed before
   the new main entry point is present on `main`; record that post-merge evidence
   gap explicitly rather than dispatching another branch or changing trust rules.

## Open Questions

No product or implementation decisions block this proposal. Existing external
references to workflow names or paths need inspection during implementation;
their presence is an operational fact, not a reason to redesign the check split.
