## Why

The [CLI experience audit](../2026-10-17-clarify-cli-experience-principles/audit.md) found
three concrete problems: help that hard-wraps through words, guided validation
that speaks in flags, and existing-layer-manifest errors without a safe next step.
Fixing these together is a bounded presentation increment that applies the new
DX goals without changing init's input semantics or filesystem behaviour.

## What Changes

- Make root and init help width-aware, with readable option-description hierarchy
  and copyable examples at representative 40-, 80-, and 100-column widths.
- Use prompt-appropriate, field-specific guided validation feedback, including
  how to clear and skip an optional field, while retaining one shared validation
  rule and flag-oriented diagnostics for flag-only invocations.
- Explain the no-overwrite policy and safe next steps when the layer manifest path is
  occupied, without reading, repairing, deleting, or replacing the entry.
- Extend the existing captured-output and bounded terminal tests, update user
  documentation, and retain before/after evidence for the three audit findings.
- Keep plain output, streams, exit outcomes, defaults, independent optional fields,
  conservative interaction detection, cancellation safety, and exclusive creation.

## Capabilities

### New Capabilities

None. This improves the existing CLI rather than introducing a new interface.

### Modified Capabilities

- `cli-interaction`: Guided validation describes the current prompt and its
  recovery action, not a flag the user is not supplying.
- `layer-initialization`: Help has a width-aware layout; occupied layer manifest paths
  produce safe recovery guidance while preserving all no-overwrite guarantees.

## Impact

- `src/main.rs`: clap help layout, examples, and explanatory text.
- `src/interaction.rs`: presentation of shared validation failures in prompts.
- `src/lib.rs`: occupied-layer-manifest diagnostic wording only; validation and file
  checks/writes are unchanged.
- `Cargo.toml` and possibly `Cargo.lock`: enable clap's documented `wrap_help`
  feature without a deliberate clap/cliclack version upgrade or a UI framework.
- Existing unit, captured-output, and PTY tests; README help/error documentation;
  a narrow decision link in `openspec/design-questions.md` during implementation.
- Preserves the completed principles change and its historical audit evidence.
  The follow-up covers remediation R1–R3. R4's prompt wrapping/resize investigation
  and R5's human visual-direction review remain separate: neither justifies an
  assumed rendering fix, minimum terminal width, or custom theme here.
- No new commands, flags, layer manifest fields/formats, runtime services, or filesystem
  operations. Help and error prose can change; scripts must not rely on their
  exact wording or help line breaks. Success result output remains unchanged.
