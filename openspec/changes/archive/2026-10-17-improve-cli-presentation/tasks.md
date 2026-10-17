## 1. Width-aware help

- [x] 1.1 Reconfirm the current main specs, audit F1–F3, locked dependencies, and uncommitted principles work; preserve unrelated changes and capture a concise before baseline for this change's verification record.
- [x] 1.2 Enable clap's `wrap_help` feature without a deliberate dependency version upgrade; apply a 100-column cap with detected-width behaviour, use next-line init descriptions, and restructure examples/input guidance without losing information or valid shell quoting.
- [x] 1.3 Add deterministic builder-level layout/content checks for 40-, 80-, and 100-column widths, including short/long help, copyable examples, and explicit long-token preservation; update captured-help assertions to tolerate legitimate line breaks without weakening semantic coverage.
- [x] 1.4 Extend the existing PTY harness with test-only size/colour options while retaining current defaults, separate streams, serialization, bounded waits, and child cleanup; verify actual help width adaptation, plain output, no writes/prompts, and the unknown-width fallback separately from explicit-width tests.

## 2. Prompt-appropriate validation

- [x] 2.1 Adapt structured shared-validation failures in `src/interaction.rs` to concise field-specific prompt instructions; explain clearing and Enter-to-skip for optionals, leave the validator and flag-only errors unchanged, and preserve other error diagnostics rather than parsing display text.
- [x] 2.2 Update the obsolete flag-oriented prompt wait and add unit/terminal coverage for invalid Name, Author, and Description, correction and clear-to-skip, enabled/disabled colour, no early writes, unchanged Unicode/whitespace/defaults, and literal `Skipped` preservation.
- [x] 2.3 Recheck flag-only empty/whitespace metadata failures, optional-only flags, non-interactive/CI/redirected execution, and cancellation to confirm presentation changes introduce no questions, acceptance changes, or output-channel regressions.

## 3. Safe existing-layer-manifest recovery guidance

- [x] 3.1 Extend only occupied-layer-manifest diagnostic formatting to name the path, explain no overwrite, give safe inspect/different-directory advice, and describe this invocation's no-change outcome without assuming layer manifest validity, reading the entry, or recommending deletion/force.
- [x] 3.2 Strengthen captured and terminal assertions for regular/malformed files, directories, symlinks/dangling symlinks, repeated init, and a layer manifest appearing during questions; retain byte/target preservation and plain stderr/nonzero/no-success checks, including long-path fidelity.

## 4. Documentation, evidence, and review

- [x] 4.1 Update README help/examples and the occupied-layer-manifest error example, and add a narrow presentation-decision link under question 10 in `openspec/design-questions.md`; review from a first-time reader's perspective without weakening the foundations or resolving unrelated questions.
- [x] 4.2 Create `verification.md` in this change with revision/environment, test commands/results, and concise before/after terminal evidence for help, guided validation, and recovery errors; explicitly review all four DX goals, label replay/platform limits accurately, and leave R4/R5 findings separate.
- [x] 4.3 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --locked`; validate this change with `openspec validate improve-cli-presentation --strict`, check Markdown links and `git diff --check`, and confirm no unintended command/schema/filesystem/success-output changes.
- [x] 4.4 Once the fixes and evidence pass, tick only R1–R3 in the earlier repository remediation checklist and its saved Project Plan, leaving R4/R5 and evidence gaps open; present the completed increment for review without automatically archiving, publishing, or applying deferred fixes.
