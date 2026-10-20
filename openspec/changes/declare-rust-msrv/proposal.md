## Why

Hydragen's README names a tested Rust compiler but does not tell developers the
oldest compiler the project supports. Declaring a minimum supported Rust version
(MSRV) and showing it in the README makes source-build requirements clear without
implying that only one Rust version works.

## What Changes

- Declare the MSRV with `[package].rust-version` in `Cargo.toml`, after verifying
  the chosen compiler against the existing locked dependencies and tests.
- Add a dynamic MSRV shield beside the Rust language badge, reading
  `[package].rust-version` from the public `main` branch's `Cargo.toml`, matching
  the current header style and linking to the build instructions.
- Keep `Cargo.toml` authoritative, with an automated check of the dynamic badge's
  source and query and the README's locally stated minimum. Badge updates follow
  merges to `main` and service caches, rather than happening instantaneously.
- Test the declared minimum in CI as well as retaining current stable tests and
  coverage. Do not move coverage tooling onto the older compiler.
- Treat MSRV as an additional independent compatibility check, updating the
  existing three-check documentation without changing test or coverage policies.
- Explain that the minimum is a source-build support baseline, not an exact
  compiler pin or a requirement to run an already-built executable.

## Capabilities

### New Capabilities

- `rust-version-support`: An authoritative, verified Rust support baseline and
  checks that prevent documentation from drifting from Cargo metadata.

### Modified Capabilities

- `readme-badges`: Permit a verified MSRV claim and add an accessible MSRV badge
  while preserving the current centred header and existing badges. Reconcile
  older URL styling and private-token wording with the already implemented
  header and later token-free coverage rules.
- `code-coverage-ci`: Narrow the pending three-policy enumeration to stable tests
  and coverage, permitting the independent MSRV check without changing their
  results, dependencies, thresholds, or upload policy.

## Impact

Affected files include `Cargo.toml`, `README.md`, `docs/coverage.md`, the CI-related
entries in `docs/going-public-checklist.md`, CI configuration, and a small
documentation consistency check with tests in maintained locations outside
`openspec/`. No CLI behaviour, layer manifest format, runtime network access,
dependency upgrade, publication, or repository visibility change is needed.

The `configure-codecov` change already updates the same badge requirements to
match the implemented HTML header. This change builds on that layout and must
retain its coverage badge and service-access rules when the deltas are combined.
Its coverage capability is further refined by `separate-test-and-coverage-checks`.
The coverage delta here builds on that pending refinement; sync those two changes
in that order before this one's overlapping deltas. Do not archive unrelated
work automatically or claim that the new job already exists in user documentation.

Integration update: those coverage changes have since been synced and archived
on `main`. The final integration uses main's maintained specifications as its
baseline, retaining later path-validation and icon requirements instead of
replaying historical deltas over them. Both compiler routes must be reverified
with main's newer implementation and dependencies.
