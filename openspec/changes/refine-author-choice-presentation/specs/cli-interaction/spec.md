## ADDED Requirements

### Requirement: Present focused author sources and aligned provenance icons

The source hints associated with grouped Author choices MUST appear only beside
the focused candidate while its native cliclack selector is active. The complete
source list MUST follow the name in dimmed parentheses on the same logical row.
Unfocused candidates and selector actions MUST NOT show source hints. The
renderer MUST NOT insert a separate detail row or truncate source text; long rows
MUST retain normal terminal wrapping. These rules MUST NOT change the format
selector or other prompts.

Source hints MUST preserve the established source order and exact deduplication.
One source MUST stand alone, two MUST be joined with `and`, and three or more MUST
use commas with `, and` before the last source. Public GitHub hints MUST use
`GitHub profile <login>` or `GitHub organisation <login>` without `on github.com`
for an exact case-insensitive public-host match. Other GitHub hosts MUST retain
`on <host>`. Unsafe login/host display components MUST still be rejected before
formatting, even if the host suffix would otherwise be omitted.

Each active Author option MUST have one icon slot before its label, retaining the
native selected/unselected radio indicator. Accepted identity-source evidence
MUST use `👤`, GitHub membership evidence MUST use `🏢`, both MUST use one `◈`, and
project declarations alone MUST use `◇`. Identity sources MUST include the
existing Git, GitHub profile, account and environment-identity sources. Project
evidence MUST neither erase nor invent identity or membership evidence. Icon
classification MUST be independent of hint deduplication and MUST NOT infer an
entity type from name spelling or claim verified authorship.

`Enter another author` MUST use emoji pencil `✏️` and `Skip` MUST use `⏭`. The
common icon slot MUST be sized from the complete glyph set, with a minimum width
of two display columns, independent of visible choices. Narrower glyphs MUST be
right-padded and every slot MUST have two separator spaces before the label, so
all option labels align. Basic terminals (`TERM=dumb`) MUST instead use `U`, `O`,
`*`, `?`, `+`, and `>` respectively in equally sized slots. Disabling colour MUST
remove decorative styling without removing semantic icons or source text.

Icons and focused hints MUST remain presentation only. They MUST NOT change
discovery calls, grouping, ordering, initial-selection eligibility, typed actions,
accepted author values, optional entry/Skip behavior, or cancellation. Submitted
and cancelled answers MUST omit the added icons and source hints. A discovered
name containing action text, icon glyphs or parentheses MUST remain an ordinary
candidate rather than be interpreted as renderer decoration or an action.

#### Scenario: Focus reveals only that candidate's sources inline

- **WHEN** the active Author selector focuses a name supported by package.json and Git
- **THEN** that row shows its unchanged name followed by ` (package.json author and Git local user.name)` in dimmed source styling
- **AND** other candidate rows and both actions show no source suffix or separate detail row
- **AND** moving focus removes the previous suffix and shows the newly focused candidate's complete sources

#### Scenario: Grouped source lists use natural-language punctuation

- **WHEN** a grouped author has one, two, three or four distinct sources
- **THEN** the hint uses the original source order with one source alone, `first and second`, `first, second, and third`, or `first, second, third, and fourth` respectively
- **AND** repeated source labels remain deduplicated without splitting a source containing its own `and` or comma

#### Scenario: Public and Enterprise GitHub sources remain distinguishable

- **WHEN** candidates come from a profile or organisation on github.com or an explicitly selected Enterprise host
- **THEN** public-host hints retain the source kind and login but omit `on github.com`, including for differently cased spellings of that exact hostname
- **AND** Enterprise hints retain `on <host>` without changing host selection or network requests
- **AND** unsafe source display components are still rejected

#### Scenario: Equal identity and organisation names use one mixed icon

- **WHEN** accepted identity and GitHub membership sources support the same trimmed name, with or without a project declaration
- **THEN** the selector offers one grouped name with one mixed icon and every distinct source in its focused hint
- **AND** grouping, position, initial-selection eligibility and the accepted author value remain unchanged

#### Scenario: Project declarations do not guess entity type

- **WHEN** only a project declaration supports a name that looks like a company or a person
- **THEN** its icon is unspecified rather than inferred from its spelling
- **AND** adding accepted identity or membership support changes only the corresponding provenance classification, not the name

#### Scenario: Icons and actions keep one aligned label column

- **WHEN** identity, organisation, mixed, unspecified, custom-entry and Skip options are displayed or scrolled into view
- **THEN** each has one padded icon slot followed by two separator spaces, and all labels start at the same measured display column
- **AND** custom entry uses the emoji pencil, including its emoji-presentation selector
- **AND** visible choices do not change the common slot width

#### Scenario: Colour-disabled and basic terminals retain meaning

- **WHEN** guided Author selection runs with NO_COLOR or in a basic terminal
- **THEN** names, focus, source text and actions remain understandable without decorative styling
- **AND** NO_COLOR alone retains the Unicode icons while TERM=dumb uses the documented aligned ASCII alternatives regardless of colour settings

#### Scenario: Long inline sources wrap and clear correctly

- **WHEN** a 40- or 80-column terminal displays a focused source list longer than the available row width, including wide names and long source tokens
- **THEN** the full inline row wraps naturally without a dedicated detail line or lost source characters
- **AND** moving to a shorter source list or an action clears stale source text from the visible viewport
- **AND** submission and Escape or Ctrl-C cancellation clear added icons and source details from the completed frame

#### Scenario: Names containing decoration or action text remain literal

- **WHEN** a candidate name contains parentheses or begins with an icon, or is literally Skip or Enter another author
- **THEN** its typed candidate identity stays distinct from the two actions
- **AND** accepting it stores the unchanged discovered name without source hints or renderer-owned icons

#### Scenario: Summaries and unrelated selectors keep their existing behavior

- **WHEN** an author is accepted, skipped or cancelled, or init advances to the format selector
- **THEN** the completed Author step uses the established accepted-name, Skipped or cancellation presentation without added source hints or icons
- **AND** the format selector retains its existing labels, layout and native input behavior
