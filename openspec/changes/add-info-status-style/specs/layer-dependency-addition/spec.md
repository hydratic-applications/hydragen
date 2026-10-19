## MODIFIED Requirements

### Requirement: Report actual additions separately from pre-existing dependencies

Successful add MUST exit zero and use one compact stdout result naming the
destination layer. Feedback MUST select its layout from the combined number of
verified requested names in the `Added` and `Already present` outcomes, after
deduplication, not from the number of source arguments.

Exactly one added dependency and no already-present outcome MUST use
`Added <dependency> to <destination>.` on one line. Exactly one already-present
dependency and no added outcome MUST use `No changes needed: <dependency> is
already a dependency of <destination>.` on one line. These single-result forms
MUST omit separate group headings, counts, lists and blank lines, and end with
one newline.

For more than one effective requested dependency, feedback MUST retain the
existing destination headline and separate nonempty `Added` and `Already present`
groups with counts. Each dependency MUST appear once in its outcome group; each
group MUST preserve first-appearance argument order. `Already present` MUST mean
present before this invocation, not a later repeat of a newly added source.
Unrequested existing dependencies MUST NOT be included or affect layout selection.
All-repeat execution MUST explicitly report no changes needed and identify the
requested already-present dependencies in the appropriate single or grouped form.

The layer manifest path MUST NOT appear by default in success feedback. Add MUST
NOT show a full branded opening, prompt/session frame, or per-source success
messages. Eligible terminal statuses MUST use shared styling, and plain output
MUST retain meaningful text without ANSI or status icons. Warnings and errors
MUST use stderr. CI, redirected streams, colour disables, and `--non-interactive`
MUST follow the shared presentation policy independently of prompt eligibility.

All-repeat results MUST use the shared blue `ℹ` informational style, whether
single or grouped, with no automatic `Info:` label. Results with actual additions,
including mixed results, MUST retain green-check success styling. Single-result
names MUST retain consistent bold emphasis in the chosen status colour; grouped
supporting content MUST retain its existing normal foreground and label/count
emphasis. Plain output wording and layout MUST remain unchanged.

#### Scenario: One dependency is added

- **WHEN** add verifies and adds one effective requested dependency
- **THEN** stdout reports `Added <dependency> to <destination>.` with no one-item list or count
- **AND** eligible terminal output retains green-check success styling

#### Scenario: One dependency was already present

- **WHEN** all supplied references identify one dependency present before invocation
- **THEN** stdout reports one `No changes needed: <dependency> is already a dependency of <destination>.` sentence
- **AND** eligible output uses blue informational styling rather than a green success check
- **AND** existing no-op edit/staging bypass, bytes, file metadata and stored reference spelling remain unchanged

#### Scenario: Several arguments identify one newly added dependency

- **WHEN** repeated or alias arguments resolve to one new named dependency
- **THEN** the single added-result sentence and green success styling are used
- **AND** the dependency is not reported as already present because of later repeated arguments

#### Scenario: Mixed additions and harmless repeats

- **WHEN** two requested sources are new and one was already declared
- **THEN** success names the destination layer and lists two names under `Added (2)` and one under `Already present (1)`
- **AND** it retains green success styling without printing the layer manifest path or unrelated dependency names

#### Scenario: Only one multi-result outcome group is populated

- **WHEN** more than one effective requested dependency is new or all were already present
- **THEN** only the relevant nonempty counted group is shown
- **AND** an all-repeat result describes no changes with a blue informational headline
- **AND** a result with additions retains green success styling

#### Scenario: Plain automation output

- **WHEN** add runs with redirected streams or CI enabled
- **THEN** stdout retains the destination and dependency names in the appropriate single or grouped layout without ANSI or decorative status icons
- **AND** warnings/errors remain on stderr and no-op results still exit zero

#### Scenario: Unrequested existing dependencies do not make a batch result

- **WHEN** the current layer has other dependencies but only one effective dependency is requested
- **THEN** only the requested name and destination appear in the single-result sentence
- **AND** unrelated declarations do not produce counts or groups or change status classification
