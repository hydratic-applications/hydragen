## Why

Adding one dependency currently produces a generic update headline and a counted
one-item list. The user approved a one-line result for this common action, with
grouped feedback retained when several effective results need distinguishing.

## What Changes

- Report one newly added dependency as `Added <dependency> to <destination>.`.
- Report one requested pre-existing dependency as `No changes needed:
  <dependency> is already a dependency of <destination>.`.
- Select the compact form from the two verified outcome groups after
  deduplication, not from the number of source arguments.
- Keep multiple-result and mixed-result grouped feedback, ordering, counts and
  meaning unchanged. Preserve eligible-terminal styling and plain equivalents.
- Change presentation only: no parsing, verification, stored-reference, mutation,
  staging, no-op, warning/error, or completion-reporting-state changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `layer-dependency-addition`: Scale success feedback to the number of effective
  requested results, preserving distinct added/already-present semantics.

## Impact

- `Presentation::add_success` and its focused captured, terminal and unit tests.
- The relevant agent guidance, concise README usage explanation only if needed,
  and design question 10's feedback resolution.
- Separate follow-up to `unify-cli-diagnostics`, implemented after its final
  validation gate on PR #7's existing branch as explicitly selected. No new
  dependency, library API, command or flag; no merge or archive requested.
