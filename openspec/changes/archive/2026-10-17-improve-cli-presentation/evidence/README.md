# CLI presentation evidence

The [verification report](../verification.md) records revision, implementation,
commands, outcomes, and limits. These are generated observations, not runtime
fixtures or a new terminal framework.

- [`before.json`](before.json): four fresh captured executions before editing
  runtime code, with environment/tool metadata and UTC timestamp.
- [`after.json`](after.json): fifteen post-change captured/PTY executions, with UTC
  timestamp, arguments, input sequences, exit status, stdout/stderr, and resulting
  layer manifest text where applicable. The UTC date can precede the 2026-10-17 local
  work date.

Captured runs close stdin and capture stdout/stderr separately. PTY runs use
distinct real terminals for stdout and stdin/stderr at the recorded `cols`/`rows`,
with `TERM=xterm-256color`. CI is cleared. `colour: true` forces `CLICOLOR_FORCE=1`;
false clears it and sets `NO_COLOR=1`, `CLICOLOR=0`. The records preserve terminal
controls and CRLF; normalised temporary paths are shown as `<project>`.

Each PTY frame records a label and `stderr_end`, a Unicode-character offset into
decoded `stderr`. Replay `stderr[:stderr_end]` for that state; these are not byte
offsets. Inputs record the marker awaited and actual bytes: `\r` is Enter,
`\u007f` is Backspace. Output and metadata are actual observations, not suggested
future appearance. A non-null layer manifest records post-run contents; an occupied
layer manifest case deliberately starts with `keep original`, which remains unchanged.

To repeat, build with `cargo build --locked`, use an absolute binary path and
fresh disposable directories outside repositories, and apply the recorded
arguments/environment/dimensions. For guided cases, wait for each input's marker
before sending its keystrokes. Use bounded waits and terminate/wait for children
on failures. No committed helper or external runtime service is needed by the CLI.

The prior audit's
[guided-retry record](../../2026-10-17-clarify-cli-experience-principles/evidence/sessions.json)
provides the unchanged pre-change validation frames. The existing Rust tests,
not these extra captures, supply race, symlink, redirect, cancellation, long-path,
and write/reporting-failure regression evidence. Run `cargo test --locked` to
repeat that behavioural coverage.

Visual review used labelled browser replay of actual output, not a full terminal
emulator or user study. Full-terminal redraw/resize, exact font/contrast/Unicode
behaviour, other operating systems, accessibility, and enjoyment judgements retain
the limitations stated in the report.
