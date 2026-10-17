## Context

Layers are Hydragen's core building blocks. Every layer must have a layer manifest
declaring its name, with optional author and description metadata. There is no
implementation yet; this change captures a small part of the format for discussion
and testing.

## Goals / Non-Goals

**Goal:** Define and validate the required name and optional author and description
using the adjacent [`layer-manifest.schema.json`](layer-manifest.schema.json).

**Non-goals:** Resolve dependencies, define how layers combine, or implement tooling.

## Decisions

### Agreed: every layer has identifying metadata

Root and dependent layers both require a name. Author and description are optional
for either: a name is enough to initialise a layer, without requiring extra
descriptive information. Name and any supplied author or description must be
strings containing at least one non-whitespace character; optional means a field
can be omitted, not that it accepts blank values or `null`.

The author string can name a person or organisation. An object with contact details
or a list of contributors can be considered later; neither is needed to identify
an author now.

A layer's intended use as a reusable starting point or a working project does not
create a different layer manifest category. No declaration of extensibility is needed.

### Agreed: layer manifest filename, location, and format

Every layer manifest is a JSON file named exactly `layer.json` at the root of
its repository. The fixed name and location give developers and tools one
consistent place to find it. This applies to both root and dependent layers;
"repository root" means the top-level directory, not a root layer.

The schema validates the file's contents, not its filename or location. Future
layer manifest discovery must check those separately.

### Agreed: dependencies are declared in the layer manifest

The `layer.json` file must list the other layers that the layer depends on. This
keeps a layer's dependency declarations alongside its identifying metadata.

A root layer may omit the dependency list or include an empty list. Both mean
that the layer has no dependencies; neither form takes precedence over the other.
This allows authors to leave out an unnecessary declaration or state it explicitly.

The dependency field name and the format of each reference remain open, including
how a dependency identifies its source. The metadata-only schema does not yet
validate dependency declarations; it will need to be extended once that structure
is agreed.

### Draft choices: editor support and schema extensibility

- Allow an optional `$schema` property so editors can locate the schema for
  validation and suggestions.
- Use JSON Schema Draft 2020-12. Do not invent a public schema address before
  there is a real place to publish it.
- Allow additional properties because this is a metadata-only draft. In
  particular, passing validation does not validate any dependency declarations.
  Rejecting unknown fields can be reconsidered when more of the format is defined.

These draft choices are proposals for review, not additional settled product
requirements. The required name, optional author and description, and non-blank
string validation are agreed above.

### Deferred: versioning

The layer manifest does not define or require a `version` field. How versioning works,
including whether it belongs in the layer manifest, remains open. Requiring the field
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

The smallest valid metadata object contains only `name`:

```json
{
  "name": "shared-agent-rules"
}
```

Author and description can be supplied independently when useful:

```json
{
  "name": "shared-agent-rules",
  "author": "Example team",
  "description": "Coding-agent instructions shared across our projects."
}
```

## Validation

Checked on 2026-10-16 with Python 3.12.12 and `jsonschema` 4.26.0 using its
Draft 2020-12 validator. The schema itself is valid, and all 41 instance checks
passed, including both JSON examples above. The checks cover name-only metadata,
each optional field independently, all metadata together, missing name, blank
and non-string values, non-object layer manifests, and the draft editor-support and
additional-property behaviour.

To repeat the checks, run the following from the repository root. It requires
`uv`, a Python tool runner, which prepares an isolated environment with the pinned
validator version and can download it if it is not cached. This is a developer
check, not a Hydragen runtime dependency; it adds no project dependency files.

```sh
uv run --no-project --with 'jsonschema==4.26.0' python - <<'PY'
import json
import re
from importlib.metadata import version
from pathlib import Path
from jsonschema import Draft202012Validator

change = Path('openspec/changes/archive/2026-10-16-define-layer-manifest')
schema = json.loads((change / 'layer-manifest.schema.json').read_text())
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema)
cases = [
    ('name only', {'name': 'example'}, True),
    ('with author', {'name': 'example', 'author': 'Example team'}, True),
    ('with description', {'name': 'example', 'description': 'Example layer'}, True),
    ('all metadata', {'name': 'example', 'author': 'Team', 'description': 'Layer'}, True),
    ('missing name', {}, False),
    ('optional fields without name', {'author': 'Team', 'description': 'Layer'}, False),
    ('supplied whitespace preserved', {'name': ' example ', 'author': ' Team '}, True),
    ('editor schema reference', {'name': 'example', '$schema': './schema.json'}, True),
    ('blank editor schema reference', {'name': 'example', '$schema': ''}, False),
    ('unvalidated extra property', {'name': 'example', 'future-field': 42}, True),
]
for field in ('name', 'author', 'description'):
    for value in ('', ' \t\n', '\u00a0\u2003', None, 42, False, [], {}):
        cases.append((f'invalid {field}: {value!r}', {'name': 'example', field: value}, False))
for value in (None, False, 42, 'example', []):
    cases.append((f'non-object layer manifest: {value!r}', value, False))
examples = re.findall(r'^```json\n(.*?)\n```', (change / 'design.md').read_text(), re.M | re.S)
assert examples, 'No JSON design examples found'
for number, example in enumerate(examples, 1):
    cases.append((f'design example {number}', json.loads(example), True))
for label, instance, expected in cases:
    errors = list(validator.iter_errors(instance))
    assert (not errors) == expected, f'{label}: unexpected result: {errors}'
print(f'Validator: jsonschema {version("jsonschema")} (Draft 2020-12)')
print(f'Schema valid; {len(cases)}/{len(cases)} instance checks passed (including {len(examples)} design examples).')
PY
```

Expected output for the current examples:

```text
Validator: jsonschema 4.26.0 (Draft 2020-12)
Schema valid; 41/41 instance checks passed (including 2 design examples).
```

A passing result checks metadata only. The draft schema deliberately accepts
unknown properties without interpreting them, so it cannot establish whether a
dependency declaration is valid. It also cannot check the layer manifest's filename or
repository-root location. Those limits do not resolve the open dependency design
or replace the location checks needed in future tooling.

## Risks / Trade-offs

- Allowing unknown fields also allows misspellings of optional fields. Required
  fields must still be present; revisit strictness as the layer manifest develops.
- This metadata draft does not establish how consumers select reproducible shared
  content. Source references and versioning remain separate design work.

## Open Questions

What naming syntax should be accepted? How should versioning work, and does it
belong in the layer manifest? How should dependencies be declared?
These remain open; this change adds no application implementation.
