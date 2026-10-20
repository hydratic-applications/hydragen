## MODIFIED Requirements

### Requirement: Select an interaction mode without blocking automation

The CLI MUST accept global `--non-interactive` before or after the subcommand.
For init, it MUST select flag-only input whenever any of `--name`, `--author`,
`--description`, or `--format` is explicitly supplied, regardless of terminal/CI
state or value validity. Otherwise, init MUST enable guided input only when all
three standard streams are terminals, the explicit non-interactive flag is absent,
and CI is disabled. A non-empty `CI` other than `0` or `false`, compared without
case sensitivity, MUST disable interaction; unset, empty, `0`, and `false` MUST NOT
disable it by themselves.

When interaction is disabled, the CLI MUST NOT prompt, read answers from standard
input, or open another terminal. Init MUST use supplied flags and documented
defaults: omitted name uses the directory name subject to schema validation;
omitted author and description stay absent; omitted format is YAML (`layer.yaml`).
The inferred format MUST NOT count as an input flag. Prompt eligibility MUST
remain separate from compact status styling and colour preferences.

The initial add command MUST require source arguments and remain prompt-free in
every environment. Missing arguments MUST be a clap usage failure, not a guided
flow. `--non-interactive` MUST remain accepted without changing add's required
arguments or disabling otherwise eligible compact status styling.

#### Scenario: Default terminal interaction

- **WHEN** init runs without input flags, with terminal streams, no explicit non-interactive control, and CI disabled
- **THEN** it enables guided input including the final format selector

#### Scenario: Metadata flags select flag-only input

- **WHEN** init receives `--name` with or without optional input flags in an otherwise eligible terminal
- **THEN** it uses supplied flags and defaults without prompts or guided-session presentation
- **AND** leaves omitted optional metadata absent

#### Scenario: Optional metadata flags without a name override

- **WHEN** init receives author or description flags without name in an eligible terminal
- **THEN** it uses the schema-valid directory default and supplied metadata
- **AND** does not ask for missing fields or format

#### Scenario: Format alone selects flag-only input

- **WHEN** init receives only `--format jsonc` as an input flag
- **THEN** it uses the schema-valid directory default, omits optional metadata, and creates `layer.jsonc` when conditions permit
- **AND** uses no prompts or guided-session presentation

#### Scenario: Explicitly disable interaction in a terminal

- **WHEN** `--non-interactive` appears before or after init with terminal streams
- **THEN** init uses supplied flags and documented defaults without prompting
- **AND** omitted optional metadata remains absent

#### Scenario: A standard stream is redirected or closed

- **WHEN** any standard stream is not a terminal
- **THEN** init does not prompt or reopen a terminal

#### Scenario: CI has a terminal

- **WHEN** init runs without input flags with terminal streams and CI enabled, including `CI=1` or `CI=true`
- **THEN** it uses its documented defaults without prompts

#### Scenario: CI is explicitly disabled

- **WHEN** `CI` is unset, empty, `0`, or `false` in any letter case
- **THEN** that value alone does not prevent otherwise eligible init interaction

#### Scenario: Add never asks for missing sources

- **WHEN** add runs without source arguments in an interactive terminal or automation
- **THEN** clap reports the missing arguments without prompts, layer manifest discovery, or file changes

#### Scenario: Add accepts the global interaction control

- **WHEN** valid local-source arguments are supplied with `--non-interactive` before or after add
- **THEN** add remains argument-only
- **AND** compact status styling follows the shared presentation policy independently
