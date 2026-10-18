# Hydragen: the name and icon

## The name

The name **Hydragen** draws on **Hydratic**, the organisation namespace under which
the project is developed.

The **-gen** ending reflects the system's generative nature: combining reusable
[layers](../GLOSSARY.md#layer)—projects or collections of code, configuration, and
other files—with a layer's own additions, modifications, and deletions. This
produces the [layer result](../GLOSSARY.md#layer-result): the files and directories
a developer works with. It offers a starting point tailored to a project's
requirements rather than a static template, while keeping inherited content
connected to its sources as the project evolves.

The resemblance to **hydrogen**, the lightest and simplest chemical element,
expresses two [guiding values](../FOUNDATIONS.md#guiding-values): **performance**
through fast, resource-efficient operations, and **DX — Developer Experience**
through simplicity and ease of use.

## The molecule: connected layers

The icon depicts a stylised hydrogen molecule, **H₂**, with two circular atoms
joined by a diagonal bond. The atoms represent layers; the bond represents a
dependency, allowing one layer to build on another.

This expresses **granular reuse** and **reuse beyond setup**: combining the pieces
a project needs while preserving their connection to shared sources. The molecule
illustrates one relationship, not a two-layer limit or a requirement that layers
depend on each other in both directions.

## The inverted bond: seeing inside the result

The bond switches to the background colour inside each atom, creating a
cut-through appearance. It expresses **visibility and traceability**: seeing
inside the composed result to understand its dependency sources and versions,
and what the current layer adds, modifies, or deletes.

It also reflects **DRY — Don't Repeat Yourself**: shared content retains an
authoritative source rather than becoming independently maintained copies.

## Name and visual use

Use **Hydragen** for the product name in headings, branding, and prose, and
`hydragen` for the executable in commands such as `hydragen init`.

The selected terminal motif uses dense 16-column by 8-row braille. Its
layout, styling, and rendering limitations are captured in the
[CLI visual-identity design](../openspec/changes/define-cli-visual-identity/design.md).
