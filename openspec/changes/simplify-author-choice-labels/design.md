## Context

The user approved replacing emoji and long parenthesised provenance with short,
stable source summaries. An initial implementation swapped category text for
sources on focus; the user found that erratic and hard to scan. For example, Git
global user.name, the GitHub profile login and the system account name display as
`(Git, GitHub, local account)`. This addresses design question 9's author display
choice, not new discovery behaviour. It applies the foundations' DX value without
changing product philosophy or introducing a glossary term. Extend the existing
everyday-development scenario with the user's shared-name/wrapping example to
record why concise source context matters beyond the smallest screens.

## Goals / Non-Goals

**Goals:** Make names prominent, reduce wrapping on small and medium terminals,
and preserve honest source context with native cliclack navigation.

**Non-Goals:** Guessing entity types, new discovery or detail interfaces, changing
metadata, deadlines, defaults or dependency versions, or fixing cliclack #115.

## Decisions

### Separate detailed evidence from compact display

Retain the existing full hints and typed source records for source ordering,
safety, grouping and tests. Derive a separate summary directly from accepted
typed sources; never parse the English hint. In first-source order, collapse and
deduplicate these display families: `project metadata`, `Git`, `GitHub` (profile),
`local account` (including environment username fallback), and `GitHub organisation`.
Git environment/configuration sources share `Git`. Project filenames, Git keys and
scopes, and GitHub logins do not appear in summaries. Retain `on <host>` for
non-public GitHub hosts and validate login/host components as before. Join display
families with comma-space, without a final `and`; a source is a unit, not English
text to split. Detailed evidence remains distinct even when summaries collapse.

### Keep row text fixed while navigating

Keep native radio indicators and align every raw name and action at the same
column, without additional icon slots. Every visible candidate appends the same
dimmed parenthesised source summary whether focused or not. Only its radio marker
and name styling change with focus; wording and line wrapping do not. Remove the
separate identity/organisation/project category suffixes instead of showing both
descriptions. The summaries already retain source-family and organisation
distinctions without guessing entity types. Basic and colour-disabled terminals
use the same meaningful text. Actions have neither suffix nor icon.
Submitted/cancelled answers remain unchanged. Natural wrapping remains available
for long names or Enterprise hosts; there is no ellipsis, extra detail row or
expansion control.

Keep the existing exact raw-label/full-hint mapping to typed actions, supplying
summaries separately to the theme. This protects literal names resembling actions,
source labels or decoration. This decision supersedes the previous icon slots,
exhaustive focused hints, natural-language punctuation and focus-dependent category
swap only for picker display. The detailed evidence contract remains internal and
unchanged.

## Risks / Trade-offs

- Less configuration detail in the picker → retain full evidence internally;
  summaries deliberately identify source families, not individual settings.
- Suffixes or hosts can still wrap → test 40/60/80-column PTYs and stale-text
  clearing, and verify focus does not alter any row text or wrapping. More summaries
  are visible at once; do not promise all names fit one line or every short viewport.
- Existing upstream visibility defect → preserve the two ignored assertions and
  change only their renderer expectations. Do not claim it fixed.

## Migration Plan

Update presentation, tests and maintained guidance together, including that focused
foundations scenario addition. No data migration, new guiding value, glossary
revision, dependency patch or archive is needed.
The stable-row refinement was first tested locally in the main workspace. The user
accepted that trial and then explicitly authorised committing and pushing the
follow-up to PR #15. Merge and archive remain separate actions.

## Open Questions

None for this increment; the user approved the display direction.
