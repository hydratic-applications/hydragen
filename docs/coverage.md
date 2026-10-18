# Code coverage

Hydragen uses `cargo-llvm-cov` to measure which executable lines its tests exercise.
It instruments the code using LLVM, the compiler infrastructure used by Rust.
GitHub Actions runs the tests on Linux for PRs, pushes to `main`, and manual runs
of the Coverage workflow.
Eligible reports are uploaded to
[Codecov](https://app.codecov.io/gh/hydratic-applications/hydragen/tree/main), which
provides source-level coverage reports and PR feedback.

The setup follows the same core approach as
[Ratatui's coverage workflow](https://github.com/ratatui/ratatui/blob/main/.github/workflows/ci.yml):
stable Rust, `cargo-llvm-cov`, an LCOV report, and the official Codecov upload
action. Unlike Ratatui's stored upload token, Hydragen uses OpenID Connect (OIDC):
GitHub issues a short-lived identity token that Codecov verifies.

## Local coverage

Use a [rustup](https://rustup.rs/)-managed toolchain so the LLVM tools match Rust.
Coverage tooling is optional; ordinary builds, tests, and CLI use do not require
it or access to Codecov.

Install the tools:

```sh
rustup toolchain install stable --profile minimal --component llvm-tools-preview
cargo +stable install cargo-llvm-cov --version 0.9.1 --locked
```

From the repository root, generate the same report format as CI:

```sh
mkdir -p target/coverage
cargo +stable llvm-cov --locked --workspace --all-targets --lcov --output-path target/coverage/lcov.info
```

LCOV is a text format recording source lines and their execution counts. The
report stays under the Git-ignored `target/` directory. This command does not
upload anything or need Codecov credentials. To inspect the previous run as a
local HTML report, use `cargo +stable llvm-cov report --html` and open
`target/llvm-cov/html/index.html`.

### What the number measures

Coverage runs library and binary unit tests, integration tests that launch the
CLI, and Unix pseudo-terminal tests that exercise real terminal interactions.
The workflow verifies that both library and CLI source lines were executed before
uploading. It does not impose a local percentage threshold or exclude production
modules to improve the number.

The uploaded metric is **line coverage on Linux**, not a promise of correctness or
coverage on every platform. The tool excludes external dependencies, build
outputs, and integration-test source files by default; inline unit-test code can
still contribute to the result. Branch and doctest coverage require unstable
tooling and are not included. Some tests deliberately kill children; those
processes may not write their final coverage counters before exiting. A local
run on another platform can therefore differ from CI.

## GitHub and Codecov setup

1. Install the Codecov GitHub app with access to this repository and ensure the
   signed-in Codecov account recognises it. App installation alone does not prove
   private report processing works.
2. Verify the account's private-repository access and usage limits. A generic
   account-limit banner does not identify which limit was reached; check the
   account details and upload/processing results before assuming uploads are
   blocked. Consult [current plan details](https://about.codecov.io/pricing/)
   rather than infer entitlement from another project's public shield.
3. Run the Coverage workflow through a PR or a push to `main`. It requests only
   repository read access and job-scoped OIDC token issuance. No `CODECOV_TOKEN`
   secret is required for the configured OIDC route. Authentication or upload
   errors fail the job rather than hide missing reports.
4. Confirm Codecov processed the report. A successful upload is not evidence that
   every dashboard feature or PR assessment is available on the account. Both
   project and patch statuses are configured; verify their actual results without
   starting a trial or changing billing merely to complete setup.

The first implementation PR can upload without having a baseline to compare
against. After merge, a push run on `main` establishes that baseline. If needed,
manually run Coverage on `main` once the workflow exists there. Then inspect the
next same-repository PR's comparisons. Do not treat missing base data as 100%
coverage or a completed comparison.

### PR checks and trust boundaries

- The Actions **coverage** job checks tests, report generation, library/CLI hit
  data, and eligible uploading. Any failure makes the job fail.
- Codecov's **project** status compares overall coverage with the base commit.
  Its **patch** status assesses changed executable lines. Both use automatic
  targets and zero allowed drop, not a fixed 80% or 100% project target. These
  assessments arrive asynchronously after report processing.
- Fork PRs run instrumented tests and report validation but **skip uploads**. A
  job summary explains that no Codecov assessment was requested. No secrets are
  sent to forks and contributed code is never run through `pull_request_target`.
  Codecov supports tokenless public-fork uploads, but adopting that route is a
  separate decision; publication does not silently enable it.
- Same-repository bot PRs use the OIDC route. Dependabot events have special
  GitHub permission rules, so confirm those runs can obtain the identity token.
  If not, investigate the event permissions; do not use privileged PR execution
  or ignore upload errors as a workaround.

The Codecov app publishes its own feedback; the workflow needs no PR or status
write permission. Configuration does not make any check required for merging or
change branch protection.

## README shield and going public

The live Shields.io image and its Codecov link explicitly select `main`, even in
a feature branch's README. The displayed number is the latest processed main
report after normal image/service cache refresh, not necessarily the newest
commit or the current PR.

**The maintainer chose a token-free image URL.** Private report data may therefore
produce an unavailable shield even when uploads and authorised dashboard access
work. Do not embed an upload token, GitHub credential, API token, or a private
badge-access token to make the image work. An ordinary README cannot interpolate
GitHub Actions secrets. Do not replace an unavailable result with a hardcoded
number or zero.

When the repository becomes public, follow the
[going-public checklist](going-public-checklist.md): verify public Codecov report
access and the same token-free shield after a main upload and cache refresh.
No private badge token is introduced by this setup, so no such token needs
revocation unless one is deliberately added later.

## Troubleshooting

- **Tests fail or time out:** inspect the failing test and terminal-test cleanup;
  do not skip terminal tests or reuse stale profiles to obtain a successful upload.
- **Missing library or CLI coverage:** check that child processes inherit LLVM
  profile settings and the report contains both `src/lib.rs` and `src/main.rs`.
- **LLVM/profile errors:** install `llvm-tools-preview` for the same Rust
  toolchain; avoid mixing it with unrelated system LLVM tools. The normal
  `cargo llvm-cov` run refreshes instrumented build data.
- **OIDC, upload, or processing failures:** check the job's permissions, the
  Codecov app's repository access, account limits, and service availability.
  Retry transient service failures; do not disable uploader integrity checks or
  introduce a stored-token fallback without a deliberate configuration decision.
- **No comparison status or numerical shield:** distinguish missing main/base
  reports, processing delays, account feature access, and image cache delays.
  A private, token-free shield can remain unavailable by design.

When updating CI tools, recheck stable releases and their supported inputs. Pin
actions to reviewed release commits and update the explicit `cargo-llvm-cov`
version alongside these local instructions. The implementation references
[Rust's coverage documentation](https://doc.rust-lang.org/rustc/instrument-coverage.html),
[cargo-llvm-cov's Codecov guidance](https://github.com/taiki-e/cargo-llvm-cov#github-actions-and-codecov),
and the [official Codecov action's OIDC guidance](https://github.com/codecov/codecov-action#using-oidc).
