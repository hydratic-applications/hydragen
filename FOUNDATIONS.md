# Hydragen foundations

This document records the problem Hydragen exists to solve and the principles
that guide it. It is an evolving foundation, not a complete specification.

## What Hydragen is

Hydragen is a development system for building distinct projects from shared code,
configuration, and project files, without maintaining the same content separately
in every repository. It aims to make reuse fast and as natural as editing directly,
saving time and mental effort. Projects take only what they need and keep their
own choices, with clear sources, versions, and local differences as they adopt
improvements. AI support is native and optional.

Its intended first interface is a command-line tool (CLI), run from a terminal and
written in Rust.

## Who it is for

Hydragen is for developers who want to share and reuse code, configuration, and
other files across projects while keeping each project's own choices. It is
especially relevant when repeated setup and carrying improvements between
separate repositories have become ongoing work.

Code and files maintained for reuse are **shared sources**; projects that use
them are **consumers**.

Developers with any level of AI use, including none, are part of the intended
audience; see the [AI support value](#guiding-values).

Examples include JavaScript, TypeScript, and npm (the JavaScript package ecosystem),
but the reuse problem also covers configuration in other formats and Markdown
instructions for coding agents.

## The problem

Setting up each project from scratch repeats tedious boilerplate work. A template
provides a quicker start, but often bundles a particular set of technologies.
Projects must add missing pieces or remove unwanted code, configuration, and
dependencies. Cleanup can be messy and incomplete. Maintaining a custom template
does not eliminate this mismatch between projects.

Hydragen's motivation comes from the author's experience maintaining many separate
repositories. Even with one maintainer, projects could use different technologies
or configurations, repeat files, or contain different code serving similar purposes.

The cost continues after setup. Separately maintained code gradually differs
between repositories—often called **drift**. An improvement in one project must
be identified and carried to others that could benefit. Plain template copies
also miss later improvements to the template; update tools help propagate them,
but still leave consumer copies to reconcile.

Differences also cost attention. Projects can use the same technology or solve the
same problem in slightly different ways, leaving developers to remember or
rediscover each setup. Switching projects or returning after a break then requires
time and mental effort before useful work can resume.

When developers see only a final file, it can be hard to tell which parts came
from a shared source, how a consumer changes or extends them, or whether a new
improvement should be shared. A record of the template repository and version
helps, but does not show how inherited content and local changes combine while
editing.

Customisation can add its own burden. Files full of template conditions become
harder to read, and combining templates can require extra conventions or stages
of generation. Importing or configuring reusable content may feel harder than
editing it directly, discouraging reuse despite possible future savings.

Hydragen's values below address these connected costs across setup, everyday
editing, and ongoing maintenance.

## Guiding values

These eight values guide Hydragen's design. They must work together to make it a
useful choice over existing approaches.

1. **DRY — Don't Repeat Yourself.** Each shared piece of code or project content
   must have one **effective source of truth**: the maintained definition that
   governs it and where shared improvements belong. This is its
   **authoritative source**. Writing it once, copying it into consumers, and keeping
   those copies updated does not satisfy this value, even when their origin is recorded.
   Shared work should avoid repeatedly solving the same problem, setting up the
   same capability, or implementing the same changes across repositories.

2. **DX — Developer Experience.** Make reuse feel as natural as editing code.
   Opting into reuse should not be harder than working on the code directly.
   Extra imports, configuration, or steps must not deter adoption. Reusable code
   should be easy to write, adopt, and maintain, reducing setup, cleanup, and
   maintenance effort. It must remain readable as customisation grows, without
   unnecessary layers of generation. Ease of use matters immediately, not only
   after long-term savings accumulate.

3. **Granular reuse — Take what you need.** Let projects share individual needs
   without adopting an entire set of technologies. This includes directory
   structures, configuration in any format, and Markdown skills and rules for
   coding agents—not only JavaScript or TypeScript modules. Reuse must work for
   needs shared by only a minority of projects, without unwanted material to
   remove. Legitimate differences must not prevent sharing common parts. Combining
   and extending parts must be supported directly, without consumers inventing
   their own layering rules or templates that generate templates.

4. **Reuse beyond setup — Keep benefiting from improvements.** Reuse must remain
   useful as projects evolve. Keep the **reuse relationship**: the ongoing
   connection from a consumer to its shared source and selected version, with a
   clear distinction between shared content and consumer-owned changes. Preserve
   that connection rather than maintain copies through repeated copying or
   synchronisation. Consumers must know which version they use and choose when to
   adopt improvements. Adopting an incompatible change may still require work in
   the consuming project.

5. **Efficiency — Save time and mental effort.** Where projects share needs, make
   consistent technology choices, configuration, and code easy to reuse. Familiar
   approaches should reduce relearning when switching projects or returning after
   a break. Consider the wider set of projects: put common solutions in shared
   sources and keep project-specific choices local. This useful consistency should
   reduce implementation and maintenance effort while preserving legitimate
   differences.

6. **AI support — Native by design, optional in use.** Hydragen must work naturally
   without AI and offer purpose-built AI support when developers choose to use it.
   AI must not be required to write, adopt, understand, or maintain shared content.
   Developers may work manually, use AI selectively, or delegate substantial work
   to agents, mixing approaches and changing them over time. Each must be a
   first-class, efficient workflow. Native support means agents have direct ways
   to discover and use shared content, with the same source and change visibility
   available to developers.

7. **Performance — Keep everyday operations fast.** Hydragen must start quickly
   and keep common operations responsive for developers and agents. Speed and
   efficient resource use must guide design decisions from the outset. Avoid
   unnecessary processing, file access, memory use, and repeated work, so the tool
   remains practical as projects and shared content grow.

8. **Visibility and traceability — Know the source and see the differences.**
   Developers and agents must be able to locate the source of truth for shared
   content, identify its origin and selected version, and see how a consumer
   changes or extends what it inherits.
   When several layers contribute, show where each contribution comes from and
   the order in which changes take effect. Shared content and local differences
   must be clear during everyday work, not only reconstructable during updates.
   This should help developers spot drift, understand intentional exceptions, and
   recognise improvements that belong in the shared source.

## Existing approaches

Each entry states a useful case, its main tradeoff, and Hydragen's intended
improvement. These are goals to demonstrate, not proven advantages.
The comparisons move from shared code and settings, through repository-based
sharing, to templates, generators, and configuration automation.

Maintenance entries are dated observations of the default branch and GitHub
releases, not all activity or package publishing. Gaps alone do not prove
abandonment; mature tools can remain useful with few changes.

### Shared packages and configuration

- **Good for:** Versioned reuse through library APIs—the functions and types a
  library exposes—or shared settings with local overrides. Examples include your
  own npm library, [ESLint configs][eslint-config], [TypeScript `extends`][typescript-extends],
  and [Prettier config packages][prettier-config]. These avoid independently
  maintaining the same implementation or settings in each project.
- **Core tradeoff:** The consuming tool controls what can be imported and how
  settings combine. Directory layouts, other configuration formats, and Markdown
  skills and rules need extra integration where tools expect actual project files.
  npm [can package those files][npm-package-files], but does not provide that
  integration by itself. Custom behaviour needs extension points or package changes.
- **Maintenance — checked 2026-10-15:** Not archived at review.
  - ESLint: commit [2026-10-14][eslint-reviewed-commit]; release
    [v10.12.0, 2026-10-02][eslint-reviewed-release].
  - TypeScript: commit [2026-10-14][typescript-reviewed-commit]; release
    [v7.0.2, 2026-08-20][typescript-reviewed-release].
  - Prettier: commit [2026-10-15][prettier-reviewed-commit]; release
    [3.9.10, 2026-10-10][prettier-reviewed-release].
- **Hydragen's goal:** Extend single-source reuse to files and structures in the
  form their tools need. Keep ordinary packages and configuration inheritance
  where they already work well.

### Monorepos and workspaces: pnpm, Turborepo, and Nx

- **Good for:** Keeping several projects in one repository—a **monorepo**—and
  sharing local packages through a [pnpm workspace][pnpm-workspaces].
  [Turborepo][turbo-workspaces] and [Nx][nx-overview] coordinate work across projects.
- **Core tradeoff:** Sharing through co-location assumes projects can live together.
  Projects that remain in separate repositories still need a way to distribute
  shared content. This is a limit of that repository arrangement, not a claim that
  Nx or Turborepo cannot use published dependencies or serve other workflows.
- **Maintenance — checked 2026-10-15:** Not archived at review.
  - pnpm: commit [2026-10-15][pnpm-reviewed-commit]; release
    [v12.11.2, 2026-10-09][pnpm-reviewed-release].
  - Turborepo: commit [2026-10-15][turbo-reviewed-commit]; release
    [v2.11.8, 2026-10-13][turbo-reviewed-release].
  - Nx: commit [2026-10-15][nx-reviewed-commit]; release
    [23.3.0, 2026-10-07][nx-reviewed-release].
- **Hydragen's goal:** Make shared parts easy to use across separate repositories,
  without requiring projects to move into a monorepo.

### Git submodules and subtrees

- **Good for:** Bringing a shared directory into a project using Git.
  A [submodule][git-submodules] references a separate repository at a chosen commit;
  a [subtree][git-subtrees] brings files into the consumer's own history and can
  merge changes back and forth.
- **Core tradeoff:** Submodules add repository setup and update steps. Subtrees
  make files ordinary local files, but keep copies that need merging. Both work
  at directory level; sharing selected fragments of existing files needs more work.
- **Maintenance — checked 2026-10-15:** Git mirror commit
  [2026-10-06][git-reviewed-commit]; [no GitHub releases][git-releases].
  Not archived at review. Git's [release tags][git-tags] are a separate publication channel.
- **Hydragen's goal:** Preserve clear origins and chosen versions with less setup
  and a clearer boundary between shared content and project-specific changes.

### Repository templates

- **Good for:** [Starting a project quickly][github-templates] from a known setup.
- **Core tradeoff:** You inherit the whole starting point, remove what does not fit,
  and maintain your own copy. Later template improvements do not flow back in.
- **Hydragen's goal:** Take only the parts you need and keep shared code connected
  to one source of truth.

### Configurable starting points: Cookiecutter

- **Good for:** [Generating a tailored starting point][cookiecutter] using prompts,
  template logic, and setup hooks.
- **Core tradeoff:** Choices customise the initial output, but you then maintain
  that code yourself. Template updates need another tool, such as cruft.
- **Maintenance — checked 2026-10-15:** Latest default-branch commit
  [2026-03-04][cookiecutter-reviewed-commit]; GitHub release
  [v2.7.1, 2026-03-04][cookiecutter-reviewed-release]. Not archived at review.
- **Hydragen's goal:** Keep reusing the selected parts as they evolve, rather than
  finish reuse when generation ends.

### Updatable templates: Copier

- **Good for:** Customisable templates with [saved answers][copier-creating], Jinja,
  a text-template language, [version selection through Git tags or commits][copier-versions],
  and [Git-based updates][copier]. Plain generated files do not require Copier
  itself to be installed.
- **Core tradeoff:** You maintain a copy and merge template updates.
  [Applying separate templates][copier-multiple] is supported; [extending a base template][copier-inheritance-lifecycle]
  adds coordination of parent versions, file overrides, and update scripts
  (**migrations**).
- **Maintenance — checked 2026-10-15:** Latest default-branch commit
  [2026-10-13][copier-reviewed-commit]; GitHub release
  [v9.19.1, 2026-10-13][copier-reviewed-release]. Not archived at review.
- **Hydragen's goal:** Combine and extend shared parts without building that
  inheritance machinery yourself. Keep one source of truth and make origins and
  project-specific changes visible while editing.

### Updatable templates: cruft

- **Good for:** Adding [template tracking and updates][cruft] to Cookiecutter projects.
- **Core tradeoff:** Automates updates to copies; it does not replace them with one
  shared implementation.
- **Maintenance — checked 2026-10-15:** Latest default-branch commit
  [2024-12-25][cruft-reviewed-commit]; GitHub release
  [2.16.0, 2024-12-25][cruft-reviewed-release]. Not archived at review.
- **Hydragen's goal:** Share the implementation itself instead of keeping separate
  implementations in sync.

### Incremental code generation: Hygen

- **Good for:** [Generating small pieces of boilerplate][hygen] inside an existing
  project, including additions to existing files.
- **Core tradeoff:** Improving a generator does not update code it already produced.
  Earlier output needs separate maintenance.
- **Maintenance — checked 2026-10-15:** Latest default-branch commit
  [2023-03-15][hygen-reviewed-commit]; GitHub release
  [v6.2.11, 2022-09-07][hygen-reviewed-release]. Not archived at review.
- **Hydragen's goal:** Make shared improvements available to existing consumers,
  not just the next generation run.

### Updating existing code: Nx migrate and Angular ng update

- **Good for:** [Nx `migrate`][nx-migrate] and [Angular `ng update`][angular-update]
  can change existing configuration and source code when upgrading dependencies.
  Generation does not always leave developers updating old output by hand.
- **Core tradeoff:** Someone must supply update instructions for the relevant
  versions. They change consumer-owned files rather than turn them into one shared
  implementation; project-specific edits still need review.
- **Maintenance — checked 2026-10-15:** Angular CLI commit
  [2026-10-15][angular-reviewed-commit]; release
  [v22.2.2, 2026-10-07][angular-reviewed-release]. Not archived at review.
  Nx's snapshot is in its
  [workspace comparison](#monorepos-and-workspaces-pnpm-turborepo-and-nx).
- **Hydragen's goal:** Preserve a clear shared source as it evolves. These tools
  offer examples of upgrades that modify consumer files.

### Configuration as code: projen

- **Good for:** [Managing project configuration as code][projen], with reusable
  components and a clear source of truth.
- **Core tradeoff:** To change managed files, edit projen's definitions and
  regenerate them. Direct edits to the generated files are not the source of truth.
- **Maintenance — checked 2026-10-15:** Latest default-branch commit
  [2026-10-15][projen-reviewed-commit]; GitHub release
  [v0.104.5, 2026-10-12][projen-reviewed-release]. Not archived at review.
- **Hydragen's goal:** Retain a clear source of truth while making shared
  configuration as natural to work with as ordinary project code.

### Configuration update tasks: mrm

- **Good for:** [Reusable tasks][mrm] that make small updates to existing project
  configuration. Its helpers support JSON, YAML, INI, Markdown, and line-based
  files; tasks can be shared through npm and grouped into presets.
- **Core tradeoff:** Authors write and maintain the [editing tasks][mrm-tasks].
  Tasks preserve existing content where instructed, but that does not by itself
  identify which resulting settings belong to a shared source.
- **Maintenance — checked 2026-10-15:** Latest default-branch commit
  [2024-10-30][mrm-reviewed-commit]; GitHub release
  [mrm@4.1.22, 2023-09-14][mrm-reviewed-release]. Not archived at review.
  Policy observed 2026-10-15: the [README][mrm-policy] welcomes bug fixes but not new features.
- **Hydragen's goal:** Make continued sharing and ownership visible, rather than
  require a collection of editing tasks to be the entire reuse model.

## Scenarios

### Starting projects with overlapping needs

Consider setting up five projects with a similar foundation. Two require a particular
technology or configuration; the other three do not.

- **Include it in the base template:** the three projects that do not need it
  inherit unnecessary material and must remove it.
- **Exclude it from the base template:** the two projects that need it must each
  add it themselves, duplicating setup work and creating an immediate opportunity
  for their implementations to drift.

Optimising the template for the majority merely determines which projects bear
the additional work. It does not eliminate that work or enable the minority to
share their common setup.

Hydragen must enable those two projects to reuse their common setup without
imposing it on the other three or requiring each consumer to recreate it
independently. For each new project, the intended experience is to select the
common parts and any relevant additions, know their sources and versions, and
start working without repeated setup or cleanup.

The project author's attempt to achieve this kind of layering in Copier involved
templates extending other templates, custom overlays—rules for putting one
template's changes over another—and a template acting as a consumer in another
generation stage. This felt recursive and hard to reason about.

Copier's [documented multi-template workflow][copier-multiple] applies independent
templates with separate answers files and separate updates. Including YAML
settings or reusing Jinja content does not by itself manage a parent template and
a child that extends it. The [upstream hierarchy discussion][copier-hierarchies]
explores templates that generate other templates, and forks maintained through
Git. In [May 2025][copier-inheritance-lifecycle], a maintainer identified
complications around different parent/child versions and which update scripts
must run. The difficulty includes managing updates, not simply template syntax.

Hydragen must make combining and extending shared parts a straightforward supported
workflow rather than require consumers to design that additional machinery themselves.

### Sharing project files beyond library APIs

A developer wants one source of truth for coding-agent skills and rules used
across many repositories. The agents need actual Markdown (`.md`) files in the
locations they read; importing a JavaScript or TypeScript library does not give
them those instructions.

Each consumer references the shared skills or rules and the version it uses.
Hydragen should make the actual files available to the agent, with their content
determined by that reference. The files available in a consumer must remain tied
to the shared source, not become separately maintained versions of the rules.

When the developer improves a rule, they change it once in the shared source.
That improvement should be available to every consuming project. A project
referencing the updated version should receive the updated Markdown content
without someone manually editing or copying the change into each repository.
The agent still reads ordinary files; the reuse comes from the shared-source
reference, not a code import.

The same need applies to directory structures and configuration outside JavaScript
or TypeScript. Even a self-published npm package needs tool support or extra wiring
to expose these files where their tools expect them. Hydragen must support reuse
in those native forms while keeping the source and version clear.

### Choosing reuse during everyday development

A developer can edit code directly or opt into reuse. If using the shared code
requires cumbersome imports, configuration, or extra steps, the direct edit can
feel easier, even when reuse would benefit other projects later.
Slow startup and repeated waits can have the same effect, even when the steps
themselves are simple.

One developer changes a shared configuration without AI tools. Another uses their
own AI assistant for help and later delegates an update to an agent. Both need to
find the same shared source and version, understand local differences, and know
where a change belongs.

The authoring side matters too. In the project author's experience, Copier's saved
answers and Jinja enabled useful fine-grained choices. Version pinning and Git
merging also worked well. Files with complex customisation became difficult to
read, however. Supporting more choices must preserve the ability to understand
the code and its variations.

Hydragen must make opting into reuse natural and easy enough that the mechanism
itself does not deter adoption. Developers should be able to gain the benefits of
reuse without making their immediate work harder than editing code directly.

### Improving code used by multiple projects

A base implementation is copied into several projects. An update tool can record
its origin and propagate later changes, but each consumer still has a separate
copy. If a developer changes one of those copies, they need to know how it differs
from the shared implementation and whether the change belongs upstream or only to
that consumer.

In the project author's Copier workflow, only the final generated file was apparent
while editing. Copier's answers file recorded the source and version, and Copier
could reconstruct differences during updates. During everyday editing, however,
it was hard to tell unchanged base content apart from additions or modifications
made in the consumer. This hid both drift and opportunities to lift
an improvement into the shared template. Git history of the consumer's own edits
did not by itself show its differences from the template.

Hydragen must preserve one effective source of truth for the shared implementation
and an explicit boundary for consumer-owned code. Developers must be able to tell
where inherited code is defined, which source and version govern it, and how the
consumer adds to, changes, or overrides it. This must remain understandable across
layers so developers know where a shared improvement belongs. Differences specific
to a consumer must be clear enough to recognise reusable improvements and
contribute them to the shared source. Keeping copies synchronised does not fulfil
this requirement.

### Returning to and switching between projects

A developer moves between projects maintained by the same person or team, or
returns to one after a break. Several use the same technology, but configure it
differently or organise similar code in different ways. Before making a change,
the developer must rebuild their understanding of how this particular project
works.

Shared approaches should make matching parts familiar across projects, reducing
time spent rediscovering setup and rebuilding that understanding. When adding a
technology, configuration, or implementation, the developer should be able to
consider whether it belongs in a shared source for several projects or only in
the current one. Improvements made at that shared level can benefit the others.
Project-specific needs must remain clear and supported; consistency should make
work easier rather than impose unnecessary uniformity.

### Intentional local changes and partially shared files

A project uses a shared ESLint configuration but needs one rule to differ. That
exception should remain distinguishable from the shared settings; a developer
should also be able to recognise when it could benefit other projects.

A related case is adopting a shared capability that adds a dependency and script
to an existing `package.json`, or settings to `.gitignore` or `tsconfig.json`.
The file already contains the consumer's own choices, so whole-file ownership
does not explain which source governs each part.

The goal in both cases is to keep shared definitions and consumer-owned choices
clearly distinguishable, including when they contribute to the same file.

[github-templates]: https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-repository-from-a-template
[cookiecutter]: https://cookiecutter.readthedocs.io/en/stable/README.html
[copier]: https://copier.readthedocs.io/en/stable/updating/
[copier-creating]: https://copier.readthedocs.io/en/stable/creating/
[copier-versions]: https://copier.readthedocs.io/en/stable/generating/#templates-versions
[copier-multiple]: https://copier.readthedocs.io/en/stable/configuring/#applying-multiple-templates-to-the-same-subproject
[copier-hierarchies]: https://github.com/copier-org/copier/issues/934
[copier-inheritance-lifecycle]: https://github.com/copier-org/copier/issues/934#issuecomment-2899021408
[cruft]: https://cruft.github.io/cruft/
[hygen]: https://github.com/jondot/hygen#readme
[projen]: https://projen.io/docs/introduction/
[npm-package-files]: https://docs.npmjs.com/cli/v11/configuring-npm/package-json#files
[hygen-reviewed-commit]: https://github.com/jondot/hygen/commit/26f76d8dede565b2bf38d0b9e9f3009ee2c8f9ae
[hygen-reviewed-release]: https://github.com/jondot/hygen/releases/tag/v6.2.11
[cruft-reviewed-commit]: https://github.com/cruft/cruft/commit/33f6b722fc6fe4b5d26a351e487372e5e4375ab2
[cruft-reviewed-release]: https://github.com/cruft/cruft/releases/tag/2.16.0
[cookiecutter-reviewed-commit]: https://github.com/cookiecutter/cookiecutter/commit/c88fbe921c97c58b65f1883ba90a0ab53cc91b34
[cookiecutter-reviewed-release]: https://github.com/cookiecutter/cookiecutter/releases/tag/v2.7.1
[copier-reviewed-commit]: https://github.com/copier-org/copier/commit/5d95bfaa3dc585d1d8f6ecabddace634c2c41e29
[copier-reviewed-release]: https://github.com/copier-org/copier/releases/tag/v9.19.1
[projen-reviewed-commit]: https://github.com/projen/projen/commit/c638770dcffca28039ae7d52cba16e2f438e80d2
[projen-reviewed-release]: https://github.com/projen/projen/releases/tag/v0.104.5
[eslint-config]: https://eslint.org/docs/latest/extend/shareable-configs
[typescript-extends]: https://www.typescriptlang.org/tsconfig/extends.html
[prettier-config]: https://prettier.io/docs/sharing-configurations
[pnpm-workspaces]: https://pnpm.io/workspaces
[turbo-workspaces]: https://turborepo.com/docs/crafting-your-repository/structuring-a-repository
[nx-overview]: https://nx.dev/docs/getting-started/intro
[nx-migrate]: https://nx.dev/docs/features/automate-updating-dependencies
[angular-update]: https://angular.dev/cli/update
[git-submodules]: https://git-scm.com/docs/git-submodule
[git-subtrees]: https://github.com/git/git/blob/master/contrib/subtree/git-subtree.adoc
[git-tags]: https://github.com/git/git/tags
[git-releases]: https://github.com/git/git/releases
[mrm]: https://github.com/sapegin/mrm
[mrm-tasks]: https://mrm.js.org/docs/making-tasks
[mrm-policy]: https://github.com/sapegin/mrm/blob/7cf2eb7562696a6881330d001c5d9fdb610bf1ef/Readme.md#contributing
[eslint-reviewed-commit]: https://github.com/eslint/eslint/commit/95e51a0518b15015304eca9844b3f64c1fc167ca
[eslint-reviewed-release]: https://github.com/eslint/eslint/releases/tag/v10.12.0
[typescript-reviewed-commit]: https://github.com/microsoft/TypeScript/commit/257e87406c6a8be277cb8cd5ef8d7b458a650390
[typescript-reviewed-release]: https://github.com/microsoft/TypeScript/releases/tag/v7.0.2
[prettier-reviewed-commit]: https://github.com/prettier/prettier/commit/f40d16101b6bc75f1b3d820b31257a90a4c976fc
[prettier-reviewed-release]: https://github.com/prettier/prettier/releases/tag/3.9.10
[pnpm-reviewed-commit]: https://github.com/pnpm/pnpm/commit/3a1a9b17974cfb689f990a976bbda23acd73fcac
[pnpm-reviewed-release]: https://github.com/pnpm/pnpm/releases/tag/v12.11.2
[turbo-reviewed-commit]: https://github.com/vercel/turborepo/commit/d5aeac9b216d691e5f00938f0915794932f0ff93
[turbo-reviewed-release]: https://github.com/vercel/turborepo/releases/tag/v2.11.8
[nx-reviewed-commit]: https://github.com/nrwl/nx/commit/a1ca38828e991e7e58ca141c3babd536200d69e8
[nx-reviewed-release]: https://github.com/nrwl/nx/releases/tag/23.3.0
[git-reviewed-commit]: https://github.com/git/git/commit/6de20f6092dcf9bdb1c8efe03db4b70c82b423dd
[angular-reviewed-commit]: https://github.com/angular/angular-cli/commit/2b5a9a620495b4c85bf5cfa3db39b632d88e6ca5
[angular-reviewed-release]: https://github.com/angular/angular-cli/releases/tag/v22.2.2
[mrm-reviewed-commit]: https://github.com/sapegin/mrm/commit/7cf2eb7562696a6881330d001c5d9fdb610bf1ef
[mrm-reviewed-release]: https://github.com/sapegin/mrm/releases/tag/mrm%404.1.22
