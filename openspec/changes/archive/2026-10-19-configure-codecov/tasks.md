## 1. Confirm service prerequisites and tooling

- [x] 1.1 Confirm Codecov recognises `hydratic-applications/hydragen`, the installed app has access, and the account supports this private repository; record any setup blocker without changing visibility or billing.
- [x] 1.2 Confirm whether the maintainer approves including the dedicated private badge-access token in the README URL, explaining exposure to readers, Shields.io, and image proxies; never substitute upload or GitHub credentials.
- [x] 1.3 Recheck Codecov's official `example-rust` workflow alongside current Rust, Codecov action, `cargo-llvm-cov`, and installation documentation. Use the design's dated stable-release snapshot as a starting point, not stale example `@v5` references. Resolve reviewed release SHA pins, use stable Rust with matching `llvm-tools-preview`, and install explicit-version prebuilt `cargo-llvm-cov` through a pinned normal install-action release with checksum verification and no fallback.

## 2. Implement coverage generation and uploading

- [x] 2.1 Add `.github/workflows/coverage.yml` with the `coverage` job on Ubuntu for PRs, pushes to `main`, and manual dispatch; include bounded execution and PR/ref-scoped concurrency without path filters.
- [x] 2.2 Generate fresh `target/coverage/lcov.info` using the locked workspace/all-targets command; validate nonempty source records and CLI/library hit data without production-code exclusions or skipped terminal tests.
- [x] 2.3 Add OIDC-authenticated Codecov uploading with explicit report selection, search disabled, uploader integrity verification enabled, and upload errors failing CI; request only contents read and job-scoped identity-token issuance, and disable checkout credential persistence.
- [x] 2.4 Guard uploads against fork PRs while preserving test/report execution and an explanatory job summary; review same-repository bot event permissions and ensure no contributed code executes through `pull_request_target`.
- [x] 2.5 Add root `codecov.yml` identifying `main` and enabling project/patch statuses with automatic targets, zero tolerance, and non-informational results, without branch-protection changes.

## 3. Add the shield and focused documentation

- [x] 3.1 Insert the linked main-branch coverage shield after dependencies in the existing README HTML header, matching its current `for-the-badge` style at 22px height as requested after the latest changes on main; preserve other badges, destinations, and wrapping, and use approved badge access only or explicitly document unavailable private access.
- [x] 3.2 Add a concise local coverage example and reference link under README Development; keep ordinary build and test commands unchanged.
- [x] 3.3 Add `docs/coverage.md` covering prerequisites, OIDC, private account and badge access, line/Linux measurement limits, fork and bot behaviour, job versus Codecov status semantics, first-main baseline, caches, and troubleshooting. Link to `docs/going-public-checklist.md` for the separate later public-access checks and badge-token removal/revocation without making publication a prerequisite or a task in this change.

## 4. Validate repository changes

- [x] 4.1 Validate workflow syntax and expressions with actionlint, check pinned action inputs, and validate `codecov.yml` with Codecov's configuration validator; confirm event, permission, fork-skip, and upload-failure behaviour.
- [x] 4.2 Run the complete instrumented suite on Linux, including pseudo-terminal tests with bounded waits and child cleanup; inspect LCOV library/binary records and subprocess hits, reporting any killed-child profile limitations.
- [x] 4.3 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --locked`; confirm no runtime dependency, tracked report output, or implementation asset loading from `openspec/` was introduced.
- [x] 4.4 Verify badge HTML, URLs, main-branch selection, descriptive alternative text, and desktop/narrow rendering; review all revised documentation from a first-time reader's perspective and confirm foundations/glossary need no changes.

## 5. Verify live rollout when authorised

- [x] 5.1 On an authorised implementation PR, observe successful coverage generation, GitHub OIDC upload, and Codecov processing; record links and distinguish missing-base bootstrap from an upload failure. Leave live verification pending if no push/PR is authorised.
- [x] 5.2 After an authorised merge, confirm the main push produces a processed main report and the README shield shows its measured coverage after cache refresh; leave numeric badge acceptance pending if private access is unresolved.
- [x] 5.3 **Superseded, not executed:** Observe project/patch comparison statuses on a subsequent same-repository PR with a processed base report, and record evidence without changing required-check settings or claiming unobserved success. The follow-up replaces Codecov assessments with Actions-owned gates and disables project/patch statuses; this checkbox records closure of the obsolete task, not successful Codecov comparisons.

**Closure — 2026-10-19:** 17 tasks verified and 1 superseded, not 18 executed.
Task 5.2 is supported by the [final main/public evidence](verification.md#current-closure--2026-10-19).
The replacement policy and its verification are recorded in
[separate-test-and-coverage-checks](../2026-10-19-separate-test-and-coverage-checks/design.md).
