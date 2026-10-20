## Context

Hydragen currently creates a layer manifest with init and provides shared,
read-only discovery, parsing, and schema validation. It does not yet edit an
existing layer manifest, interpret dependencies, retrieve their content, or render
a workspace. The current loader retains decoded unknown fields, but not original
comments or formatting. Init's metadata encoder is not an editor: using it for
add would discard other fields and user-authored source text.

This increment establishes declarations and their immediate validity, not the
whole inheritance system. It supports the foundations' granular reuse,
traceability, and developer-experience values while leaving retrieval and workspace
decisions for a later increment. No change to the core layer model is needed.

## Goals / Non-Goals

**Goals:**

- Declare one or more local dependencies with one familiar, argument-only command.
- Find the enclosing layer predictably and preserve each supplied reference.
- Validate direct dependencies and unique names before one source-preserving edit.
- Reserve a concrete exact-version published-reference format without implementing
  publishing, a registry, or adding published dependencies.
- Share schema and relationship checks with future callers, including validate.
- Make results, harmless repeats, and aggregated failures clear and visually calm.

**Non-Goals:**

- Retrieval, rendering, contribution capture, workspace layout, or Git mutation.
- Dependency-chain traversal, cycle detection beyond direct self-dependencies,
  or establishing rendering precedence.
- Registry access, publishing, guided search, adding published references, or
  dependency upgrades. No ranges, tags, automatic version choice, or lockfiles.
- Editing through a symlinked current layer manifest, locking, stale-content
  detection, or a guarantee against concurrent lost updates.
- Replacing the existing init lifecycle or implementing the validate CLI.

## Decisions

### Separate adding a relationship from preparing its content

Expose `hydragen add <source> [<source> ...]`. Require at least one source, use
clap for usage errors, and never prompt in this increment, even in an interactive
terminal. Global `--non-interactive` remains accepted without changing this route.
Do not read piped answers or present a branded session opening.

`add` describes the developer's intent to introduce dependencies. A future command
may prepare already-declared dependencies after a clone; this increment does not
choose that command or introduce a placeholder implementation. A future guided
add flow will be useful when it can search a registry and list available versions,
not merely ask for the same path argument.

### Search upward for the nearest layer manifest

Wrap shared single-directory discovery in an add-specific ancestor search. Inspect
the invocation directory, then each parent through the filesystem root. Stop at
the first directory with any occupied supported filename, including a directory
or dangling symlink at such a filename. Apply the established priority there:

`layer.yaml` > `layer.yml` > `layer.toml` > `layer.json` > `layer.jsonc`.

Do not skip an invalid selection, try a lower-priority format, or continue toward
a valid parent layer. Unexpected inspection failures are errors, not absence.
Git roots are not boundaries. This intentionally differs from init's current-
directory/repository guard; shared discovery itself still must not search upward.

Report duplicates once per affected directory per invocation, before any dependent
failure, using the existing warning contract. Repeated references or directory
symlinks must not multiply the same directory's warning. A selected symlinked
current layer manifest is refused rather than followed for editing; read-only
dependency loading continues to support symlinked layer manifests.

### Resolve relative arguments from the declaring layer

Both command arguments and stored local references use the selected layer
manifest's directory as their base. For example:

```text
Invocation directory: /work/app/src
Layer manifest:       /work/app/layer.yaml
Argument:             ../shared-config
Target:               /work/shared-config
Stored reference:     ../shared-config
```

This follows the inspected current pnpm model and allows references to be stored
unchanged. Resolving arguments from the invocation directory, as npm does, would
usually require rebasing before storage and would contradict that preservation
choice. Help must explain the base explicitly.

Do not trim, simplify, canonicalise, convert to absolute, or translate a supplied
reference for storage. Internal filesystem resolution is separate. Paths may
contain spaces; name restrictions must not accidentally be applied to path text.
Do not add environment-variable or literal-home expansion; shell expansion that
has already occurred is outside the CLI's input interpretation.

### Use ordered reference strings without duplicated names

Define an optional `dependencies` array whose items are source-reference strings:

```yaml
name: my-project
dependencies:
  - ../shared-agent-rules
  - "shared-config@1.2.0"
```

Omission and an empty array both mean no dependencies. Preserve existing order and
append genuinely new entries in first-appearance argument order. Do not sort;
rendering precedence remains a later decision.

Reject maps, tuples, source objects, and separate local version fields. A map keyed
by name would duplicate the target layer manifest's name and permit disagreement.
A heterogeneous list of path strings and name/version tuples adds two entry
shapes without a benefit over a single reference string.

Classify by syntax, not filesystem existence. Explicit relative prefixes (`./`,
`../`, and native equivalents), standalone `.`/`..`, and explicit absolute paths
are local sources. Bare `shared-config` never means a matching local directory.
Require a version for a published declaration. An explicit local path that fails
resolution never falls back to a registry.

Recognise platform-specific explicit path syntax as path syntax rather than a
published name. Support native resolution; diagnose recognisable foreign or
ambiguous forms, such as a drive-relative path, rather than translate them.
Portable content validation and host-specific path interpretation remain separate:
an unsupported host path is not evidence of a published source.

### Accept exact published declarations but defer published addition

Recognise `name@version` and `@scope/name@version`. Names follow the shared name
definition. Versions must be complete, strict SemVer 2.0.0 identifiers, including
optional prerelease and build metadata. Preserve the complete accepted identifier.
Reject `v` prefixes, partial versions, leading zeroes prohibited by SemVer,
surrounding whitespace, empty selectors, ranges, and tags such as `latest`.

Examples are `shared-config@1.2.0`, `shared-config@1.2.0-beta.1`, and
`@example/shared-config@1.2.0+build.7`. Earlier exploratory caret-range examples are
superseded by this exact-version decision. Do not accept npm's broader Git,
alias, `file:`, or `link:` syntax.

An existing published declaration is checked offline for syntax and its declared
name participates in uniqueness checks. No network access or existence claim is
made. Supplying such a reference to add receives a clear unsupported-feature
error; it does not add the reference without verification.

No lockfile is generated. Exact published syntax does not make local dependencies
immutable or decide registry artifact immutability. When registry-backed addition
arrives, requesting a different version of an already-declared layer must error,
not overwrite it. Design an explicit upgrade/version-change workflow separately.

### Adopt an explicit lowercase name policy through the maintained schema

Names use lowercase ASCII letters, digits, `.`, `_`, and `-`, with optional
`@scope/name` structure and at most 214 characters in the full name. Both scoped
components must be nonempty. Unscoped names and scope components cannot start
with `.` or `_`; the package component of a scoped name can. Reject whitespace
and other characters. Do not introduce Node-specific reserved names.

Reject uppercase rather than silently lowercasing. This provides case-insensitive
identity for valid names without maintaining alternate spelling or Unicode case-
folding rules. Author and description retain their existing non-blank string
rules and preserve their accepted content.

The published npm guidance and its validator differ. This is an explicit
Hydragen interpretation of the published policy, not a claim of byte-for-byte
validator compatibility. In particular, allow a leading dot in a scoped package
component as the published reference describes. Registry availability, confusing
similarity, authorship, trademarks, and permissions are not local syntax checks.

Extend `schemas/layer-manifest.schema.json` with shared definitions for names and
reference syntax, including exact versions. All formats, direct parsed-content
validation, init flags and prompts, defaults, and complete write candidates must
use those definitions. Do not duplicate content constraints in ad hoc Rust checks
or separate schemas. Filesystem resolution and relationship checks, which require
other layer manifests, remain separate from schema validity.

### Verify direct relationships through reusable read-only operations

Load and validate the current layer and every existing direct local dependency
before editing. Resolve existing paths from the current layer manifest directory.
Also verify every supplied source independently where possible. Each target must
be a directory with a supported layer manifest in that directory; do not search its
parents. Use shared discovery, loading, and the maintained schema rather than
checking only filename presence.

Do not traverse a target's dependency chain or require its local targets to exist.
Loading still checks the target layer manifest's own content syntax. This keeps
cost proportional to direct sources and avoids designing graph resolution now.

Use declared names as identities. The current layer's name is reserved. Reject
duplicate names already present, including repeated declarations of the same
source and local/published collisions. Existing missing or invalid local targets
are errors: without their layer manifests, uniqueness cannot be established.

For new arguments, compare matching names and resolved directories:

- Already declared from the same directory: successful no-op; retain the stored
  reference even if the new spelling differs.
- Repeated new source in a batch: retain its first occurrence only.
- Same name from different directories, or a collision with an existing published
  reference: reject the whole batch; do not replace a source or introduce aliases.
- Name equal to the current layer's name: reject, including a different directory
  declaring that name.

Expose these checks and structured diagnostics without a terminal or file mutation
so a future validate command and other dependent operations can reuse them.
The broader `add-validate-command` proposal remains separate: its eventual default
scope is not limited to these checks, and its dependency traversal policy remains
open.

### Prepare a complete batch before a source-preserving replacement

Separate discovery/loading, content validity, relationship verification, edit
planning, filesystem replacement, and terminal presentation. Return enough result
data to distinguish actual additions from sources present before the invocation.

Aggregate independently discoverable source failures in argument order. Stop when
a prerequisite, such as reading the current layer manifest, prevents meaningful
further validation. Do not mutate the layer manifest or stage an output until
verification and safe edit preparation succeed.

Retain original source text alongside decoded data for editing. Use maintained
parser spans to plan insertion-only edits rather than serialising a mutated whole
document. Preserve unrelated original bytes and decoded values, existing metadata,
and comments in YAML/YML, TOML, and JSONC, including comments associated with
existing dependencies. JSON remains strict JSON. Introducing an absent property
can require inserting a separator beside an existing property; this is necessary
structural punctuation, not permission to rewrite the existing property's text.

The implementation investigation selects these range providers:

- Existing `jsonc-parser` 0.34 AST and structural/comment tokens for JSON/JSONC.
  Ranges are UTF-8 byte offsets with exclusive ends. Keep the established decoder's
  stricter syntax policy; permissive parser defaults must not define acceptance.
- Immutable `toml_edit` 0.25 `Document`, array, and value spans for TOML. Do not
  render a whole `DocumentMut`: its documented limitations include dotted-key
  ordering and missing final newlines. Insert absent root dependencies before
  table context rather than blindly at EOF.
- The existing `serde_saphyr::granit_parser` for YAML/YML structure, styles,
  anchors/aliases, and scanner token spans, enabling comment support as needed.
  Use byte ranges, not character indexes, and do not assume a block collection's
  end event is a closing-delimiter span. Preserve stream directives and document
  boundaries as well as block, indentless, and flow sequences.

Encode only new scalar literals. An unrelated anchor or an anchored dependency
scalar is not grounds for blanket refusal. An alias used as the dependency list,
or a shared sequence anchor whose mutation would alter another field, requires a
specific safe strategy or a no-write refusal. Ordinary comments/arrays and files
without final newlines remain supported cases. Parser APIs provide locations,
not proof of preservation: tests must assert unchanged source/comment bytes in
addition to validating decoded output.

Check the prepared result with the established parser and maintained schema before
writing, and establish that only the intended dependency append changed decoded
content. If an anchor, alias, or other source construct cannot be edited without
changing unrelated meaning or losing comments, refuse with specific guidance.
Do not fall back to generic reserialisation. The refusal escape hatch does not
excuse missing support for ordinary arrays and comments in supported formats.

An all-repeat batch does not rewrite the layer manifest or create a staging file.
For an actual update, create a hidden temporary file beside the selected layer
manifest, write the prepared bytes, copy the original file permissions, synchronise
the staged file, and atomically replace the original. Use `tempfile` for exclusive
sibling creation and explicit cleanup, convert to its owned temporary path after
syncing/closing the file, commit with `std::fs::rename`, then infallibly disable
cleanup of the old temporary pathname. This avoids post-replacement permission
repair and staged-attribute normalisation. Do not intentionally truncate the
original or fall back to non-atomic copying. Attempt cleanup on success and all
handled failures; diagnose a cleanup failure accurately if cleanup is impossible.

The sibling location follows pnpm and avoids cross-filesystem rename failures from
a system temporary directory. It is not announced in normal CLI output. Abrupt
termination can leave a hidden sibling staging file in the repository. Preserving
permissions is not a promise to preserve ownership, every ACL, or extended
attribute. File synchronisation and atomic replacement are not unconditional
power-loss durability guarantees.

Do not add a project lock, stale-content recheck, or automatic merge. Concurrent
writers can overwrite each other's changes; this accepted limitation keeps the
increment close to pnpm's ordinary package-manifest update model. Reject known
symlinked current layer manifests; this is not a claim of protection against every
adversarial filesystem race.

### Report actual additions and pre-existing dependencies separately

Use one compact result on stdout, naming the destination layer rather than its
layer manifest path by default. List dependency names under separate `Added` and
`Already present` groups with counts. Omit empty groups; preserve first-appearance
argument order within each group and list each dependency once.

`Already present` means present before this invocation. A new dependency repeated
later in the same batch appears only under `Added`. A no-op reports no changes
needed and lists the requested dependencies already present, not unrelated ones.

An illustrative plain result is:

```text
Dependencies updated for my-project.

Added (2)
  shared-agent-rules
  shared-config

Already present (1)
  shared-tooling
```

Errors use stderr, one clear command-level headline, and readable source-specific
groups with reasons and correction guidance in argument order. Retain diagnostic
paths where necessary, including the relative-path base when relevant. Emit each
affected directory's duplicate warning at most once before its dependent failure.

Use the shared presentation policy: eligible terminal success has a green check,
warnings use yellow warning headlines, and errors use a red cross. Supporting
details stay readable in the normal foreground. CI, redirected streams, and
explicit styling disables retain meaningful plain text. `--non-interactive` does
not itself disable eligible status styling. No branded opening or session frame.
Review beauty, enjoyment, functionality, and ease of use together, including
narrow terminals, long names, multiple failures, and plain output.

Do not claim success before replacement, or claim no changes after replacement
succeeds but feedback fails. In the latter case, explain that the layer manifest
was already updated. Staging/cleanup failures must not be disguised as deliberate
cancellation or unconditional no-files-changed outcomes.

## Risks / Trade-offs

- **Source preservation across formats is substantial work** → evaluate maintained
  editors first, keep format-specific code separate, and test byte preservation
  outside dependency edits plus decoded-data preservation. Reject genuinely unsafe
  constructs rather than weakening the contract.
- **Stricter names break existing prototype data** → identify the compatibility
  change prominently, update project-owned examples deliberately, and give precise
  diagnostics. Never silently rename layers or user directories.
- **Nested execution uses a non-cwd path base** → document it in help and examples,
  and test identical arguments from the layer root and nested directories.
- **Upward search can cross Git boundaries** → stop at the nearest occupied layer
  manifest directory, never skip invalid selections, and name the target layer in
  feedback. This follows the chosen familiarity tradeoff, not init's guard.
- **Existing broken local dependencies block additions** → explain each problem;
  do not infer missing names or bypass uniqueness checks.
- **Staging leftovers and concurrent lost updates remain possible** → clean up on
  handled paths, report failures honestly, and document the accepted limitations.
- **Exact published references can appear before registry support** → distinguish
  offline declaration validity from verified existence; unsupported additions fail
  explicitly rather than silently accepting unverified sources.

## Migration Plan

1. Implement and test shared schema/name/reference changes, including init's
   unchanged-value defaults and prompt-specific diagnostics. Existing data with
   invalid names requires deliberate author correction.
2. Add bounded direct-dependency verification and the source-preserving edit plan,
   then atomic replacement and the CLI integration. Keep all implementation assets
   and fixtures outside `openspec/`.
3. Update the concise README, relevant glossary definitions, and agent guidance to
   explain the new command, exact-reference boundary, and safety limitations.
   Review foundations for clarity, without changing the agreed core philosophy.
4. Run regression, packaging, failure-injection, captured-output, and bounded real-
   terminal checks. Reconcile overlapping implemented presentation changes without
   reverting their styling or input policies.

The command creates no cache, lockfile, or persistent migration state. Removing
the command does not restore formerly accepted names or reinterpret declarations;
reverting schema changes would need a deliberate compatibility review. Do not
automatically rewrite user-authored layer manifests as a migration strategy.

## Open Questions

No product-level question is left open for this increment. The range-provider
and native-replacement selections above need runtime conformance tests against
the fixed preservation, permission, and safety requirements; surface a conflict
rather than silently weakening them.

Future work remains deliberately open: registry design and publication rules,
registry-backed guided selection, the explicit upgrade/version-change interface,
retrieval, graph handling, rendering, and broader validate behaviour. Follow-up
work must retain the rule that add never overwrites an existing published layer
with a different version.

## Source-backed comparisons

Observed during design on 2026-10-18; these sources inform choices rather than
making npm/pnpm behaviour a dependency of Hydragen:

- [npm naming guidance](https://docs.npmjs.com/package-name-guidelines),
  [name reference](https://docs.npmjs.com/cli/v12/configuring-npm/package-json#name),
  and [scope rules](https://docs.npmjs.com/cli/v12/using-npm/scope#description).
  [Validator v8](https://github.com/npm/validate-npm-package-name/blob/v8.0.0/lib/index.js)
  differs from that prose; the explicit policy above is authoritative here.
- [SemVer 2.0.0](https://semver.org/spec/v2.0.0.html) defines exact-version syntax.
- [jsonc-parser 0.34 AST/range APIs](https://docs.rs/jsonc-parser/0.34.0/jsonc_parser/),
  [toml_edit 0.25 document spans and limitations](https://docs.rs/toml_edit/0.25.17+spec-1.1.0/toml_edit/),
  and [Granit 2.0 parser/scanner spans](https://docs.rs/granit-parser/2.0.0/granit_parser/)
  support the insertion-only approach. They do not independently guarantee this
  change's source-preservation contract.
- [tempfile 3.27 owned temporary paths](https://docs.rs/tempfile/3.27.0/tempfile/struct.TempPath.html)
  provide explicit cleanup and an infallible success-side cleanup disarm after
  [native rename](https://doc.rust-lang.org/1.96.0/std/fs/fn.rename.html).
  Native same-filesystem replacement is not a promise covering every filesystem
  provider or hardware failure; permission-denied commits can fail without
  altering the original.
- [npm upward discovery](https://docs.npmjs.com/cli/v12/configuring-npm/folders#more-information)
  and [pnpm 12.14.0 prefix search](https://github.com/pnpm/pnpm/blob/v12.14.0/pnpm/crates/cli/src/cli_args/prefix.rs)
  support nested invocation, with tool-specific markers and workspaces that this
  increment does not import.
- [pnpm local add resolution](https://github.com/pnpm/pnpm/blob/v12.14.0/pnpm/crates/package-manager/src/add/aliasless.rs)
  uses the selected project directory; [npm local paths](https://docs.npmjs.com/cli/v12/configuring-npm/package-json#local-paths)
  normalise references differently. Hydragen deliberately keeps bare explicit paths.
- [pnpm manifest serialisation](https://github.com/pnpm/pnpm/blob/v12.14.0/pnpm/crates/package-manifest/src/serialization.rs)
  stages a sibling file; [manifest saving](https://github.com/pnpm/pnpm/blob/v12.14.0/pnpm/crates/package-manifest/src/lib.rs)
  does not provide ordinary concurrent-writer protection. [npm saving](https://github.com/npm/cli/blob/v12.2.0/node_modules/@npmcli/package-json/lib/index.js)
  instead uses direct writing. Hydragen chooses staged replacement without adding
  a broader concurrency guarantee.
