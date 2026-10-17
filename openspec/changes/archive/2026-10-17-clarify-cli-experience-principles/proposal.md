## Why

Hydragen's developer-experience value explains how reuse should feel easy, but
does not explicitly require a beautiful and enjoyable CLI. Establishing that
expectation now gives future work a shared direction and provides a basis for
reviewing the existing prototype instead of assuming it already meets the goal.

## What Changes

- Extend the existing **DX — Developer Experience** value with beauty, enjoyment,
  functionality, and ease of use as equal design goals, retaining all eight values
  and the current requirements around reuse and everyday editing.
- Apply the goal to the whole CLI experience: help, guided prompts, results, and
  errors. Describe desired outcomes without selecting a visual style.
- Reinforce the principle in CLI development guidance while preserving plain,
  predictable automation, existing interaction rules, performance, and safety.
- After implementing the documentation changes, audit the current CLI against the
  agreed principle and existing requirements, using captured output and actual
  terminal sessions as evidence.
- Produce a prioritised, reviewable remediation plan for any gaps. Do not implement
  CLI fixes as part of this change.

## Capabilities

### New Capabilities

- `cli-experience-principles`: Documentation requirements for CLI experience
  quality and an evidence-based audit with a follow-up remediation plan. This is
  a design and review contract, not a new runtime feature.

### Modified Capabilities

None. Existing `cli-interaction`, `layer-initialization`, and `layer-manifest`
behaviour remains unchanged.

## Impact

- Product principle: `FOUNDATIONS.md`, under the existing DX value.
- Development guidance: `AGENTS.md`, under CLI development.
- Planning context: a narrowly scoped resolution under success criteria in
  `openspec/design-questions.md`, linking to this change's decisions and leaving
  broader success criteria open.
- Review deliverables: `audit.md` and `remediation-plan.md` in this change's
  directory, created during implementation, after the documentation update.
- Audit inputs: `src/main.rs`, `src/interaction.rs`, `src/lib.rs`, CLI documentation,
  existing specifications, and captured-output and terminal tests.
- No runtime code, command contract, layer manifest, dependency, or archived-spec changes.
  No claim that the existing CLI complies before the audit is performed.
