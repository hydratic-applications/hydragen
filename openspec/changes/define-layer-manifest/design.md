## Context

Layers are Hydragen's core building blocks. Every layer must have a manifest
declaring its name, author, and description. There is no implementation
yet; this change captures a small part of the format for discussion and testing.

## Goals / Non-Goals

**Goal:** Define and validate the three required metadata fields using the adjacent
[`layer-manifest.schema.json`](layer-manifest.schema.json).

**Non-goals:** Resolve dependencies, define how layers combine, or implement tooling.

## Decisions

### Agreed: every layer has identifying metadata

Root and dependent layers both require the same three fields. A layer's intended
use as a reusable starting point or a working project does not create a different
manifest category. No declaration of extensibility is needed.

### Agreed: manifest filename, location, and format

Every layer's manifest is a JSON file named exactly `layer.json` at the root of
its repository. The fixed name and location give developers and tools one
consistent place to find it. This applies to both root and dependent layers;
"repository root" means the top-level directory, not a root layer.

The schema validates the file's contents, not its filename or location. Future
manifest discovery must check those separately.

### Agreed: dependencies are declared in the manifest

The `layer.json` file must list the other layers that the layer depends on. This
keeps a layer's dependency declarations alongside its identifying metadata.

A root layer may omit the dependency list or include an empty list. Both mean
that the layer has no dependencies; neither form takes precedence over the other.
This allows authors to leave out an unnecessary declaration or state it explicitly.

The dependency field name and the format of each reference remain open, including
how a dependency identifies its source. The metadata-only schema does not yet
validate dependency declarations; it will need to be extended once that structure
is agreed.

### Draft choices: minimal validation and editor support

- Represent each required field as a string containing at least one non-whitespace
  character. This catches absent or blank metadata without inventing naming rules.
- Use a simple author string, which can name a person or organisation. An object
  with contact details or a list of contributors can be considered if needed;
  neither is necessary to identify the author now.
- Allow an optional `$schema` property so editors can locate the schema for
  validation and suggestions.
- Use JSON Schema Draft 2020-12. Do not invent a public schema address before
  there is a real place to publish it.
- Allow additional properties because this is a metadata-only draft. In
  particular, passing validation does not validate any dependency declarations.
  Rejecting unknown fields can be reconsidered when more of the format is defined.

These draft choices are proposals for review, not additional settled product
requirements.

### Deferred: versioning

The manifest does not define or require a `version` field. How versioning works,
including whether it belongs in the manifest, remains open. Requiring the field
now would assume part of that design before we have explored the workflow.

### Other fields worth considering later

- **Repository:** Helps developers find the maintained source, but a URL alone
  does not define how Hydragen retrieves or identifies a layer.
- **License:** Helps consumers understand reuse permissions, especially if layers
  are distributed outside a private workspace.
- **Keywords:** Could help discovery if searching a collection of layers becomes
  part of the workflow.

None of these additional fields is defined or required by this draft schema.

## Example

This root layer's `layer.json` omits the dependency list, meaning it has no
dependencies. It satisfies the initial metadata schema without establishing
layer-name syntax, dependency reference syntax, or a versioning policy.

```json
{
  "name": "shared-agent-rules",
  "author": "Example team",
  "description": "Coding-agent instructions shared across our projects."
}
```

## Risks / Trade-offs

- Allowing unknown fields also allows misspellings of optional fields. Required
  fields must still be present; revisit strictness as the manifest develops.
- This metadata draft does not establish how consumers select reproducible shared
  content. Source references and versioning remain separate design work.

## Open Questions

What naming syntax should be accepted? How should versioning work, and does it
belong in the manifest? How should dependencies be declared?
These remain open; no implementation plan is introduced by this draft.
