## Context

The approved saved plan is **Clear, consistent CLI diagnostics**. The existing
`unify-cli-usage-errors` work is complete at 16/16 tasks: parser bodies and one
missing-current-layer prerequisite are coherent, while most operational bodies
still inherit nested library prose and add's numbered batch wrapper. The user
wants all current errors clear and consistent, with durable rules for new ones.

Confirmed inventory: `main.rs` routes parser/operation outcomes;
`usage_errors.rs` has a parser-private body; `presentation.rs` styles the first
line and indents the rest; `diagnostics.rs` composes init/add/warnings;
`interaction.rs` owns guided rejection, cancellation, directory defaults and
completion. Typed facts live in `lib.rs`, `dependencies.rs`, `layer_manifest.rs`,
its schema/edit modules, and `replacement.rs`. Reuse this inventory rather than
repeat exploration or redesign domain errors unnecessarily.

Design question 10 is relevant: the foundations already require beautiful,
enjoyable, functional, easy-to-use diagnostics in terminal and plain output.
Registry, retrieval, rendering, validation scope and filesystem safety policy are
not reopened. No new glossary term is needed.

## Goals / Non-Goals

**Goals:**

- One compact composition boundary for consistent hierarchy, labels, spacing,
  source grouping, context/recovery, optional syntax guidance and truthful state.
- Adapt every current error family and safe unexpected fallback; preserve
  guided field correction/cancellation as deliberate interaction variants.
- Prefer recognisable reference spelling without obscuring its resolution base,
  file/field locations, identities, cleanup evidence or underlying causes.
- Enforce structure and safety automatically; require human family-wide wording
  review when introducing errors.

**Non-Goals:**

- New commands, flags, dependencies, protocols, registry support, error codes,
  logging systems or a general-purpose diagnostics framework.
- Changes to parsing, validation/accepted input, discovery/selection/traversal,
  storage spelling, symlinks, edits, replacement, cancellation semantics or locks.
- Success, branding or help redesign; automatic archiving/merging; changing
  library `Display` wording merely to obtain CLI layout.

## Decisions

### Compose structured facts once at the CLI boundary

Extract the smallest reusable compact body from parser-only rendering. Support
a summary, optional context/correction, ordered source groups, optional parser
usage/example/help, and optional operation state. Adapters provide facts and safe
data leaves; shared composition owns whitespace, indentation, labels and styling.
Reuse the current clap facts/catalog and preserve its output as regression
anchors. Do not force operational failures to carry help/usage fields.

Reject matching or rewriting English `Display` strings and adding more bespoke
single-case body templates. Exhaustively match known typed families where
practical; safely retain opaque external causes and unexpected errors as data.
If typed context has been hidden inside an `io::Error`, retain a narrow typed
payload at the CLI boundary rather than recovering its meaning from prose.
No new validation or library behaviour is necessary.

### Make single failures direct and batches genuinely grouped

A single problem is the headline, with its source when useful. Multiple problems
use one concise `Dependency checks failed (N errors).` summary and source-labelled
groups in existing order. Labels distinguish supplied sources from existing
declarations and the current layer. Remove redundant outer numbering and normal
`source argument 1` prefixes. An argument/declaration position is permitted only
when identical visible labels otherwise make distinct occurrences ambiguous.

For the user's source example, the intended output is:

```text
Error: No layer manifest found in '../layer-b'.

  Tip: Choose an existing layer directory, or run 'hydragen init'
  in '../layer-b' to initialise it as a layer.

  No files were changed.
```

Do not recommend initialisation for invalid contents, schema setup errors,
missing directories, or permission failures as if they were absent layer
manifests. A source must still have its own layer manifest; no ancestor fallback
is introduced. The approved missing-current-layer transcript remains unchanged.

Multiple-source structure uses a two-space base indent, with one additional
two-space level for group content, for example:

```text
Error: Dependency checks failed (2 errors).

  Source '../layer-b'
    No layer manifest found.
    Tip: Choose an existing layer directory, or initialise this directory with 'hydragen init'.

  Source 'bare-name'
    Invalid source reference.
    Tip: Use an explicit local path beginning with ./ or ../, or an absolute path.

  No files were changed.
```

Group wording may carry cause-specific facts; groups cannot drop failures,
reorder sources, duplicate the global state statement or colour the body.
Single existing-declaration failures must remain identifiable as declarations,
not appear to blame the supplied source. A batch count is a count of failures,
not a claim that that many new dependencies were attempted.

### Display paths from existing context, not new filesystem work

Preserve the original supplied/stored reference separately from resolved
directory identity. Use that reference in source labels and derive a selected
filename relative to it when the known relationship is unambiguous. Keep native
path semantics; do not expand, normalise or alter stored/input references.

Pass invocation and declaring-directory context through binary wiring. Show a
`Paths are relative to '<base>'.` context only when the declaring base is not
already evident or known to equal invocation. This comparison must use available
facts, not extra canonicalisation/stat calls for formatting. When alias identity
is unknown, conservative explicit base context is better than a false equivalence.

Do not replace every path with a basename or strip absolute paths from opaque
causes. Keep precise paths for files to fix, ambiguous sources or conflicts,
current-file symlink refusal, and destination/staging/cleanup failures. Different
spellings for the same resolved directory must not be described as different
source identities. For two existing declarations with equal known identities,
retain both reference labels and necessary occurrence positions, explain that
both refer to the same layer directory, and do not repeat that absolute path.
Different-directory conflicts retain both relevant identity paths; unknown
identity must not be treated as equality. Full staging paths and both primary/cleanup causes remain
mandatory; the user must know exactly what to inspect before removing anything.

### Tailor recovery while preserving locations and state

Use concise sentence-case punctuated summaries, `Tip:` for actionable correction,
one blank line between useful blocks, and no filler. Keep field/instance paths,
format line/column information, relevant flags and schema-derived constraints.
Opaque messages are safely escaped leaves, not trusted multiline layout.

Init input/default/repository/discovery and creation errors receive the same
composition. Guided rejections keep cliclack's field-local frame, correction and
optional clear-and-Enter instructions. Deliberate cancellation remains one
red cancellation report; unexpected interrupted I/O remains an error. Review
warnings alongside dependent failures without changing warning order/deduplication.

Operation state comes from control flow or typed error facts, never an inference
from wording. Verification/edit refusal can report no changes; init write failure
can leave a partial newly created file; replacement failure has not replaced the
destination but can leave staging; successful creation/replacement stays successful
even when completion feedback fails. Preserve those distinctions and cleanup
instructions. Do not emit a universal no-files-changed footer.

The existing `InitCreationError` lacks a typed pre-create/write-failure distinction.
Add a narrow creation-state enum/field populated by existing control flow:
`NotCreated` means this invocation did not successfully create the target (not
that no target exists); `PossiblyPartial` means exclusive creation succeeded but
writing failed, so an empty or partially written new file may remain. Validation,
encoding and exclusive-open failures use the former; the post-open write-error
branch must construct the latter explicitly, not use the pre-creation
`From<InitError>` conversion. Document that conversion's assumption.

A private callback seam may inject failure only while writing to the already
exclusively created file. Production still calls `file.write_all`, with unchanged
observation timing, causes, `Display` and `Error::source`. Add no flush, sync,
cleanup, rollback, filesystem inspection or lifecycle policy. Tests must prove
pre-create failure, untouched collision, retained partial bytes and no rollback
or post-write observation on failure. Adding the public state field breaks Rust
struct-literal construction downstream; this source-compatibility cost is
explicitly accepted for the missing fact, not hidden by the local version status.

### Make new errors join the same contract

Extend `docs/cli-diagnostics.md` rather than create a second reference. Replace
narrow agent guidance with a short checklist: concrete problem, affected input or
file and base, useful recovery, known operation state, safe display leaves, shared
composition, and a representative family test. Known typed adapters should
require deliberate handling of new variants; unexpected payloads need safe
fallback rather than panic or missing detail.

Separate universal style/security checks from parser-only usage/help checks.
Cross-family fixtures must cover single/multiple failures, useful omissions,
ambiguous origins, current versus dependency sources, and deterministic rare
state/reporting/cleanup errors. Use actual captured and bounded PTY output,
including plain/terminal controls and narrow widths. Tests prove mechanics;
human review must still judge recovery and wording alongside related messages.

## Risks / Trade-offs

- Short paths can obscure the actual base or identity → explicit context and
  nested-invocation, alias, conflict and symlink regressions; no formatting I/O.
- Broad refactoring can change safety behaviour → keep typed domain operations
  unchanged, migrate in small reviewed phases, retain all behavioural assertions.
- Escaped opaque causes may still be verbose → preserve useful evidence rather
  than regex-edit external prose; put the clear Hydragen problem first.
- Identical templates can add noise → optional blocks and deliberate guided,
  grouped, syntax and post-write variants share conventions, not identical fields.
- Tests can bless unclear copy → inspect families together against all four DX
  goals, not only individual snapshots or colours.

## Migration Plan

1. Validate these artifacts and review the bounded execution plan.
2. Stabilise shared body/path context and migrate add verification as a family.
3. Migrate remaining operational/guided families and rare-state adapters.
4. Complete cross-family captured/PTY tests, reference/checklist and final review.

Existing parser/help/version and approved current-layer output remain anchors.
No data migration is needed. Keep changes in the existing PR without merging or
archiving. Run full locked tests, fmt, warning-denied clippy, strict validation,
packaging/build verification and an independent final result review.

## Open Questions

No product question blocks this increment. A lack of typed fact needed for safe
presentation must be surfaced and resolved with the narrowest CLI-only context
or test seam, not by changing domain behaviour or silently discarding detail.
