## Context

The baseline CLI uses clap to require a `--name` flag and accept optional author
and description flags. `src/lib.rs` validates metadata and creates `layer.json`
exclusively, without any terminal interaction. Existing command tests deliberately
close standard input and capture output, exercising the unattended path.

The user wants cliclack prompts by default when working in a terminal, while
preserving flag-based use for scripts, CI, and agents. This follows the
foundations' developer-experience and optional-AI values: easier manual use must
not make automation depend on answering questions.

The completed `add-layer-init` change contains the baseline initialisation spec
but has not been synced or archived. This change's modified requirements refer
to that baseline. Sync the baseline before syncing this follow-up; its implemented
code can already serve as the starting point for this work.

## Goals / Non-Goals

**Goals:**

- Guide terminal users through bare init using cliclack.
- Preserve a complete flag-based route with no prompts or terminal UI.
- Keep manifest validation, representation, and no-overwrite guarantees shared.
- Default omitted name to the current directory's name in both input routes.
- Establish a small CLI interaction convention for future commands.

**Non-Goals:**

- Add commands, dependency management, rendering, or contribution capture.
- Change `layer.json`, infer metadata other than name from the directory, infer
  metadata from Git, or require AI.
- Redesign cliclack's visual theme, or add a spinner system, application shell,
  or general UI framework. A narrowly scoped skipped-answer rendering override
  retains the default styling.
- Add `--yes`, forced-interactive mode, overwrite confirmation, or a force option.
- Combine metadata flags and prompt answers in one invocation.

## Decisions

### Keep clap and use cliclack at the terminal boundary

Clap continues to parse arguments and provide help and version output. Add
cliclack for the prompt session and appropriate interactive feedback, using its
default styling rather than a new visual design.

The [cliclack 0.5.6 documentation](https://docs.rs/cliclack/0.5.6/cliclack/index.html),
reviewed on 2026-10-16, provides `intro`, `input`, `outro`, and `outro_cancel`.
[`Input`](https://docs.rs/cliclack/0.5.6/cliclack/struct.Input.html) supports
validation callbacks, `.required(false)`, and an `interact` method returning an
I/O result. These APIs fit the three metadata questions; no selection widgets or
progress indicators are needed.

Use a small binary-side interaction module to select the mode, collect answers,
and render feedback. Keep terminal detection and cliclack out of manifest
serialization and file creation. Replacing clap or designing a plugin framework
would add work without improving this increment.

### Select interactive mode conservatively

Add a global `--non-interactive` flag, accepted both before and after `init`.
Guided interaction is enabled only when all of these are true:

- No metadata flags (`--name`, `--author`, or `--description`) were supplied.
- `--non-interactive` was not supplied.
- Standard input, standard output, and standard error are all terminals, checked
  with Rust's `IsTerminal` interface.
- The `CI` environment variable does not indicate a CI run. Treat a non-empty
  value other than `0` or `false` as enabled, comparing those exceptions without
  case sensitivity. Unset, empty, `0`, and `false` do not disable interaction.

Any metadata flag selects flag-only input regardless of terminal or CI state.
The presence of a flag selects this route even if its value is invalid. Global
controls such as `--non-interactive` are not metadata flags; that control disables
guided input but does not supply metadata.

If any check disables interaction, do not construct or run prompts, read from
standard input, or attempt to recover input from a separate terminal device. Use
the directory-name default when name is omitted; author and description remain
absent unless supplied. Flag-only input also uses plain output, even in a terminal.

This is intentionally conservative: redirecting any standard stream disables
prompts. Terminal detection alone is insufficient because CI jobs and agents can
allocate terminals; the explicit flag provides a reliable override. Do not add a
registry of CI providers or agent-specific environment detection.

### Separate guided and flag-only input

The agreed rule is: **No metadata flags means guided initialisation. Any metadata
flag means flag-only initialisation.** Guided input is available only when the
terminal, CI, and explicit-override checks above allow it.

Make the clap representation of name optional during parsing. An omitted name
uses the directory-name default; a terminal user can also override it through the
name prompt. Name is still required in the manifest and library validation.

In flag-only input, use a supplied name or the directory-name default, validate
every supplied value, and omit unsupplied author and description. Invalid metadata
or a flag written without its value is an error, not permission to ask for a
replacement. Preserve accepted values unchanged.

In guided input, collect all metadata through prompts, in this order:

1. **Name:** show the directory name as the default; Enter without text accepts
   it, while typed text overrides it. Reject typed whitespace-only answers with
   inline feedback and allow another attempt.
2. **Author:** optional; show `(optional)` as a dimmed suffix,
   keeping the field label at normal emphasis.
3. **Description:** optional; use the same dimmed suffix.

Show a single dimmed guidance line after the intro and before the first prompt:
"Enter accepts the default or skips an optional field." Keep optional input
areas empty, reserving placeholders for actual defaults rather than instructions.
This keeps field status in the label, accepted values in the input area, and
keyboard guidance at session level without repeating it in completed prompts.

Style only the optional suffix and guidance line with the `console` crate used
by cliclack, targeting standard error and respecting its color-support detection.
This does not change validation, skipping, or output routes.

When Enter skips an optional field, show a dimmed, italic `Skipped` on the completed
answer line rather than leaving it blank. This distinguishes a deliberate skip
from an unanswered prompt without introducing a placeholder or default. The
manifest still omits the field; `Skipped` is not a sentinel metadata value, so
typing that word explicitly remains valid input and records it unchanged.

Cliclack 0.5.6 has no per-input submitted-empty label setting. Use a small
`Theme::format_placeholder` override only during guided collection: substitute a
display cursor containing `Skipped` when the render state is submitted and the
placeholder cursor is empty. Apply italics only to the indicator, respecting
terminal styling support, then delegate formatting to the default theme so its
dim styling and frame stay unchanged. Active, invalid, cancelled, and non-empty
default states retain the original rendering. Reset the default theme when
collection returns, whether it succeeds or fails. This narrowly scoped override
is the sole exception to avoiding a custom theme; it adds completion feedback
without redesigning cliclack or changing input values.

For an optional prompt, an empty answer means omit the field. A non-empty answer
containing only whitespace is invalid and gets inline feedback. Preserve
accepted answers exactly; do not trim them. Empty strings supplied explicitly as
flags remain invalid, preserving the existing flag contract.

For example, `hydragen init --name example` never asks questions and omits both
optional fields. `hydragen init --name example --author "Example team"` omits
description without asking for it. `hydragen init --author "Example team"` uses
the directory's name and the supplied author, even in a terminal; it does not
start a name prompt.
Bare `hydragen init` starts the full guided flow when interaction is available.
Do not add a final confirmation prompt or infer other metadata defaults.

Hybrid input is a legitimate pattern but is unnecessary for this small command:
it makes omitted optional flags ambiguous between "omit" and "ask" and adds input
combinations to test. Separate routes keep flag-based invocations predictable
without duplicating the validation or file-creation logic.

### Default name to the current directory

Supplying name is optional, but a resolved non-blank name remains required in
`layer.json`. This reduces repetitive input when the directory already has a
suitable project name and makes bare unattended init useful without changing the
manifest schema. This decision supersedes the earlier missing-name error and
no-inferred-default policy in this change; it does not alter the no-hybrid rule.

Resolve the default from the current directory's final path component, after
selecting the input route. A derived name is not a metadata flag and must not
disable guided input. Do not trim, slugify, or otherwise change it. A supplied
`--name` bypasses derivation, and explicitly empty or whitespace-only flags remain
invalid rather than falling back to the directory name.

Use cliclack's `.default_input(...)` for the name prompt: it displays the name
with a default hint and returns it when Enter is pressed without text. Typing a
name replaces the default rather than appending to it. Author and description
still have no defaults and are skipped independently with Enter.

If no name override is supplied and the directory has no final component (such as
a filesystem root), its name is whitespace-only, or it is not valid UTF-8, fail
before prompts or writes with guidance to supply `--name`. Do not fabricate a
fallback or silently replace invalid bytes. Explicit names remain usable at
these locations, subject to the existing target checks.

### Share validation and check the target before questions

Extract the existing non-blank field check into a small shared validation helper
so flags, prompt callbacks, and final metadata validation cannot develop different
rules. Keep the existing JSON Schema unchanged. The shared helper must not
require cliclack or duplicate the schema as a second runtime schema.

Before any prompt, perform read-only checks for an existing entry at `layer.json`,
the current directory, and the existing repository-location guard. Report known
failures immediately rather than collecting metadata for a target that cannot be
initialised. Do not read an existing manifest or offer overwrite, repair, or merge.

Factor these checks so interactive and non-interactive execution share them.
Keep the checks in the final creation path too: read-only checks before questions
do not protect against another process creating the manifest while the user is
answering. The final write must still use exclusive creation and occur only after
all metadata has been resolved and validated.

### Preserve output channels and make cancellation safe

Keep the successful result line and manifest path on standard output after a
successful write. Cliclack's interactive prompts and session feedback belong on
standard error. Verify the selected version's actual stream behaviour in a real
terminal test; do not assume that styling or automatic cancellation behaviour
satisfies Hydragen's output and exit-status requirements.

In non-interactive mode, retain plain standard output for success and plain
standard error for failures. Do not invoke cliclack presentation, add banners,
spinners, or terminal-control sequences, or change the existing stream contract.
Help and version remain clap-managed and do not open a prompt session.

Escape, Ctrl-C during a prompt, input ending unexpectedly, or a prompt I/O error
must terminate with a nonzero status, no success message, and no created files.
Report cancellation clearly rather than panicking or silently accepting defaults.
Do not depend on the library choosing the right process exit status automatically.
If rendering feedback fails after a successful file write, do not falsely claim
that cancellation left the directory unchanged; identify that the manifest was
already created.

### Keep the convention reusable without adding future commands

Record in `AGENTS.md` that future CLI work should use cliclack for suitable
interactive components, preserve flags for every required input, and honour the
same explicit and automatically selected non-interactive mode. When a command
offers guided input and input flags, keep those routes separate rather than
prompting to complete a flag-based invocation. Plain output is appropriate for
automation; not every command needs prompts or decorative UI.

This is developer guidance, not a new product layer or public framework. The
foundations already require efficient manual and agent workflows and need no new
value or library-specific wording. No glossary term needs to change.

### Test both decisions and actual terminal sessions

Unit-test mode selection with metadata-flag presence, terminal state, and CI
values passed as inputs; avoid mutating process-wide environment variables in
parallel tests. Keep the
existing captured-output tests for flag-driven execution, adding explicit
`--non-interactive`, CI, redirected-stream, and omitted-name default coverage.

Add terminal-session tests using a pseudo-terminal: a test-controlled terminal
that lets cliclack see real terminal streams while the test supplies keystrokes.
A small development-only harness, such as `portable-pty`, is appropriate; do not
add a runtime shell or Python requirement. Set a known terminal type and clear CI
in those child processes so the test environment cannot disable the prompts.

Wait for prompt labels before sending answers. Bound waits with timeouts, terminate
children on failure, and test behaviour rather than snapshots of exact borders
or escape sequences. Cover successful guided input, prompt-free flag-only input,
optional Enter-to-skip, validation retries, cancellation, EOF, an existing manifest
before prompts, and a manifest appearing before the final write. Test explicit
non-interactive mode and CI in a terminal too, not just captured-output processes.
Include bare init in those tests so metadata flags do not hide a broken CI or
override check by independently selecting flag-only input.

## Risks / Trade-offs

- **Guided input cannot be partially pre-filled with flags.** → Keep the rule
  explicit in help and README. Developers can use either the short guided flow or
  a complete flag-based invocation; revisit hybrid input only for a concrete need.
- **Prompt libraries may choose their own streams or cancellation handling.** →
  Keep the adapter small and verify real sessions, nonzero cancellation, and
  output channels. Resolve any incompatible behaviour before claiming completion.
- **A target can change while questions are being answered.** → Use read-only
  checks for early feedback, but keep exclusive creation as the final authority.
- **Terminal tests can hang or be timing-sensitive.** → Wait on output instead of
  fixed sleeps, use bounded timeouts, and clean up child processes and temporary
  directories on every outcome.
- **Conservative detection disables prompts with redirected output.** → Keep the
  flag path complete; do not reopen a terminal behind the caller's back.

## Migration Plan

No manifest migration is needed. Existing invocations with metadata flags continue
to use those flags and plain output without prompts, whether or not they have a
terminal. Omitted optional fields stay omitted. `--non-interactive` remains an
optional explicit safeguard for scripts and agents; it is not needed just to keep
a flag-based invocation prompt-free. Bare init starts a guided session only in
an eligible terminal; otherwise it uses the directory-name default without
questions. Author-only and description-only invocations now succeed with that
same name default, rather than reporting missing name. Document both routes and
retain errors for explicitly blank metadata.

Keep `add-layer-init` as the completed baseline and sync it before syncing or
archiving this change. Do not modify its historical no-prompt requirements to
hide this follow-up's behaviour change.

## Verification

Checked on 2026-10-16 on macOS with Rust and Cargo 1.96.0, cliclack 0.5.6, and
the development-only portable-pty 0.9.0 harness.

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo test --locked` passed. The suite contains 57 tests: seven library tests,
  eleven binary tests, 25 captured-output tests, and 14 terminal tests.
- Mode selection checks 256 combinations of metadata flags, explicit override,
  standard-stream terminal states, and CI values without mutating process-wide
  environment variables. Invalid metadata flags remain flag-only.
- Terminal tests use distinct terminals for standard output and standard error,
  verifying that cliclack's UI does not enter the result stream. Tests also cover
  the displayed directory-name default and Enter acceptance, typed name overrides,
  optional skips, retries, whitespace and Unicode, Escape/Ctrl-C cancellation,
  input loss with error output still available, early target errors, and a
  manifest appearing during questions.
- Directory-name tests verify unchanged spaces and Unicode, author-only and
  description-only flags, and defaulting in CI, explicit non-interactive mode,
  and redirected-stream execution. Unusable directory names produce actionable
  errors, explicit name overrides bypass default derivation, and explicitly
  blank values are rejected rather than replaced with the default.
- A closed result pipe verifies that an output failure after creation reports
  that the manifest exists, rather than claiming no files changed. A live input
  pipe verifies that non-interactive execution does not wait for answers or EOF.
- An initial parallel run encountered macOS pseudo-terminal allocation errors
  before some CLI children started. The harness now serializes terminal sessions
  and retains bounded prompt/exit waits and child cleanup. Other tests remain
  parallel. The terminal suite passed three additional runs of
  `cargo test --locked --test interactive_init -- --test-threads=32`.
- Manual terminal checks verified accepting the displayed directory-name default
  with Enter, overriding it with typed text, optional skips, and Escape with a
  clear cancellation message and no files. Captured-output checks verified bare
  init, author-only flags, and `--non-interactive` without name all default
  correctly without prompts. All used disposable directories.
- A terminal check with color enabled confirmed that only the optional suffix
  on Author and Description receives dim styling; field labels retain normal
  emphasis.
- Terminal tests also verify that shared Enter guidance appears once, before
  the first prompt and only on standard error, and that optional labels contain
  no repeated keyboard instructions.
- Rendering tests verify that `Skipped` appears only for submitted empty input,
  never during input, validation errors, or cancellation, and that default values
  retain cliclack's rendering. Terminal tests verify feedback before the next
  question, independently omitted fields, no skip marker when both fields are
  supplied, and literal `Skipped` input preserved as metadata rather than omitted.
- Strict OpenSpec validation, `git diff --check`, and local documentation link
  checks passed. The README and CLI guidance were reviewed for first-time readers.

The manifest schema and metadata representation are unchanged. The foundations
and glossary already cover the relevant product values and terms and need no
revision; CLI implementation conventions are recorded in `AGENTS.md`. The
requirement to sync the completed `add-layer-init` baseline before syncing this
follow-up still applies.

## Archival spec reconciliation

On 2026-10-16, the bulk archive applied `add-layer-init` first and this follow-up
second. Both changes touch `layer-initialization`: the baseline supplies seven
requirements, and this follow-up modifies metadata input and command outcomes.
The other five requirements remain unchanged. The old missing-name error scenario
is superseded by the directory-name default scenarios, not retained as competing
behaviour. This follow-up also adds four `cli-interaction` requirements.

The implementation supports this order: `src/lib.rs` retains shared validation,
repository-location checks, and exclusive creation, while `src/main.rs` and
`src/interaction.rs` implement the later guided/flag-only routes, name defaults,
and skip feedback. All 57 tests and both strict change validations passed before
archival. This satisfies the earlier requirement to sync the baseline first.
The archived baseline specifications retain their historical behaviour; the main
specifications describe the combined, current implementation.

## Open Questions

No product decision remains open for this increment. Discovery, dependency
versions, workspace layout, patch structure, and purpose-built agent interfaces
remain outside this change.
