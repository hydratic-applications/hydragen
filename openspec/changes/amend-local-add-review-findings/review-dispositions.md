# PR #7 review dispositions

## Scope and evidence status

The holistic review concerned `8fb6893`. The owner approved focused amendments,
ordinary integration/delivery and the runtime-limited reporting direction.
This record distinguishes accepted findings from completed implementation.
All source gates are accepted, including YAML tasks 2.1–2.6, diagnostic source
tasks 3.1/3.2 and harness ownership remediation in 4.4/4.5. Focused documentation
5.1–5.4 passed independent review; the parent completed actual-terminal review and
the 571-test amended-tree Rust run plus 39 Python tests. Package contents and a
fresh offline build/571-test copy have also been verified. Independent final local
review accepted tasks 5.5 and 6.4 for delivery without source changes. At that
pre-delivery acceptance, only task 6.5's delivery work remained. See
[verification](verification.md) for commands, source references and evidence
boundaries; research reproductions alone are not a shipping claim.

No optional review sweep, new public error shape, registry/formatter work, stronger
descriptor detection, blanket best-effort-write propagation or automatic archive
is authorised here. The parent explicitly invoked the approved main-spec alignment;
that action is recorded below. Library Display and CLI composition remain
distinct. Final evidence must name changed files, targeted/full results and any
unexecuted environments, not merely mark an R-number addressed.

## R1 — Accepted integration, complete prerequisite

- **Finding/disposition:** Integrate inspected main without losing branch parser,
  mutation or incoming harness/coverage protections. Complete and independently
  accepted, not a task to reimplement earlier archived features.
- **Implementation:** `c3b3c8a` preserves the user's exact cancellation wording;
  `3ea1fcb` ordinarily merges it with `6ccf96b`. Shared parser-first dispatch, bare
  version/write-error handling, terminal readiness/key-loss protections and
  incoming workflows/gates/security remain. Independent review checked 27 incoming
  file blobs unchanged against the inspected main, with overlaps reviewed separately.
- **Evidence:** 544 Rust tests (110 library, 129 binary, 305 integration), 39 Python
  coverage-policy tests with isolated diff-cover 10.6.0, fmt, warning-denied
  all-target/all-feature clippy and diff checks passed. Local retained evidence:
  `pr7-phase1-rust-tests-commit.log`, `pr7-phase1-python-tests.log` and
  `pr7-phase1-terminal-evidence/` (80 loss-matrix evidence files).
- **Limitations:** Controlled probes support transient reader retention in the
  original socket broken-pipe fixture during concurrent pre-exec spawning; the
  exact original sibling is unknown. Independent closed-stdout silent EBADF is a
  separate runtime observation. Deterministic fixtures and safe anchored ownership
  remediation passed the harness gate. No read-syscall-blocked, production input-race,
  Linux/Windows/root/hosted-CI or LLVM coverage claim.
- **Contract/tasks:** Parser-first/help/version and existing safety requirements;
  task 1.1 accepted. Subsequent evidence is recorded separately from the baseline.

## Per-finding final source dispositions

| ID | Accepted disposition and bounded action | Implementation and evidence |
| --- | --- | --- |
| R2 | Fixed and independently accepted: unsafe clip/keep EOF insertion is refused, including existing-list tails and explicit scalar keys with implicit null values; safe controls remain. Loader EOF normalisation is unchanged, not repaired. | `src/layer_manifest/edit/yaml.rs`, `tests/yaml_field_order.rs` and `tests/field_order.rs`; bounded scanner-tail facts, optional indent/checked columns, refusal/no-op assertions, 87 scoped tests and final 571-test run. |
| R3 | Fixed and independently accepted: gaps are literally empty, five masking normalisers removed, nonempty data unchanged. | `src/presentation.rs`, `src/diagnostic_body.rs`, `src/usage_errors.rs`, `tests/diagnostic_body_support/mod.rs` and captured/terminal expectation consumers; five failing-first cases, 81 focused units, 24 captured usage/security tests and final representative terminal review. |
| R4 | Fixed and independently accepted: multiline entry/close/comment layout is preserved with separate boundaries and bounded insertions; compact style remains compact. The invalid implicit-null `custom:,` fixture/blocker was corrected to separated `custom: ,`. | `src/layer_manifest/edit/yaml.rs`, `tests/dependency_edit.rs`; 87 scoped tests, eight exact actual-CLI variants independently decoded/rechecked by Psych, then final Rust pass. |
| R5 | Fixed and independently accepted: non-regular entries receive neutral regular-file guidance with paths, no fallback and distinct current/dependency symlink policies. | `src/diagnostics/add_verification.rs`, `src/layer_manifest.rs`, captured and terminal directory-occupant fixtures; focused source gate, final native-denial/family runs and parent reading at 40/80 columns. |
| R6 | Fixed and independently accepted: non-string metadata requires a string without null attribution; optional omission remains conditional. No public NotString shape change or TOML null syntax. | `src/diagnostics/content.rs`, `src/layer_manifest.rs`, parsed-type regressions and diagnostic consumers; 96 parsed type/field/format combinations, each rendered for three source origins, captured corrections and parent terminal reading of numeric metadata. |
| R7 | Approved limitation documented and deterministic fixtures accepted: only errors returned by established fallible completion write/flush paths remain fatal/stateful. Best-effort warning/error delivery and production policy are unchanged. Silent closed-descriptor acceptance remains a runtime limitation. | `tests/output_failure_support/mod.rs`, `tests/fixture_harness.rs`, reporting consumers in add/init/operational suites, test-only portable ErrorKind simulation in `src/interaction.rs`, README/reference and reporting specs; anchored lifecycle source review, eight independent helper passes, 67 captured passes, safe stress, final 571-test/terminal runs. Exact original socket sibling remains unknown. |
| R8 | Fixed and independently accepted: copied-help fixture removes inherited COLUMNS so the documented fallback is deterministic; deliberate width tests are unchanged. | `tests/init.rs`; inherited COLUMNS=30 before/after evidence, final 46-test init suite within the full run and terminal help review. |
| R9 | Documentation correction accepted: actual name-only init/add flow list replaces the formatting hedge, without changing generation or removing valid block-list examples. | `README.md`; field-order/editor tests, local document review and independent documentation approval. |
| R10 | Documentation correction accepted: concise existing NO_COLOR/CLICOLOR/basic-terminal/CI/redirection/disable-over-force policy, no new semantics. | `README.md`, retained `src/presentation.rs` gates; independent documentation review and final styled/plain/control transcript families. |
| R11 | Compatibility notes accepted: concrete stricter-name rejection, LoadedLayerManifest.source, InitCreationError.state, InvalidName matches and bare-number version changes; no stability/release promise. | New `CHANGELOG.md`, checked against `src/lib.rs`, `src/layer_manifest.rs` and integrated version handling; independent documentation review, final help/version tests and source compatibility wording review. |
| R12 | Fixtures fixed and independently accepted: actual denial probing before snapshots, explicit ready/unsupported/setup-error handling across three fixtures/five consumers, restore/cleanup on all paths and unaffected coverage retained. No root-runtime claim. | `tests/permission_support/mod.rs`, `tests/fixture_harness.rs` and captured/terminal consumers; synthetic unsupported/setup-error/unwind regressions plus visible native read/create-denial execution in final terminal logs. |
| R13 | Documentation omission resolved, replacement algorithm unchanged: hard links/open handles and platform restrictions explained; no alias enumeration, new refusal or in-place fallback. | `README.md`, module docs in `src/replacement.rs`, aligned safety specification; independent documentation review and retained native open-handle replacement unit in the final host suite. No Windows execution claim. |
| R14 | Not established as a defect: deliberately retain library Display versus contextual CLI audiences and typed-fact composition. No headline-equality/deduplication refactor; factual R6 drift corrected separately. | `design.md` audience rationale, retained typed adapters in `src/diagnostics/` versus library Display, 81 focused units and final shared-family/runtime review. |
| R15 | Alternate fix accepted: independent exact output/permitted offsets/text and corruption controls, not the proposed global contiguous-insertion rule. Legitimate multiple YAML/JSONC/TOML sites remain supported. | New `tests/support/insertion_oracle.rs`, `tests/dependency_edit.rs`, `tests/yaml_field_order.rs` and `tests/field_order.rs`; source-oracle review, negative controls, independent null-flow validity correction and final full test pass. |

## Accepted source phase evidence

- **R2/R4/R15:** Tasks 2.1–2.6 passed the independent YAML gate after the
  implicit-null flow-separation blocker was corrected. The editor, three scoped
  integration suites and independent insertion helper cover bounded scanner-tail
  facts, safe controls, multiline layout and permitted-site corruption negatives.
  Accepted results are 87 tests (39 dependency-edit, 18 field-order, 11 YAML
  field-order, 19 YAML unit) plus independent re-review of eight actual-CLI
  implicit-null flow variants. The loader's underlying EOF normalisation is
  unchanged and outside this fix; it must not be reported as repaired.
- **R3/R5/R6:** Source tasks 3.1/3.2 passed independent review with 81 focused unit
  tests and 24 captured usage/security tests. Five failing-first unit cases and
  direct CLI captures established spacing/causes; typed API, schema acceptance
  and symlink/no-fallback policies are retained. Harness-owned consumer
  reconciliation and scheduled actual-terminal review have now passed; their
  final evidence is recorded separately in verification.
- **R7:** Controlled probes support the fixture's temporary-reader mechanism,
  independently of silent closed-descriptor runtime behaviour. They do not prove
  the exact original sibling identity. Deterministic fixture delivery and anchored
  ownership remediation subsequently passed independent review.
  Completion-only fatal/stateful handling and best-effort warning/error delivery
  remain unchanged; the parent's harness gate is accepted. Native read/create
  denial, synthetic unsupported/setup-error controls and safe cleanup evidence are
  included in the accepted fixture results.

## Documentation and final local review accepted

- **R9/R10/R7/R13 — `README.md`:** The name-only walkthrough now shows the actual
  flow list without a formatting hedge. The concise colour paragraph names the
  existing gates. Safety text distinguishes completion errors returned by the
  established path from best-effort warning/error delivery, retains unchanged
  no-op state, and explains runtime receipt and path-replacement limitations.
  Branding/assets, coverage guidance and other examples are unchanged.
- **R11 — `CHANGELOG.md`:** Unreleased notes name intentional name rejection,
  `LoadedLayerManifest.source`, truthful `InitCreationError.state`, exhaustive
  `InvalidName` matches and integrated bare-version output. No release or stability
  promise is added.
- **R3/R5/R6/R7 — `docs/cli-diagnostics.md`:** Literally empty separators,
  fact-appropriate regular-file/string guidance, fatal/stateful returned
  completion errors and unchanged best-effort warning/error delivery are explicit.
- **R13 — `src/replacement.rs`:** Module documentation alone explains other hard
  links/open handles, platform restrictions and no alias update/in-place fallback;
  production code is unchanged by this documentation lane.
- **`openspec/design-questions.md`:** Questions 2 and 10 link only the bounded
  preservation/layout and reporting/audience decisions. Broader questions and
  historical archives remain unchanged.
- **Local checks:** Strict validation of this change passed; 81 local links/anchors
  across the six reviewed Markdown documents passed, as did owned whitespace and
  final-newline checks and scoped `git diff --check`. No full or PTY suites were
  run in this documentation lane.
- **Clarity/foundations assessment:** Read revised wording from a first-time
  reader's perspective. The README explains practical consequences; API detail is
  confined to Unreleased notes and implementation policy to the focused reference.
  Existing foundations values and glossary meanings suffice: no new product belief
  or Hydragen term was introduced. Independent review approved tasks 5.1–5.4 and
  the parent ticked them. Independent final review also accepted task 5.5's
  source/evidence dispositions and task 6.4 for delivery; this lane does not change
  their checkboxes.

## Explicit maintained-spec alignment

The parent invoked approved alignment after the accepted source gates. Applied
six additions (two CLI interaction, four dependency addition) and three bounded
reporting modifications (one in each affected capability). All 35 unmentioned
requirements' contents and scenarios are unchanged; original requirement order
and document introductions were preserved. No requirement was removed or renamed;
there are no duplicate requirement/scenario headings. The change remains active, with no
archive or task-checkbox action by this documentation lane.

## Combined results and remaining gates

The parent measured 571 amended-tree Rust test functions (111 library, 133 binary,
327 integration), zero failed/ignored, and 39 final Python coverage-policy tests.
Formatting, warning-denied all-target/all-feature clippy, all-seven-spec/change
strict validation and diff checks passed. Scheduled terminal reruns passed 23
diagnostic test functions and one guided test, with 2,414 transcript evidence
files and independently accepted representative human wording review for task 3.3.
Interactive-binary function counts are not actual-PTY-test counts. Verification
records explicit Beauty, Enjoyment,
Functionality and Ease-of-use judgments and the remaining narrow long-path density.

Package execution returned zero and all 250 included files were inspected,
including the maintained schema and new test helpers. The fresh offline build
and 571-test copy passed with zero failures/ignored tests; 57 source/asset hashes
match the checkout exactly. See `pr7-final-package.log`,
`pr7-final-fresh-record.json`, `pr7-final-fresh-build-tests.log` and
`pr7-final-validated-source-hashes.json`. The final oracle independently checked
all 57 hashes against checkout/fresh files, both 571-test logs, the 39 Python tests
and all 250 crate files/required assets, including matching `Cargo.toml.orig`.
It confirmed no schema, dependency or coverage-policy change versus `3ea1fcb` and
accepted delivery without further source changes or tests. Delivery is tracked
separately by task 6.5 and the PR head; this record does not assert a push.
No Linux/Windows/root/hosted-CI/LLVM
coverage, global installation, release, PR merge or archive is claimed.
