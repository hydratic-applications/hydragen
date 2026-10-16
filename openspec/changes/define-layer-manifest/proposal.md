## Why

Every Hydragen layer needs a manifest that identifies it. A small JSON Schema
makes the initial metadata requirements explicit and gives editors a way to check
them while we continue exploring layers.

## What Changes

- Require a name, author, and description for every layer.
- Require the manifest to be a JSON file named exactly `layer.json` at the root
  of the layer's repository.
- Draft a JSON Schema for those fields, with optional `$schema` editor support.
- Require `layer.json` to list the other layers the layer depends on.
- Treat an omitted dependency list and an empty list as equivalent: both declare
  a root layer with no dependencies.
- Keep versioning, dependency reference syntax, and other manifest details open
  rather than design the complete format now.

## Capabilities

### New Capabilities

- `layer-manifest`: Required identifying metadata for every layer.

### Modified Capabilities

None.

## Impact

Design artifacts only. This does not implement manifest discovery, a CLI, or
runtime validation.
