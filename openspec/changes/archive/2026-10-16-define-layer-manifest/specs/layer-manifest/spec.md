## ADDED Requirements

### Requirement: Every layer has identifying metadata

Every layer MUST have a layer manifest declaring its name. Author and description MUST
be optional independently of each other. Name and any supplied author or
description MUST be strings containing at least one non-whitespace character.
These rules apply to both root and dependent layers, whether intended primarily
for reuse, as working projects, or both.

#### Scenario: Root layer metadata

- **WHEN** a root layer is defined
- **THEN** its layer manifest declares its name
- **AND** author and description can each be omitted

#### Scenario: Dependent layer metadata

- **WHEN** a dependent layer is defined
- **THEN** its layer manifest declares its name
- **AND** author and description can each be omitted

### Requirement: Layer manifest has a fixed filename and location

Every layer manifest MUST be a JSON file named exactly `layer.json` at the root
of the layer's repository. This requirement applies to both root and dependent
layers.

#### Scenario: Layer manifest at the repository root

- **WHEN** a layer declares its layer manifest
- **THEN** the layer manifest is stored as `layer.json` in the repository's top-level directory

#### Scenario: Different filename or nested location

- **WHEN** layer manifest content exists only under a different filename or in a subdirectory
- **THEN** it does not satisfy the layer manifest requirement

### Requirement: Layer manifest declares layer dependencies

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

### Requirement: A schema describes metadata requirements

The layer manifest format MUST have a JSON Schema requiring name and permitting optional
author and description, with non-blank string validation for each supplied field.
The initial draft covers metadata only; it does not establish dependency
reference syntax, how versioning works or whether it belongs in the layer manifest,
or how layers combine.

#### Scenario: Missing name

- **WHEN** a layer manifest omits name
- **THEN** it fails validation against the schema

#### Scenario: Optional metadata is omitted

- **WHEN** a layer manifest contains a valid name and omits author and description
- **THEN** it passes metadata validation against the schema

#### Scenario: Optional metadata is supplied independently

- **WHEN** a layer manifest contains a valid name and supplies either or both of author and description as non-blank strings
- **THEN** it passes metadata validation against the schema

#### Scenario: Supplied metadata is invalid

- **WHEN** name or a supplied author or description is blank, whitespace-only, or not a string, including `null`
- **THEN** it fails validation against the schema
