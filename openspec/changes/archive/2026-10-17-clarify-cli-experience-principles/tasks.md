## 1. Establish the principle and development guidance

- [x] 1.1 Re-read the current DX value, CLI guidance, interaction specifications, and this design; confirm that any intervening work is preserved and that the implemented CLI inventory is still current.
- [x] 1.2 Extend the existing DX value in `FOUNDATIONS.md` with equal beauty, enjoyment, functionality, and ease-of-use goals across help, prompts, results, and errors, including intentional plain output; retain all existing DX meaning and all eight values without prescribing a visual style.
- [x] 1.3 Add concise CLI experience review guidance under CLI development in `AGENTS.md`, linking to DX while retaining clap/cliclack guidance and every existing interaction, automation, stream, validation, cancellation, and filesystem-safety rule.
- [x] 1.4 Add a brief partial resolution under success criteria in `openspec/design-questions.md`, linking to this design's audit decision and leaving broader success criteria and unrelated questions open.
- [x] 1.5 Review the revised documents as a first-time reader, compare complete sections against their prior requirements, and check local links and whitespace before using them as the audit baseline.

## 2. Audit the existing CLI after the documentation update

- [x] 2.1 Create `audit.md` in this change directory with the baseline revision, relevant local changes, date, tool/environment settings, terminal dimensions, scenario inventory, evidence conventions, and the four-goal assessment rubric from the design; inspect current sources, README, main specs, and both integration suites.
- [x] 2.2 Build with `cargo build --locked` and run `cargo test --locked`; record results and map existing tests to relevant behavioural guarantees without treating test success as visual or enjoyment evidence.
- [x] 2.3 Capture root/init help, version, missing subcommand, unknown option, and missing flag-value behaviour; assess discoverability, scanning, wording, and consistency with actual behaviour.
- [x] 2.4 Observe guided init in disposable directories, covering default acceptance and override, independent optional skips, validation retry, completion, and Escape/Ctrl-C cancellation before and after partial input; retain terminal evidence and output-channel/exit observations with bounded waits and child cleanup.
- [x] 2.5 Assess flag-only and automation execution, including name-only and optional-only flags, defaulted names, invalid metadata, explicit non-interactive mode, CI with a terminal, and each standard stream redirected in turn; verify prompt suppression, plain output, and stream separation.
- [x] 2.6 Assess target and failure feedback for an existing manifest, an incorrect repository location, an unusable derived name, and post-write reporting failure; reference existing tests for difficult paths, and review ordinary/narrow widths, enabled/disabled colour, and long or Unicode names/paths without adding a permanent harness.
- [x] 2.7 Complete all four goal assessments for each surface with evidence and rationale; distinguish confirmed contract violations, qualitative opportunities, design-review needs, and unverified conditions, without claiming universal compliance or user research.

## 3. Plan remediation and hand off for review

- [x] 3.1 Create `remediation-plan.md` linking each actionable finding to an affected goal or requirement, proposed direction, likely files, dependencies, impact/risk priority rationale, and acceptance checks; separate adherence work from optional refinement and preserve satisfied behaviours.
- [x] 3.2 Group proposed fixes into focused later increments, identify any separate OpenSpec changes and user decisions required, retain unresolved evidence gaps, and explicitly stop before implementing CLI fixes; state a no-confirmed-gaps outcome if supported rather than inventing work.
- [x] 3.3 Review the documentation, audit coverage/evidence, and remediation plan for first-time-reader clarity and traceability; check local Markdown links, run `git diff --check`, and run `openspec validate clarify-cli-experience-principles --strict`.
- [x] 3.4 Present the audit and remediation plan for user review, explicitly distinguishing completed documentation/audit work from unimplemented fixes and any remaining unknowns; do not archive or implement follow-up changes automatically.
