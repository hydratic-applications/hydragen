## Context

Guided init already discovers and groups author names, retains their source hints,
and offers native cliclack choices followed by custom entry and Skip. This change
adopts the presentation approved in the local terminal trial. It does not reopen
the archived discovery increment or its accepted upstream visibility deferrals.

The relevant part of design question 9 is how the human CLI presents those
choices. The foundations' DX value gives appearance, enjoyment and ease of use
equal weight with functionality. The approved layout improves scanning without
additional discovery or delays, consistent with the performance value.

## Goals / Non-Goals

**Goals:** Keep names easy to scan, show complete source context on focus, align
one icon per option, and preserve native navigation and exact accepted values.

**Non-Goals:** New discovery sources, entity verification, different defaults,
changed grouping, new flags, format-selector changes, dependency upgrades or
patches, fixing cliclack #115, and unrelated CI or timing-test repairs.

## Decisions

### Reveal sources inline only on focus

The focused active candidate shows a dimmed ` (source list)` after its name.
Other candidates and the two actions show no source hint. Submitted and cancelled
answers contain no added icons or source details. Each candidate still retains
its full hint; only its visibility changes.

The trial initially put a `From …` line below the focused name. The user found the
extra row jarring. Inline parentheses retain the compact list without hiding the
source information. Long text can still wrap naturally; do not truncate sources
or insert a dedicated detail row. One source stands alone, two use `and`, and
three or more use an Oxford comma, preserving exact source deduplication/order.

### Use one icon for source evidence, not a guessed entity type

`AuthorProvenanceKind` describes the accepted sources supporting a grouped name:

- Identity: Git settings/environment author, GitHub profile, local-account names,
  or environment username fallback. Use `👤`.
- Organisation: GitHub organisation membership. Use `🏢`.
- Mixed: both identity and organisation support. Use one `◈`, not two icons.
- Unspecified: project declarations alone. Use `◇`; a project author declaration
  does not establish whether its name identifies a person or an organisation.

Project evidence neither invents nor removes known support. Aggregate provenance
independently of hint deduplication, using only accepted source records. A company
name in a Git setting remains identity-backed, not verified personal authorship.
Eligibility for initial selection remains the existing, separate rule.

Use emoji pencil `✏️` for `Enter another author` and `⏭` for `Skip`. Measure the
whole glyph set to fix a common slot at least two display columns wide, right-pad
narrower glyphs, then append two separator spaces. The user requested a clearer
gap after the pencil; applying it to every option keeps labels aligned.
`TERM=dumb` uses `U`, `O`, `*`, `?`, `+`, `>` in the same slots. Disabling colour
removes ANSI styling, not semantic icons. Icons never become selector values.

### Omit only the redundant public GitHub host

Use `GitHub profile <login>` and `GitHub organisation <login>` for an exact,
case-insensitive `github.com` host. Retain `on <host>` for other hosts, including
GitHub Enterprise, because that distinguishes the account's actual instance.
Validate safe display components before omitting the suffix; this is not a new
host-selection, authentication or request policy.

### Keep cliclack responsible for interaction and redraw

Use the existing supported theme hooks and exact label/hint mapping to typed
actions. Keep the author theme scoped to that selector and reset it afterwards.
No custom input loop, terminal reopening, library patch, or alternate selector is
needed. Let the terminal wrap long logical rows and cliclack clear previous frames.

## Risks / Trade-offs

- Long source lists can still change physical row height through wrapping → keep
  every source and test movement/clearing at 40 and 80 columns in short viewports.
- Fonts differ in emoji rendering → use measured slots and a basic-terminal
  fallback; character-cell tests are not a guarantee for every font.
- cliclack #115 still hides choices in long membership-only lists → retain both
  issue-linked visibility assertions and their existing ignores, updating only
  expected active-row prefixes. Hydragen #14 owns the dependency follow-up.
- Native-screen assertions can mismeasure emoji variation selectors → measure
  them with the base glyph and test wrapping at the right edge.

## Migration Plan

No data migration is needed. Publish the approved renderer, source-hint formatting
and tests together. A later reversal would change presentation only, not layer
manifest content. Preserve the archived increment and its historical checklist.

## Open Questions

No product decision is left open for this increment. Upstream visibility, the
existing MSRV failure and an intermittent discovery timing test remain separate
follow-ups, not acceptance claims of this change. The current foundations and
glossary need no revision: this applies existing DX goals and introduces no new
product concept.
