# Project guidance

## Alignment with the foundations

Read [FOUNDATIONS.md](FOUNDATIONS.md) before exploring, proposing, or implementing
changes to Hydragen. Check proposals and significant implementation decisions
against its problem statement, guiding values, and scenarios.

After significant changes, review whether the foundations need additional
specificity, context, or clarity. Update them where warranted without overruling
the core philosophy to accommodate an implementation. Surface conflicts for
discussion, keep unresolved questions open, and make any revisions to the
foundations explicit and explained.

## OpenSpec planning and decisions

Read [openspec/design-questions.md](openspec/design-questions.md) when exploring,
planning, or implementing changes. It records unresolved product and design
choices; it is not a specification of agreed behaviour.

Identify which questions affect the current change and resolve only what that
increment needs. Do not silently turn an unanswered question into a product
commitment or require every question to be answered before work can begin.

Record decisions and their rationale in the relevant OpenSpec change's design
and specifications. Update the question list with a brief resolution and a link
to that decision. Reflect agreed product beliefs, goals, and meaningful constraints
in FOUNDATIONS.md where appropriate; keep implementation details in OpenSpec.

## Terminology and glossary

Consult [GLOSSARY.md](GLOSSARY.md) when unsure what a Hydragen term means or which
term to use. Use its terminology consistently in documentation, discussions, and
code rather than introducing alternative names for the same concept. Use "layer
manifest" in full for Hydragen's identifying file, including in CLI text, code
names, and tests; reserve bare "manifest" for unrelated tools such as Cargo.

When exploration, design, or implementation introduces a new term or clarifies an
existing one, consider adding or updating its glossary entry. Check existing
entries first to avoid duplicates. Keep definitions concise, understandable
without prior conversation, and limited to agreed meaning; do not use a glossary
update to settle an unresolved design question. Raise ambiguous or conflicting
definitions for discussion before changing their meaning.

Keep glossary entries focused on what a term means based on agreed decisions.
Do not include design-status commentary such as "not yet decided" or lists of
unresolved questions. Keep those in openspec/design-questions.md or the relevant
OpenSpec change instead.

Within the glossary, link references to other defined terms to their headings on
the same page, including plural and possessive forms. For example, write
`A [contribution](#contribution) that...` in the addition definition. Avoid links
from a term's definition back to itself. When adding or revising entries, check
for missing cross-references and confirm that the links target existing headings.

## Developer-facing documentation

**Clarity without loss of meaning.** Write all developer-facing documentation,
including FOUNDATIONS.md, for developers who are new to Hydragen and have not read
the conversations behind it.

Use clear, simple language and concrete examples. Avoid unnecessary technical
jargon; when a technical term is needed, explain it on first use. Include enough
context to understand the problem, the proposed benefit, and the practical
consequences without relying on prior discussion or external links for essential
meaning.

Simplify wording and structure without dropping requirements, important details,
examples, or distinctions. Keep writing concise by removing repetition and vague
phrasing, rather than removing information or leaving assumptions unexplained.

Keep each section focused on what its heading promises. Lead with the information
readers are looking for, so they can quickly find the answer. Place origin stories
and background where they explain the motivation, rather than make readers infer
who the product is for or what it does.

**Required review before completing any documentation change:** Read the revised
text from a first-time reader's perspective and ask: **Can they understand what
each statement means and why it matters?** If the answer depends on prior
conversation, unexplained terms, or unstated assumptions, revise the text before
considering the change complete. Preserve its meaning and essential detail while
making it understandable.

### README scope

Keep README.md a concise entry point for human developers, not an exhaustive
technical reference or a running record of changes. Focus on what Hydragen does,
its current limitations, getting started, everyday usage, and essential safety
and automation guidance. Prefer a few useful examples over repeated explanations
of the same behaviour.

Describe user-facing choices, defaults, and rules at the level needed to use the
tool. Do not turn each option or supported feature into its own technical
tutorial. Leave implementation details, exhaustive edge cases, internal
validation rules, detailed test coverage, and historical corrections out of the
README; link to the relevant specification or focused reference when readers
need that detail.
Agent-specific technical guidance belongs in agent-facing documentation or
interfaces, not in the human introduction.

When behaviour changes, revise the existing explanation rather than appending
another implementation summary. Before adding detail, ask whether a developer
needs it to understand, try, or safely use the current tool. Preserve important
requirements in their appropriate reference rather than duplicating them all in
the README.

## Maintaining the foundations

Keep FOUNDATIONS.md focused on the product: what Hydragen is, the problem it
addresses, its guiding values, comparisons with existing approaches, and meaningful
scenarios. Keep development workflow and document-maintenance instructions here in
AGENTS.md. Keep unresolved planning questions in openspec/design-questions.md.

When updating the foundations, integrate new insights into the document as a whole.
Reconcile overlaps and remove duplication rather than appending sections in the
order of conversation. Preserve agreed meaning and essential examples.

Maintain deliberate, numbered guiding values with clear, stable names. Refine
their wording thoughtfully and avoid adding redundant values for each new insight.

Capture examples supplied by the user in the scenarios section. Extend or merge
an existing scenario when an example illustrates the same underlying point; add a
distinct scenario when it contributes a meaningfully different use case or
constraint. Preserve concrete details that explain why Hydragen is useful.

Keep comparisons balanced and grounded in primary sources. Explain when an
existing approach is useful as well as its tradeoffs against Hydragen's needs.
Distinguish limitations of a particular template or workflow from limitations of
the tool itself, and distinguish Hydragen's intended goals from proven capabilities.

Keep comparisons concise and scannable, using consistent fields for the good use
case, core tradeoff, and Hydragen's goal. Lead with concrete consequences for a
developer; avoid repeating abstract goals or cataloguing every feature. Keep
supporting detail in source links and meaningful scenarios.

Record time-sensitive maintenance evidence as dated observations with primary-source
links. Distinguish commit and release inactivity from confirmed archival or an
explicit end of maintenance; do not infer abandonment from commit age alone. Treat
these observations as historical snapshots rather than rolling status claims.
Keep each tool's maintenance observations alongside its comparison, rather than in
a separate maintenance section.
Periodic automated refreshes are planned in the pending OpenSpec change
`automate-maintenance-snapshots`. Until that workflow is implemented, recheck when
a concrete adoption decision or new evidence warrants it. Once implemented, keep
scheduled updates limited to source-backed observations; qualitative judgements
and changes to the core philosophy remain deliberately authored.

## Development workflow

Start small and focused, then grow incrementally as each part becomes understood
and comfortable to use. Explore enough to support the next useful increment;
avoid researching, designing, and implementing everything upfront.

Keep each iteration and addition of code focused and reviewable. Balance meaningful
progress with manageable scope: excessively small increments can slow progress and
create review and testing fatigue, while oversized increments can become
overwhelming.

## Schemas and implementation assets

Source code, tests, and build scripts must never load or embed files from
`openspec/`. Required implementation assets, including schemas and test fixtures,
must live in maintained project locations outside it. Documentation may still link
to OpenSpec specifications and planning decisions.

Use `schemas/layer-manifest.schema.json` to determine layer manifest content validity
for every parsed format (YAML/YML, TOML, JSON, and JSONC), flags, guided answers,
directory-name defaults, and complete candidates before writing. Field validation
must use definitions from that same schema, not duplicate constraints in ad hoc
predicates or separate field schemas. This includes layer names, dependency
references, and strict exact SemVer versions: do not add a second acceptance
grammar or SemVer parser policy. Classify validated references through the shared
schema-backed API. Keep presentation and input interpretation
(such as treating an empty optional prompt answer as omission), format parsing,
native path interpretation, filesystem resolution, cross-layer relationship
checks, repository-location checks, and safe file creation/replacement separate
from content validity.

Apply the shared lowercase name policy to every loader, init flag, guided answer,
and directory default without trimming, lowercasing, or sanitising. Directory
names and path text are not layer names. An invalid init directory-name default
must fail before prompts or writes with `--name` override guidance; a valid
explicit name bypasses default derivation. Preserve arbitrary accepted non-blank
author and description strings unchanged.

Archived schemas are historical snapshots: do not synchronize them with maintained
assets, require equality in tests, or use them as build or test inputs.

### Layer manifest field order

Guide generated layouts and maintained general-usage examples with the preferred
order `$schema`, `name`, `author`, `description`, `dependencies`, skipping absent
fields. Keep optional fields optional: init emits only name and supplied author
and description, in that order; it does not generate `$schema` or dependencies.
Accept any otherwise valid field order without ordering warnings or schema order
checks. Additional fields have no prescribed placement. Preserve examples and
test fixtures intentionally demonstrating alternative layouts, and leave archived
artifacts unchanged.

Add must not normalise existing layouts. Extend an existing dependencies list in
place; introduce a missing field after all existing root entries without moving
them. In TOML, keep the new root assignment before explicit table sections and
their leading comments. In block YAML, insert after the complete final root value
and its attached comments, before root footer comments and document-end markers.
Preserve existing field and reference order, comments, and unrelated source text;
retain the insertion-only verification and safe-write rules below.

## CLI development

Follow the [DX value](FOUNDATIONS.md#guiding-values): beauty, enjoyment,
functionality, and ease of use are equally important CLI design goals. Review
help, prompts, results, and errors together, including plain output. Consider
whether help is easy to scan, choices and defaults are clear, feedback is
consistent, and errors explain what happened and how to proceed. Assess visual
appeal and enjoyment explicitly; functional correctness alone does not complete
the review. No particular visual style or extra decoration is required. Preserve
the interaction and safety rules below and the foundations' performance value;
surface tradeoffs rather than silently demoting a goal.

Use clap for argument parsing and help, and cliclack for suitable guided prompts
and interactive feedback. Not every command needs prompts or decorative output.
Choose guided interaction, full branding, and status styling separately. Full
branded openings establish multi-step guided or long-running foreground sessions
once; one-shot actions use compact feedback. Keep terminal presentation outside
the layer manifest and filesystem logic, and use supported cliclack theme hooks
rather than replace its interaction handling.

Apply the foundations' responsiveness principle when preparing later CLI steps.
Start safe, useful read-only preparation during earlier interaction when its
inputs are already known, after interaction selection and required safety checks.
Do not postpone the first useful prompt to await optional preparation. Separate
the background work's finite lifetime/resource limits from a short user-visible
wait budget that starts only when its result is needed. For a guided wait that
cannot be avoided, use suitable cliclack progress feedback on standard error and
preserve cancellation; show no loading indicator behind another active prompt or
for results already ready. Keep one owner of terminal rendering and input.
Stop unneeded work and clean up owned child processes on cancellation, timeout,
or leaving the dependent step. Freeze displayed choices before interaction rather
than reordering them as late results arrive, and never use preparation to bypass
confirmation, automation policy, or permission boundaries. Test time spent visibly
waiting, early prompt availability, input responsiveness, progress feedback, late
results, and cleanup, not just total command duration. This is a design principle,
not a requirement to add background work to every command or a generic framework
before a concrete need exists.

When a command offers guided input and input flags, keep those routes separate.
With no input flags, allow a guided flow when interaction is available; any input
flag selects flag-only execution without questions to complete it. For init, the input
flags are `--name`, `--author`, `--description`, and `--format`. Explicit format
alone selects flag-only execution; the inferred YAML default does not count as an
input flag. Global controls such as `--non-interactive` are not input flags.
Preserve flags for input overrides, apply documented defaults without prompting,
and reject missing input that has no default. Init defaults omitted name to the
current directory's name. Its guided name prompt shows that default and accepts
it on Enter; author discovery must not replace this default with a project or
account name. Flag-only execution performs no author discovery: do not read
project authors, look up local identity, probe
`gh` or its authentication, or make GitHub requests. Preserve and validate an
explicit `--author` unchanged; omit unsupplied author and description instead of
prompting for them. Omitted format defaults to `yaml` (`layer.yaml`). Guided init
asks Name, Author, Description, then Layer manifest format, with choices labelled
by extensions without a leading dot in priority order: `yaml`, `yml`, `toml`,
`json`, `jsonc`.
The selector starts on `yaml`, accepted by Enter. Each choice still creates its
fixed `layer.<extension>` filename. Format is separate from
layer manifest metadata and does not override read-time priority.

Guided init offers directly declared authors from the init directory's
`package.json`, `Cargo.toml`, and `pyproject.toml` before Git, GitHub profile, and
local-account names, with GitHub organisation memberships last. Collect all usable
names rather than stopping at the first source, and group names by exact equality
after trimming surrounding whitespace, retaining all distinct source hints. Read
project declarations as data only: do not execute project tools, follow symlinks,
search other directories, or resolve inherited or dynamic authors. Optional
read-only GitHub requests use only `gh`'s existing authentication for the selected
host/account; never initiate login, request permissions, or change accounts.
Missing tools, credentials, connectivity, permissions, or usable source data are
silent, non-fatal discovery failures; retain other names and support offline use.
Keep credentials and source/membership records out of the layer manifest. Use the
maintained schema's author definition for discovered names; internal schema
failures remain errors, not an empty discovery result. The
[author-suggestion design](openspec/changes/suggest-init-author/design.md) defines
the supported fields, complete source order, host selection, and resource limits.

When names exist, use the native single-choice `Author (optional)` selector with
grouped source hints, then `Enter another author` and `Skip`. Initially select the
first project-declared or developer-identity name, including a project-declared
organisation. A name supported only by GitHub membership is not evidence of
authorship and must not be initially selected; if only such names exist, start on
`Enter another author`. Accepting a name proceeds directly to Description; `Skip`
omits author and does the same without another prompt. With no names, bypass the
selector and use the same empty optional Author text-input function as
`Enter another author`. Empty submission omits author and proceeds to Description,
without restoring a suggestion or returning to the selector. Keep action values
separate from name strings, use native scrolling for long lists, and show the
accepted name or `Skipped`, never source hints or action labels as metadata.

Start author preparation once, only after selecting guided input and completing
target safety checks, schema readiness, and directory-name default validation.
Do not wait for it before displaying Name or restart it on input retries. Bound
preparation to 15 seconds from its start, separately from at most 1,500 ms of
additional waiting once Author is reached; completion or either deadline ends
waiting. Keep local and GitHub work independent and bounded. Ready or expired
results need no indicator or artificial delay. During actual Author waiting only,
use the native cliclack spinner on standard error with
`Finding author suggestions…`, keeping Escape/Ctrl-C cancellation available
through foreground input handling. Clear it before any next prompt or
cancellation/error report, without a synthetic lookup-success line or timeout
warning. Freeze the collected
choices before opening Author, ignore late results, and stop unneeded work with
owned child-process cleanup on timeout, cancellation, failure, or session exit.
Workers must not read terminal input, render output, or modify project/identity
files; cleanup must not introduce an unbounded foreground wait.

Use the shared interaction policy: `--non-interactive`, CI mode, or a non-terminal
standard stream disables guided input. Do not reopen a terminal or read piped
answers to work around that choice. `--non-interactive` controls questions, not
colour. Allow compact styled flag-only statuses only when all standard streams
are terminals, CI is disabled, and `TERM` is not `dumb`. A non-empty `NO_COLOR`
or `CLICOLOR=0` disables decorative status styling; explicit disables win over
forced colour. CI or any redirected standard stream keeps status output plain
even when colour is forced. Explicit help and version remain plain successful
output on standard output. Route root and subcommand parser failures, including
future commands, through the shared parser-error presentation using these same
styling controls. Preserve clap's usage-error exit status and standard-error stream,
useful argument details, choices, suggestions, and usage/help guidance, without
branded openings or guided-session frames.

Follow the [CLI diagnostic reference](docs/cli-diagnostics.md) for all current and
future errors, not only parser failures. Use shared compact composition from typed
facts, or the supported guided variant; do not interpret English error strings or
add per-command body templates. When adding an error, check its concrete problem,
affected source and base, useful cause/recovery, known write state, escaped display
data, representative family test and human wording review against the reference's
[new-error checklist](docs/cli-diagnostics.md#new-error-checklist).
Single failures are direct; multiple dependency failures use ordered source labels
and a failure count, without redundant numbering. Prefer original reference
spelling with base context when needed, retaining actionable file, field, conflict
and staging/cleanup paths. For a missing current layer, retain the directory/parent
search, recovery and accurate no-change result without the invocation path.
Review families together in captured and bounded actual-terminal output; shared
layout and passing snapshots alone do not establish clear, useful wording.

Use green `✓` success, blue `ℹ` info for neutral outcomes, yellow `▲ Warning:`
headlines, and red `✕ Error:` headlines where styled. Info has no automatic `Info:`
prefix and does not imply failure or change an operation's stream or exit status.
Keep supporting details and recovery guidance in the normal foreground.
Rejected guided input uses a red cross, frame, and correction text,
not a recoloured warning triangle. Cancellation uses red cancellation feedback.
Plain status output retains the meaningful wording without ANSI or status icons.

After init writes successfully, report once: `Layer initialised, layer manifest
created at: layer.<extension>`, using the actual created filename, not its absolute
directory. Guided completion joins the session on standard error with no separate
stdout result. Flag-only completion uses standard output; warnings and errors
always use standard error. Preserve full diagnostic paths where needed for errors.

Adapt schema-derived validation feedback to the input route: guided feedback
names the field and explains correction without referring to flags, including
clear-and-Enter instructions for skipping an optional field. Flag-only errors
retain the relevant flag. Only a known non-blank schema failure gets non-blank
wording; preserve accurate diagnostics for other constraints. Internal schema
setup failures must stop before prompts or writes, not ask users to retry valid
input.

Share validation across input routes. Initially check repository location and all
five supported layer manifest paths before questions; any occupied entry,
including a directory or dangling symlink, refuses init without reading its
contents. Do not write until guided input, including format, is complete.
Cancellation must exit nonzero without creating files.
Give deliberate cancellation one no-files-changed report, without a duplicate
generic error. Do not suppress unexpected interrupted I/O or post-write failures
as though cancellation had already been reported.

The initial check does not reserve the directory. Keep exclusive creation
(`create_new`) for the exact chosen target, but do not repeat all-format refusal
at the final write or add a lock. A different supported filename appearing after
the initial check does not block creation. After a completed write, make a
best-effort discovery: warn on observed duplicates, naming every detected
filename and the priority selection, without rollback or changing success. A
failed observation is also a non-fatal warning. Actual write failures remain
errors and may leave a partial newly created file. A failure to report a completed
write must explain that the layer manifest was already created, not claim that no
files changed.

Use shared read-only discovery and loading for commands that depend on a layer
manifest, and report duplicates at most once per affected directory per invocation
on standard error, before a dependent failure. Handle help, version, and
argument-usage failures before operational dispatch, current-directory resolution,
layer manifest discovery or loading, schema preparation, prompts, or writes.
Test both captured-output execution and real terminal sessions, with bounded
waits and child-process cleanup.

### Local dependency addition

Keep `hydragen add <source> [<source> ...]` argument-only in every environment,
without prompts, piped answers, branded openings, or session frames. Accept global
`--non-interactive` before or after the subcommand without changing required
arguments or eligible compact styling. Add declares relationships only: no fetch,
copy, rendering, contribution capture, network access, Git mutation, or lockfile.

Use add-specific ancestor search around shared single-directory discovery: inspect
the invocation directory and parents through the filesystem root, crossing Git
boundaries, and stop at the nearest directory with an occupied supported layer
manifest filename. Keep the established priority and fail on an invalid selection
without lower-priority or ancestor fallback. This does not change init's guard or
exclusive-creation lifecycle. Refuse a symlinked or non-regular selected current
layer manifest; dependency layer manifests remain read-only and may be symlinks
to regular files.

Resolve explicit native relative and absolute arguments and stored local references
from the selected current layer manifest's directory, not the invocation directory.
Never infer a local path from a bare name or rewrite stored path text. Keep internal
resolved directory identity separate from the reference spelling. Diagnose
recognisable unsupported host path forms rather than translating them or trying a
registry. Do not expand literal home or environment-variable syntax.

Reuse structured read-only verification for the current layer, all existing direct
local dependencies, and each supplied target. A target must have its own valid
layer manifest: do not search its ancestors or traverse its dependency chain.
Existing published declarations receive offline syntax/name checks only, not an
existence claim. Reserve the current layer's name and require unique names across
local and published declarations. Reject existing duplicates, conflicting sources,
and self-name matches without replacement or aliasing. Harmless requests for the
same named resolved directory retain the existing reference; repeated new sources
retain only their first occurrence. Verify the whole batch before editing, collect
independent failures in source order, and append new references in argument order
without reordering existing entries. These reusable checks do not limit the broader
validate proposal's eventual scope or settle its traversal policy.

Retain loaded source for insertion-only, format-specific edits. Preserve comments,
their attachment to existing fields, unrelated source bytes, and decoded data;
do not use broad reserialisation as a fallback. Support ordinary arrays and
comments rather than blanket-refusing them. Reparse the prepared candidate with
the established decoder, validate it with the maintained schema, and prove that
only the intended dependency append changed decoded data. Refuse genuinely unsafe
constructs with a specific explanation before staging. All-repeat plans bypass
editing and staging entirely.

For actual updates, write a hidden sibling staging file, preserve original file
permissions, synchronise and close it, then atomically replace the selected file.
Do not truncate in place, use a non-atomic copy fallback, or repair permissions
after replacement. Attempt cleanup on handled failures and report cleanup failures
with the staging path. Keep replacement success explicit: a subsequent reporting
failure must say the layer manifest was already updated, not claim cancellation or
no files changed. Do not add locks, stale-content checks, or automatic merges;
document possible abrupt-termination leftovers and concurrent lost updates without
promising unconditional power-loss durability or preservation of all metadata.

Report one compact stdout result naming the destination layer, not its layer
manifest path by default. One effective requested dependency uses `Added <dependency>
to <destination>.` or `No changes needed: <dependency> is already a dependency of
<destination>.`, ending with one newline and no list or count. Choose from verified
outcomes after deduplication, not argument count or unrequested declarations.
All-repeat results use shared blue informational styling, including grouped
no-ops; actual additions and mixed results retain green-check success styling.
Single-result validated names use consistent bold emphasis in the status colour;
plain output has no decoration or extra `Info:` label. Multiple or mixed results
retain the destination headline and separate counted `Added` and `Already present` name
lists, omitting empty groups and unrequested dependencies. `Already present` means
present before invocation; repeated new sources belong only under `Added`.
All-repeat success reports no changes needed. Keep per-group first-appearance
order, shared plain/styled status rules, and stderr warnings/errors. Present
failures through the diagnostic reference's direct-single or ordered-source-group
structure, preserving actionable path context and deduplicated warnings before
dependent failures.

Published references require exact SemVer, including optional prerelease/build
identifiers, not ranges, tags, or implicit versions. Adding published dependencies,
publishing, registry design/access, and registry-backed guided search/version
selection remain deferred. Future published add must reject a different version
of an already-declared layer, never overwrite it. Design the separate explicit
upgrade/version-change workflow later; no command name or placeholder implementation
is selected. Retrieval, rendering, and broader validation also remain separate work.
