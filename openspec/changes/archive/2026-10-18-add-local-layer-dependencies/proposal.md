## Why

Hydragen can initialise a layer, but cannot yet declare a verified reuse
relationship with another layer. Add a focused dependency-declaration command
before retrieval and rendering, so developers can build and validate local layer
relationships without committing to a registry implementation.

## What Changes

- Add `hydragen add <source> [<source> ...]` for one or more explicit local paths.
  Discover the current layer by searching upward, and interpret relative source
  paths from its layer manifest directory. No prompts, fetching, or rendering.
- Introduce an ordered `dependencies` list of strings. Store local paths unchanged,
  without versions or duplicated names. Recognise existing published references
  such as `shared-config@1.2.0`, with exact versions only, but do not support adding
  them or contacting a registry yet.
- **BREAKING:** Replace unrestricted non-blank layer names with an explicit
  lowercase, npm-inspired naming policy throughout shared validation, including
  init, directory-name defaults, and every layer manifest loader. Do not silently
  normalise invalid names. Author and description retain their existing rules.
- Verify the current layer, all existing direct local dependencies, and every
  supplied source. Enforce unique declared names, reject self-dependencies and
  conflicting sources, and treat harmless repeats as successful no-ops. Expose
  these checks for reuse by other operations, including a future validate command.
- Verify batches before one update; preserve existing order and append new entries
  in argument order. Preserve comments and unrelated source content across all
  supported formats. Refuse an edit that cannot be performed safely.
- Use hidden sibling-file staging and atomic replacement, preserving file
  permissions. Refuse to edit a symlinked current layer manifest. Do not add
  locking or stale-content checks; interrupted processes can leave staging files.
- Provide compact, layer-name-focused feedback with separate lists for dependencies
  actually added and already present. Aggregate independent errors cleanly in
  argument order, with useful diagnostic paths and a plain-output equivalent.

### Deferred work

Publishing, registry access, adding published dependencies, registry-backed guided
search/version selection, retrieval, rendering, and broader validation remain
later increments. There is no lockfile generation, range selection, or tag support.
When published addition is implemented, adding a different version of an existing
layer must error rather than overwrite it; an explicit upgrade/version-change
workflow must be designed separately. These follow-ups are also in the project
backlog.

## Capabilities

### New Capabilities

- `layer-dependency-addition`: Local batch addition, current-layer discovery,
  reusable direct-dependency checks, name uniqueness, source-preserving editing,
  replacement safety, and CLI feedback.

### Modified Capabilities

- `layer-manifest`: Define names, ordered dependency references, strict exact
  published versions, and schema-driven content validation for every format.
- `layer-initialization`: Apply the stricter shared name policy to supplied names
  and directory defaults without changing init's location or creation lifecycle.
- `cli-interaction`: Keep init's guided-input policy while explicitly making the
  initial add command argument-only.

## Impact

- CLI parsing/help, diagnostics, presentation, and library operations for discovery,
  validation, source resolution, and editing existing layer manifests.
- `schemas/layer-manifest.schema.json` and the shared schema validator; existing
  fixtures and tests using names outside the new policy need deliberate updates.
- Source-preserving editing uses parser-guided insertions, with the selected range
  providers recorded in the design. Safe replacement mechanics require platform
  validation. No implementation asset may be loaded from `openspec/`.
- Captured-output and bounded real-terminal tests, failure-injection tests, and
  concise user documentation must cover the new command and compatibility change.
- No registry service, network dependency, Git mutation, workspace layout, or
  change to the foundations' core layer model is required.
