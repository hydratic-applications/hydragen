## Why

The existing selected-input regression already checks input-error reporting and
file safety at Name and format. This change adds coverage of a different test
environment: output stays readable, but there is no controlling terminal, the
terminal reached through `/dev/tty`, to use for fallback input. It adds
frame-triggered disconnection and a connected-input control without changing
console's existing fallback behaviour or claiming guaranteed before-selection
coverage.

## What Changes

- Add an isolated test-only launcher that keeps all three standard streams attached
  to terminals but gives the child no controlling terminal.
- Test input loss at Name and the format selector, and include a successful
  guided run through the same launcher as a control.
- Reuse the terminal-test session helpers and output checks, with the isolated
  reader and lifecycle support needed for bounded shutdown. Preserve the
  existing after-selection regression's readiness checks and assertions.
- Add a Unix-only `libc` development dependency for direct process setup and
  verification. Do not patch console or change production dependencies.
- Record exactly what the new coverage establishes: input is disconnected after
  a prompt becomes visible, not at a guaranteed point before input selection.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `test-ci`: Add unavailable-input coverage with no controlling terminal, including
  a connected-input control, bounded cleanup and explicit limits on timing claims.
  Permit narrow shared-helper lifetime corrections without changing the existing
  selected-input regression's normal completion rules.

## Impact

Implementation changes are limited to `tests/interactive_init.rs`,
`tests/support/unavailable_input.rs`, `Cargo.toml` and `Cargo.lock`, plus focused
OpenSpec records. Test implementation assets must remain outside `openspec/`.

The bounded cleanup safeguards and exceptional-path tests make this a larger
test-infrastructure addition than the initial launcher estimate. The support
stays isolated and does not migrate the existing regression's reader backend.

There are no changes to CLI behaviour, schema validation, interaction policy,
workflow or coverage settings. No upstream issue or dependency fork is required.

**This does not fix or close the separate runtime case where disconnected stdin
causes console to switch to a surviving controlling terminal.** Enforcing
original-stdin-only input in that situation remains outside this increment.
