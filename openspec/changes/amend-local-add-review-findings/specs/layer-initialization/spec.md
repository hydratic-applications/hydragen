## MODIFIED Requirements

### Requirement: Observe duplicate layer manifests after successful creation

After a successful layer manifest write, init MUST make a best-effort read-only
observation of the supported paths. If multiple paths are observed, it MUST attempt
to emit one duplicate warning identifying the current selection according to layer
manifest priority and MUST still report success for the file actually created.
The warning MUST NOT claim to prove another process ran. Init MUST NOT roll back,
delete, or rewrite either layer manifest or create a lock to prevent this situation.

If the final observation cannot complete, init MUST attempt to warn that the layer
manifest was created but duplicate checking failed, without changing creation's
successful status. Completion write/flush errors returned by the established
fallible reporting path MUST retain reporting-error semantics, including nonzero
exit and an attempted diagnostic identifying completed creation rather than
claiming no files changed. Warning/error diagnostic delivery MUST retain existing
best-effort handling without new propagation or status changes; failure to deliver
the warning MUST NOT change completed-write status. Successful output calls do not
guarantee receipt. The observation MUST NOT promise detection of files created
after it ran.

#### Scenario: Created file is not the highest-priority layer manifest

- **WHEN** init creates `layer.json` after an initially empty check and its final observation also finds `layer.yaml`
- **THEN** it attempts a warning that `layer.yaml` takes priority
- **AND** it exits successfully and reports that it created `layer.json` when the established fallible completion path returns no write/flush error
- **AND** both layer manifests remain unchanged by the observation

#### Scenario: Final observation fails

- **WHEN** the selected layer manifest is successfully created and written but final duplicate checking encounters an inspection error
- **THEN** init attempts the best-effort warning and reports the created path with successful status when the established fallible completion path returns no write/flush error
- **AND** it does not report cancellation, remove the file, or claim no files changed

#### Scenario: Concurrent initialisation selects different formats

- **WHEN** two init operations both initially observe no supported layer manifest paths and later create different supported filenames
- **THEN** both creations can succeed without locks or late all-format rejection
- **AND** each operation attempts a warning about whatever duplicates its own final observation sees
