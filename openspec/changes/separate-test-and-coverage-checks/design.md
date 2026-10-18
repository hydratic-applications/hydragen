## Context

PR #8 already contains `configure-codecov`: one GitHub Actions `coverage` job runs
instrumented tests, validates an LCOV report, and uploads it with OpenID Connect
(OIDC), GitHub's short-lived identity authentication. Codecov's project and patch
statuses currently use automatic targets based on earlier coverage. The first
private upload worked, but comparisons initially lacked a main-branch baseline.

The maintainer now wants independent test feedback and fixed minimums of 80% for
both total coverage and changed-line coverage, on this same branch and PR. This
follow-up supersedes the initial no-fixed-target decision, not the established
tooling, security, or README shield decisions.

This supports the foundations' DX and visibility values. It does not settle any
unresolved product question in `openspec/design-questions.md`, or require changes
to FOUNDATIONS.md or the glossary.

## Goals / Non-Goals

**Goals:** Separate test correctness from report generation/service failures;
use Codecov consistently for independent overall and changed-line minimums of
80%; retain useful reports for below-target commits without duplicate gates.

**Non-Goals:** New platforms, branch/doctest coverage instrumentation, source or
test-gap refactors, new credentials, fork uploads, required-check settings,
publication, billing changes, or a new coverage provider.

## Decisions

### Two independent Actions jobs with explicit responsibilities

Keep `.github/workflows/coverage.yml` as the shared event entry point, with stable
job ids `tests` and `coverage`, and explicit display names `Tests` and
`Coverage report`, respectively. Both run for PRs, pushes to `main`, and
manual dispatch without path filters, under the existing bounded timeout and
per-PR/ref concurrency policy. Run them independently, not with `needs: tests`:
the test result must remain visible even when coverage tooling or Codecov fails.

The `tests` job checks out without credential persistence, installs stable Rust,
and runs `cargo test --locked`, including normal unit, integration, terminal, and
documentation tests. It has only `contents: read`, no OIDC permission, uploader,
coverage threshold, or dependency on Codecov. Any failing test fails this job.

The `coverage` job retains the current instrumented workspace/all-targets run,
LLVM tools, pinned installer/uploader, report validation, and job-scoped OIDC
permission. Its instrumented tests can still fail: separating checks must not
hide invalid coverage evidence or force tests to pass. A coverage failure must
not rewrite the independent test result.

This runs the suite normally and under instrumentation rather than sharing
compiled artifacts between jobs. That modest duplication keeps test feedback
independent and setup simple; parallel execution avoids serialising two builds.
A single instrumented test producer plus report-consumer jobs would need raw
profiles, matching binaries/toolchains, and artifact trust rules without adding
enough benefit for this small package.

### Keep report production separate from percentage decisions

The maintainer selected Codecov for both percentage assessments, rather than a
local gate plus an overlapping Codecov project check. The report job only runs
instrumented tests, validates fresh report data, and uploads eligible reports.
It can pass with low coverage: that means report production/upload worked, not
that coverage met either minimum. Codecov's statuses make those decisions.

Generate and validate the fresh LCOV report as today. Upload eligible valid
reports regardless of the percentage. Do not add `--fail-under-lines`, a custom
coverage calculator, badge scraping, or a second percentage-enforcing job. This
avoids duplicate coverage-policy decisions and preserves low-coverage reports.

Instrumented test, report-validation, authentication, and upload failures still
fail the report job; do not suppress them with `continue-on-error`. Never assess
stale data or claim that an invalid/missing report is 0%, 100%, or a valid pass.
On forks, skip the upload as before. Consequently those PRs have test/report
results but no Codecov percentage assessments; changing that requires a separate
secure upload decision, not a silent local fallback.

### Give total and changed-line Codecov checks independent 80% targets

Set both `coverage.status.project.total-coverage` and
`coverage.status.patch.changed-lines` to `target: 80%`, `threshold: 0`, and
`informational: false`. Treat 80% as a minimum with no configured tolerance, not
an allowed regression. Retain `main` as the branch. The project check evaluates
the full uploaded project; patch evaluates executable lines added or modified by
the PR. Neither can compensate for failure of the other. A drop from 96% to 85%
passes the total target, while a 70%-covered patch still fails its own target.

GitHub therefore shows `Tests`, the supporting `Coverage report` job, and
Codecov's separate project and patch assessments where available. There are three
quality requirements (tests, total coverage, changed-line coverage), plus a
report-pipeline health result, not two total-coverage gates. A successful report
job does not replace pending or failed Codecov assessments. Changing the targets
does not make them mandatory for merging; branch protection remains untouched.

### Use descriptive check names within each provider's supported naming

The Actions job ids are implementation identifiers; their visible `name` values
must explain the responsibility without requiring developers to know Codecov
terminology. Use `Tests` and `Coverage report`. Keep threshold values in the check
details and documentation, rather than rename checks whenever a target changes.

Codecov allows named project and patch status entries, but its documented status
contexts retain the provider/type prefixes. Name the entries `total-coverage` and
`changed-lines`, producing the documented contexts
`codecov/project/total-coverage` and `codecov/patch/changed-lines`. Disable each
section's `default` entry explicitly to avoid extra bare `codecov/project` or
`codecov/patch` assessments alongside the descriptive ones. Do not claim that the
Codecov configuration can replace its prefix with an arbitrary GitHub check name.
Verify the actual service-rendered names during rollout.

This reuses Codecov's existing changed-line assessment; it does not add another
patch job or a custom status-publishing wrapper just to rename it. Such a wrapper
would add write permissions, asynchronous polling, and failure handling solely
for presentation. No extra privileges or duplicate changed-line checks are needed.
There is no separate local total gate: both percentage decisions belong to the
named Codecov statuses, as requested by the maintainer.

The named entries, disabled defaults, and 80% targets were accepted by Codecov's
configuration validator during planning. This verifies configuration validity,
not live check labels or threshold outcomes.

For PRs with no added or modified executable lines, patch coverage is not
applicable; accept Codecov's neutral/skipped/no-applicable-lines result instead
of demanding a numerical patch percentage or inventing 100%. Missing reports,
missing baseline data, processing delays, and account limitations are distinct
from that case. Do not fabricate a patch pass when Codecov cannot assess it.
An eligible code-changing PR with a processed base report is needed to verify
both pass and fail outcomes; this configuration-only increment cannot prove them
from a documentation-only PR alone. Do not add unrelated product code just to
create a diff for the service.

### Keep the follow-up specifications ordered and documentation current

The `code-coverage-ci` capability exists in the pending `configure-codecov`
change, not yet in `openspec/specs/`. This change depends on that baseline:
synchronise its additions first, then this follow-up's modifications when
updating the main specifications. Do not rewrite earlier successful verification
as evidence for the new policy, or reintroduce automatic targets when finishing
the original change. No implementation may load assets from `openspec/`.

During implementation, revise README Development and `docs/coverage.md` to explain
test versus report-pipeline failures, both Codecov 80% targets, low-coverage
uploads, service/baseline limitations, and patch applicability. Keep the
token-free shield measuring actual `main`
coverage, not a static 80% claim. Earlier verification remains historical.

### Correct the input-loss test harness without changing CLI behaviour

Linux validation exposed an existing intermittent terminal test failure. The
maintainer authorised investigating and fixing it before continuing. The
`portable-pty` writer sends a newline and end-of-transmission byte when dropped;
the test intended to disconnect input without sending any answer. That newline
can accept the current prompt, including completing the format selector, or
advance to a later prompt before the disconnect is observed.

Use a plain independently owned duplicate of the input terminal's master
descriptor in this test. It closes without injecting keystrokes. Keep the
existing nonzero-exit, single diagnostic, and no-files-created assertions at
both the name prompt and format selector. Add captured output to the exit-status
assertion for useful failure diagnostics. This is a focused test-harness repair,
not a CLI behaviour change, test skip, or coverage-driven source refactor. No
Cargo dependency or foundation/glossary change is needed.

### Preserve scripted terminal keystrokes between prompt reads

The first hosted run after the split passed normal tests but failed the Unicode
correction test under instrumentation. The expected name was a BOM (U+FEFF);
the actual name also retained the previously rejected next-line whitespace
(U+0085). The input-loss repair passed in that run.

The `console` dependency switches to raw input for each key, then restores the
previous mode before returning to the prompt library. In the test's default
canonical (line-buffered) terminal mode, a scripted Backspace arriving between
reads can be consumed by the terminal rather than delivered to the prompt.
A separate terminal experiment confirmed that the correction's Backspace byte
was lost with that baseline and preserved with noncanonical input.

Set `stty -icanon -echo -isig min 1 time 0` in the test session's existing bounded
shell child before it launches the CLI. This makes scripted input byte-oriented,
disables terminal echo and signal interpretation, and keeps output newline
handling unchanged. Fail setup if `stty` fails. The CLI still uses cliclack's
normal input handling; tests retain their Unicode values, exact expected layer
manifest content, error assertions, and cleanup. Do not mask the issue with
delays, retries, filtering input, or weaker assertions. This test-only setup does
not establish a new production input policy.

GitHub confirmed that report validation and uploading were skipped after the
instrumented failure. The Codecov assessments are service-generated after an
eligible upload, not extra Actions jobs. Check their absence separately from
configuration validity and missing-base comparisons; do not create placeholder
success statuses to make four checks appear.

## Risks / Trade-offs

- **Two builds consume more runner work** → Keep one platform and run jobs in
  parallel; do not add raw-profile artifact handling before it is needed.
- **Codecov processing or a baseline is unavailable** → Leave percentage results
  explicitly unassessed/pending; do not treat a passing report job as a coverage
  pass or silently add duplicate local enforcement.
- **An instrumented test or infrastructure failure also fails coverage** → Keep
  step names and diagnostics clear; distinct checks do not mean suppressing errors.
- **Private/fork limits prevent percentage assessment** → Preserve trust
  boundaries and report the limitation; neither percentage is claimed to be
  enforced on a run that Codecov could not assess.

## Migration Plan

Implement on the existing branch/PR, validate job independence and fixed Codecov
targets, and observe both Actions checks on an authorised push. Verify Codecov
comparisons when base and changed-code reports are available. Do not merge or
change repository settings as part of implementation. Rollback restores the
single-job/automatic-target configuration in a reviewed change, not by discarding
unrelated work.

## Sources

- [cargo-llvm-cov threshold and report options](https://github.com/taiki-e/cargo-llvm-cov/blob/v0.9.1/README.md)
- [Codecov project and patch status configuration](https://docs.codecov.com/docs/commit-status)
