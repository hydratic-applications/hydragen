## 1. Add the test-only launcher

- [x] 1.1 Add a Unix-only `libc` development dependency and update the lockfile without changing production dependency versions.
- [x] 1.2 Add a focused direct-process constructor to the terminal-test session helper using pre-opened PTY handles, `setsid()` and verified absence of a controlling terminal. Keep the post-fork callback allocation-free and avoid shell redirections.
- [x] 1.3 Reuse session infrastructure with immediate child ownership guards and prompt release of parent-side slave handles. Track child termination separately from output draining, and do not signal a known-reaped child or treat a status-observation error as termination.
- [x] 1.4 Add the new launcher's nonblocking output readers and explicit shutdown notification. Implement the separate shared 10-second cleanup deadline, bounded termination checks, forceful kill fallback, confirmed worker completion before joining and incomplete-cleanup diagnostics. Preserve the existing regression's normal reader behaviour and stop for scope review if a broad rewrite is needed.

## 2. Cover unavailable and connected input

- [x] 2.1 Add Name and full-format-frame input-loss cases through the new launcher. Close every stdin master handle without injecting answer bytes and assert the specified single error, styling, no-files report, empty stdout and empty project directory.
- [x] 2.2 Add a connected-input control that completes guided init through the same launcher and verifies the expected layer manifest and single guided completion report.
- [x] 2.3 Exercise launcher verification failure, a child that ignores SIGHUP, and a reaped child with output still pending or a deliberately retained output slave. Verify readers stop and release descriptors without waiting for EOF or signalling the reaped child.
- [x] 2.4 Exercise failed termination/status observation and incomplete cleanup with a controlled test double where necessary. Verify the separate cleanup deadline, useful PID/status/reader diagnostics, no unbounded waits or joins, and no false passing result when reader shutdown replaces EOF.
- [x] 2.5 Confirm the existing selected-input regression, termios readiness helpers, deadlines and assertions remain intact, and that no new case claims guaranteed before-selection injection.

## 3. Validate the maintained implementation

- [x] 3.1 Run `cargo fmt --check`, locked all-targets Clippy with warnings denied, and the full locked macOS test suite. Confirm ordinary guided use, cancellation, non-interactive routes, styling and schema-validation tests still run.
- [x] 3.2 Run the full locked Linux suite and instrumented Linux suite, with the new cases actually executed and no skips or retries. Retain bounded waits, output draining and owned-child cleanup.
- [x] 3.3 Record actual commands, toolchains, results, cleanup checks and remaining platform limitations in a focused verification record. Separate scratch feasibility evidence from maintained-suite results.

## 4. Review and delivery

- [x] 4.1 Obtain independent code and evidence review for process safety, coverage limits, existing-regression preservation and the absence of production or coverage-policy changes. Address findings and repeat affected checks.
- [x] 4.2 Reconcile the proposal, design and test-ci delta with the reviewed implementation, reread them for first-time-reader clarity, and run strict OpenSpec validation and diff checks. Do not weaken CLI policy or rewrite archived evidence.
- [x] 4.3 Deliver a small linked Hydragen PR describing the local test change and deferred runtime limitation. Do not merge, publish the console issue or remove worktrees.
