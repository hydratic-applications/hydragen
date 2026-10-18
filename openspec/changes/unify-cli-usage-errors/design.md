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

Installed clap 4.6.7 exposes structured `ErrorKind`, context inspection/mutation,
`RichFormatter`, and plain rendering. Retain that formatter instead of maintaining
a second formatter for every error kind. Clap hardcodes the generated `error: `
prefix and has no public prefix hook. Removing only that fixed presentation label
before passing the body to Hydragen's shared headline renderer is an intentional
envelope adaptation, not classification by English prose. If the envelope is
unexpected, retain useful safe fallback information under one generic headline.

Build the command with plain clap styles and the trusted binary name `hydragen`.
Never derive diagnostic usage from untrusted argv[0]. Retain command metadata if
needed to identify add's required `sources` argument through structured missing-
argument context rather than English messages or a second argv scan.

Before rich rendering, escape every raw string/list context leaf and generated
single-line suggestion leaf containing copied raw argument text. Read a styled
leaf's encoded content before escaping so controls become visible rather than
disappearing through ANSI stripping. Generated usage/help layout is trusted only
because the binary name and command definitions are static and plain. If a future
custom value parser contributes an external error source, use an explicit safe
fallback for that source rather than letting rich rendering emit it unchecked.
Unknown/preformatted payloads must likewise remain safe without breaking known
generated help layout or silently losing diagnostics.

The trust boundary is explicit: raw argument/value strings and lists are escaped;
suggestion styled leaves retain their encoded content before escaping; only known
clap-generated Usage/static help preserves layout. Non-exhaustive unknown context
kinds/value variants must not pass unchecked to rich rendering. For an unknown
shape, external validation source, raw/preformatted error, or unexpected envelope,
compose a trusted generic headline with the opaque diagnostic's ANSI-preserving
content escaped as one data leaf, retaining separately trusted usage/help where
available. This exceptional opaque fallback is not normal whole-message escaping:
it prevents an unstructured payload from inventing trusted layout. Do not silently
drop its useful explanation or strip away malicious controls before escaping.

Preserve the original exit-code and stream decision before consuming the error.
Explicit DisplayHelp/DisplayVersion are stdout/success; root no-arguments
DisplayHelpOnMissingArgumentOrSubcommand remains stderr/status 2 with a clear
missing-command headline and its useful static help layout. Global-flag-only root
MissingSubcommand remains a usage failure as well. No extra CLI flags or commands
are added just to exercise synthetic parser error kinds.

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

### Record the policy without rewriting historical decisions

Update current agent guidance and link the user-approved decision from design
question 10. The pending `define-cli-visual-identity` change establishes the shared
style; this follow-up supersedes only the usage-error plain exception, preserving
its layout, guided-session, help/version, and stream rules. Do not edit archived
changes or unrelated active artifacts to make the history look uniform.

The existing foundations already explain the DX principle; review them at
completion but do not add a new guiding value or CLI implementation procedure.

## Risks / Trade-offs

- Parser richness can be lost in a replacement renderer → reuse clap context and
  formatting, test unknown options/commands, missing option values, invalid format
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
- Framework output may vary with upgrades → test the public structured adapter,
  use supported hooks, and keep fallback behaviour rather than matching prose.

## Migration Plan

Implement the shared route and tests together, update the explicit current-policy
exception, then review captured and real-terminal evidence before release. No
data migration is needed. Scripts still receive nonzero usage failures on stderr
without ANSI; exact English wording is improved, not promised as a parsing API.

## Open Questions

No product-level question remains. The installed API investigation above selects
plain structured-context sanitisation plus the existing rich formatter. Runtime
unit, captured, and PTY tests must prove the integration before completion.
