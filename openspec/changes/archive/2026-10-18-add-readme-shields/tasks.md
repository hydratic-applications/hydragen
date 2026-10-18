## 1. Add the README badge row

- [x] 1.1 Add the three static Shields.io badges and three dynamic badges (deps.rs dependencies, GitHub stars, and owner-account sponsors) below the README title using the shortcut reference syntax, order, `flat-square` style, and alternative text in the design; group all twelve image and destination definitions at the bottom of the README, and preserve the existing introduction and limitations.

## 2. Verify presentation and scope

- [x] 2.1 Check all six image URLs and their Markdown destinations: verify bottom-of-file references, static badges, local anchors, and links to the deps.rs report, repository, stargazers, and owner's Sponsors page. Record live results; accept deps.rs `unknown`, stars `repo not found`, and an unavailable sponsorship destination while private access or account setup prevents operation. Confirm dynamic stars and account-wide sponsors use the service URLs, not hardcoded zeros; deps.rs uses no `path` parameter. Do not add credentials, change visibility, or enable sponsorship. Treat these as documentation checks, not build or test dependencies.
- [x] 2.2 Inspect the rendered introduction in light and dark themes at desktop and phone-sized widths; verify square corners and no gradients, readability, compact presentation, natural wrapping, and useful alternative text when images are blocked.
- [x] 2.3 Read the updated README from a first-time reader's perspective and inspect the diff: retain all essential prose, add no unsupported claims or unrelated changes, and keep CLI behavior, dependencies, and build/test scripts unchanged. Run `git diff --check`.
