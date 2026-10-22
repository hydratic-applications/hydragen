# Concise author presentation verification

Verified locally on macOS on 2026-10-22.

## Result

The picker now uses dimmed source-evidence categories on unfocused names and short
source-family summaries on focus. Candidate and action icon slots are removed.
The common three-source example becomes `(Git, GitHub, local account)`, while the
complete original hint remains available internally. No source lookup, ordering,
eligibility, author value, filesystem lifecycle or preparation deadline changes.

## Checks

- `cargo test --locked`: **698 passed, 0 failed, 3 existing ignores**, including
  **54 production interactive-init tests**. A subsequent neutral fixture-name
  replacement passed all **13 author-discovery grouping tests**.
- `cargo fmt --all -- --check`, all-target Clippy with warnings denied, locked
  build, and `git diff --check`: passed. The main workspace's debug binary is rebuilt.
- Strict validation: this change and all **9 maintained specs** passed. Structural
  comparison confirms the maintained presentation requirement exactly matches its
  delta. No archive or unrelated planning changes were made.
- Actual PTY text captures at **40, 60 and 80 columns** were inspected. The focused
  identity example occupies one row at 60/80 columns and wraps naturally at 40;
  names and actions remain aligned. Longer Enterprise summaries are separately
  checked at all three widths, including wide characters and stale-text clearing.

## Regression coverage

The new grouping tests cover repeated project/Git/account/membership records,
first-source display order, retained detailed hints, safe public/Enterprise host
handling and mixed eligibility. Existing detailed-source tests remain unchanged.
The theme tests cover dimmed suffixes, all four categories, Unicode/basic
separators, action identity, unmodified completed values and unrelated selectors.
Production PTYs cover exact values, no-colour/basic output, source-family collapse,
mixed categories, literal emoji/action/category-like names, wrapping and cleanup.
The fixture GitHub host override reaches only fake `gh`; no real authentication or
network is used by these tests.

## Human review and documentation

The names lead visually, secondary text is dimmed, and actions no longer compete
with them through decorative icons. The displayed hint explains source families
without repeating settings, logins or project filenames. This implements the
user-approved direction; it does not claim all long names fit one line.

README, agent guidance, the design-question resolution and maintained specification
now describe this behaviour. They were reviewed for a first-time reader's meaning
and practical consequences. FOUNDATIONS.md explicitly extends the existing
everyday-development scenario with the user's shared-name/wrapping example,
including medium-width terminals. This applies the existing DX value rather than
adding a value or changing the product philosophy. No glossary term is introduced.

## Preserved limitations

- Explicitly running the two ignored long-membership tests still produces their
  **two expected upstream visibility failures**, after correct returned values.
  Their keys, ignore reasons, value assertions and visibility assertions remain;
  only expected row prefixes changed. Hydragen #14 still owns the cliclack fix.
- An initial concurrent binary/PTY run hit an existing subprocess fixture's startup
  timeout in `author_cutoff_preserves_profile_and_first_page_and_reaps_stalled_later_page`.
  It passed unchanged in isolation and in the full suite. No runner/timing policy
  was weakened to mask this observation.
- These are local macOS results, not current hosted coverage/MSRV or Windows
  validation. The PR's separate missing `tests/rust_version` MSRV problem and
  other deferred work are outside this change.
