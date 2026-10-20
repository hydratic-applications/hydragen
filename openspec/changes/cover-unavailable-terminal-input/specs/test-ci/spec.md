## ADDED Requirements

### Requirement: Unavailable-input coverage has no fallback terminal

Unix terminal tests SHALL include a separate guided-init launcher with stdin,
stdout and stderr attached to terminals but no controlling terminal. The launcher
SHALL verify the absence of a controlling terminal before executing Hydragen and
SHALL fail with a useful diagnostic if that setup cannot be established.
The launcher SHALL be test-only and SHALL reuse the existing session
infrastructure rather than replace production terminal handling.

The tests SHALL disconnect every dedicated stdin master handle after observing
the Name prompt or the complete format-selector frame. The format case SHALL
first complete the metadata questions through actual guided input. Output
terminals SHALL remain available so the failure can be observed.

Both cases SHALL require a nonzero exit, exactly one correctly styled input-error
diagnostic and one no-files-changed report, empty stdout, no cancellation or
success wording, and no layer manifest or other project file. Assertions SHALL
NOT depend on a particular platform's errno or EOF wording.

The tests SHALL retain the existing normal-operation timeout, output draining
and child-status diagnostics. Parent-owned slave descriptors SHALL NOT prevent
reader completion. Setup failures, timeouts and unexpected child exits SHALL
fail rather than skip or retry the case. Arbitrary readiness sleeps, injected
readiness keys and extended normal-operation deadlines SHALL NOT mask failures.

A connected-input control SHALL complete guided init through the same launcher
and verify the expected layer manifest, one guided success report and empty
stdout, with no other project files created.

The existing selected-input regression and its readiness checks and assertions
SHALL remain intact. The new cases SHALL NOT treat a visible frame as evidence of
input selection, claim guaranteed before-selection injection, relax production
interaction policy, or claim to fix switching to a surviving controlling terminal.

#### Scenario: Input is lost at Name with no fallback terminal

- **WHEN** guided init runs through the verified no-controlling-terminal launcher and the test disconnects stdin after the Name prompt is visible
- **THEN** it observes the shared input-failure, styling, empty-stdout and no-files assertions within the existing bound
- **AND** it makes no claim about whether stdin had already been selected

#### Scenario: Input is lost at the format selector with no fallback terminal

- **WHEN** guided init completes its metadata questions through that launcher and the test disconnects stdin after the full format-selector frame is visible
- **THEN** it observes the same input-failure and no-files assertions without supplying a format answer
- **AND** the output terminals remain available for diagnostics

#### Scenario: Connected input completes through the same launcher

- **WHEN** the test leaves stdin connected and supplies valid guided answers, including a format selection
- **THEN** init succeeds and creates only the expected layer manifest
- **AND** it reports guided success once on stderr with empty stdout

#### Scenario: Test setup or cleanup cannot establish the required conditions

- **WHEN** the launcher cannot verify the absence of a controlling terminal, input/output setup fails, or child completion exceeds its bound
- **THEN** the test fails with useful setup or captured-output diagnostics
- **AND** it releases owned descriptors and performs bounded child cleanup without signalling a child known to have been reaped

### Requirement: No-controlling-terminal sessions have bounded exceptional cleanup

The new launcher SHALL track observed child termination separately from output
draining. Successful normal completion SHALL require both a reaped child and EOF
from every output reader within the unchanged normal-operation bound. Reader
shutdown after a failure SHALL NOT count as normal EOF or make a failed case pass.

Failure cleanup SHALL have one separate 10-second deadline, starting when cleanup
begins and shared by termination, reader shutdown and completion observation.
This deadline SHALL NOT extend the time available for normal test success.

The launcher SHALL release parent-side slave handles, perform bounded child-status
checks and retain a forceful termination fallback. It SHALL NOT signal a child
known to have been reaped, treat an observation error as termination, or follow a
failed termination attempt with an unbounded `wait()`.

New output readers SHALL support explicit shutdown even while a slave remains
open. Reader shutdown SHALL be requested promptly, without spending the entire
shared cleanup deadline on child termination first. The launcher SHALL confirm
worker completion before joining. Detaching a live worker SHALL NOT be treated
as successful cleanup. If termination or worker
completion cannot be confirmed within the cleanup deadline, the test SHALL fail
with an incomplete-cleanup diagnostic containing the child PID, last observed
status and reader state, without an unbounded wait or join or a false claim that
resources were cleaned up.

#### Scenario: Reaped child still has an open output slave

- **WHEN** the child has been reaped but output remains pending or a deliberately retained slave prevents EOF
- **THEN** failure cleanup stops the output workers and releases their owned descriptors within its separate bound
- **AND** it does not signal the reaped child or count worker cancellation as successful normal completion

#### Scenario: Child ignores the initial termination signal

- **WHEN** a failed session's child ignores SIGHUP
- **THEN** cleanup uses a forceful termination fallback and bounded completion observation
- **AND** output collection and worker shutdown share the same cleanup deadline

#### Scenario: Cleanup cannot confirm termination or worker completion

- **WHEN** a termination or observation failure prevents confirmation of cleanup within its deadline
- **THEN** the test fails with the required incomplete-cleanup diagnostic
- **AND** it does not silently abandon cleanup, wait indefinitely or report success

## MODIFIED Requirements

### Requirement: Terminal input-loss tests establish selected-input readiness

The dedicated-stdin PTY input-loss test SHALL verify a canonical baseline before
child spawn and, after the relevant Name or format-selector frame, observe that
same PTY becoming noncanonical before closing all input master handles. It SHALL
use supported termios observation (`MasterPty::get_termios` and named `ICANON`
flags), without platform-specific bit constants or new Cargo dependencies.
The transition SHALL be treated as evidence of stdin selection, not proof that
a read syscall is blocked or all terminal settings are raw. A visible frame alone
SHALL NOT be treated as selected-input readiness.

Readiness waiting SHALL retain the existing timeout, output-event draining and
child-status checks. Unsupported termios observation, early child exit and timeout
SHALL fail with useful diagnostics rather than skip or extend the test. Helper
tests SHALL cover pending/ready, expired, unsupported and exited-child states.
The finish completion condition and normal-operation timeout SHALL remain
unchanged, and timeout failures SHALL retain child-status and reader-count
diagnostics. Narrow shared-helper descriptor-lifetime and child-cleanup
corrections needed by the no-controlling-terminal launcher SHALL be permitted,
including avoiding signals to known-reaped children and supporting bounded
shutdown of the new launcher's readers. Such corrections SHALL preserve this
regression's input-selection readiness checks, normal reader behaviour and all
failure assertions.

Both injection cases SHALL retain nonzero exit, one correctly styled input-error
diagnostic, empty standard output, no cancellation or success wording, and no
created files or layer manifest. No retries, arbitrary readiness sleeps or input
keystrokes, skipped tests or weakened assertions SHALL mask failures. This
requirement governs test setup only: it SHALL NOT relax production interaction
policy or imply that input loss before stdin selection has been fixed.

#### Scenario: A prompt is visible before input selection

- **WHEN** the target Name or selector frame is visible but the dedicated PTY is still canonical
- **THEN** the test waits for a noncanonical transition within the existing bound
- **AND** it does not disconnect input based on output alone

#### Scenario: Selected input is disconnected

- **WHEN** the dedicated input PTY has transitioned from its verified canonical baseline
- **THEN** the test closes every input master and checks the existing failure/style/no-files assertions
- **AND** it makes no claim that a read syscall was already blocked

#### Scenario: Input readiness cannot be established

- **WHEN** termios observation is unsupported, the child exits early or the deadline expires
- **THEN** the test fails with the relevant state and captured-output diagnostics
- **AND** it does not skip the case, inject readiness input or increase the timeout

#### Scenario: Shared lifetime corrections preserve selected-input coverage

- **WHEN** shared helpers gain the permitted descriptor-lifetime or cleanup corrections
- **THEN** the selected-input test retains its readiness checks, normal completion condition, operational deadline, reader behaviour and failure assertions
- **AND** it remains distinct from frame-visible disconnection with no controlling terminal
