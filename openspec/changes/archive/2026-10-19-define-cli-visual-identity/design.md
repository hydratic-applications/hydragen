# Expressive CLI identity: selected console design

## Context

On 2026-10-17, the author selected an expressive CLI direction and reviewed static
previews in macOS Terminal. The earlier
[presentation increment](../2026-10-17-improve-cli-presentation/design.md)
improved layout and wording but explicitly excluded a visual identity. This
document records the selected visual decisions and the focused implementation
approved after exploration.

Hydragen's icon depicts a stylised hydrogen molecule: two atoms connected by a
bond, arranged diagonally from upper-left to lower-right. The connector
reverses into negative space inside the circular atoms. The terminal motif
approximates the icon; it is not a replacement master logo. The
meaning of the name and icon is recorded in
[docs/brand.md](../../../../docs/brand.md).

## Goals / Non-Goals

**Goals:**

- Make the CLI recognisably Hydragen rather than relying only on default component
  styling.
- Preserve the selected motif, text hierarchy, alignment, and spacing.
- Distinguish the product identity from the operation being performed, allowing
  the same visual structure to support different operations later.
- Match the amount of branding to the interaction, keeping routine actions quiet.
- Retain cliclack's interaction components rather than build a prompt renderer.
- Share success, warning, and error styling across guided and compact output.

**Non-Goals:**

- Define a watch command; watch appeared only as an illustrative future operation.
- Introduce a general colour-override framework, automatic font detection, or a
  master asset format.
- Redesign help or change validation, cancellation, or filesystem safety behaviour.
- Replace cliclack's prompt components or change their input behaviour.
- Add a terminal font requirement, decorative delay, or new rendering dependency.

## Decisions

### Preserve the meaning of the name and icon

The name combines its relationship to the Hydratic namespace with the generative
nature of layers and an association with hydrogen's lightness and simplicity.
The icon depicts a stylised H₂ molecule: its atoms represent layers and its bond
represents a dependency. This reflects performance, ease of use, granular reuse,
and continued relationships to shared sources.

The inverted bond expresses seeing inside a composed result to understand its
inherited content and local contributions, connecting the mark to visibility,
traceability, and authoritative shared sources. The meaning of the name and icon
is documented in
[docs/brand.md](../../../../docs/brand.md); the terminal design must preserve that
meaning rather than substitute a different metaphor.

### Keep the dense 16-column by 8-row braille motif

Use the dense braille candidate reviewed as “Compact B”. Its size and shape were
preferred to the shorter and taller alternatives. Preserve the circular intent
of both ends and the diagonal connection rather than stretching the drawing or
substituting a generic chain or stack symbol.

Braille character boundaries remain visible in the reviewed font: internal dots
can sit closer together than dots in neighbouring characters. The author accepted
this limitation for now, not as an ideal or a universally solved spacing problem.

The sparser braille alternative was too thin. Half-block and quarter-block
alternatives retained solid fill but were less visually appealing to the author.
The selected baseline therefore retains braille's denser texture.

### Separate identity from operation

The selected arrangement has three text lines to the right of the motif:

- **Hydragen:** title case and bold, in the default foreground. The version sits
  two spaces to its right, with a `v` prefix and dimmed styling. `v0.1.0` in the
  preview is illustrative; a future implementation must use its actual version.
- **INIT:** the operation label, uppercase, bold, and accented.
- **Initialise a layer:** the operation description, in the normal foreground
  without bold or dimmed styling.

The motif uses the same accent as the operation label. Console previews used ANSI
cyan; a final colour policy and an exact match to the supplied icon's colours have
not been selected. Uppercase labels, grouping, and wording distinguish operation
from identity without depending solely on colour.

The product name is **Hydragen** in branding and prose. The executable remains
lowercase in commands, such as `hydragen init`.

Keep the identity stable while changing the operation label and description.
For example, **WATCH** uses “Watching for changes” in the normal foreground, with
the same hierarchy as init. A future foreground watch session fits the header policy
below; this example does not define the command or its lifecycle.

### Use the full header to establish a session, not announce every invocation

Reserve the full motif, product name, version, operation label, and description
for a terminal experience in which the developer stays with Hydragen across
multiple interactions or ongoing activity:

- **Multi-step guided flows:** guided init uses the full opening because the
  developer answers several questions within one coherent process.
- **Long-running foreground sessions:** a future watch command should use the
  same opening to establish the context for subsequent status and event output.
  Such a session does not need prompts to qualify.
- **One-shot actions:** omit the full opening. Report the useful result or failure
  concisely, even when the command is run manually in a terminal.

The distinction is the interaction's purpose, not its command name, an arbitrary
duration threshold, or the number of internal steps. A one-shot action that takes
time may need progress feedback, but that alone does not warrant a branded
opening. Conversely, guided init can finish quickly and still benefit from one.

The header establishes context that remains useful during a session. Repeating
it for routine actions consumes scrollback, pushes results away from the command,
and makes successive invocations harder to scan. Beauty and enjoyment include
restraint: output should be recognisable through consistent wording, spacing, and
status treatment without repeating the logo and product title.

Show the full opening once when a qualifying session begins, not before each
question, watch event, update cycle, or completion message. Help, version output,
argument-usage errors, and failures that prevent a session from starting do not
need a session opening. Omitting it does not remove useful operation context,
progress, results, or recovery guidance.

Apply this to the invocation, not uniformly to every use of a command. Guided
`hydragen init` gets the header; flag-only init does not, but can use compact
coloured status feedback in a terminal. CI and redirected execution do not acquire
a decorative opening. Prompt eligibility and `--non-interactive`'s role in
disabling questions remain unchanged. Styling and completion streams are addressed
separately below.

For future commands, distinguish foreground terminal presentation from unattended
or captured output when defining their interface. Do not infer that every flag
on a future long-running command disables its header: init's input-route rule is
not a general display rule for unrelated commands.

### Use cliclack for suitable guided interactions

Hydragen owns its motif and the identity/operation opening. Use cliclack where
interactive input or walking the developer through a process is useful, as in
guided init. For those flows, retain its input, selector, validation, cancellation,
and redraw handling rather than recreate them. Its components already provide an
attractive, enjoyable interaction while avoiding custom terminal complexity.

Cliclack is not a mandatory presentation layer for every command. Decide whether
a guided interaction is useful separately from whether a session warrants the
branded header. A future watch session can use the header without questions or a
cliclack-driven interaction; a one-shot action can simply report its result.
Other cliclack feedback components may be used where suitable, but their use is
not required merely for visual consistency. Do not introduce questions to justify
using the library.

Where cliclack is used, take its defaults and supported styling/theme APIs as the
baseline. The static guided-init study describes the desired hierarchy, not a
requirement to reimplement its literal characters or layout in custom components:

- Keep submitted values and connecting rails visually quieter than the active
  question, while leaving completed answers readable for review.
- Mark the active question with the accent and a bold label.
- Accent the selected format while keeping other choices readable.
- Keep optional annotations subdued and submitted skips explicit, dimmed, and
  italic. Preserve the distinction between a skipped answer and stored metadata.

Keep the existing input order, defaults, optionality, validation, and cancellation
behaviour. The motif need not be repeated inside prompts. Prefer a small
cliclack-supported adjustment over exact mockup matching that would require a
replacement renderer. Status styling follows the decisions below; inline
validation and cancellation use the red treatment described below.

### Share status styling without coupling it to prompts or the header

Use a consistent terminal treatment for operational feedback:

- **Success:** a green `✓` and green message, with the created filename bolded.
- **Warning:** a yellow `▲` and a headline explicitly labelled `Warning:`.
- **Error:** a red `✕` and a headline explicitly labelled `Error:`.

Keep explanatory details and recovery instructions in the normal foreground,
indented beneath the headline. Do not colour a whole multi-line diagnostic or
dim essential recovery guidance. Meaning must remain clear without colour.
Warnings alone remain non-fatal; actual failures retain nonzero exit status.

Guided flows use suitable cliclack feedback/footer facilities and supported text
styling. Compact commands use the same status conventions without acquiring
prompts, session rails, or a large header. Cliclack is not mandatory for those
messages. Showing a status does not justify showing the product identity again.

Treat input mode, header applicability, and status styling as separate decisions.
Flag-only terminal execution can use colour and status icons without questions.
Captured and unattended output stays plain: the same meaningful wording without
ANSI decoration or decorative status icons, and with the full diagnostic details.
Honour disabled-colour preferences according to the increment policy below.
Disabling colour must not disable guided input by itself or change a message's
meaning.

### Use a conservative presentation policy for this increment

Keep the existing three-terminal/CI/input-flag checks for guided init. Select
status decoration separately, but require all three standard streams to be
terminals, CI to be disabled, and `TERM` not to be `dumb`. This keeps a mixed
redirected invocation plain on both output streams, including when colour is
forced. `--non-interactive` disables questions, not eligible compact styling.

A non-empty `NO_COLOR` or `CLICOLOR=0` disables colour and decorative status
formatting, even with `CLICOLOR_FORCE`. A force variable never overrides the
terminal/CI eligibility gate. Configure the library's stdout and stderr style
decisions consistently because some default cliclack styles target stdout even
though its prompts are written to stderr. Disabled styling does not disable
questions. Help, version, and clap argument-usage output remain unchanged/plain.

Use the reviewed standard terminal cyan accent for this increment, with standard
green/yellow/red status roles rather than introducing exact RGB colours or a
theme framework. Plain compact statuses omit decorative icons and ANSI styling.
Guided no-colour prompts retain meaningful cliclack markers and text; normal
terminal redraw controls are not colour styling.

Measure the full opening's visible width, including the actual version. Render
the exact selected motif/text composition only if a known terminal width fits
it. With insufficient/unknown width or `TERM=dumb`, use text-only product/version,
operation label, and description instead. Do not stretch, crop, or inspect fonts
to compensate for character-cell differences. Basic-terminal guided editing
still belongs to cliclack; this increment does not promise a control-code-free
interactive renderer. Other unsupported-font cases remain a documented limit.

### Use red for rejected input and cancellation

Rejected input is an error, not a warning merely because it can be corrected.
Use the same red `✕` as operational errors, with a red frame and correction
message. Keep the icon and colour pairing consistent: a rejected answer must not
use a warning triangle simply recoloured red. Retain cliclack's prompt structure,
using its supported theme API to adjust the error-state symbol and colour rather
than replace validation or input handling.

Keep the entered value available for correction. Name the field and explain how
to fix it; optional-field feedback also explains clearing the field and pressing
Enter to skip. Correcting an answer resumes the normal prompt flow without a
separate command-level error announcement. A flag-only validation failure remains
a terminal error with the relevant flag, not a prompt to provide another answer.

Cancellation also uses red, retaining cliclack's cancellation treatment and its
`■` marker where supported. Clearly say that initialisation was cancelled rather
than suggest success or an unexpected input failure. Cancellation remains nonzero
and creates no files. Keep unexpected input or I/O failures distinct in wording,
even though they also use red.

The theme observes cliclack's cancelled state and omits its generic footer reason,
while retaining the cancelled prompt marker. After the prompt returns a bare
interruption consistent with that observed state, the application explicitly
delivers the init-specific cancellation reason through cliclack's cancellation
outro. Only a successful footer write creates the typed already-reported outcome;
main then exits nonzero without a second `Error:` line. A theme-formatting callback
alone is not evidence of delivery. Preserve OS/custom causes, interruptions
without an observed cancellation, and failed explicit footer writes as reportable
errors. Reset the guided theme on every return path.

This ownership follows cliclack 0.5.6's public API: cancellation formatting happens
before terminal clear/write/flush operations, and deliberate cancellation returns
a bare `Interrupted` error. The application cannot prove delivery from that error
kind or a formatting marker. Keep this adapter narrow and cover cancellation
observation, error causes, and explicit footer success/failure with injected-result
tests; do not build a replacement renderer or suppress generic interrupted I/O.

Placement, wording, and behaviour distinguish an editable field error, a deliberate
stop, and a failed operation; separate colours are not needed for those meanings.
The yellow `▲` pairing remains reserved for warnings; rejected input uses the red
`✕` pairing even though it can be corrected in place.

### Report init completion once, naming the layer manifest rather than its directory

Use this success wording, substituting the filename actually created:

> Layer initialised, layer manifest created at: layer.yaml

The terminal form prefixes it with the green checkmark and bolds `layer.yaml`.
Use `layer.yml`, `layer.toml`, `layer.json`, or `layer.jsonc` when that is the
selected format. Do not repeat the absolute directory: init runs in the directory
chosen by the developer, so the useful confirmation is the successful action and
the layer manifest filename. Do not append a second success or path-only result
line.

For guided init, use one cliclack-style completion on stderr with no separate
stdout result. For flag-only init, use one result line on stdout, styled when
appropriate for the terminal and plain when captured or unattended. Warnings and
errors remain on stderr. This keeps one visible completion in either input route.

These intentionally revise the old output contracts: plain flag-only output
becomes conditional on presentation conditions, and successful guided init no
longer reports the layer manifest separately on stdout. The change's delta
specifications and `AGENTS.md` guidance explicitly identify those replacements.

Keep truthful post-write failure reporting. If writing completed but feedback
failed, explain that the layer manifest was created. The shortened success location
does not remove paths needed to diagnose write failures or other errors.

### Name current-directory collisions without repeating the absolute path

When init refuses an occupied layer manifest path, explain the file's role before
its name. Use exactly two lines, substituting the actual occupied filename even
when its format differs from the requested format:

```text
Error: This directory already has a layer manifest (layer.yaml).
  Initialisation creates a new layer manifest; it won’t overwrite an existing one.
```

This tells developers why the existing file prevents initialisation without
requiring them to recognise `layer.yaml`. The supporting sentence explains both
what init does and why it refuses. Do not append generic inspection,
different-directory, or no-files-changed advice; those lines obscure the reason
for refusal rather than add a necessary action.

Naming the entry's role does not mean its contents are valid or have been read:
it can be a malformed file, directory, or symlink. Entry-preservation guarantees
remain unchanged. Adapt this structured error at the CLI boundary for both
preflight and exclusive-creation collisions; preserve the library error and
other diagnostic paths unchanged.

### Preserve the selected grid and spacing

Count rows and columns from one. The motif occupies an unchanged 16-column by
8-row grid, including its blank braille cells. Start the grid at the left edge,
without an additional ordinary-space indent. Its two leading blank braille cells
place the leftmost visible part of the motif two cells from the edge. Keep one
ordinary space between the grid and the text. Do not add spaces between the
braille characters.

All three text lines start at column 18. The title/version sits on motif row 3;
the operation label on row 5; the description on row 6. Row 4 has no text beside
the motif, separating identity from operation. The earlier two-line arrangement
felt less balanced and did not distinguish these roles as clearly.

This plain-text reference preserves the exact glyphs and alignment, but not their
bold, dimmed, or accent styling:

```text
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⣰⣾⣿⣿⣷⡄⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⣿⣿⡁⠈⠻⡿⠀⠀⠀⠀⠀⠀⠀⠀ Hydragen  v0.1.0
⠀⠀⠙⠿⣿⡦⢠⣶⣄⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠙⠿⠃⠺⣿⣶⣄⠀⠀ INIT
⠀⠀⠀⠀⠀⠀⠀⠀⣾⣦⡀⢈⣿⣿⠀⠀ Initialise a layer
⠀⠀⠀⠀⠀⠀⠀⠀⠘⢿⣿⣿⡿⠏⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
```

The motif's own blank cells contribute to visible whitespace. Its two-cell left
inset is part of the grid; the one-cell text gap is added after the grid. These
describe the character layout, not the exact pixel position of each font's dots.

### Review in a real console, not only HTML

Browser studies did not reliably show dot spacing or terminal proportions. The
author requested actual console previews and supplied a screenshot showing the
character-grouping issue. The final composition and spacing were subsequently
reviewed and accepted through static previews in macOS Terminal, whose preview
tab reported SFMono-Regular at size 11.

These were printed design studies, not the running Hydragen UI. They establish
the author's preference in that environment, not cross-platform rendering,
contrast, accessibility, or interactive redraw correctness. No final screenshot
or original vector asset has been added to the repository with this capture.

The completed increment's actual runtime checks, automated results, and evidence
limits are recorded separately in [verification.md](verification.md).

## Risks / Trade-offs

- Font-dependent dot spacing and cell proportions → retain the reviewed baseline,
  record the limitation, and review other environments before claiming portability.
- Dimmed version text and prompt hints may become difficult to read in some
  palettes → check their legibility during implementation design.
- The fixed composition needs horizontal space → use the text-only fallback when
  it cannot fit; unsupported-font behaviour remains a limit, not a minimum width.
- A branded opening could add noise to automation or frequent commands → preserve
  plain captured/unattended output and apply the session-based header policy.
- Status presentation changes existing output contracts → explicitly align the
  specifications and project guidance before implementation, preserving all
  unaffected channel, exit-status, and safety guarantees.
- Exact mockup matching could recreate terminal complexity → keep cliclack's
  components and use supported presentation adjustments.
- Planning reference mistaken for an implementation asset → source, tests, and
  build scripts must not load this document. Any eventual runtime motif must live
  in a maintained location outside `openspec/`.

## Open Questions

- Where should the master icon assets live, and in which formats?
- How should future long-running commands select foreground presentation versus
  unattended output, within the session-based header policy?
- Which additional terminals or fonts need verification beyond the current
  macOS/PTY review? Automatic braille/font detection is outside this increment.
- Would a later release benefit from explicit colour controls or a different
  palette? This increment uses the selected standard colours and disable policy.

The foundations' DX value reflects the presentation-restraint principle; exact
header and cliclack choices remain here. No new domain term needs a glossary
entry. The delta specifications and tasks define the approved implementation
increment; broader future command and branding questions do not block it.
