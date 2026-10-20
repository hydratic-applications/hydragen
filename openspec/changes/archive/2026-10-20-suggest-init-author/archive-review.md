# Archive review: guided init author suggestions

## Completion decision — 2026-10-20

Hydragen's maintainer explicitly accepts this change as **completed with accepted
deferred work** and authorises its archive. The useful guided-author feature and
its documentation are complete for that accepted scope. This decision supersedes
the earlier “not ready to archive” and “CRITICAL before archive” conclusions; it
does not turn the underlying findings into passing behaviour or relax a normative
requirement.

The checklist remains **25/27**. Tasks **3.3 and 4.5 stay unchecked**, with their
original descriptions and deferred regression assertions retained. Their
upstream-dependent portions are transferred to
[Hydragen #14](https://github.com/hydratic-applications/hydragen/issues/14), the
authoritative future completion path. Future work closes that issue against its
acceptance checks; it does not need to reopen or rewrite this archived checklist.
The maintainer accepted both incomplete task records and archiving without another
spec sync because all eight delta requirement blocks already match the main specs.

## Reviewed runtime and CI evidence

The [reviewed public proof](https://github.com/hydratic-applications/hydragen/pull/15#issuecomment-6280696832)
records [run 39349524907](https://github.com/hydratic-applications/hydragen/actions/runs/39349524907)
for exact runtime head `3a79aa9b0364a7b478f89107acd5fe6adf648095`, against base
`328644ec72012e11ae01ffbf7213694ebd40e7be`.

| Check | Reviewed outcome |
| --- | --- |
| Normal and instrumented Linux Rust suites | Each passed 673 tests, with 0 failures and 3 ignored entries: two #115 deferrals and the child-only terminal driver. |
| Coverage-policy suite | 42 passed. |
| Retained coverage report | Run/head/base identity and report digest matched. All 32 inventoried Rust source hashes and line counts matched the immutable runtime head; LCOV contained the exact source inventory. |
| Renamed terminal-test source | `src/interaction/author_terminal.rs` has 593 covered / 656 measured lines. Its unchanged 32,110-byte body has SHA-256 `c0e9a0742a1c4b074cec77b68c961e5eab47c8a7f9d983443410f62cf4d830b1`. |
| Total coverage | 13,734 / 14,131 lines = 97.19%, passing the unchanged 80% gate. |
| Changed-line coverage | 3,193 / 3,375 lines = 94.61%, passing the unchanged 80% gate. |
| Independent evidence review | Accepted the artifact, source completeness, gate counts and publication evidence. The unchanged coverage helper reproduced both gate results from the retained artifact. |
| Codecov upload | Correctly skipped under the main-only upload policy for a PR. |

The report includes existing test/helper code. Restoring the renamed helper's
visibility changes measured scope; it is not evidence of improved production-only
testing. The earlier missing-record failure in run 39348268145 remains documented
in [verification.md](verification.md#pr-ci-coverage-export-failure-and-approved-compatibility-fix).
These results belong to the exact runtime head above, not to a later archive-only
documentation commit or its CI run. No fresh runtime tests were needed for this
documentation-only archive.

## Retained exceptions and follow-up

- **Native selector display defect:** [cliclack #115](https://github.com/fadeevab/cliclack/issues/115)
  still affects long membership-only Author lists. [Upstream PR #116](https://github.com/fadeevab/cliclack/pull/116)
  is not adopted here. #14 owns adopting a suitable upstream fix without vendoring,
  forking or locally patching cliclack, removing the two issue-specific ignore
  markers, passing the original regressions and repeating full validation before
  removing the [README limitation](../../../../README.md#guided-setup). Both ignored
  tests remain unchanged; this archive does not claim full conformance.
- **Nonblocking evidence gap:** project todo
  `43984919-c934-4122-88a2-c77ccabf64f9` retains the direct equal-Git/valid-GitHub-profile
  grouping fixture follow-up. Existing shared grouping evidence remains indirect
  for that exact pair; it is not a demonstrated runtime defect or an archive blocker.
- **Windows:** compilation/runtime validation remains unverified. Linux results
  do not certify Windows-specific process, file or console behaviour.

## Archive method and preservation

All four planning artifacts were complete. The three delta files contain eight
requirements and 76 named scenarios, already synced exactly to the main specs.
The active change passed strict validation before moving. Archive used validation
enabled, without `--no-validate`:

```sh
openspec archive suggest-init-author --yes --skip-specs --json
```

The CLI archived the folder as `2026-10-20-suggest-init-author` and reported
`specsUpdated: false`. `.openspec.yaml` and all three normative delta files are
preserved byte-for-byte; no schema snapshot is part of this change. The maintained
specifications, runtime, tests, README, dependencies and workflow remain unchanged.
The 20 pinned `734ce99` source URLs retain their historical meaning. Relative
document links and incoming AGENTS/design-question links are repaired for the new
folder depth. Pre-archive validation and byte comparison establish preservation;
this does not claim the CLI separately validated an archived directory as an active
change. PR #15 is not merged; merge remains outside this archive lane.

Pre/post-move SHA-256 checksums matched:

| Preserved file | SHA-256 |
| --- | --- |
| `.openspec.yaml` | `b552ab6519df8f66b705724100f565c5cd842bc9389aeaa177da9cec3d02a685` |
| `specs/cli-interaction/spec.md` | `4663d2d1cff2bb281eef323630ad4f779fa2bd0995e08630c9e4efc3c02d6f1a` |
| `specs/cli-experience-principles/spec.md` | `4f2b125d30f4f11d9d12b7818b675392757822a5db298ba515c6a9e935e03782` |
| `specs/layer-initialization/spec.md` | `2b8cdb170fe8818c3dc025e9ec97357272a4b6749eb596193b9b3253550ee852` |

After moving, all nine maintained specs passed strict validation and remained
byte-identical to `3a79aa9`; all eight archived delta blocks still matched them.
`openspec list --json` no longer listed this as an active change. All 100 actual
local Markdown file/anchor links checked across the archive and incoming guidance
resolved. The two unchecked task records, all 27 checkbox descriptions, 20 pinned
historical URLs and all runtime/test files were preserved. Other archives were
not changed.
