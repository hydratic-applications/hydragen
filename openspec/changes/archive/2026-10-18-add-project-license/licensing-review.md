# Licensing prerequisite review

Reviewed on 2026-10-18 for the source-repository licensing increment, not as a
legal opinion or certification of future binary distributions.

## Ownership and imported material

- Leyton Blackler explicitly confirmed authority to license the Hydragen-owned
  work under GPL-3.0-or-later during implementation.
- Git history begins in 2026 and lists Leyton Blackler as author throughout;
  this supports the notice year but is not itself proof of ownership.
- Reviewed the tracked file list, source/tests for attribution and copying
  notices, the brand documentation, and imported OpenSpec skill metadata.
  No vendored Cargo dependency sources or external image/font assets are tracked.
- OpenSpec skills identify MIT licensing and generator version 1.5.0. Commands
  are also OpenSpec-derived. The installed `@fission-ai/openspec` 1.5.0 package's
  MIT notice matches the upstream [license](https://github.com/Fission-AI/OpenSpec/blob/main/LICENSE):
  `Copyright (c) 2024 OpenSpec Contributors`. Preserve it in root
  `THIRD_PARTY_NOTICES.md` without changing the imported skills' license metadata.
- The FSF copyright in the official GPL text belongs to the license document and
  remains unchanged; Hydragen's own notice is separate.

## Locked dependency review

`cargo metadata --locked --offline --format-version 1` resolved 185 third-party
packages, including transitive, development, and platform-specific packages.
Every package declares a license expression; none declares a license-file-only
custom license. This review considered the full resolved graph, not only the
dependencies active on this machine.

The expressions offer MIT, Apache-2.0, BSD-2-Clause, MIT-0, or Unlicense choices,
or require Unicode-3.0 or Zlib terms. Two expressions also require BSD-3-Clause
or Unicode-3.0 alongside an MIT/Apache choice. Some packages use older slash
notation for MIT/Apache alternatives. LLVM-exception and LGPL-2.1-or-later occur
only as alternatives where Apache-2.0 or MIT is also available.

No declared license blocks the GPLv3-or-later project grant when compatible
alternatives are selected and applicable notices and conditions are preserved.
This is a metadata compatibility review, not an audit of every dependency file.

Reviewed the additional notices in `encoding_rs` 0.8.42 (`COPYRIGHT` and
`LICENSE-WHATWG`), `unicode-ident` 1.0.27 (`LICENSE-UNICODE`),
`icu_collections` 2.3.0 (`LICENSE`), and `foldhash` 0.2.0 (`LICENSE`). These retain
attribution/permission notices, disclaimers, and where applicable non-endorsement
or altered-source conditions; GPL licensing does not erase those obligations.
Apache-2.0 also requires license/notice preservation and marking changed files.

Some downloaded packages declare their license without including a root license
file. They are not copied into this repository. Binary distribution is outside
this change: its notice review must obtain the appropriate upstream texts and
cover actual release targets/features, rather than treating this snapshot as a
complete distributable notice bundle. No unresolved project-grant conflict was
identified in this review.
