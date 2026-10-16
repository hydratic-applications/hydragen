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
code rather than introducing alternative names for the same concept.

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
Keep terminal presentation outside the manifest and filesystem logic.

When a command offers guided input and input flags, keep those routes separate.
With no input flags, allow a guided flow when interaction is available; any input
flag selects flag-only execution without questions to complete it. For init, the input
flags are `--name`, `--author`, and `--description`. Global controls such as
`--non-interactive` are not metadata. Preserve flags for input overrides, apply
documented defaults without prompting, and reject missing input that has no
default. Init defaults omitted name to the current directory's name; its guided
name prompt shows that default and accepts it on Enter. Omit unsupplied author
and description instead of prompting for them in flag-only execution.

Use the shared interaction policy: `--non-interactive`, CI mode, or a non-terminal
standard stream disables guided input. Do not reopen a terminal or read piped
answers to work around that choice. Keep flag-only output plain, result output on
standard output, and prompts and diagnostics on standard error.

Share validation across input routes, check known target errors before questions,
and do not write until guided input is complete. Cancellation must exit nonzero
without creating files. Preserve exclusive creation and other filesystem safety
checks even after a read-only check succeeds. Test both captured-output execution
and real terminal sessions, with bounded waits and child-process cleanup.
