# Rust 1.89.0 verification

## Result

On 2026-10-19, the proposed Rust 1.89.0 baseline successfully built Hydragen and
passed all 190 Rust tests on Apple Silicon macOS (`aarch64-apple-darwin`). This
includes unit tests, captured-output CLI tests, and real-terminal interaction
tests. There were no failed or ignored tests; the documentation-test harness
contained no tests.

Both commands exited successfully:

```sh
cargo +1.89.0 build --locked
cargo +1.89.0 test --locked
```

No dependency downgrades, source changes, lockfile changes, or Rust-version check
bypasses were used. The metadata declaration, README badge, and CI job are still
planned work, not implemented by this verification.

## Toolchain and checkout

- Rust: `rustc 1.89.0 (29483883e 2025-08-04)`.
- Cargo: `cargo 1.89.0 (c24e10642 2025-06-23)`.
- Source commit: `6ccf96b635f99c3ef62d91961b8d9d6218fbf382`.
- `Cargo.toml` SHA-256:
  `0ee0b2f8ba251e3a7a5ae9cfd70190f2bbf99256d1843c66e772fa4dcbc52702`.
- `Cargo.lock` SHA-256:
  `c1df7232e8cd9f059f886c37811e4e462b5adfc7802363bd1e8156774e63940a`.
- No changes to source, tests, schemas, or Cargo files were present; pending
  changes were limited to this proposal and its design-question entry.

The official rustup installer was downloaded over HTTPS and checked against its
published SHA-256. `RUSTUP_HOME`, `CARGO_HOME`, and `CARGO_TARGET_DIR` were isolated
in a temporary directory, and the installer used `--no-modify-path`. The existing
Homebrew compiler and user shell configuration were not changed.

The full local log is temporarily available at
`/private/var/folders/p9/02qtjw2138dd0jbmmjb0k45r0000gp/T/opencode/hydragen-msrv-uxxvwlz4/verification.log`.
That file is not a maintained project asset; this document preserves the result
and the identity of the tested inputs.

## Limits and remaining checks

This verifies the current locked checkout on this host, not every Rust release,
target, or future dependency resolution. The proposed CI job must repeat the
locked build and full tests on its existing Linux platform, explicitly selecting
the compiler from `Cargo.toml`. Rerun verification if implementation changes
affect the tested inputs before committing the compatibility claim.

The dynamic badge endpoint was separately verified to read Hydragen's public
Cargo file and a hyphenated `rust-version` key from a public example. Hydragen's
own numeric badge still needs a post-merge check once `main` declares its MSRV.

## Implementation verification on 2026-10-19

After adding `rust-version = "1.89.0"`, the dynamic README badge, the offline
helper and its regressions, and the independent MSRV job, local checks passed:

- Rust 1.89.0: locked build and all 190 Rust tests, including terminal tests.
- Current stable Rust 1.99.1 (`58cf8f9c3`, 2026-10-13), Cargo 1.99.1: formatting,
  locked build, Clippy across all targets with warnings denied, and all 190 Rust
  tests. Stable was installed into the same isolated temporary toolchain home,
  not made the user's default compiler.
- Python 3.14.6: all 23 MSRV helper regressions, including invalid declarations,
  stale README claims, offline operation, dynamic source/query validation, and
  execution of the workflow's actual version-output shell block. Invalid input
  never produced an Actions output value.
- `actionlint` accepted the workflow. Removing only the new MSRV job from the
  file exactly reproduced the previous committed workflow, so all existing jobs,
  dependencies, permissions, gates, and upload policy remained byte-for-byte
  unchanged. The lockfile also remained unchanged.
- All eight existing workflow-boundary and upload-validator regression tests
  passed against the updated workflow. The new job did not alter the isolated
  uploader's execution or permission boundary.
- A local preview rendered through GitHub's Markdown API was reviewed in a real
  browser at desktop (1440px) and phone (390px) widths. All nine badges remained
  styled consistently and wrapped without header overflow. The MSRV link selected
  `#build-and-run-locally`; its descriptive main-scoped alternative text survived
  rendering. The preview used lightweight GitHub-like layout CSS, not a claim of
  pixel-identical GitHub page styling.
- The live dynamic image currently says `NO RESULT`, as expected while public
  `main` lacks `rust-version`. The local prose and Cargo declaration identify the
  minimum independently of that service result.

The tested implementation's `Cargo.toml` SHA-256 is
`3390ad824cd80c8926288add4e9289985f86e3d3ecc2992eb185c2152774b353`;
`Cargo.lock` retains the hash recorded above. Temporary full Rust logs are beside
the original verification log as `implementation-msrv.log` and
`implementation-stable.log`. These checks cover the uncommitted working tree,
not a new committed or remotely tested revision.

At the end of local verification, tasks 4.3–4.5 remained open: ordered
specification sync must preserve the pending
`configure-codecov` and `separate-test-and-coverage-checks` deltas, a real Linux
MSRV CI run must be observed for the implementation revision before merge, and
the numeric dynamic badge must be checked after merging to `main`. Neither local
testing nor `actionlint` substitutes for those external acceptance checks. At that
stage no commit, push, PR, merge, or unrelated archive had been performed. The
subsequent PR and Linux evidence are recorded below. No foundations or
glossary change is needed for this implementation-only support policy.

## Linux CI verification

Draft [PR #13](https://github.com/hydratic-applications/hydragen/pull/13) runs the
implementation commit `bc110bb51da6c9a1912939e94aa7893855b49fa0` in
[Actions run 39335525424](https://github.com/hydratic-applications/hydragen/actions/runs/39335525424).
The run completed successfully on 2026-10-19 UTC and was inspected on 2026-10-20.

The separate [MSRV job](https://github.com/hydratic-applications/hydragen/actions/runs/39335525424/job/118343948252)
ran on Ubuntu 24.04.5, host `x86_64-unknown-linux-gnu`. Checkout logged the tested
commit, and the compiler-identification step logged:

```text
MSRV: 1.89.0
rustc 1.89.0 (29483883e 2025-08-04)
cargo 1.89.0 (c24e10642 2025-06-23)
```

The job passed all 23 Python helper regressions, validated the local declaration
and README, and successfully ran `cargo "+$MSRV" build --locked` followed by
`cargo "+$MSRV" test --locked`. All 190 Rust tests passed with zero failures or
ignored tests, including all 27 terminal-interaction tests. The documentation-test
harness contained zero tests.

The independent [stable Tests job](https://github.com/hydratic-applications/hydragen/actions/runs/39335525424/job/118343948505)
also passed. Both jobs started independently and have separate visible status
results; neither has a job dependency on the other. Coverage report production
passed, total coverage passed at 2379/2475 lines (96.12%), and changed-line coverage
reported N/A with valid evidence of no applicable measured Rust lines. The
main-only Codecov uploader was correctly skipped for the PR.

This satisfies task 4.4 for the implementation revision. The documentation-only
follow-up recording this evidence does not alter implementation inputs. Tasks
4.3 (ordered specification sync) and 4.5 (numeric badge verification after merge)
remained open at that stage. The PR was still a draft; no merge or archive had
been performed.

## Ordered specification sync on 2026-10-20

The main specifications were reconciled from `configure-codecov`, followed by
`separate-test-and-coverage-checks`, then `declare-rust-msrv`. This updated
`openspec/specs/readme-badges/spec.md` and created the maintained
`code-coverage-ci`, `test-ci`, and `rust-version-support` specifications.

The result preserves the centred HTML header, separate test and coverage results,
exact Actions-owned coverage gates, validated evidence, and isolated main-only
OIDC uploads, while adding the ninth badge and independent MSRV contract. The
historical Markdown-reference requirement and Codecov-owned percentage policy
were removed or superseded as specified by the earlier changes.

Two stale badge details needed explicit reconciliation: old `flat-square` URL
examples now follow the already adopted `for-the-badge` style, and the older
optional private coverage badge-token route now follows the later token-free
coverage requirement. Historical-token revocation guidance remains. These
resolutions are included in the MSRV badge delta and design, so a later sync
does not restore the contradictions. No implemented image, workflow, runtime,
Cargo metadata, or lockfile was changed by this reconciliation.

Validation passed for all nine maintained main specifications and all three
affected changes in strict mode. A read-only composition check verified that
all 22 requirements in the four affected capabilities match the ordered deltas
and preserved base content, including the explicit removal and rename. Repeating
the composition produced the same result. Whitespace checks also passed.

Task 4.3 is complete. Only task 4.5, checking the live numeric MSRV shield after
an approved merge, remains open. No other change was archived or marked complete,
and the foundations and glossary remain unchanged because the sync adds no new
product belief or terminology. No merge or archive was performed.

## Current-main integration on 2026-10-20

With approval, main commit `328644ec72012e11ae01ffbf7213694ebd40e7be` was
integrated into the PR branch. This brings in main's local dependency addition
implementation and updated dependencies. The coverage archives, foundations,
glossary, and other changes arriving with that commit were already on main;
this integration did not independently archive or redesign that work.

Conflicts in the coverage, README badge, and test specifications were resolved
against main's maintained contracts. The resolution retains its Codecov path-fix
safeguard, uploader-inventory scenario, external-account-setup scenario, and
native-colour coverage icon, alongside the independent MSRV policy and token-free
badge rule. The MSRV deltas reproduce the resulting requirements when applied
to this main baseline, and repeating that application makes no further changes.
All ten maintained specifications and this change pass strict validation.

The integrated checkout passed locked builds and all **572 Rust tests** on both
Rust 1.89.0 and current stable 1.99.1 on `aarch64-apple-darwin`, with zero failed
or ignored tests. Stable formatting and Clippy across all targets with warnings
denied passed. The only source adjustment relative to main replaces
`chunks_exact(2)` with `as_chunks::<2>().0.iter()` in the YAML key/value scan,
as requested by stable Clippy. It preserves complete-pair iteration and remains
compatible with Rust 1.89.0. No dependencies were downgraded, and `Cargo.lock`
remains byte-for-byte identical to the integrated main commit.

The compiler versions remain those recorded above: Rust/Cargo 1.89.0 for minimum
support and Rust/Cargo 1.99.1 for stable. Tested input SHA-256 hashes are:

- `Cargo.toml`: `9e9824ba1a292eb55fe4028863ccc153e84ec100d843b336fc37a396bf3fd8d5`.
- `Cargo.lock`: `bda80bacec441689a16d045036eeff4f8c8540d098ee1c07f9d1e5f140e65705`.

All 23 MSRV helper tests and all 39 coverage-helper tests passed, including the
real diff comparison and isolated uploader checks. The offline declaration check,
workflow lint, and whitespace checks passed. Removing just the MSRV job exactly
reproduces main's workflow, preserving every existing job and upload boundary.

Two local environment problems were kept separate from compatibility evidence.
The earlier temporary Cargo cache had an incomplete dependency source directory;
the successful runs used a fresh isolated Cargo home populated from cached
archives. One coverage-helper test initially rejected macOS's `/var` temporary
path alias; all 39 passed with `TMPDIR` set to the canonical `/private/var` path.
No source validation or test assertions were weakened to bypass either issue.

Temporary final Rust logs are beside the earlier logs as
`current-main-msrv-verified.log` and `current-main-stable-verified.log`.
The foundations and glossary need no MSRV-specific additions. Task 4.4 is reopened
until Linux CI verifies this integrated implementation; the earlier 190-test
Linux result does not establish compatibility for these changed inputs.
The PR remains ready for review, but has not been merged or archived. The final
numeric main-branch badge check remains post-merge.

## Integrated Linux CI evidence

[Actions run 39345429531](https://github.com/hydratic-applications/hydragen/actions/runs/39345429531)
passed on 2026-10-20 for integration commit
`b5557047cd911b56dd72892920c24c7d52da03b0`, against main base
`328644ec72012e11ae01ffbf7213694ebd40e7be`.

The [MSRV job](https://github.com/hydratic-applications/hydragen/actions/runs/39345429531/job/118376750921)
logged Rust `1.89.0 (29483883e 2025-08-04)` and Cargo
`1.89.0 (c24e10642 2025-06-23)` on Ubuntu, host
`x86_64-unknown-linux-gnu`. It passed the locked build, all 572 Rust tests with
zero failed or ignored tests, and all 23 MSRV helper tests.

The separate [stable Tests job](https://github.com/hydratic-applications/hydragen/actions/runs/39345429531/job/118376750668)
used Rust 1.99.1 and passed all 572 Rust tests and all 39 coverage-helper tests.
Both compiler jobs started independently, with separate visible results and no
job dependency between them. Coverage report production and both gates passed:
total coverage was 10,564/10,787 measured lines (97.93%), and changed-line coverage
was 4/4 measured lines (100%). The main-only Codecov uploader was correctly
skipped for this PR.

This completes task 4.4 for the integrated implementation. Only task 4.5, the
numeric main-branch badge check after an approved merge, remains open. Recording
this evidence changes documentation only, not the tested implementation. No PR
merge, archive, release, or required-check configuration change was performed.

## Clearer badge label and balanced rows on 2026-10-20

Following review, the badge now says `Minimum Rust` and includes a white Rust
logo, making its purpose understandable without knowing the MSRV abbreviation.
It retains the dynamic main-branch source, field query, colours, style, accessible
label, build-section link, and 22px image height. Only the existing row break was
moved: status, language, minimum Rust version, dependencies, and coverage form
the first group; license, repo, stars, and sponsors form the second.

GitHub-rendered header HTML was checked in a local browser preview at 1440px and
390px viewport widths. With an explicitly illustrative 1.89.0 badge value, the
desktop groups measured approximately 736px and 676px including spacing. The
actual dynamic badge, which still has no numeric main-branch result before
merge, also fits the desktop layout. At phone width, the groups wrap naturally
and the page does not overflow horizontally. Preview-only numeric substitution
does not change the real dynamic URL or satisfy task 4.5.

All 24 offline helper tests passed, including the new row-group regression and
expanded checks rejecting the old abbreviation, incorrect or missing logos, and
duplicate parameters. The consistency check, strict validation of this change
and all ten maintained specifications, and whitespace checks passed. The main
badge specification and change delta agree. No Rust source, dependency, lockfile,
workflow, or compatibility baseline was changed in this presentation follow-up.
