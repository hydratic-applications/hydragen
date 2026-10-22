//! The same verification scenarios drive captured and real-terminal checks.
//! Synthetic/unreachable setup and I/O variants remain in the binary unit tests;
//! these fixtures exercise the public command and real filesystem policies.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use hydragen::LayerManifestFormat;

pub const CASES: &[&str] = &[
    "missing-manifest",
    "missing-directory",
    "not-directory",
    "invalid-reference",
    "published-add",
    "invalid-utf8",
    "decode-yaml",
    "decode-yml",
    "decode-toml",
    "decode-json",
    "decode-jsonc",
    "metadata-object",
    "metadata-required",
    "metadata-type",
    "metadata-type-number",
    "metadata-blank",
    "metadata-name",
    "schema-location",
    "current-decode",
    "current-nonregular",
    "dependency-nonregular",
    "existing-missing-directory",
    "existing-missing-manifest",
    "existing-decode",
    "existing-published-syntax",
    "dependency-published-syntax",
    "existing-published-conflict",
    "existing-local-conflict",
    "existing-duplicates",
    "existing-identical-duplicates",
    "existing-alias-duplicates",
    "existing-distinct-directory-conflict",
    "existing-self-conflict",
    "self-conflict",
    "nested-relative",
    "nested-absolute",
    "nested-conflict-previous-base",
    "mixed-existing-and-arguments",
    "repeated-ambiguous",
    "escaped-label-collision",
    #[cfg(not(windows))]
    "unsupported-host-path",
    #[cfg(unix)]
    "current-symlink",
    #[cfg(unix)]
    "dependency-symlink-decode",
    #[cfg(unix)]
    "dependency-dangling-symlink",
    #[cfg(unix)]
    "alias-selected-decode",
    #[cfg(unix)]
    "permission-denied",
];

pub struct Case {
    pub label: &'static str,
    #[cfg(unix)]
    permission_guard: Option<super::permission_support::PermissionGuard>,
    pub root: tempfile::TempDir,
    pub app: PathBuf,
    pub invocation: PathBuf,
    pub arguments: Vec<String>,
    facts: Vec<String>,
    absent: Vec<String>,
    labels: Vec<String>,
    base: bool,
    exact: Option<String>,
    preserved: Vec<Snapshot>,
    unreadable: Option<PathBuf>,
}

struct Snapshot {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
    modified: SystemTime,
    len: u64,
    link: Option<PathBuf>,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    mode: u32,
}

impl Case {
    pub fn try_new(label: &'static str) -> std::io::Result<super::permission_support::Setup<Self>> {
        use super::permission_support::Setup;
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("app");
        fs::create_dir(&app).unwrap();
        fs::write(app.join("layer.yaml"), "name: app\n").unwrap();
        let mut case = Self {
            label,
            #[cfg(unix)]
            permission_guard: None,
            root,
            invocation: app.clone(),
            app,
            arguments: vec!["add".into(), "../rules".into()],
            facts: Vec::new(),
            absent: vec!["oneOf".into(), "schemas listed".into()],
            labels: Vec::new(),
            base: false,
            exact: None,
            preserved: Vec::new(),
            unreadable: None,
        };
        case.layer("rules", "rules");
        match label {
            "missing-manifest" | "nested-relative" | "nested-absolute" => {
                fs::create_dir(case.root.path().join("layer-b")).unwrap();
                case.arguments[1] = "../layer-b".into();
                if label.starts_with("nested") {
                    case.nested();
                }
                if label == "nested-absolute" {
                    case.arguments[1] = case.root.path().join("layer-b").display().to_string();
                    case.base = false;
                }
                let reference = case.arguments[1].clone();
                case.facts
                    .push(format!("No layer manifest found in '{reference}'."));
                case.facts
                    .push(format!("in '{reference}' to initialise it as a layer."));
                if label == "missing-manifest" {
                    case.exact = Some("Error: No layer manifest found in '../layer-b'.\n\n  Tip: Choose an existing layer directory, or run 'hydragen init'\n  in '../layer-b' to initialise it as a layer.\n\n  No files were changed.\n".into());
                    case.absent.push(case.root.path().display().to_string());
                }
            }
            "missing-directory" => {
                case.arguments[1] = "../missing".into();
                case.facts.extend(strings(&[
                    "Could not resolve local directory '../missing'.",
                    "Cause:",
                    "path and its parent directories exist",
                ]));
                case.absent.push("hydragen init".into());
                case.absent.push(case.root.path().display().to_string());
            }
            "not-directory" => {
                fs::write(case.root.path().join("file"), "keep").unwrap();
                case.arguments[1] = "../file".into();
                case.facts.extend(strings(&[
                    "Local source '../file' is not a directory.",
                    "not a file",
                ]));
                case.absent.push("hydragen init".into());
            }
            "invalid-reference" => {
                case.arguments[1] = "bare-name".into();
                case.facts.extend(strings(&[
                    "Invalid source reference 'bare-name'.",
                    "./ or ../",
                    "Bare names are not local paths",
                ]));
                case.absent
                    .extend(strings(&["Schema rule:", "hydragen init", "non-blank"]));
            }
            "published-add" => {
                case.arguments[1] = "@example/rules@1.2.3-beta.1+build.7".into();
                case.facts.extend(strings(&["Adding published layer '@example/rules@1.2.3-beta.1+build.7' is not supported yet.", "does not access a registry"]));
                case.absent.push("hydragen init".into());
            }
            "unsupported-host-path" => {
                case.arguments[1] = r"C:\layers\rules".into();
                case.facts.extend(strings(&[
                    r"Cannot resolve local path 'C:\layers\rules' on this host.",
                    "Windows drive or backslash paths are not native here",
                    "not expanded",
                ]));
                case.absent.push("hydragen init".into());
            }
            "invalid-utf8" => {
                fs::write(case.root.path().join("rules/layer.yaml"), [0xff, 0xfe]).unwrap();
                case.facts.extend(strings(&[
                    "Could not read layer manifest '../rules/layer.yaml'.",
                    "Cause:",
                    "UTF-8",
                ]));
                case.absent
                    .extend(strings(&["hydragen init", "Correct the yaml syntax"]));
            }
            #[cfg(unix)]
            "permission-denied" => {
                case.unreadable = Some(case.root.path().join("rules/layer.yaml"));
                case.facts.extend(strings(&[
                    "Could not read layer manifest '../rules/layer.yaml'.",
                    "Cause:",
                    "Check access permissions for the path and its parent directories",
                ]));
                case.absent.extend(strings(&[
                    "hydragen init",
                    "UTF-8",
                    "Correct the yaml syntax",
                ]));
            }
            #[cfg(unix)]
            "current-symlink"
            | "dependency-symlink-decode"
            | "dependency-dangling-symlink"
            | "alias-selected-decode" => {
                use std::os::unix::fs::symlink;
                if label == "current-symlink" {
                    let external = case.root.path().join("external.yaml");
                    fs::rename(case.app.join("layer.yaml"), &external).unwrap();
                    symlink(&external, case.app.join("layer.yaml")).unwrap();
                    case.facts.push(format!(
                        "Cannot edit symlinked current layer manifest '{}'.",
                        fs::canonicalize(&case.app)
                            .unwrap()
                            .join("layer.yaml")
                            .display()
                    ));
                    case.facts.extend(strings(&[
                        "regular layer manifest file instead",
                        "link and its target have not been changed",
                    ]));
                    case.absent.push("../rules".into());
                } else if label == "alias-selected-decode" {
                    fs::write(case.root.path().join("rules/layer.yaml"), "[").unwrap();
                    symlink(
                        case.root.path().join("rules"),
                        case.root.path().join("alias"),
                    )
                    .unwrap();
                    case.arguments[1] = "../alias/./".into();
                    case.facts.push(
                        "Could not decode yaml layer manifest '../alias/./layer.yaml'.".into(),
                    );
                    case.absent.push(case.root.path().display().to_string());
                } else {
                    let external = case.root.path().join("external.yaml");
                    fs::remove_file(case.root.path().join("rules/layer.yaml")).unwrap();
                    if label == "dependency-symlink-decode" {
                        fs::write(&external, "[").unwrap();
                    }
                    symlink(&external, case.root.path().join("rules/layer.yaml")).unwrap();
                    if label == "dependency-symlink-decode" {
                        case.facts.push(
                            "Could not decode yaml layer manifest '../rules/layer.yaml'.".into(),
                        );
                    } else {
                        case.facts.extend(strings(&[
                            "Could not read layer manifest '../rules/layer.yaml'.",
                            "Cause:",
                            "path and its parent directories exist",
                        ]));
                    }
                    case.absent.push("Cannot edit symlinked".into());
                    case.absent.push(external.display().to_string());
                }
                case.absent.push("hydragen init".into());
            }
            label if label.starts_with("decode-") => {
                let format = match label {
                    "decode-yaml" => LayerManifestFormat::Yaml,
                    "decode-yml" => LayerManifestFormat::Yml,
                    "decode-toml" => LayerManifestFormat::Toml,
                    "decode-json" => LayerManifestFormat::Json,
                    _ => LayerManifestFormat::Jsonc,
                };
                fs::remove_file(case.root.path().join("rules/layer.yaml")).unwrap();
                let source = if format == LayerManifestFormat::Toml {
                    "name = [\n"
                } else {
                    "[\n"
                };
                fs::write(
                    case.root.path().join("rules").join(format.filename()),
                    source,
                )
                .unwrap();
                case.facts.push(format!(
                    "Could not decode {format} layer manifest '../rules/{}'.",
                    format.filename()
                ));
                case.facts.extend(strings(&[
                    "Details:",
                    "line",
                    "column",
                    "Lower-priority files and parent layers are not fallbacks.",
                ]));
                case.facts.push(format!("Tip: Correct the {format} syntax"));
                case.absent.push(case.root.path().display().to_string());
                case.absent.push("hydragen init".into());
            }
            "metadata-object"
            | "metadata-required"
            | "metadata-type"
            | "metadata-type-number"
            | "metadata-blank"
            | "metadata-name"
            | "schema-location" => {
                let (source, facts) = match label {
                    "metadata-object" => ("[]", strings(&["one object/mapping document"])),
                    "metadata-required" => (
                        "{}",
                        strings(&[
                            "Missing required metadata field 'name'",
                            "Add the required 'name' field",
                        ]),
                    ),
                    "metadata-type" | "metadata-type-number" => (
                        if label == "metadata-type" {
                            "name: rules\nauthor: null\n"
                        } else {
                            "name: rules\nauthor: 42\n"
                        },
                        strings(&[
                            "/author (type):",
                            "not of types \"string\", \"object\"",
                            "Schema rule: '/$defs/author/type'",
                            "Tip: Use the value type required by the reported schema constraint.",
                        ]),
                    ),
                    "metadata-blank" => (
                        "name: rules\ndescription: '  '\n",
                        strings(&[
                            "Metadata field 'description'",
                            "non-whitespace",
                            "non-blank",
                        ]),
                    ),
                    "metadata-name" => (
                        "name: Invalid-Name\n",
                        strings(&["Metadata field 'name' is invalid", "lowercase"]),
                    ),
                    _ => (
                        "name: rules\ndependencies: [7]\n",
                        strings(&["/dependencies/0", "string", "Schema rule:"]),
                    ),
                };
                fs::write(case.root.path().join("rules/layer.yaml"), source).unwrap();
                case.facts = facts;
                case.facts
                    .push("Invalid layer manifest '../rules/layer.yaml'.".into());
                case.absent.extend(strings(&["hydragen init", "--name"]));
                if matches!(label, "metadata-type" | "metadata-type-number") {
                    case.absent.extend(strings(&[
                        "Metadata field 'author' must be a string",
                        "omit it only if it is optional",
                    ]));
                }
                case.absent.push(case.root.path().display().to_string());
                if label == "metadata-type-number" {
                    case.absent.push("null".into());
                }
            }
            "current-decode" => {
                case.current("[");
                case.facts.push(format!(
                    "Could not decode yaml layer manifest '{}'.",
                    fs::canonicalize(&case.app)
                        .unwrap()
                        .join("layer.yaml")
                        .display()
                ));
                case.facts
                    .extend(strings(&["Details:", "Correct the yaml syntax"]));
                case.absent.extend(strings(&["../rules", "hydragen init"]));
            }
            "current-nonregular" | "dependency-nonregular" => {
                let path = if label == "current-nonregular" {
                    case.app.join("layer.yaml")
                } else {
                    case.root.path().join("rules/layer.yaml")
                };
                fs::remove_file(&path).unwrap();
                fs::create_dir(&path).unwrap();
                if label == "current-nonregular" {
                    case.facts
                        .push(fs::canonicalize(&path).unwrap().display().to_string());
                    case.facts
                        .push("The selected current layer manifest is not a regular file.".into());
                    case.absent.push("../rules".into());
                } else {
                    case.facts.extend(strings(&[
                        "'../rules/layer.yaml'",
                        "regular file (or a symlink to one)",
                    ]));
                }
                case.facts
                    .push("Lower-priority files and parent layers are not fallbacks.".into());
                case.absent.push("hydragen init".into());
                case.absent.push("is a symlink".into());
            }
            "existing-missing-directory" | "existing-missing-manifest" | "existing-decode" => {
                case.current("name: app\ndependencies: [../stored]\n");
                if label != "existing-missing-directory" {
                    case.layer("stored", "stored");
                }
                if label == "existing-missing-manifest" {
                    fs::remove_file(case.root.path().join("stored/layer.yaml")).unwrap();
                    case.facts.push(
                        "No layer manifest found for existing dependency '../stored'.".into(),
                    );
                } else {
                    case.facts.push("Existing dependency '../stored'".into());
                    if label == "existing-decode" {
                        fs::write(case.root.path().join("stored/layer.yaml"), "[").unwrap();
                        case.facts.push(
                            "Could not decode yaml layer manifest '../stored/layer.yaml'.".into(),
                        );
                    } else {
                        case.facts
                            .push("Could not resolve local directory '../stored'.".into());
                    }
                    case.absent.push("hydragen init".into());
                }
                case.absent
                    .extend(strings(&["Source '../rules'", "(declaration", "(argument"]));
            }
            "existing-published-syntax" => {
                case.current("name: app\ndependencies: ['rules@latest']\n");
                case.facts.extend(strings(&[
                    "Invalid dependency reference in layer manifest",
                    "Field: /dependencies/0",
                    "published reference with an exact version",
                    "@scope/shared-config@1.2.3",
                ]));
                case.absent.push("Schema rule:".into());
                case.absent
                    .extend(strings(&["Source '../rules'", "hydragen init", "--name"]));
            }
            "existing-published-conflict"
            | "existing-local-conflict"
            | "nested-conflict-previous-base" => {
                if label == "existing-published-conflict" {
                    case.current("name: app\ndependencies: ['rules@1.2.3']\n");
                    case.facts
                        .push("Conflicting source: Existing dependency 'rules@1.2.3'".into());
                    case.absent.push("Existing source path:".into());
                } else {
                    case.layer("stored", "rules");
                    case.current("name: app\ndependencies: [../stored]\n");
                    case.facts
                        .push("Conflicting source: Existing dependency '../stored'".into());
                    case.facts.push(format!(
                        "Existing source path: {}",
                        fs::canonicalize(case.root.path().join("stored"))
                            .unwrap()
                            .display()
                    ));
                }
                if label == "nested-conflict-previous-base" {
                    case.nested();
                    case.arguments[1] = fs::canonicalize(case.root.path().join("rules"))
                        .unwrap()
                        .display()
                        .to_string();
                }
                case.facts.extend(strings(&[
                    "Layer name 'rules' is already in use.",
                    "existing declarations are not replaced",
                ]));
                case.facts.push(format!(
                    "Requested source path: {}",
                    fs::canonicalize(case.root.path().join("rules"))
                        .unwrap()
                        .display()
                ));
                case.absent.extend(strings(&[
                    "hydragen upgrade",
                    "hydragen init",
                    "(declaration",
                    "(argument",
                ]));
            }
            "dependency-published-syntax" => {
                fs::write(
                    case.root.path().join("rules/layer.yaml"),
                    "name: rules\ndependencies: ['shared@latest']\n",
                )
                .unwrap();
                case.facts.extend(strings(&[
                    "Invalid dependency reference in layer manifest '../rules/layer.yaml'.",
                    "Field: /dependencies/0",
                    "published reference with an exact version",
                ]));
                case.absent
                    .extend(strings(&["Schema rule:", "hydragen init"]));
                case.absent.push(case.root.path().display().to_string());
            }
            "existing-duplicates" => {
                case.current("name: app\ndependencies: ['rules@1.2.3', 'rules@2.0.0']\n");
                case.arguments[1] = "../other".into();
                case.layer("other", "other");
                case.facts.extend(strings(&[
                    "Existing dependency 'rules@2.0.0'",
                    "Conflicting source: Existing dependency 'rules@1.2.3'",
                    "existing declarations are not replaced",
                ]));
                case.absent.push("Source '../other'".into());
            }
            "existing-identical-duplicates" | "existing-alias-duplicates" => {
                let second = if label == "existing-identical-duplicates" {
                    "../rules"
                } else {
                    "../rules/."
                };
                case.current(&format!(
                    "name: app\ndependencies: [../rules, '{second}']\n"
                ));
                case.layer("other", "other");
                case.arguments[1] = "../other".into();
                case.facts.extend(strings(&["Layer name 'rules' is already in use.", "Remove the redundant existing dependency declaration from the current layer manifest"]));
                let identity = fs::canonicalize(case.root.path().join("rules")).unwrap();
                case.facts
                    .push("Both declarations refer to the same layer directory.".into());
                case.absent.push(identity.display().to_string());
                case.absent
                    .extend(strings(&["Existing source path:", "Declared source path:"]));
                if label == "existing-identical-duplicates" {
                    case.facts.extend(strings(&[
                        "Existing dependency '../rules' (declaration 2)",
                        "Conflicting source: Existing dependency '../rules' (declaration 1)",
                    ]));
                } else {
                    case.facts.extend(strings(&[
                        "Existing dependency '../rules/.'",
                        "Conflicting source: Existing dependency '../rules'",
                    ]));
                    case.absent.push("(declaration".into());
                }
                case.absent.extend(strings(&[
                    "Choose a source",
                    "Source '../other'",
                    "Requested source path:",
                ]));
            }
            "existing-self-conflict" => {
                case.current("name: app\ndependencies: [app@1.2.3]\n");
                case.facts.extend(strings(&["Existing dependency 'app@1.2.3'", "Conflicting source: Current layer", "Remove or correct the existing dependency declaration that uses the current layer's reserved name"]));
                case.facts.push(format!(
                    "Current layer path: {}",
                    fs::canonicalize(&case.app).unwrap().display()
                ));
                case.absent.extend(strings(&[
                    "Choose a source",
                    "Source '../rules'",
                    "Requested source path:",
                ]));
            }
            "existing-distinct-directory-conflict" => {
                case.layer("stored", "rules");
                case.current("name: app\ndependencies: [../stored, ../rules]\n");
                case.layer("other", "other");
                case.arguments[1] = "../other".into();
                case.facts.extend(strings(&[
                    "Layer name 'rules' is already in use.",
                    "Existing dependency '../rules'",
                    "Conflicting source: Existing dependency '../stored'",
                    "Remove or correct the conflicting existing dependency declarations",
                ]));
                case.facts.push(format!(
                    "Existing source path: {}",
                    fs::canonicalize(case.root.path().join("stored"))
                        .unwrap()
                        .display()
                ));
                case.facts.push(format!(
                    "Declared source path: {}",
                    fs::canonicalize(case.root.path().join("rules"))
                        .unwrap()
                        .display()
                ));
                case.absent.extend(strings(&[
                    "Choose a source",
                    "Source '../other'",
                    "Requested source path:",
                    "(declaration",
                    "redundant existing dependency declaration",
                ]));
            }
            "self-conflict" => {
                case.layer("self", "app");
                case.arguments[1] = "../self".into();
                case.facts.extend(strings(&[
                    "Layer name 'app' is already in use.",
                    "Source '../self'",
                    "Conflicting source: Current layer",
                ]));
                case.facts.push(format!(
                    "Current layer path: {}",
                    fs::canonicalize(&case.app).unwrap().display()
                ));
                case.facts.push(format!(
                    "Requested source path: {}",
                    fs::canonicalize(case.root.path().join("self"))
                        .unwrap()
                        .display()
                ));
            }
            "mixed-existing-and-arguments" => {
                case.current("name: app\ndependencies: [../missing, 'reserved@1.2.3']\n");
                case.layer("reserved", "reserved");
                case.arguments =
                    strings(&["add", "../rules", "../reserved", "bare-name", "../gone"]);
                case.labels = strings(&[
                    "Existing dependency '../missing'",
                    "Source '../reserved'",
                    "Source 'bare-name'",
                    "Source '../gone'",
                ]);
                case.facts.extend(strings(&[
                    "Conflicting source: Existing dependency 'reserved@1.2.3'",
                    "Invalid source reference.",
                ]));
            }
            "repeated-ambiguous" => {
                case.current("name: app\ndependencies: [../missing, ../missing]\n");
                case.arguments = strings(&["add", "../missing", "bare-name", "../missing"]);
                case.labels = strings(&[
                    "Existing dependency '../missing' (declaration 1)",
                    "Existing dependency '../missing' (declaration 2)",
                    "Source '../missing' (argument 1)",
                    "Source 'bare-name'",
                    "Source '../missing' (argument 3)",
                ]);
                case.facts.push("Invalid source reference.".into());
            }
            "escaped-label-collision" => {
                case.arguments = strings(&["add", "../raw\npath", r"../raw\npath"]);
                case.labels = strings(&[
                    r"Source '../raw\npath' (argument 1)",
                    r"Source '../raw\npath' (argument 2)",
                ]);
                case.facts
                    .push(r"Could not resolve local directory '../raw\npath'.".into());
            }
            _ => panic!("unknown diagnostic case {label}"),
        }
        // Selected source spelling is retained; errors must never change any
        // readable fixture bytes, inode or mtime, or leave a staging sibling.
        #[cfg(unix)]
        if let Some(path) = &case.unreadable {
            match super::permission_support::deny_read(path)? {
                Setup::Ready(guard) => case.permission_guard = Some(guard),
                Setup::Unsupported(reason) => return Ok(Setup::Unsupported(reason)),
            }
        }
        case.snapshot(case.root.path().to_path_buf());
        Ok(Setup::Ready(case))
    }

    fn layer(&self, directory: &str, name: &str) {
        let path = self.root.path().join(directory);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("layer.yaml"),
            format!("name: {}\n", serde_json::to_string(name).unwrap()),
        )
        .unwrap();
    }

    fn current(&self, source: &str) {
        fs::write(self.app.join("layer.yaml"), source).unwrap();
    }

    fn nested(&mut self) {
        self.invocation = self.app.join("src/deep");
        fs::create_dir_all(&self.invocation).unwrap();
        self.base = true;
    }

    fn snapshot(&mut self, path: PathBuf) {
        let metadata = fs::symlink_metadata(&path).unwrap();
        self.preserved.push(Snapshot {
            path: path.clone(),
            bytes: (metadata.is_file() && self.unreadable.as_ref() != Some(&path))
                .then(|| fs::read(&path).unwrap()),
            modified: metadata.modified().unwrap(),
            len: metadata.len(),
            link: metadata
                .file_type()
                .is_symlink()
                .then(|| fs::read_link(&path).unwrap()),
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
        });
        if metadata.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                self.snapshot(entry.unwrap().path());
            }
        }
    }

    pub fn assert_diagnostic(&self, text: &str) {
        let failures = self.labels.len().max(1);
        let labels = self.labels.iter().map(String::as_str).collect::<Vec<_>>();
        super::diagnostic_body_support::assert_verification(text, failures, &labels);
        let same_identity = matches!(
            self.label,
            "existing-identical-duplicates" | "existing-alias-duplicates"
        );
        assert_eq!(
            text.matches("Both declarations refer to the same layer directory.")
                .count(),
            usize::from(same_identity),
            "{} must state shared identity exactly once, and only when proven: {text}",
            self.label
        );
        if same_identity {
            let references = if self.label == "existing-identical-duplicates" {
                "Existing dependency '../rules' (declaration 2)\n  Conflicting source: Existing dependency '../rules' (declaration 1)"
            } else {
                "Existing dependency '../rules/.'\n  Conflicting source: Existing dependency '../rules'"
            };
            assert!(
                text.contains(&format!(
                    "\n  {references}\n  Both declarations refer to the same layer directory.\n"
                )),
                "{} must retain declaration roles, positions and order: {text}",
                self.label
            );
        }
        for fact in &self.facts {
            assert!(text.contains(fact), "{} lost {fact:?}: {text}", self.label);
        }
        for absent in &self.absent {
            assert!(
                !text.contains(absent),
                "{} unwanted {absent:?}: {text}",
                self.label
            );
        }
        assert_eq!(
            text.contains("Paths are relative to '"),
            self.base,
            "{} base: {text}",
            self.label
        );
        if self.base {
            assert!(
                text.contains(&format!(
                    "Paths are relative to '{}'.",
                    fs::canonicalize(&self.app).unwrap().display()
                )),
                "{text}"
            );
        }
        if let Some(exact) = &self.exact {
            assert_eq!(
                text.strip_prefix("✕ ").unwrap_or(text),
                exact,
                "approved direct transcript"
            );
        }
        self.assert_unchanged();
    }

    pub fn assert_unchanged(&self) {
        for before in &self.preserved {
            let metadata = fs::symlink_metadata(&before.path).unwrap();
            assert_eq!(
                metadata.len(),
                before.len,
                "{} changed size: {:?}",
                self.label,
                before.path
            );
            assert_eq!(
                metadata.modified().unwrap(),
                before.modified,
                "{} changed mtime: {:?}",
                self.label,
                before.path
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::{MetadataExt, PermissionsExt};
                assert_eq!(metadata.ino(), before.inode);
                assert_eq!(metadata.permissions().mode(), before.mode);
            }
            if let Some(bytes) = &before.bytes {
                assert_eq!(fs::read(&before.path).unwrap(), *bytes);
            }
            if let Some(link) = &before.link {
                assert_eq!(fs::read_link(&before.path).unwrap(), *link);
            }
            #[cfg(unix)]
            if self.unreadable.as_ref() == Some(&before.path) {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(metadata.permissions().mode() & 0o777, 0);
            }
            if metadata.is_dir() {
                let expected = self
                    .preserved
                    .iter()
                    .filter(|entry| entry.path.parent() == Some(before.path.as_path()))
                    .count();
                assert_eq!(
                    fs::read_dir(&before.path).unwrap().count(),
                    expected,
                    "{} unexpected files/staging in {:?}",
                    self.label,
                    before.path
                );
            }
        }
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}
