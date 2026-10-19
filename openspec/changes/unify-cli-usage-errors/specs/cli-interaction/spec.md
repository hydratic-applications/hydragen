## ADDED Requirements

### Requirement: Present parser failures through the shared error style

Root and subcommand parser failures MUST use one shared presentation route, not
per-command style exceptions. Eligible terminal output MUST use the existing red
`✕ Error:` headline and normal-foreground details without a branded opening or
guided-session frame. Plain output MUST retain meaningful `Error:` wording,
usage, and correction information without ANSI or decorative status icons.

Eligibility MUST follow the shared policy: all standard streams are terminals,
CI is disabled, TERM is not dumb, and no nonempty NO_COLOR or CLICOLOR=0 disable
is present. Explicit disables and CI/redirect gates MUST win over forced colour.
Global `--non-interactive` MUST affect questions, not otherwise eligible styling.
Usage errors MUST use stderr and retain clap's nonzero usage status, ordinarily 2.
Operational errors MUST retain their existing status, presentation, and meaning.

#### Scenario: Parser errors share terminal presentation

- **WHEN** root, init, or add rejects a missing argument, unknown option/command, invalid value, or conflicting arguments in an eligible terminal
- **THEN** one red cross error headline is shown with readable normal-foreground details
- **AND** stderr carries the failure without prompts, branding, or a stdout result

#### Scenario: Usage failures in automation

- **WHEN** a parser failure occurs with CI, any redirected standard stream, a dumb terminal, or explicit colour disabling
- **THEN** output remains plain even when colour is forced
- **AND** stderr and the nonzero usage status remain unchanged

#### Scenario: Non-interactive terminal error

- **WHEN** a parser failure occurs in an otherwise eligible terminal with global `--non-interactive`
- **THEN** it uses shared compact error styling without asking questions

### Requirement: Explain invalid invocations using clap's command context

Clap MUST remain responsible for parsing and argument validity. The shared
presentation adapter MUST preserve useful argument names, supplied values,
allowed choices, available suggestions, usage, and help hints from its structured
diagnostics rather than replace every failure with a generic message. It MUST NOT
infer error meanings by matching human-readable parser error text or introduce a
second parser. Unsupported error-context shapes MUST still produce a safe useful
fallback rather than panic or accidentally execute the command.

An add invocation without a source MUST clearly explain that at least one source
is required, show relevant usage, one valid explicit-local-path example, and a
hint for `hydragen add --help`. This MUST remain a prompt-free usage failure,
not a full unsolicited help dump, project check, or implicit empty add operation.

#### Scenario: Add has no source

- **WHEN** add is invoked without sources, including with only a global interaction flag
- **THEN** stderr explains the missing source with usage, an example, and an add-help hint
- **AND** it exits nonzero without asking questions or performing layer discovery

#### Scenario: Invalid format retains correction context

- **WHEN** init receives an unsupported format value
- **THEN** the shared error identifies the option and invalid value and retains the supported choices
- **AND** it does not turn the error into a missing-layer or generic failure

#### Scenario: Unknown option retains a useful suggestion

- **WHEN** clap provides a spelling suggestion for an unknown option or command
- **THEN** the shared error retains that suggestion and relevant usage/help context
- **AND** no operation is executed automatically on the user's behalf

### Requirement: Parse arguments before layer discovery and preserve explicit help

All parser failures MUST be handled before current-directory layer discovery,
layer-manifest loading, schema preparation, prompts, or writes. A missing or
invalid layer manifest MUST NOT replace the argument error or produce a duplicate
warning for a command whose syntax was rejected. Only successful operational
parsing MUST allow layer prerequisites to be checked.

Supported explicit help and version invocations MUST remain successful plain
output on their established stream, with no error headline, status icon, branding,
layer discovery, validator setup, prompts, or writes. Existing width-aware help
and accepted help/version forms MUST remain unchanged. A clap help-on-missing-
input failure MUST NOT be treated as a successful explicit help request.

#### Scenario: Missing source outside a layer

- **WHEN** bare add is run where no enclosing layer manifest exists
- **THEN** the missing-source usage error is reported first without searching for a layer

#### Scenario: Valid syntax outside a layer

- **WHEN** add receives a source argument where no enclosing layer manifest exists
- **THEN** normal operational discovery reports the missing current layer manifest
- **AND** it does not report a missing-source usage error

#### Scenario: Help in a directory with invalid duplicate layer manifests

- **WHEN** supported root or subcommand help or version is requested in a directory with invalid or duplicate layer manifest paths
- **THEN** it succeeds with plain help/version output and no layer warnings or errors
- **AND** no files are changed

### Requirement: Keep untrusted parser diagnostic values inert

Untrusted option, argument, supplied-value, and suggestion/context text MUST use
the shared display-only escaping policy before being placed in diagnostic layout.
Raw terminal controls, carriage returns, line feeds, Unicode separator/bidi
controls, and other characters covered by that policy MUST NOT execute or spoof
diagnostic lines. Application-generated layout, width adaptation, and eligible
status styling MUST remain intact. Escaping MUST NOT alter parsing, accepted
argument values, filesystem input, or stored data.

#### Scenario: Unknown option contains terminal controls

- **WHEN** an invalid option or value includes control characters
- **THEN** its diagnostic shows readable escaped text rather than emitting those raw controls
- **AND** generated usage/help line breaks and the shared error style remain intact

#### Scenario: Ordinary Unicode and literal escapes

- **WHEN** a diagnostic value includes ordinary Unicode or literal backslashes
- **THEN** those characters remain understandable and are not repeatedly escaped
- **AND** valid path arguments still reach operational handling unchanged

### Requirement: Normalize parser diagnostics into one optional-block structure

Parser failures MUST use one shared body renderer, not a bespoke missing-source
body alongside framework-formatted bodies. Structured clap facts MUST populate a
presentation model without introducing another argument grammar or interpreting
human-readable framework messages to identify error meanings or command paths.
The sole command-path envelope exception MUST be a narrow match of clap-generated
`Usage: ` metadata against complete trusted canonical command-path boundaries;
semantic error-prose interpretation and hand-written argv scanning remain forbidden.

The body MUST order a consistently punctuated sentence-case headline, optional
context/correction, relevant usage with at most one useful example, and a final
command-specific help hint. Supporting text MUST use a two-space base indent,
one blank line between blocks, and consistent label capitalization including
`Possible values:`, `Tip:`, `Usage:`, and `Example:`. Choice lists MUST NOT use a
separate square-bracket style. Omitted blocks MUST leave no empty labels or filler.
The shared plain/styled routes MUST preserve the same structure and meaning.

Relevant usage and help MUST refer to the erroring command when its identity is
unambiguously verified. Current root/init/add failures MUST be covered, including
value errors for which clap omits Usage context. Unsupported future graph or
context shapes MUST retain explicit unknown ownership and safe general help,
without inventing a root origin or a scoped example. Command-context recovery MUST NOT change accepted arguments,
execute operations, perform layer discovery, or reclassify which parser failure
occurred. Untrusted fields MUST retain the established display escaping. Unknown
context/fallback failures MUST preserve safe useful information without exposing
framework debug names as the ordinary user interface.

Explicit help MUST retain its own plain documentation layout. Operational
failures MUST retain existing safety/state information and MUST NOT acquire
irrelevant parser usage/examples merely to fill a template.

#### Scenario: Missing source and invalid format share organization

- **WHEN** bare add and init with an invalid format produce different error facts
- **THEN** both use the same section order, indentation, punctuation, and label conventions
- **AND** invalid format shows normal `Possible values:` and `Tip:` lines before usage/example and init-specific help
- **AND** missing source omits unnecessary context while retaining its useful add example and add-specific help

#### Scenario: Command context is retained for value failures

- **WHEN** a parser error lacks framework-generated usage context
- **THEN** current supported root/init/add cases still receive the correct command-specific usage/help
- **AND** a value resembling a command or a later command token cannot redirect the diagnostic context
- **AND** ambiguous or unsupported future cases retain unknown ownership and safe general help rather than inventing an origin

#### Scenario: Future error kinds follow the shared contract

- **WHEN** another command or parser failure is added
- **THEN** it uses the same shared layout and family-level style checks
- **AND** content-specific choices, suggestions, or examples do not introduce another rendering template
