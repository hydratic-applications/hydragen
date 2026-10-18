## Context

Hydragen is a Rust CLI and library in one root Cargo package. The repository has
no project license file, Cargo license field, or README licensing section. Its
README has six linked badges; the maintained badge specification currently
prohibits a license claim because none has been established.

The owner selected `GPL-3.0-or-later` and supplied Leyton Blackler as the
copyright-holder name. Hydratic is the GitHub organisation/brand, not a confirmed
separate legal owner. Ownership of all existing material has not yet been
confirmed. This design records the selected terms, not a completed legal audit.

The change supports the foundations' visibility and traceability value by making
reuse terms explicit. It does not change how layers work or resolve the existing
open questions about dependency sources, rendering, or contribution capture.

## Goals / Non-Goals

**Goals:**

- Establish an explicit GPLv3-or-later grant for Hydragen-owned content before
  the owner makes the repository public.
- Keep the license text, project notice, Cargo identity, and README shield
  consistent and available without GitHub license detection.
- Preserve third-party rights and explain the distinction between licensing
  Hydragen and licensing independent content used with it.

**Non-Goals:**

- Changing repository visibility, publishing a crate, or distributing binaries.
- Adding release compliance automation, a contributor agreement, a new license
  exception, dual licensing, or a blanket claim over third-party content.
- Adding CLI license banners, commands, network requests, or layer manifest fields.
- Retrofitting headers into every source file or imported agent skill in this
  increment. Separate file notices required by existing terms must be preserved.

## Decisions

### Adopt GPL-3.0-or-later with an explicit project application notice

Use GNU GPL version 3 or, at the recipient's option, any later version published
by the Free Software Foundation. The owner chose this after considering MIT,
Apache-2.0, and GPLv3, then considering GPLv3-only versus GPLv3-or-later.

GPL permits commercial use and sales, but covered distributions must meet its
source-availability and licensing conditions. It does not require every private
modification to be published. The later-version option offers potential future
compatibility without forcing an upgrade; it also authorises recipients to
choose future terms the owner has not yet seen. Merely deleting that option from
a future checkout cannot withdraw it from compliant recipients of earlier grants.

Add the complete, unmodified text from
<https://www.gnu.org/licenses/gpl-3.0.txt> as root `LICENSE`. Keep the Free Software
Foundation's copyright in that document: it identifies ownership of the license
text, not ownership of Hydragen. Do not replace it with Leyton's name, edit the
license's example placeholders, or insert project-specific terms into the text.

Use the README's new `## License` section for the project-specific application
notice: identify Hydragen, include `Copyright (C) 2026 Leyton Blackler` (verify
the year against project history), and include the standard GPL application
paragraphs covering redistribution/modification under version 3 or later,
no warranty, and where to find the license. Link to local `LICENSE`. This notice
is necessary because the generic GPL text alone does not select the project's
version policy. Keep surrounding explanation short rather than adding a legal
tutorial to the README.

### Use Cargo's standard SPDX identity

Set `[package].license = "GPL-3.0-or-later"` in `Cargo.toml`. SPDX is the
standardised license identifier Cargo and package registries understand. Do not
use the ambiguous shorthand `GPL-3.0` or replace the identity with a custom
`license-file`-only declaration. Keep `LICENSE` and the application notice in
the package file list; verify with `cargo package --list --allow-dirty` without
publishing anything. No dependency or lockfile update is needed for this field.

Reference: [Cargo license metadata](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields).

### Use a static, exact license shield linked to the local file

Append the license badge after Sponsors so the existing six retain their order.
Use `[![License Badge]][License]`, with the two definitions at the README's
bottom, following the existing reference style:

- Image: `https://img.shields.io/badge/license-GPL--3.0--or--later-blue?style=flat-square`
- Destination: `LICENSE`

Shields escapes literal hyphens by doubling them in static badge paths. The
rendered value must be `GPL-3.0-or-later`, not merely `GPL v3`. The image does not
inspect GitHub or require public repository access. The local file is the
authority; the shield is a maintained summary, not automatic license detection.
README prose must state the exact license even if images fail to load.

A GitHub-backed license shield was considered but could report an unknown state
while the repository is private and might not distinguish the later-version
policy. A static image avoids both issues at the cost of requiring consistency
checks when terms change. Do not add badge generation or network checks to the
build or test process.

### Preserve ownership boundaries and check before applying

Before adding the grant, confirm with the owner that they can license the
Hydragen-owned work. Review tracked third-party/imported files, including agent
skills and assets, and the resolved dependencies' license metadata and required
notices. Use the locked dependency graph rather than checking only direct
dependencies. Preserve existing notices and document any needed exclusions or
attributions in a focused maintained file if the review finds them; do not
silently relabel third-party content as Leyton's GPL work. Escalate unresolved
ownership, incompatible terms, or missing permissions before proceeding.

The root declaration covers Hydragen-owned code and associated project material
except content with its own applicable terms. Independently authored projects
and layers do not become GPL merely because they use Hydragen. Content copied
from dependencies retains its applicable terms; this change neither supplies
permissions for those dependencies nor implements license-compliance checking
for layer reuse. GPLv3 [section 2](https://www.gnu.org/licenses/gpl-3.0.html#section2)
explains the distinction between running a program and covered output.

### Keep product guidance aligned without changing the layer model

Record the agreed project-license choice and scope in the shared question list.
During implementation, add a short product-level licensing statement to
`FOUNDATIONS.md`; do not add a ninth guiding value or turn it into a compliance
manual. No new Hydragen-specific term needs a glossary entry. Future layer
license policy and enforcement remain open rather than being inferred here.

## Risks / Trade-offs

- Unverified ownership or incompatible third-party terms → Treat the initial
  rights/notices review as a prerequisite; stop and ask rather than granting
  rights the owner may not hold.
- A static shield drifts from the actual grant → Review the exact SPDX string
  across Cargo, the README notice, and the badge together.
- License detection shows only generic GPLv3 → Keep the later-version grant
  explicit in the application notice and Cargo; service detection is not proof
  of the version policy or a completion gate.
- Adding a license is mistaken for release compliance → State that binary
  distribution needs its own corresponding-source and third-party-notice review;
  this increment does not certify future releases or publish anything.
- A later GPL version is undesirable to the owner → Record that choosing it is
  a recipient option already authorised by this policy; earlier grants are not
  undone by reverting repository files. See
  [GPLv3 section 14](https://www.gnu.org/licenses/gpl-3.0.html#section14).

## Migration Plan

1. Confirm licensing authority and resolve any third-party findings.
2. Add the canonical license, application notice, Cargo metadata, README badge,
   and concise foundations statement together.
3. Check canonical text, metadata, package contents, Markdown references, and
   badge appearance at desktop and narrow widths. Run existing regression checks.
4. Leave public visibility and publication to a separate explicit owner action.

Before any grant has been distributed, draft-file corrections are ordinary
repository edits. After distribution, reverting files is not a way to revoke
already-granted GPL rights from compliant recipients.

## Open Questions

- The owner has supplied the holder name but has not confirmed all licensing
  authority or the absence of employer/client rights and outside contributions.
  Confirm these facts before applying; they do not change the selected version policy.
- The review may identify third-party notice or permission requirements not yet
  inventoried. Record findings and resolve blockers rather than assume none exist.
