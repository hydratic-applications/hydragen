# Author presentation verification

## Result

Verified locally on macOS on 2026-10-20 after the user approved the terminal
trial. The approved implementation is ready for publication to PR #15 without
merging or archiving this change. All eight tasks are complete, with one new
requirement and all ten scenarios covered. No blocking implementation/specification
divergence was found in the final review.

## Completeness and correctness

`src/interaction.rs` implements focused-only inline hints and the single padded
icon slot through the existing cliclack theme. `src/author_discovery/mod.rs`
implements natural-language source joining, safe GitHub host hints and provenance
aggregation independent of hint deduplication. No discovery request, ordering,
eligibility, accepted value, filesystem rule or format-selector behavior changed.

The following tests cover every scenario in the added requirement:

| Scenario | Evidence |
| --- | --- |
| Focus reveals only that candidate's sources inline | `only_the_focused_active_author_candidate_has_an_inline_hint`; production focus/source and long-to-short screen checks |
| Natural-language punctuation | `source_hint_joining_handles_empty_single_pairs_and_oxford_lists`; `joining_keeps_source_text_order_exact_deduplication_and_eligibility`; production one/two/three/four-source fixture |
| Public and Enterprise hosts | `github_hints_omit_the_public_host_but_keep_enterprise_context_and_safety`; unchanged fake-gh explicit Enterprise host/API fixture |
| One mixed icon for equal names | `project_evidence_neither_erases_nor_invents_known_provenance`; `hint_deduplication_does_not_discard_provenance_evidence`; production equal project/identity/profile/membership group |
| No inferred entity type | `provenance_describes_sources_even_when_identity_settings_name_a_company`; production project-only and membership-only choices |
| Aligned candidate/action slots | `all_author_icons_share_one_slot_and_keep_hints_inline`; production Unicode/basic measured label-column assertions |
| Colour-disabled/basic meaning | Production no-colour/basic fixtures, including explicit controls and ASCII icons; unit decorated/plain slot combinations |
| Wrapping, clearing and cancellation | `narrow_inline_sources_wrap_naturally_and_clear_on_shorter_rows_actions_and_submission`; `focused_author_cancellation_clears_sources_and_icons_from_the_visible_frame`; real PTYs at 40/80 columns, 16 rows |
| Literal names | `candidate_names_that_match_actions_remain_literal_author_metadata` includes action text, old/new pencil prefixes and parentheses; accepted metadata is compared exactly |
| Summaries/unrelated selectors | `author_details_never_leak_into_submit_cancel_or_error_rows`; `focused_author_details_are_dim_and_other_selectors_keep_their_layout`; production accepted/skip summaries and guided format fixtures |

The screen helper in `tests/interactive_init.rs` applies cursor movement, erasure,
wrapping and viewport scrolling, rather than treating transcript history as the
current screen. It measures zero-width variation selectors with their base glyph
and includes an emoji-pencil right-edge wrap check. It is a bounded model of the
controls emitted by this CLI, not a universal terminal emulator or font test.

## Validation

- `cargo test --locked`: **692 passed, 0 failed, 3 ignored**. This includes all
  **53 production interactive-init PTYs**, controlled author PTYs, and the
  unchanged flag-only/discovery-boundary tests.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo build --locked`: passed; main-workspace `target/debug/hydragen` rebuilt.
- `git diff --check`: passed.
- `openspec validate refine-author-choice-presentation --strict`: passed.
- `openspec validate --specs --strict`: all **9 maintained specs passed**.
- Structural comparison confirms the new requirement exactly matches its synced
  main-spec block, all **25 prior CLI-interaction requirement blocks** are
  unchanged, and the new requirement has **10 scenarios**.
- Archive, dependency files, workflows, foundations and glossary are unchanged.

## Coherence and human review

The final code follows the existing theme, typed-action and scoped-reset patterns;
it does not replace cliclack input handling. The user tested the local binary and
approved the inline parentheses, shorter public GitHub hints, emoji pencil and
shared two-space gap. This is evidence for the chosen terminal experience, not a
claim that every terminal font renders emoji identically.

Read the proposal, design, specification, README edit and developer guidance from
a first-time reader's perspective. The text defines source evidence without
claiming verified human authorship, explains why Enterprise hosts remain visible,
and distinguishes logical rows from natural terminal wrapping. The foundations'
existing DX and performance values already cover this work; no new product term
or glossary/foundations revision is needed.

## Preserved limitations and follow-ups

- Explicitly running `cargo test --locked --bin hydragen long_membership --
  --ignored` still yields the **two expected visibility failures** from cliclack
  #115: hidden initial custom entry and hidden Up-selected name/summary. Returned
  author values remain correct. Exact function comparison confirms only the two
  required active-row prefix literals changed, not keys, ignore reasons, value
  assertions, visibility/summary conjunctions or failure messages. Hydragen #14
  remains open; no dependency patch or fix is claimed. The third ignored test is
  the child-only terminal driver used by active tests.
- PR #15's previously observed MSRV job fails before compilation because
  `tests/rust_version` is unavailable on this branch. This presentation change
  does not reconcile main or repair that job, and local tests are not a claim of
  hosted Linux, MSRV, Windows or current coverage-gate success.
- One existing Git guard-expiry test failed in an earlier concurrent trial run
  and passed unchanged on rerun. The final full suite also passed it. Project todo
  `0868375c-b105-408d-b153-84b8f3d16763` tracks that intermittent failure separately;
  no deadlines or assertions were weakened here.
- The previously recorded direct equal-Git/valid-GitHub-profile fixture gap remains
  separate. This change covers equal environment identity/profile/project/membership
  grouping, not that exact pairwise source fixture.
