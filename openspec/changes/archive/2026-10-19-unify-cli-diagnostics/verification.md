# Verification follow-up: terminal input-loss fixture

## Observation — 2026-10-19

While validating the separate `compact-single-add-results` follow-up, the init
input-loss test unexpectedly completed successfully at its format selector.
Init production code had not changed. Earlier passing suites did not establish
the fixture was correct under every interleaving.

The Unix writer returned by `portable-pty` 0.9.0 `take_writer()` sends a newline
and VEOF from its destructor before closing. Dropping it while the original
master remains open can submit the selector before the subsequent hangup.
See the [dependency source](https://docs.rs/crate/portable-pty/0.9.0/source/src/unix.rs).

A controlled test left both original master and slave open, dropped only that
writer, observed successful init, and independently decoded the expected name,
author and description. This demonstrated an injected submit key, not an
application automatically accepting defaults after a physical disconnect.

## Test-only correction

The loss fixture now writes through a close-only duplicated `std::fs::File` and
closes all input master handles without injecting Enter or Ctrl-D. Setup keys use
the separate stdin terminal's observed raw mode and fresh complete-frame
acknowledgements. There is no raw-mode gate at the hangup point.

All stage, width and colour cases, ten-second bounds, cancellation/EOF/no-file
assertions and child cleanup remain. The controlled writer-drop test and traces
make future fixture failures diagnosable. Application, library and dependency
behaviour are unchanged.

## Evidence and limits

- Ten stop-on-first-failure focused invocations passed: 160 actual disconnect
  sessions with bounded concurrent Cargo checks and no retries.
- All 30 interactive-init tests passed after the correction.
- The full working checkout passed 509 tests, including 52 PTY tests. This count
  includes the independently implemented compact-success tests; it is not a claim
  that the isolated diagnostics commit alone contains all 509 tests.
- The original uninstrumented failure's exact width, colour and metadata are not
  recoverable. The controlled reproduction establishes the mechanism, not a
  recovered trace of that original run.
- Retained pre-hangup snapshots were raw; the fixture does not enforce raw mode
  at disconnect, and those samples are not exhaustive OS-interleaving proof.

Private local evidence is retained in the approved temporary workspace under
`hydragen-cli-diagnostics-phase3/input-loss-investigation`; it is not a build or
test input. No production EOF-safety violation was reproduced.
