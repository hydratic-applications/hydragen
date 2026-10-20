## ADDED Requirements

### Requirement: Present neutral outcomes using shared informational styling

Compact presentation MUST support informational messages as a reusable shared
style, using blue `ℹ` in eligible terminals without an automatic `Info:` prefix.
Supporting lines MUST remain in the normal foreground. Plain output MUST preserve
the existing meaningful wording without colour, an icon or a new label.

Info MUST represent neutral observations rather than imply a new change. It MUST
NOT by itself change an operation's exit status or output channel. Actual
affirmative results MUST retain success styling; warning/error styling and guided
interaction MUST remain unchanged. Existing terminal, CI, redirection,
colour-disable-over-force and `--non-interactive` rules MUST govern info too.

#### Scenario: Neutral compact message in a terminal

- **WHEN** an informational result is rendered in an eligible terminal
- **THEN** it uses the shared blue info icon and headline, without `Info:`
- **AND** supporting lines remain uncoloured and unbranded

#### Scenario: Informational output in automation

- **WHEN** informational output is captured, redirected, in CI, or explicitly colour-disabled
- **THEN** its text remains unchanged without ANSI, status icon or added label
- **AND** its established stream and status are unchanged even when colour is forced
