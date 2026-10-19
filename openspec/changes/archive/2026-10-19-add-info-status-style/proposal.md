## Why

A green success check makes an already-present dependency look newly added.
The user approved a shared informational style for neutral outcomes, starting
with all no-change add results, without changing their successful exit status.

## What Changes

- Add reusable blue `ℹ` compact informational feedback, without an `Info:` prefix.
- Use it for single and grouped all-repeat add results; actual additions and
  mixed results retain green success styling.
- Preserve every result's wording, layout, stdout delivery and exit status 0.
  Plain output remains unchanged, without colour or status icons.
- Record the semantic distinction and tests so future neutral feedback reuses
  the shared style rather than a command-local colour or symbol.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-interaction`: Shared compact info style and existing automation controls.
- `layer-dependency-addition`: Informational no-op results rather than success
  decoration, preserving the compact-single and grouped-result contracts.

## Impact

- Shared presentation, focused unit/captured/terminal regressions, maintained
  status guidance, and design question 10's feedback resolution.
- Follows `compact-single-add-results` on PR #7's explicitly selected branch.
  No parsing, validation, filesystem, library API, dependency, error/warning,
  guided interaction, or completion-reporting changes; no merge/archive requested.
