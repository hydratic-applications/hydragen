# Codecov implementation verification

## Current closure — 2026-10-19

The rollout is verified on public `main` revision
`6ccf96b635f99c3ef62d91961b8d9d6218fbf382`. [Run 39208602693](https://github.com/hydratic-applications/hydragen/actions/runs/39208602693)
passed all five jobs, including isolated OIDC uploading. Codecov processed all
nine source files at **2,379/2,475 covered lines (96.12%)**. Anonymous access,
run/revision identity, report digest, immutable source hashes and per-file counts
were independently checked against the retained current-run artifact. The
unchanged token-free main shield returned **96%**, with its native Codecov icon,
`for-the-badge` style and 22px README height. See the
[public completion evidence](https://github.com/hydratic-applications/hydragen/pull/10#issuecomment-6260969474)
and the [follow-up's final rollout record](../2026-10-19-separate-test-and-coverage-checks/actions-gates-verification.md#final-main-and-public-rollout--2026-10-19).

Task 5.2 is now verified. Task 5.3 is **superseded, not executed**: the
[follow-up design](../2026-10-19-separate-test-and-coverage-checks/design.md) replaces
Codecov project/patch assessments with independent Actions gates and disables
those service statuses. Closure is **17 verified tasks plus 1 superseded task**,
not 18 executed tasks or proof of successful Codecov comparisons. The dated
observations below preserve earlier private-access and pre-merge checkpoints;
they do not describe the current repository or rollout status.

## Prerequisite check — 2026-10-18

- Opened `https://app.codecov.io/gh/hydratic-applications/hydragen` in the browser.
  The browser is signed out; Codecov shows a not-found page explaining that
  private repositories require login. This does not establish whether the
  repository is configured or whether the account supports private coverage.
- Codecov setup: [Coverage Analytics setup page](https://app.codecov.io/gh/hydratic-applications/hydragen/new).
  Successful report processing remains unverified at this stage.
- Codecov's official pricing page, checked 2026-10-18, lists 250 private
  uploads for the free Developer plan and unlimited public uploads:
  <https://about.codecov.io/pricing/>.
- That pricing page lists Patch Coverage and Status Checks on Developer, but
  marks Project Coverage unavailable on Developer (both private and open-source)
  and available on Pro. This marketing label alone does not establish whether
  ordinary aggregate coverage badges, report views, or a particular project
  status are restricted for this account. Do not infer a paid-plan requirement
  for the README shield from that row.
- The maintainer supplied Ratatui as a public example. Its Codecov page exposes
  branch coverage totals, its `codecov.yml` configures a project status, and its
  CI uses stable Rust, `cargo-llvm-cov`, and Codecov action v7.1.1. None of this
  reveals Ratatui's billing plan or proves a paid subscription. Codecov's official
  open-source page explicitly advertises free coverage and status checks:
  <https://about.codecov.io/for/open-source/>. Retain account-dependent behaviour
  as a live-verification question rather than a confirmed paid-plan blocker.
- The maintainer approved proceeding with both project and patch configuration,
  aiming for coverage reporting comparable to Ratatui where the account supports
  it. Task 1.1 remains incomplete only for account-specific private processing and
  feature verification. No trial, upgrade, visibility, membership, billing, or
  account-setting changes were made.
- Task 1.2 is complete: the maintainer chose to keep the badge URL token-free.
  The private shield may show an unavailable result until public access is
  enabled. Do not retrieve or embed a badge-access token.

## Implemented and locally verified — 2026-10-18

- Added `.github/workflows/coverage.yml`, root `codecov.yml`, a main-specific
  token-free README shield, local coverage instructions, and `docs/coverage.md`.
  Kept the going-public checklist and clarified that token revocation is
  conditional: this implementation introduces no badge token.
- Rechecked the official Rust example, current Rust/cargo-llvm-cov guidance,
  Ratatui's workflow, current action releases, and supported inputs. Release
  commits used by the workflow:
  - checkout v7.0.1: `3d3c42e5aac5ba805825da76410c181273ba90b1`.
  - install-action v2.87.29: `8cca0918f8471c33515202b73f56ab27f2bed283`.
  - codecov-action v7.1.1: `303a32d7a59b442fa8d48b6a1cc6825c09c847a5`.
  - cargo-llvm-cov is explicitly versioned at 0.9.1; checksum verification remains
    enabled, with the install action's fallback disabled.

### Static workflow and report checks

- `actionlint .github/workflows/coverage.yml` passed.
- `curl --data-binary @codecov.yml https://codecov.io/validate` returned `Valid!`,
  confirming the main branch, automatic project/patch targets, zero threshold,
  and non-informational statuses.
- Parsed the actual workflow and checked each pinned action's input names against
  its pinned `action.yml`. Verified event coverage, scoped permissions, the
  timeout, disabled credential persistence, explicit report selection,
  `use_oidc: true`, `disable_search: true`, and `fail_ci_if_error: true`.
- Evaluated the actual upload/skip expressions for main pushes, manual dispatch,
  same-repository PRs, and fork PRs. Only forks skip uploading. GitHub's implicit
  success condition prevents either post-validation step from reporting success
  after a preceding failure. Actual bot/OIDC permissions remain a rollout check.
- Executed the workflow's actual Python report validator against generated
  fixtures: it accepted a valid library/CLI report and rejected empty reports,
  missing CLI source, zero CLI hits, and malformed hit counts. Also ran that exact
  validator successfully against the real Linux report in a read-only container.

### Rust tests and coverage

- macOS, Homebrew Rust 1.96.0: `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` passed.
- The first `cargo test --locked` run failed
  `losing_terminal_input_reports_failure_without_creating_a_layer_manifest`.
  That same existing test passed when rerun in isolation, and the entire suite
  then passed on a second full run: **186 tests**, plus zero doctests. No Rust
  source or test was changed, skipped, or ignored. Retain this observation as a
  potential timing-sensitive terminal-test issue rather than hide the first run.
- Instrumented Linux run: Rust 1.99.1, LLVM 23.1.1, cargo-llvm-cov 0.9.1, native
  `aarch64-unknown-linux-gnu` in a disposable Debian Bookworm Rust container. The
  image digest was
  `sha256:a24e292b4af7bdee351a31d3a6379972a5652f0ea22a1b6d6883324ac4f5097c`.
  The container had a 30-minute outer timeout, init-based child reaping, and
  automatic removal; source was mounted read-only and artifacts stayed under
  the ignored `target/coverage-linux/` directory.
- Installed matching LLVM tools. Verified the downloaded cargo-llvm-cov release
  archive against SHA-256
  `abf5f13c1520f8756d2192bfaaeadb0208f5b7eaa8cadc15bf847befa42b7360`.
  An initial container setup attempt stopped before tests because the slim image
  lacked curl; downloaded and verified the archive on the host for the subsequent
  successful run. This did not change the GitHub-hosted workflow.
- Ran `cargo llvm-cov --locked --workspace --all-targets --lcov --output-path
  /artifacts/lcov.info` with only the target/report paths relocated into the
  container's artifact mount. **All 186 tests passed**, including all 25 terminal
  tests; no production exclusion or coverage threshold was added.
- The generated report contains all nine Rust source files, with **2,348 of 2,443
  executable lines covered (96.11%)**. This is a local feature-branch measurement,
  not a published `main` result or a promised Codecov percentage after service
  processing. Inline unit-test code follows stable tooling defaults.
- CLI subprocess data is present: the executable's `main` function has a nonzero
  execution counter (508), rather than only a zero-count unit-test variant.
  Deliberately killed processes can still miss their final counter flush; no
  tests were omitted to avoid that limitation.

### README and documentation

- Initially checked the actual linked image URL, explicit `main` branch,
  `flat-square` style, `Code coverage on main` alternative text, unchanged existing badge
  destinations, and absence of badge-token query parameters.
- Rendered the actual revised README in a local preview, checking desktop
  (1440px) and mobile (390px) layouts. The header fits/wraps without sideways badge
  scrolling; the live coverage shield currently displays `unknown`, as expected
  without a processed publicly accessible main report.
- During PR preparation, `main` contained newer badge styling commits. The
  maintainer approved matching their `for-the-badge` style and 20px height for the
  coverage shield. Preserved all of main's existing badge changes and aligned
  the proposal, design, and delta specification with that decision.
- Reviewed README, coverage instructions, and the going-public checklist from a
  first-time reader's perspective. No product, glossary, or foundations change is
  warranted: the change is optional development infrastructure, not CLI behaviour.
- No runtime or Cargo dependency changed; implementation does not load or embed
  assets from `openspec/`. Generated reports, fixtures, and previews stay under
  ignored `target/` paths rather than become project implementation assets.

## First hosted PR run — 2026-10-18

- Opened [implementation PR #8](https://github.com/hydratic-applications/hydragen/pull/8)
  after the maintainer authorised committing, pushing, and opening it. The first
  [Coverage run](https://github.com/hydratic-applications/hydragen/actions/runs/39122559635)
  passed in 1 minute 44 seconds on hosted Ubuntu 24.04 at commit
  `e7c0a3484ff33e8b80460f11abb52e2c7b17a016`.
- All 186 tests passed on the hosted runner, including all 25 terminal tests.
  The workflow's library/CLI report validation passed, GitHub issued the OIDC
  token, and Codecov accepted the upload for processing. Uploader integrity
  verification remained enabled.
- The [processed commit report](https://app.codecov.io/gh/hydratic-applications/hydragen/commit/e7c0a3484ff33e8b80460f11abb52e2c7b17a016/tree)
  showed one merged upload, browsable source coverage, and **2,348/2,443 lines
  covered (96.11%)**. This verifies report processing and aggregate coverage
  viewing, not the availability of every service feature.
- The comparison view reports **Missing Base Commit**, because the default
  branch has no coverage report yet. A Codecov patch check was observed pending;
  no successful project/patch comparison is claimed. This is the expected
  first-report bootstrap condition, not an upload failure.
- The runner emitted an informational notice that `ubuntu-latest` will begin
  migrating to Ubuntu 26 on 2026-10-19. The workflow deliberately tracks the hosted
  Ubuntu label; this notice did not fail the job.

## Historical pre-merge checkpoint — 2026-10-19

### README alignment — 2026-10-19

- Fetched main through `bcda0ca` and integrated its newer wordmark, Rust website
  link, and 22px badge heights. At the maintainer's request, the Codecov shield
  now uses the same `for-the-badge` style at 22px height. Its explicit `main`
  branch, report link, alternative text, and token-free URL are unchanged.
- Aligned the change's current badge requirements with main while preserving the
  earlier 20px styling decision above as historical verification evidence.

**Progress at that checkpoint: 16/18 tasks complete.** Tasks 5.2 and 5.3 were unchecked.

- No merge had yet been authorised or performed. Hosted verification above covered a
  same-repository human-authored PR, not fork or Dependabot runtime behaviour.
- The remaining work was to verify a main report, numerical badge and subsequent
  PR comparisons. Public Codecov report access was still pending; private
  `unknown` was accepted by the maintainer's token-free decision.
- Configuration validation and local coverage alone did not establish those live
  outcomes. The current closure above uses later main/public evidence and explicitly
  identifies the superseded comparison task.
