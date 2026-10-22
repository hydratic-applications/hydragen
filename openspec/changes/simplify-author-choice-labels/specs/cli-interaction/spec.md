## RENAMED Requirements

- FROM: `### Requirement: Present focused author sources and aligned provenance icons`
- TO: `### Requirement: Present concise author sources and quiet provenance labels`

## MODIFIED Requirements

### Requirement: Present concise author sources and quiet provenance labels

Active Author choices MUST retain native radio indicators without added candidate
or action icons. Raw names and actions MUST start at the same display column.
Every visible candidate MUST append the same dimmed parenthesised source summary
whether focused or not. Focus MUST change only the radio indicator and name styling,
not row wording, suffix, spacing or line wrapping. Separate source-evidence category
suffixes MUST NOT replace or duplicate those summaries. The text MUST describe
accepted sources without inferring a person's or company's identity from spelling.
Project evidence MUST neither erase nor invent identity/membership evidence.

Summaries MUST be derived from accepted structured sources, deduplicating display
families in first-source order: `project metadata`, `Git`, `GitHub`, `local account`,
and `GitHub organisation`. Git settings and author environment sources MUST share
`Git`; project files MUST share `project metadata`; account-name/username and
environment username fallbacks MUST share `local account`. Git keys/scopes,
project filenames and account logins MUST NOT inflate these display summaries.
Non-public GitHub hosts MUST retain `on <host>`; public-host matching MUST remain
case-insensitive, with unsafe login/host components rejected before display.
Families MUST be joined with comma-space, without a final `and`.

Complete detailed hints and source distinctions MUST remain internal for grouping,
ordering, safety and eligibility. This display rule supersedes detailed provenance
wording only in the picker, including the other author-discovery requirements'
references to visible hints. It MUST NOT change discovery, grouping, ordering,
initial selection, typed actions, accepted values, omission or cancellation.
Actions MUST show no summary. Submitted and cancelled answers
MUST omit renderer-added suffixes. Literal names containing action text, icons,
category words or parentheses MUST remain unchanged candidates. Other selectors
MUST remain unchanged. Disabling colour MUST preserve meaningful text.

The renderer MUST NOT truncate text, insert a detail row, or add expansion input.
Long names and remaining long summaries MUST wrap naturally and clear on redraw.

#### Scenario: Source summaries stay fixed while focus moves
- **WHEN** focus moves among names supported by different source families
- **THEN** every visible candidate keeps its same compact parenthesised summary
- **AND** only the radio marker and name styling change, without rewrapping rows
- **AND** actions remain plain

#### Scenario: Repeated settings collapse without losing evidence
- **WHEN** the same name comes from multiple Git settings, GitHub profile and local-account sources
- **THEN** its summary is `Git, GitHub, local account`
- **AND** all distinct detailed hints, ordering and eligibility remain unchanged

#### Scenario: Project files share one family
- **WHEN** three supported project files declare the same author
- **THEN** the picker shows one `project metadata` summary family

#### Scenario: Organisation and Enterprise context remain distinct
- **WHEN** profile and membership evidence support one name on an Enterprise host
- **THEN** the summary retains separate GitHub and GitHub organisation families with the host
- **AND** those same source distinctions remain visible without focusing the name
- **AND** unsafe login/host components remain rejected

#### Scenario: Project metadata does not establish entity type
- **WHEN** only project declarations support a company-like or person-like name
- **THEN** its summary says `project metadata`, not an inferred entity type

#### Scenario: Colour-disabled and basic terminals preserve text
- **WHEN** NO_COLOR is set or TERM is dumb
- **THEN** summaries stay readable without emoji, added icon slots or category separators
- **AND** the same source text remains visible regardless of focus or colour settings

#### Scenario: Names and actions remain aligned and literal
- **WHEN** a name contains Skip, parentheses, emoji or a category-like suffix
- **THEN** its complete raw name remains aligned with the plain actions
- **AND** accepting it stores only that unchanged name

#### Scenario: Long rows wrap and clear naturally
- **WHEN** a 40-, 60- or 80-column terminal displays a long name or Enterprise host
- **THEN** all summary text wraps naturally without truncation or extra detail rows
- **AND** moving focus within the same visible choices preserves all row text and wrapping
- **AND** submission and cancellation clear the picker summaries from completed feedback

#### Scenario: Completion and other selectors remain unchanged
- **WHEN** Author is accepted, skipped or cancelled, or the format selector opens
- **THEN** completed feedback has no renderer-added categories, hints or icons
- **AND** format labels, layout, input and cancellation retain their existing behaviour
