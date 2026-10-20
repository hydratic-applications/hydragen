## Context

`Cargo.toml` uses Rust edition 2024 but has no `rust-version`. The README says
Rust 1.96.0 was tested; `.github/workflows/coverage.yml` tests rolling `stable`.
An edition, a previously tested compiler, and a minimum supported compiler answer
different questions. The MSRV is the oldest compiler Hydragen commits to
supporting, not necessarily the oldest one on which some code happens to compile.

The user selected an explicit MSRV in Cargo metadata with a matching shield.
This supports the foundations' DX and efficiency values by making build
requirements clear and avoiding contradictory version claims. It does not
change the product model or require a foundations revision. Question 8 in
`openspec/design-questions.md` is relevant only to documenting this implementation
support policy; Rust's wider motivation and its place in the foundations remain
unresolved.

The current README has a centred HTML header with eight badges. The pending
`configure-codecov` delta captures that implemented layout; the base badge spec
still describes an older seven-badge Markdown paragraph. Preserve the implemented
header and combine overlapping deltas without losing the Codecov requirements.

## Goals / Non-Goals

**Goals:**

- One authoritative MSRV in Cargo metadata, verified with locked builds and tests.
- A dynamic, accessible main-branch MSRV badge and concise local build guidance.
- Automated detection of version drift, independent of external badge services.
- Continued stable-compiler and coverage testing alongside the MSRV check.

**Non-Goals:**

- Finding the oldest possible compiler by downgrading dependencies or rewriting
  code, testing every intervening Rust release, or promising new platforms.
- Pinning contributors to an exact toolchain or requiring Rust to run a built CLI.
- Moving coverage tools, formatting, or linting onto the MSRV compiler.
- Publishing a crate, exposing the repository, adding badge credentials, changing
  runtime behaviour, or adding an MSRV command to the CLI.

## Decisions

### Make Cargo metadata the support contract

Use `[package].rust-version` with a full stable version, including a patch
component, so CI can install the exact baseline rather than silently selecting a
later patch. Keep edition 2024 unchanged. The README explains the minimum and
recommends current stable for ordinary development; newer stable compilers are
expected to work, with CI checking the baseline and rolling stable rather than
every version in between. The claim applies to Hydragen's source builds and tests
with its checked-in `Cargo.lock`, not every future dependency resolution.

Raise the baseline deliberately when code or dependency requirements need it, in
the same reviewed change as the metadata and documentation updates. Do not
automatically advance it whenever stable advances. This is a prototype support
policy, not a commitment to maintain older release branches or a release cadence.

Alternative: show only the last-tested compiler. Rejected because it does not
provide the minimum-version contract requested by the user.

### Verify a candidate before making a numeric claim

Read-only `cargo metadata --locked --offline` on 2026-10-19 reports
`serde-saphyr` 2.0.0 requiring Rust 1.89. This identified 1.89.0 as the first
candidate, rather than proving compatibility from metadata alone. Edition 2024
would suggest an older floor and ignore dependencies and Hydragen's source code.

Rust 1.89.0 subsequently passed `cargo +1.89.0 build --locked` and
`cargo +1.89.0 test --locked`: all 190 Rust tests passed on Apple Silicon macOS.
The [verification record](verification.md) identifies the compiler, source commit,
locked inputs, commands, and limits. Choose **1.89.0** for the initial declaration.
No dependencies or source files were changed to achieve compatibility.

Repeat the check on CI's Linux platform and rerun it before committing if
implementation changes affect the tested inputs. Do not bypass version checks,
silently change the lockfile, downgrade dependencies, or replace test evidence
with metadata inference. Toolchain download or infrastructure failure is not
evidence of compiler incompatibility.

Alternative: declare 1.96.0 because the README mentions it. Rejected because a
historical successful build does not establish the best initial support floor.
An exhaustive `cargo-msrv` search is unnecessary for this focused increment.

### Use a dynamic shield sourced from the public main branch

The repository is public, confirmed with GitHub on 2026-10-19. No token or private
access mechanism is required. Shields.io's
[dynamic TOML badge](https://shields.io/badges/dynamic-toml-badge) can read
`https://raw.githubusercontent.com/hydratic-applications/hydragen/main/Cargo.toml`
using the JSONPath selector `$.package['rust-version']`. A live probe against
Hydragen's existing `package.edition` returned `2024`; a second probe of the exact
hyphenated-key selector against Serde's public Cargo file returned its declared
`1.56`. Hydragen's own numeric MSRV result cannot be verified until its declaration
is available on `main`.

Use this image URL, with HTML-safe `&amp;` separators in the README source:

```text
https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fhydratic-applications%2Fhydragen%2Fmain%2FCargo.toml&query=%24.package%5B%27rust-version%27%5D&label=Minimum%20Rust&style=for-the-badge&color=D34516&logo=rust&logoColor=white
```

Place the image immediately after the language badge. Retain 22px height, the
centred HTML structure, and wrapping. Link to `#build-and-run-locally` and use
alternative text `Minimum supported Rust version on main`, without a hardcoded
number that could go stale. The build prose gives the local checkout's exact
minimum, replaces the old last-tested sentence, and briefly explains that the
badge tracks `main`. A branch or historical README intentionally still displays
main's minimum, just as the coverage badge displays main's coverage. Source users
must follow their checkout's `Cargo.toml` and local build guidance.

Use the visible label `Minimum Rust` and a white Rust logo, rather than expecting
readers to know the abbreviation MSRV. Keep the existing badge style and colours.
With this wider label, place status, language, minimum Rust version, dependencies,
and coverage in the first row; place license, repo, stars, and sponsors in the
second. Moving only the row break preserves all badge links and their relative
order while distributing their widths more evenly. Both groups can still wrap
on narrow displays; do not use fixed widths to force equal rows.

The displayed value follows changes merged to `main` after normal Shields.io and
image-proxy cache refreshes. Do not promise instantaneous synchronisation or a
branch-aware result. Before the first declaration reaches `main`, or during a
service failure, retain the real URL and local guidance rather than invent a
version or treat image availability as a compatibility test.

A small standard-library Python check reads local `Cargo.toml` using `tomllib`,
checks its minimum against the README's build sentence, and validates that the
badge uses the intended TOML endpoint, public main-branch source, field selector,
and main-scoped alternative text. It does not compare the live main result with
an unmerged branch's version. Keep it and its regression tests in maintained
locations such as `scripts/check_rust_version.py` and `tests/rust_version/`, never
loading files from `openspec/`. Missing or inconsistent claims fail with recovery
guidance. Use the same parsed Cargo value for the CI toolchain, without a second
version literal in workflow YAML. CI never needs to fetch the image.

Alternative: a static Shields.io badge with its number checked against Cargo in
the MSRV job. This is a valid fallback if dynamic access becomes impractical,
but is not selected now that token-free public access and the selector work.
A crates.io MSRV badge would instead describe a published crate, not `main`.

### Add an independent MSRV job without changing existing checks

Add a separate job named `MSRV`, with bounded execution, read-only repository
permissions, no secrets, and checkout conventions matching the existing workflow.
Run on PRs, main pushes, and manual
runs. Read and validate `rust-version`, check README consistency, run the checker's
regression tests, install that exact compiler, then build and test with `--locked`.
Explicitly select the compiler for Cargo commands so a runner's default cannot
mask an incompatibility. Use Python 3.11 or newer for `tomllib`.

Validate the parsed Cargo string before emitting any Actions output, environment
value, or toolchain argument. Accept only a canonical ASCII numeric
`major.minor.patch` string, with no leading zeroes except for a component equal to
zero. Reject aliases (`stable`, `beta`, `nightly`), prereleases, custom toolchain
names, options, whitespace/newlines, and shell metacharacters. Pass the validated
value through a step environment variable and quoted command arguments, never as
interpolated shell source or through `eval`. Explicit selectors such as
`cargo "+$MSRV" test --locked` must override runner defaults. Regression tests
must cover rejected values and prove that invalid input is not emitted for later
steps. Log actual compiler and Cargo versions before running the build and tests.

Leave the existing `Tests` and coverage job names, dependencies, measurements,
toolchains, gates, and upload policy unchanged. Do not make coverage depend on
the MSRV job or run coverage and its auxiliary tools twice. Repository branch
protection is a separate maintainer setting, not an automatic effect of adding
the job.

Alternative: turn the existing tests job into a matrix. A separate job provides
clear baseline failures without renaming the existing check or coupling it to
coverage policy changes already under way.

### Keep quality-check documentation and overlapping specifications consistent

The pending `separate-test-and-coverage-checks` specification says Actions owns
"exactly three quality policies". Scope that enumeration to its stable-test and
coverage responsibilities: `Tests`, `Total coverage`, and `Changed-line coverage`.
The new `MSRV` job is an additional compiler-compatibility check with its own
visible result, not a coverage policy or pipeline-health job. Neither normal
tests nor either coverage gate depends on it, and it does not depend on them.

During implementation, revise the existing CI explanation in `README.md`,
`docs/coverage.md`, and CI-related entries in `docs/going-public-checklist.md` to
reflect the extra check without implying the previous three are the complete CI
set. Preserve existing statuses, gates, upload rules, and branch protection.
Do not turn this into a general rewrite of the publication checklist.

The new coverage delta copies the complete affected requirements from
`separate-test-and-coverage-checks`. Sync overlapping specifications in this
order: `configure-codecov`, then `separate-test-and-coverage-checks`, then
`declare-rust-msrv`. This preserves the eight-badge baseline and removes its old
MSRV prohibition before adding the ninth badge; it also preserves the independent
coverage gates before narrowing their enumeration. If another change is not ready
to sync or has changed, rebase and validate these deltas against the agreed
baseline rather than silently archiving it or restoring stale requirements.

The spec reconciliation also applies the existing `for-the-badge` decision to
older deps.rs, stars, and sponsors URL examples, and retains the HTML source
conventions that replace Markdown shortcut references. The earlier optional
private coverage badge-token route is superseded by the later token-free shield
requirement in `separate-test-and-coverage-checks` and the implemented coverage
reference. Retain the historical-token revocation guidance, but do not preserve
a contradictory requirement to add a token. These reconciliations are recorded
in this change's badge delta so subsequent sync does not restore stale rules.
They change specifications only, not existing badge URLs, CI or runtime behaviour.

During integration, `main` advanced to `328644ec72012e11ae01ffbf7213694ebd40e7be`,
including the already synced and archived coverage changes and the local-add
implementation. Resolve against that maintained baseline rather than replaying
old archived deltas over newer decisions. Preserve the newer Codecov path-fix
requirement and uploader-inventory scenario, the external-account-setup scenario,
and the native-colour Codecov icon contract. Keep MSRV's independent result and
the explicit token-free reconciliation above. Preserve all automatically merged
source and dependency changes from main; rerun both compiler routes and Linux CI
against the integrated implementation before relying on its compatibility claim.

Stable Clippy 1.99.1 flagged main's YAML key/value scan for using
`chunks_exact(2)`. Use `as_chunks::<2>().0.iter()` instead, retaining the same
complete-pair iteration and ignored remainder. This standard-library API is
available on the declared compiler; verify the adjustment on both compilers
without suppressing the lint or raising the MSRV.

## Risks / Trade-offs

- **Later source or test changes can invalidate the initial verification.** →
  Rerun the baseline checks for changed inputs before committing and in CI.
- **Dependency updates can raise compiler requirements.** → Locked CI must fail
  until maintainers either keep compatible dependencies or deliberately raise
  the baseline and all checked claims together.
- **The dynamic badge tracks main, not an unmerged branch, and can be cached.** →
  State its scope, keep checkout-specific requirements in local metadata and
  checked prose, and do not compare a live main value with a feature branch in CI.
- **External images can fail or be cached.** → Keep requirements in local prose
  and metadata, with accurate alternative text and no build-time image fetching.
- **CI covers only selected compilers and its existing test platform.** → Describe
  the evidence honestly; do not imply every compiler or OS was tested.
- **Badge and coverage deltas overlap pending changes.** → Apply the documented
  sync order or rebase deliberately, preserving the HTML layout, independent
  gates, and permission boundaries. Do not silently archive another change.

## Migration Plan

Verify and record the compiler baseline, then add metadata, documentation,
consistency checks, and CI together. Run both compiler test routes, checker
regressions, and README layout review before merging. No user data migration is
needed. Before merge, observe a successful Linux `MSRV` job for the actual
implementation revision and record its run URL, measured commit, logged
compiler/Cargo versions, and successful locked build and full-test results.
Workflow syntax checks or the existing macOS result do not satisfy that acceptance
item. The live numeric badge check remains post-merge. If the support claim must
be withdrawn, remove the declaration, badge, minimum-version prose, and associated
check together rather than leave conflicting
claims. Do not remove pre-existing tests or coverage infrastructure.

## Open Questions

- Verify the real numeric Hydragen shield after the declaration reaches `main`;
  successful endpoint probes alone do not verify that future service result.
- Whether to require the new MSRV status in branch protection remains a maintainer
  choice and does not block creating the CI job.
