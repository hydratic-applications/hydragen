## MODIFIED Requirements

### Requirement: Select an interaction mode without blocking automation

The CLI MUST accept global `--non-interactive` before or after the subcommand.
For init, it MUST select flag-only input whenever any of `--name`, `--author`,
`--author-email`, `--author-url`, `--description`, or `--format` is explicitly
supplied, regardless of terminal/CI state or value validity. Otherwise, init MUST
enable guided input only when all three standard streams are terminals, the
explicit non-interactive flag is absent, and CI is disabled. A non-empty `CI`
other than `0` or `false`, compared without case sensitivity, MUST disable
interaction; unset, empty, `0`, and `false` MUST NOT disable it by themselves.

When interaction is disabled, the CLI MUST NOT prompt, read answers from standard
input, or open another terminal. Init MUST use supplied flags and documented
defaults: omitted layer name uses the directory name subject to schema validation;
omitted author and description stay absent; omitted author contacts stay absent
unless supplied explicitly through shorthand or contact flags; omitted format is
YAML (`layer.yaml`). Contact flags without `--author` MUST fail usage validation,
not trigger a default author or a question. The inferred format MUST NOT count as
an input flag. Prompt eligibility MUST remain separate from compact status styling
and colour preferences.

The initial add command MUST require source arguments and remain prompt-free in
every environment. Missing arguments MUST be a clap usage failure, not a guided
flow. `--non-interactive` MUST remain accepted without changing add's required
arguments or disabling otherwise eligible compact status styling.

#### Scenario: Default terminal interaction

- **WHEN** init runs without input flags, with all three standard streams attached to terminals, no `--non-interactive` flag, and CI mode disabled
- **THEN** it enables guided input, including the final format selector

#### Scenario: Metadata flags select flag-only input

- **WHEN** init receives `--name` in an otherwise eligible interactive terminal, with or without optional input flags
- **THEN** it uses only supplied flags and documented defaults without prompts or guided-session presentation
- **AND** omitted optional metadata fields remain absent from the layer manifest
- **AND** eligible compact status styling does not turn it into a guided session

#### Scenario: Optional metadata flags without a name override

- **WHEN** init receives author or description flags without layer name, even in an eligible interactive terminal
- **THEN** it uses the schema-valid directory default and supplied metadata
- **AND** it does not ask for name, other metadata, or format

#### Scenario: Format alone selects flag-only input

- **WHEN** init receives `--format jsonc` without metadata flags in an otherwise eligible interactive terminal
- **THEN** it uses the schema-valid directory default, omits author and description, and creates `layer.jsonc` if other conditions are satisfied
- **AND** it asks no questions and uses no guided-session presentation

#### Scenario: Explicitly disable interaction in a terminal

- **WHEN** `--non-interactive` is supplied before or after `init`, even with terminal streams
- **THEN** init does not prompt and uses supplied flags, the directory-name default, and the YAML format default when omitted
- **AND** omitted author and description remain absent
- **AND** the flag does not independently disable eligible compact terminal styling

#### Scenario: A standard stream is redirected or closed

- **WHEN** any of standard input, output, or error is not a terminal
- **THEN** init does not prompt or attempt to read answers from another terminal

#### Scenario: CI has a terminal

- **WHEN** init runs without input flags, CI mode is enabled, including by `CI=1` or `CI=true`, and all standard streams are terminals
- **THEN** init still runs non-interactively
- **AND** it uses the directory-name and YAML defaults without asking questions

#### Scenario: CI is explicitly disabled

- **WHEN** `CI` is unset, empty, `0`, or `false` in any letter case
- **THEN** that value alone does not prevent otherwise eligible terminal interaction

#### Scenario: Add never asks for missing sources

- **WHEN** add runs without source arguments in an interactive terminal or automation
- **THEN** clap reports the missing arguments without prompts, layer manifest discovery, or file changes

#### Scenario: Add accepts the global interaction control

- **WHEN** valid local-source arguments are supplied with `--non-interactive` before or after add
- **THEN** add remains argument-only
- **AND** compact status styling follows the shared presentation policy independently

#### Scenario: Contact flags never start guided discovery

- **WHEN** either contact flag is supplied, with or without its required author flag
- **THEN** the invocation never starts guided input, project/identity discovery, Git/gh probes, workers, or network requests
- **AND** missing required author is a usage error rather than an interactive completion

### Requirement: Collect metadata through a guided cliclack session

Guided init MUST use cliclack to collect Name, Author, unresolved Author email,
unresolved Author URL, Description, and Layer manifest format in that order,
without input flags. Init MUST NOT combine input flags and prompt answers within
one invocation or require a final confirmation. Skipping Author MUST bypass both
contact steps. A contact supplied through recognised explicit Author shorthand
MUST skip its later prompt; no representation-choice question MUST be added.

Before the first prompt, the session MUST explain once that Enter accepts the
current selection/default or skips empty optional text. Optional metadata prompts
MUST be labelled as optional. Author, contact, and Description text inputs MUST
start empty, without helper value placeholders, discovered defaults, or editable
prefilling. The Name prompt's placeholder MUST represent its actual default value.

After any bounded preparation wait and indicator cleanup, if the frozen snapshot
contains at least one distinct name, the Author step MUST use a native cliclack
single-choice selector. It MUST list every grouped name with its source hint,
then `Enter another author`, then `Skip`. The first group with a project
declaration or developer-identity source MUST be initially selected, including a
project-declared organisation. If only GitHub-membership groups exist,
`Enter another author` MUST be initially selected instead. Enter MUST accept the
current selection. Choosing a name MUST set the author object's name and retain
that selected group's contact evidence without opening another Author prompt.
Multiple author names MUST NOT be selected or combined. Long lists MUST remain
navigable without discarding names.

Choosing `Skip` MUST omit author and proceed directly to Description, without
opening a text input or asking for confirmation. Choosing `Enter another author`
MUST open the same empty optional Author text prompt used when discovery finds
no names. An empty answer there MUST also omit author and proceed to Description,
without restoring any discovered name or returning to the selector. If discovery
finds no names, init MUST bypass the Author selector entirely and open that shared
text prompt directly. Non-empty explicit text MUST use the shared shorthand/literal
name interpreter and MUST NOT infer suggestions by matching its name back to
discovered groups. Action identity MUST be separate from candidate name text.
Omission MUST NOT depend on special input tokens or undocumented key combinations.

Before prompts or author preparation, the directory-name default MUST satisfy the
maintained schema's shared lowercase layer-name definition. An invalid default
MUST fail with guidance to supply a valid `--name`, without silently trimming,
lowercasing, or sanitising it. A valid explicit `--name` MUST bypass default
derivation and select flag-only execution rather than author discovery.

The Name prompt MUST display the schema-valid current directory's name as a
default and accept it when the user presses Enter without text. A typed name MUST
override that default and satisfy the same schema definition without silent
conversion. Any schema-invalid typed name, including a whitespace-only answer,
MUST receive field-specific inline correction feedback and another opportunity
to answer. Literal author names and description MUST remain arbitrary non-blank
strings, including Unicode and surrounding whitespace, not subject to layer-name
syntax. Author and description MUST be optional independently. An empty text
answer MUST skip the optional field, while a non-empty whitespace-only answer MUST
be rejected with inline feedback. Accepted component values MUST be preserved,
apart from recognised shorthand syntax removal; skipped fields MUST be absent
rather than empty strings or `null`. The completed Author step MUST show the
accepted name for a selected option or literal input, or the accepted explicit
shorthand when it supplied contacts, never source hints or action labels.
After an optional field is skipped through an empty answer or selector Skip,
its completed prompt MUST show a dimmed, italic `Skipped` indicator rather than a
blank answer. This is presentation only: it MUST NOT become a default or stored
value, appear before submission, or mark accepted values, cancellation, or format
choices as skips. Bypassed contact prompts MUST NOT add synthetic completed rows.

The final `Layer manifest format` question MUST be a selector with choices labelled
by the extensions `yaml`, `yml`, `toml`, `json`, and `jsonc`, without a leading dot or
`layer.` prefix, in priority order. `yaml` MUST be initially selected and accepted
by Enter. The completed prompt MUST also show the extension alone.
The selector MUST NOT allow arbitrary filenames or rename the fixed basename.
Format MUST be kept separate from metadata. No file MUST be created until all
guided answers, including format, are complete.

Guided validation feedback MUST identify the current field and explain how to
correct the answer in prompt terms, without referring to metadata flags. For an
optional field, it MUST also explain that clearing the field and pressing Enter
skips it. Guided and flag-only input MUST use the same metadata validation rules;
adapting the message MUST NOT change validation or stored values. Flag-only
validation diagnostics MUST continue to identify the relevant flag without
starting prompts. The meaning of guided feedback MUST remain clear without colour.

Prompt-specific wording MUST be derived from structured schema failures, not a
second validity check or matching human-readable validator messages. Known
non-blank failures MUST receive non-blank correction guidance; other schema rule
failures MUST retain accurate field/path and rule information, including captured
author components. Internal schema setup failures MUST stop init before prompts
or writes, without asking the user to retry valid metadata or mislabelling them
as invalid answers.

#### Scenario: Initialise through prompts

- **WHEN** an eligible terminal user runs `hydragen init` without input flags
- **THEN** cliclack asks Name, Author, unresolved Author email, unresolved Author URL, Description, and Layer manifest format in order
- **AND** only after valid answers, optional skips, and a format choice does init create the layer manifest

#### Scenario: Skip optional fields

- **WHEN** the user answers Name with a valid name and submits empty optional text or chooses selector Skip
- **THEN** that field is omitted and its completed prompt shows `Skipped`
- **AND** author omission bypasses both contact steps without preventing description, and contact omission affects only that contact

#### Scenario: Accept or override the directory-name default

- **WHEN** the Name prompt opens
- **THEN** it shows the current directory's name as the default
- **AND** Enter without text accepts that name unchanged
- **AND** typing a valid name uses that answer instead of the default

#### Scenario: One discovered name still offers visible alternatives

- **WHEN** exactly one distinct name eligible for initial selection is found and no other distinct names are found
- **THEN** the selector offers that name, Enter another author, and Skip
- **AND** the discovered name is initially selected

#### Scenario: Invalid directory default stops before author discovery

- **WHEN** guided init would otherwise be eligible but the directory-name default violates the shared name definition, including uppercase or spaces
- **THEN** init fails before prompts, author preparation, or writes with guidance to supply a valid `--name`
- **AND** it does not lowercase, trim, sanitise, or replace the directory name with a discovered author

#### Scenario: Reject an invalid guided name without changing it

- **WHEN** the directory default is valid but the developer types a layer name such as `Shared-Config` or `café`
- **THEN** schema-derived inline feedback explains how to correct Name without referring to flags
- **AND** the answer remains editable rather than silently converted, no file is written, and author preparation is not restarted

#### Scenario: Preserve arbitrary accepted author and description

- **WHEN** literal author name or description contains Unicode, uppercase, or surrounding whitespace and satisfies its non-blank schema definition alongside a valid layer name
- **THEN** the accepted component is preserved unchanged rather than subjected to layer-name restrictions
- **AND** discovery normalization does not transform explicit author or description input

#### Scenario: Accept the initially selected author

- **WHEN** the user presses Enter on the initially selected author name
- **THEN** that name becomes the author object's name and the completed step shows the accepted name rather than `Skipped`
- **AND** the next prompt is Author email, using only the selected group's evidence

#### Scenario: Choose a different discovered name

- **WHEN** several distinct names are listed and the user chooses a non-initial name
- **THEN** only the selected name becomes the author object's name and the next prompt is Author email
- **AND** neither source hints nor other groups' names or contacts are stored or offered as its metadata

#### Scenario: Skip directly from the selector

- **WHEN** the user chooses Skip in the Author selector
- **THEN** author is omitted, the completed step shows `Skipped`, and the next prompt is Description
- **AND** no author text input, contact step, or confirmation is shown

#### Scenario: Enter another author through the shared text prompt

- **WHEN** the user chooses Enter another author and types `Example team` into the next prompt
- **THEN** the shared empty optional Author input collects that literal name
- **AND** empty optional email then URL inputs follow without suggestions, and the eventual author object records that name

#### Scenario: Empty custom author still skips

- **WHEN** the user chooses Enter another author and presses Enter without text in the shared Author input
- **THEN** author is omitted, the completed input shows `Skipped`, and the next prompt is Description
- **AND** no discovered name is reapplied, contact prompt opened, or selector shown again

#### Scenario: No names means direct optional text entry

- **WHEN** no usable names are discovered
- **THEN** no Author selector appears and the shared empty optional Author input opens directly
- **AND** typing a valid name supplies author, followed by unresolved empty contact inputs, while empty Author omits it and proceeds to Description

#### Scenario: Candidate names cannot trigger selector actions

- **WHEN** a usable discovered name is literally `Skip` or `Enter another author` and the user selects that name option
- **THEN** it is stored as the selected author object's name rather than interpreted as an action

#### Scenario: Correct invalid answers

- **WHEN** any typed non-empty metadata answer is whitespace-only
- **THEN** the prompt identifies the field and explains how to supply a non-blank answer without referring to a metadata flag
- **AND** it allows another attempt and creates no file while input remains invalid or incomplete

#### Scenario: Clear an invalid optional answer to skip

- **WHEN** a whitespace-only answer is submitted for Author, Author email, Author URL, or Description
- **THEN** feedback explains supplying a non-blank answer and clearing the field then pressing Enter to skip
- **AND** clearing and submitting omits that field and shows submitted-skip feedback
- **AND** clearing Author also bypasses both contact steps

#### Scenario: Flag-only validation retains flag context

- **WHEN** a metadata flag is supplied with an empty or whitespace-only value
- **THEN** its diagnostic identifies that flag and the invocation fails without asking a replacement question
- **AND** it uses compact terminal error styling or plain captured/unattended output as appropriate
- **AND** no files are created

#### Scenario: Guided validation without colour

- **WHEN** guided input is available but terminal colour is disabled and a non-empty whitespace-only answer is submitted
- **THEN** the field and correction or optional-skip instructions remain understandable from the text
- **AND** validation, retry, and no-early-write behaviour are unchanged

#### Scenario: Author choices without colour

- **WHEN** guided input is available without terminal colour and author names are discovered
- **THEN** optional labels, grouped source hints, selection, custom entry, and Skip remain understandable without colour, including contact steps

#### Scenario: Accept the default format

- **WHEN** the user completes the metadata questions and presses Enter on the initial format selection
- **THEN** init creates `layer.yaml` if exclusive creation and writing succeed
- **AND** no format property is added to the layer manifest

#### Scenario: Choose another format without renaming the layer manifest

- **WHEN** the user chooses `yml`, `toml`, `json`, or `jsonc` from the selector
- **THEN** init creates the corresponding fixed `layer.<extension>` filename in that format if creation succeeds
- **AND** the identifying name entered earlier does not alter the filename

#### Scenario: Cancel during either author input route

- **WHEN** the user cancels in an Author/contact selector, shared optional text input, or later unfinished prompt
- **THEN** init exits unsuccessfully with the existing cancellation feedback and no layer manifest is created

### Requirement: Present concise author sources and quiet provenance labels

Active Author, email, and URL choices MUST retain native radio indicators without
added candidate or action icons. Raw labels and actions MUST start at the same
display column. Every visible candidate MUST append the same dimmed parenthesised
source-family summary whether focused or not. Focus MUST change only the radio
indicator and label styling, not row wording, suffix, spacing, or line wrapping.
No category suffix MUST replace or duplicate source summaries. Names MUST NOT be
interpreted as evidence of a person's or company's identity.

Summaries MUST derive from accepted structured sources, deduplicating families
in first-source order: `project metadata`, `Git`, `GitHub`, `local account`, and
`GitHub organisation`. Git settings/environment share `Git`, project files share
`project metadata`, and local-account/username sources share `local account`.
Contacts MUST use only records supplying that exact contact, not every source of
the name. No summary MUST be derived by parsing English hints. Project filenames,
Git keys/scopes, and account logins MUST remain out of display summaries.
Non-public GitHub hosts MUST retain `on <host>` independently per source family;
public-host matching MUST remain case-insensitive. Unsafe login/host components
MUST still be rejected. Families MUST join with comma-space, without a final `and`.

Full detailed hints, contact-field provenance, record identities, ordering,
eligibility, and provenance distinctions MUST remain internal even when display
families collapse. This rule supersedes visible full-hint references in other
author-discovery requirements only in the picker. Discovery, grouping, ordering,
selection defaults, typed actions, accepted values, omission, and cancellation
MUST remain unchanged. Actions MUST have no summary. Submitted/cancelled answers
MUST omit renderer-added suffixes. Basic and colour-disabled terminals MUST retain
the same meaningful text. Literal labels containing action text, parentheses, or
icons MUST remain unchanged candidates, without shorthand reinterpretation.

The renderer MUST preserve native scrolling and wrapping without truncation,
extra detail rows, or expansion input. Theme state MUST reset between fields and
before format selection. Unrelated prompts MUST remain unchanged.

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

#### Scenario: Contact summaries use only actual field sources

- **WHEN** multiple project files and Git settings supply the same contact
- **THEN** its summary is `project metadata, Git` while full field-level evidence remains internal
- **AND** name-only or differently valued contact sources do not appear in that summary
- **AND** distinct Enterprise hosts and profile/organisation sources remain separate

#### Scenario: Stable contact rows and native scrolling

- **WHEN** email or URL focus moves among candidates or actions at 40, 60, or 80 columns
- **THEN** every visible candidate retains identical summary wording and wrapping on and off focus, including without colour or in basic terminals
- **AND** long lists scroll natively to their final candidates and plain actions without dropping values
- **AND** submission or Escape/Ctrl-C cancellation clears source summaries from completed feedback

#### Scenario: Contact punctuation remains literal and themes reset

- **WHEN** a contact value contains action words or parentheses and is accepted or followed by custom entry
- **THEN** its typed candidate identity remains distinct and accepting it preserves its exact value
- **AND** completed answers show accepted values, original explicit shorthand, Skipped, or cancellation without added suffixes
- **AND** later contact and format selectors retain their own labels, layout, and native input

### Requirement: Discover guided author names

Guided init MUST start best-effort author discovery once in the background after
successful target preflight, schema readiness, and directory-name default
validation, before Name and without waiting for discovery to complete. It MUST
collect usable candidates from supported project author declarations,
`GIT_AUTHOR_NAME`, applicable Git `author.name` and `user.name` values, the current
local account's display/full name, and its login username. It MUST NOT stop after
the first usable candidate. If the account API cannot supply a usable login
username, it MUST also collect usable platform-appropriate environment usernames:
`LOGNAME` then `USER` on Unix, or `USERNAME` on Windows. Otherwise it MUST NOT add
environment username alternatives. Associated contacts MUST be collected only
through the supported contact-evidence requirement, independently of name validity.

Git values MUST be read for the init directory using Git's own configuration and
include handling, with their scopes and internal origins recorded. Distinct names
from applicable repository-local, global, and other active scopes MUST remain
available even when another scope overrides them. Inactive conditional includes
MUST NOT be scanned. The lookup MUST work without a repository or email address.
Presentation order MUST put project-declared authors before `GIT_AUTHOR_NAME`,
Git `author.name`, Git `user.name`, optional GitHub profile name, account display/full
name, account username, environment username fallbacks, then optional GitHub
organisation names. Project files MUST be ordered `package.json`, `Cargo.toml`,
`pyproject.toml`, with entries in declaration order. This MUST be a stable tie-break
within the project tier, not a reason to hide other project names. Organisation
names MUST be sorted by normalized name and then login. Within each Git key,
normal scope precedence MUST determine ordering, with later definitions first
within a scope.

Discovered candidates MUST have surrounding whitespace trimmed. Empty values,
invalid text encoding, and control characters, including embedded line breaks and
terminal escape sequences, MUST be rejected as discovery input. Remaining names
MUST satisfy the maintained schema's author-name definition. These discovery
checks MUST NOT transform explicit user answers. Content failures MUST discard
only that invalid field; internal schema setup failures MUST remain fatal before
prompting or writing.

Names MUST be grouped by exact equality after trimming, with one option per
distinct name. Grouping MUST NOT change case, internal whitespace, or spelling.
Each group MUST retain its first position in presentation order, all distinct
contributing sources in that order, and the underlying contact-bearing source
records. Hints MUST identify project filename/field, Git key and scope,
account-name/username source, environment variable, or GitHub identity/host as
appropriate. Source records MUST distinguish project declarations,
developer-identity sources, and GitHub membership sources without guessing entity
type from spelling. Hints MUST be safe single-line display text and MUST NOT
expose full configuration paths or unrelated account details. Source hints MUST
NOT become author metadata.

Discovery MUST NOT require Git, `gh`, authentication, or network access to complete
init. It MUST NOT initiate authentication, read prompt answers, or change identity
settings or project files. Remote requests MUST be limited to the optional GitHub
source specified below; local Git and account lookup MUST remain local. Discovery
MUST NOT infer names from email addresses, hostnames, commit history, or remote
URLs. Unavailable sources, failed reads, malformed configuration, invalid
candidates, and timeouts MUST be silent and non-fatal and MUST NOT discard names
from successful sources. If all sources are unusable, the result MUST contain no
names. Discovery MUST NOT read npm author preferences, `DEBFULLNAME`, generic
`NAME`, or Git committer identity in this increment. Git subprocesses MUST share
a 500 ms execution budget, use bounded output capture and disconnected stdin,
and be terminated and reaped on timeout. Exhausting that budget MUST skip
remaining Git subprocesses without preventing the local-account fallback.

Flag-only execution MUST NOT perform author or contact discovery. This includes
any input flag, explicit non-interactive mode, CI mode, or any non-terminal
standard stream. Omitted author MUST remain absent on that route; explicit author
input MUST use the shared interpreter and schema without consulting machine
identity. Help, version, argument-usage failures, and failed target preflight MUST
NOT perform discovery.

#### Scenario: Git author environment name leads the identity tier

- **WHEN** guided init has no usable project-declared author but has a usable `GIT_AUTHOR_NAME` and other usable identity sources
- **THEN** the trimmed environment author name is the first grouped option
- **AND** discovery also collects usable names from the other sources

#### Scenario: Repository and global names remain available

- **WHEN** `GIT_AUTHOR_NAME` and effective `author.name` are absent, and repository and global configuration supply different usable `user.name` values
- **THEN** guided init offers both names, with repository before global and their scopes identified
- **AND** applicable conditional includes and enabled worktree settings are read through Git, without scanning inactive includes

#### Scenario: Author-specific Git configuration wins without requiring email

- **WHEN** no usable `GIT_AUTHOR_NAME` exists and effective `author.name` and `user.name` differ, with no configured email
- **THEN** guided init offers both names with `author.name` before `user.name`

#### Scenario: Global configuration works before Git setup

- **WHEN** init runs outside a repository and global Git `user.name` is usable
- **THEN** guided init includes that name without creating a Git repository

#### Scenario: Collect account alternatives even when Git supplied a name

- **WHEN** Git supplies a name and both account display/full name and login username are usable
- **THEN** discovery collects all of them, with account values following Git values
- **AND** identical values are grouped rather than repeated

#### Scenario: Environment usernames are a last account fallback

- **WHEN** the account API cannot provide a usable login username
- **THEN** discovery collects usable platform-appropriate environment username values after other sources
- **AND** a usable account display/full name does not prevent this username fallback

#### Scenario: A successful account username lookup avoids environment alternatives

- **WHEN** the account API provides a usable login username and environment username variables differ
- **THEN** the account username is included and environment usernames are not consulted

#### Scenario: The same name from multiple sources is one choice

- **WHEN** Git local `user.name` supplies ` Alex Morgan ` and account display name supplies `Alex Morgan`
- **THEN** the selector contains one `Alex Morgan` option whose hint identifies both sources
- **AND** it retains its Git-priority position and repeated source labels are not duplicated
- **AND** any associated contact records remain attached to that group

#### Scenario: Distinct spellings are preserved

- **WHEN** sources supply `Alex Morgan`, `alex morgan`, and `Alex  Morgan`
- **THEN** those remain separate options rather than silently rewriting the names

#### Scenario: Reject unsafe or invalid discovered values

- **WHEN** a discovery candidate is blank after trimming, has invalid encoding or control characters, or fails its field's content validation
- **THEN** it is not displayed or accepted as a suggestion
- **AND** discovery continues without warning, retaining independently valid name/contact fields

#### Scenario: Git lookup fails or stalls

- **WHEN** Git is missing, configuration cannot be read, or lookup exceeds the shared execution budget
- **THEN** discovery silently tries the local-account fallback
- **AND** a timed-out Git child is terminated and reaped without reading terminal input
- **AND** any names already found remain available

#### Scenario: No usable identity is available

- **WHEN** all discovery sources are absent, fail, or yield unusable names
- **THEN** the Author text prompt starts empty, with no author selector, warning, or invented author
- **AND** Enter omits author and bypasses both contact steps

#### Scenario: Automation does not discover or infer an author

- **WHEN** init selects flag-only execution despite project author files, discoverable local identity, and available authenticated GitHub access
- **THEN** it performs no author/contact lookup or identity questions and no project-author reads
- **AND** it does not probe for `gh`, inspect authentication, or make network requests
- **AND** absent author stays absent and explicit author components are interpreted and validated without consulting local identity

#### Scenario: Non-executing and rejected invocations do not look up identity

- **WHEN** help, version, an argument-usage failure, or a failed initial target check stops init
- **THEN** no author or contact discovery occurs

### Requirement: Read project-declared authors before identity suggestions

Guided discovery MUST read directly declared authors from the init directory's
`package.json`, `Cargo.toml`, and `pyproject.toml`, once each, after successful target
preflight and schema readiness. These candidates MUST precede all machine/account
identity and GitHub membership suggestions. All supported files present MUST be
considered; differing declarations MUST remain choices, not cause an error.

Supported author forms MUST be limited to:

- `package.json` `author`: an object with a string `name`, or npm's documented
  person string with a name and optional `<email>` and `(URL)` components.
  Retain associated object/shorthand email and URL separately from the name.
- `Cargo.toml` `[package].authors`: each string in the authors array, with an
  optional trailing `<email>` component, retained with that entry's name.
- `pyproject.toml` `[project].authors`: each entry's string `name` and optional
  string `email`, associated only within that entry. No author URL is inferred.

Documented person-string contact syntax MUST be removed when extracting the
declared name, while valid contacts remain on its source record. Explicit
object/table name fields MUST NOT be interpreted as person-string shorthand.
Missing/blank names, email-only entries, malformed or ambiguous shorthand, and
unsupported author/name types MUST contribute no candidate. Invalid optional
contacts MUST NOT remove an independently extractable valid name or other contact.
Names MUST NOT be guessed from email addresses. Each accepted author entry MUST
be a separate candidate, never concatenated with other entries. Extracted names
and contacts MUST use their corresponding maintained schema definitions, discovery
normalization, and grouping rules. Invalid array entries MUST be skipped
independently without discarding valid entries. Source hints MUST identify the
filename and field.

Reads MUST be limited to regular non-symlink files at those exact paths in the
init directory, with a maximum of 1 MiB per file. Missing, unreadable, oversized,
invalidly encoded, or syntactically malformed files MUST be skipped silently
without changing any existing file or discarding candidates from other sources.
Discovery MUST parse the files as data directly, without requiring their package
managers or validating the entire ecosystem project. It MUST NOT execute project
configuration, build backends, package managers, or build tools; search parent,
child, or dependency projects; follow symlinks or workspace references; or resolve
dynamically generated author values.

Discovery MUST NOT read contributors, maintainers, copyright notices, npm author
preferences, `[workspace.package].authors`, inherited Cargo author tables, or
tool-specific Python author fields in this increment. Directly listed Python
author entries MUST remain usable even if a backend could extend them dynamically;
discovery MUST NOT run that backend. Existing deprecated Cargo authors declarations
MUST remain usable as suggestions without updating or warning about the project.
Project homepage, repository URL, and other project-level fields MUST NOT be
treated as author contact details.

A project-declared author MUST be eligible for initial selection regardless of
whether it names a person, team, or organisation. GitHub membership alone MUST
NOT override that project's attribution. Grouping a project name with GitHub
membership or identity sources MUST retain its project-priority position and
eligibility. The developer MUST still select a single name, enter another, or skip;
discovery MUST NOT silently write every declared author or attach all its contacts
without contact selection. Flag-only execution and early-exit paths MUST perform
no project-author discovery reads.

#### Scenario: Existing project author precedes current developer identity

- **WHEN** `package.json` declares `Example team` and Git/system/GitHub profile sources name the current developer
- **THEN** `Example team` is listed first and initially selected, with its project source identified
- **AND** the current developer's names remain available as alternatives

#### Scenario: npm author object and shorthand extract names

- **WHEN** a supported `package.json` author is an object with `name` or a person string such as `Alex Morgan <alex@example.com> (https://example.com)`
- **THEN** discovery extracts the declared name separately and retains valid contacts on that declaration's record
- **AND** missing names and email-only author data are not converted into names

#### Scenario: Cargo and Python author arrays offer separate choices

- **WHEN** `[package].authors` or `[project].authors` directly lists several usable names and optional emails
- **THEN** each distinct extracted name is offered individually in declaration order within its file
- **AND** emails stay associated with their own entries, invalid entries are skipped, valid entries remain, and names are not joined into one author

#### Scenario: Multiple project files disagree

- **WHEN** `package.json`, `Cargo.toml`, and `pyproject.toml` declare different usable authors
- **THEN** all are offered before identity suggestions, in the stable project-file order
- **AND** no declaration is hidden or reported as a conflict

#### Scenario: A project name also appears in other sources

- **WHEN** the same normalized name appears in two project files, Git, and GitHub
- **THEN** one option identifies all contributing sources and retains its earliest project position
- **AND** differing valid contacts remain alternatives rather than replacing one another

#### Scenario: A project explicitly credits an organisation

- **WHEN** a project author field declares an organisation name also returned by GitHub membership lookup
- **THEN** the grouped name remains eligible for initial selection because the project explicitly credits it
- **AND** it is not treated as a membership-only suggestion

#### Scenario: Malformed or unsafe project entries are best-effort failures

- **WHEN** one project file is missing, unreadable, oversized, malformed, invalidly encoded, a symlink, or not a regular file
- **THEN** that file contributes no author and produces no warning
- **AND** other project files and identity sources are still considered without modifying existing files

#### Scenario: Inheritance and unrelated attribution do not trigger broader discovery

- **WHEN** author data exists only in Cargo workspace inheritance, Python dynamic/tool-specific metadata, contributors/maintainers, or parent/child/dependency projects
- **THEN** discovery does not use that data or search/execute anything to resolve it
- **AND** explicitly listed supported author entries remain usable

#### Scenario: Project-author discovery never invokes project tools

- **WHEN** guided init reads project author metadata
- **THEN** no package manager, build backend, project hook, or executable configuration is run
- **AND** absent package-manager executables do not prevent direct parsing or init

### Requirement: Discover optional GitHub names through existing gh authentication

Guided discovery MUST attempt optional read-only GitHub lookup through `gh` when
the executable is available, using only its existing authentication. It MUST use
one host: a non-empty `GH_HOST` when supplied, otherwise `github.com`, passed
explicitly to each request. It MUST use the account/token selected by `gh` for
that host, without enumerating other accounts or hosts, switching accounts,
extracting credentials, inferring a host from repository remotes, or launching
login or permission-refresh commands. Prompts MUST be disabled and stdin
disconnected for child processes.

A successful authenticated-user lookup MUST contribute the profile's usable
display name, falling back to its usable login when the display name is absent or
invalid. Source hints MUST identify the actual account login and host. After a
successful profile lookup, discovery MUST attempt accessible organisation names
for that same authenticated viewer, preferring each organisation's usable display
name and otherwise its usable login. If profile authentication fails, organisation
lookup MUST be skipped. Responses for a different viewer MUST be ignored rather
than mixed into the first account's choices. The free-text profile company field
MUST NOT be treated as an organisation membership.

The existing authenticated profile request MUST retain usable declared website
and profile URL and include its response headers without another request. The
GraphQL page query MUST request the matching viewer's website/profile URL and
organisations' public email, website URL, and profile URL. It MUST request viewer
public email only when the profile response's `X-OAuth-Scopes` header advertises
`user`, `read:user`, or `user:email`. Missing, empty, or unrecognised scope evidence
MUST omit only viewer email; it MUST NOT suppress organisation discovery, prompt
for permissions, launch an authentication probe, or retry a rejected query.
Credentials without advertised classic scopes MAY therefore omit viewer email
even if the field would be accessible. REST `/user` email MUST remain excluded,
and unrequested viewer email MUST NOT be attached. Organisation lookup MUST fetch
at most 100 records per page, not
issue a separate detail request per organisation. Further pages MUST be requested
only while the shared budget remains. Usable complete records from successful
pages MUST survive later page failure/timeout. Missing, null, or invalid contacts
MUST NOT discard independently usable names or other contacts. Public viewer
email MUST be attached only after the viewer login matches the earlier profile.

Profile and organisation candidates MUST use shared normalization, schema-backed
field validation, and cross-source grouping. Hints MUST distinguish profile from
organisation and identify login/host and the actual contact source field. Tokens,
raw API responses, membership records, and unrelated profile data MUST NOT be
displayed or stored. Public contact values MUST be displayed only as their own
labelled choices, and written only after explicit selection or entry. Declared
website MUST precede profile URL within a record, with both retained when distinct.
A scheme-less/invalid website MUST be discarded independently. If a profile URL
is missing, discovery MUST use a validated selected-host/login construction or
omit it, not guess an unrelated website. API resource URLs MUST NOT be offered
as profile URLs.

All `gh` subprocesses and organisation pages MUST share the preparation attempt's
15-second lifetime and, once Author is reached, its additional 1,500 ms wait
deadline. A separate 1,500 ms timer MUST NOT expire GitHub lookup while Name is
being answered. No request or page MUST reset these deadlines. Captured output
MUST be bounded. Timeouts/cancellation MUST terminate and reap the child, with no
retry or persistent response caching. Missing `gh`, missing/expired credentials,
offline operation, API errors, rate limits, insufficient permissions, and token/SSO
policy limitations MUST be silent and non-fatal. They MUST NOT discard usable
local or previously validated GitHub candidates or trigger authentication/permission
questions. An empty organisation result MUST NOT be reported as proof that the
user has no memberships. Flag-only execution MUST NOT attempt any of this lookup.

A group supported solely by GitHub membership MUST NOT be the initial author
selection. Membership MUST NOT be treated as evidence of this layer's authorship
or ownership. A group also supported by a project declaration or developer-identity
source MUST remain eligible on that independent basis, including project-declared
organisations. Only the selected name and explicitly accepted contacts MUST be
written as an author object; account/membership records and hints MUST NOT be
persisted. Discovery MUST NOT enumerate private email addresses, call GitHub's
email-list API, request new permissions, construct noreply email addresses, scrape
websites, or infer email/website from names or domains.

#### Scenario: GitHub profile and Git author differ

- **WHEN** the active GitHub profile has a usable name different from the local Git author
- **THEN** both names are offered, with Git before GitHub and each source identified
- **AND** no Git setting or GitHub profile is changed

#### Scenario: GitHub profile name falls back to login

- **WHEN** the authenticated profile has no usable display name but has a usable login
- **THEN** the login is offered as the personal candidate with account and host identified
- **AND** its contacts remain associated with that record, not a guessed display-name alias

#### Scenario: GitHub and local names are grouped together

- **WHEN** the same normalized name is supplied by Git and the authenticated GitHub profile
- **THEN** it appears once with both source hints, retaining its Git-priority position and both sources' contact alternatives

#### Scenario: Accessible organisations are explicit alternatives

- **WHEN** organisation lookup returns accessible memberships
- **THEN** each usable display name or login fallback appears after project and identity choices with an organisation/login/host hint
- **AND** free-text company is not added as an organisation and contacts stay associated with their returned organisation

#### Scenario: Only GitHub membership names are available

- **WHEN** discovery finds usable organisation names but no project-declared or developer-identity candidate
- **THEN** the selector still lists those names, Enter another author, and Skip
- **AND** Enter another author is initially selected, not an organisation

#### Scenario: A name has both personal and organisation sources

- **WHEN** an organisation name is identical after normalization to a developer-identity candidate
- **THEN** they are one choice with both sources identified and associated contacts retained
- **AND** it remains eligible for initial selection because of its developer-identity source

#### Scenario: One host and credential selection are honoured

- **WHEN** `GH_HOST` selects a host and `gh` has multiple configured accounts or hosts
- **THEN** requests explicitly target only that host using its currently selected credentials
- **AND** hints identify the returned profile rather than assuming Git identity or querying other accounts

#### Scenario: Default host is explicit

- **WHEN** `GH_HOST` is unset or empty
- **THEN** optional requests explicitly target `github.com` without inferring another host from Git remotes

#### Scenario: Viewer identity changes during lookup

- **WHEN** a GraphQL response identifies a viewer different from the earlier profile response
- **THEN** its contacts and organisations are ignored rather than mixing accounts
- **AND** earlier valid candidates are preserved

#### Scenario: Missing gh, authentication, or connectivity

- **WHEN** `gh` is unavailable, unauthenticated, has expired credentials, or cannot reach GitHub
- **THEN** init silently retains local choices or uses the shared empty Author input when none exist
- **AND** no login, permission request, warning, or authentication retry occurs

#### Scenario: Organisation access is restricted

- **WHEN** profile lookup succeeds but organisation lookup fails, lacks permissions, or returns no memberships
- **THEN** profile and local choices remain usable without requesting extra permissions
- **AND** no claim is made that the user belongs to no organisations

#### Scenario: Membership permission does not include viewer email permission

- **WHEN** existing authentication permits the parent organisation query but its profile response advertises no `user`, `read:user`, or `user:email` scope
- **THEN** discovery omits viewer email from the query and still offers accessible organisation names and their valid public contacts
- **AND** profile websites/URLs and other sources' emails remain available, with no private REST email, new requests, query retries, authentication changes, or permission prompts

#### Scenario: Token scope evidence is unavailable

- **WHEN** the profile response has no usable `X-OAuth-Scopes` header, including credentials that do not advertise classic scopes
- **THEN** discovery conservatively omits viewer email while retaining profile names/URLs and attempting organisation lookup
- **AND** accepted-scope headers and body text are not treated as evidence of granted permissions

#### Scenario: Existing token explicitly permits public viewer email

- **WHEN** the existing profile response advertises `user`, `read:user`, or `user:email`
- **THEN** the same paged query also requests public viewer email and attaches a valid value only to the matching earlier profile
- **AND** no additional request or permission change occurs

#### Scenario: A later organisation page fails

- **WHEN** earlier pages supplied usable records and a later page fails or exceeds the shared deadline
- **THEN** earlier names and validated contacts remain available and paging stops without retries
- **AND** the budget is not reset and no per-organisation detail requests are made

#### Scenario: A gh process stalls

- **WHEN** a `gh` process reaches the effective preparation or Author-wait deadline
- **THEN** it is terminated and reaped, no further GitHub processes run, and earlier usable records remain available

#### Scenario: Select an organisation without persisting its membership data

- **WHEN** the developer selects an organisation and completes its optional contact steps
- **THEN** the author object contains only the selected name and accepted contacts, without login/host hints or membership metadata
- **AND** name selection proceeds to Author email, not directly to Description

#### Scenario: Public email and alternative URLs

- **WHEN** a selected GitHub source has an accessible public email, a valid website, and a distinct valid profile URL
- **THEN** its email is offered and both URLs appear with website before profile URL
- **AND** no contact is written merely because the author name was selected

#### Scenario: Public email is absent

- **WHEN** a GitHub source has no usable public email but may have private account emails
- **THEN** it contributes no email suggestion, without email-list requests or permission prompts
- **AND** URL suggestions, other sources' email choices, and empty custom entry remain available

## ADDED Requirements

### Requirement: Select optional author contacts from associated source records

After a discovered Author group is selected, each unresolved contact step MUST
use only that group's frozen records. Distinct emails and URLs MUST be grouped
independently by exact equality after discovery trimming, retaining all distinct
actual contact-source hints in stable existing source/declaration order. Grouping
MUST NOT case-fold emails, normalise URI spelling, or merge different name groups.
Equal names with different contacts MUST retain every distinct valid choice.
The email decision MUST NOT filter the group's URL choices.

A field with suggestions MUST use a native cliclack single-choice selector labelled
`Author email (optional)` or `Author URL (optional)`. It MUST list all suggestions,
then `Enter another email` or `Enter another URL`, then `Skip`, initially selecting
the first suggestion even when only one exists. Accepting a suggestion MUST proceed
without a second text/confirmation step. Custom entry MUST open a shared empty
optional text input without default, suggestions, or return to the selector.
With no suggestions, that text input MUST open directly. Empty submission or Skip
MUST omit only that contact. Invalid input MUST receive schema-derived correction
and optional clear-and-Enter guidance; it MUST NOT be silently trimmed or omitted.

Custom author input MUST have no inferred contact suggestions even if its name
equals a discovered name. Contacts already captured from explicit shorthand MUST
be considered supplied and bypassed. Author omission MUST bypass both contact
steps. Cancellation at any displayed step MUST cancel init with its established
single nonzero/no-files-changed report, not become an optional skip.

Contact selectors MUST use the author selector's stable compact comma-separated
source-family summaries on every visible candidate, focused or not, without added
icons or changed wrapping during navigation. Summaries MUST derive only from
structured records supplying that exact contact; full actual field-level hints
MUST remain internal. Native scrolling, wrapping, basic-terminal text, and colour
controls MUST remain intact, without truncation or separate detail rows.
Completed steps MUST show accepted values or `Skipped`, without added suffixes or
action labels. Typed actions and scoped theme state MUST prevent candidate text
from triggering actions or changing later fields/format presentation.

#### Scenario: Equal name with multiple contacts

- **WHEN** project, Git, and GitHub records share an author name but supply different emails and URLs
- **THEN** one author name choice leads to all its distinct valid emails and URLs in their respective steps
- **AND** repeated equal contact values combine only their actual contact-source hints

#### Scenario: One contact suggestion

- **WHEN** one email or URL suggestion is available for the selected group
- **THEN** its selector still offers the suggestion, custom entry, and Skip
- **AND** Enter explicitly accepts that initial suggestion

#### Scenario: Custom name matches a suggestion

- **WHEN** Enter another author is chosen and a discovered name is typed manually
- **THEN** no inferred contact choices/defaults are restored
- **AND** only unresolved empty optional contact inputs appear

#### Scenario: Contact omission and custom entry are independent

- **WHEN** email is skipped or supplied through custom entry for a selected discovered author
- **THEN** URL still offers all the selected name group's valid URL choices
- **AND** empty custom contact submission omits only that contact without reopening its selector

#### Scenario: Missing or invalid contacts degrade independently

- **WHEN** some records have no contact fields or an invalid contact and no valid suggestions remain for a field
- **THEN** that field uses the empty optional text input without warning or invented default
- **AND** independently valid names and the other field's suggestions remain usable

#### Scenario: Contact selectors preserve presentation boundaries

- **WHEN** a contact selector is active, completed, cancelled, scrolled, or followed by another selector
- **THEN** every visible candidate shows the same compact summary on and off focus, using only its actual structured contact evidence
- **AND** completed values contain no sources/icons, action text is not stored, and later selectors retain their proper presentation

### Requirement: Associate contact evidence conservatively within the existing preparation

Supported contact evidence MUST be limited to same-declaration npm email/URL,
same-entry Cargo/Python email, paired `GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`, matching
Git configuration name/email, and the selected GitHub host/account/organisation's
public profile contacts. System account and environment username sources MUST
remain name-only. Generic `EMAIL`, committer email, OS address books, commit
history, remotes, project URLs, and guessed `username@host` MUST NOT be used.

Git MUST retain NUL-framed key/scope/origin data from its own include-aware query.
A name/email association MUST be made only within the same scope, origin, and
`author` or `user` key family when the bucket has exactly one distinct usable
name. Every distinct valid email in that bucket MUST be retained; repeated equal
names MUST remain unambiguous. A bucket with distinct names MUST retain those
name choices without attaching its emails. Different scopes/origins/families or
environment/configuration records MUST NOT be zipped or cross-paired. Origins
MUST stay internal rather than appear as full filesystem paths in source hints.

Discovered contacts MUST be trimmed, checked for safe display, and validated with
their maintained nested definitions. Invalid contact input MUST discard only that
contact; an internal schema setup failure MUST remain fatal. Name/contact data
MUST be published within the existing bounded preparation and frozen together
before Author. Existing 15-second lifetime, at most 1,500 ms additional Author
wait, 500 ms Git budget, output bounds, independent local/GitHub lanes, foreground
input/render ownership, cancellation, and child cleanup MUST remain unchanged.
Email and URL steps MUST NOT initiate lookups, workers, spinners, or additional
wait allowances. Late results MUST NOT alter displayed or future contact choices.

Written contacts MUST require explicit entry or selection. Usage guidance MUST
explain that locally declared email may be private and written contacts may become
public when sharing the layer manifest. Source records, credentials, and membership
details MUST NOT be persisted as author metadata.

#### Scenario: Git work and personal identities

- **WHEN** local and global Git configurations each contain the same name with a different associated email
- **THEN** the grouped name retains both email alternatives in normal source-priority order
- **AND** a differently named configuration's email is not attached to it

#### Scenario: Git association is ambiguous

- **WHEN** one scope/origin/key-family bucket contains different names and one or more emails
- **THEN** all usable names remain available without that bucket's email suggestions
- **AND** other unambiguous buckets still contribute their associated emails

#### Scenario: Git values are split across unrelated origins or families

- **WHEN** a name and email occur only in different scopes, origins, or author/user families
- **THEN** discovery does not manufacture a pair from those independent values
- **AND** the name remains usable with optional empty contact entry

#### Scenario: Environment and system account boundaries

- **WHEN** `GIT_AUTHOR_NAME` and `GIT_AUTHOR_EMAIL` are explicitly available
- **THEN** their record can contribute that email
- **AND** a system-only name, generic `EMAIL`, or email-only environment value does not trigger guessed contact association

#### Scenario: Contacts freeze with names

- **WHEN** preparation reaches completion or either existing deadline
- **THEN** names and their already validated contacts freeze in one snapshot and pending owned children enter cleanup
- **AND** Email/URL introduce no new lookup or wait and ignore later contact results

#### Scenario: Responsive Name and bounded cleanup remain intact

- **WHEN** enriched GitHub/contact discovery is delayed or cancelled while Name or Author wait is active
- **THEN** Name remains promptly available, only the actual Author wait shows the existing loading indicator, and the established deadlines still apply
- **AND** cancellation cleans up without unbounded foreground joining, late output, new prompts, or file creation
