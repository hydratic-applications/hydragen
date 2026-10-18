# Project licensing verification

Verified on 2026-10-18. This change does not publish the repository or a package,
and does not certify compliance of future binary releases.

## Licensing consistency

- `Cargo.toml`, the README notice, and the license shield all identify
  `GPL-3.0-or-later`. The project notice names Leyton Blackler with the verified
  year 2026 and explicitly selects GPL version 3 or any later version.
- Root `LICENSE` contains the complete GPLv3 text, including its unchanged FSF
  copyright and generic application examples. SHA-256:
  `3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986`.
- The official GNU text was fetched successfully during proposal preparation.
  Implementation-time refetches of GNU's endpoint timed out or reset. An
  independent comparison with GNU GCC's
  [COPYING3](https://github.com/gcc-mirror/gcc/blob/master/COPYING3) matched the
  entire file except four URL updates present in the current text fetched during
  planning: FSF HTTP to HTTPS, two GNU licenses HTTP links to HTTPS, and the final
  `philosophy/why-not-lgpl.html` URL to `licenses/why-not-lgpl.html` over HTTPS.
  No claim is made of a fresh byte-for-byte download comparison with GNU's site.
- The full OpenSpec MIT notice in `THIRD_PARTY_NOTICES.md` matches the installed
  generator package's notice. See [the prerequisite review](licensing-review.md)
  for ownership confirmation and dependency-review scope.

## Package and regression checks

- `cargo metadata --no-deps --format-version 1`: Hydragen's license field is
  exactly `GPL-3.0-or-later`.
- `cargo package --list --allow-dirty`: includes `LICENSE`, `README.md`, and
  `THIRD_PARTY_NOTICES.md`. Reports the existing warning about absent
  documentation/homepage/repository metadata; no license-metadata warning.
- `cargo fmt --check`: passes.
- `cargo clippy --all-targets -- -D warnings`: passes.
- `cargo test --locked`: all 186 unit/integration tests pass, including the
  bounded real-terminal interaction tests; doc-tests contain no tests.
- `openspec validate add-project-license --strict`: passes.
- `git diff --check`: passes.
- Reviewed the diff: no source, tests, layer manifest schema, lockfile,
  dependency declarations, CLI behavior, publication settings, or visibility
  changes. Licensing and documentation assets remain outside runtime logic.

## README presentation and documentation review

Reviewed a local Ruby RDoc Markdown preview in the browser at desktop
(1440 × 900) and mobile (390 × 844) sizes. The preview restores image alt text
from the README references because RDoc otherwise omits it; it is not a claim
that GitHub's renderer itself was tested.

- All seven badges retain the specified order and flat-square styling. The new
  badge renders `license: GPL-3.0-or-later`, fits naturally with the existing row,
  and wraps without horizontal scrolling at the reviewed widths.
- Following the new badge opened the local `LICENSE` file, confirmed by browser
  navigation and the GPL text. No public repository access or credentials needed.
- With images deliberately unavailable, the browser displays the seven alt
  labels, including `License Badge`; the prose still identifies the precise
  license and links to its local text.
- Existing live service responses (`unknown` dependencies and `repo not found`
  stars) remain as returned by their services, not replaced or concealed.
- A first-time-reader review covered the README, foundations addition,
  third-party notices, and shared decision. They distinguish the project grant
  from third-party terms and independent layers, do not imply an automatic
  future-version upgrade, and do not claim completed binary-release compliance.
- Foundations explicitly gain one concise licensing/scope paragraph under
  “What Hydragen is”; the eight guiding values and layer model are unchanged.
