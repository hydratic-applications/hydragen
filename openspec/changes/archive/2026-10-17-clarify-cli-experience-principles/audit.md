# CLI experience audit

## Baseline and method

Audit started on 2026-10-16 against revision
`e2d50e853db6bb5fb2c9abd53a87aef2fc2ba712`. Local changes establish the new DX
paragraph in `FOUNDATIONS.md`, CLI guidance in `AGENTS.md`, and the narrow
success-criteria decision in `openspec/design-questions.md`. This OpenSpec change
and its evidence are uncommitted. Runtime sources and dependencies are unchanged.

Environment: macOS 26.5.2, arm64; Rust 1.96.0, Cargo 1.96.0, OpenSpec 1.5.0,
Python 3.14.6. CLI version 0.1.0; clap 4.6.7, cliclack 0.5.6, console 0.16.6.
Captured commands use isolated disposable directories. Terminal sessions use
`TERM=xterm-256color`, explicit terminal dimensions, and controlled CI/colour
variables. Ambient `CI`, `NO_COLOR`, `CLICOLOR`, and `CLICOLOR_FORCE` were unset.

The baseline is the updated DX value and CLI guidance, constrained by the current
CLI interaction, layer initialisation, and manifest specifications. Reviewed
`src/main.rs`, `src/interaction.rs`, `src/lib.rs`, README, all three main specs,
`tests/init.rs`, and `tests/interactive_init.rs`. The implemented scope remains
`init`, clap help/version, and their success/failure paths; future commands are
not assessed.

Assess each surface separately for **beauty**, **enjoyment**, **functionality**,
and **ease of use**. **Meets** means supported within the observed conditions,
not universal compliance; **gap** identifies a concrete shortfall; **needs design
review** marks a qualitative choice that needs human review; **not verified**
means evidence is missing. Separate existing-contract violations from new-goal
design opportunities. There is no numeric aesthetic score or user-research claim.

## Evidence and verification

`cargo build --locked` and `cargo test --locked` passed on 2026-10-16. The suite
contains 57 tests: seven library, eleven binary, 25 captured-output integration,
and fourteen terminal integration tests. No runtime files were changed for this
audit, and no new permanent test harness was added.

Additional evidence is retained in [31 session records](evidence/sessions.json).
See the [evidence guide](evidence/README.md) for record fields, input conventions,
and reproduction. Captured commands used closed stdin and separate stdout/stderr.
Interactive observations used real, separate pseudo-terminals for stdin/stderr
and stdout, with five-second waits and child-process cleanup. Recorded frames
include prompt-active, invalid-answer, and completed states—not just stripped
text. Sessions used 100×30 cells normally and 40×30 for narrow presentation;
these are observation conditions, not newly agreed minimum supported widths.

The ANSI frames were replayed in a temporary browser renderer and visually
inspected. It interprets cursor movement, line clearing, wrapping, colour, dim,
italic, and reverse-video input cursors. This is **rendered replay of actual PTY
output, not a capture from Terminal.app or another full terminal emulator**.
Its palette and Unicode cell widths are approximations; it does not establish
exact contrast, font appearance, resize handling, or redraw correctness in a
particular terminal. No new runtime renderer or visual theme is part of this
change. Raw bytes are preserved so observations can be checked independently.

### Scenario evidence

| Surface | Session IDs or passing test evidence | Observed outcome |
| --- | --- | --- |
| Help and version | `help-root`, `help-init`, `version`, `help-normal`, `help-narrow` | Exit 0, stdout only, no writes; help text remains identical at 40 and 100 columns. |
| Argument errors | `missing-command`, `unknown-option`, `missing-value` | Exit 2, stderr only, no writes; usage or a help hint is supplied. |
| Flag-only input | `flag-name`, `flag-author`, `flag-description`, `flag-default`, `terminal-flag` | Exit 0, stdout result, no prompts; defaults and supplied metadata recorded. |
| Invalid flags | `flag-invalid`, `terminal-invalid-flag` | Exit 1, stderr diagnostic, no manifest or replacement prompt. |
| Guided success | `guided-default`, `guided-override`, `guided-author-only`, `guided-description-only` | Exit 0; prompts/completion on stderr, result path on stdout; default acceptance, Unicode override, independent skips, and supplied metadata work. |
| Guided retry | `guided-retry` | Name and author whitespace fail inline; correction succeeds with exit 0. Passing `invalid_answers_can_be_corrected_without_creating_files_early` also verifies no premature writes. |
| Cancellation | `cancel-escape`, `cancel-ctrl-c`, `cancel-partial-escape`, `cancel-partial-ctrl-c` | Exit 1, no stdout, no manifest; cancellation and no-change message on stderr. |
| Automation | `terminal-ci`, `terminal-noninteractive`, `terminal-flag` | Exit 0, plain stdout, empty stderr, no questions even with terminal streams. |
| Each redirected stream | `redirecting_any_single_stream_disables_guided_input` | Passing PTY test redirects stdin, stdout, and stderr independently; all succeed without prompts and retain result-channel rules. |
| Existing target | `existing-manifest`; `an_existing_manifest_is_reported_before_any_questions` | Exit 1, stderr only, original bytes preserved; passing PTY test confirms rejection before questions. |
| Repository/default errors | `nested-repository`, `unusable-default`; `repository_location_errors_happen_before_questions` | Exit 1, no writes; identifies repository root or instructs an explicit name. |
| Reporting failure after write | `a_result_output_failure_does_not_claim_the_manifest_was_not_created` | Passing test closes the result pipe; nonzero outcome says the manifest was created and confirms its bytes, not a false no-change message. |
| Other safety failures | `write_failures_report_the_path_without_false_success`, `a_manifest_created_during_questions_is_not_overwritten`, `losing_terminal_input_reports_failure_without_creating_a_manifest` | Passing tests cover partial-write disclosure, creation races, and actual terminal input loss. |
| Presentation conditions | `guided-nocolour`, `guided-narrow-long`, `help-narrow`, `help-normal`, `guided-override` | NO_COLOR preserves meaning; long/CJK/emoji default is stored unchanged; narrow wrapping needs improvement/review below. |

The redirected-stream, race, input-loss, and reporting-failure rows rely on the
existing tests and inspected assertions, not additional manual reproductions.
This avoids unsafe manipulation just to obtain a screenshot. The current suite
also verifies symlink preservation, concurrent exclusive creation, Unicode
round-trips, and the interaction-mode combinations. It does not prove aesthetic
quality, subjective enjoyment, or cross-platform presentation.

## Four-goal assessment

These are reviewer judgements tied to the evidence above, not user-study results.
In particular, **needs design review** for enjoyment is not a finding that users
dislike the CLI. A restrained plain presentation is not inherently a gap.

| Surface | Beauty | Enjoyment | Functionality | Ease of use |
| --- | --- | --- | --- | --- |
| Root help/version | Meets: short, aligned structure | Needs design review: no user feedback | Meets: correct channels/status, no writes | Meets: command and help entry points are visible |
| Init help | Gap: hard wraps break hierarchy (F1) | Needs design review: dense presentation adds likely friction | Meets: options and input rules are accurate | Gap: descriptions and examples are harder to scan (F1) |
| Argument errors | Meets: concise, consistent clap layout | Needs design review: recovery wording has not been user-tested | Meets: exit 2, stderr, no writes | Meets: missing value/unknown option and help hint are explicit |
| Guided happy path | Meets in replay: coherent state symbols and emphasis | Needs design review: welcoming/effective feel remains subjective | Meets: defaults, overrides, skips, channel separation | Meets: one-time Enter guidance, labelled optionals, explicit skip feedback |
| Guided validation | Meets: local warning is visually distinct | Needs design review: route-mismatched wording may interrupt flow | Meets: invalid answers retry safely | Gap: prompt feedback speaks in flags (F2) |
| Completion | Needs design review: guided outro plus plain result is deliberate but stylistically mixed (F5) | Needs design review: repetition may or may not help | Meets: success follows creation, stdout path preserved | Meets: completed state and created path are explicit |
| Cancellation | Needs design review: framed cancellation followed by plain diagnostic (F5) | Needs design review: two messages merit human review | Meets: both keys and partial-input paths fail safely | Meets: cancelled outcome and no files changed are explicit |
| Plain/automation input and output | Meets: intentionally brief and uncluttered | Needs design review: predictable execution is positive, enjoyment untested | Meets: no prompts/ANSI UI, correct channels and defaults | Meets: optional-only flags work, omission is predictable |
| Existing-manifest error | Meets: readable plain diagnostic | Needs design review: being blocked may need better guidance | Meets: rejects without reading/overwriting | Gap: no safe next step is stated (F3) |
| Repository/default errors | Meets: plain, consistent diagnostics | Needs design review: error experience untested with users | Meets: fails before writes | Meets: root location or explicit-name remedy is given |
| Difficult failure paths | Not verified visually: test evidence only | Not verified: no usability session | Meets within tested paths: truthful diagnostics and safety | Meets for inspected messages: affected path/state is stated |
| Narrow/long guided presentation | Gap in replay: broken frame continuity/wrapped words (F4) | Needs design review: actual perceived friction is untested | Meets for recorded session: full value survives and init succeeds; exact redraw unverified | Gap in replay: long default is harder to follow (F4) |
| No-colour guided presentation | Meets in replay: labels/symbols survive without colour | Needs design review: contrast and preference untested | Meets: defaults/skips and manifest preserved | Meets: meaning does not depend on colour or italics |

## Findings

### F1: Help does not adapt to terminal width

**Classification:** concrete presentation/usability gap against the new DX goal;
not a confirmed violation of an existing runtime requirement.

`help-init`, `help-normal`, and `help-narrow` all emit the same content after
normalising PTY CRLF, with a longest logical line of 117 characters. At 40 cells,
the option-description column starts near the right edge; automatic terminal
wrapping splits words and drops continuation lines to the left margin. Even at
100 cells, the name description and longest example wrap mid-word. Examples and
input-route guidance are useful and should be preserved, not removed to shorten
the help.

`src/main.rs` contains long option descriptions and literal `after_help` prose;
`Cargo.toml` enables clap derive but not its optional wrapping feature. Investigate
clap's supported width-aware facilities and deliberate example layout in a
separate change rather than hand-wrapping blindly. Do not require colour to fix
this problem.

### F2: Guided validation uses flag-oriented wording

**Classification:** concrete ease-of-use gap against the new guidance; shared
validation behaviour itself meets the existing contract.

`guided-retry` shows `--name must contain at least one non-whitespace character.`
inside the Name prompt and the equivalent `--author` error inside Author. This
identifies the field, but refers to an input mechanism the user is not using.
`src/interaction.rs` passes the shared `InitError` display through to cliclack;
`src/lib.rs` formats that error as a flag. Preserve the common non-blank rule and
unchanged accepted values, but adapt presentation at the terminal boundary.

### F3: Existing-manifest feedback stops without explaining how to proceed

**Classification:** concrete recovery-guidance opportunity under the new goal;
not enough evidence to call the existing actionable-diagnostic contract violated.

`existing-manifest` reports the exact path and says initialisation was cancelled
with no changes. It neither says that the file was deliberately left unread nor
offers a safe next action. Other target errors are stronger: the repository error
names the correct root and the unusable-default error says to supply `--name`.
The manifest diagnostic could say that init does not overwrite existing entries
and suggest inspecting the entry or choosing a different directory. It must not
suggest blindly deleting it, assume a valid existing layer, add a force mode,
or read/repair the entry.

### F4: Narrow guided presentation needs a wrapping pass

**Classification:** concrete layout limitation in recorded output/replay, with
terminal-specific redraw and Unicode details still needing direct-terminal review.

`guided-narrow-long` succeeds and stores the complete CJK/emoji default unchanged.
However, the 53-character guidance message exceeds 40 cells, and the long default
wraps across rows without a continuing frame/indent. Replay shows words split
mid-word. The recording includes appropriate multi-row clearing sequences; there
is **no confirmed redraw corruption or data loss**. Before altering cliclack
presentation, reproduce in a full terminal emulator, including editing long input
and resizing. Use a small layout correction if warranted; do not truncate stored
values or redesign the theme as an assumed solution.

### F5: Overall visual direction and feedback cohesion need human review

**Classification:** optional qualitative refinement, not a proven adherence gap.

Normal guided output has clear, coherent default styling and intentional skip
feedback. Clap help is intentionally plain (`ColorChoice::Never`), runtime errors
use `Error:`, clap errors use `error:`, completion includes a guided outro plus
the required stdout result, and cancellation has both a framed library message
and a plain no-change diagnostic. These differences are observable but are not
automatically wrong: separate result/diagnostic streams are necessary, and plain
output is part of the contract. Human review should decide whether any difference
is distracting enough to warrant change. No branding, colour palette, custom
theme, or forced decoration is recommended by default.

## Limits and conclusion

No confirmed breach of the existing interaction, manifest, or filesystem-safety
contracts was found within the executed cases and inspected passing tests. This
is not a proof of universal correctness. The new experience goals expose specific
layout and wording improvements (F1–F4), while F5 requires a preference decision.

Not verified: Windows/Linux presentation, actual GUI terminal palettes and fonts,
screen-reader use, contrast across light/dark terminal themes, terminal resize
behaviour, long-input cursor editing, broader grapheme/emoji sequences, and user
perceived enjoyment. Ordinary and narrow observations are not benchmarks or a
commitment to a particular minimum width. Startup felt prompt in the test runs,
but no performance threshold or benchmark has been established.

The foundations addition remains appropriate after this audit: no product belief
needs weakening to accommodate the implementation. Existing values, glossary
terms, and scenarios need no further changes. See the
[remediation plan](remediation-plan.md) for the bounded follow-up work; no runtime
fixes have been applied here.
