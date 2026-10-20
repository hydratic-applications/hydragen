# CLI experience principles specification

## Purpose

Define the CLI experience goals recorded in the product foundations and
contributor guidance, and the evidence and remediation-planning obligations of
the change that established them. This is a documentation and review contract;
command behaviour remains defined by the CLI interaction and initialisation specs.

## Requirements

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

### Requirement: Record safe preparation and visible responsiveness in existing values

`FOUNDATIONS.md` MUST refine the existing DX and performance values to favour
responsive interaction: prepare likely later-step information in the background
when safe and useful, avoid moving optional delays to startup, explain unavoidable
waits with progress feedback, and keep cancellation available. It MUST distinguish
background lifetime/resource limits from the time a developer actually waits once
the information is needed. Preparation MUST NOT assume user decisions, bypass
input/privacy/filesystem safety, or disrupt an active choice with late results.

The document MUST preserve all eight named guiding values and their existing
commitments rather than introduce a separate value for this technique. It MUST
integrate the guided-init example into a meaningful existing scenario: author
suggestions can be prepared while Name is answered, with bounded visible waiting
only if still needed at Author. It MUST present this as intended behaviour, not
claim that unimplemented runtime work or measured performance is already proven.
Concrete timeout values, worker architecture, and library APIs MUST remain in
the relevant change design/specifications rather than the product foundations.

#### Scenario: A first-time reader understands the responsiveness principle

- **WHEN** a developer reads the revised DX and performance values
- **THEN** they can distinguish preparing later-step information from guessing or applying an unconfirmed decision
- **AND** they understand why background work and visible waits need separate limits without relying on prior discussion
- **AND** the existing eight values and their previous commitments remain intact

#### Scenario: The init example explains the practical benefit

- **WHEN** a developer reads the everyday-development scenario
- **THEN** it explains how preparation during Name can remove a later Author pause
- **AND** unfinished optional work still has visible progress, cancellation, and a bounded wait rather than an indefinite loading state

### Requirement: Reinforce responsive preparation in CLI implementation and review guidance

`AGENTS.md` MUST carry reusable guidance to consider safe read-only preparation
after required interaction/safety checks when its inputs are already known. It
MUST call for early useful prompts, independently bounded background work and
visible waiting, suitable cliclack progress on guided waits, one owner of terminal
input/rendering, stable displayed choices, and cleanup of unneeded work. Existing
input-route, output-channel, permission, cancellation, and filesystem safety rules
MUST remain in force. This guidance MUST NOT require background work for every
command or a generic framework before a concrete need exists.

Review guidance MUST require evidence about when the first useful prompt appears,
whether earlier input stays responsive, actual visible waiting, progress/cancellation,
late results, and cleanup, not just total command duration or passing functional
tests. Wording MUST be understandable to developers new to Hydragen and MUST NOT
imply that showing an animation by itself makes a command fast or enjoyable.

#### Scenario: A later CLI change considers preparation

- **WHEN** a contributor consults the CLI development guidance for a multi-step command
- **THEN** it prompts them to consider overlapping safe preparation with earlier input where useful
- **AND** it preserves automation, confirmation, permission, and safety boundaries rather than authorising speculative side effects

#### Scenario: Review distinguishes responsiveness from animation

- **WHEN** a change adds background preparation and loading feedback
- **THEN** its review considers early prompt availability, input responsiveness, visible-wait duration, cancellation, late results, and cleanup
- **AND** neither a spinner nor total-duration measurements alone establish that the experience goals are met
