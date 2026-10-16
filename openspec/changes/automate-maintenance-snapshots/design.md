## Context

`FOUNDATIONS.md` contains manually researched maintenance notes beside individual
tool comparisons. The owner now wants periodic automated refreshes rather than an
ongoing manual checking obligation. This is repository infrastructure, separate
from Hydragen's CLI.

## Goals / Non-Goals

**Goals:** Keep primary-source evidence current, make updates reviewable, and
preserve the distinction between activity signals and maintenance judgements.

**Non-Goals:** Automatically rank alternatives, infer abandonment from inactivity,
rewrite guiding values, or implement Hydragen product features.

## Decisions

1. **Use GitHub Actions weekly and on demand.** A weekly UTC schedule is the
   requested cadence, with `workflow_dispatch` for manual checks. This avoids a
   separate scheduler and keeps cadence adjustable in the workflow.
2. **Track explicit repositories.** Cover the fourteen repositories now compared:
   `cookiecutter/cookiecutter`, `copier-org/copier`, `cruft/cruft`, `jondot/hygen`,
   `projen/projen`, `eslint/eslint`, `microsoft/TypeScript`, `prettier/prettier`,
   `pnpm/pnpm`, `vercel/turborepo`, `nrwl/nx`, `git/git`, `angular/angular-cli`,
   and `sapegin/mrm`. Keep the list and each repository's document location in one
   small registry. Broad categories such as repository templates and libraries
   have no single upstream maintenance status; do not discover repositories by
   scraping arbitrary documentation links.
3. **Collect factual GitHub evidence.** Fetch the current default-branch commit,
   latest published GitHub release, archival state, observation date, and source
   URLs. A missing release is a valid result, not a request failure. GitHub release
   dates do not stand in for package-registry publication dates.
4. **Generate bounded evidence beside each tool.** A small updater renders dates,
   links, and archival facts inside repository-specific markers in each tool's
   comparison. Keep each observation in that one location, without a separate
   maintenance table or section. Keep surrounding comparison text independent of
   changing dates so automatic updates cannot leave contradictory current-status
   claims. Support several repositories in one comparison, but render each
   repository's evidence once: Nx's migration entry links to its workspace entry.
   Leave separately dated policy evidence, such as mrm's bug-fix-only contribution
   policy, outside the generated blocks. Avoid an AI dependency for factual rendering.
5. **Update one automation pull request.** Use a stable branch to create or update
   a documentation PR. A successful new check date is evidence worth recording even
   when upstream facts are unchanged; repeating a run with identical facts and
   observation date produces no diff. Prefer a reviewable PR to direct commits to
   the default branch, consistent with the project's review workflow.
6. **Publish only complete observations.** Collect all repositories successfully
   before changing documentation. Missing markers, authentication errors, rate
   limits, or unavailable repositories fail visibly without replacing the last
   complete snapshot. Keep permissions limited to API reads and the branch/PR
   writes needed for publication.

## Risks / Trade-offs

- GitHub API or token restrictions → preserve existing evidence and report failure
  in the workflow; verify repository settings permit automated PR creation.
- Dates can be mistaken for maintenance verdicts → label the evidence and its
  scope, and retain human-authored interpretation outside the generated blocks.
- Scheduled Actions can be delayed or disabled → retain the observation date and
  manual trigger so a stale snapshot is visible and can be refreshed on demand.

## Migration Plan

Introduce the registry, updater, and per-tool bounded snapshot blocks; add the workflow;
exercise a manual run and review its diff. Update `AGENTS.md` to describe the live
process once implemented. Disabling the workflow leaves the last dated snapshot
readable and the manual research method available.

## Open Questions

- Choose the weekly run day and time, and confirm PR-creation permissions when implementing.
