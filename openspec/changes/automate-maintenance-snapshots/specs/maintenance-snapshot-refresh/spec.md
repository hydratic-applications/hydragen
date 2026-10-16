## ADDED Requirements

### Requirement: Periodic and on-demand checks

The repository SHALL provide a weekly GitHub Actions schedule and a manual trigger
to refresh maintenance evidence for explicitly tracked upstream repositories.

#### Scenario: Scheduled or manual run
- **WHEN** the weekly schedule fires or a maintainer triggers the workflow
- **THEN** the same refresh process checks every configured repository

### Requirement: Source-backed factual observations

Each successful check SHALL record its observation date, the current default-branch
commit date and URL, the latest published GitHub release date and URL if present,
and archival state for each repository. The output SHALL identify these as GitHub
activity signals rather than a complete measure of maintenance.

#### Scenario: Reader views a tool comparison
- **WHEN** a reader views a tracked tool in the existing approaches section
- **THEN** its dated maintenance evidence appears within that tool's entry rather
  than requiring a separate maintenance section

#### Scenario: Grouped or repeated tool comparisons
- **WHEN** a comparison covers several repositories or a tool appears in more than
  one comparison
- **THEN** every tracked repository has one dated evidence entry, and repeated
  comparisons link to that entry instead of duplicating its facts

#### Scenario: Repository has a release
- **WHEN** a tracked repository has a published GitHub release
- **THEN** its entry includes the observed commit, release, archival state, and links

#### Scenario: Repository has no release
- **WHEN** a successful API response establishes that no published release exists
- **THEN** the entry explicitly records no GitHub release rather than failing or
  inventing a release date

### Requirement: Authored foundations remain distinct from generated facts

The updater SHALL modify only repository-specific bounded maintenance-evidence
blocks within tool entries in `FOUNDATIONS.md`. It MUST NOT infer abandonment or
support status from inactivity, or rewrite qualitative comparisons or guiding
values. Authored narrative referring to the snapshot SHALL remain valid as dates
and archival states change.

#### Scenario: Separately dated maintenance policy
- **WHEN** a tool has human-authored evidence of a contribution or support policy
- **THEN** refreshing commit and release observations leaves that evidence and its
  own observation date unchanged

#### Scenario: Upstream activity resumes
- **WHEN** a previously inactive repository gains a newer commit or release
- **THEN** the evidence reflects the new activity without leaving a current-status
  assertion elsewhere in the comparison that contradicts it
- **AND** the updater does not invent an "actively maintained" verdict

### Requirement: Reviewable and repeatable publication

Successful checks SHALL refresh the observation date even if upstream facts are
unchanged. Changes SHALL be proposed through a single reusable automation PR,
without automatically merging or committing to the default branch.

#### Scenario: Existing automation PR
- **WHEN** a refresh changes the snapshot and an automation PR is already open
- **THEN** the workflow updates that PR instead of opening a duplicate

#### Scenario: Identical repeated run
- **WHEN** the same observations are rendered for the same observation date
- **THEN** the updater produces no documentation diff or additional PR

### Requirement: Failed checks preserve the previous snapshot

The updater SHALL publish a new snapshot only after collecting all configured
repositories successfully and finding the expected document boundaries. Failures
SHALL be visible in the workflow result without publishing partial observations.

#### Scenario: API failure or missing document boundary
- **WHEN** a repository request fails or a required boundary marker is missing
- **THEN** the workflow fails visibly and leaves the previously published snapshot
  unchanged
