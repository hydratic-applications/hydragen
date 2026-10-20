## Why

Showing every author source beside every choice makes guided init harder to scan.
The approved terminal trial keeps names prominent, reveals sources only for the
focused choice, and uses aligned icons without changing which author is accepted.

## What Changes

- Show the full source list in dimmed parentheses to the right of only the focused
  author name. Do not add a separate detail row; allow normal terminal wrapping.
- Join two sources with `and` and three or more with an Oxford comma.
- Prefix author choices with one icon describing their source evidence, and use
  an emoji pencil for custom entry and a skip icon for Skip. Keep equal-width
  slots and a two-space icon-to-label gap, including basic-terminal fallbacks.
- Omit redundant `on github.com` text while retaining non-default GitHub hosts.
- Preserve discovery, grouping, ordering, initial selection, native interaction,
  cancellation, stored values, and the existing upstream visibility deferrals.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-interaction`: Define focused inline source hints, source-list wording,
  provenance icons, aligned action icons, and public-versus-Enterprise host hints.

## Impact

Changes are limited to author-source presentation in `src/author_discovery/mod.rs`,
the Author theme in `src/interaction.rs`, and their unit and terminal tests. No
new dependency, flags, network requests, metadata fields, or filesystem behavior
are introduced. The format selector is unchanged.

The archived `suggest-init-author` record remains historical. Hydragen #14 still
owns the two deferred cliclack visibility regressions; this change does not fix
or close them. The unrelated PR #15 MSRV failure remains separate work.
