## Why

Initialising a layer currently requires remembering and supplying flags even when
a developer is working directly in a terminal. Guided prompts can make that first
action easier, while scripts, CI, and agents still need a complete flag-based path
that never waits for input.

## What Changes

- Use cliclack for guided `hydragen init` when no metadata flags are supplied and
  the environment allows interaction, asking for name, author, and description.
- Treat any supplied `--name`, `--author`, or `--description` flag as a flag-only
  invocation. Never ask for missing values in that invocation, even in a terminal.
  Do not combine flags and prompt answers within one invocation.
- Make supplying name optional: default it to the current directory's name in
  either route, and display that default in the prompt so Enter accepts it. Keep
  name required and non-blank in the resulting layer manifest. Let developers skip
  optional author and description prompts without writing blank strings or `null`.
- Add a global `--non-interactive` flag. Automatically avoid prompts in CI and
  whenever standard input, output, or error is not a terminal.
- Preserve existing flag-based execution in every environment: omitted optional
  author and description stay absent, omitted name uses the directory name, and
  output stays plain. Explicitly blank metadata remains an error.
- Check for an existing layer manifest and unsupported repository location before asking
  questions. Cancellation or failed input must not create files; exclusive
  creation remains the final protection against overwriting a layer manifest.
- Establish a development convention to use cliclack for suitable future CLI
  interactions, always with an equivalent non-interactive path.
- Keep `hydragen init --name example` prompt-free without requiring another flag.
  Bare init now offers guided input in an interactive terminal; `--non-interactive`
  remains an explicit safety control rather than a prerequisite for flag-only use.

## Capabilities

### New Capabilities

- `cli-interaction`: Terminal and CI detection, explicit non-interactive mode,
  cliclack presentation, and cancellation behaviour for the CLI's current command.

### Modified Capabilities

- `layer-initialization`: Accept metadata through either flags or a guided prompt
  session, never both in one invocation, while retaining validation, layer manifest
  format, and file-safety guarantees.

The `layer-initialization` baseline is in the completed `add-layer-init` change,
which has not yet been archived or synced to the main specifications. This change
builds on that implementation and modifies its existing requirement names; sync
that baseline before syncing this follow-up. Do not rewrite the completed change
or introduce a second copy of its unchanged requirements here.

## Impact

- Update CLI argument handling and add a small interaction boundary around
  cliclack; keep layer manifest handling usable without terminal UI.
- Add the cliclack dependency and update `Cargo.lock`. Add terminal-session test
  support as a development-only dependency if needed.
- Reuse metadata validation and add read-only target checks before prompting,
  without changing the layer manifest schema or weakening exclusive creation.
- Extend automated tests, README usage examples, and CLI guidance in `AGENTS.md`.
- Keep dependency management, rendering, contribution capture, new commands,
  custom themes, spinners, and an interactive application shell out of scope.
