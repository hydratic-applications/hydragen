## Why

Guided `hydragen init` always asks developers to type an author even when their
project, machine, or GitHub account already declares a suitable name. Offering
these names, including organisation names, as explicit choices reduces repeated
setup without making author required or changing automation defaults.

## What Changes

- Collect directly declared project authors from `package.json`, `Cargo.toml`, and
  `pyproject.toml` in the init directory, ahead of all machine/account suggestions.
  Extract names from supported author fields without copying contact details,
  running project tools, or searching parent/child projects.
- Collect other usable author names only in guided init, from Git author settings and
  the local account's display name and username. Use environment username values
  only when the account API cannot supply a usable username.
- If `gh` is available with existing authentication, also attempt read-only GitHub
  profile and organisation lookups within bounded background preparation. Use the profile
  display name, falling back to its login; use each organisation's display name,
  falling back to its login. Never launch login or request additional permissions.
- Start author discovery in the background before Name is answered, without
  delaying that prompt. Separate a 15-second preparation lifetime from a maximum
  1.5-second additional wait starting only when Author is reached. If work is
  already complete, continue immediately; otherwise show a cancellable cliclack
  loading indicator, then use the available results without waiting indefinitely.
- Freeze grouped suggestions before showing Author and stop accepting late
  results, so choices do not change under the developer. Stop unneeded background
  work and clean up owned child processes on timeout, failure, or cancellation.
- When at least one name is found, show a single-choice Author selector containing
  every distinct name, followed by `Enter another author` and `Skip`. Initially
  select the highest-priority project-declared or developer-identity name so Enter
  accepts it. An organisation explicitly credited by the project is eligible;
  GitHub-membership-only choices are not. If only membership suggestions exist,
  start on `Enter another author`.
- Group identical names into one option whose hint identifies all contributing
  sources; do not require developers to choose between duplicate values.
- When no usable name is found, bypass the selector and show an empty optional
  text input. Custom entry also uses an empty optional input; Enter without text
  omits author. Never prefill editable text or introduce special skip tokens.
- Treat unavailable tools, credentials, connectivity, permissions, and failed or
  timed-out lookups as ordinary absence, without warnings or authentication
  prompts; retain names found through other sources. Offline use remains supported.
- Preserve flag-only behaviour: no project-author, machine, or GitHub discovery;
  explicit `--author` wins, and omitted author stays absent.
- Update prompt guidance and tests so accepting a suggestion and skipping author
  are both clear, including without colour.
- Capture the general principle in the existing DX/performance values and CLI
  guidance: prepare likely next steps safely, minimise visible waits, and provide
  clear feedback when waiting is unavoidable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-interaction`: Define project-first author discovery alongside local and
  optional authenticated GitHub sources, grouped names with source hints,
  conditional single-choice selection or optional text entry, and the unchanged
  boundary between guided and flag-only execution; overlap discovery with Name
  and bound any additional Author-step wait with native progress feedback.
- `cli-experience-principles`: Document and reinforce safe background preparation,
  separate visible-wait and work-lifetime limits, and review of perceived
  responsiveness across future CLI changes.
- `layer-initialization`: Replace the former blanket prohibition on Git commands
  with a narrow exception for optional read-only author discovery in eligible
  guided init, including optional GitHub requests through existing `gh`
  authentication. Preserve offline/no-Git operation, prompt-free automation,
  early-exit isolation and the prohibition on Git mutations and project-tool
  execution.

## Impact

- CLI wiring in `src/main.rs`, guided prompts and validation feedback in
  `src/interaction.rs`, and a small independently testable author-discovery module
  with session-scoped workers, deadlines, cancellation, and incremental results.
- Existing JSON/TOML parsers can read the three project formats without invoking
  their package managers or introducing a new required tool.
- Git configuration is read locally when Git is available. Optional GitHub
  requests use `gh`'s existing credentials for one host/account, without requiring
  a GitHub SDK, credential management, or GitHub for ordinary init use.
- A cross-platform local-account lookup may add a focused Rust dependency.
- Unit, captured-output, and real-terminal tests; concise help/README wording and
  aligned agent guidance where existing empty-author instructions change.
- Refine the existing DX and performance values in `FOUNDATIONS.md` without adding
  another guiding value; reinforce the reusable principle in `AGENTS.md`.
- Author suggestions do not change the layer manifest schema, serialization,
  repository-location rules, creation safety, or existing-file discovery.
- Integrate with main's already-agreed lowercase layer-name policy and shared CLI
  diagnostics without redefining them. Name defaults and answers use the maintained
  schema; author and description keep their arbitrary non-blank string rules.
  Parser failures precede discovery, while eligible compact errors use shared
  styling and explicit help/version remain plain.
