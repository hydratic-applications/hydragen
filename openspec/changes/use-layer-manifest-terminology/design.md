# Use layer manifest terminology

## Decision

Use **layer manifest** in full for the file that identifies a Hydragen layer,
rather than shortening the term to “manifest”. This distinguishes it from other
tools' configuration files, such as Cargo's manifest (`Cargo.toml`). Use ordinary
sentence casing: “layer manifest” in prose and “Layer manifest” at the start of a
sentence, heading, or prompt.

Apply the term to the glossary, foundations, README, agent guidance, OpenSpec
documentation, CLI help and diagnostics, schema descriptions, Rust names, and
tests. The Rust module is `layer_manifest`; types use names such as
`LayerManifestFormat`, `LoadedLayerManifest`, and `CreatedLayerManifest`.
`InitError::ExistingLayerManifest` names the occupied-path error.

This is a terminology change, not a change to file contents, supported `layer.*`
filenames, validation rules, discovery priority, input routes, or creation safety.
Rust callers must update the renamed module, types, and error variant; aliases
using the shorter terminology are not retained.

## Historical references

Update archived documentation's terminology and links without changing its
decisions or claiming that old requirements describe current behavior. Preserve
archived change identifiers and directory names, recorded terminal-output JSON
(including its field names and case identifiers), and the archived schema
snapshot. These remain historical records, not implementation inputs. Cargo's
own terminology and external documentation URLs also remain unchanged.
