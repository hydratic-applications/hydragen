## RENAMED Requirements

- FROM: `### Requirement: Present focused author sources and aligned provenance icons`
- TO: `### Requirement: Present concise author sources and quiet provenance labels`

## MODIFIED Requirements

### Requirement: Present concise author sources and quiet provenance labels

Active Author choices MUST retain native radio indicators without added candidate
or action icons. Raw names and actions MUST start at the same display column.
Unfocused candidates MUST append a dimmed source-evidence category: `identity`,
`organisation`, `identity + organisation`, or `project`. These MUST use the existing
evidence aggregation without inferring a person's or company's identity from name
spelling. Project evidence MUST neither erase nor invent identity/membership
evidence. Categories MUST use ` · ` as separator, or ` - ` on basic terminals.

The focused candidate MUST instead append a dimmed parenthesised source summary.
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
Actions MUST show neither category nor summary. Submitted and cancelled answers
MUST omit renderer-added suffixes. Literal names containing action text, icons,
category words or parentheses MUST remain unchanged candidates. Other selectors
MUST remain unchanged. Disabling colour MUST preserve meaningful text.

The renderer MUST NOT truncate text, insert a detail row, or add expansion input.
Long names and remaining long summaries MUST wrap naturally and clear on redraw.

#### Scenario: Focus switches category to source summary
- **WHEN** a project-and-Git-backed name becomes focused
- **THEN** it shows ` (project metadata, Git)` instead of its category
- **AND** unfocused names show their categories and actions remain plain

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
- **AND** the unfocused category is `identity + organisation`
- **AND** unsafe login/host components remain rejected

#### Scenario: Project metadata does not establish entity type
- **WHEN** only project declarations support a company-like or person-like name
- **THEN** its category is `project`, not an inferred entity type

#### Scenario: Colour-disabled and basic terminals preserve text
- **WHEN** NO_COLOR is set or TERM is dumb
- **THEN** categories and summaries stay readable without emoji or added icon slots
- **AND** basic terminals use the ASCII category separator

#### Scenario: Names and actions remain aligned and literal
- **WHEN** a name contains Skip, parentheses, emoji or a category-like suffix
- **THEN** its complete raw name remains aligned with the plain actions
- **AND** accepting it stores only that unchanged name

#### Scenario: Long rows wrap and clear naturally
- **WHEN** a 40-, 60- or 80-column terminal displays a long name or Enterprise host
- **THEN** all summary text wraps naturally without truncation or extra detail rows
- **AND** moving, submitting or cancelling clears stale summaries and categories

#### Scenario: Completion and other selectors remain unchanged
- **WHEN** Author is accepted, skipped or cancelled, or the format selector opens
- **THEN** completed feedback has no renderer-added categories, hints or icons
- **AND** format labels, layout, input and cancellation retain their existing behaviour
