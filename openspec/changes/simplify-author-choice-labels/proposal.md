## Why

Author emoji feel out of place in the CLI, while exhaustive source hints wrap even
on medium-width terminals. Use quieter text and short source summaries so names
remain easy to scan without losing the evidence behind a suggestion.

## What Changes

- Replace author and action icons with plain labels. Unfocused names get a dimmed
  source-evidence category; focused names get a compact parenthesised source summary.
- Summarise and deduplicate source families instead of listing configuration keys,
  scopes, project filenames, and account logins. Retain organisation and Enterprise
  host distinctions. Do not truncate or add an expansion interaction.
- Preserve discovery, exact-name grouping, eligibility, ordering, navigation,
  cancellation, accepted values and known upstream display deferrals.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-interaction`: Replace author icon slots and exhaustive focused provenance
  with text categories and concise source-family summaries.

## Impact

Author discovery's choice presentation data, the scoped cliclack theme, unit and
terminal regressions, README and agent guidance. No dependencies, filesystem rules,
input policies, or layer manifest content change.
The existing foundations scenario also captures the supplied shared-name/wrapping
example, without changing the guiding values.
