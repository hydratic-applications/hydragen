# Audit evidence guide

[`sessions.json`](sessions.json) records 31 executions on 2026-10-16 of the
unchanged `target/debug/hydragen` binary built with `cargo build --locked`.
The [audit](../audit.md) records revision, environment, test results, and limits.

## Record fields

- `id`: stable identifier referenced by the audit.
- `mode`: `captured` means stdin was `/dev/null` and stdout/stderr were separate
  pipes; `pty` means separate real pseudo-terminals for stdout and stdin/stderr.
- `args` and `directory`: arguments and disposable working directory. Reproduce
  using your own isolated directory, not the project checkout or the recorded
  temporary directory, which may no longer exist.
- `exit`, `stdout`, `stderr`: actual status and output, preserving ANSI sequences,
  CRLF, and Unicode. JSON escapes control characters; decoding restores them.
- `manifest`: bytes decoded as UTF-8 if an entry existed after execution, otherwise
  null. This alone does not mean the invocation created the file: the
  `existing-manifest` case deliberately began with `keep original` and retained it.
- PTY records also include `cols`, `rows`, `colour`, `ci`, and `inputs`. Each input
  records a prompt/error marker awaited before sending the indicated bytes.
- `frames`: labels with `stderr_end`, a Unicode-character offset into decoded
  `stderr`. Replay `stderr[:stderr_end]` to inspect the state immediately before
  the corresponding input or at completion. These offsets are not byte offsets.

## Conditions and reproduction

All executions used `TERM=xterm-256color`. CI was cleared except for
`terminal-ci` (`CI=true`). Colour was forced with `CLICOLOR_FORCE=1`, except for
`guided-nocolour`, which removed that variable and set `NO_COLOR=1`, `CLICOLOR=0`.
Other executions still retained plain automation output under forced colour.
PTY sizes were 100×30, except `help-narrow` and `guided-narrow-long` at 40×30.
Colour settings for captured commands match the normal forced-colour condition.

For manual reproduction, build the binary, use an absolute path to it, create a
fresh disposable working directory outside existing repositories, and run the
recorded arguments there. Set terminal width/height and environment as above for
PTY cases. For captures, close stdin and capture stdout and stderr separately.
For guided runs, wait for the recorded marker before each input. In decoded
inputs, `\r` is Enter, `\u007f` is Backspace, `\u001b` is Escape, and `\u0003`
is Ctrl-C. Inspect the resulting manifest only after the command has exited.

The existing-manifest case creates `layer.json` containing `keep original` before
execution. The nested-repository case creates a `.git` directory in the parent
and executes from its `nested` directory. The unusable-default case uses a
directory whose final component is three spaces. No external repository data is
needed. Automated observation used bounded five-second waits and guaranteed
child termination/cleanup, without a permanent harness or dependency addition.

Raw ANSI output was visually reviewed using temporary rendered replay, not a
full GUI terminal. Keep that limitation when interpreting layout, palette, and
Unicode widths. Input-loss, each-single-stream redirection, race, write failure,
and closed-result-pipe evidence comes from the passing existing Rust tests named
in the audit, not extra records in this JSON. Run `cargo test --locked` to repeat
that behavioural coverage. No user research or cross-platform visual checks are
represented by these records.
