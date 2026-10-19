# Code coverage

Hydragen uses `cargo-llvm-cov` to measure which executable lines its tests exercise.
It instruments Rust code using LLVM, the compiler infrastructure used by Rust.
GitHub Actions owns three quality checks: **Tests**, **Total coverage**, and
**Changed-line coverage**. Each applicable coverage gate requires at least 80%,
using exact line counts. These checks run for PRs, including forks, pushes to
`main`, and manual Coverage workflow runs, without depending on Codecov.

The supporting **Coverage report** job produces and retains the shared evidence.
A separate **Codecov main report** job sends eligible main reports to
[Codecov](https://app.codecov.io/gh/hydratic-applications/hydragen/tree/main) for
source-level browsing and the README shield. These two pipeline-health jobs are
not extra quality policies. Codecov's project and patch statuses are disabled.

## Local coverage

Use a [rustup](https://rustup.rs/)-managed toolchain so the LLVM tools match Rust.
Coverage tooling is optional; ordinary builds, Rust tests and offline CLI use do
not require it, Python, `diff-cover`, Codecov, or upload credentials.

Install the coverage tools:

```sh
rustup toolchain install stable --profile minimal --component llvm-tools-preview
cargo +stable install cargo-llvm-cov --version 0.9.1 --locked
```

From the repository root, generate the raw LLVM report used as CI input:

```sh
mkdir -p target/coverage
cargo +stable llvm-cov --locked --workspace --all-targets --lcov --output-path target/coverage/lcov.info
```

LCOV is a text format recording source lines and their execution counts. The
report stays under Git-ignored `target/`. This command does not upload or enforce
a percentage. To inspect its results as HTML, use
`cargo +stable llvm-cov report --html` and open `target/llvm-cov/html/index.html`.

Coverage-policy helper tests are separate from Cargo's tests and include real
`diff-cover` integration. With Python 3.10 or later, install the pinned tool in an
isolated environment, then run:

```sh
python3 -m venv target/coverage-venv
target/coverage-venv/bin/python -m pip install 'diff-cover[toml]==10.6.0'
export DIFF_COVER="$PWD/target/coverage-venv/bin/diff-cover"
python3 -m unittest discover -s tests/coverage -v
```

`DIFF_COVER` selects the isolated executable; otherwise tests look for `diff-cover`
on `PATH`. The helper always supplies its own TOML configuration to avoid
project-specific exclusions. Python 3.10 needs the `toml` extra to read it;
Python 3.11 and later have a built-in TOML parser. CI also installs pinned
diff-cover in isolation for its combined **Tests** check and changed-line job.
These dependencies are optional developer tooling, not Rust/runtime dependencies;
ordinary `cargo test --locked` is unchanged.

### What the number measures

The instrumented run covers library and binary unit tests, integration tests that
launch the CLI, and Unix pseudo-terminal tests for real terminal interactions.
The producer requires executed library and CLI lines and valid coverage for every
tracked production Rust file under `src/` at the measured commit, including nested
modules. A genuinely unmeasured source file produces a diagnostic, not an
exemption. Deleted files are not required; renamed files use their head paths.

The metric is **line coverage on Linux**, not proof of correctness or coverage on
every platform. The tool's default scope excludes external dependencies, build
outputs and integration-test source; inline unit-test code can still contribute.
No new production exclusions improve the score. Branch and doctest coverage need
unstable tooling and are not included. Some tests deliberately kill child
processes that may not write final counters before exit. Other platforms can
therefore produce different results. Codecov may also process source scope
differently, so its displayed percentage need not exactly match the LLVM gate.

The gate counts each measured source line once, covered when its execution count
is positive. LLVM's raw summary can count overlapping groups of functions, so its
percentage can differ from counting distinct lines in the same report. Before
assessment, the producer validates the raw report and writes a standard line-only
LCOV report with totals recalculated from those distinct lines. No source file or
measured line is removed; a different total does not mean tests improved or scope
was reduced. Consumers and the uploader accept only this canonical report: its
line records and recalculated summaries must agree.

## What the checks mean

- **Tests** runs `cargo test --locked`, including normal unit, integration,
  terminal and documentation tests, plus the Python helper tests above using an
  isolated pinned diff-cover executable. Any test failure fails this check. It
  has read-only access and no coverage-service or percentage dependency.
- **Coverage report** runs fresh stable Rust/LLVM instrumentation once, validates
  and normalizes LCOV data, and retains it with identifying metadata. It does not
  enforce a percentage or upload. Instrumented-test, report, validation and
  retention errors fail this job; a valid low-coverage report does not.
- **Total coverage** validates that shared evidence and requires at least 80% of
  all measured executable lines to be covered, with a positive measured count.
- **Changed-line coverage** independently requires at least 80% of the measured
  executable lines added or modified by a PR to be covered. A valid PR with none
  is explicitly **not applicable** (N/A), not numerically 100%. Main and manual
  non-PR runs report N/A by event policy, not by comparing with an arbitrary ref.

For covered count `C` and positive measured count `N`, both gates pass exactly
when `5 * C >= 4 * N`. Exactly 80% passes; a smaller fraction fails even if rounded
display text says 80%. For example, 4 of 5 passes, while 7,999 of 10,000 fails.
High total coverage cannot compensate for poorly tested changes, or vice versa.

The two gate jobs depend only on valid report production, not on one another,
normal tests or uploading. Low total coverage does not skip changed-line analysis
or discard the report. If production fails, dependent gates are blocked, not
coverage passes. An upload or service failure cannot change either local result.

### Revisions and report identity

PR coverage measures the event's immutable head commit, not GitHub's synthetic
merge checkout. Changed-line analysis compares the merge base (the shared ancestor
of that head and the event's immutable base commit) to the same head. It does not
follow a moving `main` branch or need a processed Codecov base report.

The retained artifact contains exactly canonical `lcov.info` and `manifest.json`. The
manifest identifies repository, run ID/attempt, event, head/base/merge-base
revisions, report digest and the tracked production-source inventory. Each source
entry records its SHA-256 content digest and source-file line count. Coverage
counts are validated from LCOV line records and summaries, not separate aggregate
fields in the manifest. Consumers check metadata against independently expected
run/event context and their verified checkout. Missing refs or reports, wrong
identities, omitted sources, invalid counts and path mismatches are errors, never
N/A. Non-PR runs set base and merge base equal to the measured head.

Pinned `diff-cover` reads an explicit merge-base-to-head diff file, forced to text
so Git attributes cannot hide source changes as binary data. External diff
programs and text-conversion filters are disabled. The repository
adapter validates its JSON integer counts and line sets against that diff and
LCOV, then applies the exact threshold. It does not use the tool's rounded
percentage threshold or turn missing source mappings into zero applicable lines.
Deleted lines do not add measured head lines; additions and renames must map to
the measured head's source paths.

### Repository gate helper

Run `python3 scripts/coverage_gate.py` from the repository root with one of these
subcommands:

- `prepare --report RAW --output NEW_DIR` validates raw LLVM LCOV and writes
  canonical line-only LCOV plus the identifying manifest to a new directory. The
  output directory must not already exist.
- `total --artifact DIR` revalidates that artifact and applies total policy.
- `changed --artifact DIR` revalidates it and applies changed-line policy. Add
  `--diff-cover EXECUTABLE` to select an isolated diff-cover 10.6.0 executable;
  otherwise it uses `diff-cover` on `PATH`.

Every subcommand requires `--event`, `--head`, `--base`, `--repository`, `--run-id`
and `--run-attempt`. Use the actual event (`pull_request`, `push` or
`workflow_dispatch`), full immutable head/base commit SHAs, `owner/repository` and
the run's positive integer ID/attempt. For non-PR use, base must equal head. The
checkout must match head, with complete non-shallow history and exactly one merge
base. Supply expected context independently; do not copy it out of an untrusted
artifact just to make validation pass.

The helper exits **0** for pass or explicit N/A, **1** for a below-target result,
and **2** for invalid inputs or evidence. It rejects duplicate LCOV records,
lines and summaries rather than silently counting them twice. Current validation
accepts regular tracked `src/` Rust files whose directory and filename components
use ASCII letters, digits, `_` or `-`, ending in `.rs`. Unsupported paths fail
explicitly; they are not exempted. Reports are limited to 50 MiB and manifests to
2 MiB. These checks protect evidence handling; they do not narrow the inventory
to files the tool happens to measure.

## Codecov setup and upload safety

Install the Codecov GitHub app with access to the repository and confirm report
access separately from upload acceptance. The configured authentication is OpenID
Connect (OIDC): GitHub issues a short-lived identity token that Codecov verifies.
No `CODECOV_TOKEN` secret is needed. Account limits or service features can affect
browsing and publishing, but do not decide the local Actions coverage gates.

Only pushes to `main` and manual runs on `main` upload. Every PR, including
same-repository, bot and fork PRs, retains local coverage evidence without uploading.
Publication does not silently enable fork uploads or privileged execution.

The **Codecov main report** job alone has OIDC issuance permission. It uses a fresh
runner with no checkout, Cargo, repository scripts, build caches or executable
artifacts.
It downloads only the named current-run report data, validates allowed files,
types, sizes, schema, run/repository/commit context and digest using inline workflow
code, including strict canonical line records and summary agreement, then invokes
the pinned official uploader. The commit, branch, repository
and report file are explicit; automatic report search is disabled, integrity
checks stay enabled and plugins are disabled with `noop`.

Valid below-target main reports still upload because the upload job depends only
on the producer. Authentication/upload failures fail that job visibly, not the
local gates. All other jobs are read-only; there is no PR/status write permission,
`pull_request_target` execution or privileged `workflow_run` bridge. Configuration
does not change branch protection or make checks required for merging.

## README shield and going public

The existing live Shields.io URL and Codecov destination explicitly select `main`,
even in a feature branch's README. The number is the latest processed main report
after normal service/image cache refresh, not necessarily the latest commit or
the current PR. A PR run cannot establish the main upload route; verify a real
main push or manual main run once the workflow exists there.

The shield URL deliberately contains no badge-access token. Private report access
can leave it unavailable even when uploading and authorised browsing work. Never
embed upload, GitHub, API or private badge tokens to make the image work, or replace
an unavailable value with a fixed percentage or zero. A README cannot interpolate
GitHub Actions secrets.

Follow the [going-public checklist](going-public-checklist.md) before publication.
Afterward, verify anonymous GitHub and Codecov access separately, a processed main
upload and the same token-free shield after cache refresh. Public visibility does
not by itself establish report access or account features; no billing change is
required to determine the local gates.

## Troubleshooting

- **Normal or instrumented tests fail/time out:** inspect failing assertions and
  terminal child cleanup. Do not skip tests or reuse stale profiles for a green run.
- **Missing production-source or execution evidence:** inspect the named file and
  the immutable head's tracked `src/` inventory. Check that child processes inherit
  LLVM profile settings. Do not exempt the file or treat absent coverage as N/A.
- **LLVM/profile errors:** install matching `llvm-tools-preview` for the Rust
  toolchain. Avoid unrelated system LLVM tools; generate fresh instrumented data.
- **Wrong revision, digest or source path:** regenerate evidence for the exact
  event head/base and run attempt. Resolve required history and repository-relative
  paths; do not fall back to moving refs or a different run's artifact.
- **A displayed 80% fails:** inspect integer counts. Rounding is presentation only.
- **`No Toml lib installed` with Python 3.10:** install
  `'diff-cover[toml]==10.6.0'` in the selected isolated environment. The gate's
  explicit TOML configuration needs that optional parser on Python 3.10.
- **Changed-line N/A seems wrong:** confirm report/diff/path validation succeeded
  and that the event is a PR before interpreting zero applicable measured lines.
- **OIDC/upload fails:** check main-event eligibility, job-only OIDC permission,
  app repository access and service availability. Keep integrity checks enabled;
  do not add a stored-token fallback, broaden PR permissions or ignore the error.
- **No numerical main shield:** distinguish an absent main report, processing
  delay, report-access restrictions and image caching. Local gates are unaffected.

When updating tooling, recheck supported inputs and stable releases, pin actions
to reviewed commits, and keep explicit tool versions and these instructions aligned.

## References

- [Rust coverage instrumentation](https://doc.rust-lang.org/rustc/instrument-coverage.html)
- [cargo-llvm-cov 0.9.1](https://github.com/taiki-e/cargo-llvm-cov/blob/v0.9.1/README.md)
- [LLVM 23.1.1 LCOV export](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageExporterLcov.cpp)
- [LLVM 23.1.1 function-group summaries](https://github.com/llvm/llvm-project/blob/llvmorg-23.1.1/llvm/tools/llvm-cov/CoverageReport.cpp)
- [diff-cover 10.6.0](https://github.com/Bachmann1234/diff_cover/blob/v10.6.0/README.rst)
- [Git merge-base](https://git-scm.com/docs/git-merge-base)
- [GitHub OIDC](https://docs.github.com/en/actions/concepts/security/openid-connect)
- [Official Codecov action OIDC guidance](https://github.com/codecov/codecov-action#using-oidc)
- [Codecov status configuration](https://docs.codecov.com/docs/commit-status)
