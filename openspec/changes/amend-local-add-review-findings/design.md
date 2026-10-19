## Context

PR #7's holistic review examined commit `8fb6893`. Existing specifications already
require declaration-only add, shared schema validation, source-preserving edits,
no-op staging bypass, atomic sibling-file replacement and truthful diagnostics.
Most findings are defects under those contracts, not permission to redesign them.
The [disposition record](review-dispositions.md) maps R1–R15 to bounded work and
evidence; this change does not reopen earlier archived features.

Integration is an accepted prerequisite, not pending implementation here.
Commit `c3b3c8a` preserves the user's exact cancellation wording,
`Layer initialisation cancelled. No files were changed.` Commit `3ea1fcb` ordinarily
merges that commit with inspected main `6ccf96b`. Independent review accepted the
combined baseline: 544 Rust tests (110 library, 129 binary, 305 integration), 39
Python coverage-policy tests, formatting and all-target/all-feature warning-denied
clippy. Incoming coverage protections and the shared parser-first route remain;
version output is the integrated bare number. These are baseline results, not
evidence that the remaining review findings are resolved.

One initial baseline run transiently failed a socket-based broken-pipe fixture.
Controlled probes support temporary reader retention during concurrent pre-exec
spawning, not a persistent post-exec leak; the exact original sibling process is
unknown. Independently closing stdout reproduces the separate silent-EBADF runtime
case. The deterministic fixture implementation, including safe process-group
ownership remediation, has now passed independent review. Do not conflate those
fixture results with production output guarantees. The parent subsequently ran
571 amended-tree Rust tests and 39 Python tests successfully, verified package
contents, and passed a fresh offline build and another 571-test run. Independent
final local review accepted the documentation and validation for delivery; at
that pre-delivery acceptance, only task 6.5's delivery work remained. Linux,
Windows, root, hosted CI and LLVM coverage execution are not claimed. Accepted
evidence and remaining gates are recorded in [verification](verification.md).

## Goals / Non-Goals

**Goals:**

- Preserve normal YAML meaning and comment/layout attachment at the affected
  insertion boundaries, with independent source-sensitive regressions.
- Correct diagnostic spacing and factual causes without changing input validity
  or the public `MetadataError::NotString { field }` shape.
- Make environmental test preconditions explicit while retaining real denial and
  failure evidence.
- Describe the approved completion-reporting boundary and existing replacement
  limitations accurately in focused current documentation.
- Keep amendments reviewable and validate independent implementation lanes before
  final delivery.

**Non-Goals:**

- A formatter, another production decoder, blanket EOF block-string refusal,
  reserialisation, schema/name/reference policy changes or new public error shapes.
- Registry, rendering, contribution capture, chain traversal, locks, stale-content
  detection, automatic merge or in-place replacement.
- File-descriptor checks, terminal reopening, preflight mutation refusal, rollback,
  or newly propagating best-effort warning/error diagnostic write failures.
- A generic Display/CLI deduplication refactor, optional suggestion sweep, UID
  dependency, unrelated CI policy changes or global installation.
- Solving the separate pre-selection `/dev/tty` input-reader race. PTY readiness
  observations do not prove a read syscall is blocked or fix that product race.
- Automatic main-spec synchronisation, archive, PR merge, publication or an API
  stability promise. Main-spec alignment requires the parent's explicit action.

## Decisions

### Guard the actual scanner scalar using bounded source facts

The maintained decoder can normalise EOF block-string behaviour. Comparing its
original and candidate values is necessary but cannot alone prove preservation.
Before staging, inspect the affected scanner scalar and bounded source context for
the actual literal/folded header, strip (`-`), clip (default) or keep (`+`) mode,
header physical line ending, body start, effective content prefix and raw scalar
end. Bound header inspection by prior lexical delimiters/properties and scalar
start, excluding comments/properties; do not search arbitrary earlier `+`/`-`
characters or assume a content-start span includes its header.

Apply the guard to the actual insertion, including missing fields and existing
dependency-list tails, nested mappings/sequences and unrelated final data. A
terminal explicit block-scalar **key** can have an implicit null value: following
only the final AST value with `block_tail` then misses the scalar. Guard the
scanner tail affected by the insertion instead.

`Span.indent` is optional, including for valid explicitly indented all-space
content. Do not unwrap it, treat a content-start column as structural/header
indentation, or infer effective indentation from an unchecked hint. A content-start
column can provide a prefix only after independently checking the actual spaces
and available parser hint. Pause if source facts disagree or header/body safety
cannot be discriminated; do not silently broaden rejection.

Refuse an insertion that newly terminates an unsafe clip/keep body and changes its
trailing-newline meaning. Preserve demonstrated-safe strip, header-only,
already-terminated and safe clip cases, including genuine indentation-only final
clip lines where safe. Cover excess body indentation and empty keep whitespace;
preserve safe existing-list insertions before unfinished indentation-only lines
where applicable. Strip-mode excess spaces remain scalar content.

**Alternatives:** Changing loader semantics or adding another production parser
would enlarge acceptance scope. Broad refusal would reject demonstrated-safe
ordinary YAML. Relying solely on the current decoder repeats the defect. Fixed
source/meaning expectations use the research evidence without introducing Ruby or
another external test/runtime dependency.

The accepted implementation guards unsafe edits; it does not change the loader's
underlying EOF block-string normalisation. Decoder conformance remains outside
this correction, so passing preservation tests must not be described as a loader fix.

### Separate flow collection boundaries from insertion layout

Find a flow collection's real scanner closing delimiter independently of planning
entry layout. The missing-block-field path also uses a final unrelated flow value's
end, so fixing entry placement must not move its root-field boundary.

Keep compact collections compact. In multiline flow collections retain own-line
entry/closing-delimiter layout, inline comments, more-indented attached comments
and footer comments. A needed comma can precede the old inline comment while new
items precede the footer/close: two bounded insertions are legitimate. Include
later-line commas, empty lists, root mappings, batches and CRLF. Preserve existing
bytes rather than normalising whitespace or rewriting a collection.

**Alternative:** One insertion at the close crowds new entries onto the closing
line or changes comment attachment. Reusing entry-placement logic to discover a
value's end couples distinct callers and can misplace a missing root field.

Independent review caught an invalid implicit-null flow-mapping fixture during
implementation: inserting a bare comma after `custom:` emitted `custom:,`, which
the maintained decoder accepted but independent YAML decoding rejected. The
accepted correction inserts ` ,` after the value indicator, yielding `custom: ,`
without rewriting original bytes. Eight actual-CLI variants (YAML/YML, LF/CRLF,
with/without inline comment) now retain exact expected text and the null value in
independent Psych checks. This is a source-separation correction, not a broader
loader-policy change or reason to weaken the independent insertion oracle.

### Test permitted source changes independently

Specify exact expected output and permitted insertion offsets/text from fixtures,
not from the candidate or production scanner. Removing only those inserted spans
must reconstruct the original bytes. Negative controls must reject comment,
literal and unrelated-field corruption even where subsequence or same-decoder
checks pass. Retain one-insertion assertions where true, not globally: TOML,
JSONC and corrected YAML can need two sites.

Retain established reparse, maintained-schema and intended decoded-append proofs.
Refusal/no-op paths preserve bytes and bypass staging. Keep fixtures outside
OpenSpec; source/tests/build scripts must not read these planning artifacts.

**Alternative:** A global contiguous-insertion rule would reject legitimate
supported edits, while decoded equality alone cannot prove source preservation.

### Correct diagnostic facts without changing audiences or API shapes

Emit literally empty block separators; remove the five test normalisations that
masked indentation-only gaps. Retain nonempty supporting/group indentation and
escaped data, without arbitrary trimming.

Use neutral regular-file wording for `NotRegular`: the selected entry may be a
directory, not a symlink. Preserve paths, no fallback, current-file symlink refusal
and readable dependency-symlink support. For `NotString { field }`, require a
string without asserting null was supplied; optional omission advice remains
conditional and TOML must not imply null support. Schema-backed acceptance and the
public variant remain unchanged.

Library Display and CLI typed-fact composition serve deliberately different
audiences. CLI context includes invocation/base, source role and batch cardinality
that general Display lacks. Keep the adaptation boundary; fix demonstrated factual
drift such as R6 individually. Do not parse English, force headline equality or
introduce a generic abstraction for R14.

**Alternative:** Adding actual-type fields or deduplicating wording would change
APIs/architecture unnecessarily rather than correct the proven factual problems.

### Bound completion reporting without changing diagnostic delivery

Completion write/flush errors returned by the established fallible reporting path
remain fatal and retain the actual cause and created/updated/unchanged state.
Warning/error diagnostic delivery remains existing best effort. In particular,
`report_warning` intentionally ignores stderr write failure so warning delivery
does not change completed-write status. This clarification adds no propagation or
status change for those writes and promises no visible diagnostic on unavailable
stderr.

The reviewed Unix runtime can accept an unavailable standard stream without an
error; analogous Windows behaviour is documented but not runtime-tested here.
Successful output calls do not prove receipt. No guaranteed delivery, rollback,
descriptor inspection, terminal reopening, preflight refusal or raw-output bypass
is introduced. Stronger detection requires a separate owner-approved policy.
Bound closed-stdout/broken-pipe subprocess tests where supported, use deterministic
setup, and simulate portable BrokenPipe with `ErrorKind`, not universal OS code 32.

**Alternative:** A blanket rule making every output-API error fatal would change
the intentional best-effort warning/error route and exceeds approval. Platform
syscalls/prechecks would introduce a new mutation/output policy, not merely clarify
the existing one.

### Make fixture preconditions explicit and skips narrow

Pin the copied-help-example test's width while retaining intentional width tests;
rerun with inherited `COLUMNS=30`. For three Unix permission-denial fixtures, probe
actual read/create denial before state snapshots. All five consumers distinguish
ready, unsupported-with-reason and setup-error; skip only unsupported denial
scenarios. Unexpected errors or success after a verified denial remain failures.
Restore permissions/remove probe artifacts on every path. Retain unaffected
permission-preservation and Windows tests, visible `--show-output` skip evidence
and real unprivileged denial coverage. Do not add UID detection/dependencies or
claim root execution without running it.

**Alternative:** Blanket skips or a UID heuristic would hide defects without
establishing whether the environment actually enforces the fixture's permissions.

### Retain output-fixture ownership until all group signals finish

The accepted test helper uses a private FIFO with explicit reader ownership and
reaping to distinguish a genuinely broken pipe, a live-reader negative control and
a separately closed descriptor. Its direct shell child remains an unreaped
process-group anchor through the final group signal. The irreversible lifecycle
is `Anchored → SignalsFinished → Reaped`; the sole `try_wait` is guarded so it
cannot reap the anchor while group signalling is still possible. Cleanup and Drop
must never send cached group signals after release. Shell bookkeeping distinguishes
a reader that can still be killed from a PID retained only to wait for it.

The shell publishes the measured CLI's completion status only after collecting
its children/readers, then waits for release. The intentionally killed anchor's
status is not mistaken for the CLI's result. One deadline bounds setup, execution,
capture and cleanup; forced supervisor death still depends on operating-system
orphan reaping and fails explicitly within that bound if cleanup cannot finish.

**Alternative:** Reaping the group leader before sending later signals leaves
numeric process/group IDs available for reuse; a `kill(0)`-style existence check
does not establish ownership. Independent review blocked that earlier helper and
accepted the anchored lifecycle after failing-first state-order coverage, eight
helper tests, captured tests and safe stress runs. This changes test support, not
production descriptor handling, EPIPE policy or mutation semantics.

### Keep documentation focused on existing behaviour and this decision

Correct the README name-only init/add walkthrough to the actual generated flow
list, preserving valid block-list examples elsewhere. Summarise existing colour
gates concisely: nonempty `NO_COLOR`, `CLICOLOR=0`, dumb terminals, CI/redirection
and explicit-disable-over-force precedence. Qualify completion-reporting sentences
and explain the runtime limitation once; retain existing best-effort diagnostics.

Document that atomic replacement changes the selected path, not the original
file's other hard links or necessarily its already-open handles. Some platforms
restrict replacement while handles are open; do not promise universal success,
enumerate hard links or add an in-place fallback. Put detailed qualifications in
the specification and replacement module docs, not a README tutorial.

Add concise Unreleased compatibility notes for deliberate stricter name rejection,
`LoadedLayerManifest.source`, `InitCreationError.state`, exhaustive matches on
`InitError::InvalidName`/`MetadataError::InvalidName`, and integrated bare-number
version output. These are existing incompatibilities, not a release date or public
API stability promise.

Refine only design question 2's preservation/layout subset and question 10's
diagnostic/reporting subset with links to this decision. The foundations' DX,
Efficiency, Performance and visibility values already support the work; no new
product belief or glossary term is needed. Review current docs as a first-time
reader. Do not rewrite older archives or unrelated active proposals.

## Risks / Trade-offs

- **Source/parser hints are incomplete or disagree** → Pause and surface the
  acceptance tradeoff; do not blanket-refuse valid source or rewrite headers.
- **Decoded equality hides source meaning/layout changes** → Independent exact
  output, fixed source-sensitive expectations and corruption controls supplement,
  rather than replace, production verification.
- **Output limitation is mistaken for ignored completion errors** → Name the
  established fallible completion boundary and retain fatal/stateful handling;
  explicitly preserve best-effort diagnostic delivery.
- **Environment fixtures quietly pass without exercising denial** → Capability
  outcomes and visible scoped skips, plus unprivileged denial evidence.
- **Parallel PTY suites create misleading failures** → Schedule PTY-heavy review
  sequentially with bounded waits/child cleanup; separate baseline, integration
  and amendment failures.
- **Current docs become a historical change dump** → Local README corrections,
  concrete Unreleased compatibility notes and focused reference/design detail.

## Migration Plan

No data migration or new flags are required. Implement failing-first regressions
and bounded corrections against the accepted integrated baseline, review each
lane independently, then align current docs. Run final locked Rust tests, fmt,
all-target/all-feature warning-denied clippy, Python coverage-helper tests, strict
current-spec/new-change checks, local links/whitespace, source-package verification
and a fresh offline build/test copy without OpenSpec or old build outputs.
Preserve coverage gates and report only actually executed environments/results.

The parent owns approved focused commits, ordinary push and concise PR-description
refresh after independent final review. This change does not merge PR #7, archive
itself or synchronise main specs automatically. Rollback of an amendment would
restore the identified defect, not require rewriting users' files or weakening
prior safety requirements.

After the YAML source gate and diagnostic source review, the parent explicitly
invoked main-spec alignment for this change. The three corresponding maintained
specifications now contain the six added requirements and three bounded reporting
modifications. All unmentioned requirements/scenarios remain unchanged; the change
remains active. Alignment was not used as a substitute for runtime validation:
the harness source gate, combined 571-test Rust run and independently accepted
terminal-family review have since passed. Package-content inspection and the
fresh offline build/571-test copy also passed; 57 source/asset hashes match the
checkout exactly. Independent final local review accepted tasks 5.5 and 6.4 for
delivery without source changes. Delivery is recorded separately in task 6.5 and
the PR head, not inferred from this acceptance. No automatic archival is authorised.

## Open Questions

No product question blocks this bounded increment. Ambiguous source facts or a
new comment-attachment policy require escalation rather than an invented rule.
Stronger unavailable-stream detection, the separate input-reader race, published
addition/upgrades, a formatter and broader validate traversal remain separate.
