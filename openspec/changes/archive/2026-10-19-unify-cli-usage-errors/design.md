## Context

The user approved a global presentation change after seeing bare `hydragen add`
print clap's default missing-arguments error. The current `Cli::parse()` exits
before shared presentation is selected. This is why parser failures look unlike
the operational failures handled by `Presentation::diagnostic`.

`AGENTS.md` deliberately keeps usage errors plain today. This change replaces that
exception, not the policy that parses arguments before accessing layer manifests.
It follows the foundations' DX value across errors/help and its performance value
by avoiding project access for commands that cannot run. Design-question 10's CLI
experience discussion is relevant; no registry, traversal, or source decisions
need reopening.

## Goals / Non-Goals

**Goals:**

- One shared parser-error route across root and subcommands, including future
  parser failures, with the same terminal/plain presentation as other errors.
- Short, actionable missing-source guidance and preserved clap usage, hints,
  allowed values, argument names, and spelling suggestions where available.
- Successful plain help/version independent of project state; no prompts,
  discovery, schema setup, or writes on any parsing/help/version outcome.
- Preserve error streams/exit status and display-only control-character escaping.

**Non-Goals:**

- Changing accepted arguments, adding prompts, changing discovery priority, or
  checking for a layer manifest when syntax is invalid.
- A new visual identity, branded errors, styled help/version, or colour flags.
- Replacing clap, changing runtime error meanings, or modifying layer content.

## Decisions

### Classify parsing outcomes before operational work

Replace the automatic printing/exiting parse entry point with clap's fallible
parsing API. Route supported explicit help/version outcomes through clap's normal
plain output with their successful status. Route actual parser failures through
one binary-local adapter and return clap's usage-error status, ordinarily 2.
Do not classify success versus failure by matching rendered message strings.

Presentation detection is terminal/environment inspection, not layer discovery;
it can occur before parsing so usage failures share the same style decision.
No operational dispatch, current-layer check, validator setup, or prompt starts
until parsing succeeds. A help-on-missing-input outcome remains a usage failure
if clap classifies it that way; it must not accidentally become explicit help.

### Preserve clap's command knowledge and diagnostic detail

Use supported structured error/context and formatter APIs. Keep parsing and
suggestions in clap, and adapt only the diagnostic presentation. Missing add
sources receive a direct explanation that at least one source is required, a
usage line, one valid local-path example, and a hint for add help. Other errors
retain their useful context rather than becoming a generic failure.

Do not build a second argument parser, scan argv to rediscover subcommands, or
infer error meanings from the English message.

Installed clap 4.6.7 exposes structured `ErrorKind` and diagnostic context. Keep
clap as the sole argument parser and source of choices/suggestions, but adapt its
structured facts into one presentation model before rendering. Retaining its
fully rendered bodies was the initial approach; the user observed that this still
mixed bracketed choices/lowercase tips and custom layouts. The approved refinement
therefore gives Hydragen ownership of body organization as well as the headline.
Do not attempt to obtain consistency by patching English rich-formatted strings.

Build the command with plain clap styles and the trusted binary name `hydragen`.
Never derive diagnostic usage from untrusted argv[0]. Retain command metadata if
needed to identify add's required `sources` argument through structured missing-
argument context rather than English messages or a second argv scan.

Escape every raw context leaf and single-line suggestion containing copied raw
argument text before placing it in the presentation model. Read a styled leaf's
encoded content before escaping so controls become visible rather than disappear
through ANSI stripping. Only usage/help generated from static plain command
metadata is trusted layout. Unknown/preformatted payloads and external parser
sources remain opaque data in a safe fallback using the same outer organization.

The trust boundary is explicit: raw argument/value strings and lists are escaped;
suggestion styled leaves retain their encoded content before escaping; only known
clap-generated usage/static help preserves layout. Non-exhaustive unknown context
kinds/value variants must not pass through unchecked. For an unknown shape,
external validation source, or raw/preformatted error, use a trusted generic
headline with the opaque diagnostic's ANSI-preserving content escaped as one data
leaf, retaining separately trusted usage/help. This exceptional opaque treatment
prevents unstructured data from inventing layout. Retain useful explanation but
do not expose internal ContextKind/debug field names as the normal user interface.

Preserve the original exit-code and stream decision before consuming the error.
Explicit DisplayHelp/DisplayVersion are stdout/success; root no-arguments
DisplayHelpOnMissingArgumentOrSubcommand remains stderr/status 2 with a clear
missing-command headline and useful command choices in the shared body, not a
separate full-help body. Global-flag-only root
MissingSubcommand remains a usage failure as well. No extra CLI flags or commands
are added just to exercise synthetic parser error kinds.

### Render one familiar optional-block grammar

Parser diagnostics share this order:

1. One sentence-case, consistently punctuated error headline.
2. Optional context/correction, with labels such as `Possible values:` and `Tip:`.
3. Relevant `Usage:` and at most one useful `Example:`, grouped together.
4. A final command-specific `For more information, try 'hydragen ... --help'.`

Supporting lines use a two-space base indent. Blocks have one blank line between
them; lines within a block have no extra blank lines. Choices are not wrapped in
square brackets, tips have consistent capitalization, and no empty placeholders
appear when a block is omitted. Only the headline is red; plain output changes
decoration, not structure. Explicit help retains its separate documentation layout.

Examples are trusted optional command content, not independent renderers or
automatically executed corrections. Missing add source retains its existing
example; invalid init format can show `hydragen init --format yaml`. Other errors
need not acquire filler examples. Keep argument spelling/values accurate while
normalizing generated prose and labels, not user data.

The common visual rules also guide operational diagnostics, but this increment
preserves dependency failure groups except for the missing-current-layer
prerequisite described below. Write-state messages and cancellation flows remain
unchanged. Do not force usage/example blocks onto failures where they are irrelevant.

The structured adapter must retain the exact erroring command context even when
clap omits a Usage context (as for invalid format values). Confirm a supported
metadata-only technique before application edits; do not infer command paths from
English prose, scan argv by hand, change argument validity, or run operations while
recovering context. Ambiguous future context must fail safely rather than invent
a command-specific repair.

The installed clap API has no universal public error-command field. Build a small
catalog from the static built command graph: canonical paths, generated usage/help,
argument identities/display strings, global classification, and optional trusted
examples. Construct it only on the parser-failure adaptation path, not successful
operational parsing or explicit help/version. Generate help documents lazily only
when pseudo-help identification needs them; never replay parsing or call value
validators while building/consulting the catalog. For current value errors without Usage, resolve an exact InvalidArg
display-string match only when one non-global argument owns it. Restrict this to
appropriate typed error kinds; a suggestion for another command is not provenance.
For Usage-bearing errors, map only clap's fixed generated `Usage: ` envelope to
complete canonical command-path boundaries, longest match first. This narrow
framework-envelope mapping is permitted, like the previous fixed-label adaptation;
it is not English error-prose or argv parsing. Document that limitation honestly.

Prefer canonical usage newly generated from the resolved command catalog for the
shared block, so errors in one command have a predictable synopsis rather than
varying smart-usage tails. Preserve the contextual argument/count/conflict facts
elsewhere in the model. Only trusted static command definitions supply layout.
If ownership is ambiguous or the graph/usage layout is unsupported, retain an
explicit Unknown result, suppress unverified scoped examples, and offer general
`hydragen --help` without claiming the error originated at root. Add invariant
tests for current command graph, reused/global arguments, and unsupported layouts.

Do not replay with `ignore_errors(true)`: source/API probes showed that clap can
finish parsing a child before a parent value/count/conflict fails, so deepest
partial matches do not identify the error origin. Replaying also reruns successful
custom validators. A future typed-parser wrapper can enrich errors locally when
an actual ambiguity requires it, but is unnecessary for the current unique init
arguments and cannot solve structural or missing-value errors universally.

Record the grammar in a short maintained diagnostic reference linked from agent
guidance. Test errors together as a family: common section ordering, indentation,
labels, punctuation, safe fields, and plain/styled equivalence, plus representative
captured and bounded real-terminal cases. Individual snapshots alone must not
bless two internally consistent but different templates.

### Apply existing visual and security rules at shared boundaries

Use the existing red `✕ Error:` headline only when all standard streams are
terminals, CI is disabled, TERM is not dumb, and explicit colour disables are
absent. Forced colour cannot bypass these gates. Keep details in the normal
foreground, meaningful plain equivalents without status icons, and errors on
stderr. `--non-interactive` does not disable eligible error styling.

Keep help/version plain with no error headline or banner. Retain their current
width-aware formatting and output semantics. A usage error shows concise relevant
usage/recovery information, not an unsolicited full help dump for bare add.

Escape untrusted argument/context values for display before composing generated
layout, using the existing diagnostic-text policy. Never escape a whole composed
message (which would destroy its layout), emit untrusted terminal controls, or
rewrite values that clap validates. Unrecognised error kinds retain safe useful
fallback context; this cannot depend on a growing list of per-command renderers.

### Keep error priority independent of layer state

Bare add reports missing-source guidance, an unknown option reports that option,
and supported help/version work anywhere. A syntactically valid add invocation
then discovers the current layer and reports a missing layer manifest if absent.
This is a deliberate Hydragen policy, not a claim that every CLI uses the same
priority. Directory-dependent missing-input special cases were considered and
rejected because they obscure syntax errors and perform unrelated work.

### Explain a missing current layer without a batch wrapper

The user found the operational missing-layer diagnostic confusing after the parser
refinement: a generic counted headline and `1. the current layer` hid a simple
prerequisite failure. Its absolute invocation path added bulk without identifying
a particular file to repair. Present this failure directly:

```text
Error: No layer manifest found in this directory or its parents.

  Tip: Run this command from an existing layer directory, or run
  'hydragen init' in the directory you want to use as a layer.

  No files were changed.
```

Match the structured single `Current` / `MissingCurrent` failure in CLI diagnostic
wiring, not its English display text. Keep read-only ancestor discovery, its full
root traversal, and parser-first priority unchanged. Retain the invocation path
in the underlying error for other callers; only the CLI presentation omits it.
Do not show a count, source group, separate search paragraph, usage, or help filler.
Both recovery options matter: the developer may be in the wrong directory rather
than need a new layer. Use the existing shared plain/styled error presentation.

This is not a rule to strip paths or flatten every single dependency error. Keep
paths where they identify invalid, unreadable, or conflicting files and sources;
preserve other dependency groups, source order, warnings, and write-state safety.
No new product term or foundational principle is needed.

### Record the policy without rewriting historical decisions

Update current agent guidance and link the user-approved decision from design
question 10. The pending `define-cli-visual-identity` change establishes the shared
style; this follow-up supersedes only the usage-error plain exception, preserving
its layout, guided-session, help/version, and stream rules. Do not edit archived
changes or unrelated active artifacts to make the history look uniform.

The existing foundations already explain the DX principle; review them at
completion but do not add a new guiding value or CLI implementation procedure.

## Risks / Trade-offs

- Parser richness can be lost in a replacement renderer → retain clap context,
  test unknown options/commands, missing option values, invalid format
  choices, conflicts and fallback paths, not only missing add sources.
- Help is represented as a clap error type → distinguish its kind, stream, and
  status explicitly; test supported help/version and missing-input help separately.
- Untrusted argv could inject display controls → escape raw context values while
  retaining generated usage/suggestion layout and ordinary Unicode/backslashes.
- Plain/styled expectations change → deliberately migrate captured and PTY tests,
  retaining stdout/stderr, exit-code, no-FS, cancellation, and no-write assertions.
- No-warning tests alone cannot prove no discovery → combine hostile-layer
  captured/PTY cases with explicit source/call-order review that every parser,
  help, and version outcome returns before operational dispatch. Do not add a
  filesystem abstraction solely to instrument this presentation change.
- Framework context may vary with upgrades → test the structured adapter, use
  supported hooks, and keep safe fallback behaviour rather than matching prose.
- Full-command help/usage can be wrong if failed parsing loses context → confirm
  framework-based context recovery before implementation and test errors before
  and after command selection, global options, and values resembling commands.

## Migration Plan

Implement the shared route and tests together, update the explicit current-policy
exception, then review captured and real-terminal evidence before release. No
data migration is needed. Scripts still receive nonzero usage failures on stderr
without ANSI; exact English wording is improved, not promised as a parsing API.

## Open Questions

No product-level question remains. The approved body-layout refinement requires
confirming framework command-context recovery before implementing the shared
model. Runtime unit, family-layout, captured, and PTY tests must prove that useful
parser facts, safety, command context, and the approved grammar are retained.
