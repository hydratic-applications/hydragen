# Going-public checklist

Use this checklist when preparing to make `hydratic-applications/hydragen` public
on GitHub. The repository stays private until the maintainer deliberately changes
its visibility. This is not a requirement to publish a release or a Cargo package.

Keep adding publication-related follow-ups here as they arise. This is a starting
list, not evidence that a security review or service setup has already happened.
Some items depend on integrations that are still being planned; mark them not
applicable if they are not in use when publication happens.

## Before changing visibility

- [ ] Review the files and Git history that will become public for credentials,
  private configuration, personal information, and material not intended for
  publication. Removing a file from the latest commit does not remove its history.
  Revoke or rotate exposed credentials; decide separately whether history needs
  cleaning before publication.
- [ ] Confirm the [license](../LICENSE), README license notice, and
  [third-party notices](../THIRD_PARTY_NOTICES.md) still cover the content being
  published and that there is permission to publish it.
- [ ] Review GitHub Actions workflows for public contributions: fork PRs must not
  receive secrets or run contributed code in a privileged context. **Testing** runs
  **Run tests on stable Rust** and **Run tests on minimum supported Rust** independently.
  **Coverage** runs **Generate coverage report**, **Check total coverage**, and
  **Check changed-line coverage** on fork PRs but skips every PR's
  **Upload main coverage to Codecov** job. Only that isolated main uploader may
  request OIDC; it must not execute repository or artifact code.
  Publication does not automatically change that policy.
- [ ] Verify the candidate's separate **Run tests on stable Rust**,
  **Run tests on minimum supported Rust**, **Check total coverage**, and
  **Check changed-line coverage** results and retained report.
  Both applicable coverage gates require inclusive, unrounded 80%. Changed-line
  N/A requires valid evidence of no applicable measured PR lines, not a missing
  report/ref or skipped producer.
  Do not use Codecov project/patch statuses as substitutes for these local gates.

## After changing visibility

- [ ] Check the repository and README while signed out, including images and
  linked documentation, to confirm what external readers can actually access.
- [ ] If Codecov is configured, confirm its repository/report access is public,
  not just the GitHub repository. Confirm a `main` workflow run has uploaded a
  report and Codecov has processed it.
- [ ] Verify the main-branch coverage shield works **without a badge token**, after
  normal service cache refresh. Keep its existing branch-specific Shields.io URL
  and Codecov report link; do not substitute a fixed percentage.
- [ ] If a private Codecov badge-access token was embedded in the README, remove
  its query parameter and revoke that token in Codecov if it is no longer needed
  by another consumer. Check for other references first. Removing it from the
  current README does not remove it from Git history, so removal alone is not
  revocation. This is the badge-access token, not an upload token or GitHub token.
  The current coverage setup deliberately uses no badge token, so this cleanup
  applies only if one is added later.
- [ ] Verify the next eligible PR receives independent **Run tests on stable Rust**
  and **Run tests on minimum supported Rust** results in **Testing**, plus both
  Actions-owned coverage gates in **Coverage**, including forks without upload
  credentials. Confirm Codecov project/patch statuses remain disabled rather than
  becoming duplicate percentage policies. Review any old check-name or workflow-path
  references using the [migration guidance](coverage.md#workflow-and-check-name-migration).
- [ ] Verify a real main push or manual-main run uploads the matching current-run
  report using the separate OIDC job, including if valid coverage is below target.
  PR runs must not upload. Public access does not require unauthenticated uploads
  or privileged fork execution; do not widen permissions to make a result appear.
- [ ] Recheck the README's deps.rs, stars, and sponsors badges and their links.
  Public visibility may resolve repository-access errors after cache refresh;
  sponsorship availability is a separate account setting. Do not replace an
  unavailable service result with a fabricated healthy status or zero count.

## Keep separate decisions separate

Making the repository public does not authorise package publishing, releases,
billing changes, sponsorship setup, or new required checks for merging. Review
those independently if they are wanted.
