# CLI presentation verification

## Baseline

The runtime baseline is Git revision
`e2d50e853db6bb5fb2c9abd53a87aef2fc2ba712`, with the earlier uncommitted
principles/documentation work preserved. The baseline locked build passed before
editing runtime files. Fresh captured output is in [before.json](evidence/before.json);
guided before states remain in the earlier audit's
[unaltered session records](../2026-10-17-clarify-cli-experience-principles/evidence/sessions.json).

Environment: macOS 26.5.2 arm64, Rust/Cargo 1.96.0. The baseline command timestamp
is recorded in UTC in `before.json` (2026-10-16T22:31:52Z; local work date
2026-10-17). Captures close stdin, separate stdout/stderr, and use disposable
directories outside repositories. Temporary paths are normalised to `<project>`
in occupied-path evidence; existing bytes are checked after execution.

The before init help has a 117-character longest logical line. Guided validation
refers to `--name`/`--author`; the occupied-manifest error reports cancellation
without a safe next step.

## Implemented changes and dependency review

- Clap's stable `wrap_help` feature supplies detected-width wrapping with a
  100-column cap/fallback. Init descriptions use following lines; examples use
  short commands and explicit shell continuations. The first 40-column test caught
  clap splitting the original single-line example between `--name` and its value;
  shortening that sample name to `shared-rules` preserved valid copyable output.
- Guided feedback adapts the existing structured invalid-metadata error in
  `src/interaction.rs`. The validator and flag-only errors are unchanged; other
  error variants retain their diagnostic. Optional feedback explicitly explains
  clearing the field and pressing Enter to skip.
- Occupied-manifest errors now separate the reason, policy, next step, and
  no-change outcome into plain-text lines. They refer to this command's actions,
  remain accurate for arbitrary entries, and preserve the full path.
- Lockfile review found only clap_builder's new `terminal_size` edge and the
  `terminal_size` 0.4.4 package required by help wrapping. Existing package versions,
  including clap 4.6.7 and cliclack 0.5.6, did not change. No UI framework or new
  runtime service was introduced.

The library diff changes only the occupied-manifest display string: metadata
validation/serialization, preflight, exclusive creation, writes, and repository
checks are unchanged. CLI flags, mode detection, successful result output,
cancellation, and file-failure reporting retain their existing implementations.

## Behavioural verification

The final checks passed on the local work date 2026-10-17:

```text
cargo build --locked
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test interactive_init -- --test-threads=32
openspec validate improve-cli-presentation --strict
git diff --check
```

The full suite now has **66 passing tests**: seven library, fifteen binary,
28 captured-output integration, and sixteen terminal integration tests. The
terminal suite also passed its separate 32-thread invocation; its internal lock
continues to serialize terminal allocations and bounded waits/cleanup remain.

Coverage added or strengthened:

- Deterministic root/init short/long help at 40/80/100 columns, complete content,
  following-line description hierarchy, intact examples, and explicit preservation
  of a 60-character unbreakable token without exempting other prose from wrapping.
- Thirty-two real-terminal help cases: 40/80/100/160 columns, root/init, `-h` and
  `--help`, and colour forced on/off. Output stays plain, follows the cap, is on
  stdout, opens no prompts, and creates no files. Terminal version and argument
  error cases retain their channels/statuses and plain output under forced colour.
- Captured help with all standard streams non-terminal uses the fallback layout.
  A Unix test executes all four copied examples and checks the resulting metadata,
  not merely whether their text resembles commands.
- All three guided fields reject whitespace with field-specific instructions and
  no early file creation; coloured and no-colour sessions exercise correction and
  independent clear-to-skip for both optional fields. Unicode and surrounding
  whitespace remain unchanged, as do default acceptance and literal `Skipped`.
- Existing valid/malformed files, directories, symlinks/dangling symlinks, repeat
  init, concurrent creation, a manifest appearing during prompts, and long Unicode
  paths retain no-overwrite/preservation checks and gain recovery-advice assertions.
- The rest of the existing suite still verifies optional-only flags, CI,
  explicit non-interactive mode, individual redirected streams, cancellation/input
  loss, partial-write disclosure, and truthful reporting after a result-pipe failure.

## Fresh terminal evidence and visual review

[after.json](evidence/after.json) records fifteen additional captures/PTY sessions;
the [evidence guide](evidence/README.md) describes their settings and replay fields.
Help was observed at 40, 80, and 100 columns with `TERM=xterm-256color`, normal
30-row terminals, and forced colour (which help deliberately does not use).
Guided invalid/corrected sessions used separate stdout and stdin/stderr terminals,
both enabled and disabled colour, bounded five-second waits, and child cleanup.

| Init help evidence | Longest logical line | Outcome |
| --- | --- | --- |
| Before captured output | 117 characters | Word/option hierarchy breaks through terminal autowrap |
| After PTY at 40 columns | 40 characters | Prose wraps on words; examples remain intact |
| After PTY at 80 columns | 78 characters | Same information with readable continuation indentation |
| After PTY at 100 columns | 99 characters | Capped layout; same examples and input facts |

The help comparison was visually inspected in the browser using actual plain
output, reproducing only the old terminal's 40-cell ASCII autowrap. Guided
invalid-author before/after frames and no-colour invalid-description frames were
also replayed and visually inspected. These are **rendered recordings of actual
PTY bytes, not GUI terminal screenshots**. Palette, font rendering, contrast,
Unicode width, resize handling, and full terminal redraw behaviour are not proven
by that renderer. Raw controls are retained for independent inspection.

Actual optional-author feedback changed from:

```text
--author must contain at least one non-whitespace character.
```

to:

```text
Enter a non-blank author, or clear the field and press Enter to skip.
```

The current occupied-entry message is demonstrated by `after-occupied`:

```text
Error: '<project>/layer.json' already exists.
'hydragen init' does not overwrite existing entries.
Inspect the entry or run 'hydragen init' in a different directory.
This command did not change any files.
```

It exits 1 with no stdout and leaves the initial `keep original` bytes intact.
The additional lines improve the grouping without a terminal styling dependency;
long paths remain intact even when an actual terminal has to wrap them.

## Review against the four equal DX goals

- **Beauty:** the reviewed help layout has a consistent option/description
  hierarchy instead of split words at the right edge; recovery advice has clear
  line grouping. Guided warning styling is preserved, not replaced with a theme.
  This is a bounded reviewer judgement based on replay, not universal visual
  approval or a claim about all terminal palettes.
- **Enjoyment:** eliminating flag terminology from prompts and explaining safe
  recovery removes specific interruptions to the flow. Retry and optional-skip
  feedback remain predictable. Whether users find the overall CLI enjoyable still
  needs human feedback under R5; test success is not enjoyment evidence.
- **Functionality:** copied examples produce the expected manifests; accepted
  input, defaults, channels, exit outcomes, and safety checks remain intact across
  the passing captured and terminal suites.
- **Ease of use:** help remains complete and scannable at the reviewed widths;
  prompt feedback explains correction/skipping; occupied entries get a safe next
  action without a destructive suggestion or an assumption of manifest validity.

R1–R3's scoped acceptance checks pass. The foundations already express the relevant
product goals and need no further change; no glossary term or scenario was added.
README was checked against the implemented messages and help, and the success-
criteria question has only a narrow link to this increment's decisions.

R4's long-input/cursor/resize investigation and R5's broader visual-cohesion and
human enjoyment review remain open. No theme, prompt-frame, success/cancellation,
manifest-schema, minimum-width, cross-platform, accessibility, or performance
target decision is inferred from this work. The earlier audit/evidence remain
unchanged as the historical before baseline; only the completed remediation
checklist entries are updated after verification.
