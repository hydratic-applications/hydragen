# Extension-only init format labels

## Decision

Label the guided `hydragen init` Manifest format choices `yaml`, `yml`, `toml`,
`json`, and `jsonc`, without a leading dot or `layer.` prefix. Use the same labels
in the completed prompt. The question already explains that the user is choosing
a format; repeating the shared filename prefix makes the options longer without
helping distinguish them. These labels also match the existing `--format` values.

Keep the choice order, initial YAML selection, generated `layer.<extension>`
filenames, and manifest discovery priority unchanged. The final result still
reports the full created path. This is a presentation change only, consistent
with the foundations' developer-experience value; no foundation or glossary change
is needed.

This supersedes only the exact-filename labels in the earlier manifest-format
decision. The updated requirements are recorded in the
[manifest-format CLI interaction specification](../support-layer-manifest-formats/specs/cli-interaction/spec.md#requirement-collect-metadata-through-a-guided-cliclack-session).
