## Context

The completed `clarify-cli-experience-principles` change established beauty,
enjoyment, functionality, and ease of use as equal DX goals, then audited the
prototype. Its [remediation plan](../2026-10-17-clarify-cli-experience-principles/remediation-plan.md)
identifies help layout (R1), guided validation wording (R2), and occupied-layer-manifest
recovery guidance (R3) as concrete, local improvements. This change combines
those three related presentation fixes into one reviewable increment. It does
not implement every later audit recommendation.

Current boundaries are suitable: `src/main.rs` owns clap declarations;
`src/interaction.rs` owns terminal input/feedback; `src/lib.rs` validates metadata
and creates files without a terminal dependency. The shared validator returns
`InitError::InvalidMetadata`, whose display text is flag-oriented. Guided
callbacks currently display that error unchanged. An occupied layer manifest path has
a distinct error variant; improving its display does not require new I/O.

Clap 4.6.7 is locked, with only its `derive` feature explicitly enabled. Help
disables colour and uses long literal `after_help` lines. The existing terminal
test harness uses separate result/diagnostic terminals, a 120-column size, CI
controls, ten-second bounds, serialization, and child cleanup. Captured tests
assume some help phrases are contiguous and one terminal retry test waits for
`--author must contain`; those expectations need deliberate updates.

The foundations already provide the needed product goals and the glossary needs
no new terms. Only a small link under design question 10's CLI success-criteria
resolution is needed during implementation. Questions about agent-specific
interfaces, performance thresholds, minimum terminal width, and future commands
remain open.

## Goals / Non-Goals

**Goals:**

- Readable, width-aware root/init help with complete, copyable examples.
- Guided validation that names the field and explains recovery in prompt terms.
- Safe, accurate advice when any existing entry occupies the layer manifest path.
- Regression tests and before/after evidence for all three fixes, while preserving
  existing input, output, cancellation, and filesystem contracts.

**Non-Goals:**

- A new theme, colour policy, branding, terminal renderer, spinner, or decoration.
- Long-input editing/resize fixes, a full cliclack wrapping redesign, or declaring
  a minimum terminal width; these remain the evidence-gated R4 follow-up.
- Removing the two output channels or redesigning success/cancellation feedback;
  broader visual cohesion and user-perceived enjoyment remain R5 review topics.
- New flags/commands, hybrid input, layer manifest formats/schema work, altered validation
  semantics, new file operations, or dependency version upgrades as a side effect.
- Rewriting the prior audit, its raw evidence, or archived OpenSpec decisions.

## Decisions

### Use clap wrapping and a simpler option hierarchy

Enable the existing clap dependency's stable `wrap_help` feature. Let clap detect
terminal width, cap help width at 100 columns with `max_term_width(100)`, and use
its documented 100-column fallback when width cannot be determined. Do not set a
fixed `term_width` in production: that would disable adaptation. Use explicit
`term_width` only in builder-level tests for deterministic layout cases. Preserve
`ColorChoice::Never` so help/errors remain plain under forced-colour environments.

Use `next_line_help(true)` on init so descriptions begin below option names
rather than near the right edge at 40 columns. Root command help can retain its
compact command list, with clap wrapping its prose. This costs some vertical
space but keeps one consistent init hierarchy instead of inventing a dynamic
layout engine.

Rewrite literal examples into short command lines and deliberate shell
continuations. For example:

```sh
hydragen init
hydragen init --name shared-rules
hydragen init \
  --name shared-agent-rules \
  --author "Example team" \
  --description "Shared agent rules."
hydragen init --non-interactive
```

Keep each source example line short enough for the reviewed narrow layout after
indentation, and preserve argument boundaries/quoting. Do not let generic prose
wrapping create an invalid copyable command. Reflow the input-route explanation
as short labelled prose, preserving every fact: no metadata flags selects guided
input only with three terminal streams and CI disabled; any metadata flag selects
flag-only input; name defaults to the directory name; optional author/description
are omitted when not supplied; the explicit override always disables questions.
Do not shorten help by dropping meaningful information.

At 40, 80, and 100 columns, normal prose and descriptions must wrap at word
boundaries with readable continuation indentation. Preserve an unbreakable token
that cannot fit rather than truncating or changing it; these widths are review
cases, not a promise of arbitrary-width support. No new CLI flag or environment
policy is introduced to select width.

**Alternatives rejected:** a fixed 80-column help for all terminals (does not
address narrower terminals), handwritten wrapping of clap's generated sections
(duplicates parser facilities), and adding colour to solve layout (does not
solve the observed problem).

### Adapt validation messages at the terminal boundary

Keep `validate_metadata_value` authoritative and unchanged. The private
`validate_answer` helper in `src/interaction.rs` can return a displayable prompt
message after delegating to it. Match the structured invalid-metadata error;
do not parse its rendered text or copy the whitespace predicate. Preserve the
empty-optional shortcut. If another error type is encountered, preserve its
diagnostic rather than silently treating it as a blank-value failure.

Use concise, field-specific instructions. The intended meaning is “Enter a
non-blank name” for Name and “Enter a non-blank author/description, or clear the
field and press Enter to skip” for optional fields. Final copy can be refined
during terminal review without changing these requirements. Do not prefix prompt
feedback with `--name`, `--author`, or `--description`. Flag-only diagnostics
continue to identify those flags and never start follow-up prompts.

Name's displayed directory default, Enter acceptance, typed override, independently
optional fields, unchanged accepted whitespace/Unicode, and literal `Skipped`
handling all remain unchanged. Whitespace-only optional input is still invalid;
it is not automatically trimmed into an empty skip. No write occurs before valid
input is complete.

**Alternative rejected:** changing `InitError::InvalidMetadata` display globally
would make flags less clear and couple library formatting to guided input. A
second validator would risk inconsistent acceptance rules.

### Improve occupied-path advice without inspecting the entry

Extend only the `ExistingLayerManifest` display branch in `src/lib.rs`, keeping it
plain and free of terminal/layout dependencies. Name the occupied path, explain
that init never overwrites existing entries, and suggest inspecting the entry or
running init from the intended different directory. State that this invocation
did not change files. Do not imply that the entry contains a valid layer manifest or
that Hydragen has read it. Retain the recognisable “already exists” reason.
Use deliberate line breaks between the reason, policy, recovery action, and
no-change outcome so the advice does not become one long line. This is fixed
plain-text formatting, not a general width-aware diagnostic renderer.

This advice must cover regular files, malformed layer manifests, directories, existing
symlinks, dangling symlinks, and an entry that appears during guided input. Keep
read-only preflight checks and exclusive creation as they are. Do not add a
confirmation, force flag, deletion instruction, validation/repair operation, or
an unimplemented follow-up command. Long paths remain intact even when the
terminal wraps them; this increment does not add a general diagnostic renderer.

### Verify presentation separately from behaviour

Extend the existing suites rather than add a framework:

- Binary-side builder tests using `clap::CommandFactory` and explicit widths
  verify wrapping/layout deterministically. Check meaningful content and ordinary
  prose widths, not byte-for-byte snapshots of all help output. Make any long-token
  exception explicit, not a blanket exemption for overlong descriptions.
- Parameterise the existing PTY harness with test-only dimensions and colour
  settings while retaining current defaults for existing tests. Apply the selected
  dimensions to both terminals; retain serialization, timeouts, and cleanup.
- Observe root/init help at 40, 80, and 100 columns, both `-h` and `--help`, and
  captured output without a terminal. Keep help/version prompt-free, write-free,
  and on their correct channels; argument errors remain nonzero on stderr.
- Update the retry test's obsolete flag-text wait and cover invalid Name, Author,
  and Description, corrections, and clearing an invalid optional answer to skip.
  Check no early files, input preservation, no sentinel `Skipped`, and the unchanged
  invalid-flag route. Exercise guided feedback with colour enabled and disabled.
- Strengthen occupied-entry tests with the safe-next-step assertions, retaining
  preservation checks for files/directories/symlinks and the layer-manifest-appears-during-
  questions race. No newly introduced path should invoke prompts or change files.

Create `verification.md` in this change during implementation. Record revision,
environment, terminal settings, test commands/results, and concise before/after
terminal evidence against F1–F3. Review beauty, enjoyment, functionality, and ease
of use explicitly, including how new wording affects the flow. Tests alone do not
prove aesthetic quality or enjoyment. Label replay evidence and unavailable
terminal/platform checks accurately; do not claim a GUI capture or user research
that was not performed. A discovered R4 issue is recorded separately, not silently
fixed under this proposal.

### Integrate documentation without replacing the principles

Update README's help/examples and existing-layer-manifest error example to match the
implemented interface. Add a brief presentation-decision link under question 10
in `openspec/design-questions.md`; keep broader unresolved topics intact. Review
whether any product clarification is warranted after implementation, but none
is currently needed in FOUNDATIONS.md, AGENTS.md, or the glossary.

Only mark R1–R3 complete in the repository remediation checklist and its saved
Project Plan after the corresponding implementation and evidence pass. Leave
R4/R5 and their verification gaps open. Keep the earlier audit immutable as the
baseline rather than editing old observations to resemble the new output.

## Risks / Trade-offs

- **Wrapping makes examples invalid to copy.** → Use deliberate short shell
  continuation lines; verify actual arguments/quotes as well as readable prose.
- **Captured output varies with environment.** → Use explicit builder widths for
  deterministic assertions and controlled real-terminal cases for integration;
  verify clap's documented fallback rather than assuming every pipe has no width.
- **Longer recovery text adds noise or overclaims safety.** → Keep one concise
  plain diagnostic; describe this invocation's actions and no-overwrite policy
  without assuming validity or recommending removal.
- **Feedback changes accidentally alter accepted metadata.** → Delegate to the
  existing validator and retain all whitespace/default/optional regression cases.
- **PTY tests become flaky.** → Extend the existing serialized harness with bounded
  waits and child cleanup; do not rely on fixed sleeps or unbounded prompt reads.
- **Three fixes grow into a redesign.** → Keep R4/R5 deferred; no prompt frame,
  colour policy, success-output, filesystem, or schema changes in this increment.

## Migration Plan

1. Reconfirm current specs/dependencies and the prior audit; preserve the completed
   principles changes already in this checkout.
2. Implement and verify help layout, then prompt feedback and occupied-path copy
   in focused edits. Review any lockfile changes as feature-related only.
3. Update README/question links, record before/after verification, and update only
   the completed remediation checklist entries. Run the full project checks and
   strict OpenSpec validation before handoff.

No layer manifest migration or invocation change is required. Error prose/help layout
are intentionally changed; success result output, flags, exit behaviour, and
channels remain compatible. If a fix needs revision, adjust only its presentation
changes, not unrelated foundations or user work. No archive or publication is
automatic.

## Open Questions

No unresolved product decision blocks R1–R3. Exact prose and clap's wrapping of
literal examples must be checked during implementation. Minimum terminal width,
R4 resize/grapheme behaviour, R5 visual preferences, user-perceived enjoyment,
agent interfaces, and performance thresholds remain outside this change.

## Primary documentation

Reviewed on 2026-10-17 for the locked clap 4.6.7 APIs:

- [Feature flags](https://docs.rs/clap/4.6.7/clap/_features/index.html):
  `wrap_help` enables terminal-sized wrapping.
- [`term_width`](https://docs.rs/clap/4.6.7/clap/builder/struct.Command.html#method.term_width)
  and [`max_term_width`](https://docs.rs/clap/4.6.7/clap/builder/struct.Command.html#method.max_term_width):
  detected width, 100-column unknown-width fallback, and a cap without fixing width.
- [`next_line_help`](https://docs.rs/clap/4.6.7/clap/builder/struct.Command.html#method.next_line_help)
  and [`after_help`](https://docs.rs/clap/4.6.7/clap/builder/struct.Command.html#method.after_help):
  description hierarchy and explanatory text shared with long help by default.
