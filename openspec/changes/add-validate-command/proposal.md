## Why

Developers need to know whether a project conforms to the requirements of the
particular Hydragen version they are running, including whether its layers,
contributions, and dependency relationships are valid. Capture this future command
now so that validation is not mistaken for manifest parsing alone while the layer
and dependency model is still being developed.

## What Changes

The following is agreed product intent for a future `hydragen validate` command,
not an implementation-ready design:

- By default, validate everything applicable to the project: its manifest,
  contributions (including patches and deletion markers), and compatibility with
  its base/dependency layers. Detect invalid or broken layer, contribution, and
  dependency relationships, not just malformed individual files.
- Allow users to explicitly limit what is validated. Conceptual examples are
  “validate only the manifest” and “validate patches”; these are use cases, not
  settled CLI syntax. Whether selection uses arguments, flags, or subcommands
  remains open.
- Assess conformance to the requirements supported by the running Hydragen
  version. A successful check is not a promise of compatibility with every past
  or future Hydragen version, nor proof that the application builds or passes its
  tests. How version requirements are represented is still undecided.

This change currently records the proposal only. It does not add a command or
authorise implementation; `init` remains the only implemented project command.
Design, specifications, and tasks must follow further exploration and planning.

## Capabilities

### New Capabilities

- `layer-validation`: Proposed capability for project-wide validation against the
  running Hydragen version, with all applicable checks by default and explicitly
  selected narrower scopes. Its detailed boundaries require planning before a
  specification is written.

### Modified Capabilities

None proposed at this stage. Reuse existing manifest requirements rather than
inventing a competing definition of validity. Further planning must identify any
necessary changes to existing capability requirements explicitly.

## Impact

- Future work will affect CLI command handling, shared validation logic, user
  documentation, and tests. The implementation structure and any additional
  library dependencies are not selected here.
- Manifest checks should build on the shared discovery, loading, and validation
  work in [support-layer-manifest-formats](../archive/2026-10-17-support-layer-manifest-formats/proposal.md)
  and its [discovery/loading design](../archive/2026-10-17-support-layer-manifest-formats/design.md#separate-discovery-loading-validation-and-presentation).
  That completed change deliberately excludes a validate command and must remain
  separate from this future work.
- The existing [manifest specification](../../specs/layer-manifest/spec.md) provides
  metadata and supported-format context, including the synchronized format and
  schema decisions. Neither its metadata schema nor format parsing alone
  defines project-wide validation.
- The [layer model](../../../FOUNDATIONS.md#the-layer-model) and
  [glossary](../../../GLOSSARY.md) establish contributions and dependencies. A
  patch is named after an inherited file with `.patch` appended, but its internal
  format is unresolved. A deletion marker is an empty file with `.delete`
  appended; its contextual checks still need design. These conventions are
  context, not a complete validation algorithm.

## Non-goals

- Running application builds, tests, or general-purpose code quality checks.
- Automatically repairing, rewriting, migrating, or otherwise correcting project
  content or dependency relationships.
- Settling dependency references, patch syntax, composition rules, network
  behaviour, or version declarations merely to make this proposal appear ready.
- Implementing validation or changing the active manifest-format work in this
  proposal-only capture.

## Planning still required

Use the relevant [open design questions](../../design-questions.md), especially
shared sources and references, partially shared files, incompatible updates, and
patch structure, to guide a later focused exploration:

- Which supported rules make a manifest, contribution, or relationship valid for
  a particular running version? How should invalid version declarations and
  unsupported versions or features be recognised and reported, without inventing
  a version field or compatibility scheme now?
- What patch format will Hydragen use? How will validation check patches against
  inherited content, and how will it check deletion targets, empty markers, and
  conflicting contributions? Which checks require composition of layer results?
- Which dependency sources and selected versions must be available, how far
  should validation follow dependency chains, and which relationship and
  compatibility checks are required? What happens when dependencies are missing
  or inaccessible? Local availability, fetching, and offline/network behaviour
  are not decided here.
- How should users select and combine scopes? Which prerequisite checks does a
  narrow scope need, and how should a partial check be distinguished from full
  project validation? How is applicability determined without silently omitting
  checks that cannot be completed?
- How should errors, warnings, unsupported features, and incomplete checks be
  presented? Should validation continue after individual failures? Exit codes,
  output streams, diagnostic structure, and any machine-readable output contract
  still require design within the existing CLI conventions.
- What is the first useful, reviewable implementation increment, and what
  prerequisites must land first without weakening the default-all intent?

Do not create design, specification, or task artifacts until that planning is
requested. This proposal preserves intent and open questions, not implementation
readiness.
