# CLI diagnostics

This reference governs all current and future Hydragen CLI errors: make clear what
failed, what to do next, and whether work completed. Parser failures reject syntax
before an operation runs; operational failures concern accepted commands, such as
loading or writing a layer manifest. Both use shared compact composition. Guided
field rejection and cancellation retain cliclack interaction, not compact layout.

## Shared body structure

Compose useful blocks in this order, omitting those the failure does not need:

1. **Problem:** One sentence-case, punctuated `Error:` headline explaining what
   failed. A single failure leads directly with its concrete problem and affected
   source when useful, without a generic count or numbered wrapper.
2. **Useful context and correction:** Argument details, cause, field/file location,
   resolution base, choices or suggestion. Use consistent labels, including
   `Possible values:` and `Tip:` for correction. Do not bracket choice lists.
3. **Ordered source groups, for multiple dependency failures:** Use the headline
   `Dependency checks failed (N errors).` and source-labelled groups in established
   failure order. Count failures, not new additions. Distinguish supplied sources,
   existing declarations and the current layer. Omit redundant numbering; include
   occurrence positions only to distinguish otherwise identical labels.
4. **Syntax guidance, for parser failures:** Relevant `Usage:` followed immediately
   by at most one useful `Example:` in the same block, then a separate final
   `For more information, try 'hydragen ... --help'.` hint. An example demonstrates
   a valid next command, not an executed correction. Omit irrelevant syntax
   guidance from operational failures.
5. **Known operation state:** Report what was or was not written when relevant,
   once for the whole report. See [mutation and reporting state](#mutation-and-reporting-state).

Indent supporting lines two spaces and source-group content two more. Use one
blank line between blocks and consecutive lines within them. Keep labels
capitalized as shown and summaries punctuated. Preserve quoted spelling and case;
sentence case does not permit changing user data. Omit unhelpful context, repeated
paths and filler without empty labels or extra gaps. Root missing-command errors
use command choices, not a full-help body.

## Examples

These terminal transcripts show the approved structure without embedding colour
codes (ANSI sequences). In eligible terminals, only the `✕ Error:` headline is red;
supporting text stays in the normal foreground. Plain output omits the icon and
colour, preserving `Error:`, wording, indentation, block order and spacing.

### Missing add source

```text
✕ Error: Add requires at least one source.

  Usage: hydragen add [OPTIONS] <SOURCE>...
  Example: hydragen add ../shared-config

  For more information, try 'hydragen add --help'.
```

### Invalid init format

```text
✕ Error: Invalid value 'yam' for '--format <FORMAT>'.

  Possible values: yaml, yml, toml, json, jsonc
  Tip: Did you mean 'yaml'?

  Usage: hydragen init [OPTIONS]
  Example: hydragen init --format yaml

  For more information, try 'hydragen init --help'.
```

### Missing dependency layer manifest

Here `../layer-b` is an existing directory with no supported layer manifest:

```text
✕ Error: No layer manifest found in '../layer-b'.

  Tip: Choose an existing layer directory, or run 'hydragen init'
  in '../layer-b' to initialise it as a layer.

  No files were changed.
```

## Operational prerequisites and paths

For add's missing current layer, use `No layer manifest found in this directory
or its parents.`, then the tip `Run this command from an existing layer directory,
or run 'hydragen init' in the directory you want to use as a layer.` and the
no-files-changed result in separate blocks. Split the tip before `'hydragen init'`
so the command and explanation stay together. Omit a generic counted/numbered
wrapper, absolute invocation path and separate search paragraph: the headline
already explains the search without identifying a file to fix.

Prefer original supplied or stored references when they identify sources clearly;
keep paths to invalid, unreadable or conflicting files. Relative dependency paths
resolve from the declaring layer manifest's directory, not the invocation
directory. Show `Paths are relative to '<base>'.` only when the base is not already
evident or known to equal invocation. Use available context, never extra filesystem
work for formatting; unknown alias identity calls for conservative base context,
not assumed equality.

Reference spelling is not identity: different paths can resolve to one directory.
For two existing declarations with known equal identities, retain both labels and
necessary occurrence positions, then state `Both declarations refer to the same
layer directory.` rather than printing the same absolute path twice. Genuine
different-directory conflicts retain both relevant identity paths; unknown
identity is not equality. Keep precise file/field and line/column locations,
current-file symlink refusal paths, and destination/staging/cleanup paths where
needed. Do not blindly use basenames or edit opaque causes to shorten paths.

## Recovery and interaction variants

Recovery must address the known cause. A missing layer manifest in an existing
directory can warrant init advice; a missing directory, invalid existing content,
permission failure or internal schema setup failure cannot. Occupied init paths
need the filename and the explanation that init creates rather than overwrites a
layer manifest, not assumptions about valid contents or generic recovery prose.
Correct invalid selected contents, not a fallback file. Internal setup failures
must not blame valid input or ask users to retry it. Retain relevant flags, decoder
locations and schema-derived constraints. For known invalid stored dependency
references, explain the stored choices, including exact-version published
declarations. Do not confuse that with add's separate refusal to add published
dependencies.

Guided rejection keeps supported cliclack handling: name the field, explain the
schema-appropriate correction without flag terminology, and retain clear-and-Enter
instructions for skipping optional fields. Keep retry and no-early-write rules.
Deliberate cancellation produces one red cancellation report, a nonzero exit and
accurate no-change information; unexpected interrupted I/O remains an error.
Review adjacent warnings for consistent terms, labels and safe display. Retain
stderr and deduplication, and warn before dependent failures; do not add parser
help, branding or a new success style.

## Mutation and reporting state

Use typed facts and control flow, never English message matching, to distinguish:

- **Before creation or editing:** Verification/edit refusal can report no changes.
  An init pre-creation failure means this invocation did not create the target,
  not that the target is absent; an occupied file may already exist.
- **Failed new-file write:** Exclusive creation succeeded, so an empty or partial
  newly created file may remain. Identify it; do not claim no files changed.
- **Failed replacement:** The destination was not replaced, but staging may remain.
  Retain the primary failure, any secondary cleanup failure and exact staging path.
  Advise inspection before removal, not blind deletion or a blanket no-change claim.
- **Completed creation or replacement:** Report created or updated state accurately;
  subsequent warnings do not undo completion.
- **Failed completion reporting:** Say the layer manifest was already created or
  updated and preserve the delivery cause. If the operation was a no-op, say it
  remained unchanged instead. Do not claim cancellation or apply a universal
  no-files-changed footer.

## Command context and shared rendering

Keep clap responsible for argument grammar, validation, choices and suggestions.
Adapt structured clap and operational facts into one compact model; shared
composition owns layout. Match known typed variants exhaustively where practical.
Keep opaque external/unexpected causes as useful escaped data, not syntax or
layout instructions. Do not match English messages, regex-patch prose, introduce a
second parser or per-command body templates. Trusted examples are command content,
not a reason for command-specific layout.

Use verified command paths for parser usage and help: `hydragen --help`,
`hydragen init --help`, or `hydragen add --help`, including value failures where
clap supplies no usage. Generate usage from trusted command definitions, not
guessed argument text. A narrow mapping of clap's fixed generated `Usage: `
envelope to complete known command paths is allowed, not interpreting error prose
or replaying argument parsing.

If a parser failure's command is unknown, preserve that uncertainty: retain safe useful
details and general `hydragen --help` without claiming root ownership. Omit
unverified command-specific usage/examples rather than inventing a repair. Safe
fallbacks use shared composition, not framework debug field names as the interface.

## Presentation and safety boundaries

- Use the shared red `✕ Error:` headline only when all three standard streams are
  terminals, CI is disabled, `TERM` is not `dumb`, and no explicit colour disable
  applies. A nonempty `NO_COLOR` or `CLICOLOR=0` disables styling. Forced colour
  cannot override these gates; `--non-interactive` controls questions, not eligible
  styling. Do not add branding or guided-session frames to compact failures.
- Keep warnings/errors on stderr and existing exit statuses, including clap's
  usage-error status (ordinarily 2). Explicit help/version stay plain stdout with
  status 0 and no error headline; help retains its separate width-aware layout.
- Handle rejected invocations, help, and version before operational dispatch,
  current-directory resolution, layer manifest discovery or loading, schema
  preparation, prompts, or writes. Do not inspect project state to decide which
  syntax error to report. Add remains argument-only in every environment.
- Escape untrusted arguments, paths, values, causes and suggestions before layout.
  Controls become visible text, not terminal actions or invented diagnostic lines.
  Keep trusted layout separate from data. Display must not normalise or sanitise
  accepted values, alter paths or storage, rerun validation, or change filesystem
  behaviour.

## New-error checklist

When adding an error or known variant:

- Supply a concrete typed problem, affected source/file/field and relevant base.
- Preserve useful causes and provide cause-specific recovery when it helps.
- Identify known operation state; do not infer mutation from error wording. When
  reusing an adapter, do not assume the operation phase of its earlier callers.
- Escape untrusted data at the display boundary, preserving accepted values.
- Use shared composition or the supported guided variant, with a safe unexpected
  fallback rather than a panic or silently missing detail.
- Add a representative family fixture with behavioural assertions, then review
  its wording beside related errors. Shared layout cannot guarantee clear prose.

## Tests and experience review

Review root/init/add families: parser arguments, values, options, commands,
conflicts and fallbacks; init inputs, defaults and prerequisites; loading, schema
and encoding; add verification; editing, creation, replacement and cleanup; guided
input and cancellation; and completion reporting. Keep universal style, security
and state assertions separate from parser-only usage/example/help checks. Assert
shared order, punctuated sentence case, indentation, single block gaps, labels and
useful omissions; retain parser usage/example adjacency and help target. Do not
conceal competing body templates.

Cover single/multiple failures, existing versus supplied sources, ambiguous
occurrences, nested invocation bases, same-identity aliases, genuine conflicts and
current versus dependency symlinks. Exercise rare state, I/O, reporting and cleanup
failures through deterministic test seams, not unreliable real-disk failures.

Compare styled/plain structure and meaning, with colour only on eligible headlines.
Check CI, every redirected-stream combination, dumb terminals, explicit colour
disables against force, and `--non-interactive`. Hostile values and suggestions
must remain inert; ordinary Unicode, backslashes and accepted values must survive.
Test known command ownership and ambiguous fallback separately: a command-like
token must not redirect help.

Use captured output and bounded actual-terminal tests with child-process cleanup.
Check status, streams, prompts or their absence, and file state, including no-write,
no-op and partial/completed writes. Combine hostile layer manifest cases with a
call-order review: silence alone does not prove parsing preceded project access.
Preserve operational safety assertions.

Inspect representative errors together at normal and narrow terminal widths.
Assess beauty, enjoyment, functionality, and ease of use explicitly: the hierarchy
should be calm and readable, choices and recovery actionable, and one-shot output
free of unnecessary ceremony. Passing snapshots alone do not establish a coherent
experience.
