## Why

The foundations compare alternatives using maintenance evidence that changes over
time. Periodic automated checks should keep that evidence useful without requiring
the project owner to repeatedly research and edit it manually.

## What Changes

- Add a weekly, manually runnable GitHub Actions workflow to check all fourteen
  repositories tracked in the foundations' comparisons: the five original tools
  and nine additions from the external review.
- Refresh dated observations of default-branch commits, GitHub releases, and
  archival state, with primary-source links, alongside each tool's comparison.
- Propose documentation updates through a reusable pull request; leave qualitative
  maintenance judgements and Hydragen's philosophy under human authorship.
- Separate generated observations from authored comparison text so refreshed facts
  do not leave stale, date-dependent conclusions behind.
- Update project guidance to describe the automated refresh process.

## Capabilities

### New Capabilities

- `maintenance-snapshot-refresh`: Scheduled collection and reviewable publication
  of maintenance evidence for compared projects.

### Modified Capabilities

None.

## Impact

- A new GitHub Actions workflow, a small updater, and an explicit repository list.
- Per-tool maintenance entries in `FOUNDATIONS.md` and their source links.
- Maintenance instructions in `AGENTS.md`.
- GitHub API access and permissions to update an automation branch and pull request.
- Repository-maintenance automation only; no change to Hydragen's product behaviour.
