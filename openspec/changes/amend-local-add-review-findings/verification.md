# Review amendment verification

## Evidence boundary

The parent accepted every source gate, including harness ownership remediation,
and the focused documentation review. The amended-tree Rust/Python runs and
scheduled actual-terminal review passed as recorded below. Package contents and
the fresh offline copy's successful build/571-test result were verified.
Independent final local review accepted tasks 5.5 and 6.4 for delivery without
source changes or additional tests. At that pre-delivery acceptance, only task
6.5's delivery work remained; this record is not a push or publication announcement.
No Linux, Windows, root, hosted-CI or LLVM coverage execution is claimed here.
Retained local evidence is identified by short artifact names; those artifacts are
not build/test inputs or required repository files.

## Accepted integration prerequisite

`c3b3c8a` preserves the exact user cancellation wording; ordinary merge `3ea1fcb`
combines it with inspected main `6ccf96b`. Independent review accepted 544 Rust
tests (110 library, 129 binary, 305 integration), 39 Python coverage-policy tests,
formatting, all-target/all-feature warning-denied clippy and diff checks. Relevant
artifacts are `pr7-phase1-rust-tests-commit.log`, `pr7-phase1-python-tests.log` and
`pr7-phase1-terminal-evidence/`. These are baseline counts, not amended-tree totals.

## Accepted YAML source gate — R2, R4 and qualified R15

### Implementation and preserved boundaries

- [Scanner-tail/source-facts checks](../../../src/layer_manifest/edit/yaml.rs#L394-L507)
  inspect actual terminal scalars, including explicit scalar keys with implicit
  null values. Checked content prefixes do not require unwrapping an optional
  indentation hint. Demonstrated-safe strip/header-only/terminated/clip controls,
  byte-preserving refusal and no-op staging bypass remain covered.
- [Flow bounds and insertion planning](../../../src/layer_manifest/edit/yaml.rs#L510-L659)
  separate delimiter lookup from layout, retain inline/footer attachment and
  permit bounded comma/item insertions without reformatting compact collections.
- [Independent insertion helper](../../../tests/support/insertion_oracle.rs#L1-L43),
  [exact flow fixtures](../../../tests/dependency_edit.rs#L844-L1079) and
  [source-sensitive EOF fixtures](../../../tests/yaml_field_order.rs#L310-L458)
  independently specify permitted offsets/text, reconstruct original bytes and
  reject comment/literal/unrelated-field corruption. The test oracle does not
  impose a single contiguous insertion on YAML, JSONC or TOML.
- The maintained decoder, shared schema and intended decoded-append proof remain.
  **The loader's EOF block-string normalisation is unchanged and outside scope.**
  The edit guard must not be described as repairing decoder conformance.

### Failing-first evidence and accepted reruns

R2 unsafe-tail cases and R4 exact footer/close layout failed before correction;
see `pr7-yaml-r2-failing-first.log` and `pr7-yaml-r4-failing-first.log`.
The accepted corrected scope is **87 tests**: 39 dependency-edit, 18 field-order,
11 YAML field-order and 19 YAML unit tests. The corresponding suite commands are:

```sh
cargo test --locked --test dependency_edit --test field_order --test yaml_field_order
cargo test --locked --lib layer_manifest::edit::yaml::tests
```

Retained corrected evidence: `pr7-yaml-null-fixed-targeted-tests.log`,
`pr7-yaml-null-fixed-unit-tests.log` and `pr7-yaml-null-fixed-clippy.log`.
Targeted formatting/clippy/whitespace checks passed in the YAML lane; the later
combined checks are recorded separately below. This documentation pass did not
rerun Rust or PTY suites while other lanes were active.

### Independent review blocker and correction

Independent actual-CLI review found two invalid implicit-null flow-root variants:
a bare comma inserted immediately after `custom:` produced `custom:,`, although
the maintained decoder and an erroneous expected fixture accepted it. The fixture
was corrected first and failed before the production fix. The
[value-indicator branch](../../../src/layer_manifest/edit/yaml.rs#L562-L582) now
inserts ` ,`, producing separated `custom: ,` while preserving original bytes.
The [permanent eight-case regression](../../../tests/dependency_edit.rs#L1009-L1033)
checks both extensions, LF/CRLF, and with/without an inline comment.

The standalone checker invoked actual `hydragen add ./dep` and compared complete
expected source and independently decoded Psych data, including retained null.
Its retained invocation form is:

```sh
ruby pr7-yaml-null-separation.rb "$PWD/target/debug/hydragen"
```

The scratch checker is local review evidence, not a new repository/runtime/test
dependency. Eight of eight variants passed (Ruby 3.2, Psych 5.0.1, libyaml 0.2.5),
and the independent reviewer reran all eight against the actual CLI before
accepting tasks 2.1–2.6. Evidence: `pr7-yaml-implicit-null-failing-first.log`,
`pr7-yaml-null-separation-20261019-85683-vak2o8/results.json` and
`pr7-yaml-oracle-null-recheck-t22auwro/results.json`. The prior blocker is resolved;
its detection demonstrates why exact-source fixtures also need independent
validity review, not permission to weaken the permitted-insertion oracle.

## Accepted diagnostic source review — R3, R5 and R6

- [Shared presentation](../../../src/presentation.rs#L126-L150) now emits genuinely
  empty separators; [family assertions](../../../tests/diagnostic_body_support/mod.rs#L13-L57)
  reject indentation-only gaps rather than normalising them away. Five masking
  normalisers were removed without trimming nonempty data.
- [Selected-entry adaptation](../../../src/diagnostics/add_verification.rs#L486-L496)
  states regular-file requirements neutrally, retaining actionable paths and the
  current/dependency symlink and no-fallback distinctions.
- [Metadata adaptation](../../../src/diagnostics/content.rs#L50-L76) and
  [library Display](../../../src/layer_manifest.rs#L232-L244) require strings
  without inventing a null cause; omission advice remains conditional. Public
  `NotString` shape and schema acceptance are unchanged.
- [Parsed-type regression](../../../src/diagnostics/add_verification.rs#L1386-L1460)
  covers 96 parsed type/field/format combinations, each rendered for three source
  origins, not 96 separate Rust tests.
  TOML does not acquire null syntax. Library Display and contextual CLI rendering
  remain distinct audiences; no R14 refactor was introduced.

Five focused unit regressions failed first. Accepted corrected results comprise
**81 unit tests**: presentation 9, diagnostic body 3, usage errors 19,
add-verification 39 and operations 11. The corresponding scopes are:

```sh
cargo test --locked --bin hydragen presentation::tests
cargo test --locked --bin hydragen diagnostic_body::tests
cargo test --locked --bin hydragen usage_errors::tests
cargo test --locked --bin hydragen diagnostics::add_verification::tests
cargo test --locked --bin hydragen diagnostics::operations::tests
cargo test --locked --test usage_errors --test diagnostic_safety -- --test-threads=1
```

The final command passed **24 captured tests** (15 usage, 9 security). Retained
unit logs use `pr7-diagnostics-passing-*.log`; five failing-first logs use
`pr7-diagnostics-failing-first-*.log`. Seven direct CLI correction captures are in
`pr7-diagnostics-captured-corrections.json`. That JSON does not itself contain
filesystem snapshots; unchanged-state evidence also comes from unit/source
assertions and the worker's checks, not an invented JSON field.
Harness-owned consumer reconciliation and scheduled actual-terminal review have
subsequently passed; their evidence is recorded below rather than attributed to
this earlier source gate.

## Accepted environment and reporting fixtures — R7, R8 and R12

Controlled probes support temporary socket reader retention when a concurrent
sibling inherits the peer during pre-exec spawning, even with close-on-exec set.
They do not identify the exact sibling from the original baseline failure and
found no persistent post-exec leak. Isolated no-reader controls returned EPIPE;
controlled retained-reader init/add/no-op controls accepted completion writes.
This explains the fixture's unsupported no-reader assumption without weakening
production completion or no-op assertions.

Independent truly closed-stdout controls reproduce the separate runtime behaviour
that can silently accept an unavailable stream (EBADF at the OS boundary). This
is not the same as the socket fixture transient. The original sibling identity
remains unknown; the controlled mechanism is a supported explanation, not a
reconstruction of that original process. No production EPIPE suppression or new
production output policy was introduced.

### Width and permission prerequisites

- The [copied-help fixture](../../../tests/init.rs#L544-L552) removes inherited
  `COLUMNS`, leaving intentional width tests unchanged. The inherited
  `COLUMNS=30` control failed before the targeted correction and passed afterward:
  `pr7-phase4-width-before.log` and `pr7-phase4-width-after.log`.
- [Permission support](../../../tests/permission_support/mod.rs#L11-L139) probes
  real read/create denial before snapshots and distinguishes ready,
  unsupported-with-reason and setup-error across the three fixtures/five consumers.
  [Helper regressions](../../../tests/fixture_harness.rs#L97-L158) cover visible
  synthetic unsupported classification, non-skip setup errors, unwind restoration
  and probe cleanup. Synthetic unsupported evidence is not a root execution claim.
  Final terminal logs contain native read/create-denial labels, establishing that
  the current host exercised actual denial rather than silently skipping it.

### Deterministic broken-pipe fixture and ownership remediation

The [output helper](../../../tests/output_failure_support/mod.rs#L117-L234)
retains its direct shell child as an unreaped group anchor until the final signal.
The only `try_wait` is inside guarded reaping after `SignalsFinished`; the lifecycle
cannot reopen group signalling. Cleanup/Drop never send late cached group signals.
The [shell protocol](../../../tests/output_failure_support/mod.rs#L340-L407)
distinguishes killable reader ownership from wait-only bookkeeping and records
the actual CLI completion status, not the deliberately killed anchor's status.
One overall deadline bounds setup, execution, capture and cleanup. If the supervisor
is killed, orphan reaping remains an explicit operating-system dependency and a
bounded cleanup failure rather than an unconditional guarantee.

Independent review blocked the earlier reap-before-final-signal helper because
PID/PGID reuse could target an unrelated group. The accepted remediation has
failing-first lifecycle ordering coverage, eight helper tests independently rerun,
67 captured tests, ten batches of five reporting tests and five batches of three
cleanup stress cases, all passing without forced PID-reuse experiments. Evidence:
`pr7-phase4-anchor-order-before.log`, `pr7-phase4-anchor-helpers-final.log`,
`pr7-phase4-anchor-captured.log`, `pr7-phase4-anchor-clippy.log` and
`pr7-phase4-anchor-safe-stress.json`. The corresponding captured/helper scopes are:

```sh
cargo test --locked --test fixture_harness
cargo test --locked --test add --test init --test operation_diagnostics
```

Live-reader negative controls require successful complete reports; true
no-reader cases require ordinary failure status and truthful created/updated/
unchanged state. Independent closed-descriptor controls retain the runtime
acceptance limitation. Original bytes/inode/mtime assertions and no-staging
expectations were not weakened. Test-only BrokenPipe simulation uses portable
`ErrorKind`, with native OS-error preservation covered separately.

The approved policy remains unchanged: completion write/flush errors returned by
the established fallible path are fatal and retain created/updated/unchanged
state. Warning/error diagnostic delivery is best effort, with no new propagation
or status changes. No descriptor prechecks, terminal reopening or rollback.

## Accepted documentation and explicit main-spec alignment

Independent review approved tasks 5.1–5.4: focused README example/colour/reporting
and replacement guidance, concrete Unreleased compatibility notes, the diagnostic
reference, replacement module docs and relevant design-question links. No new
foundations belief or glossary term was needed. The per-finding dispositions now
incorporate accepted source/harness and parent runtime evidence. Task 5.5's
documentation passed final oracle review for task 5.5; this is not a task-checkbox
or delivery action by the documentation lane.

The parent explicitly invoked alignment after the source gates. The result is
six added requirements and three modified requirements across CLI interaction,
dependency addition and initialisation. All 35 unmentioned requirements' contents
and scenarios are unchanged, original requirement order/introduction
text is retained, modified requirements retain all their scenario headings, and
there are no duplicate requirement/scenario headings. No removal or rename was
applied. The change remains active and unarchived; tasks/plan checkboxes are owned
by the parent.

Alignment validation passed: `openspec validate --specs --strict` (7 specifications)
and `openspec validate amend-local-add-review-findings --strict`. A structural
comparison against the pre-alignment requirements verified the counts and
preservation above, including exact agreement with every added/modified delta.
Scoped `git diff --check` passed. Local link/heading/source-line-range and
whitespace/final-newline checks cover the aligned specs and this phase's evidence
documents; they are separate from the runtime and packaging evidence below.

## Parent amended-tree validation

### Rust and Python

The parent ran `cargo test --locked --quiet`: **571 Rust test functions passed,
zero failed or ignored** — 111 library, 133 binary and 327 integration tests.
Evidence: `pr7-final-rust-tests.log`. The four interactive binaries contain 13 add,
35 init, 2 operational and 8 usage test functions (58 total), but include pure
fixture classifiers. **58 is not an actual-PTY-test count.**

The final Python coverage-policy run passed **39 tests**, using isolated
diff-cover 10.6.0 with explicit `DIFF_COVER`, a physical `TMPDIR` and
`PYTHONDONTWRITEBYTECODE=1`; see `pr7-final-python-tests.log`. The command scope is
`python3 -m unittest discover -s tests/coverage`. These are actual final-run
results, not extrapolations from the integration baseline.

The parent also passed `cargo fmt --check`,
`cargo clippy --all-targets --all-features -- -D warnings`, strict validation of all
seven main specs and this change, and diff checks.
No coverage threshold, workflow or security policy was relaxed.

### Scheduled actual-terminal review

After the full run, the parent reran the three diagnostic terminal binaries
sequentially, then one guided retry/completion/cancellation test:

```sh
cargo test --locked --test interactive_add --test interactive_operation_diagnostics --test interactive_usage_errors -- --test-threads=1 --show-output
cargo test --locked --test interactive_init guided_rejection_retry_completion_and_cancellation_keep_their_interaction_at_review_widths -- --test-threads=1 --show-output
```

The first command passed 23 test functions (13 + 2 + 8); the second passed one.
Evidence: `pr7-final-terminal-tests.log`, `pr7-final-guided-terminal-tests.log` and
`pr7-final-terminal-evidence/` with **2,414 actual transcript evidence files**.
Those files are artifacts, not a claim of 2,414 distinct test functions. Coverage
includes widths 40/80/100/120, styled/plain output, CI, explicit colour disables
and mixed redirection. Native read/create-denial labels are visible in the terminal
log. Guided header, rejection/retry, completion and cancellation were observed.

The parent's representative reading included 40/80-column non-regular directory
errors, numeric metadata without null attribution, plain missing-current-layer
feedback, ANSI invalid-format feedback and guided interaction. Independent review
accepted task 3.3's actual-terminal and maintained-spec assessment. The following are
**qualitative DX judgments**, informed by that reading rather than proven by
snapshot/test success alone:

- **Beauty:** Consistent headline colour, normal-foreground detail and truly empty
  block gaps produce a calm hierarchy across related failures. Long absolute paths
  remain visually dense and can wrap naturally in narrow terminals; no additional
  diagnostic-wrapping feature was selected.
- **Enjoyment:** One-shot output avoids unnecessary branding/session ceremony,
  while the guided identity appears once and editable rejection/cancellation feel
  deliberate rather than like duplicate command errors.
- **Functionality:** Concrete causes, paths, allowed choices and actual mutation
  state answer what failed without null misattribution or false no-change claims.
  Captured/terminal state assertions support this judgment without guaranteeing
  output receipt on an unavailable stream.
- **Ease of use:** Familiar blocks and cause-specific tips make correction easier
  to find; guided retry preserves editable input, and cancellation communicates the
  no-change outcome once. The remaining narrow-path density is a limitation, not
  grounds for silently shortening actionable paths or adding a new formatter.

## Verified package and fresh offline copy

`cargo package --locked --offline --allow-dirty` completed with exit zero. The
parent inspected the package's 250 files and verified inclusion of the maintained
schema, insertion/permission/output test helpers and `fixture_harness` tests.
`pr7-final-package.log` records only the existing Cargo warning about missing
repository/homepage/documentation metadata; this is not a packaging failure or
authority for an optional metadata change. No release was published.

The fresh source copy excludes `.git/`, `openspec/`, `target/`, `.slim/` and
`__pycache__/`. It used a fresh Cargo target directory, disabled incremental
compilation and cleared compiler wrappers; original build outputs were absent.
Both commands completed with exit zero:

```sh
cargo build --locked --offline --quiet
cargo test --locked --offline --quiet
```

The fresh run passed all 571 Rust test functions again, with zero failed or ignored.
The parent compared 57 source/test/schema/Cargo/script file hashes: all match the
checkout exactly. Evidence is `pr7-final-fresh-record.json`,
`pr7-final-fresh-build-tests.log` and `pr7-final-validated-source-hashes.json`.
This establishes the tested copy's independence from OpenSpec and old outputs,
not a claim of execution on untested platforms or of release publication.

## Final local acceptance and delivery boundary

The final oracle accepted tasks 5.5 and 6.4 for delivery, independently checking
all 57 file hashes against checkout/fresh sources, both 571-test logs, the 39 Python
tests, all 250 crate files and required assets, and matching `Cargo.toml.orig`.
Schema, dependencies and coverage policy are unchanged versus `3ea1fcb`; no hidden
API change or further source/test work was required for acceptance.

At pre-delivery acceptance only parent-owned task 6.5 remained. Its commits, push
and PR-description update are tracked separately by that task and the PR head;
local review acceptance does not itself assert they happened. No PR merge,
automatic archive, global installation or publication is authorised by this record.

### Delivery and later upstream changes

The approved amendments were committed as `417c426` (YAML), `fe1cf74`
(diagnostics and fixtures) and `8c6d96b` (documentation and specifications), then
ordinary-pushed to [PR #7](https://github.com/hydratic-applications/hydragen/pull/7).
Its description was replaced with the current scope, validation and limitations.
The change remains active and unarchived, and the PR remains open.

During that original delivery, main advanced to `4a82827`, including dependency/
schema updates and coverage-change archival. Those changes were not part of the
then-validated `6ccf96b` integration. GitHub reported a merge conflict and a
checkout-free check identified `Cargo.toml`. This is a historical observation,
not a current source blocker: the separately authorised integration and renewed
validation below resolve it. Local validation does not establish the pushed PR's
current mergeability or hosted check results.

## Current-main integration after delivery

On 2026-10-19, following the owner's separate approval to integrate current main
and commit/push once complete, ordinary merge
`7a1008852be1fc770a461b75c5c288fe3781cafe` combined branch `8af225b` with fixed
inspected main `4a8282776c605df51be3040647da7243f1d4da59`. Independent merged-source
review accepted this tree. Earlier 544/571-test and 250-file package observations
above retain their original boundaries; the renewed measurements below describe
this merge, not a global replacement of historical evidence.

- **Dependency/schema compatibility:** The resolved `Cargo.toml` retains branch
  `serde-saphyr` comment support and runtime `tempfile`/`toml_edit`, while taking
  upstream `jsonschema` 0.59.0 and `sha2` 0.11.1. The lockfile is the exact combined
  package delta, not an unexplained regeneration. Both SHA fingerprint sites use
  explicit two-digit lowercase hex encoding; the branch-specific name-policy site
  has a fixed Python-derived hash regression. Friendly diagnostic classification
  remains fingerprint-gated and separate from schema validity. Schema bytes,
  parser/offline boundaries and name/exact-reference acceptance policy are unchanged.
- **Fixed digest checks:** Independently recomputed values match their annotations
  and fixed vectors in `pr7-currentmain-schema-fingerprints.json`:
  layer manifest schema SHA-256
  `50390284666d10e63b7b7a63e2089bccd8ebdbb68a06ac105886f445754301c6`;
  nonblank-pattern SHA-256
  `f308e30ae4a261113af20ed108cd840829a39d26af2024f66ab1b4e08ffc3514`;
  name-rules SHA-256
  `225f6b51735509610a0236ccd835ca20f7a00934da13e48254d59ddfb4a7f6a8`.
  No new grammar or schema acceptance rule is introduced by the digest adaptation.
- **Merge preservation:** Independent review checked 18 incoming-only files,
  15 incoming deletions and 117 unchanged branch-only files exactly, plus the
  combined lockfile delta. Incoming coverage specifications and archived coverage
  changes remain as supplied by main; approved amendment specs/fixtures and
  unrelated active changes are preserved. Evidence: `pr7-currentmain-preservation.json`.
- **Renewed validation:** 572 locked Rust tests passed with zero failed/ignored;
  the 20 schema unit tests are included in that scope, not an extra 20 tests.
  The isolated Python coverage-policy suite passed 39 tests. Formatting,
  all-target/all-feature warning-denied clippy, all 9 main specs and this active
  change passed strict validation. Native read/create-denial execution is visible
  in the logs. Evidence: `pr7-currentmain-rust-tests.log`,
  `pr7-currentmain-schema-tests.log`, `pr7-currentmain-python-tests.log`,
  `pr7-currentmain-clippy.log`, `pr7-currentmain-spec-validation.json` and
  `pr7-currentmain-change-validation.json`.
- **Fresh copy/package/source identity:** Fresh offline build and 572 tests passed,
  zero failed/ignored, without Git/OpenSpec/local-progress directories, original
  build outputs or compiler wrappers. Package verification succeeded with 252
  files; 58 source/asset hashes match the current commit, package and fresh copy.
  Evidence: `pr7-currentmain-fresh-build-tests.log`,
  `pr7-currentmain-fresh-record.json`, `pr7-currentmain-package-verify.log`,
  `pr7-currentmain-package-contents.json` and
  `pr7-currentmain-validated-source-hashes.json`.

`pr7-currentmain-integration-result.json` records the accepted merge and measured
results. Independent review recomputed the digests and verified preservation,
hash/package/fresh-copy identity and the semantic boundaries above. At that source
acceptance, final documentation/evidence acceptance and parent-owned section 7
delivery remained separate gates; source acceptance does not imply a push, current
hosted check result or PR mergeability. Execution was local macOS only, with no
root, Linux, Windows, hosted-CI or LLVM coverage claim. Existing loader EOF
normalisation, the separate pre-selection input-reader race, silent closed-stream
acceptance and the unknown original socket sibling remain qualified as above.
No PR merge, automatic archive, release or global installation is authorised.
