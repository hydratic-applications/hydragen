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
remain open. The PR remains a draft; no merge or archive has been performed.
