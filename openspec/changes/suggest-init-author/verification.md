# Verification: guided init author suggestions

## Historical snapshot and newer integration status

The original assessment below records the 2026-10-20 review of runtime
[`734ce99`](https://github.com/hydratic-applications/hydragen/tree/734ce9977ce1cdb2c66991b58f6c91c3fc1f0014)
and the subsequent documentation sync committed in `11da971`. Its test counts,
scenario inventory, preservation comparisons and line references belong to those
snapshots, not the newer integration with main
`328644ec72012e11ae01ffbf7213694ebd40e7be` now in progress. Relative source links
still open the current checkout; use the pinned runtime snapshot for the cited
historical line numbers. These references are not claims about current line
locations or fresh tests.

The newer integration retains main's already-agreed lowercase layer-name policy
and shared diagnostics alongside author suggestions. Historical evidence that
Name accepted whitespace or Unicode does not describe the current name contract:
directory defaults and typed names must satisfy the maintained name definition
without conversion. Author and description keep their arbitrary non-blank string
rules. The reconciled guided-collection specification adds three scenarios for
invalid defaults, invalid guided names and preserved optional metadata; the
historical 70-scenario inventory below has not been rewritten as new evidence.

### Current integration validation

The orchestrator's completed validation hooks and accepted independent reviews
cover the working merge candidate `HEAD 11da971` plus main
`328644ec72012e11ae01ffbf7213694ebd40e7be`, with the reviewed conflict resolutions.
No runtime edits followed those tests. These are fresh candidate results, not a
reuse of the historical 288-test result; this documentation update did not rerun
the runtime suites.

| Check | Current candidate result |
| --- | --- |
| `cargo build --locked`, `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings` | All passed. |
| `cargo test --locked` | **673 passed, 0 failed, 3 ignored** across 23 nonempty suites; 0 doctests. Passed tests comprise 112 library, 207 binary and 354 integration tests. The binary suite has 210 entries, including the two #115 deferrals and child-only driver. Relevant passing subsets include 46 init, 47 interactive-init and 15 author-route tests, plus 12 active controlled-terminal tests within the binary suite. |
| `cargo test --locked --bin hydragen long_membership -- --ignored` | **0 passed, 2 expected visibility failures in 0.09 seconds**; correct returned metadata and no fixture failure. |
| Coverage-policy tests | **39 passed**, using the existing diff-cover 10.6.0 virtual environment and canonical `TMPDIR`; no installation. These are policy checks, not measured coverage. |
| Strict OpenSpec validation | All **9 main specs** and this change passed. The aggregate run reported **12 passed, 1 failed**, solely the pre-existing `add-validate-command` change with no delta specs. |
| Independent integration reviews | Runtime and test/document/index reviews accepted with no blockers. **Final publication review of the updated report remains pending**; no push or PR is claimed. |

Tasks 3.3/4.5 remain open for the accepted #115 exception, with **25/27 complete**.
The direct equal-Git/valid-GitHub-profile grouping fixture gap remains an open
project follow-up, not a newly demonstrated runtime defect. Full conformance and
archive readiness are not established. Windows and actual hosted Linux coverage
remain unverified until the corresponding platform/CI evidence is available.

Documentation checks on this in-progress integration passed strict validation for
the reconciled `cli-interaction` main spec and `suggest-init-author` change, plus
scoped whitespace checks. Read-only comparison with both parents retained all
requirement and scenario headings: 20 unrelated main requirement blocks and all
4 feature-only author requirement blocks are verbatim. The reconciled main spec
has 25 requirements and 127 scenarios; all 5 interaction delta blocks match it
exactly, and a second sync changes nothing. These are document checks, not runtime
or independent integration-review results.

### Current integration requirement inventory

The current change has **8 delta requirements and 76 named scenarios** across
three capabilities: 5 interaction requirements / 69 scenarios, 2 experience-
principle requirements / 4 scenarios, and 1 modified initialisation requirement /
3 scenarios. This replaces the historical 7-requirement/70-scenario inventory only
for the current integration; it does not change what was tested at `734ce99`.
The initialisation delta explicitly supersedes the former blanket Git-command
prohibition with the already-agreed eligible-guided, optional read-only lookup
exception. All other initialisation requirements remain unchanged.

The three name/optional-metadata reconciliation scenarios now have direct passing
evidence in the current suite:

- Invalid directory defaults: `invalid_guided_directory_defaults_stop_before_discovery_without_rewriting`
  in `src/main.rs`, and `invalid_directory_defaults_fail_before_guided_prompts_and_allow_explicit_overrides`
  in `tests/interactive_init.rs`.
- Invalid guided names: `guided_name_policy_errors_explain_correction_without_flags`
  in `src/interaction.rs`, and `invalid_guided_name_rules_explain_correction_and_retry_before_any_write`
  in `tests/interactive_init.rs`.
- Preserved arbitrary author/description: `flag_metadata_preserves_arbitrary_author_and_description_with_a_valid_name`
  in `src/main.rs`, plus the optional-field terminal cases in
  `tests/interactive_init.rs`, including Unicode acceptance without normalisation.

The existing initialisation evidence routes below also ran in the current full
suite. Source inspection supplements them; the local-Git/offline scenario retains
its composed-evidence qualification rather than claiming a dedicated end-to-end
test:

| Initialisation scenario | Existing evidence route and qualification |
| --- | --- |
| Run in an offline non-interactive environment | `runs_without_stdin_git_or_service_configuration` in `tests/init.rs` clears the child environment and supplies an empty tool path. The captured route tests in `tests/author_discovery_routes.rs` check input flags, non-interactive/CI/redirection and early-exit canaries. This scenario is preserved verbatim. |
| Guided init can suggest a local Git author while offline | `real_git_honours_scopes_includes_conditions_worktrees_and_command_values` in `src/author_discovery/identity.rs` exercises local scoped/include configuration and lookup outside a repository. Production `git_arguments` constructs only a configuration read. Together with the existing guided selector tests, this is composed evidence, not a dedicated offline end-to-end Git-author selection test. Its test-only Git setup is not an init runtime action. |
| Guided init works without Git or GitHub access | `missing_git_and_gh_still_allow_empty_custom_author_without_restoring_suggestions` in `tests/interactive_init.rs` removes both fixture executables, exercises custom omission and completes init without warning or extra project files. Other sources may still offer names; this is not a claim that missing tools force an empty snapshot. |

After this reconciliation, strict validation passed for all 9 main specs and the
change. Read-only checks confirmed exact alignment and second-sync idempotence
for all 8 current delta requirements. Initialisation remains at 9 requirements
and grows from 46 to 48 scenarios; its other 8 requirement blocks, Purpose and
existing offline scenario are preserved verbatim. Scoped whitespace checks passed.

The current validation results above supersede the earlier pending-runtime status,
not the historical evidence below. Runtime validation and the scoped independent
integration reviews are complete; final publication review of this updated report
remains pending. The #115 exception, open fixture follow-up and platform/coverage
limits remain explicit.

## Assessment and scope

Reviewed on 2026-10-20 against integrated runtime HEAD
`734ce9977ce1cdb2c66991b58f6c91c3fc1f0014` (`734ce99`), whose parents are the
feature checkpoint `41fed19` and main `4a82827`.

**The useful feature has an accepted landing limitation; this change is not ready
to archive.** Two tasks remain incomplete for the upstream native selector's
long membership-only display defect. Those are **CRITICAL before archive**, not
new blockers for the broader feature landing accepted by Hydragen's maintainer.
The intended requirements have not been relaxed.

| Dimension | Result |
| --- | --- |
| Completeness | **25/27 tasks complete**; 3.3 and 4.5 remain open for their precise upstream-dependent portions. All **7 delta requirements** have implementation or documentation evidence. |
| Correctness | All **70 named scenarios** inventoried: 66 interaction scenarios and 4 documentation/review scenarios. Evidence includes active tests, composed unit/integration checks and document inspection, not 70 direct passing tests. Long membership-only variants still fail; one exact Git/profile grouping fixture is missing. |
| Coherence | Architecture, schema authority, automation boundaries and foundations align with the design. The explicitly accepted native display exception remains unresolved. Platform and coverage limits are qualified below. |

Inputs were `AGENTS.md`, `FOUNDATIONS.md`, `openspec/design-questions.md`, all of
this change's proposal, design, delta specifications and tasks, and the actual
source/test bodies referenced below. `openspec status` identified the
`spec-driven` workflow; `openspec instructions apply` independently reported
25 complete and 2 remaining tasks. Planning-artifact completeness is not runtime
conformance or permission to archive.

**Final synchronised-snapshot reconciliation is complete.** Both final main
specifications and the final design/tasks follow-up notes were reread. Read-only
comparison against HEAD and the deltas confirms exact matching for all 7 affected
requirement blocks, verbatim preservation of all 7 unrelated blocks, unchanged
Purpose sections and an idempotent sync (applying the deltas again changes
nothing). `cli-interaction` grows from 4 to 8 requirements and 24 to 81 scenarios;
all 24 previous scenario headings remain, with 22 bodies verbatim and 2 modified
exactly as required by the delta. `cli-experience-principles` grows from 4 to 6
requirements and 6 to 10 scenarios. The final notes accurately retain 25/27 tasks,
the accepted exception, Hydragen #14/upstream #115 and open proposed PR #116, and
the integrated validation/platform limits. The orchestrator and independent
reviewer also accepted sync preservation and idempotence. No runtime, test or
dependency changes were made for this verification.

## Findings by severity

### CRITICAL before archive: 2 outstanding task portions

1. **Task 3.3: initial selection visibility and related completed feedback in long
   membership-only lists are incomplete.**
   [`src/interaction.rs:532–579`](../../../src/interaction.rs#L532) correctly
   selects the typed `EnterAnother` action when no group is eligible, after all
   names, with native `max_rows(7)`. With at least seven membership-only groups,
   that initial action is outside cliclack 0.5.6's first visible window. Moving Up
   can leave the newly selected name and its accepted summary invisible too.
   This violates the intended discoverable selection/completed-name experience,
   not the returned author value. Publication cutoff, freezing, grouping, typed
   actions and ordinary scrolling are already implemented; do not describe all
   of task 3.3 as absent.

2. **Task 4.5: the two long membership-only visibility regressions do not pass.**
   [`src/interaction/author_tests.rs:598–645`](../../../src/interaction/author_tests.rs#L598)
   retains the original assertions in
   `long_membership_only_selector_visibly_selects_custom_and_enter_opens_empty_input`
   and
   `moving_up_from_long_membership_custom_visibly_selects_and_summarises_the_name`.
   Only their issue-specific ignore attributes defer them. The first proves empty
   author still returns correctly before failing initial visibility; the second
   proves `Membership 19` is returned before failing selection/summary visibility.
   Other selector and terminal fixture work in task 4.5 is implemented and active.

Reproduce both known failures without real account/network discovery:

```sh
cargo test --locked --bin hydragen long_membership -- --ignored
```

The integrated-tree run reached both intended visibility assertions: **0 passed,
2 failed in 0.08 seconds**, with correct values and no setup/cleanup failure.
This is one upstream defect reflected in two incomplete tasks, not two newly
discovered production defects. The third ignored entry,
`interaction::author_tests::terminal_driver`, is a child-only harness invoked by
active controlled-terminal tests; it is not another deferred feature scenario.

Follow-up: [Hydragen #14](https://github.com/hydratic-applications/hydragen/issues/14)
tracks adoption of an upstream fix for
[cliclack #115](https://github.com/fadeevab/cliclack/issues/115).
[Upstream PR #116](https://github.com/fadeevab/cliclack/pull/116) is an **open
proposal**, not an adopted or released fix. `Cargo.lock:233–234` still selects
unmodified cliclack 0.5.6. Once an appropriate upstream fix is available and
adopted, remove both issue-specific ignore markers, pass both regressions, repeat
full validation, and only then complete 3.3/4.5 and reassess archive readiness.
Do not vendor, fork, patch, reorder actions or add a runtime workaround to declare
these tasks complete. The [accepted exception](design.md#accept-the-native-selector-limitation-pending-an-upstream-fix)
explicitly permits useful feature landing without claiming these scenarios pass.

### WARNING: 1 direct scenario-test gap

**The exact “GitHub and local names are grouped together” fixture is absent.**
The code uses the correct common path: valid profiles publish `GithubProfile`
records in `src/author_discovery/github.rs:81–90`, and exact grouping/source order
is applied by `src/author_discovery/mod.rs:220–255`. Existing tests exercise
Git/environment/account grouping (`identity.rs:160`), project/account/membership
grouping (`mod.rs:283`), and project/organisation grouping (`github.rs:202`), but
none supplies the same normalized name as both a Git value and a valid GitHub
profile and asserts their combined hints and Git-priority position.

Recommendation: add that deterministic unit/fake-`gh` fixture in a later test-only
follow-up, asserting a single choice, both distinct hints, Git-priority position
and eligibility. This is a **test-evidence gap**, not a demonstrated runtime bug,
not a reason to undo accepted landing, and not additional runtime work in this
documentation-only verification. The inventory below marks it as indirect rather
than claiming direct complete coverage.

### Known validation limits, not newly found implementation defects

- **Linux coverage remains unmeasured here.** Passing coverage-policy tests and
  synthetic profile-forwarding proofs do not establish actual total/changed-line
  coverage. Run the integrated candidate through the Linux workflow and assess
  both applicable **80% gates** before claiming those checks pass. No thresholds
  or exclusions were changed. See
  [`.github/workflows/coverage.yml:81–161`](../../../.github/workflows/coverage.yml#L81).
- **Windows compilation and runtime remain unverified.** Target-specific Job
  Objects, reparse-point file handling and exact console-mode restoration exist,
  but the macOS evidence is not Windows evidence. Compile and exercise those
  branches before making a Windows-validation claim.
- Deadlines bound awaiting optional results, accepted publication and ordinary
  subprocess work, not arbitrary kernel/account-service latency. Non-interruptible
  calls stay isolated in bounded-count workers; escaped Unix process groups and
  stalled kernel operations are explicitly qualified in the design. Do not turn
  the 15-second/1,500 ms/500 ms values into absolute wall-clock guarantees for
  every OS operation or terminal redraw.
- The separate production stdin-before-reader `/dev/tty` fallback issue is not
  claimed fixed by the test-harness input-loss changes.
- **Repository-wide OpenSpec validation is not wholly green.** The final aggregate
  strict run reported 11 passed and 1 failed: the unrelated, pre-existing
  `add-validate-command` change has no delta specifications. Its strict JSON
  diagnostic confirms the missing-delta error; `git ls-tree` confirms HEAD already
  contains only its `.openspec.yaml` and `proposal.md`. All eight main
  specifications and this change passed strict validation. The unrelated planning
  artifact was not changed or treated as a new defect in this implementation.

No additional concrete production defect or code-pattern suggestion was found in
this review. The missing direct fixture above is the only new finding; the two
archive-critical task portions and the validation limits were already known.

## Completeness: requirement inventory

There are **4 ADDED + 1 MODIFIED** interaction requirements and **2 ADDED**
experience-principle requirements: **7**, not 6. No requirement is wholly missing.
“Evidence found” below is not a claim of unqualified full conformance.

| ID | Delta requirement | Implementation/documentation evidence and qualification |
| --- | --- | --- |
| I1 | ADDED: Discover guided author names | `src/main.rs:140–204`; `src/author_discovery/identity.rs:9–151`; `mod.rs:128–255`. Project-first ordering, all-source collection, exact grouping, eligibility, maintained validation and silent source failures are implemented. Startup-boundary tests supplement executable canaries. 14 scenarios below. |
| I2 | ADDED: Prepare author suggestions during earlier input and bound visible waiting | `src/author_discovery/preparation.rs:11–255`; `process.rs:85–203`; `src/interaction/author_wait.rs:10–57,105–228`. Independent lanes, two clocks, foreground cancellation, atomic cutoff and owned cleanup are implemented with the platform bounds above. 10 scenarios below. |
| I3 | ADDED: Read project-declared authors before identity suggestions | `src/author_discovery/project.rs:13–212`; maintained fixtures under `tests/fixtures/author-discovery/projects/`. Fixed local files, bounded safe opens and direct JSON/TOML extraction; no tool execution or inherited/dynamic resolution. 9 scenarios below. |
| I4 | ADDED: Discover optional GitHub names through existing gh authentication | `src/author_discovery/github.rs:9–168`; `process.rs:92–106`. Explicit selected host, existing credentials, profile/login fallback, viewer-bound batched pagination, shared deadlines and retained partial records. Membership is not authorship; the membership-only selector display exception still applies. 14 scenarios below, including the direct fixture gap. |
| I5 | MODIFIED: Collect metadata through a guided cliclack session | `src/interaction.rs:352–423,484–619`; `src/lib.rs:224–270`. Typed native selection, shared optional text, direct Skip, unchanged accepted values, prompt order, schema feedback and delayed exclusive write. **Partially conformant** for accepted-name feedback/visible selection in long membership-only lists. 19 scenarios below. |
| P1 | ADDED: Record safe preparation and visible responsiveness in existing values | `FOUNDATIONS.md:90–113,152–164,481–509` preserves all eight values, separate limits, safe preparation and the practical init example without embedding concrete runtime budgets in the values. 2 reader-review scenarios below. |
| P2 | ADDED: Reinforce responsive preparation in CLI implementation and review guidance | `AGENTS.md:177–213,215–337` retains automation/safety/output rules and requires early useful input, independent bounds, one terminal owner, stable choices and evidence beyond animation/total duration. 2 guidance/review scenarios below. |

Task arithmetic is objective: sections 1/2/3/4/5 contain 6/4/5/8/4 tasks, with
6/4/4/7/4 complete. Checked tasks 5.3 and 5.4 record completed scoped validation
and review, not completion of deferred behaviour or all platform/coverage checks.

## Correctness: code-path checks

- **Source/schema authority:**
  [`mod.rs:187–210`](../../../src/author_discovery/mod.rs#L187) trims only
  discovered names, rejects unsafe display text and calls
  `schema::validate_field("author", ...)`. Content failures discard a candidate;
  setup failures remain errors. The field validator references the maintained
  schema's property (`src/layer_manifest/schema.rs:13,210–214,264–276`), not a
  copied author constraint. Explicit input preserves its string. Complete metadata
  is validated again before encoding/writing (`src/lib.rs:24–40,238–244`).
  Source/test searches found no OpenSpec asset loads; embedded schemas point to
  `schemas/layer-manifest.schema.json`.
- **No lookup in automation/early exits:**
  [`src/main.rs:140–204`](../../../src/main.rs#L140) checks routing/preflight/
  readiness/default before guided-only startup; the flag route takes the explicit
  author unchanged and leaves omission as `None`. `Mode::select` checks all three
  streams, CI and overrides. `src/main.rs:328–458` injects the *entire* startup
  boundary, covering project/account work that tool canaries cannot observe.
  Captured-route tests explicitly acknowledge that their canaries alone do not
  count arbitrary file or account API reads.
- **Process/deadline ownership:**
  [`preparation.rs:69–131,171–255`](../../../src/author_discovery/preparation.rs#L69)
  uses one short state lock, two lanes and at most two process reservations.
  Validation and OS calls run outside the lock. Publication rechecks cutoff after
  validation; freeze closes collection before grouping and does not wait for
  cleanup. Author gets a one-time fresh allowance, clamped to the original
  lifetime; Git's guard starts at lookup. `process.rs:92–203` disconnects stdin,
  captures stdout/stderr separately at 1 MiB plus a detection byte each, uses
  groups/Jobs, polls without blocking child waits and joins only finished readers.
  Cleanup acknowledgement/drain is bounded at 100 ms; stalled OS work stays in an
  isolated lane, not a foreground join or a new stream of helper threads.
- **Foreground waiting/freezing:**
  [`author_wait.rs:10–57`](../../../src/interaction/author_wait.rs#L10) skips
  indicators for ready/expired results, polls Escape/Ctrl-C only between prompts,
  clears progress and restores terminal state before the next step. `TERM=dumb`
  gets the same temporary meaningful text because the native renderer hides
  animation. Operational errors are not converted to deliberate cancellation.
  The frozen vector is the sole selector input; late results cannot alter it.
- **Typed selector/custom/Skip:**
  [`src/interaction.rs:20–42,149–235,532–579`](../../../src/interaction.rs#L532)
  separates candidate indices from actions. Candidate names matching action labels
  remain names. Both no-name and custom routes call the same `optional_input`;
  empty input maps to `None`. Skip bypasses that input. Scoped theme state hides
  action transitions, strips hints from normal accepted summaries and resets before
  the format selector. It cannot repair upstream filtering of off-screen rows.
- **Write paths remain separate:**
  `src/main.rs:180–186` calls `create_init` only after the entire collector returns,
  including format. [`src/lib.rs:162–172,222–270`](../../../src/lib.rs#L222)
  retains initial all-format refusal, exact-target `create_new`, full validation,
  write-error/partial-file semantics and best-effort duplicate observation. There
  is no late all-format lock/refusal. Tests preserve other files, alternative-format
  races and post-write reporting semantics. Only `Metadata` name/author/description
  is passed to creation, not credentials, source records or memberships.

### Scenario evidence key

The tables retain **every exact scenario title**. A reference such as `ID:160`
means the test beginning at line 160 in the linked file below (or the specified
code/document range). Several tests exercise multiple conditions; conversely,
composed evidence does not mean one direct end-to-end test for every sentence.
The inventory is not a coverage percentage or a claim that every variant passes.

| Key | Maintained source/test file |
| --- | --- |
| ID | [`src/author_discovery/identity.rs`](../../../src/author_discovery/identity.rs) |
| MD | [`src/author_discovery/mod.rs`](../../../src/author_discovery/mod.rs) |
| PJ | [`src/author_discovery/project.rs`](../../../src/author_discovery/project.rs) |
| GH | [`src/author_discovery/github.rs`](../../../src/author_discovery/github.rs) |
| PR | [`src/author_discovery/preparation.rs`](../../../src/author_discovery/preparation.rs) |
| PX | [`src/author_discovery/process.rs`](../../../src/author_discovery/process.rs) |
| MN | [`src/main.rs`](../../../src/main.rs) |
| UI | [`src/interaction.rs`](../../../src/interaction.rs) |
| CT | [`src/interaction/author_tests.rs`](../../../src/interaction/author_tests.rs), controlled production-function pseudo-terminal (PTY) tests with injected sources |
| PT | [`tests/interactive_init.rs`](../../../tests/interactive_init.rs), real CLI executable in isolated PTYs |
| RT | [`tests/author_discovery_routes.rs`](../../../tests/author_discovery_routes.rs), captured-route tests |

### I1: Discover guided author names — 14 scenarios

| Scenario | Evidence / qualification |
| --- | --- |
| Git author environment name leads the identity tier | ID:160 `author_environment_is_collected_before_all_git_and_account_alternatives`. |
| Repository and global names remain available | ID:298, real scoped Git/include/worktree fixture; ID:270 tests later definitions. |
| Author-specific Git configuration wins without requiring email | ID:298 asserts author-key precedence without configuring email. |
| Global configuration works before Git setup | ID:298 also queries outside the fixture repository and checks global alternatives. |
| Collect account alternatives even when Git supplied a name | ID:160,217 collect both account fields after Git and assert source hints. |
| Environment usernames are a last account fallback | ID:241 tests unusable API username despite usable real name and platform fallback order. Windows branch is not executed on this host. |
| A successful account username lookup avoids environment alternatives | ID:217 panics on any environment read; ID:160 also counts environment access. |
| The same name from multiple sources is one choice | ID:160; MD:283 assert one trimmed value and distinct ordered source hints. |
| Distinct spellings are preserved | MD:263 checks unchanged case/internal whitespace; PJ:391 keeps differently cased names separate; exact grouping code MD:220–255. |
| Reject unsafe or invalid discovered values | MD:263,283; ID:270 invalid UTF-8/control records; PJ:432; central content/setup paths MD:194–210 and PR:321. |
| Git lookup fails or stalls | PX:231,262,380 cover missing/failing/flood/stall/reap; ID:378 proves Git-guard expiry leaves account fallback usable. |
| No usable identity is available | ID:263; CT:535 `empty` route proves direct optional input, empty/valid/retry/cancel, no selector or invented author. |
| Automation does not discover or infer an author | MN:328 observes all startup; RT:376,405,418,431,452,478 and PT:1869,1890,1956,2219 cover flags/CI/redirection, preserved explicit values and omission. |
| Non-executing and rejected invocations do not look up identity | MN:382 covers parse/preflight/schema/default startup boundaries; RT:515,555,578,595,615 covers real executable early exits. |

### I2: Preparation and visible waiting — 10 scenarios

| Scenario | Evidence / qualification |
| --- | --- |
| Results are ready while Name is being answered | CT:648 proves early usable Name/retry with no spinner and released-source immediate Author; CT:738 clears an actual wait on completion. |
| Fast Name entry leaves a short residual wait | PR:276 deterministic fresh allowance; CT:673 measures fresh visible wait after a background-clock offset. The illustrative 200 ms is not a separate literal fixture. |
| Longer background preparation is useful rather than prematurely timed out | PR:276 and GH:320 keep work active at 3,000 ms; CT:648 covers completion during Name and no Author wait. The 3-second/4-second example is composed evidence, not a four-second sleep test. |
| The preparation lifetime expires before Author | PR:306; PR:700 verifies actual owned-child lifetime cleanup; CT:701 proves no restarted lookup/indicator after expiry during Name. |
| Partial results survive a visible-wait timeout | PR:577 retains profile/first page while reaping a stalled later page; CT:673 retains early author and rejects late publication; CT:759 keeps Author usable after child cleanup. |
| Source completion races with the cutoff | PR:414 blocks validation across close; PR:276 and CT:673 reject late records without changing choices. |
| Name validation retry does not restart preparation | CT:648 retries Name while one fixture source remains pending; same collector/preparation handle UI:501–504; deadlines PR:276,454. |
| Cancel Name while lookups are active | CT:718 loops Name/wait and Escape/Ctrl-C, checks one report, no Author/files and reaped process; PR:700 covers Cancel/Drop. |
| Cancel during the visible loading indicator | CT:718,829; cancellation is observed in the foreground and no fallback prompt follows. |
| Disabled colour or basic terminal feedback | CT:829 tests `xterm-256color`/`dumb` without colour; CT:774 tests operational EIO/bare Interrupted, indicator clearing and exact terminal restoration rather than false cancellation. |

### I3: Project-declared authors — 9 scenarios

| Scenario | Evidence / qualification |
| --- | --- |
| Existing project author precedes current developer identity | PT:1290 proves default/project-first ordering without changing directory-derived Name; ID:160 and MD ordering cover the remaining identity tiers. |
| npm author object and shorthand extract names | PJ:282,291,303,323,432 cover explicit names, contact removal, punctuation, malformed/email-only input and unsupported shapes. |
| Cargo and Python author arrays offer separate choices | PJ:370 asserts individual ordered valid entries and independent invalid-entry omission. |
| Multiple project files disagree | PJ:391 and PT:1339 assert fixed file order, all names retained and no project-file changes. |
| A project name also appears in other sources | MD:283, GH:202, PT:1339 compose project/account/membership and project/environment grouping with ordered hints. |
| A project explicitly credits an organisation | MD:283; GH:202; PT:1290 verifies a project organisation matching membership remains the initial name. |
| Malformed or unsafe project entries are best-effort failures | PJ:454,466,529,549,576,592,603,612,633,658 cover parse/encoding/size/read/permission/nonregular/symlink cases and retained files/names. Permission omission is asserted only when the runner actually lacks read permission. |
| Inheritance and unrelated attribution do not trigger broader discovery | PJ:420,426,466,490 reject unsupported inheritance/fields/traversal while retaining directly listed dynamic-Python authors. |
| Project-author discovery never invokes project tools | PJ:490 uses executable canaries and unsupported backend declarations; direct collector PJ:13–160 has no command invocation. |

### I4: Optional GitHub names — 14 scenarios

| Scenario | Evidence / qualification |
| --- | --- |
| GitHub profile and Git author differ | PT:1403 uses distinct fixture Git author/profile and verifies identity-default provenance plus organisation selection; GH:202 checks actual returned profile hints; priority is explicit in MD:128–140. Composed, not a dedicated pairwise-order fixture. |
| GitHub profile name falls back to login | GH:202 supplies unsafe display name and asserts usable login/profile hint. |
| GitHub and local names are grouped together | **Indirect only; WARNING above.** MD:220–255 is shared exact grouping, but no direct equal Git/valid-profile fixture was found. |
| Accessible organisations are explicit alternatives | GH:202 tests display/login fallback, sorted names, membership-only eligibility, ignored company, 100-item query and no per-org detail calls; PT:1403 selects an organisation. |
| Only GitHub membership names are available | CT:585 and UI:1025 prove correct custom default for a short list. **Long-list visible selection/feedback variants fail** in the two deferred CT:600,622 tests, despite correct values. |
| A name has both personal and organisation sources | MD:283 publishes account plus membership (and project) into one eligible group with all hints; source-independent eligibility is MD:237–249. |
| One host and credential selection are honoured | GH:202 fake executable demands explicit enterprise host, disabled prompts and inherited synthetic selected credential; PT:1403 checks fixture-only environment/requests. No real credential/account probing. |
| Default host is explicit | GH:369 checks unset/empty host; PT:1403 asserts both requests explicitly target `github.com`. |
| Viewer identity changes during lookup | GH:264 retains earlier profile/page but discards the changed-viewer page. |
| Missing gh, authentication, or connectivity | GH:298 tests nonzero profile/no org follow-up; PX:231 tests unavailable processes; PT:1641 tests absent tools/offline custom omission. Authentication/network causes share generic failure handling rather than separate real-service fixtures. |
| Organisation access is restricted | GH:264 nonzero later response retains prior names; GH:104–128 code rejects errors/handles empty nodes without warnings. Permissions/empty membership variants are supported by the same path, not exhaustively status-specific fixtures. |
| A later organisation page fails | GH:264 tests later nonzero/viewer change; PR:577 tests actual stalled later page under Author cutoff with earlier complete names retained. |
| A gh process stalls | GH:320 tests lifetime cutoff without a launch-time 1.5-second expiry; PR:577 and PR:700 verify Author/lifetime/cancel child reaping and no next page. |
| Select an organisation without persisting its membership data | PT:1403 asserts only selected name in actual layer manifest, next Description, stripped summary hints and isolated credentials. |

### I5: Guided metadata collection — 19 scenarios

| Scenario | Evidence / qualification |
| --- | --- |
| Initialise through prompts | PT:1185,1811 validate order, actual outputs, values and no file before all answers/format. |
| Skip optional fields | PT:1230,1573 check independent omission and submitted `Skipped`; CT:535 covers empty text on both entry routes. |
| Accept or override the directory-name default | PT:1212 accepts directory default unchanged; PT:1185 and CT:648 override it, preserving whitespace/Unicode. |
| One discovered name still offers visible alternatives | CT:535 `custom` fixture publishes exactly one candidate, traverses native custom action and exercises text; UI:539–579 constructs both actions and initial candidate. |
| Accept the initially selected author | PT:1290,1604 verify accepted name/next Description and metadata; CT:648 verifies ready author. **Long membership-only Up-selected summaries remain the explicit exception**, not an ordinary eligible default. |
| Choose a different discovered name | PT:1339 selects Python alternative; PT:1403 selects organisation; PT:1495 selects last name after native scrolling. Only that name is stored. |
| Skip directly from the selector | PT:1573 asserts one completed Author question, `Skipped`, next Description and omitted author despite action-like names. |
| Enter another author through the shared text prompt | CT:535 tests `custom` and `empty` via the same production input function; PT:1185 writes custom author unchanged. |
| Empty custom author still skips | CT:535 and PT:1641 assert omission, next Description and no reapplied suggestion. |
| No names means direct optional text entry | CT:535 `empty` asserts no selector; typed and empty answers both proceed to Description. Driver result `null` represents `None` for inspection, not layer manifest encoding; actual omission uses `Metadata` serialization. |
| Candidate names cannot trigger selector actions | PT:1541 and UI:1044 distinguish literal `Skip`/`Enter another author` from typed actions. |
| Correct invalid answers | PT:1702,1785 exercise inline validation/retry and no early write; UI:790,810 preserve setup-versus-content and generic rule diagnostics. |
| Clear an invalid optional answer to skip | CT:535 covers both Author entry paths; PT:1702 also covers independent Description clear/skip and persisted omission. |
| Flag-only validation retains flag context | RT:478; PT:1930; UI:894 assert flag diagnostics and no replacement prompts/files. |
| Guided validation without colour | PT:1702 iterates colour on/off with the same correction/skip instructions and write safety. |
| Author choices without colour | PT:1604 checks labels, hints, actions, accepted value and no styling on colour-disabled/basic terminals. **Does not make long membership-only initial visibility conformant.** |
| Accept the default format | PT:1212 and PT:1811 accept initial YAML, check fixed filename and no format metadata. |
| Choose another format without renaming the layer manifest | PT:1811 loops all five formats and asserts extension-only labels/order, fixed basename and independent content. |
| Cancel during either author input route | PT:1991 selector/later prompt cancellation; PT:1672 custom text; CT:535 both no-name/custom text and Escape/Ctrl-C. Nonzero, one report and no layer manifest. |

### P1/P2: Documentation and review — 4 scenarios

These are reader/design-review checks, not automated UI acceptance tests.

| Scenario | Evidence / qualification |
| --- | --- |
| A first-time reader understands the responsiveness principle | `FOUNDATIONS.md:90–113,152–164`: safe preparation versus unconfirmed decisions, separate background/visible limits, eight named values retained. Read for meaning and practical consequences, not only keywords. |
| The init example explains the practical benefit | `FOUNDATIONS.md:481–509`: Name overlaps author suggestions, a bounded indicated/cancellable pause when needed, stable choices, intended rather than benchmark claims. |
| A later CLI change considers preparation | `AGENTS.md:197–213`: consider read-only overlap after checks, do not require a universal framework, preserve confirmation/automation/permissions and clean up. |
| Review distinguishes responsiveness from animation | `AGENTS.md:179–185,209–213`; historical controlled-terminal observations plus current CT:648,673,701,718,738,759,774,829 check early input, visible waiting, restoration and cleanup. No spinner-only or total-duration-only inference. |

## Coherence and developer experience

The implementation follows the design's separation: source discovery has no
terminal or creation policy, the preparation handle owns lifetime/publication,
foreground waiting owns temporary event input, native cliclack owns actual
prompts, and library creation retains schema/filesystem safety. Focused
`whoami`/`process-wrap`/`crossterm` helpers and target-specific APIs match the
documented approach without an async framework or dependency fork. Temporary
test injection exists only in unit-test builds, not as a production discovery
disable or permissions bypass.

The foundations' DX and performance values support the change: less repeated
typing without implicit attribution in automation; preparation overlaps Name
without moving network delay to startup; source hints explain choices without
becoming metadata. Questions 9 and 11 are resolved only for this increment's
interaction and bounds, not all future agent interfaces/performance targets.
No ninth value or glossary term is needed to accommodate the implementation.
The display defect conflicts with the intended experience and remains an explicit
exception, rather than a reason to weaken the foundations.

Historical macOS controlling-terminal review observed early Name/no competing
spinner, ready project suggestions, hint-free accepted metadata, meaningful
colour-disabled hierarchy and compact completion; a stalled fake lookup was
cancellable with one report and owned-child cleanup. The recorded 56/26 ms Name
and 25 ms cancellation timings are individual historical observations, not new
measurements, benchmarks or guarantees. They support judgements about beauty,
enjoyment, function and ease of use together with current automated PTYs. This
verification did **not** freshly manually exercise every scenario. Native hints,
explicit custom/Skip actions, clean summaries and meaningful plain wait feedback
support the four DX goals; invisible selected rows demonstrably do not.

## Validation provenance and inspected files

The following integrated-tree validation was already executed and independently
reviewed before the `734ce99` merge commit. This documentation-only review reuses
that evidence; it did not duplicate the full suite or run real account/network
discovery. The ignored `.slim/deepwork/suggest-init-author.md` and
`.slim/deepwork/suggest-init-author-landing-review.md` contain the local historical
review/command provenance; this tracked report preserves its material outcomes
without requiring those local files to understand the assessment.

| Check | Integrated evidence |
| --- | --- |
| `cargo build --locked` | Passed. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --locked --all-targets -- -D warnings` | Passed. |
| `cargo test --locked` | **288 passed, 0 failed, 3 ignored**: 26 library + 104 binary + 15 author routes + 42 init + 7 lifecycle + 39 interactive + 31 layer manifest + 6 schema + 11 YAML integer + 7 YAML scalar; 0 doctests. Two ignored upstream regressions plus one child driver, not an all-scenarios-green claim. |
| Explicit deferred command above | **0 passed, 2 expected visibility failures, 0.08 s**; correct metadata, no harness failure. Earlier 0.07 s run was pre-integration evidence, not this run. |
| `python3 -m unittest discover -s tests/coverage -v` with explicit existing `DIFF_COVER` and canonical `TMPDIR` | **39 passed**, including real diff-cover integration and exact 80% policy boundary tests. First unconfigured run failed for host tool-path and macOS `/var` versus `/private/var` aliasing. Correctly configured rerun used existing diff-cover **10.6.0**, without installs or source fixes. Policy tests are not measured runtime coverage. |
| Strict OpenSpec validation and whitespace checks | Passed on the integrated tree. After final document reconciliation, strict validation was repeated successfully for this change and both affected main specs; local links, all 70 scenario titles, exact delta matching, preservation, sync idempotence and whitespace checks passed. |
| Final wider OpenSpec validation | The orchestrator's `openspec validate --specs --strict --no-interactive` passed all 8 main specs. Its `openspec validate --all --strict --no-interactive` reported **11 passed, 1 failed**, solely for the pre-existing unrelated `add-validate-command` missing-delta error. A separate strict JSON diagnostic and HEAD tree inspection corroborated that cause; no unrelated repair was made. |

Final documentation checks repeated in this verification:

```sh
openspec validate suggest-init-author --type change --strict --no-interactive
openspec validate cli-interaction --type spec --strict --no-interactive
openspec validate cli-experience-principles --type spec --strict --no-interactive
```

All three passed. Read-only consistency checks confirmed each of the 70 delta
scenario titles appears exactly once in this report and exists in the final main
specs, all local file links resolve, all 7 delta blocks match the final main specs,
all unrelated blocks and Purpose sections are preserved, sync is idempotent, and
only tasks 3.3/4.5 remain unchecked. The report was reread from a first-time
reader's perspective. Full Rust/Python suites were not rerun for these
documentation-only changes; the integrated evidence above remains applicable.

Credential-isolated subprocess fixtures deliberately forward only
`LLVM_PROFILE_FILE` unchanged, including non-UTF-8 bytes, empty/absent values and
`%p`/`%m` placeholders (`tests/author_discovery_routes.rs:168–268`,
`tests/support/terminal_identity.rs:68–207`). Synthetic instrumented children
prove profiles are written outside init targets, not actual coverage percentages.
The PTY harness retains absolute `/bin/stty`, queued-key handling, dedicated-stdin
readiness and silent descriptor-close fixes from main. Active input-loss tests
cover Name, Author selector/text and format without claiming a production
fallback fix.

**15 Rust source/test files were inspected directly:** `src/main.rs`,
`src/interaction.rs`, all six `src/author_discovery/{mod,project,identity,github,preparation,process}.rs`
files, `src/interaction/{author_wait,author_tests}.rs`, `src/lib.rs`,
`src/layer_manifest/schema.rs`, `tests/author_discovery_routes.rs`,
`tests/interactive_init.rs`, and `tests/support/terminal_identity.rs`.
Supporting inspection included the maintained schema, Cargo dependency/lock
entries, coverage workflow, README, foundations/guidance and all change artifacts.
Source/test reference searches and focused status/count commands were read-only.

**Final assessment:** 2 critical outstanding task portions must be resolved before
archive; 1 direct scenario-test gap should receive a focused test-only follow-up.
The accepted broader landing scope is unchanged, and final synchronised-document
reconciliation and scoped strict validation are complete. The unrelated
repository-wide missing-delta failure remains disclosed; Linux coverage and
Windows are not certified. Do not describe this change as fully conformant or
ready to archive.
