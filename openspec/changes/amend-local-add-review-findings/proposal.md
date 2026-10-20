## Why

The review of PR #7 found YAML edits that can change trailing string content or
disturb multiline collection layout despite passing decoded-data checks. Focused
corrections are also needed for misleading diagnostics, environment-dependent
tests and current documentation before delivering the reviewed local-add work.

## What Changes

- Refuse specific YAML insertion boundaries that change block-string meaning,
  including existing dependency tails and terminal scalar keys with implicit null
  values, without broadly refusing safe block strings.
- Preserve multiline YAML flow entries, closing delimiters and attached comments,
  allowing separate bounded comma/item insertions where needed.
- Strengthen preservation regressions with independent exact-output and permitted
  insertion expectations alongside existing decoding/schema checks.
- Correct whitespace-only diagnostic separators, non-regular-entry causes and
  non-string metadata wording without changing the public `NotString` variant.
- Stabilise copied-help examples and permission-denial fixtures through explicit
  width and capability-aware setup, retaining meaningful failure coverage.
- Clarify completion reporting: write/flush errors returned by the established
  fallible path remain fatal and stateful; successful calls do not guarantee
  receipt. Warning/error diagnostic delivery remains best effort, without new
  propagation, descriptor checks, rollback or status changes.
- Correct the generated YAML walkthrough and colour summary; document existing
  Rust source incompatibilities and atomic replacement's effects on hard links
  and open handles. Do not promise API stability or a release.
- Preserve the accepted integration baseline, including bare-number version
  output, incoming coverage/harness work and the user's cancellation wording.
  Keep library Display and CLI typed-fact adaptation distinct rather than
  introducing a generic wording refactor.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `layer-dependency-addition`: Precise YAML source/layout preservation and
  independent regression evidence, replacement-identity documentation, and a
  bounded completion-reporting clarification.
- `cli-interaction`: Literally empty compact diagnostic separators and truthful
  completion reporting at the established fallible boundary, retaining existing
  best-effort diagnostic delivery.
- `layer-initialization`: Qualify post-creation completion reporting without
  changing non-fatal observations or best-effort warning delivery.

## Impact

YAML insertion planning, shared diagnostic presentation/typed adapters, selected
integration fixtures, current documentation and review evidence. Existing
requirements already govern most bug corrections; unrelated capabilities and
historical archives remain unchanged. No new command, flag, schema grammar,
production decoder, runtime service, dependency, registry operation, lock or
concurrent-edit policy is introduced. This follow-up documents existing Rust
source breaks rather than introducing a new API shape or stability commitment.
