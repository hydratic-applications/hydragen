## Why

Guided init can suggest an author name, but it discards contact details already
declared alongside that name. Extend the `fresh-gecko` author work with familiar
npm-style author metadata and optional, source-labelled contact choices, reducing
typing without guessing which identity or contact details a developer wants to
publish in a layer manifest.

## What Changes

- Accept author as a non-blank string, including npm's `Name <email> (URL)`
  shorthand, or an object with required `name` and optional `email` and `url`.
  Preserve existing non-blank strings and read-only source layouts.
- Have init emit the object form whenever author is supplied, with no
  representation-choice question. **BREAKING:** generated author output changes
  from a string to an object, and the library metadata API must represent both.
- Recognise name/email/URL shorthand in explicitly entered Author input and
  `--author`. Skip later contact prompts for values already supplied in shorthand.
  Treat input that is not recognised shorthand as the literal author name.
- Otherwise ask Author email after Author, then Author URL, before Description.
  Each contact step conditionally offers suggestions, custom entry, and Skip;
  without suggestions it is an empty optional text input.
- Retain contact details on individual source records before grouping equal
  author names. Offer all distinct contact values associated with the selected
  group, not contact details belonging to other names. Custom author entry does
  not infer contacts, even when its text matches a suggestion.
- Skipping Author, including empty custom entry, omits the entire author and
  bypasses both contact steps, as clarified during proposal preparation.
- Extend existing project/Git/GitHub discovery without broadening authentication
  or adding new identity providers. Use public GitHub email only, and offer a
  declared website before the GitHub profile URL. Keep preparation bounded and
  freeze contacts together with names before Author.
- Provide explicit `--author-email` and `--author-url` flags for automation, with
  the same prompt-free, no-discovery policy as existing input flags.

## Capabilities

### New Capabilities

None. This extends existing metadata, initialisation, and interaction contracts.

### Modified Capabilities

- `layer-manifest`: Support string/object author values and shared schema-backed
  nested validation without rewriting existing source or tightening legacy strings.
- `layer-initialization`: Generate structured authors in all five formats, interpret
  explicit shorthand, and support complete flag-only contact input.
- `cli-interaction`: Collect contacts in order, retain source associations through
  grouping, and extend bounded optional discovery and conditional selectors.

## Impact

- Maintained schema and schema-backed field APIs in
  `schemas/layer-manifest.schema.json` and `src/layer_manifest/schema.rs`.
- Typed metadata, loading, initialisation serialization, and source-preserving add
  verification in `src/lib.rs`, `src/layer_manifest.rs`, and related edit modules.
- CLI flags/help, guided collection/theme, and discovery in `src/main.rs`,
  `src/interaction.rs`, and `src/author_discovery/`.
- Maintained fixtures, captured-output and actual-terminal tests, README usage,
  glossary, agent guidance, foundations scenarios, and design-question decisions.
- No npm/Node runtime, new required network service, private GitHub email lookup,
  account mutation, migration command, or changes to init/add filesystem safety.
- Planning only in this change-creation session; implementation is a later apply.
