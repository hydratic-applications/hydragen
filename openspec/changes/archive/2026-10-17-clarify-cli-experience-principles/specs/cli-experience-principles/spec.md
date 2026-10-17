## ADDED Requirements

### Requirement: State equal CLI experience goals within DX

`FOUNDATIONS.md` MUST extend the existing **DX — Developer Experience** value to
state beauty, enjoyment, functionality, and ease of use as equally important CLI
design goals. It MUST apply them to help, prompts, results, and errors, including
plain output where appropriate, without prescribing a visual style. It MUST
preserve the existing eight values and the current DX requirements concerning
reuse, readability, and ordinary workspace editing.

#### Scenario: Review the revised product value

- **WHEN** a contributor reads the updated guiding values
- **THEN** the existing DX value explicitly establishes all four goals with equal importance across the CLI
- **AND** neither a particular visual style nor a ninth value is introduced
- **AND** the existing DX commitments remain intact

### Requirement: Reinforce quality without changing CLI contracts

`AGENTS.md` MUST connect CLI development guidance to the DX goal and require
review of visual appeal, enjoyment, functionality, and ease of use across help,
prompts, results, and errors. It MUST preserve current tool choices, separate
input routes, interaction detection, plain flag-only output, output channels,
validation, performance, cancellation safety, and filesystem safety guidance.
It MUST NOT require prompts or decorative output for every command or treat
plain output as exempt from intentional design.

#### Scenario: Review guidance for a CLI change

- **WHEN** a contributor consults CLI development guidance
- **THEN** it calls for considering all four experience goals without prescribing a visual theme
- **AND** the current interaction, automation, and safety rules remain in effect
- **AND** meeting functional requirements alone is not presented as completing the experience-quality review

### Requirement: Audit the implemented CLI against the established baseline

After the documentation update, the change MUST include an `audit.md` report
covering implemented help/version/argument errors, guided input, flag-only input,
success feedback, validation and target errors, cancellation, automation, and
representative presentation conditions. It MUST assess ordinary and narrow
terminal widths, supported and disabled colour, and long or Unicode names/paths.
It MUST use both captured-output evidence and actual terminal observation, with
existing tests supplying evidence for difficult failure paths where appropriate.

The report MUST record the date, revision and relevant local changes, environment,
commands or test cases, inputs, output channels, and exit outcomes. For each
surface it MUST assess all four goals, identifying whether each meets the goal,
has a gap, needs design review, or was not verified, with supporting rationale.
It MUST distinguish confirmed contract violations from qualitative opportunities
and disclose evidence limitations. It MUST NOT infer beauty or enjoyment from
test success alone or claim unperformed user research.

#### Scenario: Review an audited guided session

- **WHEN** the report assesses guided init
- **THEN** its evidence includes actual terminal presentation, not just stripped captured text
- **AND** it covers defaults, optional skips, validation retry, completion, and cancellation
- **AND** it assesses all four goals with reasons and separates observed facts from design judgement

#### Scenario: Verify automation and difficult failures

- **WHEN** the report assesses non-interactive execution and failure handling
- **THEN** it references execution or test evidence for prompt suppression, stream separation, exit outcomes, and relevant safety guarantees
- **AND** untested environments or paths are explicitly marked as not verified rather than assumed compliant

### Requirement: Produce a traceable remediation plan without applying fixes

After the audit, the change MUST include a `remediation-plan.md` linking proposed
work to findings and affected goals or existing requirements. Each actionable
item MUST describe its proposed direction, likely affected files, dependencies,
priority rationale, and acceptance checks. The plan MUST distinguish necessary
adherence work from optional refinement, identify decisions or evidence still
needed, and organise work into reviewable increments. Runtime or contract changes
MUST be identified for later OpenSpec changes and user review, not implemented
under this change.

#### Scenario: A confirmed gap needs follow-up

- **WHEN** the audit identifies a confirmed gap
- **THEN** the remediation plan traces a proposed increment back to the finding and affected goal or requirement
- **AND** it states how to verify the improvement and preserve existing contracts
- **AND** prioritisation is explained by impact and risk rather than a hierarchy among the four goals
- **AND** this change stops before implementing the fix

#### Scenario: The audit finds no confirmed gaps

- **WHEN** the audit identifies no confirmed gaps
- **THEN** the remediation plan states that outcome without inventing fixes
- **AND** any unresolved design judgements or unverified conditions remain visible for review
