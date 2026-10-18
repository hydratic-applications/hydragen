## Context

Hydragen is a single Rust package with library and binary targets. Tests include
unit tests, subprocess-based CLI tests, and Unix pseudo-terminal tests. There are
no GitHub Actions workflows in this checkout. The repository is
`hydratic-applications/hydragen`, is currently private, and uses `main` as its
default branch. The maintainer has installed the Codecov GitHub app; installation
alone does not generate coverage or establish private-repository account access.
The maintainer intends to make the repository public later, but not as part of
this change. Private operation is the current requirement, not a reason to delay
CI until publication; no publication date is assumed.

The README has a centred HTML badge header. While this change was prepared,
`main` adopted `for-the-badge` styling at 20px height. The maintainer explicitly
chose to match that newer style for coverage when resolving the README merge,
superseding this proposal's initial `flat-square` choice without reverting any
of the other badge updates.
The existing badge specification still describes an older Markdown layout and
forbids coverage badges because there was no coverage infrastructure. This change
updates those affected requirements without redesigning the header.

This supports the foundations' DX, efficiency, and visibility values: reviewers
get repeatable evidence without extra manual work. It changes development
infrastructure, not product behaviour. None of the unresolved product questions
in `openspec/design-questions.md` needs resolution; coverage is not a substitute
for the broader success criteria in question 10. No foundations or glossary
changes are needed.

## Goals / Non-Goals

**Goals:**

- Run existing Rust tests with line-coverage instrumentation on PRs and `main`.
- Publish reports and project/patch checks through Codecov, with visible failures
  for broken tests, report generation, or authenticated uploading.
- Display the latest processed `main` coverage through a live Shields.io badge.
- Keep setup, private-repository constraints, and local reproduction understandable.

**Non-Goals:**

- A platform test matrix, CI formatting/lint expansion, nightly branch coverage,
  doctest instrumentation, coverage-driven source refactors, or test-gap cleanup.
- A fixed percentage target, branch-protection changes, mandatory PR comments,
  repository publication, or automatic paid-account changes.
- Network access during ordinary local tests or a runtime Codecov dependency.

## Decisions

### Use Codecov's official Rust example as a reference, not a verbatim template

Review both the [official example's GitHub Actions workflow](https://github.com/codecov/example-rust/blob/e7ff5cca324870002988e59a1cd7a600b58c3305/.github/workflows/rust.yml)
and its [grcov output configuration](https://github.com/codecov/example-rust/blob/e7ff5cca324870002988e59a1cd7a600b58c3305/.github/actions-rs/grcov.yml).
At the reviewed revision, the workflow runs on main pushes and PRs to main, uses
nightly profiling flags and `actions-rs/grcov`, generates LCOV, and uploads through
`codecov/codecov-action@v5` with a token held in GitHub secrets and
`fail_ci_if_error: true`. This confirms the separation between running tests with
coverage instrumentation, generating a report, and uploading it to Codecov.

The example's README instead describes Travis, kcov, and a Bash uploader, with a
doctest limitation explicitly dated 2016. Do not treat those instructions as the
current GitHub Actions setup or a current statement of Rust's capabilities.
Preserve the workflow's useful LCOV/upload pattern, but use stable
`cargo-llvm-cov` to avoid its nightly flags and extra grcov configuration. Use the
Codecov action's current authentication documentation for OIDC rather than assume
the example's stored token is the only supported route. These are deliberate
adaptations to Hydragen, not claims that the official example uses our chosen
tooling or proves private OIDC uploads work here. Retain our explicit report
selection, SHA pins, full test-target coverage, and fork trust boundary.

### Follow current stable documentation and releases, not stale sample versions

Rust's [instrumentation-based coverage documentation](https://doc.rust-lang.org/rustc/instrument-coverage.html)
describes compiler instrumentation through `-C instrument-coverage` and recommends
matching LLVM tools from `llvm-tools-preview`. `cargo-llvm-cov` wraps this compiler
facility; its current stable documentation directly demonstrates LCOV uploads to
Codecov. This makes it an appropriate modern Rust coverage workflow, not a claim
that Rust mandates one Cargo coverage tool. Branch and doctest coverage remain
documented unstable features of `cargo-llvm-cov`; do not add nightly merely to
make this line-coverage integration look more current.

Release checks on 2026-10-18 found these latest non-prerelease releases:

- [cargo-llvm-cov v0.9.1](https://github.com/taiki-e/cargo-llvm-cov/releases/tag/v0.9.1).
- [codecov-action v7.1.1](https://github.com/codecov/codecov-action/releases/tag/v7.1.1).
- [actions/checkout v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1).
- [taiki-e/install-action v2.87.29](https://github.com/taiki-e/install-action/releases/tag/v2.87.29).

Treat these as a dated implementation starting point, not permanent version
requirements. Recheck stable releases and compatibility when implementing or
updating the workflow. Resolve SHA pins from release tags on the normal release
history. In particular, examples still showing Codecov `@v5` are not evidence
that v5 is the newest release; the reviewed v7.1.1 action still documents and
implements the chosen OIDC and explicit-report inputs.

### Generate one Linux line-coverage report with cargo-llvm-cov

Add `.github/workflows/coverage.yml` with a stable `coverage` job on an Ubuntu
runner, a bounded job timeout, `pull_request` events targeting any branch, pushes
to `main`, and manual dispatch for baseline/recovery runs. Do not filter paths:
documentation-only PRs should still have predictable checks. Cancel superseded
runs only within the same PR/ref.

Install stable Rust with `llvm-tools-preview` and a versioned `cargo-llvm-cov`.
For CI, use a release-SHA-pinned `taiki-e/install-action` with an explicit
`tool: cargo-llvm-cov@<version>`, default checksum verification enabled, and
`fallback: none`. Its maintained prebuilt installation avoids compiling the
coverage tool on every run. Do not SHA-pin the moving `@cargo-llvm-cov` shorthand:
the install action explicitly advises pinning a normal release instead. Install
LLVM tools from the same Rust toolchain used for tests, not an unrelated system
LLVM package. Local installation instructions can still use Cargo.
LCOV is a text format recording executable source lines and their hit counts.
Use `cargo llvm-cov --locked --workspace --all-targets --lcov --output-path
target/coverage/lcov.info`, creating the report directory first. This runs library,
binary, and integration test targets, including Unix terminal tests. Use the
tool's normal fresh instrumentation rather than reuse old profiles. Retain its
default exclusions for external dependencies, build outputs, and integration-test
sources; do not exclude production modules to improve the percentage. Inline unit
test code follows stable tooling behaviour; this is not an exact measurement of
production-only lines or all platforms.

Verify LCOV has source records for `src/main.rs` and the library modules, with
nonzero hit counts from CLI subprocesses. Instrumented child processes must
inherit LLVM profile settings, including those launched through a pseudo-terminal.
Existing tests deliberately kill some children, so those children may not flush
profiles; do not hide such limitations or skip tests to make coverage pass.

LCOV gives a familiar line-coverage metric supported by Codecov. Tarpaulin would
introduce a different tracing model; nightly region/branch/doctest instrumentation
adds complexity unnecessary for this increment. Linux keeps the initial scope
small without claiming platform-wide coverage.

### Authenticate through OIDC and keep PR execution unprivileged

OpenID Connect (OIDC) lets GitHub issue a short-lived identity token that Codecov
verifies, instead of the workflow storing a long-lived upload credential.
Use the maintained `codecov/codecov-action` with `use_oidc: true`. Grant
`contents: read` and job-scoped `id-token: write`; do not grant repository write,
PR write, or status-write permissions to the test job. The installed Codecov app
publishes its own checks. Pin actions to reviewed full commit SHAs with readable
release comments and select current compatible releases during implementation.
Disable checkout credential persistence because this workflow never pushes.
Keep Codecov uploader integrity verification enabled; do not set
`skip_validation: true` or use a legacy uploader to bypass setup failures.

Pass only `target/coverage/lcov.info` using `files`, disable report searching, and
set `fail_ci_if_error: true`. Upload only after tests and report validation succeed.
Do not suppress failures with `continue-on-error` or pretend an upload is complete
before Codecov processes it.

OIDC avoids maintaining an upload secret and the separate Dependabot-secret path.
This does not establish that every bot event can obtain an OIDC token: GitHub
applies special restrictions to Dependabot-triggered runs. Verify those events
explicitly rather than assume a same-repository head grants normal permissions.
Confirm Codecov recognises this private repository and accepts GitHub OIDC before
declaring the integration operational. A repository upload token is a documented
fallback only if OIDC proves unavailable and the maintainer agrees; it belongs in
GitHub secrets, never tracked files, URLs, or logs.

Use `pull_request`, never `pull_request_target` to execute contributed code.
Fork PRs still generate coverage, but skip uploading when the head repository
differs from the base repository. Explain the skip in the job summary rather
than claim Codecov checks ran. This is an explicit security limitation, including
if the repository later becomes public. Do not request broader permissions or
send secrets to fork code as a workaround. Same-repository bot PRs use the same
OIDC route; validate actual event-permission behaviour during rollout.
This skip policy is a scoped choice, not a Codecov limitation: the action and
token documentation support tokenless public-fork uploads. A separate public-fork
upload design can use that supported route without executing PR code through
`pull_request_target` or sharing secrets; it need not copy the current skip policy
forever. No public upload-authentication settings are changed in this increment.

### Make coverage comparisons explicit without a fixed target

Add root `codecov.yml` with `codecov.branch: main`. Configure both project and
patch default statuses with `target: auto`, `threshold: 0`, and
`informational: false`. Project compares overall coverage with the base; patch
measures changed executable lines against Codecov's automatic target. These are
real success/failure checks, but not an arbitrary 80% or 100% floor. Keep defaults
for unrelated features rather than add flags, components, or comment policy.

An initial processed `main` report is needed for meaningful comparisons. Missing
base data is a bootstrap condition, not proof of full coverage. Document the
difference between the Actions `coverage` job (test/report/upload success) and
asynchronous Codecov project/patch statuses (coverage assessment). Creating checks
does not make them required for merging; branch protection is a separate decision.

### Add a main-specific shield without changing unrelated badges

Insert coverage after dependencies and before license in the current header,
keeping the second row's repo, stars, and sponsors badges unchanged. Use:

- Image: `https://img.shields.io/codecov/c/github/hydratic-applications/hydragen/main?style=for-the-badge`, with `height="20"` to match the header.
- Link: `https://app.codecov.io/gh/hydratic-applications/hydragen/tree/main`
- Alternative text: `Code coverage on main`

Use existing HTML `<a><img></a>` markup, escape query separators as `&amp;`, and
allow ordinary wrapping without fixed widths. Do not hardcode a percentage or
embed generated coverage images in Git.

Shields.io documents a dedicated Codecov badge token for private repositories.
Before adding one, obtain the maintainer's approval: a README URL exposes that
token to anyone who can read the file and to Shields.io and image proxies. It
must be the badge-access token from Codecov's badge settings, never an upload,
API, or GitHub token. Do not retrieve or commit it as part of proposal creation.
**Implementation decision:** The maintainer chose a token-free badge URL. Do not
retrieve or embed a badge token; accept an unavailable private badge and verify
the numerical shield after public report access is enabled.
Without approved badge access, retain the token-free live URL and document the
unavailable state and remaining setup. Do not claim the numerical private badge
works; live acceptance remains incomplete until it does. A GitHub Actions secret
cannot be interpolated into an ordinary README image URL.

When the maintainer separately makes the repository public, verify that Codecov
also exposes the main-branch report publicly. The same branch-specific Shields.io
URL and report link can then work without a badge token. Document removing any
previously approved badge token from the README and revoking it in Codecov if no
longer needed; removing it from the current file does not remove it from Git
history. Track these publication-time actions in the maintained
[going-public checklist](../../../docs/going-public-checklist.md), rather than
relying on this change's eventual archive as a reminder.
Keep OIDC uploads unchanged. Public visibility alone does not establish
successful reporting, and it does not change this increment's fork-upload policy.

Keep ordinary build/test commands unchanged. Add a short local coverage example
and link under README Development to `docs/coverage.md` for OIDC, private access,
fork limitations, status semantics, bootstrap, and troubleshooting. Essential
usage remains readable if all badge services are unavailable. The reference also
explains the later public-access checks and optional badge-token cleanup, without
making publication a setup step for this change.

## Risks / Trade-offs

- **Private repository access or account entitlement is incomplete** → Verify in
  Codecov; report the prerequisite rather than change visibility or billing.
- **Badge access discloses private coverage metadata** → Obtain approval for the
  dedicated token and explain URL exposure; leave unavailable access explicit.
- **Terminal tests behave differently or time out under instrumentation on Linux**
  → Exercise the complete suite on Linux, retain bounded waits and cleanup, and
  investigate any failure rather than disable the relevant tests.
- **Coverage is a limited quality signal** → Explain the line/Linux scope and
  keep behavioural tests and human review authoritative for correctness.
- **Network failures fail the coverage job** → This is deliberate so missing
  uploads are visible; maintainers can retry without altering code or thresholds.
- **Forks have no upload or Codecov assessment** → Keep local test evidence and a
  clear skipped-upload summary; revisit a secure fork strategy separately.
- **Existing badge specs describe stale source markup** → Replace only affected
  layout, label, and allowed-claim requirements; preserve all existing destinations
  and unrelated service behaviour.

## Migration Plan

1. Verify private Codecov access and plan entitlement; resolve badge-token consent
   if a working private numerical shield is required immediately.
2. Add the workflow, Codecov settings, badge, and focused documentation; validate
   YAML, action inputs, complete tests, and report paths locally and on Linux.
3. Use an authorised implementation PR to observe the Actions job and Codecov
   processing. A first PR may lack base coverage; record that limitation.
4. After merge, confirm the push run produces the first processed `main` report.
   Check the branch-specific badge after cache refresh, then observe comparison
   statuses on a subsequent same-repository PR. Do not fabricate this evidence
   before an authorised push/PR/merge is available.
5. Roll back by removing the workflow, Codecov settings, badge, and coverage docs
   in a normal reviewed change. No CLI data or runtime migration is involved.

## Open Questions

- Does the current Codecov account expose this private repository with the needed
  coverage/check access? Verify during implementation; app installation alone is
  not evidence of successful processing. The maintainer approved proceeding with
  both project and patch configuration while account-dependent behaviour remains
  pending live verification, aiming for Ratatui's public coverage experience.
- **Resolved — badge access:** Keep the URL token-free, as explicitly selected by
  the maintainer. An unavailable private shield does not block the CI route.

## Sources

Reviewed on 2026-10-18; verify current releases and service behaviour at implementation:

- [Codecov's official Rust example](https://github.com/codecov/example-rust/tree/e7ff5cca324870002988e59a1cd7a600b58c3305), especially its workflow and grcov configuration; distinguish these from its Travis/kcov README instructions.
- [Rust instrumentation-based coverage](https://doc.rust-lang.org/rustc/instrument-coverage.html).
- [cargo-llvm-cov v0.9.1 CI guidance and limitations](https://github.com/taiki-e/cargo-llvm-cov/blob/v0.9.1/README.md).
- [install-action installation and pinning guidance](https://github.com/taiki-e/install-action#readme).
- [Codecov v7.1.1 OIDC guidance](https://github.com/codecov/codecov-action/tree/v7.1.1#using-oidc) and [action inputs/implementation](https://github.com/codecov/codecov-action/blob/v7.1.1/action.yml).
- [Codecov token and public-fork authentication rules](https://docs.codecov.com/docs/codecov-tokens).
- [GitHub's Dependabot workflow restrictions](https://docs.github.com/en/code-security/dependabot/troubleshooting-dependabot/troubleshooting-dependabot-on-github-actions).
- [cargo-llvm-cov usage, CI, exclusions, and limitations](https://github.com/taiki-e/cargo-llvm-cov#readme)
- [Codecov action inputs and OIDC authentication](https://github.com/codecov/codecov-action#readme)
- [Codecov project and patch status configuration](https://docs.codecov.com/docs/commit-status)
- [Shields.io branch-specific Codecov badge and private token](https://shields.io/badges/codecov-with-branch)
