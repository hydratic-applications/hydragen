# Changelog

## Unreleased

### Changed

- Layer names now follow the shared lowercase naming rules. Previously accepted
  nonblank names containing uppercase, spaces or other disallowed characters are
  rejected by init and layer manifest loading. Correct existing names deliberately;
  Hydragen does not silently convert them. See the
  [layer-name rules](README.md#the-layer-manifest).
- Rust callers constructing `LoadedLayerManifest` must now supply `source`, the
  original UTF-8 layer manifest text, including comments and formatting.
- Rust callers constructing `InitCreationError` must now supply `state`:
  `InitCreationState::NotCreated` when this invocation did not create the target,
  or `InitCreationState::PossiblyPartial` when exclusive creation succeeded but
  writing failed. These states do not assert that no other process changed files.
- Exhaustive Rust matches on `InitError` and `MetadataError` must handle their
  `InvalidName { guidance }` variants.
- `hydragen --version` and `hydragen -V` now print only the version number, without
  the executable name or a `v` prefix.
