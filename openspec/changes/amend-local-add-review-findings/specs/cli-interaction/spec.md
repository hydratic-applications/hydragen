## ADDED Requirements

### Requirement: Use literally empty compact diagnostic separators

Compact diagnostic block separators MUST contain no spaces or tabs in both plain
and styled routes. Supporting and source-group indentation MUST remain intact on
nonempty lines. Presentation MUST NOT trim or rewrite user data to obtain empty
separators, and regression assertions MUST NOT normalise away whitespace-only gaps.

#### Scenario: A compact diagnostic has multiple blocks

- **WHEN** a parser or operational diagnostic separates context, guidance or operation-state blocks
- **THEN** each separating line is empty rather than indentation-only
- **AND** nonempty content retains the shared hierarchy and escaped display data

### Requirement: Bound completion reporting guarantees to the established fallible path

The CLI MUST retain fatal/stateful handling of completion write/flush errors
returned by the established fallible reporting path, including the actual cause.
Warning/error diagnostic delivery MUST retain its existing best-effort behaviour;
warning delivery failure MUST NOT change completed-write status. This clarification
MUST NOT introduce new propagation or status changes for diagnostic writes.
A successful output call MUST NOT be represented as proof that a recipient
received the report; the runtime can accept an unavailable standard stream
without returning an error.

This boundary MUST NOT introduce descriptor prechecks, another terminal, mutation
preflight refusal, output rollback or suppression of completion errors returned
by the established fallible path. Existing stream assignments, warning meanings
and nonzero error statuses MUST remain unchanged.

#### Scenario: The completion path returns BrokenPipe after an update

- **WHEN** replacement succeeds and the established fallible completion write/flush path returns BrokenPipe
- **THEN** the command fails and attempts a diagnostic retaining the updated state and delivery cause
- **AND** it does not roll back or claim that no files changed

#### Scenario: Completion reporting fails after creation or a no-op

- **WHEN** the established fallible completion write/flush path returns an error after creation or an all-repeat add
- **THEN** the command exits nonzero and attempts a diagnostic retaining the actual cause
- **AND** that diagnostic distinguishes already-created from unchanged state without claiming cancellation

#### Scenario: Warning delivery fails after completed creation

- **WHEN** creation completes and best-effort warning delivery encounters a stderr write error
- **THEN** warning delivery does not change completed-write status or introduce new error propagation
- **AND** any completion-reporting failure retains its separate established fallible handling

#### Scenario: The runtime silently accepts an unavailable stream

- **WHEN** the output API returns success despite the standard stream being unavailable
- **THEN** the CLI makes no guaranteed-delivery or rollback claim
- **AND** it does not add platform-specific descriptor checks or an alternative terminal

## MODIFIED Requirements

### Requirement: Preserve truthful mutation and reporting state

Diagnostic state MUST come from typed facts and control flow, not message text.
The CLI MUST distinguish no write, possible partial newly created files, failed
replacement with possible staging leftovers, completed creation/replacement, and
completed work whose established fallible completion path returned a write/flush
error. A generic no-files-changed footer MUST NOT be applied to all failures.
Primary and secondary cleanup causes and exact staging paths MUST remain available
with inspection-before-removal guidance.

Deliberate cancellation MUST retain one cancellation report and nonzero exit;
unexpected interrupted I/O MUST NOT be silently suppressed as cancellation.
Warnings MUST retain deduplication and precede dependent failures. Parsing,
validation, filesystem lifecycle, source order, stream/status and terminal/plain
controls MUST remain unchanged. Completion write/flush errors returned by the
established fallible reporting path MUST remain fatal/stateful. Warning/error
diagnostic delivery MUST retain existing best-effort handling without new
propagation or status changes; warning delivery failure MUST NOT change
completed-write status. Successful output calls MUST NOT imply guaranteed receipt.
No descriptor precheck, terminal reopening or rollback is introduced.

#### Scenario: Creation can leave a partial new file

- **WHEN** an init write fails after exclusive creation
- **THEN** the diagnostic does not claim that no files changed
- **AND** it identifies the affected file and preserves the actual failure

#### Scenario: Replacement and cleanup both fail

- **WHEN** replacement fails and staging cleanup also fails
- **THEN** the selected layer manifest is reported as not replaced
- **AND** both causes and the exact staging path are preserved with safe inspection advice
- **AND** the message does not imply no filesystem changes or unconditional deletion safety

#### Scenario: Feedback fails after a completed write

- **WHEN** creation or replacement completes but the established fallible completion write/flush path returns an error
- **THEN** the command exits nonzero and attempts a diagnostic stating that the layer manifest was already created or updated
- **AND** it retains the delivery cause without claiming cancellation or no changes, even if stderr delivery is unavailable
