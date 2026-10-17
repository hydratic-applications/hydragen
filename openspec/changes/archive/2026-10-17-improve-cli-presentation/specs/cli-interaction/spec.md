## MODIFIED Requirements

### Requirement: Collect metadata through a guided cliclack session

Guided init MUST use cliclack to collect name, author, and description in that
order, without metadata flags. Init MUST NOT combine metadata flags and prompt
answers within one invocation or require a final confirmation.

Before the first prompt, the session MUST explain once that Enter accepts the
default or skips an optional field. Optional prompts MUST be labelled as optional;
their input areas MUST start empty rather than containing helper placeholders.
The name prompt's placeholder MUST represent its actual default value.

The name prompt MUST display the current directory's name as a default and accept
it when the user presses Enter without text. A typed name MUST override that
default. The resolved name MUST be non-blank; a typed whitespace-only name MUST
receive inline validation feedback and another opportunity to answer. Author and
description MUST be optional independently; an empty answer MUST skip the field,
while a non-empty whitespace-only answer MUST be rejected with inline feedback. Accepted
values MUST be preserved unchanged, and skipped fields MUST be absent from the
layer manifest rather than written as empty strings or `null`.
After Enter skips an optional field, the completed prompt MUST show a dimmed, italic
`Skipped` indicator rather than a blank answer. This is presentation only: it
MUST NOT become a default or a stored metadata value, appear before submission,
or mark cancellation as a skip.

Guided validation feedback MUST identify the current field and explain how to
correct the answer in prompt terms, without referring to metadata flags. For an
optional field, it MUST also explain that clearing the field and pressing Enter
skips it. Guided and flag-only input MUST use the same metadata validation rules;
adapting the message MUST NOT change validation or stored values. Flag-only
validation diagnostics MUST continue to identify the relevant flag without
starting prompts. The meaning of guided feedback MUST remain clear without colour.

#### Scenario: Initialise through prompts

- **WHEN** an eligible terminal user runs `hydragen init` without metadata flags
- **THEN** cliclack asks for name, author, and description in order
- **AND** only after valid answers or optional skips does init create the layer manifest

#### Scenario: Skip optional fields

- **WHEN** the user answers the name prompt with a valid name and presses Enter without text at an optional prompt
- **THEN** that field is omitted from the layer manifest
- **AND** its completed prompt shows `Skipped` before the next question or final feedback
- **AND** skipping author does not prevent supplying description, or vice versa

#### Scenario: Accept or override the directory-name default

- **WHEN** the name prompt opens
- **THEN** it shows the current directory's name as the default
- **AND** Enter without text accepts that name unchanged
- **AND** typing a valid name uses that answer instead of the default

#### Scenario: Correct invalid answers

- **WHEN** any typed non-empty answer is whitespace-only
- **THEN** the prompt identifies the field and explains how to supply a non-blank answer without referring to a metadata flag
- **AND** it allows another attempt and creates no file while input remains invalid or incomplete

#### Scenario: Clear an invalid optional answer to skip

- **WHEN** a whitespace-only answer is submitted for Author or Description
- **THEN** its feedback explains both supplying a non-blank answer and clearing the field then pressing Enter to skip
- **AND** clearing the input and pressing Enter skips only that field and shows its submitted-skip feedback
- **AND** the layer manifest omits the skipped field after the remaining input is complete

#### Scenario: Flag-only validation retains flag context

- **WHEN** a metadata flag is supplied with an empty or whitespace-only value
- **THEN** the plain diagnostic identifies that flag and the invocation fails without asking a replacement question
- **AND** no files are created

#### Scenario: Guided validation without colour

- **WHEN** guided input is available but terminal colour is disabled and a non-empty whitespace-only answer is submitted
- **THEN** the field and correction or optional-skip instructions remain understandable from the text
- **AND** validation, retry, and no-early-write behaviour are unchanged
