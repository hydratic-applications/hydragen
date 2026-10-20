## Why

Parser errors now share a body, but operational errors still expose nested error
strings, redundant numbers and full paths, and generic recovery paragraphs. The
approved CLI-wide plan extends consistent, clear diagnostics to every current
error family and makes that consistency part of adding future errors.

## What Changes

- Use shared compact diagnostic composition for concrete problems, optional
  context and recovery, ordered source groups, syntax guidance where relevant,
  and accurate operation state. Preserve guided interaction's distinct layout.
- Present a single add failure directly. Present multiple failures as source-
  labelled groups without redundant numbering and without miscounting existing
  declaration failures as new additions.
- Prefer supplied or stored references over repeated absolute paths. Explain the
  declaring-directory base when necessary, and preserve actionable file,
  conflict, staging and cleanup paths and diagnostic locations.
- Adapt each current error family from structured facts, including init inputs
  and prerequisites, loading/schema failures, editing, replacement and cleanup,
  guided input/cancellation, unexpected errors, and post-write reporting.
- Expand the maintained diagnostic reference, contributor checklist and shared
  family tests so new errors inherit conventions and receive wording review.
- Preserve parsing, discovery, validation, mutation, ordering, security, status,
  stream and terminal-control behaviour. No new commands, flags or dependencies.
- **BREAKING (Rust source compatibility only):** Add typed creation state to
  `InitCreationError` so CLI adapters can distinguish pre-creation failures from
  a possibly partial newly created file. Downstream Rust struct literals must
  supply the new field; existing error formatting and CLI semantics are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-interaction`: CLI-wide diagnostic composition, concise unambiguous source
  context, cause-specific recovery, truthful operation state and durable review.

## Impact

- Binary presentation and typed error adapters, with narrowly scoped context or
  testability wiring where existing error facts are insufficient.
- Captured, real-terminal, security and deterministic rare-failure tests.
- `docs/cli-diagnostics.md`, `AGENTS.md`, and design question 10's resolution.
- This follows `unify-cli-usage-errors`; it does not reopen its completed tasks or
  rewrite archived decisions. Existing parser output and approved missing-current
  output remain regression anchors. No foundations or glossary revision is
  currently needed.
