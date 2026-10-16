## ADDED Requirements

### Requirement: Every layer has identifying metadata

Every layer MUST have a manifest declaring its name, author, and description.
This applies to both root and dependent layers, whether intended
primarily for reuse, as working projects, or both.

#### Scenario: Root layer metadata

- **WHEN** a root layer is defined
- **THEN** its manifest declares its name, author, and description

#### Scenario: Dependent layer metadata

- **WHEN** a dependent layer is defined
- **THEN** its manifest declares its name, author, and description

### Requirement: Manifest has a fixed filename and location

Every layer's manifest MUST be a JSON file named exactly `layer.json` at the root
of the layer's repository. This requirement applies to both root and dependent
layers.

#### Scenario: Manifest at the repository root

- **WHEN** a layer declares its manifest
- **THEN** the manifest is stored as `layer.json` in the repository's top-level directory

#### Scenario: Different filename or nested location

- **WHEN** manifest content exists only under a different filename or in a subdirectory
- **THEN** it does not satisfy the layer's manifest requirement

### Requirement: Manifest declares layer dependencies

The `layer.json` file MUST list the other layers that the layer depends on.
Omitting the dependency list and providing an empty list MUST both mean that the
layer has no dependencies and is therefore a root layer. The dependency field
name and reference format remain open.

#### Scenario: Layer depends on other layers

- **WHEN** a layer depends on other layers
- **THEN** those dependencies are listed in its `layer.json` file

#### Scenario: Omitted dependency list

- **WHEN** a layer's `layer.json` omits the dependency list
- **THEN** the layer has no dependencies and is a root layer

#### Scenario: Empty dependency list

- **WHEN** a layer's `layer.json` includes an empty dependency list
- **THEN** the layer has no dependencies and is a root layer, with the same outcome as omitting the list

### Requirement: A schema describes required metadata

The manifest format MUST have a JSON Schema describing its required metadata.
The initial draft covers metadata only; it does not establish dependency
reference syntax, how versioning works or whether it belongs in the manifest,
or how layers combine.

#### Scenario: Missing required metadata

- **WHEN** a manifest omits any of the three required metadata fields
- **THEN** it fails validation against the schema
