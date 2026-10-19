# Layer manifest schema and format integration verification

Verified on 2026-10-17 on macOS with Rust 1.96.0.

## Result

`jammy-gecko` was fast-forward merged into `main` from `e2d50e8` to `9c91f33`
without conflicts. This brought in `e7a4d28` (layer manifest formats) and `9c91f33`
(a proposal for a future validation command). The proposal was preserved but not
implemented. The separate `merry-egret` checkout was not modified.

The integrated implementation uses `schemas/layer-manifest.schema.json` for
content validation after decoding YAML, YML, TOML, JSON, or JSONC. The same schema
governs field feedback, defaults, typed metadata, encoders, and complete loaded
documents. Additional data remains available after validation. YAML remains the
init default; selection priority, output styles, no-fallback loading, and the
prepared/exclusive-create lifecycle are unchanged.

The originating archive is untouched. Source and tests have no `openspec/`
references, and source has no remaining `is_whitespace` content predicate.
The schema implementation and its tests never load planning artifacts.

## Assessment findings and corrections

- **Complete-document validation:** The previous loader checked only extracted
  identifying fields. It now validates the original decoded document first,
  including `$schema`, without losing additional fields or choosing another schema.
  Invalid `$schema` values are an intentional tightening of prior loader input.
- **Whitespace compatibility:** JSON Schema's `\S` and Rust's Unicode whitespace
  predicate disagree on U+0085 and U+FEFF. The user approved an explicit Unicode
  White_Space class in the maintained schema, preserving existing CLI behaviour.
  Both supported regex engines pass the regression matrix. The archive is unchanged.
- **Truthful diagnostics:** Setup failures remain distinct from invalid content;
  they do not become blank-field errors, directory-name override guidance, or
  prompt retries. A reviewed-rule fingerprint enables friendly non-blank wording
  without copying its regex into Rust. A changed rule without renewed diagnostic
  approval falls back to its actual schema error; validity is unaffected.
- **YAML integer overflow:** Recognized extension spellings could become strings
  on overflow. The user approved rejection of overflowing numeric spellings while
  preserving recognized in-range forms and quoted/explicit-string controls.
- **YAML binary conversion:** Binary-tagged data could silently become UTF-8 text,
  including keys. The user approved rejection before conversion. Review caught
  additional local and legacy tag identities; regressions now cover all four
  resolved identities and 20 corresponding spellings recognized by the pinned
  parser, including directives, keys, anchors, and nested content.

Independent review approved the integration plan, corrected validator foundation,
runtime adapters, and corrected parser boundary before final verification.

## Local build and regression checks

All passed:

```sh
cargo fmt --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
cargo build --release --locked --offline
git diff --check
openspec validate use-maintained-manifest-schema --strict
```

The full suite has **161 passing tests**: 26 library, 18 binary, 37 captured init,
7 init lifecycle, 18 real pseudo-terminal, 31 layer manifest/codec, 6 schema integration,
11 YAML integer, and 7 YAML scalar tests. Cases within these tests cover all five
formats, metadata combinations, wrong types/null, invalid `$schema`, additional
data preservation, selected-only schema failure, exact output/string preservation,
invalid input without writes, cancellation, concurrent creation, symlinks, and
post-write observation/reporting failures.

The YAML characterization baseline contained 4,324 records. After correction,
all 1,008 overflowing integer cases were rejected, quoted/string controls retained
their meanings, and all 84 writer-string encodings round-tripped. The expanded
binary-tag regressions additionally cover local/legacy forms omitted from the
first characterization. These checks occur before shared content validation;
the schema does not reconstruct erased source types.

`jsonschema` resolves to 0.58.6 and `sha2` to 0.10.9, both with default features
disabled. Feature inspection found no enabled HTTP/file retrieval, TLS, async
resolution, or `jsonschema` macro feature in the schema dependency boundary.
Ordinary transitive derive/procedural-macro dependencies are not prohibited.

## Fresh-source, package, and executable-only verification

A disposable source copy contained Cargo files, README, source, tests, and the
maintained schema, but no `openspec/` or existing build output. A fresh target
directory successfully completed locked offline build and all 161 tests.

`cargo package --list --allow-dirty --offline` included the maintained schema;
`cargo package --locked --allow-dirty --offline` successfully verified the packaged
crate. Existing package-metadata warnings were not treated as product changes.

Only the built executable was copied into a separate directory, with no source
or schema sidecar. Valid flag-only init succeeded for all five formats. Invalid
metadata exited nonzero without creating files in all five cases. Help/version
also succeeded. A macOS sandbox invocation with network operations denied
successfully initialized a layer.

The existing committed pseudo-terminal harness was reused against that isolated
executable by changing only the test executable path in the disposable test copy.
All 18 terminal tests passed, including guided defaults, invalid retries, optional
skips, cancellation, format selection, stream separation, and bounded child cleanup.
The same 18 scenarios also passed against the isolated release executable.
This is scripted real-terminal evidence, not a human visual/accessibility audit.

## Actual rebuilt-schema failure probes

Only the disposable schema asset was changed and rebuilt for these probes; the
production CLI has no runtime schema override.

- An added whole-object `maxProperties` constraint caused direct library init to
  fail before inspecting a nonexistent directory and prepared creation to fail
  before opening its target. No files were created.
- A malformed bundled regex caused explicit-name, directory-default, and eligible
  guided invocations to fail before prompts or writes. A whitespace-only directory
  did not hide the setup failure behind name-override guidance. Help/version
  remained usable.
- An unavailable HTTP reference failed with the explicit external-retrieval-disabled
  diagnostic. A live local HTTP schema server observed **zero requests**.
- A reference to an existing, valid external schema file also failed with retrieval
  disabled rather than reading it. Help/version remained usable for both variants.

The disposable schema source was restored after each probe. No working-checkout
schema, OpenSpec files, or other agent work was removed or renamed.

## Performance observations and limits

The release executable was 7,181,456 bytes. Ten local process-level samples gave
a help median of 2.271 ms and an init median of 4.271 ms. One first help launch
took 445.815 ms; the init samples ranged from 3.860 to 4.824 ms. These observations
include process creation and local OS effects, are not a controlled benchmark,
and have no measured pre-change baseline. They do not prove cross-platform
performance. Cached-validator behaviour is covered separately by tests.

The schema remains metadata-only and permits uninterpreted additional fields.
The library reports the first schema violation, not an aggregate. Parser upgrades
must recheck the supported numeric grammar, tag identities, and regression matrix.
No public schema hosting, automatic editor integration, new validation command,
or unrelated CLI-presentation work was added.

## Local evidence

Detailed machine-local logs were retained under the approved temporary workspace:

```text
/audit/
  hydragen-schema-standalone-qwuyraih/results.json
  hydragen-schema-standalone-qwuyraih/boundary-results.json
  hydragen-schema-standalone-qwuyraih/release-terminal-results.json
  hydragen-yaml-characterization/results.jsonl
  hydragen-yaml-characterization/results-after.jsonl
```

These are supplemental execution records, not required build/test/runtime inputs.
This document preserves the conclusions without depending on those temporary files.
