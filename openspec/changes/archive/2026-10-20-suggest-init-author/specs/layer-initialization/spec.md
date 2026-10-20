## MODIFIED Requirements

### Requirement: Initialisation works without external runtime services

Initialisation MUST NOT require network access, AI services, the Git executable,
or an external schema validator.

Only an eligible guided invocation MAY run optional read-only Git configuration
queries for author suggestions and optional read-only GitHub requests through
`gh`'s existing authentication, within the sources, startup checks, deadlines,
cancellation and cleanup rules defined by the CLI interaction specification.
Local Git lookup MUST remain local; remote requests MUST be limited to that
optional GitHub source. Unavailable tools, credentials, connectivity or usable
results MUST NOT prevent initialisation, require authentication, or produce a
discovery warning. Other available names and optional author entry/omission MUST
remain usable. Flag-only execution and early-exit paths defined by the CLI
interaction specification MUST NOT perform author discovery, including Git
commands, `gh` availability/authentication probes, or GitHub requests.

Init MUST NOT run other Git commands, initialise Git, or modify a repository's
history, configuration, or staging area. It MUST NOT execute project configuration,
package managers or build tools to discover authors, initiate authentication,
request additional permissions, or switch GitHub accounts. Optional discovery
MUST NOT become an initialisation prerequisite.

#### Scenario: Run in an offline non-interactive environment

- **WHEN** valid initialisation is run without terminal input, network access, AI services, or a Git executable
- **AND** all filesystem and location conditions are satisfied
- **THEN** it succeeds using the same command-line interface and layer manifest format

#### Scenario: Guided init can suggest a local Git author while offline

- **WHEN** eligible guided init has a usable local Git author setting but no network access
- **THEN** optional author discovery can offer that name through read-only local Git configuration queries
- **AND** init does not initialise Git, change repository history/configuration/staging, or execute project tools
- **AND** GitHub unavailability does not prevent choosing, entering, or omitting author and completing init

#### Scenario: Guided init works without Git or GitHub access

- **WHEN** eligible guided init runs without Git or usable authenticated GitHub access
- **THEN** discovery silently retains other available names or opens the shared empty optional Author input when none exist
- **AND** the developer can enter or omit author and complete init without authentication questions, Git initialisation, or project-tool execution
