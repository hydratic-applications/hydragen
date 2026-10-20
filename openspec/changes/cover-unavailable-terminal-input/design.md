## Context

The current input-loss test gives stdin its own pseudoterminal (PTY), a software
terminal pair. The test drives the master endpoint and the child uses the slave
endpoint. Another terminal stays open for output and is also the child's
controlling terminal, which is the terminal reached through `/dev/tty`. On Linux,
console 0.16.6 can use that terminal if stdin disconnects before a key read selects
its input.

The existing regression waits for stdin to become noncanonical, accepting input
without line buffering, before disconnecting it. This is evidence that stdin has
been selected, not that a read syscall is blocked. That test remains useful and
unchanged.

For ordinary input-error coverage, we can instead give the child no controlling
terminal. Standard output and error can still use PTYs. When stdin disappears,
there is no fallback terminal, and Hydragen receives a real input error.

Scratch direct-Rust launcher experiments exercised Name and format on Linux and
macOS with unchanged Hydragen binaries. They support this design, but are not
validation of the maintained test changes. The foundations' DX and efficiency
values favour a focused test addition over maintaining a dependency patch for
this testing need.

## Goals / Non-Goals

**Goals:**

- Check input-error reporting and the no-write guarantee at Name and format.
- Keep output readable and verify the test really has no controlling terminal.
- Reuse the existing session helper with safe, bounded cleanup.
- Keep the existing selected-input test, timing semantics and assertions intact.

**Non-Goals:**

- Change production input routing, cancellation, styling or validation.
- Prevent console using a surviving controlling terminal in other arrangements.
- Claim that a visible prompt proves input selection or that these new cases
  disconnect at a guaranteed point before selection.
- Patch or vendor dependencies, publish an upstream issue, rewrite terminal
  handling, change coverage policy, or add production test hooks.

## Decisions

### Use a direct test-only launcher with no controlling terminal

Open stdin, stdout and stderr PTY slave handles in the parent using `O_NOCTTY`,
which prevents those opens acquiring a controlling terminal. Launch the actual
Hydragen executable through `std::process::Command` with the pre-opened handles.
After fork and before exec, call `setsid()` to start a session without a
controlling terminal, then verify that opening `/dev/tty` fails with `ENXIO`, the
expected no-controlling-terminal error. Keep that callback allocation-free and
limited to async-signal-safe operations so it does not use resources left locked
by other parent threads. Capture errno before another operation can overwrite it;
format contextual diagnostics only in the parent. Unexpected verification results
must fail the test launch.

Add `libc` as a Unix-only development dependency for these OS calls. Reuse
portable-pty for terminal allocation and its existing `Child` implementation for
standard Rust children. This adds no production dependency or private API use.

Do not launch through a shell: a session leader can acquire a controlling terminal
when it opens terminal paths for redirection. For that reason, merely calling
portable-pty's `set_controlling_tty(false)` is not enough for the current
shell-based launcher. An external `setsid` tool is also unnecessary and would add
a platform assumption.

### Add frame-visible failure coverage, not a timing claim

Use the new launcher for two cases: disconnect stdin after Name is visible, and
disconnect after the complete format selector is visible following valid metadata
answers. Close every input-master handle, including the plain duplicated writer.
Do not use portable-pty's writer destructor, which injects newline/EOT bytes.

Assert a nonzero exit, one styled input error, one no-files-changed message, empty
stdout, no cancellation or success wording, and an empty project directory.
Do not require a particular OS error: closure after the visible frame can occur
before or after console selects stdin, and Linux and macOS report loss differently.

Also complete a normal guided run through the new launcher and check the created
layer manifest. This control shows the launcher did not accidentally disable
guided input or make normal input unusable.

A controlled gate before console's selection could use additional test-helper
and theme plumbing, but that is not needed for this error-handling increment.
Syscall interception and production environment switches would add unnecessary
complexity. Preserve the original selected-input test rather than replacing its
readiness checks with frame visibility.

### Keep descriptor and child ownership explicit

Prepare fallible reader handles before spawning. Immediately after spawn, guard
the child so failures during setup still kill and reap it. Start reader threads
only after a successful spawn and after installing that guard, so a spawn failure
cannot leave blocking readers behind. Drop the parent `Command` and all
parent-owned output slave handles promptly; otherwise reader EOF can be delayed
after the child exits. Transfer the guard into the session without an unguarded
ownership gap.

Track child termination separately from completed output draining. Normal
completion still requires a reaped child and EOF from every output reader. A
child-status observation error is not evidence of termination. Stopping readers
during failed cleanup must not count as successful EOF or turn a timeout into a
passing result.

### Give failed sessions a separate bounded shutdown path

Keep the existing 10-second normal-operation deadlines. Once failure cleanup
starts, use one separate 10-second cleanup deadline shared by child termination
and reader shutdown. This replaces potentially unbounded cleanup; it does not
extend the deadline for passing the test.

For the new launcher's output readers, use nonblocking output-master reads with
readiness waiting and an explicit shutdown notification. A standard-library Unix
socket pair can provide that notification: closing the supervisor's end wakes
each worker waiting on its own output and the shutdown descriptor. Workers check
shutdown before reading, treat `WouldBlock` as pending input and release their
owned descriptors when stopping. All supervisor-end copies must close for that
notification to work. Distinguish interruption, pending input, read failure,
terminal EOF/hangup and requested shutdown; only genuine output completion can
satisfy the normal completion condition. Apply nonblocking mode only to the new
launcher's separate output terminals, not its stdin writer or the existing
regression's shared input/output terminal.

On a failure, release parent-side slave handles, terminate a child not known to
have been reaped, and observe termination with bounded status checks rather than
an unconditional blocking `wait()`. Use a forceful termination fallback if the
child ignores SIGHUP; do not substitute portable-pty's SIGHUP-only cloned killer.
Collect already available output and request reader shutdown promptly while child
termination is still being observed. Do not spend the whole shared deadline
waiting for the child before starting worker shutdown. A retained slave handle
must not make the worker wait indefinitely for EOF. Confirm workers have finished
before joining them; dropping a live `JoinHandle` merely detaches the worker and
is not successful resource cleanup.

If child termination or worker shutdown cannot be confirmed within the cleanup
deadline, fail with an explicit incomplete-cleanup diagnostic including the child
PID, last observed status and reader state. Do not claim the child was reaped or
the workers stopped, and do not perform an unbounded wait or join. Keep setup-time
guards on this same bounded cleanup path.

Allow only the small shared-helper lifetime changes and new-reader shutdown
support needed for this strategy. The existing test-ci requirement currently
restricts finish-timeout changes to diagnostics; the delta explicitly permits
these corrections while preserving its readiness checks, normal deadline,
completion condition and assertions. Do not edit archived evidence or migrate
the existing regression to a new reader backend without a separate scope review.
If this needs a broad harness rewrite, stop and reassess before implementing it.

### Keep the runtime input-policy question separate

This decision concerns test setup, not a new promise about original-stdin-only
production input. It neither fixes the live-alternative-terminal case nor
weakens the CLI interaction specification. Preserve the historical distinction
in the [coverage verification record](../archive/2026-10-19-separate-test-and-coverage-checks/actions-gates-verification.md#terminal-input-selection-harness-increment).

The existing interaction choices in design question 9 remain unchanged. Record
the deferred input-source question separately in `openspec/design-questions.md`.
No foundation or glossary update is needed: no product belief or Hydragen term
is being introduced or redefined.

## Risks / Trade-offs

- **Accidental controlling-terminal acquisition:** Open handles before launching,
  avoid shell redirections and verify absence before exec.
- **Leaked descriptors or child processes:** Guard ownership immediately, release
  parent-side slave handles, make new readers stoppable and exercise retained-slave
  and termination-failure paths under the separate cleanup deadline.
- **Overstated regression coverage:** Name the cases unavailable-input tests and
  state that frame visibility is not a selection barrier.
- **Platform-specific error wording:** Assert the shared failure contract rather
  than a particular errno. Run maintained tests on Linux and macOS.
- **Disproportionate complexity:** Keep changes local to the terminal-test helper
  and stop if they grow into another terminal framework or a general rewrite.
  The reviewed implementation uses an isolated support module, with roughly
  half its lines devoted to exceptional-path tests. This is larger than the
  initial launcher estimate because bounded reader shutdown and failed-cleanup
  diagnostics need explicit ownership and observation. Review found these parts
  justified by the amended requirements; the existing reader backend is retained.

## Migration Plan

No production migration is required. Add the test-only dependency and launcher,
then the two failure cases and connected-input control. Run formatting, locked
all-targets Clippy, full Linux/macOS tests and instrumented Linux tests. Record
exact results and obtain independent review before proposing the implementation
for merge. Do not change workflows or thresholds to obtain a pass.

## Open Questions

- Whether to enforce original-stdin-only input when another terminal remains
  available is deferred. This proposal does not settle or implement it.
- Maintained-harness portability and failure-path cleanup have been validated on
  Apple Silicon macOS and Linux aarch64; see the [verification record](verification.md).
  Linux x86-64 execution remains for the unchanged PR checks. Windows terminal
  behaviour is outside this Unix-only change.
