# CLI visual identity verification

## Baseline and scope

Implementation began after the author exited explore mode on 2026-10-17. The
existing uncommitted brand/foundations/design documentation belongs to this
session and is preserved. No unrelated checkout changes were discarded, and no
upstream merge, publication, or archive was performed during implementation.

Environment recorded during implementation: macOS 26.5.2 (25F84), Rust 1.96.0,
Cargo 1.96.0. The checkout base is
`fea2feea5ac481588985fca4859fc4c31f290615`; verification used the implementation and
documentation as uncommitted working-tree changes on that base, before the
subsequent request to create a feature branch and commit.

Before runtime edits, `cargo test --locked` passed **171 tests**: 26 library,
23 binary, 40 captured init, 7 lifecycle, 20 terminal integration, and 55 other
manifest/schema/codec tests. This is baseline evidence, not evidence that the new
presentation has been implemented.

The initial and detailed implementation plans received independent reviews. The
library audit confirmed that the locked cliclack/console public APIs can support
the design without new dependencies or a replacement interaction renderer.

## Contract checks completed before implementation

- Both delta specifications include the required complete modified requirement
  blocks and preserve unrelated help/input/filesystem requirements.
- `AGENTS.md` explicitly aligns the approved conditional flag-only styling and
  guided stderr-only completion contracts.
- `openspec validate define-cli-visual-identity --strict` passed.
- Local documentation links and decision anchors were checked.
- `git diff --check` passed before implementation.

## Runtime verification

The completed implementation passes **186 tests**: 26 library, 32 binary,
41 captured init, 7 lifecycle, 25 terminal integration, and 55 other
manifest/schema/codec tests. The original 171-test baseline remains covered.

Final passing commands:

```sh
cargo build --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
openspec validate define-cli-visual-identity --strict
git diff --check
```

The independent runtime review approved the implementation and separately reran
95 affected tests before a final added regression. Its only non-blocking finding,
missing direct guided-completion failure coverage, was addressed through a small
injected-outro seam and test. The reviewer approved that follow-up and independently
ran it. Both `Interrupted` and `BrokenPipe` failures preserve the full manifest
path, completed-creation wording, original error details, and non-cancellation
classification.

No dependencies, manifest/schema/library logic, or lifecycle test behaviour were
changed. Runtime motif and presentation assets live in `src/presentation.rs`, not
OpenSpec or the temporary preview files.

## Actual Terminal checks

The built `target/debug/hydragen` was executed in disposable directories outside
the repository in macOS Terminal, using SFMono-Regular at size 11. The native
window was 100 columns by 48 rows for normal checks and 32 columns for the narrow
check; its width was restored afterward. These were real CLI interactions, not
the earlier static ANSI studies.

- **Guided, normal width:** the motif/title/version/operation opening appeared
  once, with the two-cell visible inset and normal-foreground operation
  description. A three-space Name produced the cross-marked correction state;
  the directory remained empty. Backspacing and entering `shared-rules` resumed
  the flow. Author was skipped, Description supplied, and default YAML accepted.
  The created manifest contained the expected name and description without author.
  The ending reported once: `Layer initialised, manifest created at: layer.yaml`.
- **Narrow and colour-disabled:** at 32 columns with `NO_COLOR=1`, init used the
  text-only identity and operation opening and still asked for Name. Ctrl-C
  returned to the shell with one cancellation/no-files-changed reason. The
  disposable directory remained empty.
- **Compact terminal success:** flag-only JSONC init produced the checkmarked
  single-line success naming `layer.jsonc`, with no header or session frame. The
  JSONC file contained the supplied name.
- **Warning and refusal:** adding an existing YAML fixture alongside that JSONC
  file produced one duplicate warning, then the occupied-manifest error. Priority,
  no-merge policy, safe recovery, and no-change information were retained. Both
  existing files remained unchanged, with no prompt or branded opening.

Native Terminal text was inspected through its application read-back. A direct
window-image capture failed (`could not create image from window`), so no GUI
screenshot or independently inspected palette/contrast result is claimed. The PTY
tests verify the actual red/green/yellow SGR roles, bold/dim span boundaries,
editable retries, and output channels. Text read-back and SGR assertions do not
prove pixel rendering or contrast in every font or light/dark palette.

## Current-directory collision refinement

After the initial implementation commit `4ba0d3b`, the occupied-manifest CLI
headline was shortened to `layer.yaml already exists in the current directory.`,
using the actual occupied filename for every format and retaining the recovery
lines. This adapts the structured preflight or creation error only at the CLI
boundary; library errors and other diagnostic paths are unchanged.

The full 186-test suite and all-target clippy pass after this refinement. Added
unit coverage checks every format for both error wrappers and preserves other
full paths; captured and PTY tests retain entry-preservation checks and now assert
the shorter headline. The actual built CLI displayed the shorter refusal in the
normal Terminal window, including when JSONC was requested but YAML was occupied.

## Behaviour and presentation coverage

- Header geometry/version, exact-fit and too-narrow/unknown/basic fallbacks, no
  header on flag-only/help/version/usage/preflight failures, and once-per-session
  identity are covered by unit and PTY tests.
- All five formats identify the actual created filename. Guided success has empty
  stdout; flag-only success is on stdout. Warnings/errors stay on stderr.
- CI, mixed redirected streams, captured forced-colour output, `NO_COLOR`,
  `CLICOLOR=0`, and `TERM=dumb` retain the selected conservative policy. Colour
  controls cannot enable questions or override capture/CI safeguards.
- Red-cross validation, preserved editable values, optional clear-to-skip, and
  red cancellation retain native cliclack interaction. Cancellation reporting
  requires observed Cancel plus the explicit successful reason delivery; OS/custom
  interruptions and failed delivery remain reportable.
- Duplicate warning ordering/nonfatality, exact-target exclusive creation,
  all-format initial refusal, alternate-format races, schema-derived validation,
  no early writes, and truthful post-write errors remain covered.

## Review against the four DX goals

- **Beauty:** the running CLI preserves the selected balanced identity and a
  coherent status/icon hierarchy; colour spans are tested, with font/palette
  inspection limits noted above. Braille cell seams remain an accepted limitation.
- **Enjoyment:** one-shot output stays compact, completion is not repeated, and
  cliclack retains the familiar editing/retry experience. This is a design review,
  not a claim of new usability-study evidence.
- **Functionality:** all 186 tests pass, including the unchanged safety and
  manifest-content coverage and the intentionally revised output contracts.
- **Ease of use:** defaults, optional skips, selected format, correction actions,
  cancellation, successful creation, and safe recovery remain explicit without
  relying on colour alone.

Remaining limits are cross-platform/palette/font verification, automatic braille
support detection, and cliclack's documented bare-interruption ambiguity after an
observed Cancel. Basic-terminal guided editing still uses cliclack cursor controls;
only its opening/status fallback is undecorated. None of these limits is hidden
behind a claim that static previews constitute runtime verification.
