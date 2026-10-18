## Why

Argument errors exit through clap before reaching Hydragen's shared presentation,
so missing sources and invalid flags look unlike operational failures. Make those
errors equally clear and deliberate without making help or malformed commands
depend on the current directory's layer manifests.

## What Changes

- Route argument errors for every command through one shared presentation adapter:
  red `✕ Error:` headlines in eligible terminals, plain `Error:` headlines in
  unattended or redirected output, with readable normal-foreground details.
- Give a missing add source a short explanation, usage, one example, and a help
  hint. Preserve clap's useful argument names, valid choices, and suggestions for
  other failures rather than replacing them with a generic message.
- Keep argument validation ahead of layer discovery. Explicit help and version
  remain successful plain output, available from any directory without layer
  discovery, schema setup, prompts, or writes.
- Preserve clap's usage-error exit status and stderr channel, shared colour
  controls, and display-only escaping of untrusted diagnostic values.
- Replace the existing policy exception that keeps clap usage errors plain even
  in eligible terminals. Help/version remain plain; no branded error openings or
  per-command style exceptions are introduced.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-interaction`: Shared parser-error presentation, actionable usage guidance,
  context-independent parsing, and preserved help/version/automation behaviour.

## Impact

- CLI entry-point parsing, shared presentation/diagnostic wiring, and focused
  captured-output and bounded real-terminal tests across root, init, and add.
- Agent guidance and the relevant design-question resolution. The foundations'
  existing DX/performance values already support this change; no core belief or
  layer-model revision is needed.
- No changes to accepted arguments, layer discovery, input defaults, content
  validation, mutation logic, dependencies, registry scope, or archived changes.
