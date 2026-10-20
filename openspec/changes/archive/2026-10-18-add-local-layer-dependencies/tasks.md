## 1. Shared content rules and compatibility

- [x] 1.1 Extend the maintained layer manifest schema with the agreed lowercase/scoped name definition, optional ordered dependency string array, explicit local reference syntax, and strict exact SemVer published references. Keep filesystem checks separate and do not add ranges, tags, or lockfiles.
- [x] 1.2 Route field checks, direct parsed-content validation, all five format loaders, and complete candidates through those same schema definitions. Test scoped names, full-name length, prerelease/build identifiers, invalid reference shapes, and unchanged unknown-field acceptance.
- [x] 1.3 Apply the stricter name definition to init flags, guided answers, and directory defaults without normalisation. Preserve optional metadata semantics, prompt-specific correction guidance, valid explicit overrides, input-route selection, and existing file safety.
- [x] 1.4 Deliberately update project-owned fixtures and expectations using newly invalid names. Add regressions for uppercase/whitespace rejection, unchanged valid values, schema-invalid directory defaults, and no duplicate content predicates.

## 2. Current-layer and local-source discovery

- [x] 2.1 Add an ancestor-search operation for add using shared single-directory discovery. Test nearest selection through the filesystem root, Git-boundary crossing, missing layers, inspection failures, and no fallback past occupied invalid selections; leave init's guard unchanged.
- [x] 2.2 Interpret explicit local paths from the selected layer manifest directory while retaining original reference strings. Test root/nested invocation equivalence, absolute paths, redundant components, spaces, native forms, bare-name rejection, and recognisable unsupported platform forms.
- [x] 2.3 Retain source text for editing and refuse a known symlinked current layer manifest without changing its link or target. Preserve read-only dependency layer manifest symlink support and reject non-regular targets without blocking.
- [x] 2.4 Reuse discovery/loading for current and target layers, carrying discovery diagnostics through failures and reporting duplicates once per affected directory, including repeated or directory-symlink references.

## 3. Reusable direct-relationship verification and batch planning

- [x] 3.1 Expose structured read-only verification usable independently of the CLI. Verify the current layer and all existing direct local dependencies, parse existing published names offline, and verify supplied local targets without target-ancestor lookup or dependency-chain traversal.
- [x] 3.2 Enforce name uniqueness across the current layer, existing declarations, and new arguments. Test self-name conflicts, duplicate existing entries, local/published collisions, and different directories claiming the same name without implicit replacement.
- [x] 3.3 Build an ordered batch plan that appends only new references, preserves existing order, retains existing reference spellings on harmless repeats, and keeps first occurrences of repeated new sources.
- [x] 3.4 Aggregate independent supplied-source errors in argument order and retain useful existing-dependency failures without speculative downstream checks. Test whole-batch refusal and no mutation on any verification error.
- [x] 3.5 Track dependencies actually added separately from those present before invocation. Test mixed outcomes, repeat-only plans, alternate paths to the same directory, and repeated new arguments appearing only in the added outcome.

## 4. Source-preserving edits in every supported format

- [x] 4.1 Evaluate maintained editing facilities for JSON/JSONC, TOML, and YAML/YML against comment and unrelated-source preservation. Record the selected approach and any genuinely unsupported construct in the design before relying on it; surface requirement conflicts rather than weaken them.
- [x] 4.2 Implement JSON and JSONC dependency insertion/append with strict format rules, comment/trailing-comma handling where supported, unchanged unrelated text, and no generic reserialisation fallback.
- [x] 4.3 Implement TOML dependency insertion/append with comment and unrelated-text preservation, including valid placement around tables and existing string arrays.
- [x] 4.4 Implement YAML/YML dependency insertion/append for ordinary block/flow arrays and comments. Preserve anchors/aliases where safe and explicitly refuse edits that would change unrelated meaning or discard comments.
- [x] 4.5 Reparse edited candidates with established decoders, validate complete content with the maintained schema, and verify that only the intended dependency append changed decoded data before staging. Test absent/empty dependencies, all formats, unknown fields, attached comments, line endings, and unsupported-edit no-write behaviour.

## 5. Atomic replacement and failure boundaries

- [x] 5.1 Implement hidden sibling-file staging, complete writes, original-permission preservation, staged-file synchronisation, and atomic replacement without in-place truncation or non-atomic fallback. Keep presentation and parsing outside this operation.
- [x] 5.2 Clean up staging files on success and handled failures, and accurately surface cleanup failures. Test unchanged originals on pre-replacement failures, permission handling, and no staging for all-repeat plans; document abrupt-termination leftovers.
- [x] 5.3 Preserve the accepted no-lock/no-stale-check boundary and add tests that verify whole-file replacement without claiming concurrent lost-update or unconditional power-loss protection.
- [x] 5.4 Exercise completion-report failures after a successful replacement and ensure the error explains that the layer manifest was already updated rather than claiming no changes or cancellation.

## 6. CLI integration and presentation

- [x] 6.1 Add clap parsing and help for one-or-more positional sources, including the declaring-directory path base, explicit-path requirement, and unsupported published additions. Keep all execution argument-only and accept global `--non-interactive` before or after the subcommand.
- [x] 6.2 Present compact destination-layer-name results with separate counted `Added` and `Already present` name lists, omitted empty groups, input-order stability, and a no-change result for repeat-only execution. Do not display the layer manifest path by default or list unrequested dependencies.
- [x] 6.3 Present one clear batch-failure headline and readable source-specific groups with reasons, diagnostic paths, and correction guidance. Integrate deduplicated warnings before dependent failures and avoid unsupported-command recovery advice.
- [x] 6.4 Apply shared terminal/plain styling and stdout/stderr policies without branding or prompt frames. Test CI, redirected/mixed streams, colour disables/force, dumb terminals, interaction controls, and help/version/usage with no layer discovery.
- [x] 6.5 Review captured output and actual bounded terminal sessions for mixed success, all-repeat success, multiple failures, long/scoped names, and narrow widths. Assess beauty, enjoyment, functionality, and ease of use explicitly; clean up child processes and record remediation for observed gaps.

## 7. Documentation and completion checks

- [x] 7.1 Revise the concise README for add, nested invocation, path preservation, exact published-declaration-only support, the breaking name policy, and accepted replacement/concurrency limitations. Avoid an exhaustive implementation summary.
- [x] 7.2 Update relevant glossary entries and agent guidance for agreed names, declarations, validation reuse, and command boundaries. Check glossary cross-links and review whether foundations need clarification without changing their core philosophy; explain any warranted revision.
- [x] 7.3 Preserve deferred registry/publishing, guided search/version selection, and explicit upgrade/version-change work. State that future published add must reject a different existing version rather than overwrite it; do not implement those follow-ups or narrow the broader validate proposal.
- [x] 7.4 Run formatting, clippy with warnings denied, locked tests, packaging/build independence checks, and the new captured-output/terminal/failure tests. Keep fixtures and implementation assets outside OpenSpec, reconcile overlapping presentation deltas, and review changed documentation from a first-time reader's perspective.
