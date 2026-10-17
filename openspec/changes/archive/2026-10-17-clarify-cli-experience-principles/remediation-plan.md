# CLI DX remediation plan

## Steps

- [x] Improve width-aware help layout without losing information.
- [x] Make guided validation wording match prompt-based input.
- [x] Add safe recovery guidance for existing-layer-manifest errors.
- [ ] Verify and improve narrow guided presentation where supported by evidence.
- [ ] Review overall visual cohesion and remaining evidence gaps with the user.

## Scope and ordering

This plan follows the [CLI experience audit](audit.md) of the unchanged CLI at
revision `e2d50e853db6bb5fb2c9abd53a87aef2fc2ba712`. The principles, audit, and
this plan are complete under `clarify-cli-experience-principles`; **the fixes below
are not implemented or authorised by that completion**.

Treat beauty, enjoyment, functionality, and ease of use as equally important.
Priorities express likely user impact and implementation risk, not a hierarchy
among those goals. P1 means start here after approval; P2 means follow in a
focused increment. No emergency or data-loss fix was found. Each change must
include behavioural verification and visual review; neither substitutes for the
other. Do not claim user-tested enjoyment without obtaining user feedback.

R1 and R2 are independent. R3 can follow either. R4 starts with direct-terminal
evidence before selecting an implementation; do not bundle it into a theme
redesign. R5 is a decision/evidence review, not an automatic styling task.

## R1: Improve help layout — P1 adherence work

**Finding:** [F1](audit.md#f1-help-does-not-adapt-to-terminal-width), especially
`help-init`, `help-normal`, and `help-narrow` in the
[session records](evidence/sessions.json).

**Affected goals:** beauty and ease of use directly; clearer scanning should
reduce friction, while functionality and the full input contract must remain.
This is needed to address the recorded layout gap, not to add decoration.

**Direction:** use clap's supported width-aware help facilities, checking whether
the optional wrapping feature must be enabled. Restructure long option prose and
literal example/guidance text so continuation lines retain hierarchy and words
are not arbitrarily split by terminal autowrap. Give longer examples a readable,
copyable layout. Preserve interactive and flag-only examples, directory-name
default, independently optional fields, CI/stream conditions, and the no-hybrid
rule. Plain help is acceptable; colour is not the solution to width problems.

**Likely files:** `src/main.rs`, `Cargo.toml`/`Cargo.lock` only if clap features
need adjustment, `tests/init.rs`, `tests/interactive_init.rs`, and README if help
examples or documentation need synchronising. Keep any new PTY cases within the
existing bounded harness rather than adding a framework.

**Dependencies and review:** verify the current clap feature/API behaviour before
choosing configuration. Use a separate OpenSpec change, tentatively
`improve-cli-help-layout`, to specify observable help requirements and test scope.
No visual-style decision is needed to start.

**Acceptance checks:**

- At representative 40-, 80-, and 100-column widths, option descriptions and
  explanatory prose wrap at sensible boundaries with readable continuation
  indentation. These are verification cases, not a product minimum-width promise.
- Long command examples remain understandable and copyable; a genuinely long
  unbreakable token is not silently truncated to satisfy a line-length assertion.
- Root/init help, missing-command/argument diagnostics, and version still use
  correct output channels/statuses, open no prompts, and create no files.
- All current help content and input-route guarantees remain present. Use content
  assertions plus visual review, not a brittle full escape-sequence snapshot.
- Run formatting, clippy, and locked tests; review recorded ordinary/narrow output
  in a full terminal and compare with the baseline evidence.

**Priority rationale:** help is the entry point for every unfamiliar invocation;
the 117-character output lines create an observed problem even at 100 columns.
The likely correction is local and does not require a new aesthetic direction.

## R2: Present route-aware validation feedback — P1 adherence work

**Finding:** [F2](audit.md#f2-guided-validation-uses-flag-oriented-wording),
`guided-retry` invalid-name and invalid-author frames.

**Affected goals:** ease of use and enjoyment of correcting input; preserve clear
warning presentation and functional validation guarantees.

**Direction:** keep one shared non-blank validation rule, but adapt its message at
the presentation boundary. For example, Name could say “Enter a name containing
at least one non-whitespace character,” while an optional field could explain
“Enter a non-blank author, or clear the field and press Enter to skip.” Exact
wording needs review; these are examples, not new validation semantics. Retain
flag-oriented diagnostics for flag-only invocations. Do not trim accepted input,
collapse whitespace-only input into a skip, or make optional fields required.

**Likely files:** `src/interaction.rs` for prompt feedback; `src/lib.rs` only if
a structured error helper is genuinely needed, without importing terminal code;
the existing unit and terminal tests. README needs changes only if describing
new observable feedback.

**Dependencies and review:** independent of R1. Use a focused later OpenSpec
change, tentatively `clarify-cli-input-feedback`, to record the presentation-only
decision and any feedback requirement additions. Review final wording before
implementation; keep validation semantics in the existing specs unchanged.

**Acceptance checks:**

- Whitespace-only Name/Author/Description answers produce field-specific,
  prompt-appropriate recovery instructions, with no `--flag` reference in guided
  validation feedback.
- Optional feedback explains clearing/skipping rather than implying a mandatory
  value. Empty optional input still skips; a literal `Skipped` remains metadata.
- Retries, default acceptance, Unicode, and surrounding whitespace behave exactly
  as before. No file exists while answers remain incomplete or invalid.
- Flag-only errors still identify the relevant flag, remain plain, and never
  trigger questions. Shared validation has not been duplicated across routes.
- Run the existing tests plus targeted retry/message checks and visually review
  invalid/corrected states with and without colour.

**Priority rationale:** the mismatch occurs at the moment a user needs help, is
directly observed, and can likely be corrected with a small presentation adapter.

## R3: Improve existing-target recovery guidance — P2 adherence work

**Finding:** [F3](audit.md#f3-existing-layer-manifest-feedback-stops-without-explaining-how-to-proceed),
`existing-manifest` and the matching pre-prompt rejection tests.

**Affected goals:** ease of use and the experience of recovering from an error;
retain readable plain diagnostics and filesystem safety.

**Direction:** explain the deliberate no-overwrite behaviour and suggest safe
next steps: inspect the existing entry or initialise in the intended different
directory. Do not assume the entry is a valid layer manifest, read or validate it,
suggest deletion as a default remedy, add overwrite confirmation/force, or invent
an unimplemented command as the next action. The same advice must remain accurate
for files, directories, and dangling symlinks.

**Likely files:** `src/lib.rs` diagnostic formatting, `src/main.rs` only if
presentation needs a separate adapter, `tests/init.rs`, `tests/interactive_init.rs`,
and README's example error. Keep display decisions out of filesystem checks.

**Dependencies and review:** can follow R1 or R2; can share R2's later change if
scope remains focused, otherwise use a separate diagnostic-guidance change. The
OpenSpec record must distinguish improved copy from altered no-overwrite policy.

**Acceptance checks:**

- The diagnostic names the occupied path, states that it was not overwritten, and
  gives a safe next step without implying that the existing layer is valid.
- File/directory/symlink cases reject before prompts; original entries and targets
  stay unchanged. A target appearing during questions remains protected.
- Errors retain nonzero status, stderr-only plain output, and no success claim.
  Long paths do not lose essential information; narrow presentation is reviewed.
- README and tests match the final wording without depending on incidental
  punctuation or hard-coded temporary paths.

**Priority rationale:** the current diagnostic is truthful and safe, so this is
not an urgent safety defect. Clearer recovery completes the new guidance without
requiring extra command behaviour.

## R4: Review narrow guided layout — P2 evidence-led adherence work

**Finding:** [F4](audit.md#f4-narrow-guided-presentation-needs-a-wrapping-pass),
especially `guided-narrow-long`; no data loss or confirmed redraw bug was found.

**Affected goals:** beauty and ease of use of the prompt frame and long values;
retain enjoyment of a clear flow and full functional value preservation.

**Direction:** first reproduce the captured output in a full terminal emulator
at 40, 80, and 100 columns, including long ASCII/CJK/emoji defaults, typed
overrides, invalid answers, backspacing, and resizing mid-prompt. Investigate
whether a small cliclack-supported layout adjustment or guidance-text change
improves continuity. Do not infer a width-counting bug from the audit's approximate
renderer. If there is an upstream issue, document a minimal reproduction and
consider the smallest adapter or upstream fix; do not silently upgrade dependencies.

**Likely files:** initially evidence/report updates only. A justified fix would
likely touch `src/interaction.rs` and `tests/interactive_init.rs`; dependency
changes require an explicit decision and locked verification. Update README only
for genuine behaviour or support-limit changes.

**Dependencies and review:** starts after direct-terminal evidence is available;
may proceed independently of R1–R3. Use a separate OpenSpec change for any fix,
tentatively `improve-narrow-guided-layout`. Agree any minimum-width policy or
larger visual direction with the user rather than treating these test widths as
an existing product promise.

**Acceptance checks:**

- Document the full terminal/environment and distinguish observed wrapping from
  confirmed redraw/editing defects; if replay is not representative, correct the
  finding instead of implementing speculative fixes.
- Guidance and long-value display retain readable hierarchy at the reviewed
  widths; any unavoidable limitation is explicit rather than a silent claim.
- Full names and metadata remain unchanged in the layer manifest; no truncation or
  normalisation is introduced to make presentation easier.
- Active, invalid, submitted, skipped, and cancelled states remain distinguishable
  without colour. Cursor restoration and cleanup are checked after cancellation.
- New regressions are covered within the bounded PTY harness; review screenshots
  or terminal recordings as well as test assertions.

**Priority rationale:** the layout issue is observed but normal flow is usable,
and terminal-specific correctness is still uncertain. Evidence comes before a
potentially risky rendering change.

## R5: Human design review and verification backlog — no automatic redesign

**Finding:** [F5](audit.md#f5-overall-visual-direction-and-feedback-cohesion-need-human-review)
and the audit's [limits](audit.md#limits-and-conclusion).

**Classification:** optional refinement and incomplete evidence, not confirmed
non-adherence. Requiring colour, unified prefixes, a custom theme, or fewer
feedback lines without review would introduce unagreed style decisions.

Review root/init help, guided success, no-colour output, validation, and
cancellation with the user after the focused fixes. Ask whether the difference
between clap/plain output and cliclack feedback feels coherent, whether repeated
completion/cancellation messages help, and whether the overall flow is enjoyable.
Start by showing what actually exists, not choosing a theme in advance.

For each preference, retain the current design or create an evidence-backed
follow-up with before/after examples and acceptance checks for all four goals.
Keep result output on stdout and prompts/diagnostics on stderr even if reducing
perceived repetition. If no meaningful problem is identified, record the review
and leave the restrained default style intact.

Remaining verification includes Windows/Linux, full-terminal light/dark palettes,
screen-reader use, contrast, resize behaviour, and more complex Unicode editing.
Record conditions and observed outcomes; do not claim this has already been done.
Choose the next checks based on the actual intended environments rather than
building an exhaustive cross-platform framework upfront. No performance targets
are inferred from this audit.

**Likely files:** initially the audit and this plan; later implementation files
depend on the approved decision. There is no prerequisite for a runtime edit.
Completion means a recorded decision and evidence scope, or a separately approved
OpenSpec follow-up—not a forced cosmetic change.

## Preserve and verify in every increment

- Shared validation and unchanged accepted metadata; name defaults, independent
  optional skips, and explicit submitted-skip feedback.
- No metadata flags → eligible guided route; any metadata flag → flag-only route.
  CI, non-terminal streams, and `--non-interactive` remain prompt-free.
- Plain flag-only output, stdout result path, and stderr prompts/diagnostics.
- Early target checks, exclusive creation, no overwrite/repair, cancellation
  without writes, and truthful post-write failure reporting.
- Fast everyday operations, no decorative delays, and no new runtime services.
- Focused diffs, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --locked`, strict validation of each later OpenSpec change, and
  review of actual terminal output rather than test success alone.

Stop here for user review. No follow-up change has been created, no runtime fix
has been applied, and no visual direction has been selected by this plan.
