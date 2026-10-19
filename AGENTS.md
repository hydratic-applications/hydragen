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
predicates or separate field schemas. Keep presentation and input interpretation
(such as treating an empty optional prompt answer as omission), format parsing,
repository-location checks, and safe file creation separate from content validity.

Archived schemas are historical snapshots: do not synchronize them with maintained
assets, require equality in tests, or use them as build or test inputs.

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
even when colour is forced. Help, version, and clap usage errors remain plain.

Use green `✓` success, yellow `▲ Warning:` headlines, and red `✕ Error:` headlines
where styled; keep supporting details and recovery guidance in the normal
foreground. Rejected guided input uses a red cross, frame, and correction text,
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
on standard error, before a dependent failure. Do not scan layer manifests for
help, version, or argument-usage failures. Test both captured-output execution and
real terminal sessions, with bounded waits and child-process cleanup.
