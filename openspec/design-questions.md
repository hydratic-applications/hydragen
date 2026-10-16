# Open design questions

These questions are shared planning context for OpenSpec changes to Hydragen.
They describe unresolved product and design choices, not agreed requirements.
[FOUNDATIONS.md](../FOUNDATIONS.md) records the product's beliefs and goals;
[AGENTS.md](../AGENTS.md#openspec-planning-and-decisions) explains how to use and
maintain this question list.

Resolve the questions relevant to the current change as they become necessary.
The examples below are possibilities, not selected designs or a requirement to
design everything upfront.

1. **Audience and initial scope.** Who should the first useful version serve:
   individual developers, teams, or both? Which ecosystems should it support first,
   and how broad should support beyond them be?
2. **Shared sources and references.** Where do shared sources live—a repository,
   package, or directory? How do developers and agents find available sources and
   decide which to use? How does a consumer select a source and version, and which
   commands support that workflow? Should references pin a version, follow updates
   automatically, or support both by consumer choice?
   - **Partial resolution:** Every layer has a manifest declaring its required
     name, optional author and description, and the other layers it depends on. Name
     and any supplied author or description must be non-blank strings. An
     omitted dependency list and an empty list both mean no dependencies. See the
     [manifest decision and draft schema choices](changes/archive/2026-10-16-define-layer-manifest/design.md#decisions).
     Source locations, reference syntax, and how versioning works (including
     whether it belongs in the manifest) remain open.
3. **Files and tool dependency.** Are shared files committed into a consumer's
   repository, or created or linked by Hydragen when needed? What project layout
   does this require? Can collaborators and continuous integration (CI) build,
   run, and edit the project without Hydragen installed? What must work after a
   plain clone?
   - **Partial resolution:** The layer manifest is a JSON file named exactly
     `layer.json` at the root of the layer's repository. See the
     [manifest location decision](changes/archive/2026-10-16-define-layer-manifest/design.md#agreed-manifest-filename-location-and-format).
   - **Partial resolution:** The CLI renders the layer result of a dependent layer
     into its workspace. The layer result is the files and directory structure
     produced by applying its contributions to inherited content; the workspace
     is the directory where developers and agents edit those files. Workspace
     files are not committed; the repository records the manifest
     and the layer's own contributions. The CLI handles translation between the
     two. See [the layer model](../FOUNDATIONS.md#the-layer-model).
     Directory layout, how files are made available, and installation requirements
     for collaborators and CI remain open.
4. **Local changes.** May a consumer edit a shared file in place? How would a local
   exception, a change to the shared source, or a decision to stop sharing be
   distinguished, and what happens to each on the next update? How can a consumer
   propose or contribute a reusable improvement back to the shared source?
   - **Partial resolution:** Developers and agents edit ordinary files in the
     dependent layer's workspace. The CLI uses
     [contribution capture](../GLOSSARY.md#contribution-capture) to reflect those
     edits in its contributions rather than requiring users to author patches or
     deletion markers during ordinary editing. See
     [the layer model](../FOUNDATIONS.md#the-layer-model).
     Commands and translation timing, handling shared-source improvements, stopping
     sharing, and update behaviour remain open.
5. **Partially shared files.** How are shared dependencies, scripts, or settings
   combined with consumer-owned content in one file? How are conflicting choices
   resolved while preserving one [authoritative source](../GLOSSARY.md#authoritative-source)
   for each shared part?
6. **Incompatible updates.** What happens when a shared source removes a file,
   changes its structure, or requires consumers to change their code? Who supplies
   and runs migration steps, and how are updates across layers coordinated?
7. **Product boundaries and non-goals.** Which responsibilities should Hydragen
   explicitly leave to existing tools—for example package management, monorepo
   coordination, builds, or Git operations? These exclusions, called non-goals,
   have not been agreed. Ordinary packages remain appropriate where they already fit.
8. **Rust and the CLI.** What benefits motivate the intended Rust implementation,
   and should this detail remain in the product foundations or move to an
   implementation document?
9. **Agent interfaces.** Which purpose-built interfaces best support agents, and
   which should come first? Options include an MCP server (Model Context Protocol,
   a standard way for AI applications to use tools) and skills—reusable instructions
   that teach agents how to use Hydragen. The [AI support value](../FOUNDATIONS.md#guiding-values)
   sets the requirement; the specific interfaces remain open.
10. **Success criteria.** Which real projects and observations should we use to
    assess success? The assessment should cover:
    - Effort to adopt reuse compared with editing directly.
    - Clarity of source definitions, selected versions, ownership, and how a
      consumer changes or extends inherited content.
    - Work saved by shared improvements and effort spent returning to or switching
      between projects.
    - The usefulness of the workflows described in the AI support value.
11. **Performance targets.** What response times and resource limits should startup
    and common operations meet, and for what project sizes?
12. **Patch structure.** What internal structure should `.patch` files use to
    describe modifications to inherited files? The filename suffix does not select
    an existing patch format. See [Patch](../GLOSSARY.md#patch) for the agreed meaning.
