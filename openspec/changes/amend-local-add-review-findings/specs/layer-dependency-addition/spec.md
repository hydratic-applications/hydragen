## ADDED Requirements

### Requirement: Preserve YAML block-string meaning at insertion boundaries

Dependency editing MUST account for source-level trailing-newline behaviour of
literal and folded YAML/YML block strings when inserting separators or new text,
including missing-field edits and existing dependency-list tails. Equal values
from a decoder that normalises an EOF case MUST NOT alone establish safety. The
editor MUST specifically refuse an insertion that would change that string's
normal YAML meaning before staging, while retaining safe ordinary block strings,
the established decoder, shared schema and intended-change verification.

This protection MUST include terminal block-scalar keys with implicit null values,
not only terminal scalar values reached by following the final AST value. Optional
parser indentation hints MUST NOT substitute for checked header/body source facts.

#### Scenario: A separator newly terminates an unsafe EOF body

- **WHEN** an insertion would newly terminate a clip or keep block-string body and change its retained trailing newlines
- **THEN** preparation fails with a specific explanation before staging
- **AND** original bytes remain unchanged, whether dependencies are absent or already present

#### Scenario: The terminal scalar is a key with an implicit null value

- **WHEN** the terminal scanner scalar is a block-string mapping key with an implicit null value and an insertion would change its trailing-newline meaning
- **THEN** the source-sensitive guard refuses the unsafe insertion before staging even though the AST final value is null
- **AND** it does not rely only on following the last AST value

#### Scenario: Explicitly indented all-space content has no indentation hint

- **WHEN** a valid explicitly indented block scalar has all-space content and its optional parser indentation hint is absent
- **THEN** preparation checks actual header/body source facts and any content-column prefix rather than unwrapping or inventing an indentation hint
- **AND** a safe case remains editable while a meaning-changing insertion is refused before staging

#### Scenario: The block-string boundary is safe

- **WHEN** a supported strip, header-only, already-terminated or safe clip source arrangement permits insertion without changing string meaning
- **THEN** that arrangement is not refused merely for containing an EOF block string
- **AND** header comments, indentation and meaningful body whitespace remain intact
- **AND** the same protection and safe acceptance apply to both YAML extensions

### Requirement: Retain multiline YAML flow layout during dependency insertion

YAML/YML edits MUST retain existing multiline flow-collection entry, closing-line
and footer-comment layout, and keep compact flow collections compact. Necessary
comma and item text MUST use separate insertion boundaries when required to retain
existing bytes and comment attachment. Closing-boundary discovery for an unrelated
final flow value MUST remain distinct from entry-layout planning when placing a
missing root dependencies field. The editor MUST retain batch order and existing
line-ending style, including CRLF, without becoming a general formatter.

#### Scenario: Old entry has an inline comment before an own-line close

- **WHEN** extending a multiline flow dependencies list requires a comma before an existing inline comment and new items before a footer or closing delimiter
- **THEN** separate insertions retain the old entry/comment relationship and the own-line closing delimiter
- **AND** batch order and the existing line-ending style remain unchanged

#### Scenario: Existing commas and attached comments remain in place

- **WHEN** a multiline flow collection has a comma on a later line or more-indented comments attached to its last entry
- **THEN** additions preserve that existing text and its attachment without moving the closing delimiter onto an entry line
- **AND** the prepared result passes the established parsing, schema and intended-change checks

#### Scenario: Empty list or root mapping uses a multiline close

- **WHEN** dependencies are an empty multiline flow list or a flow root mapping requires a new dependencies member before an own-line closing delimiter
- **THEN** insertion preserves the multiline closing/footer layout
- **AND** a compact counterpart remains compact rather than being reformatted

#### Scenario: Missing field follows an unrelated final flow value

- **WHEN** dependencies are absent and the final existing root value is a multiline flow collection
- **THEN** the new root field follows that complete value and its attached comments
- **AND** the unrelated collection is neither extended nor rearranged

### Requirement: Verify source preservation independently of decoded equality

Preservation regression tests MUST include independently specified exact output
and permitted insertion boundaries/text, with removal of only those insertions
reconstructing original bytes. They MUST reject comment, scalar and unrelated-field
corruption even if an existing-byte subsequence or decoded-data comparison passes.
The test contract MUST allow multiple bounded insertions where format syntax or
comment attachment requires them, without weakening production decoded-change or
pre-staging verification. One-insertion assertions MUST remain where that is the
actual edit contract, not be imposed globally across formats.

#### Scenario: Corruption retains a weak subsequence proof

- **WHEN** a test candidate corrupts a comment, literal or unrelated field while retaining original bytes as a subsequence
- **THEN** the independent preservation oracle rejects it
- **AND** the expected sites/text are not derived from the corrupted candidate or production scanner

#### Scenario: A valid edit needs two insertion sites

- **WHEN** preserving an inline comment requires separate comma and item insertions in supported YAML, JSONC or TOML
- **THEN** the test verifies those specific permitted sites and reconstructs original bytes
- **AND** it does not require one global contiguous insertion

#### Scenario: Refusal or harmless repeat does not stage a candidate

- **WHEN** preparation refuses an unsafe boundary or all requested dependencies are harmless repeats
- **THEN** regression coverage verifies unchanged source bytes and no staging file
- **AND** intended-change verification remains required for actual edits

### Requirement: Document file identity effects of atomic replacement

Current safety documentation MUST distinguish replacement of the selected path
from in-place mutation of its original file. It MUST explain that other hard links
continue to identify the original file, and already-open handles can continue to
refer to that file when the platform permits replacement while they remain open.
This clarification MUST NOT introduce hard-link enumeration, a new refusal policy
or an in-place fallback, or promise replacement succeeds with every open handle.

#### Scenario: The original file has another name or an open reader

- **WHEN** atomic replacement succeeds for the selected layer manifest path
- **THEN** documentation does not imply that other hard links or already-open readers necessarily observe the new file
- **AND** permission preservation and existing replacement/cleanup guarantees remain unchanged

## MODIFIED Requirements

### Requirement: Present actionable grouped errors without misreporting writes

Usage, discovery, validation, edit-preparation, and write failures MUST exit
nonzero with accurate stderr diagnostics. A single failure MUST lead directly
with its concrete problem, without a generic count or source-group wrapper.
Multiple independent verification errors MUST use a concise failure-count
headline and source-labelled groups in established source order, explaining
causes and correction without redundant numbering. Diagnostic paths MUST remain available
where needed; source-resolution errors MUST make the declaring-directory base
clear when relevant.

Duplicate layer manifest warnings MUST use shared discovery results and appear
at most once per affected directory per invocation, before a dependent failure,
even when several references resolve to that directory. Warning alone MUST NOT
cause failure.

Feedback MUST distinguish failures before replacement from completion write/flush
errors returned by the established fallible reporting path after replacement.
It MUST NOT claim success before replacement or claim no files changed after a
completed update or a known cleanup failure. If that completion path returns an
error after replacement, add MUST exit nonzero and attempt a diagnostic explaining
that the layer manifest was already updated, retaining the delivery cause.
Warning/error diagnostic delivery MUST retain existing best-effort handling,
without new propagation or status changes. Successful output calls do not guarantee
receipt; this limitation MUST NOT suppress errors returned by the established
fallible completion path or introduce descriptor checks or rollback. Diagnostics
MUST NOT recommend unimplemented upgrade or registry commands as though they exist.

#### Scenario: Verification failure has no update

- **WHEN** source checks fail before staging
- **THEN** stderr reports a single problem directly or multiple failures in ordered source groups with guidance and accurately reports no changes
- **AND** stdout contains no success result

#### Scenario: Repeated target discovery has duplicates

- **WHEN** several references identify one directory with multiple layer manifests
- **THEN** stderr warns once for that directory, naming candidates and selection in priority order
- **AND** the warning precedes any failure depending on its selected layer manifest

#### Scenario: Completion output fails after replacement

- **WHEN** replacement completes but the established fallible completion write/flush path returns an error
- **THEN** the command exits nonzero and attempts a diagnostic explaining that the layer manifest was already updated and retaining the delivery cause
- **AND** it does not report cancellation or no files changed, even if stderr delivery is unavailable
