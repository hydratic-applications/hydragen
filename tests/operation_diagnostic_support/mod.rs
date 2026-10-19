//! Real CLI fixtures for the remaining operational families. Rare internal
//! setup/candidate/cleanup states stay in deterministic source-unit seams.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime};

use hydragen::LayerManifestFormat;

pub fn labels() -> Vec<String> {
    let mut labels = [
        "name-empty",
        "name-blank",
        "author-empty",
        "author-blank",
        "description-empty",
        "description-blank",
        "name-case",
        "name-syntax",
        "name-length",
        "name-control",
        "default-case",
        "default-blank",
        "default-control",
        "repo-nested",
        "duplicate-warning",
        "unsafe-yaml-edit",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    for format in LayerManifestFormat::ALL {
        for kind in ["file", "directory"] {
            labels.push(format!("occupied-{}-{kind}", format.extension()));
        }
        #[cfg(unix)]
        for kind in ["symlink", "dangling"] {
            labels.push(format!("occupied-{}-{kind}", format.extension()));
        }
    }
    #[cfg(unix)]
    labels.extend(
        [
            "repo-inspection-loop",
            "init-open-permission",
            "replacement-create-permission",
            "init-write-empty",
            "current-directory-init",
            "current-directory-add",
            "init-completion",
            "add-completion",
            "add-noop-completion",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    // macOS rejects these directory bytes before a CLI can be invoked (EILSEQ).
    // Its no-UTF-8 default is still covered by the existing in-memory unit seam.
    #[cfg(all(unix, not(target_os = "macos")))]
    labels.push("default-nonutf8".into());
    labels
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Unchanged,
    Occupied,
    Partial,
    Created,
    Updated,
    Noop,
    Removed,
}

pub struct Case {
    pub label: String,
    #[cfg(unix)]
    permission_guard: Option<super::permission_support::PermissionGuard>,
    root: tempfile::TempDir,
    pub app: PathBuf,
    pub arguments: Vec<String>,
    pub shell_prelude: Option<String>,
    #[cfg(unix)]
    output_failure: bool,
    #[allow(dead_code)]
    pub terminal_eligible: bool,
    state: State,
    summary: String,
    facts: Vec<String>,
    absent: Vec<String>,
    snapshot: Vec<Entry>,
}

struct Entry {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    link: Option<PathBuf>,
    modified: SystemTime,
    len: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    mode: u32,
}

impl Case {
    // Non-permission callers must not silently turn an unsupported fixture into
    // success. Family consumers use try_new and handle each outcome explicitly.
    #[allow(dead_code)]
    pub fn new(label: &str) -> Self {
        match Self::try_new(label).expect("operation fixture setup failed") {
            super::permission_support::Setup::Ready(case) => case,
            super::permission_support::Setup::Unsupported(reason) => panic!("{label}: {reason}"),
        }
    }

    pub fn try_new(label: &str) -> std::io::Result<super::permission_support::Setup<Self>> {
        use super::permission_support::Setup;
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("keep.txt"), "untouched user content\n").unwrap();
        let mut case = Self {
            label: label.into(),
            #[cfg(unix)]
            permission_guard: None,
            root,
            app,
            arguments: vec!["init".into(), "--name".into(), "app".into()],
            shell_prelude: None,
            #[cfg(unix)]
            output_failure: false,
            terminal_eligible: true,
            state: State::Unchanged,
            summary: String::new(),
            facts: Vec::new(),
            absent: vec![
                "Usage:".into(),
                "Example:".into(),
                "For more information,".into(),
            ],
            snapshot: Vec::new(),
        };
        if let Some(rest) = label.strip_prefix("occupied-") {
            let (extension, kind) = rest.split_once('-').unwrap();
            let filename = format!("layer.{extension}");
            let path = case.app.join(&filename);
            match kind {
                "file" => fs::write(&path, b"unread invalid contents \xff").unwrap(),
                "directory" => {
                    fs::create_dir(&path).unwrap();
                    fs::write(path.join("keep"), "keep").unwrap();
                }
                #[cfg(unix)]
                "symlink" | "dangling" => {
                    let target = case.root.path().join("target");
                    if kind == "symlink" {
                        fs::write(&target, "keep target").unwrap();
                    }
                    std::os::unix::fs::symlink(target, path).unwrap();
                }
                _ => unreachable!(),
            }
            case.state = State::Occupied;
            case.summary = format!("This directory already has a layer manifest ({filename}).");
            case.facts.push(
                "Initialisation creates a new layer manifest; it won’t overwrite an existing one."
                    .into(),
            );
            case.absent
                .extend(strings(&["Tip:", "delete", "--force", "invalid contents"]));
        } else {
            match label {
                "name-empty" | "name-blank" | "author-empty" | "author-blank"
                | "description-empty" | "description-blank" => {
                    let (field, kind) = label.split_once('-').unwrap();
                    let flag = format!("--{field}");
                    if field == "name" {
                        case.arguments.pop();
                    } else {
                        case.arguments.push(flag.clone());
                    }
                    case.arguments
                        .push(if kind == "empty" { "" } else { " \t\n\u{85}" }.into());
                    case.summary = format!("Invalid value for '{flag}'.");
                    case.facts.push(
                        "The value must contain at least one non-whitespace character.".into(),
                    );
                    case.facts
                        .push(format!("Tip: Supply a non-blank value for '{flag}'"));
                    if field != "name" {
                        case.facts.push("omit this optional flag".into());
                    } else {
                        case.absent.push("omit this optional flag".into());
                    }
                }
                "name-case" | "name-syntax" | "name-length" | "name-control" => {
                    case.arguments[2] = match label {
                        "name-case" => "Invalid-Name".into(),
                        "name-syntax" => "two words".into(),
                        "name-length" => "a".repeat(215),
                        _ => "bad\x1b[2J\n\u{202e}".into(),
                    };
                    case.summary = "Invalid value for '--name'.".into();
                    case.facts.extend(strings(&[
                        "lowercase",
                        "Correct '--name'",
                        "names are not rewritten automatically",
                    ]));
                    if label == "name-length" {
                        case.facts.push("214".into());
                    }
                    case.absent.extend(strings(&[
                        "non-blank",
                        "non-whitespace",
                        "omit this optional flag",
                    ]));
                }
                "default-case" | "default-blank" | "default-control" | "default-nonutf8" => {
                    let name = match label {
                        "default-case" => std::ffi::OsString::from("Invalid-Directory"),
                        "default-blank" => "   ".into(),
                        "default-control" => "bad\x1b[2J\n".into(),
                        #[cfg(unix)]
                        _ => {
                            use std::os::unix::ffi::OsStringExt;
                            std::ffi::OsString::from_vec(b"bad\xff".to_vec())
                        }
                        #[cfg(not(unix))]
                        _ => unreachable!(),
                    };
                    let new = case.root.path().join(name);
                    fs::rename(&case.app, &new).unwrap();
                    case.app = new;
                    case.arguments = strings(&["init", "--format", "yaml"]);
                    case.summary = format!(
                        "Could not use directory '{}' as the layer name.",
                        shown(&fs::canonicalize(&case.app).unwrap())
                    );
                    case.facts
                        .push("Supply --name explicitly with a valid layer name".into());
                    case.facts.push(
                        if label == "default-nonutf8" {
                            "no usable UTF-8 name"
                        } else if label == "default-blank" {
                            "non-whitespace"
                        } else {
                            "lowercase"
                        }
                        .into(),
                    );
                }
                "repo-nested" => {
                    fs::create_dir(case.root.path().join(".git")).unwrap();
                    case.summary = "Cannot initialise a layer below the repository root.".into();
                    case.facts.push(format!(
                        "repository root '{}'",
                        shown(&fs::canonicalize(case.root.path()).unwrap())
                    ));
                }
                "duplicate-warning" => {
                    for format in LayerManifestFormat::ALL {
                        fs::write(case.app.join(format.filename()), b"not parsed \xff").unwrap();
                    }
                    case.state = State::Occupied;
                    case.summary =
                        "This directory already has a layer manifest (layer.yaml).".into();
                    case.facts.push("Initialisation creates a new layer manifest; it won’t overwrite an existing one.".into());
                }
                "unsafe-yaml-edit" => {
                    case.add_fixture("name: app\ndependencies: &deps []\ncustom: *deps\n");
                    case.summary = format!(
                        "Could not safely edit yaml layer manifest '{}'.",
                        case.selected()
                    );
                    case.facts.extend(strings(&[
                        "Details:",
                        "Adjust the source construct",
                        "retaining its intended values and comments",
                    ]));
                }
                #[cfg(unix)]
                "repo-inspection-loop" => {
                    std::os::unix::fs::symlink(".git", case.app.join(".git")).unwrap();
                    case.summary = format!(
                        "Could not inspect repository marker '{}'.",
                        shown(&fs::canonicalize(&case.app).unwrap().join(".git"))
                    );
                    case.facts
                        .extend(strings(&["Cause:", "operating-system cause"]));
                }
                #[cfg(unix)]
                "init-open-permission" | "replacement-create-permission" => {
                    if label == "replacement-create-permission" {
                        case.add_fixture("name: app\n");
                        case.summary = format!(
                            "Could not create a sibling staging file at '{}'.",
                            shown(&fs::canonicalize(&case.app).unwrap())
                        );
                        case.facts.extend(strings(&[
                            "Cause:",
                            "Permission denied",
                            "was not replaced",
                            "Check directory access and available space",
                        ]));
                        case.absent.push("No files were changed".into());
                        case.facts.push(format!(
                            "at path \"{}/.hydragen-layer-manifest-",
                            shown(&fs::canonicalize(&case.app).unwrap())
                        ));
                        case.facts.push(format!(
                            "The selected layer manifest '{}' was not replaced.",
                            case.selected()
                        ));
                    } else {
                        case.summary =
                            format!("Could not create layer manifest '{}'.", case.selected());
                        case.facts.extend(strings(&[
                            "Cause:",
                            "Permission denied",
                            "Check access permissions",
                        ]));
                    }
                    match super::permission_support::deny_create(&case.app)? {
                        Setup::Ready(guard) => case.permission_guard = Some(guard),
                        Setup::Unsupported(reason) => return Ok(Setup::Unsupported(reason)),
                    }
                }
                #[cfg(unix)]
                "init-write-empty" => {
                    case.state = State::Partial;
                    // This child deliberately forbids regular-file writes,
                    // including LLVM profiles. Keep profiler output separate
                    // from the unchanged CLI diagnostic assertions.
                    case.shell_prelude = Some(
                        "export LLVM_PROFILE_FILE=/dev/null\ntrap '' XFSZ\nulimit -f 0".into(),
                    );
                    case.summary = format!("Could not write layer manifest '{}'.", case.selected());
                    case.facts.extend(strings(&[
                        "Cause: File too large",
                        "An empty or partially written layer manifest may remain",
                        "this command did not remove it",
                        "\n  Tip: Inspect the newly created file and resolve the reported write failure before retrying.\n",
                    ]));
                    case.absent
                        .extend(strings(&["available space", "directory access"]));
                }
                #[cfg(unix)]
                "current-directory-init" | "current-directory-add" => {
                    fs::remove_file(case.app.join("keep.txt")).unwrap();
                    case.arguments = if label.ends_with("add") {
                        strings(&["add", "../rules"])
                    } else {
                        strings(&["init", "--name", "app"])
                    };
                    case.state = State::Removed;
                    case.shell_prelude = Some("rmdir \"$PWD\"".into());
                    case.summary = "Could not determine the current directory.".into();
                    case.facts.extend(strings(&[
                        "Cause:",
                        "working directory still exists and is accessible",
                    ]));
                }
                #[cfg(unix)]
                "init-completion" | "add-completion" | "add-noop-completion" => {
                    // This inherently redirected case uses the same bounded
                    // private-reader runner in captured and terminal consumers.
                    case.output_failure = true;
                    case.terminal_eligible = false;
                    case.facts
                        .extend(strings(&["output destination", "was already"]));
                    if label == "init-completion" {
                        case.state = State::Created;
                        case.summary = "Could not report initialisation completion.".into();
                        case.facts.push(format!(
                            "Layer manifest '{}' was already created",
                            case.selected()
                        ));
                    } else {
                        let noop = label == "add-noop-completion";
                        case.add_fixture(if noop {
                            "name: app\ndependencies: [../rules]\n"
                        } else {
                            "name: app\n"
                        });
                        case.state = if noop { State::Noop } else { State::Updated };
                        case.summary = if noop {
                            "Could not report the no-change result for 'app'."
                        } else {
                            "Could not report completion for 'app'."
                        }
                        .into();
                        if noop {
                            case.facts.retain(|fact| fact != "was already");
                            case.facts.extend(strings(&[
                                "requested dependencies were already present",
                                "layer manifest was not updated",
                            ]));
                        } else {
                            case.facts.push("was already updated".into());
                        }
                    }
                }
                _ => panic!("unknown operation case {label}"),
            }
        }
        case.snapshot = snapshot(case.root.path());
        Ok(Setup::Ready(case))
    }

    fn add_fixture(&mut self, source: &str) {
        fs::write(self.app.join("layer.yaml"), source).unwrap();
        fs::create_dir(self.root.path().join("rules")).unwrap();
        fs::write(self.root.path().join("rules/layer.yaml"), "name: rules\n").unwrap();
        self.arguments = strings(&["add", "../rules"]);
    }

    fn selected(&self) -> String {
        shown(&fs::canonicalize(&self.app).unwrap().join("layer.yaml"))
    }

    pub fn assert_output(&self, text: &str) {
        let error = text.find("Error: ").expect("operational error");
        let compact = &text[error..];
        super::diagnostic_body_support::assert_compact(compact);
        assert_eq!(
            compact.lines().next().unwrap(),
            format!("Error: {}", self.summary),
            "{}: {text}",
            self.label
        );
        for fact in &self.facts {
            assert!(text.contains(fact), "{} lost {fact:?}: {text}", self.label);
        }
        for fact in &self.absent {
            assert!(
                !text.contains(fact),
                "{} unexpected {fact:?}: {text}",
                self.label
            );
        }
        let no_change = matches!(self.state, State::Unchanged | State::Removed)
            && self.label != "replacement-create-permission";
        assert_eq!(
            compact.matches("No files were changed.").count(),
            usize::from(no_change),
            "{} state: {text}",
            self.label
        );
        if self.label == "duplicate-warning" {
            let prefix = &text[..error];
            super::diagnostic_body_support::assert_warning(
                prefix.strip_suffix("✕ ").unwrap_or(prefix),
            );
            assert_eq!(text.matches("Warning:").count(), 1, "{text}");
            assert!(text.find("Warning:").unwrap() < error, "{text}");
            assert!(text.contains("layer.yaml, layer.yml, layer.toml, layer.json, layer.jsonc."));
            assert!(text.contains("layer.yaml takes priority"));
            assert!(text.contains("Tip: Layer manifests are not merged"));
        }
        self.assert_disk();
    }

    fn assert_disk(&self) {
        if self.state == State::Removed {
            assert!(!self.app.exists());
            assert_eq!(fs::read_dir(self.root.path()).unwrap().count(), 0);
            return;
        }
        let new_file = matches!(self.state, State::Partial | State::Created);
        let changed_file = self.state == State::Updated;
        let target = self.app.join("layer.yaml");
        for old in &self.snapshot {
            if (new_file && old.path == self.app)
                || (changed_file && (old.path == self.app || old.path == target))
            {
                continue;
            }
            let metadata = fs::symlink_metadata(&old.path).unwrap();
            assert_eq!(
                metadata.modified().unwrap(),
                old.modified,
                "{} mtime: {:?}",
                self.label,
                old.path
            );
            assert_eq!(
                metadata.len(),
                old.len,
                "{} size: {:?}",
                self.label,
                old.path
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                assert_eq!(metadata.ino(), old.inode);
                assert_eq!(metadata.permissions().mode(), old.mode);
            }
            if let Some(bytes) = &old.bytes {
                assert_eq!(fs::read(&old.path).unwrap(), *bytes);
            }
            if let Some(link) = &old.link {
                assert_eq!(fs::read_link(&old.path).unwrap(), *link);
            }
        }
        let after = snapshot(self.root.path());
        let mut expected = self
            .snapshot
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        if new_file {
            expected.push(target.clone());
        }
        expected.sort();
        let mut actual = after
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>();
        actual.sort();
        assert_eq!(actual, expected, "{} changed entries/staging", self.label);
        if self.state == State::Partial {
            assert_eq!(fs::read(&target).unwrap(), b"");
        }
        if matches!(self.state, State::Created | State::Updated | State::Noop) {
            let document: serde_json::Value =
                serde_saphyr::from_str(&fs::read_to_string(target).unwrap()).unwrap();
            hydragen::layer_manifest::schema::validate(&document).unwrap();
            assert_eq!(document["name"], "app");
            if self.state != State::Created {
                assert_eq!(document["dependencies"], serde_json::json!(["../rules"]));
            }
        }
    }

    #[allow(dead_code)]
    pub fn comparable(&self, text: &str) -> String {
        let mut text = text
            .strip_prefix("✕ ")
            .unwrap_or(text)
            .replace(
                &shown(&fs::canonicalize(self.root.path()).unwrap()),
                "<fixture>",
            )
            .replace(&shown(self.root.path()), "<fixture>");
        if self.label == "replacement-create-permission" {
            // Compare meaning across separate native attempts without demanding
            // the same random tempfile suffix. The real transcript retains the
            // exact path; assert_output and disk checks are never normalised.
            let prefix = ".hydragen-layer-manifest-";
            let start = text.find(prefix).expect("precise attempted staging path") + prefix.len();
            let count = text[start..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .count();
            assert_eq!(count, 6, "unexpected staging filename: {text}");
            text.replace_range(start..start + count, "<unique>");
        }
        text
    }

    #[allow(dead_code)]
    pub fn captured(&self, environment: &[(&str, &str)]) -> Output {
        #[cfg(unix)]
        if self.output_failure {
            let arguments = self
                .arguments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            return super::output_failure_support::run_environment(
                &self.app,
                &arguments,
                super::output_failure_support::Stream::Stdout,
                super::output_failure_support::Delivery::BrokenPipe,
                environment,
            )
            .expect("bounded completion-output fixture failed")
            .output;
        }
        let stdout = tempfile::NamedTempFile::new().unwrap();
        let stderr = tempfile::NamedTempFile::new().unwrap();
        let mut command = if let Some(prelude) = &self.shell_prelude {
            let mut command = Command::new("/bin/sh");
            command.args([
                "-c",
                &format!("{prelude}\nexec \"$@\""),
                "operation-fixture",
                env!("CARGO_BIN_EXE_hydragen"),
            ]);
            command
        } else {
            Command::new(env!("CARGO_BIN_EXE_hydragen"))
        };
        command
            .current_dir(&self.app)
            .args(&self.arguments)
            .stdin(Stdio::null());
        // RLIMIT_FSIZE must affect the new layer manifest, not suppress its
        // diagnostic by also limiting our regular capture files. This bounded
        // fixture's diagnostic fits comfortably in a pipe; timeout/Drop still
        // terminates a child that unexpectedly fills it.
        if self.state == State::Partial {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
        } else {
            command
                .stdout(stdout.reopen().unwrap())
                .stderr(stderr.reopen().unwrap());
        }
        for key in [
            "CI",
            "NO_COLOR",
            "CLICOLOR",
            "CLICOLOR_FORCE",
            "FORCE_COLOR",
            "TERM",
        ] {
            command.env_remove(key);
        }
        command.envs(environment.iter().copied());
        let mut child = Running(command.spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "{} timed out", self.label);
            std::thread::sleep(Duration::from_millis(10));
        };
        if self.state == State::Partial {
            use std::io::Read;
            let mut out = Vec::new();
            let mut err = Vec::new();
            child
                .0
                .stdout
                .take()
                .unwrap()
                .read_to_end(&mut out)
                .unwrap();
            child
                .0
                .stderr
                .take()
                .unwrap()
                .read_to_end(&mut err)
                .unwrap();
            Output {
                status,
                stdout: out,
                stderr: err,
            }
        } else {
            Output {
                status,
                stdout: fs::read(stdout.path()).unwrap(),
                stderr: fs::read(stderr.path()).unwrap(),
            }
        }
    }
}

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}
fn shown(path: &Path) -> String {
    hydragen::diagnostic_text::display(path.display()).to_string()
}

fn snapshot(path: &Path) -> Vec<Entry> {
    let metadata = fs::symlink_metadata(path).unwrap();
    let mut entries = vec![Entry {
        path: path.into(),
        bytes: metadata.is_file().then(|| fs::read(path).unwrap()),
        link: metadata
            .file_type()
            .is_symlink()
            .then(|| fs::read_link(path).unwrap()),
        modified: metadata.modified().unwrap(),
        len: metadata.len(),
        #[cfg(unix)]
        inode: {
            use std::os::unix::fs::MetadataExt;
            metadata.ino()
        },
        #[cfg(unix)]
        mode: {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode()
        },
    }];
    if metadata.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            entries.extend(snapshot(&entry.unwrap().path()));
        }
    }
    entries
}
