## Context

The user approved replacing add's one-item success list with a concise sentence.
`unify-cli-diagnostics` intentionally excluded success output and is complete;
this linked follow-up changes success presentation only on the same PR/branch.
The foundations' DX and performance values support scaling feedback to the
interaction. Design question 10's dependency feedback decision is relevant.

## Goals / Non-Goals

**Goals:** One useful sentence for one effective requested result; preserve
readable groups for multiple or mixed results, shared styling and plain output.

**Non-Goals:** Changing add's parsing, verification, deduplication, references,
filesystem lifecycle, warning/error feedback, completion-state handling, or the
CLI-wide diagnostic contract. No new commands, flags, dependencies or rendering.

## Decisions

### Select the layout from verified outcomes

`Presentation::add_success` already receives distinct added and already-present
groups from the verified plan. When their combined length is one, use:

```text
Added layer-a to layer-b.
```

or:

```text
No changes needed: layer-a is already a dependency of layer-b.
```

Names are the validated dependency and destination layer names, not directory
spelling. Eligible terminals prepend the existing green `✓` and emphasise the
two names consistently; plain output preserves the sentence without decoration.
Output ends in one newline, without a separate heading, count, list or blank line.

Use outcome cardinality rather than argument count: duplicate and alias arguments
for one effective dependency still produce one sentence. Unrequested existing
dependencies do not affect the choice or appear in feedback. A repeated newly
added source remains added, never already present.

### Preserve multi-result output and operational behaviour

For more than one effective requested dependency, keep the current grouped
feedback byte-for-byte: destination headline, nonempty counted `Added` and
`Already present` groups, names in first-appearance order. All-repeat output still
explicitly says no changes are needed. Do not abbreviate a mixed result merely
because one group contains one item.

This is a presentation branch, not another verification/deduplication pass. Keep
success stdout, stderr warnings/errors, style gates and `--non-interactive`
behaviour. Do not change the completion writer, its flush policy, or truthful
created/updated/no-op reporting failures. Single all-repeat plans still bypass
editing and staging and retain the existing stored reference.

## Risks / Trade-offs

- Counting argv would misclassify aliases/repeats → use the two outcome groups.
- A shorter result could hide no-op status → retain explicit `No changes needed`.
- Shared style might drift → test captured/plain/styled equivalence, normal
  terminal controls and scoped names; preserve multi/mixed snapshots.
- Tests could lose filesystem checks during wording migration → retain source
  bytes, inode/mtime, reference spelling, no staging and post-write state checks.

## Migration Plan

Make the focused renderer change with unit/captured/PTY regressions, then align
current guidance and concise README text only where it describes the old rule.
Validate both this change and the unchanged diagnostic contract. Keep the work in
a separate commit on PR #7 without merging, archiving or main-spec syncing.

## Open Questions

None blocking. No new foundational principle, glossary meaning or interaction
policy is introduced.
