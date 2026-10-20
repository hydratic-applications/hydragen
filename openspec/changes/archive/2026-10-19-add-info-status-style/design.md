## Context

The user approved blue informational feedback for neutral no-change outcomes.
The completed `compact-single-add-results` change supplies current single/grouped
layouts, but decorates both added and already-present results as green successes.
This follows design question 10 and the foundations' DX value; no new foundational
principle or product glossary term is required.

## Goals / Non-Goals

**Goals:** Reusable compact info treatment, consistent all-repeat add feedback,
unchanged text/layout/plain output and successful operational semantics.

**Non-Goals:** A logging framework, new flags or dependencies, guided-session
redesign, reclassification of every read-only command, or changes to verification,
mutation, no-op bypass, reporting, errors, warnings, help or exit statuses.

## Decisions

### Add info at the shared presentation boundary

Extend the existing severity/style boundary with blue `ℹ` informational feedback.
Share its colour and symbol with add result rendering rather than hardcoding them
inside add. Info has no automatic `Info:` label, keeping neutral messages concise
and plain wording identical. Supporting lines stay in the normal foreground.

Success remains an affirmative completion, such as adding a dependency. Info
describes a neutral observation, such as no changes being needed. This distinction
is not a blanket rule that every read-only operation must become informational.
Keep warning/error labels, symbols and colours unchanged; existing guided info
continues through supported cliclack handling.

### Classify the complete verified add outcome

Use the existing verified no-op outcome, not argument count: a single or grouped
all-repeat result is informational. Any actual additions, including a mixed
added/already-present result, retain success styling. Single no-op names use bold
blue emphasis; grouped no-op destination emphasis is blue while supporting groups
retain normal foreground, existing bold labels and dim counts.

Keep every emitted word, newline, grouping rule, stdout channel and status 0.
CI, redirected streams, dumb terminals, colour disables and force interactions
use the existing style policy. `--non-interactive` still controls questions only.
Formatting adds no filesystem work, and no-op editing/staging remains bypassed.

## Risks / Trade-offs

- Info could be mistaken for failure → preserve explicit no-change wording,
  stdout and status 0; document the meaning instead of adding a noisy label.
- Command-local decoration could drift → reuse shared style/symbol selection and
  test standalone info, single/grouped no-ops and added/mixed results together.
- Existing snapshots assume all results are green → migrate only no-op decoration;
  retain plain byte assertions and storage, ordering and reporting guarantees.

## Migration Plan

Implement focused presentation and test changes, then update maintained status
guidance and the linked decision. This delta carries the full latest result
requirement from `compact-single-add-results`; apply that preceding delta before
this one if specifications are synchronised later. Do not rewrite earlier change
history or archive/sync/merge as part of implementation.
