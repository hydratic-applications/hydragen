# Author contact implementation verification

## Scope and status

This records evidence for `extend-init-author-details` in the current macOS
checkout, starting from `c5d6962d4a4a07c4c88e1ecd90a73fe268446ee5`.
Implementation, local validation, and independent final conformance review are
complete, within the explicit platform/coverage/upstream exclusions below.
See [tasks.md](tasks.md) for checked implementation progress and
[design.md](design.md) for the selected behaviour and source/privacy boundaries.
No commit, pull request, publication, or archive has been performed by this work.

## Foundation and shared interface

The schema/model/parser phase received independent review. The review found and
the implementation corrected shorthand recognition that had rejected ordinary
name punctuation ahead of otherwise recognisable contact suffixes. For example,
`Alex (team) <alex@example.com>` now retains the name and captures email;
`Example @team <not-an-email>` retains syntax-only components for discovery but
fails explicit input at `/author/email`. Repeated/reversed contact suffixes still
do not permit partial extraction.

Focused correction validation passed:

```sh
cargo test --locked --lib --test author_metadata \
  --test layer_manifest_schema --test dependency_edit --test add_diagnostics
```

Result: **182 tests passed**. The previously failing binary diagnostic unit test
also passed after its string-only author expectation was updated to the shared
string/object/required-name schema contract.

These checks passed after that correction:

```sh
cargo test --locked --all-targets --no-run
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check
```

The sequential discovery-interface checkpoint was separately reviewed and
accepted. Its focused run passed **69 discovery tests**, including independently
grouped actual-contact provenance, retained source records, exact host/login
enrichment, component-wise best-effort filtering, and frozen late-result races.
All-target no-run compilation passed. Two unused integration API warnings at this
intermediate checkpoint are not final lint evidence; source/CLI integration is
responsible for using the newly delivered API.

Library tests and fixture-owned insertion assertions cover schema changes,
offline contact formats, nested failure paths, extra author-object data,
legacy-string preservation, all five formats, commented YAML and TOML
inline/table/dotted layouts, LF/CRLF, and verified-add replacement. These are
focused results, not a substitute for final full-suite validation.

## Coverage-policy and documentation preparation

Pinned `diff-cover[toml]==10.6.0` was installed into the ignored local
`target/coverage-venv`, without changing global tooling or runtime dependencies.
The first policy run passed 41 of 42 tests; the absolute-path inventory test hit
macOS's `/var` versus `/private/var` temporary-path alias. With the approved
canonical temporary directory and unchanged test assertions, this rerun passed:

```sh
TMPDIR=/private/var/folders/p9/02qtjw2138dd0jbmmjb0k45r0000gp/T/opencode \
  DIFF_COVER="$PWD/target/coverage-venv/bin/diff-cover" \
  python3 -m unittest discover -s tests/coverage -v
```

Result: **42 tests passed** in 20.104 seconds. These are coverage-policy helper
tests, not a measured Rust line-coverage percentage. `cargo-llvm-cov` is not
installed in this environment. No instrumented local report or hosted Linux
percentage-gate result is claimed.

Prepared README, glossary, and agent guidance received a first-time-reader
review. The revision distinguishes unchanged unparsed legacy author strings
from schema-checked object/init contacts, says init always generates an object
when author is supplied, and explains contact-publication risk. A local link
check passed **281 Markdown targets**, excluding literal code examples.

## Integration and final verification

### Discovery source enrichment

Source-side tasks 3.2–3.6 received independent review and were accepted without
blocking findings. Source enrichment uses the shared recogniser and independent
schema-backed contact filtering, exact Git scope/origin/family associations, and
matching GraphQL viewer/organisation public contact data. Authenticated REST
email is deliberately excluded. Fake-request logs assert existing profile/page
calls only; fallback-URL tests reject authority/path injection. Controlled cutoff
and cancellation tests retain earlier validated contacts without changing frozen
choices or extending resource/deadline limits.

```sh
cargo test --locked --bin hydragen author_discovery::
cargo test --locked --all-targets --no-run
cargo clippy --locked --bin hydragen -- -D warnings
cargo clippy --locked --bin hydragen --tests -- -D warnings
```

All passed; the discovery test run contained **79 passing tests**. Source-only
formatting and diff checks passed. No real-account requests, network tests,
runtime dependencies, or consumer API changes were required.

### Guided, flag-only, and terminal integration

The completed CLI lane received independent review against the source APIs and
selected design. Its final focused repeat passed **385 tests**, with zero failures
and the three retained ignores: binary 248; discovery routes 15; init 49;
interactive init 56; operation diagnostics 2; usage errors 15.

Actual pseudo-terminal runs cover all shorthand bypass combinations,
selected/custom/no-name authors, same-name custom entry, independent contact
selection/omission/custom entry, long native-scrolling lists, invalid-field
retry/clearing, Escape/Ctrl-C on selector/text routes, frozen late contacts,
responsive Name input, no additional contact-step waits, and format-theme
isolation. `target/author-contact-review/` holds local ANSI/plain captures for
40/80-column contact wrapping in styled, colour-disabled, and basic terminals;
diagnostic families were reviewed at 40/80/100 columns. Help also has
40/80/100-column checks. These ignored build-directory captures are local review
evidence, not maintained executable fixtures or a portable artifact guarantee.

The experience review found the conditional selectors useful without adding a
representation question or repeated confirmation; contact sources remain visible
only on focus, and completed answers are compact. Native rendered-screen tests
assert wrapping/redraw clearing rather than inferring appearance from stripped
transcript history alone. Plain diagnostics preserve causes, correction and
no-write state. These are authored UX judgments supported by actual-terminal
evidence, not a claim that passing snapshots alone proves enjoyment or beauty.

### Full repository validation

The first whole-suite run found six field-order fixtures still expecting init to
generate string authors. Only CLI-generated expectations were corrected through
a separate `init_metadata` helper; library/manual legacy-string fixtures and
independent byte/order/insertion assertions remain unchanged.

The final uninterrupted full-suite run passed:

```sh
TMPDIR=/private/var/folders/p9/02qtjw2138dd0jbmmjb0k45r0000gp/T/opencode \
  cargo test --locked -- --test-threads=1
```

Result: **746 passed, 0 failed, 3 retained ignores** — 122 library, 248 binary,
376 integration tests, and zero doctests. Serial execution bounds terminal/test
contention in this macOS environment. One earlier verification command reached
its 120-second command limit; the uninterrupted retry above completed. No
assertions, coverage settings, or exclusions were weakened.

These final checks also passed:

```sh
cargo build --locked
cargo test --locked --all-targets --no-run
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all --check
git diff --check
openspec validate extend-init-author-details --strict
```

An additional actual-binary smoke check verified that
`Alex (team) <alex@example.com> (https://alex.example)` becomes the three-field
object in JSON, and conflicting explicit email values fail without creating files.
The complete maintained-doc/change link check passed **283 local targets** at
that checkpoint, excluding literal examples. Source/test inspection found no
executable loads of OpenSpec assets, and archived artifacts remain unchanged.

### Compatibility and explicit limitations

- Init now always writes an author object when supplied, including name-only
  authors. Existing strings are still loaded without parsing or conversion.
- The public library changes `Metadata.author` from `Option<String>` to
  `Option<Author>`; `Author::String` preserves legacy input and `Author::Object`
  holds a name, optional contacts and retained unknown members. Older unmodified
  binaries that understand only string authors cannot read newly generated objects.
- Add preserves existing author representation, extra decoded data, comments,
  source layout and bytes outside the intended dependency insertion. Schema-backed
  contact checks are offline syntax checks, not ownership or reachability checks.
- GitHub lookup uses public viewer/organisation email only, with matching
  host/login and no private-email enumeration or new permission/authentication
  flow. Local Git/project contacts may be private and require explicit selection.
- All actual-terminal and runtime evidence here is macOS-only. Windows/Linux
  runtime behaviour and hosted Linux coverage gates are not verified by this run.
  The 42 passing coverage-policy tests are not a measured coverage percentage;
  instrumentation tooling was unavailable locally.
- The child-only terminal driver ignore and the two accepted upstream cliclack
  initial-selection/feedback exclusions remain intact. No new ignores were added;
  those two upstream regressions were not rerun as passing tests or resolved by
  this increment.

### Conformance review

The change contains **16 requirements and 147 scenarios** across its three delta
specifications. The schema/model, source-enrichment, and combined CLI phases each
received independent review with actionable findings corrected before acceptance.
Final review accepted completeness, correctness and coherence with no critical or
warning findings. All **25 tasks** are complete. The single non-blocking suggestion
was to refresh the internal progress summary; that was corrected without changing
runtime code or historical evidence.

| Dimension | Final assessment |
| --- | --- |
| Completeness | 25/25 tasks; implementation evidence for all 16 requirements |
| Correctness | All 147 scenarios reviewed against implementation and maintained tests, subject to the explicit inherited upstream exclusion |
| Coherence | Selected schema, input, source association, privacy, presentation and lifecycle decisions followed |

This is an implementation-and-evidence review, not a claim that every scenario
has a uniquely named test or that the untested platforms/coverage gates passed.

| Requirement | Scenarios | Implementation/test evidence |
| --- | ---: | --- |
| Every layer has identifying metadata | 2 | Maintained schema, Metadata validation; layer_manifest_schema and author_metadata tests |
| A schema describes metadata requirements | 9 | schema.rs meta-validation, shared-definition mutation, unknown-field and no-retrieval tests |
| Parse familiar formats into the same layer manifest data | 12 | Shared decoders/loaders; layer_manifest and author_metadata all-format/duplicate/extra-data tests |
| Schema-backed feedback and metadata semantics | 4 | Structured nested violations; root/field parity, setup failures, guided and flag diagnostics |
| Familiar string/object author representations | 6 | author.rs and maintained author fixtures; independent dependency_edit insertion assertions |
| Explicit metadata without prompts | 9 | InitArgs/flag_metadata; init, discovery-route and terminal tests |
| Minimal readable output | 4 | Shared encoders and complete pre-write validation; author_metadata and field_order |
| Explicit shorthand without inferred contacts | 7 | recognise/interpret boundaries and typed flag conflicts; punctuation, literal fallback and all bypass tests |
| Interaction mode without blocking automation | 11 | Mode/input-flag detection and parser-first dispatch; route/parser/terminal tests |
| Guided cliclack collection | 22 | Selected/custom/omitted typed state and optional input; controlled PTY default/retry/skip/cancel/no-early-write tests |
| Focused sources and aligned provenance icons | 10 | Scoped theme/contact provenance; unit and rendered-screen PTY wrapping/clearing/colour/format-isolation tests |
| Guided author-name discovery | 14 | Discovery ordering/grouping, local fallback; isolated Git, injected-source and bounded-process tests |
| Project-declared authors first | 9 | project.rs and maintained fixtures; same-entry filtering, source exclusions, read bounds and non-execution |
| Optional GitHub through existing authentication | 16 | Existing profile/page requests, viewer match, REST-email exclusion, safe URL construction; fake-request/partial-page/timeout tests |
| Associated optional contact selection | 6 | Frozen selected-group index and independent pools; actual-provenance/single-choice/custom-equality/omission PTYs |
| Conservative contact association and preparation | 6 | Git origin/scope/family buckets, environment pairing and shared snapshot; ambiguity/split-identity/freeze/responsiveness/cleanup tests |

Package inventory validation also passed: `cargo package --list --allow-dirty`
includes the new author module, maintained schema and author fixtures, while
excluding internal deepwork files. No additional runtime dependency was introduced.

**Final assessment:** Ready for archive within the approved scope and documented
exclusions. No archive or commit has been performed automatically.
