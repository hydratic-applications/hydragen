## 1. Shared author model and schema

- [x] 1.1 Extend the maintained schema with string/object author and nested name/email/URL definitions, required object name, optional contacts, and retained additional object data; validate the schema itself and leave archived assets unchanged.
- [x] 1.2 Extend cached schema-backed field validation to whole author values and nested fields, enable offline format assertions, and preserve structured nested error paths and fail-before-prompt setup errors without duplicate Rust validity rules.
- [x] 1.3 Replace string-only author extraction with a typed representation in library metadata, loaders, and relevant callers, preserving original representation, decoded extra data, and source text; cover all five formats, TOML table/dotted/inline forms, duplicate keys, and invalid known-field types.
- [x] 1.4 Add maintained schema/loader fixtures for name-only and full strings/objects, independent optional fields, Unicode/whitespace, invalid names/contacts, extra nested data, and effective offline email/URI validation; verify add preserves string/object author bytes and comments during dependency appends.

## 2. Explicit input and structured generation

- [x] 2.1 Implement a shared shorthand interpreter with full-input recognition, literal-name fallback for unrecognised explicit input, exact component preservation, and schema-derived component errors; test name+email, name+URL, all three, punctuation, malformed/ambiguous forms, whitespace, and no partial extraction.
- [x] 2.2 Add `--author-email` and `--author-url`, requiring `--author` through clap, and include both in input-route selection; implement exact duplicate acceptance and differing shorthand/flag contact rejection without discovery or prompts.
- [x] 2.3 Have guided and flag-only init construct object authors, including name-only authors; emit nested YAML/JSON objects and TOML root inline tables with stable root/nested field order, omissions, exact accepted values, and trailing newline.
- [x] 2.4 Test explicit flags, shorthand, contact conflicts, orphaned/missing-value flags, directory defaults, help/version/parser priority, schema setup failures, and complete-candidate validation before writes; retain exclusive creation, duplicate observation, and truthful reporting behaviour.

## 3. Contact-bearing discovery records

- [x] 3.1 Retain underlying source records and validated contact provenance through exact-name grouping; derive exact-value email/URL groups independently with stable source order, deduplicated actual contact hints, and no case/URI normalisation or cross-name leakage.
- [x] 3.2 Extend project extraction for npm object/shorthand contacts, Cargo entry email, and Python named-entry email, retaining independently valid names/contacts and all existing regular-file, 1 MiB, no-symlink, no-traversal, no-execution limits.
- [x] 3.3 Extend NUL-framed Git discovery to names/emails and retain scope/origin/key-family association; implement unambiguous same-bucket pairing and explicit environment name/email pairing without zipping unrelated values, scanning inactive includes, or guessing email from accounts.
- [x] 3.4 Extend existing GitHub REST/GraphQL responses with public viewer email and organisation contacts, declared website/profile alternatives, safe host/login fallback URL construction, and viewer matching; retain name results when contacts are absent/invalid and add no private-email, per-organisation, retry, or credential-management calls.
- [x] 3.5 Freeze names and contacts in the existing preparation snapshot; preserve independent lanes, 15-second lifetime, 1,500 ms Author wait, 500 ms Git budget, capture limits, single foreground input/render owner, and bounded child cleanup, with no contact-step lookups or waits.
- [x] 3.6 Add deterministic discovery tests using maintained project fixtures, isolated real Git configuration, fake `gh`, injected clocks, and controlled completion signals; cover conflicting equal-name contacts, ambiguous/split Git identities, partial pages, host/login mismatches, malformed URLs, missing public email, frozen late contacts, and cancellation cleanup without real accounts/network.

## 4. Guided author and contact collection

- [x] 4.1 Retain typed selected-group versus custom-entry identity after Author; skip the whole author on empty/Skip, parse explicit shorthand, bypass captured contact fields, and never reattach a custom typed name to a discovered group.
- [x] 4.2 Add conditional Author email then Author URL selection/text collection with exact labels/actions, first-suggestion selection, independent Skip/custom/empty behaviour, no suggestions/defaults for custom authors, and no email-driven filtering of URL choices.
- [x] 4.3 Reuse scoped cliclack presentation for contact choices: stable compact comma-separated source-family summaries on every visible candidate, derived only from actual structured contact evidence; no icons or focus-dependent text, native scrolling/wrapping, accepted-value/Skipped summaries, and basic/no-colour text; keep shorthand-captured details visible in the original Author answer and reset themes before later prompts. Retain full field-level hints internally.
- [x] 4.4 Extend typed diagnostics and guided validation feedback for nested fields, contact-format failures, and explicit input conflicts through the shared diagnostic contract; review escaped values, relevant flag/field context, correction/skip guidance, and no-write facts without matching English error strings.
- [x] 4.5 Add bounded actual-terminal tests for all shorthand prompt-bypass combinations, selected/custom/no-name authors, equal-name multi-contact choices, one/no suggestion, custom contact entry/clearing, independent contact omission, long/wrapped source rows, format selector isolation, and Escape/Ctrl-C at each new input route.
- [x] 4.6 Verify enriched preparation keeps Name promptly usable and only actual Author waiting shows progress; test no loading/spinner/request at Email/URL, stable partial snapshots, cancellation and no late output/files, and preserve the separately accepted upstream selector regressions/exclusions without claiming them fixed.

## 5. Documentation and verification

- [x] 5.1 Revise existing README/help explanations concisely for object output, manual string compatibility, shorthand, contact flags and conditional steps, public-GitHub-only discovery, local-email publication risk, and old-binary compatibility; review help at 40/80/100 columns without turning README into a discovery reference.
- [x] 5.2 Update the glossary's existing layer manifest definition, agent-facing schema/input/discovery guidance, and design-question implementation status in place; review the foundations scenario added with this proposal without moving CLI implementation detail into foundations or creating redundant terms/values.
- [x] 5.3 Review revised documentation as a first-time reader, validate glossary/internal links, confirm all executable assets remain outside OpenSpec, and ensure historical schemas/changes remain untouched.
- [x] 5.4 Run formatting, locked build/tests, all-target lint, and applicable repository coverage-policy checks; review both captured output and bounded actual-terminal output for beauty, enjoyment, clarity, responsiveness, and safety, recording commands/results and any pre-existing/deferred failures separately.
- [x] 5.5 Run strict OpenSpec validation and verify the final implementation against every delta scenario, explicitly recording generated-output/library API changes, source-preservation evidence, privacy boundaries, and any untested platform behaviour before declaring implementation complete.
