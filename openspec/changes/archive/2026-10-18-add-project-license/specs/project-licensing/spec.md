## ADDED Requirements

### Requirement: Canonical project license and explicit application notice

The repository SHALL contain a root `LICENSE` file with the complete, unmodified
GNU GPL version 3 text from `https://www.gnu.org/licenses/gpl-3.0.txt`, including
its original license-document copyright and application guidance. The README
SHALL contain a `License` section identifying Hydragen as licensed under
`GPL-3.0-or-later` and linking to `LICENSE`.

The project application notice SHALL identify Leyton Blackler as copyright holder
with a verified project copyright year and SHALL state that recipients can
redistribute and/or modify Hydragen under GNU GPL version 3 or, at their option,
any later version published by the Free Software Foundation. It SHALL include
the standard GPL application notice's no-warranty wording and license-location
guidance, separately from the unmodified license text.

#### Scenario: Find the license in a source checkout

- **WHEN** a developer reads the README and root license file without network access
- **THEN** the README identifies the exact GPLv3-or-later policy, project copyright
  holder, and warranty disclaimer
- **AND** its local link leads to the complete GPLv3 text without project-specific
  changes to that text

#### Scenario: Distinguish the project grant from the license document

- **WHEN** a reader compares the project notice with the generic GPL text
- **THEN** Leyton Blackler is identified as the project's copyright holder, not
  substituted for the Free Software Foundation in the license document
- **AND** the project notice explicitly grants the later-version option rather
  than relying on the generic license's example placeholders

### Requirement: Consistent Cargo license metadata and package contents

The root Cargo package SHALL declare `license = "GPL-3.0-or-later"` in its
`[package]` section. The Cargo package file list SHALL include root `LICENSE`
and the README containing the application notice. Verification SHALL NOT require
publishing the package or changing repository visibility.

#### Scenario: Inspect package licensing locally

- **WHEN** a maintainer inspects Cargo metadata and the package file list
- **THEN** the Hydragen package's license is exactly `GPL-3.0-or-later`
- **AND** `LICENSE` and `README.md` are included without any publication step

### Requirement: Preserve rights and independent-content boundaries

Before applying the project-wide licensing declaration, the implementer MUST
confirm the owner's authority to license the Hydragen-owned work and review
tracked third-party material and the locked dependency graph for applicable
terms, required notices, and GPL compatibility. Existing third-party notices
SHALL be preserved; third-party material SHALL NOT be represented as solely
owned or newly relicensed by Leyton Blackler. Unresolved ownership or license
conflicts MUST be raised for resolution before the declaration is applied.

The README SHALL briefly explain that using Hydragen does not, by itself, require
independently authored projects or layers to use the GPL, and that third-party
content remains subject to its applicable terms. The declaration SHALL NOT claim
to grant rights over all content handled by the CLI.

#### Scenario: Encounter a rights or license conflict

- **WHEN** the initial review cannot establish licensing authority or identifies
  incompatible terms or missing permissions
- **THEN** the issue is reported and resolved before applying the project-wide grant
- **AND** third-party notices are not removed or silently replaced

#### Scenario: Use Hydragen with an independently authored project

- **WHEN** a developer reads the licensing scope before using Hydragen on their
  independently authored layer
- **THEN** the README does not imply that using the CLI automatically relicenses
  that layer under the GPL
- **AND** it does not promise permission to reuse third-party content independently
  of that content's applicable terms

### Requirement: Licensing changes remain separate from publication and runtime

Adding licensing assets SHALL NOT change repository visibility, publish packages
or binaries, alter CLI behavior, add layer manifest fields, or add runtime,
build, or test dependencies on remote license or badge services. The maintained
license text and notice SHALL live outside `openspec/`.

#### Scenario: Review the licensing increment

- **WHEN** the implementation is completed and checked
- **THEN** the license declaration is available locally while publication remains
  a separate owner-controlled action
- **AND** the CLI and layer manifest behavior are unchanged
- **AND** no source code, test, or build script loads licensing assets from `openspec/`
