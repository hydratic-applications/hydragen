## Context

The pending `configure-codecov` integration supplies stable Rust instrumentation,
LCOV reporting, OIDC uploading, and a token-free main-branch shield. This follow-up
separates normal tests, coverage policy, report production, and publishing.

The active decision is three GitHub Actions-owned quality checks with independent
inclusive 80% coverage minimums. It supersedes this change's earlier Codecov-only
policy and the original integration's PR upload scope. Earlier verification is
historical evidence for the previous design, not validation of these gates.

This supports the foundations' developer-experience and visibility values. No
unresolved product question in `openspec/design-questions.md` affects this
infrastructure increment; no foundation or glossary revision is needed.

## Goals / Non-Goals

**Goals:** Make correctness and each coverage policy independently visible on all
PRs; use exact, reproducible line counts; preserve valid low-coverage reports;
make local policy independent of Codecov; isolate upload privileges from tested
code and executable artifacts.

**Non-Goals:** New platform guarantees, branch/doctest coverage, production Rust
changes, coverage-driven exclusions, runtime dependencies, PR uploads, new stored
credentials, billing or required-check changes, or publication as part of this
implementation.

## Decisions

### Separate three quality checks from report-pipeline health

Keep the existing bounded workflow for PRs, pushes to `main`, and manual dispatch,
without path filters, retaining concurrency controls and child-process cleanup.

- `Tests` runs `cargo test --locked` and
  `python3 -m unittest discover -s tests/coverage -v`. It uses stable Rust, only
  content-read permission, and checkout without persisted credentials. Normal
  unit, integration, terminal, and documentation tests remain included. The
  Python suite uses isolated pinned diff-cover for real diff integration; this
  does not add that dependency to ordinary Rust tests.
- `Coverage report` independently runs fresh stable Rust/LLVM coverage with
  `cargo-llvm-cov` 0.9.1, `--locked --workspace --all-targets`, and LCOV output on
  Linux. It validates and retains data without a percentage decision or OIDC.
- `Total coverage` and `Changed-line coverage` are sibling data consumers. Each
  depends only on successful report production, not on `Tests`, the other
  consumer, or Codecov. Invalid production visibly blocks assessment; it is not
  replaced with a placeholder success.
- The separate `Codecov main report` upload job depends only on the producer.
  Its service failure fails that job, not either percentage result. It is pipeline
  health, not a fourth quality policy, just as `Coverage report` is not another
  quality policy.

This retains normal and instrumented test runs to keep correctness feedback
independent of instrumentation failures. Share normalized report data once,
instead of rerunning instrumentation for each gate or sharing executable builds.

### Measure immutable revisions and retain run-bound data

For a PR, check out the event's head SHA, not GitHub's default synthetic merge
checkout. Acquire the immutable event base SHA and enough history to compute
`git merge-base BASE HEAD`; compare that merge base to the same measured `HEAD`.
Never use a moving `origin/main` or a different checkout's source coordinates.
Verify checkout identity before generation. Missing refs or merge bases are
errors. Main/manual runs measure their selected commit without inventing a PR.

Generate LCOV once with default tool scope: no new exclusions, external
dependencies/build outputs/integration-test source excluded by the tool's normal
rules, and inline unit-test code still counted. Require execution evidence for
library and CLI paths. Derive a dynamic inventory of tracked `src/**/*.rs` files
from the immutable measured head (including files directly under `src/`). Require
every such file in coverage: a genuinely unmeasured file gets an explicit error,
not an exemption. Head-deleted files are absent from the inventory; renamed files
use their head paths. Do not hardcode the current file count.

Validate LCOV structure, integer counts, line coordinates, and source paths;
normalize valid paths to repository-relative names. Reject unresolved, ambiguous,
outside-repository, stale, or mismatched sources. Reject duplicate records with
a useful diagnostic rather than silently counting them twice.
Malformed/missing reports, source omissions, or impossible counts are errors,
never a zero-line result.

### Separate raw LLVM validation from canonical line evidence

LLVM 23.1.1 exports detailed `DA` records (source-line number and execution count)
from merged file coverage, but exports `LF`/`LH` line totals from function-group
summaries. Within an instantiation group (compiled versions of the same function),
LLVM takes maxima before adding groups; distinct groups can overlap on source
lines. Function details can also enumerate instantiations while function summaries
count groups. Raw summary/detail equality is therefore not a valid assumption.

Validate raw LLVM syntax, required fields and paired summaries, nonnegative
integer counts and covered/found pair bounds, duplicate records/fields/lines,
inventory, paths, source coordinates and known library/CLI hits. Do not assert
raw grouped line/function summary equality with enumerated detail records, or
other cross-summary/detail bounds not guaranteed by LLVM. This accepts valid
LLVM output without weakening source completeness or line-level policy.

Then emit only `SF`, `DA`, recomputed `LF`/`LH` and `end_of_record` records in
canonical repository-relative `lcov.info`. Preserve every validated unique
source-line record and its count. Count each line once; it is covered exactly
when its execution count is positive. Recompute found/covered summaries from
these records. Reject duplicates rather than double counting. Function/branch
metadata is validated as raw input but is not retained in this line-policy report.

Consumers and the workflow-inline uploader accept only the strict canonical
line-only format and require derived summary equality. There is no permissive
raw mode for them. This needs no extra artifact, LLVM JSON export, or second
segment-to-line coverage engine; the existing LLVM line records already provide
the policy input. The CLI and two-file artifact contract are unchanged.

Retain exactly `lcov.info` and `manifest.json`. The manifest has `schema_version`,
`repository`, `run_id`, `run_attempt`, `event`, `head`, `base`, `merge_base`,
`report_sha256` and `sources`; each source maps its head path to `sha256` and
`lines` (the number of source-file lines). It binds the report to repository,
current run and attempt, event, immutable revisions and source inventory. Non-PR
base and merge base equal head. Coverage counts are validated from canonical LCOV
records and summaries rather than duplicated as aggregate manifest fields. Consumers receive
expected context independently from the event/run and verified checkout, not by
trusting the manifest to describe itself. Recheck identity, schema, digest,
inventory, paths and counts before policy. This prevents stale or cross-run data
from silently deciding the current commit's result.

### Apply exact inclusive arithmetic in each consumer

For a positive measured-line count, pass exactly when
`5 * covered >= 4 * measured`, with integer `0 <= covered <= measured`. Exactly
80% passes; any smaller fraction fails even if a display rounds it to 80%. Show
covered/measured counts and a readable percentage, but never use formatted output
as policy input. A zero total denominator is invalid evidence, not N/A.

Low total coverage cannot suppress changed-line assessment, report retention, or
an eligible upload. Conversely, a high changed-line score cannot compensate for
low total coverage. No `continue-on-error` or fabricated status hides a failure.

### Use pinned diff analysis, not its rounded threshold

Install `diff-cover` 10.6.0 in an isolated CI environment (an optional developer
tool, not a Rust/runtime dependency). Its explicit TOML configuration requires
the `toml` extra on Python 3.10; Python 3.11+ supplies the parser in its standard
library. Generate an explicit Git diff file for
`merge-base(event base, measured head) -> measured head` and pass it using
`--diff-file`. Force Git's textual diff with `--text`, keeping `--no-ext-diff` and
`--no-textconv`, so a `.gitattributes` entry such as `src/lib.rs -diff` cannot
suppress source hunks and turn uncovered changes into N/A. Request JSON output
with the pinned version's supported format.
Do not let defaults, moving refs, or tool configuration add exclusions or alter
the comparison. Do not use its rounded percentage or `--fail-under` decision.
The adapter invokes diff-cover with a zero threshold so that only the adapter's
exact-count decision enforces the 80% policy.

The maintained adapter validates the JSON schema, integer covered/measured
counts and per-file line sets against LCOV and the actual diff. Cross-check
changed source paths so a skipped or mismatched coverage source cannot turn into
N/A. Apply the same exact arithmetic to the validated changed measured lines.
Validate the root and per-source JSON object shapes explicitly so malformed
structure produces the intended diagnostic and error exit, not an uncaught
exception.

An otherwise valid PR diff with zero applicable measured executable lines is a
successful, explicit **not applicable** result, without a numerical 100%. Deleted
lines do not add executable head lines; additions and renames must map to head
paths correctly. Main pushes and manual non-PR runs report N/A by event policy,
after validating their report, not by comparing against an arbitrary revision.
Missing refs, reports, source mappings or required records are failures, not N/A.

### Keep Codecov for browsing and the shield, not enforcement

Set `coverage.status.project: false` and `coverage.status.patch: false`, retaining
`main` as the configured branch. Do not add a status-publishing wrapper, duplicate
Codecov percentage assessments, or an account-feature dependency to local gates.
Retain the actual token-free main Shields.io image URL and report destination;
do not hardcode a percentage. Codecov may process source scope differently from
the LLVM policy report, so identical displayed percentages are not promised.

### Isolate authenticated uploading to main-only data consumption

Only pushes to `main` and manual runs on `main` may enter the separate upload job.
All PR uploads are disabled, including same-repository, bot, and fork PRs. Grant
`id-token: write` only to this job; all other jobs remain read-only. Use reviewed
full SHA action pins. Never introduce `pull_request_target`, a `workflow_run`
privilege bridge, repository/PR/status write permissions, or stored-token fallback.

Use a fresh upload runner with no checkout, Cargo, repository scripts, build cache,
or executable artifact consumption. Download only the explicitly named artifact
from the current run into a dedicated directory. A small inline workflow-owned
validator checks the exact allowed data filenames, regular-file types, sizes,
schema, repository/run/attempt/event/commit context, inventory metadata, and
report digest, plus canonical-only LCOV structure and derived summary agreement.
Reject unexpected data, symlinks, identity mismatches and tampering;
never execute artifact content. The uploader's expected context comes from the
trusted main event and run, not the artifact itself.

Use the pinned official Codecov action with OIDC, explicit commit/branch/slug and
report file, automatic search disabled, integrity checks enabled, and
`plugins: noop`. Authentication/upload failures remain visible in the upload job.
Valid below-target main reports still upload because neither gate is its
dependency. Fork and other PR checks need no upload or credential expansion.

Preserve the validated repository-relative LCOV source paths during Codecov
processing with `codecov.disable_default_path_fixes: true`. The uploader's
temporary-directory file inventory contains upload files, not the production
sources: it must not be used to predictively rewrite or discard valid `src/`
paths. This setting records the already implemented processing fix in
[PR #10](https://github.com/hydratic-applications/hydragen/pull/10),
not a new workflow change. It retains the no-checkout boundary and strict
artifact/source validation; unrelated upload files do not redefine report scope.

### Synchronise the input-loss test after dedicated stdin selection

The terminal input-loss test redirects standard input to a separate
pseudo-terminal (PTY), while the process's controlling terminal and error output
remain usable. A visible prompt is not proof that the prompt library has selected
that input. With the locked cliclack 0.5.6 and console 0.16.6 dependencies, the
prompt frame can be flushed before console checks whether stdin is a terminal.
On Linux, disconnecting the dedicated input PTY in that interval can make the
check fail and cause console to read `/dev/tty`, the surviving controlling
terminal, instead. A controlled reproduction confirmed this matching race; the
earlier hosted transcript alone cannot establish its unique cause.

This increment repairs the test's injection setup, not the production fallback.
Before spawning, require the dedicated stdin PTY to be canonical (line-buffered).
After observing the target Name or format-selector frame, wait for that same PTY
to become noncanonical (input delivered without line buffering), then close all
of its master handles. That transition establishes that stdin has been selected;
it does not prove a read system call is currently blocked, or that every terminal
setting is in raw mode. Do not infer readiness from output alone.

Observe the transition through portable-pty's supported `MasterPty::get_termios`
API and the `ICANON` name exposed by `local_flags.iter_names()`, without
platform-specific bit constants or new Cargo dependencies. Use the existing
`TIMEOUT` deadline and output-event draining while checking child status. Missing
termios support, early child exit or an expired deadline must fail diagnostically,
not skip the test or extend its wait. Add two helper tests for pending/ready,
expired, unsupported and exited-child states. Improve `Session::finish` timeout
diagnostics with child status and closed/expected reader counts only; retain its
existing timeout, completion condition and child cleanup.

Keep both Name and selector injection cases and all existing assertions: nonzero
exit, one correctly styled input-error diagnostic, empty standard output, no
cancellation or success wording, and no created files or layer manifest. Do not
add retries, arbitrary readiness sleeps or keystrokes, weaker assertions, or
test/coverage exclusions. Run fixed 100-repetition batches for macOS, normal
Linux and instrumented Linux, exercising both cases each time and failing a batch
on any failure, then full suites, independent review and a fresh hosted run.

Production input loss **before** console selects stdin can still enter the
controlling-terminal fallback. That runtime gap remains a separate open follow-up;
this test-only change neither fixes it nor relaxes the existing interaction policy.
Production Rust, Cargo dependencies, workflow and coverage policy remain unchanged.

### Keep specifications ordered and documentation focused

`configure-codecov` introduces `code-coverage-ci` outside the main specifications.
When synchronisation is separately requested, apply its ADDED requirements first,
then this change's MODIFIED requirements, including authenticated-upload scope,
and its additions. Do not change archived assets or load `openspec/` from code,
tests, or build scripts.

Revise existing explanations instead of appending competing policies. README
Development stays concise; `docs/coverage.md` explains scope, checks, exact
arithmetic, N/A, run identity, OIDC and troubleshooting. The publication checklist
expects Actions-owned gates and main-only OIDC verification while retaining
anonymous-access, token-free shield and fork-safety checks. Keep historical
verification separate from new technical evidence and pending hosted/main work.

## Risks / Trade-offs

- Extra jobs and duplicate normal/instrumented builds cost runner work; consumers
  share one data artifact and instrument only once.
- Cross-job evidence adds validation complexity; test identity, digest, source and
  path failures rather than trusting self-described artifacts.
- Changed-line tooling can round results or miss paths; validate integer JSON and
  line sets against immutable Git/LCOV evidence before applying policy.
- Service availability can prevent report browsing or a fresh shield; it cannot
  alter local policy results. Report service failures honestly, without widening
  permissions, changing billing or substituting a static number.

## Migration and verification

Implement helpers/fixtures outside `openspec/`, then reconcile their actual CLI
with the documentation. Validate exact/above/below-80 cases (including values
displayed as 80), independent gate outcomes, valid N/A, malformed/missing data,
unavailable refs, wrong run/attempt/event/repository/revisions/digest, source/path
omissions, duplicate records, invalid counts, additions, renames and deletions.
Exercise valid raw LLVM grouped summaries that differ from unique line/function
details, missing/invalid summary pairs, strict canonical tampering, malformed JSON
object shapes and Git-attribute binary-diff suppression. Run preparation and
consumers on the actual production report, not only synthetic fixtures.
Exercise pinned diff-cover against real temporary Git fixtures. Run Python tests,
Actionlint, formatting, Clippy, strict OpenSpec validation, normal macOS/Linux and
instrumented Linux suites with bounded terminal waits and child cleanup.

Independently review the exact implementation and trust boundaries. Observe the
three revised hosted quality checks and retained report on the authorised PR
candidate. Main OIDC upload, public report access and the main shield require a
later eligible main run and separate publication readiness; do not claim them from
PR evidence. No merge, publication, billing or required-check operation is part
of these implementation tasks. Any rollback must be reviewed and preserve other
work rather than silently restoring superseded policy.

## Sources

- [cargo-llvm-cov 0.9.1 scope and report options](https://github.com/taiki-e/cargo-llvm-cov/blob/v0.9.1/README.md)
- [LLVM 23.1.1 LCOV line details and summaries](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageExporterLcov.cpp)
- [LLVM 23.1.1 function-group report construction](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageReport.cpp)
- [LLVM 23.1.1 summary maxima and addition](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageSummaryInfo.h)
- [diff-cover 10.6.0 LCOV, diff-file, JSON and percentage behaviour](https://github.com/Bachmann1234/diff_cover/blob/v10.6.0/README.rst)
- [Git merge-base](https://git-scm.com/docs/git-merge-base)
- [GitHub artifact transfer between jobs](https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/storing-and-sharing-data-from-a-workflow)
- [GitHub OIDC security](https://docs.github.com/en/actions/concepts/security/openid-connect)
- [Codecov action OIDC and supported inputs](https://github.com/codecov/codecov-action#using-oidc)
- [Codecov disabling project and patch statuses](https://docs.codecov.com/docs/commit-status)
- [cliclack 0.5.6 prompt interaction source](https://docs.rs/crate/cliclack/0.5.6/source/src/prompt/interaction.rs)
- [console 0.16.6 terminal input selection source](https://docs.rs/crate/console/0.16.6/source/src/unix_term.rs)
