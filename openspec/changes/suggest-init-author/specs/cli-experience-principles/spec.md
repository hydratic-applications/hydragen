## ADDED Requirements

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
