## Why

Hydragen's recent presentation improvements made help and feedback clearer, but
deliberately left branding and visual identity unchanged. A recognisable CLI
identity supports the foundations' equal goals of beauty, enjoyment,
functionality, and ease of use.

## What Changes

Implement the console design selected on 2026-10-17, beginning with init and
shared status feedback. The author has approved proceeding from exploration to
implementation.

- Use a dense, 16-column by 8-row braille interpretation of the author's icon.
- Separate the stable product identity from the operation being performed:
  **Hydragen**, a dimmed version, an uppercase operation label, and an operation
  description in the normal foreground.
- Preserve the selected alignment and spacing so subsequent work starts from
  the reviewed console design rather than reconstructing it from conversation.
- Record the accepted font-spacing limitation and the remaining design questions.
- Link the meaning of the name and icon to the selected CLI design.
- Reserve full branded openings for multi-step guided or long-running foreground
  sessions, once per session; keep one-shot actions concise and automation plain.
- Use cliclack where interactive input or a guided process is useful, not for
  every command. Keep a quieter completed state and clear active-question
  hierarchy through supported styling adjustments in those flows.
- Share green-check success, yellow-triangle warning, and red-cross error
  treatment across guided and compact terminal output; keep captured and
  unattended output plain.
- Use the same red-cross pairing for rejected input as for other errors, and red
  cancellation feedback. Preserve cliclack's interaction handling and distinguish
  correction, cancellation, and failure through placement, wording, and behaviour.
- Report init completion once as “Layer initialised, layer manifest created at:
  layer.yaml”, using the actual filename instead of an absolute directory path.
  Guided completion uses stderr without a second stdout result; flag-only
  completion uses stdout and can be styled in a terminal.

## Capabilities

### New Capabilities

None selected.

### Modified Capabilities

- `cli-interaction`: session identity, guided prompt/error styling, and cancellation
  reporting while preserving input routes and validation rules.
- `layer-initialization`: single completion message, shared status styling,
  conservative presentation eligibility, and revised success-output streams.

## Impact

Changes affect binary-side presentation, integration/PTY tests, and user/developer
guidance, not layer manifest content or filesystem behaviour. The delta
specifications and revised `AGENTS.md` guidance identify the replacements for always-plain
flag-only output and the separate guided stdout result. Preserve input-route,
cancellation, and safety semantics. No dependency, replacement prompt renderer,
help redesign, or new command is needed.

The meaning of the name and icon is recorded in
[docs/brand.md](../../../docs/brand.md), separately from the terminal layout.
