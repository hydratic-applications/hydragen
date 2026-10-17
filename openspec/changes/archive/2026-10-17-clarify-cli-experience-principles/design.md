## Context

`FOUNDATIONS.md` is the authoritative product document. Its eight guiding values
include **DX — Developer Experience**, which currently focuses on making reuse as
natural as ordinary editing. It does not explicitly address how the CLI looks
or whether it is enjoyable. The user has agreed that these concerns belong within
DX, with beauty, enjoyment, functionality, and ease of use as equal design goals.
The goal covers the whole CLI and does not prescribe a visual style.

The prototype implements only `hydragen init`. Clap supplies arguments, help, and
version output in `src/main.rs`; help currently disables colour explicitly.
`src/interaction.rs` selects guided versus flag-only execution and presents
cliclack prompts, defaults, optional-field hints, skip feedback, and completion.
`src/lib.rs` supplies metadata validation and filesystem behaviour independently
of presentation. Dependencies include clap, cliclack, and console; portable-pty
is used only for terminal tests. None needs changing for this increment.

`AGENTS.md` already requires plain flag-only output, separate guided and flag-only
input routes, conservative interaction detection, stream separation, shared
validation, cancellation safety, and exclusive creation. The main specifications
record these behaviours. `tests/init.rs` covers captured execution;
`tests/interactive_init.rs` covers real terminal streams with bounded waits and
child cleanup. Those tests are useful evidence, but passing behaviour tests does
not establish that an interface looks good or is enjoyable.

Relevant open questions are success criteria (question 10) and, as constraints,
agent interfaces (9) and performance targets (11). This change defines a narrow
CLI review approach; it does not choose an agent-specific interface, set timing
targets, or settle broader product success criteria.

## Goals / Non-Goals

**Goals:**

- Make all four CLI experience goals explicit and equally important within DX.
- Give contributors practical review guidance without selecting a visual theme.
- Preserve every existing product value and CLI interaction/safety contract.
- After the documentation update, audit implemented surfaces and produce a
  prioritised plan for bringing observed gaps into adherence.
- Separate verified behaviour, qualitative design judgement, and unknowns.

**Non-Goals:**

- Implement CLI fixes or change help, prompts, output, input routes, or defaults.
- Add a ninth value, a new UI framework, visual style guide, custom theme,
  dependency, command, output format, or performance threshold.
- Audit unimplemented dependency management, rendering, watch, or contribution
  capture workflows, or resolve other backlog items.
- Rewrite archived decisions, change layer manifest semantics, or claim compliance
  before observation and review.

## Decisions

### Extend DX rather than add a value

Add a cohesive paragraph to the existing DX value, preserving its name, number,
and existing requirements. Beauty and enjoyment belong to developer experience;
a separate value would split related concerns and unnecessarily change the stable
eight-value structure.

Proposed wording for implementation review:

> The CLI should be beautiful, enjoyable, functional, and easy to use, with each
> treated as an equally important design goal. This applies to help, prompts,
> results, and errors—not only guided flows. The experience should feel
> intentionally designed, including when plain output is appropriate, without
> requiring a particular visual style.

Exact prose can be refined for readability during implementation, but all these
commitments and the original DX meaning must remain. Keep library choices and
review procedures out of the foundations. No new glossary term or scenario is
needed: this adds an interface-quality goal, not a new layer-model concept or
user-supplied scenario.

### Translate the goal into review guidance, not a prescribed aesthetic

Add a short opening paragraph under CLI development in `AGENTS.md`, linking to
the DX value. Explain that contributors should review the whole experience for
visual appeal, enjoyment, functionality, and ease of use together. Useful review
questions include whether help is easy to scan, choices and defaults are clear,
feedback is consistent, errors explain what happened and how to proceed, and
presentation feels coherent. These are prompts for judgement, not a compulsory
palette, layout, tone, or demand for extra decoration.

Retain clap/cliclack guidance and all current interaction rules below it. Plain
automation output is deliberately designed, not an exemption from quality.
Equal importance does not authorise breaking safety or automation to improve
appearance; nor does functional correctness establish that visual quality is
complete. Surface genuine tradeoffs rather than silently demoting a goal.

Alternatives rejected: a mandatory expressive or restrained style (not agreed),
beauty as optional finishing polish (contradicts equal importance), or decorating
every command (conflicts with the existing interaction contract).

### Record only the relevant success-criteria decision

During implementation, append a brief partial resolution under question 10 in
`openspec/design-questions.md`, linking to this design's
`#audit-after-establishing-the-principle` section. Record the four CLI review
dimensions and evidence-based audit approach. Leave the question's existing
assessment topics and broader project-selection questions intact. No changes to
questions 9 or 11 are needed because their unresolved choices remain unresolved.

### Keep the principle contract separate from runtime specifications

Add `cli-experience-principles` as a documentation and review capability. Do not
modify `cli-interaction` merely to attach an aesthetic goal to current prompt
behaviour. Its concrete requirements remain audit constraints. Leave archived
OpenSpec records unchanged: the earlier decision not to extend the foundations
was appropriate to that earlier increment and does not prevent this new decision.

### Audit after establishing the principle

Complete and review the documentation update before auditing. This ordering makes
the written goals the review baseline instead of adjusting them to fit existing
behaviour. Re-read current sources when auditing; planning observations are not
findings, and the CLI may change before implementation.

Create `audit.md` alongside this design. Record the date, Git revision and any
relevant working-tree changes, OS, Rust/tool versions, terminal dimensions, and
colour/environment settings. Use disposable directories, never initialise the
project checkout, and retain commands, inputs, stdout/stderr, and exit statuses.
Store concise transcripts or terminal captures beside the report if needed;
avoid committing binaries or large temporary logs.

Cover the following implemented surfaces:

- Root help, init help, version, missing subcommand, unknown option, and a flag
  missing its value: discoverability, wording, examples, and scanning.
- Guided init: displayed name default and override, optional skips, validation
  retry, completion, and Escape/Ctrl-C cancellation, including after partial input.
- Flag-only init: name-only and optional-only flags, directory-name default,
  explicitly invalid metadata, success feedback, and lack of follow-up prompts.
- Automation: explicit non-interactive mode, CI with a terminal, and each standard
  stream redirected in turn; check prompt suppression and output channels.
- Errors: existing layer manifest, incorrect repository location, unusable directory-name
  default, and post-write reporting failure. Use existing tests for difficult
  failure paths rather than unsafe manual filesystem or process manipulation.
- Presentation conditions: ordinary and narrow terminal widths, supported colour
  and disabled colour, and long or Unicode names/paths. Record readability,
  wrapping, consistency, and whether essential meaning survives without styling.

Start from `src/main.rs`, `src/interaction.rs`, `src/lib.rs`, README, current
specifications, and the existing two integration suites. Run the existing locked
test suite and use its results as behavioural evidence. Supplement it with actual
terminal observation; stripped text cannot establish spacing, emphasis, or redraw
quality. Any scripted interaction must retain bounded waits and child cleanup.
No new permanent test harness is required by this change.

For each reviewed surface, assess all four goals explicitly. Use **meets**,
**gap**, **needs design review**, or **not verified**, with a concrete rationale
and evidence reference. Identify confirmed contract violations separately from
qualitative improvement opportunities. Do not invent a numeric beauty score or
claim user-tested enjoyment without user research. Record inaccessible
environments and judgement calls rather than filling them with assumptions.

### Plan remediation without implementing it

Create `remediation-plan.md` after the audit. Each actionable item must link to
the finding and affected goal or existing requirement, describe a proposed
direction, identify likely files and dependencies, and define acceptance checks.
Distinguish fixes necessary for adherence from optional refinements. Prioritise
by user impact and risk, not by ranking the four goals; explain priorities.

Group related items into focused, reviewable increments, identify any behaviour
or specification changes that need a separate OpenSpec proposal, and flag visual
directions needing user approval. Include satisfied areas that should be preserved
and unresolved findings needing additional evidence. If there are no confirmed
gaps, say so and retain any unverified or subjective review items; do not invent
work. Stop at the plan for user review. Approved fixes belong to later changes.

## Risks / Trade-offs

- **Subjective goals become vague approval gates.** → Pair every judgement with a
  concrete observation, keep all four dimensions visible, and flag design choices
  for user review instead of pretending tests prove enjoyment.
- **New prose weakens existing DX meaning or automation rules.** → Compare the
  complete revised sections against the originals and current specifications;
  retain all constraints and the eight-value structure.
- **A small principle update grows into a redesign.** → Limit implementation to
  documentation, the audit, and remediation planning; no runtime fixes in scope.
- **Audit results depend on environment or stale code.** → Record revision and
  conditions, distinguish observed from test-derived evidence, and disclose limits.
- **The other pending change edits the same documents.** → Keep edits local to DX,
  CLI development, and success criteria; do not touch maintenance comparisons or
  the maintenance-automation guidance.

## Migration Plan

1. Update and review the two principle/guidance documents and the narrow question
   resolution, without changing runtime behaviour.
2. Perform the audit against that baseline and create the remediation plan.
3. Review the reports and validate the OpenSpec change before completion. Archive
   only through the normal later workflow; keep remediation implementation separate.

There is no data or runtime migration. If wording needs correction, revise only
this change's documentation edits, preserving unrelated work. Missing audit
evidence remains explicitly open; documentation completion alone does not mean
the audit or remediation planning is complete.

## Open Questions

No further product choice blocks the documentation update. The audit will reveal
whether fixes are needed, what visual decisions require review, and which
environments remain unverified. Those findings are deliberately not guessed in
this plan. Broader audience, agent-interface, and performance-target questions
remain outside this change.
