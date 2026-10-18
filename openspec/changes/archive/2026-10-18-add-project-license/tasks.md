## 1. Confirm licensing prerequisites

- [x] 1.1 Confirm with Leyton Blackler that they have authority to license the Hydragen-owned work; check the copyright year against project history and raise any employer/client or outside-contributor ownership questions before adding the grant.
- [x] 1.2 Review tracked imported material (including agent skills and assets) and the locked dependency graph for license compatibility and required notices. Record findings, preserve existing terms, and resolve any blockers or necessary attributions/exclusions before proceeding; do not assume third-party material is owned by Leyton.

## 2. Establish the project license and shield

- [x] 2.1 Add root `LICENSE` using the complete, unmodified official GNU GPLv3 text, preserving the Free Software Foundation's license-document copyright and generic application guidance.
- [x] 2.2 Set `[package].license = "GPL-3.0-or-later"` in `Cargo.toml`, without changing dependencies, the lockfile, or publication settings.
- [x] 2.3 Add a concise README License section containing Hydragen's copyright notice, the standard GPLv3-or-later application paragraphs and warranty disclaimer, a local `LICENSE` link, and the independent-project/third-party scope explanation from the design.
- [x] 2.4 Append `[![License Badge]][License]` after Sponsors, with the specified static Shields.io image URL and relative `LICENSE` destination defined at the README's bottom; retain the existing badges, reference style, and wrapping layout.
- [x] 2.5 Add a short product-level licensing/scope statement to `FOUNDATIONS.md`, consistent with the design, and verify the shared licensing decision remains accurate without changing unrelated open questions or glossary terms.

## 3. Verify consistency and completion

- [x] 3.1 Compare `LICENSE` with the official text and verify that the README application notice, badge value, and Cargo metadata all identify `GPL-3.0-or-later`. Check local links and use `cargo metadata --no-deps --format-version 1` and `cargo package --list --allow-dirty` to confirm the identity and inclusion of `LICENSE` and `README.md`, without publishing.
- [x] 3.2 Review the rendered README at desktop and phone widths: confirm seven badges in the specified order, flat-square styling, readable exact license wording, wrapping without horizontal scrolling, correct license navigation, and useful prose/alternative text when images are unavailable. Keep network checks manual, not build/test requirements.
- [x] 3.3 Read all revised documentation as a first-time reader; verify it explains the license's scope without implying automatic GPL licensing of independent layers, ownership of third-party content, completed release compliance, or automatic future-version upgrades.
- [x] 3.4 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, and `openspec validate add-project-license --strict`; review the diff to confirm no CLI, layer manifest, repository-visibility, or publication changes.
