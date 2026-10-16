## 1. Evidence collection and rendering

- [ ] 1.1 Add the fourteen tracked repositories and their document locations to one registry; collect source-linked commit, release, archival, and observation-date facts through the GitHub API.
- [ ] 1.2 Add repository-specific bounded snapshot rendering beside each tool in `FOUNDATIONS.md`; keep surrounding prose independent of changing dates and avoid a separate maintenance section.
- [ ] 1.3 Verify rendering and failure behaviour with fixtures covering grouped entries, repeated tools, preserved policy notes, newer activity, no release, archival changes, identical reruns, API failures, and missing document markers.

## 2. Scheduled publication

- [ ] 2.1 Add weekly and manual GitHub Actions triggers with the permissions needed to query repositories and publish a documentation PR.
- [ ] 2.2 Create or update one stable automation branch and PR; preserve existing documentation on failed checks and avoid duplicate PRs.

## 3. Integration and documentation

- [ ] 3.1 Validate the workflow, exercise a manual refresh, and inspect the resulting diff and PR behaviour.
- [ ] 3.2 Update `AGENTS.md` with the implemented cadence, tracked-repository location, manual refresh process, and boundaries on automated edits.
