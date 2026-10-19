## 1. Lock in generated order and flexible input

- [x] 1.1 Add source-order assertions for init in all five formats, covering name-only output, each optional field independently, all metadata, reordered flags, and the shared guided encoder. Preserve exact accepted strings and optional-field omission.
- [x] 1.2 Add loader/schema regression cases for dependencies-first and interleaved metadata, `$schema`, and unknown fields across formats. Assert identical decoded data, no source rewrite, and no ordering-only CLI diagnostic; retain invalid-content and duplicate-key rejection controls.
- [x] 1.3 Add failing init-then-add integration coverage across all five formats: metadata remains ordered and the first dependency field follows it. Assert source placement, not merely parsed-object equality, using maintained fixtures outside `openspec/`.

## 2. Append missing YAML dependencies safely

- [x] 2.1 Replace block-YAML missing-field prepending in `src/layer_manifest/edit/yaml.rs` with a bounded parser/scanner-backed boundary after the final root value and its attached comments, before root footer comments and document-end markers. Keep insertion-only editing and shared candidate verification.
- [x] 2.2 Update dependencies-first expected outputs without removing their preservation cases. Cover leading/inline/indented/footer comments, nested final mappings and sequences, aliases, explicit keys, root/key properties, directives, BOM, document markers, Unicode, supported line endings, indentation, and final-newline presence for both YAML extensions.
- [x] 2.3 Add literal/folded final-block-string tests for strip, clip, and keep trailing-newline behaviour, meaningful blank lines, multiline quoted strings, and EOF variations. Demonstrate unchanged decoded values on safe edits and specific pre-staging refusal for genuinely unsafe source arrangements, without prepend or reserialisation fallback.

## 3. Verify the shared editing contract

- [x] 3.1 Extend cross-format edit tests to assert that missing dependencies follow all existing root entries, including interleaved `$schema` and unknown data. Cover flow YAML, JSONC closing comments/trailing commas, and TOML dotted assignments plus explicit tables/arrays of tables and their leading comments; adjust an editor only if those tests expose a mismatch.
- [x] 3.2 Verify that existing nonpreferred layouts and empty dependency arrays are extended in place, existing references stay ordered, new references append in argument order, and all-repeat requests create no edits or staging files. Retain atomic replacement, permissions, cleanup, and failure-before-write regressions.

## 4. Align guidance and validate the increment

- [x] 4.1 Add concise preferred-order and flexible-input guidance to the README's existing layer manifest section, align maintained general-usage examples and `AGENTS.md`, and preserve intentionally alternative test layouts and archived artifacts.
- [x] 4.2 Promote the proposed field-order resolution in `openspec/design-questions.md` once implemented. Extend the existing returning-to-projects scenario in `FOUNDATIONS.md` with the reported init-then-add consistency example, keeping exact field rules in OpenSpec; confirm no new glossary term is needed. Review revised documentation for first-time-reader clarity and explain any foundations revision.
- [x] 4.3 Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --locked`; validate this change with `openspec validate guide-layer-manifest-field-order --strict`. Confirm builds/tests still use no OpenSpec assets.
- [x] 4.4 Exercise init followed by add with captured output and a real terminal using bounded waits and child cleanup. Inspect the resulting files for readable metadata-first placement and unchanged comments, and review the unchanged help/prompts/results/errors and plain-output behaviour against the CLI's beauty, enjoyment, functionality, and ease-of-use goals.
