# CLI diagnostics

This reference defines how Hydragen presents command-line errors. A familiar
structure helps developers find the problem and the next useful action, even when
different errors need different information. Parser failures reject command
syntax before an operation runs. Operational failures occur after syntax is
accepted, such as a layer manifest write failing. Root and subcommand parser
failures use the shared body below; operational failures share the visual
principles while retaining useful source grouping and safety information.

## Shared body structure

Render parser failures in this order:

1. **Problem:** One sentence-case, punctuated `Error:` headline explaining what
   prevented the command from running.
2. **Context and correction, when useful:** Argument details, choices, or a
   suggestion that helps resolve the problem. Use consistent labels, including
   `Possible values:` and `Tip:`. Do not enclose a choice list in square brackets.
3. **Usage and optional example:** Relevant `Usage:` followed immediately by at
   most one useful `Example:`. Keep these lines in the same block. An example
   should demonstrate a valid next command, not fill space or execute a correction.
4. **Help:** A final `For more information, try 'hydragen ... --help'.` hint for the
   verified erroring command.

Use a two-space base indent for supporting lines and exactly one blank line
between blocks. Lines within a block are consecutive. Keep labels capitalized as
shown and generated summaries consistently punctuated. Preserve the spelling and
case of quoted arguments and values; sentence case is not permission to change
user data.

Optional blocks do not require every error to contain identical information.
Omit unhelpful context and filler examples without leaving empty labels or extra
blank lines. Root missing-command errors use command choices within this same
structure, not a separate full-help body.

## Examples

These illustrative terminal transcripts show the approved structure. In an
eligible terminal, only the `✕ Error:` headline is red; the supporting text stays
in the normal foreground. The code blocks show the symbol without embedding
terminal colour codes (ANSI sequences). Plain output omits the `✕` and colour
codes, retaining `Error:` and the same wording, indentation, block order, and
spacing. These are the existing presentation rules, not a separate colour policy
for usage errors.

### Missing add source

For `hydragen add`, the problem is clear without a context block:

```text
✕ Error: Add requires at least one source.

  Usage: hydragen add [OPTIONS] <SOURCE>...
  Example: hydragen add ../shared-config

  For more information, try 'hydragen add --help'.
```

### Invalid init format

For `hydragen init --format yam`, choices and a suggestion help correct the value:

```text
✕ Error: Invalid value 'yam' for '--format <FORMAT>'.

  Possible values: yaml, yml, toml, json, jsonc
  Tip: Did you mean 'yaml'?

  Usage: hydragen init [OPTIONS]
  Example: hydragen init --format yaml

  For more information, try 'hydragen init --help'.
```

## Operational prerequisites and paths

Lead with the concrete prerequisite that prevents an operation from running.
For add's missing current layer, use `No layer manifest found in this directory
or its parents.`, then the tip `Run this command from an existing layer directory,
or run 'hydragen init' in the directory you want to use as a layer.` and the
no-files-changed result in separate blocks. Split the tip before `'hydragen init'`
so the command and its explanation stay together. Do not wrap this failure in a
generic counted headline or numbered current-layer group.

Omit the absolute invocation path and a separate search paragraph here: the
headline explains the search scope without identifying a particular file to fix.
Keep paths when they identify invalid, unreadable, or conflicting files or sources.
This exception does not remove other dependency failure groups or their details.

## Command context and shared rendering

Keep clap responsible for argument grammar, validation, choices, and suggestions.
Adapt its structured diagnostic facts into one presentation model: a record of
the summary, details, usage, optional example, and help target. Render that model
once. Do not identify errors by matching English messages, patch rendered
English with regular expressions, scan arguments with a second parser, or create
per-command body templates. Optional trusted examples are command content, not a
reason for command-specific layout code.

Use the verified erroring command's complete path for usage and help:
`hydragen --help`, `hydragen init --help`, or `hydragen add --help`. Current root,
init, and add failures must retain that context, including value failures for
which clap does not supply usage. Generate canonical usage from trusted command
definitions rather than guessed argument text. A narrow mapping of clap's fixed
generated `Usage: ` envelope to complete known command paths is allowed; this does
not permit interpreting error prose or replaying argument parsing.

If a future command layout or diagnostic cannot identify which command failed,
keep the erroring command unknown. Retain safe useful details and offer general
`hydragen --help` without claiming the error originated at root. Omit unverified
command-specific usage or examples rather than inventing a repair. Generic
fallback errors use the same outer structure when diagnostic facts cannot be
safely interpreted; framework debug field names are not the ordinary interface.

## Presentation and safety boundaries

- Use the shared red `✕ Error:` headline only when all three standard streams are
  terminals, CI is disabled, `TERM` is not `dumb`, and no explicit colour disable
  applies. A nonempty `NO_COLOR` or `CLICOLOR=0` disables styling. Forced colour
  cannot override these gates; `--non-interactive` controls questions, not eligible
  styling. Do not add branding or guided-session frames to parser failures.
- Preserve clap's usage-error status, ordinarily 2, and standard-error output.
  Supported explicit help and version remain plain successful standard output,
  with status 0 and no error headline. Help keeps its separate, width-aware
  documentation layout rather than adopting the error-body structure.
- Handle rejected invocations, help, and version before operational dispatch,
  current-directory resolution, layer manifest discovery or loading, schema
  preparation, prompts, or writes. Do not inspect project state to decide which
  syntax error to report. Add remains argument-only in every environment.
- Escape untrusted argument, value, and suggestion text for display before
  composing layout. Controls must become visible text, not terminal actions or
  invented diagnostic lines. Keep trusted generated layout separate from data;
  preserve useful opaque fallback explanations safely. Display changes must not
  normalize accepted arguments, alter paths or stored data, or rerun validators.
- Operational failures keep dependency failure groups, actionable paths,
  cancellation behaviour, and accurate write-state information. Never imply that
  no files changed after a completed write or replacement. Do not add irrelevant
  `Usage:` or `Example:` blocks to filesystem errors just to fill this structure.

## Tests and experience review

Review diagnostics as a family, not only as isolated snapshots. Cover missing
arguments, invalid values, unknown options and commands, conflicts, and generic
fallbacks across root and subcommands. Assert the common section order,
sentence-case punctuated summaries, two-space base indent, single block gaps,
consistent labels, consecutive usage/example lines, optional-block omission, and
correct command-specific help. Different error facts must not conceal different
body templates.

Compare styled and plain output for equal structure and meaning, with colour only
on the eligible headline. Check CI, every redirected-stream combination, dumb
terminals, explicit colour disables against force, and `--non-interactive`.
Include hostile diagnostic values and copied suggestions to prove controls cannot
escape the display boundary, ordinary Unicode and literal backslashes survive,
and accepted argument values stay unchanged. Exercise known command ownership and
ambiguous fallback separately; a token resembling a command must not redirect
the help target.

Use both captured output and bounded real-terminal tests, with child-process
cleanup. Check exit status, output stream, absence of prompts or writes, and plain
help/version. Combine hostile layer manifest cases with a call-order review to
verify parsing returns before project access; silence alone does not prove that
no discovery occurred. Preserve existing operational safety assertions.

Inspect representative errors together at normal and narrow terminal widths.
Assess beauty, enjoyment, functionality, and ease of use explicitly: the hierarchy
should be calm and readable, choices and recovery actionable, and one-shot output
free of unnecessary ceremony. Passing snapshots alone do not establish a coherent
experience.
