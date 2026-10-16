## 1. Establish the Rust CLI

- [x] 1.1 Add one Cargo package producing the `hydragen` binary, with `clap`, `serde`, and `serde_json`, a committed `Cargo.lock`, and a narrowly scoped ignore rule for build output. Confirm that the package builds.
- [x] 1.2 Add `hydragen init` with required `--name` and independently optional `--author` and `--description` flags, non-interactive argument handling, top-level and init help, and name-only and optional-metadata examples. Do not add type selectors, prompts, or dependency-management options.

## 2. Implement safe initialisation

- [x] 2.1 Match the manifest schema's metadata rules: require a non-blank name, validate author and description as non-blank strings only when supplied, and preserve accepted values unchanged. Serialise name and only the supplied optional fields as indented UTF-8 JSON with a trailing newline before creating files; never write absent values as empty strings or `null`.
- [x] 2.2 Implement the current-directory repository-location guard described in the design: accept no enclosing `.git` entry or one at the target, reject an ancestor's `.git` file or directory, and report unexpected inspection errors without writes. Do not execute Git or create directories.
- [x] 2.3 Create `layer.json` exclusively without replacing existing files, directories, or symlinks. Leave other project files untouched, report success only after writing completes, and report validation, location, and filesystem failures on standard error with a nonzero exit status and the relevant field or path.

## 3. Verify the command contract

- [x] 3.1 Add focused tests for metadata validation and JSON round-tripping, including name-only input, each optional field independently, all fields together, missing name, empty and whitespace-only supplied values, preserved surrounding whitespace, quotes, backslashes, newlines, and Unicode. Verify the expected property set, omission of absent optional fields, indentation, and trailing newline, and check that validation agrees with the manifest schema's metadata rules.
- [x] 3.2 Add isolated command-level tests for successful initialisation with and without conventional Git metadata, `.git` files, nested-location refusal, missing arguments, help, repeated init, existing malformed manifests, directory and symlink collisions, and preservation of existing project files. Assert exit status and output streams. Confirm exclusive creation cannot overwrite an entry created before opening the output.
- [x] 3.3 Exercise inspection and output-write failures with deterministic tests or documented platform-appropriate checks, including the absence of false success output. Verify a valid invocation with stdin closed and no Git executable available; do not add runtime network or AI dependencies.

## 4. Document and review the increment

- [x] 4.1 Add a README with prerequisites, local build and test commands, name-only and optional-metadata init examples, supported directory layouts, output and error behaviour, and the current limits. Explain that only name is required, init creates a layer, dependencies determine its role, and dependency management, rendering, and contribution capture are not implemented. Read the documentation as a first-time user and resolve unexplained terms or assumptions.
- [x] 4.2 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`; manually try the documented command in a disposable directory. Review alignment with `FOUNDATIONS.md` and `GLOSSARY.md`, recording any newly discovered conflict instead of changing their meaning to fit the implementation.
