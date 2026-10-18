## Why

Hydragen is intended to become public, but the repository currently has no project
license file or Cargo license declaration. Establish explicit reuse terms before
publication and make the chosen license easy to find without relying on a badge
service to inspect a private repository.

## What Changes

- Adopt GNU GPL version 3 or any later version (`GPL-3.0-or-later`) for
  Hydragen-owned project content, with Leyton Blackler named in the project's
  copyright notice and existing third-party terms preserved.
- Add the complete, unmodified GPLv3 text in a root `LICENSE` file, a clear
  project-specific application notice, and matching Cargo package metadata.
- Add a linked license shield to the existing README badge row and a concise
  License section explaining the version choice and scope.
- Check ownership, third-party notices, and dependency license compatibility
  before applying the project-wide declaration; raise any unresolved issues
  rather than claiming rights over others' work.
- Keep repository publication, package publication, release distribution
  automation, and CLI behavior outside this change.

## Capabilities

### New Capabilities

- `project-licensing`: The authoritative license text, project application notice,
  Cargo license identity, and licensing scope for Hydragen.

### Modified Capabilities

- `readme-badges`: Extend the existing six-badge row with a seventh license badge,
  including its placement, reference links, accessible label, and truthful claim.
  Replace the current prohibition on a license badge once licensing is established.

## Impact

- Adds `LICENSE`; updates `README.md` and the `[package]` metadata in `Cargo.toml`.
- Updates licensing-related product documentation where needed, without changing
  the layer model, layer manifest schema, CLI output, runtime behavior, or dependencies.
- Covers reuse and distribution of Hydragen itself; using the CLI does not, by
  itself, impose the GPL on independently authored projects or layers.
- Does not change repository visibility or publish artifacts. The intended
  public release remains a separate owner-controlled action after implementation
  and any ownership or third-party issues are resolved.
