# CI workflow separation verification

## Scope and environment

Verified locally on 2026-10-20 in branch `gentle-koala`, against uncommitted changes
based on `a6da2b6498d38cc3959485846797bd412479f75d`. No implementation revision has
been committed or pushed, and no pull request has been opened for this change.

- Platform: Apple Silicon macOS 26.5.2 (`aarch64-apple-darwin`).
- Rust/Cargo: Homebrew 1.96.0; rustc commit
  `ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96`, LLVM 22.1.6.
- Python: 3.14.6.
- actionlint: 1.7.12.
- diff-cover: 10.6.0 with its TOML extra, installed in the ignored
  `target/coverage-venv` environment. No Cargo/runtime dependency was added.

These local results are not Linux acceptance evidence or a new verification of
the minimum compiler. The workflow still selects that compiler from validated
Cargo metadata and runs its existing locked build and full tests in CI.

## Workflow preservation review

Compared all six job bodies against `git show HEAD:.github/workflows/coverage.yml`
using a read-only Python assertion script. After removing each job's display-name
line, the coverage jobs are unchanged. The two moved testing jobs are unchanged
after accounting for `env.TEST_HEAD` replacing `env.COVERAGE_HEAD` and their
checkout step labels referring to a test head instead of a measured head. The
coverage workflow's entire pre-job header is byte-identical to its predecessor.

The implementation retains every job id, action pin, command, runner, job timeout,
coverage artifact identity, consumer dependency, threshold, and upload restriction.
Testing has its own head context and `testing-` cancellation group; coverage keeps
its `coverage-` group. Both workflows have the same three trigger events without
filters, and neither waits for or dispatches the other.

Expanded regressions assert the exact six display labels, workflow membership,
event blocks, cancellation groups, immutable checkouts, stable-compiler test and
helper commands, job bounds, read-only permissions, action pins, single coverage
measurement, shared report use, and the main-only uploader's isolation. Existing
tests still execute the actual workflow-inline upload validator and the actual
MSRV handoff shell code. Neither suite reads OpenSpec files.

## Local commands and results

All commands run from the checkout root:

```sh
actionlint .github/workflows/tests.yml .github/workflows/coverage.yml
python3 -m unittest discover -s tests/rust_version -v
python3 scripts/check_rust_version.py
cargo test --locked
openspec validate separate-ci-workflow-concerns --strict
git diff --check
```

- actionlint: passed for both workflows, with no diagnostics.
- Rust-version helper suite: 24 passed, including actual Actions handoff execution.
- Offline Rust-version consistency check: passed, reporting MSRV 1.89.0.
- Rust suite: 572 passed, zero failed/ignored, including subprocess and real-terminal
  tests; the documentation-test target also succeeded with zero examples.
- Strict OpenSpec validation: passed.
- Whitespace check: passed.

Installed the optional isolated helper dependency:

```sh
python3 -m venv target/coverage-venv
target/coverage-venv/bin/python -m pip install 'diff-cover[toml]==10.6.0'
```

The first coverage-helper invocation used the inherited macOS temporary root:

```sh
DIFF_COVER="$PWD/target/coverage-venv/bin/diff-cover" \
  python3 -m unittest discover -s tests/coverage -v
```

It passed 41 of 42 tests. The existing absolute-source-path fixture failed because
Python's temporary path began with `/var/`, while Git returned its canonical
`/private/var/` spelling. The source-path validator rejects that mismatched prefix.
Neither this fixture nor the validator was changed to conceal the failure.

Reran the entire suite using the pre-existing, approved canonical temporary directory:

```sh
TMPDIR=/private/var/folders/p9/02qtjw2138dd0jbmmjb0k45r0000gp/T/opencode \
  DIFF_COVER="$PWD/target/coverage-venv/bin/diff-cover" \
  python3 -m unittest discover -s tests/coverage -v
```

All 42 passed, including real diff-cover integration and inline-validator tests;
zero were skipped. After tightening the trigger-block equality assertion, reran
the affected four workflow tests successfully:

```sh
python3 -m unittest discover -s tests/coverage -k WorkflowTests -v
```

## Documentation review

Reviewed README Development, the coverage reference, and CI-related publication
checklist entries from a first-time reader's perspective. The workflow grouping,
parallel compiler-test labels, percentage checks, single-report generation/reuse,
upload restrictions, separate manual dispatches, and six-name migration mapping
are explicit. Existing local commands, badge claims, compiler guidance, coverage
policies, and publication boundaries remain intact.

No foundations, glossary, or unresolved-product-question update is warranted:
this change reorganises CI presentation and lifecycle without changing the product
model or compiler-support policy.

## Read-only GitHub migration review

Queried the existing remote configuration without changing it:

```sh
gh api repos/hydratic-applications/hydragen/branches/main/protection
gh api 'repos/hydratic-applications/hydragen/rulesets?includes_parents=true'
gh api repos/hydratic-applications/hydragen/commits/main/check-runs \
  --jq '.check_runs[] | {name, status, conclusion, head_sha, details_url}'
```

The branch-protection endpoint returned HTTP 404 with the explicit message
`Branch not protected`. The ruleset list, including parent rulesets, returned `[]`.
No required-check or required-workflow rule was found there to migrate at review.
This is a dated observation, not a guarantee about future rules or other external
integrations.

The existing successful [main run 39350085845](https://github.com/hydratic-applications/hydragen/actions/runs/39350085845)
at `a6da2b6498d38cc3959485846797bd412479f75d` emitted the six old leaf names:
`Tests`, `MSRV`, `Coverage report`, `Total coverage`, `Changed-line coverage`, and
`Codecov main report`. Those actual API names are distinct from explanatory
workflow-qualified labels. All six display names change in this implementation;
the internal ids remain stable. New emitted identities must still be observed
after publishing the implementation.

## Initial PR verification before the upload-visibility refinement

The authorised implementation commit `ea234d484a24c541ddec68fba8f04b3d047064dd`
was pushed and [PR #16](https://github.com/hydratic-applications/hydragen/pull/16)
opened. Its [Testing run 39354854343](https://github.com/hydratic-applications/hydragen/actions/runs/39354854343)
passed both compiler jobs, and its [Coverage run 39354854331](https://github.com/hydratic-applications/hydragen/actions/runs/39354854331)
passed generation and both gates. The uploader appeared as skipped. That observed
clutter motivated the user-approved refinement below; it is not the final
job-graph acceptance evidence.

At the original local-verification checkpoint, before that PR was opened, no
commit, push, PR creation, manual
workflow dispatch, upload, merge, archive, repository-setting change, or
coverage-policy change had been performed.

## Approved refinement: omit uploading from PR results

The user approved separate PR/main coverage entry points with shared read-only
implementation. `Coverage` now has only the producer and two gates; `Coverage on
main` declares those jobs plus the isolated uploader. The shared producer/check
steps live in two local composite actions, preserving direct job names and
producer-only dependency edges without nested workflow calls or cross-run data.
This explicitly supersedes the earlier expectation of a skipped upload check.

Read-only source comparison against `ea234d4` confirmed that the complete uploader
job body is unchanged in `coverage-main.yml`. All three read-only job declarations
are identical between entry points, including names, runner, bounds, permissions,
credential-free full-history checkout, and calls to the same local actions.
Only their trigger/context/cancellation headers differ. Testing is unchanged.
The extracted producer steps also match the prior job byte-for-byte after
normalising indentation and removing the composite action's explicit Bash shell
declarations; the added mode adapter is exercised by the actual-shell tests.

Local verification of the refinement, on the same macOS/Python/Rust environment:

```sh
actionlint .github/workflows/tests.yml .github/workflows/coverage.yml .github/workflows/coverage-main.yml
TMPDIR=/private/var/folders/p9/02qtjw2138dd0jbmmjb0k45r0000gp/T/opencode \
  DIFF_COVER="$PWD/target/coverage-venv/bin/diff-cover" \
  python3 -m unittest discover -s tests/coverage -v
python3 -m unittest discover -s tests/rust_version -v
python3 scripts/check_rust_version.py
cargo test --locked
openspec validate separate-ci-workflow-concerns --strict
git diff --check
```

Results: all three entry points passed actionlint; all 47 coverage-helper tests,
24 Rust-version tests, and 572 Rust tests passed with no ignored/skipped tests.
The Rust-version consistency check, strict OpenSpec validation, and whitespace
check passed. The same canonical temporary-directory setting avoids the already
recorded macOS alias mismatch; no validation was weakened.

The new regressions execute actual composite-action shell blocks, not a rewritten
model: mode validation accepts only `total`/`changed`, rejects empty/other modes,
newlines and shell-injection payloads; assessment preserves total-pass/changed-fail
and main below-target/changed-line-N/A outcomes. Existing inline-validator tests
now extract the unchanged uploader code from `coverage-main.yml`. Static checks
assert exact event routing, no upload/OIDC in the PR entry point, shared step
reuse, distinct cancellation, all external action pins, and read-only local-action
execution after checkout. Documentation and specifications were reviewed together
for the three entry points, manual publishing choice, and PR upload absence.

## Live acceptance of the refined PR graph

The authorised implementation revision
`7b8952fd7384ae147aad97d134ded5e89326cfc9` was pushed to PR #16. Both runs completed
successfully on 2026-10-20 for that exact immutable head:

- [Testing 39356072340](https://github.com/hydratic-applications/hydragen/actions/runs/39356072340):
  `Run tests on stable Rust` and `Run tests on minimum supported Rust` passed.
- [Coverage 39356072467](https://github.com/hydratic-applications/hydragen/actions/runs/39356072467):
  `Generate coverage report`, `Check total coverage`, and
  `Check changed-line coverage` passed.

The commit-scoped workflow-run list contained exactly Testing and Coverage. The
PR's check rollup contained exactly those five successful leaf names, with no
uploader check, no skipped upload result, and no Coverage on main run. The three
coverage jobs' shared-action steps were explicitly successful in the jobs API.

Testing logs identify stable Rust 1.99.1 (`58cf8f9c3`, 2026-10-13), and exact MSRV
Rust 1.89.0 (`29483883e`, 2025-08-04), with Cargo 1.89.0 (`c24e10642`). Both
compiler jobs passed all 572 Rust tests with zero failed/ignored tests. The minimum
job's locked build completed successfully, and the helper suites passed all 24
Rust-version and 47 coverage-policy regressions on Linux.

Downloaded artifact `12234914010`, named
`coverage-39356072467-1-7b8952fd7384ae147aad97d134ded5e89326cfc9`, as data into an
approved temporary directory. It contains exactly `lcov.info` and `manifest.json`.
The independently expected context came from the GitHub run API: repository
`hydratic-applications/hydragen`, event `pull_request`, run `39356072467`, attempt
`1`, head `7b8952fd7384ae147aad97d134ded5e89326cfc9`, and event base
`a6da2b6498d38cc3959485846797bd412479f75d`.

Re-ran the maintained `total` and `changed` commands against this artifact and
those independently supplied values in the exact implementation checkout. Both
revalidated its schema, digest, run/revision identity, merge base, and all 24
tracked production Rust source identities. The canonical report SHA-256 is
`52da5320a50c672f92bcde3964e7d6eeb954c553ad4abcd785d5f33b2f334dab`.
Total coverage passed at 10,564/10,787 measured lines (97.93%). Changed-line
assessment reported explicit N/A after validating the comparison, because this
workflow/documentation change has no applicable measured Rust line changes; it
did not fabricate 100% coverage.

Task 4.2 is complete for this implementation. Independence and main uploader
eligibility under failing gates are covered by the direct dependency graph and
actual-shell/validator regressions, without deliberately failing a PR or uploading.

### Post-merge main publication follow-up

The new Coverage on main entry point has not yet run on `main`, because PR #16
has not been merged. Its complete uploader body is unchanged and its new trigger,
guard, permissions, and dependencies pass static/regression checks, but this is
not live proof of a new-entry-point OIDC upload. After an authorised merge, observe
the first main run, validate its same-run report and upload, and check Codecov
processing separately. No merge, manual upload, repository-setting change, or
archive was performed to obtain this PR evidence.
