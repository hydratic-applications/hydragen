//! Read directly declared project authors as data, never through project tools.

use std::fs::{File, OpenOptions};
use std::io::Read;
use std::path::Path;

use hydragen::layer_manifest::schema;

use super::{Context, ProjectFile, Source};

const MAX_FILE_BYTES: u64 = 1024 * 1024;

pub(super) fn collect(directory: &Path, context: &Context) -> Result<(), schema::Error> {
    collect_with_reader(directory, context, read_project_file)
}

fn collect_with_reader(
    directory: &Path,
    context: &Context,
    mut read: impl FnMut(&Path) -> Option<String>,
) -> Result<(), schema::Error> {
    for (filename, file) in [
        ("package.json", ProjectFile::PackageJson),
        ("Cargo.toml", ProjectFile::CargoToml),
        ("pyproject.toml", ProjectFile::PyprojectToml),
    ] {
        if !context.active() {
            break;
        }
        let Some(contents) = read(&directory.join(filename)) else {
            continue;
        };
        let active = match file {
            ProjectFile::PackageJson => collect_package(&contents, context)?,
            ProjectFile::CargoToml => collect_toml(&contents, context, false)?,
            ProjectFile::PyprojectToml => collect_toml(&contents, context, true)?,
        };
        if !active {
            break;
        }
    }
    Ok(())
}

fn read_project_file(path: &Path) -> Option<String> {
    let file = open_regular_file(path)?;
    read_bounded(file)
}

fn open_regular_file(path: &Path) -> Option<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // NOFOLLOW closes the final-entry symlink race; NONBLOCK avoids waiting
        // for a FIFO writer before we can check the opened handle's file type.
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        const SECURITY_IDENTIFICATION: u32 = 0x0001_0000;
        options
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .security_qos_flags(SECURITY_IDENTIFICATION);
    }
    // Do not silently offer weaker symlink safety on unsupported platforms.
    #[cfg(not(any(unix, windows)))]
    return None;

    let file = options.open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return None;
        }
    }
    Some(file)
}

fn read_bounded(reader: impl Read) -> Option<String> {
    // Metadata is only an early rejection: the file may grow after opening.
    // Read one detection byte, but never allocate/capture an unbounded file.
    let mut bytes = Vec::new();
    reader
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn collect_package(contents: &str, context: &Context) -> Result<bool, schema::Error> {
    let Ok(document) = serde_json::from_str::<serde_json::Value>(contents) else {
        return Ok(true);
    };
    let Some(author) = document.get("author") else {
        return Ok(true);
    };
    let name = match author {
        serde_json::Value::Object(author) => author.get("name").and_then(|name| name.as_str()),
        serde_json::Value::String(author) => person_name(author, true),
        _ => None,
    };
    match name {
        Some(name) => context.publish(
            name,
            Source::Project {
                file: ProjectFile::PackageJson,
                index: 0,
            },
        ),
        None => Ok(true),
    }
}

fn collect_toml(contents: &str, context: &Context, python: bool) -> Result<bool, schema::Error> {
    let Ok(document) = toml::from_str::<toml::Value>(contents) else {
        return Ok(true);
    };
    let section = if python { "project" } else { "package" };
    let Some(authors) = document
        .get(section)
        .and_then(|section| section.get("authors"))
        .and_then(|authors| authors.as_array())
    else {
        return Ok(true);
    };
    for (index, author) in authors.iter().enumerate() {
        let name = if python {
            author
                .as_table()
                .and_then(|author| author.get("name"))
                .and_then(|name| name.as_str())
        } else {
            author
                .as_str()
                .and_then(|author| person_name(author, false))
        };
        let Some(name) = name else { continue };
        let file = if python {
            ProjectFile::PyprojectToml
        } else {
            ProjectFile::CargoToml
        };
        if !context.publish(name, Source::Project { file, index })? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Parse only unambiguous person shorthand, not explicit object/table names.
/// Cargo permits a trailing email; npm also permits a following URL component.
fn person_name(person: &str, npm: bool) -> Option<&str> {
    let person = person.trim();
    let contact = person.find(|character| character == '<' || (npm && character == '('));
    let (name, mut suffix) = match contact {
        Some(offset) => (&person[..offset], &person[offset..]),
        None => (person, ""),
    };
    // Bare addresses/URLs and broken delimiters are not declared names. Ordinary
    // punctuation (including Cargo names containing parentheses) stays literal.
    if name.trim().is_empty()
        || name.contains(['@', '<', '>'])
        || name.contains("://")
        || (npm && name.contains(['(', ')']))
    {
        return None;
    }
    if let Some(email) = suffix.strip_prefix('<') {
        let end = email.find('>')?;
        let address = &email[..end];
        let (local, domain) = address.split_once('@')?;
        if local.is_empty()
            || domain.is_empty()
            || domain.contains('@')
            || address.contains(['<', '>', '(', ')'])
            || address.chars().any(char::is_whitespace)
        {
            return None;
        }
        suffix = email[end + 1..].trim();
    }
    if npm && let Some(url) = suffix.strip_prefix('(') {
        let url = url.strip_suffix(')')?;
        // A plain parenthetical such as "Alex (team)" is ambiguous, unlike an
        // explicit URL. Require a URI scheme, without inventing network access.
        let (scheme, location) = url.split_once(':')?;
        if !scheme.starts_with(|character: char| character.is_ascii_alphabetic())
            || !scheme
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "+-.".contains(character))
            || location.is_empty()
            || url.contains(['<', '>', '(', ')'])
            || url.chars().any(char::is_whitespace)
        {
            return None;
        }
        suffix = "";
    }
    suffix.is_empty().then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{self, Cursor};
    use tempfile::TempDir;

    const FILES: [&str; 3] = ["package.json", "Cargo.toml", "pyproject.toml"];

    fn fixture(name: &str) -> TempDir {
        let directory = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/author-discovery/projects")
            .join(name);
        for filename in FILES {
            let path = source.join(filename);
            if path.is_file() {
                fs::copy(path, directory.path().join(filename)).unwrap();
            }
        }
        directory
    }

    fn choices(directory: &Path) -> Vec<super::super::Choice> {
        let context = Context::for_tests();
        collect(directory, &context).unwrap();
        context.choices_for_tests().unwrap()
    }

    fn names(directory: &Path) -> Vec<String> {
        choices(directory)
            .into_iter()
            .map(|choice| choice.name)
            .collect()
    }

    #[test]
    fn closed_context_collects_no_records_and_does_not_open_project_files() {
        let directory = fixture("conflicts");
        let context = Context::for_tests();
        context.close_for_tests();
        collect(directory.path(), &context).unwrap();
        collect_with_reader(directory.path(), &context, |_| {
            panic!("closed collection must not open a project file")
        })
        .unwrap();
        assert!(context.choices_for_tests().unwrap().is_empty());
    }

    #[test]
    fn closure_during_missing_or_malformed_file_prevents_the_next_open() {
        for first_result in [None, Some("malformed JSON".to_owned())] {
            let directory = fixture("conflicts");
            let context = Context::for_tests();
            let mut opens = Vec::new();
            collect_with_reader(directory.path(), &context, |path| {
                opens.push(path.file_name().unwrap().to_owned());
                context.close_for_tests();
                first_result.clone()
            })
            .unwrap();
            assert_eq!(opens, [std::ffi::OsString::from("package.json")]);
            assert!(context.choices_for_tests().unwrap().is_empty());
        }
    }

    #[test]
    fn npm_person_string_extracts_only_name() {
        let directory = fixture("npm-string");
        assert_eq!(names(directory.path()), ["Zoë O’Connor & Co., 水 🦀"]);
        let choices = choices(directory.path());
        assert_eq!(choices[0].hint, "package.json author");
        assert!(choices[0].eligible);
    }

    #[test]
    fn explicit_names_are_not_parsed_as_person_shorthand() {
        let directory = fixture("literal-names");
        assert_eq!(
            names(directory.path()),
            [
                "Example (Research) <literal> @team",
                "Python (Research) <literal> @team"
            ]
        );
    }

    #[test]
    fn supported_shorthand_contacts_and_punctuation() {
        for (person, npm, expected) in [
            ("Alex Morgan", true, "Alex Morgan"),
            ("Alex Morgan <alex@example.com>", true, "Alex Morgan "),
            ("Alex Morgan (https://example.com)", true, "Alex Morgan "),
            (
                " Alex Morgan <alex@example.com> (https://example.com/path) ",
                true,
                "Alex Morgan ",
            ),
            ("Alex<alex@example.com>(https://example.com)", true, "Alex"),
            ("A. O’Connor, Jr. & 水-🦀", true, "A. O’Connor, Jr. & 水-🦀"),
            ("Example (Research)", false, "Example (Research)"),
            ("Alex <alex@example.com>", false, "Alex "),
        ] {
            assert_eq!(person_name(person, npm), Some(expected), "{person:?}");
        }
    }

    #[test]
    fn email_only_and_ambiguous_shorthand_is_not_guessed() {
        for person in [
            "",
            "   ",
            "alex@example.com",
            "<alex@example.com>",
            "Alex alex@example.com",
            "https://example.com",
            "Alex <alex@example.com",
            "Alex alex@example.com>",
            "Alex <>",
            "Alex <alex>",
            "Alex <@example.com>",
            "Alex <alex@>",
            "Alex <alex@@example.com>",
            "Alex <alex @example.com>",
            "Alex <<alex@example.com>>",
            "Alex <alex@example.com> trailing",
            "Alex <alex@example.com> <other@example.com>",
        ] {
            for npm in [true, false] {
                assert_eq!(person_name(person, npm), None, "{person:?}, npm={npm}");
            }
        }
        for person in [
            "(https://example.com)",
            "Alex (team)",
            "Alex (https://example.com",
            "Alex https://example.com)",
            "Alex (https://example.com) trailing",
            "Alex (https://example.com) <alex@example.com>",
            "Alex (https://example.com) (https://other.example)",
            "Alex (https://example.com/a(b))",
            "Alex (https://example.com/with space)",
            "Alex (1http:example)",
            "Alex (:example)",
            "Alex (https:)",
        ] {
            assert_eq!(person_name(person, true), None, "{person:?}");
        }
        assert_eq!(
            person_name("Alex <alex@example.com> (https://example.com)", false),
            None
        );
    }

    #[test]
    fn invalid_array_entries_are_independent_and_central_validation_is_used() {
        let directory = fixture("arrays");
        let choices = choices(directory.path());
        assert_eq!(
            choices
                .iter()
                .map(|choice| choice.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Cargo First",
                "Cargo (Research)",
                "Python First",
                "Python Last"
            ]
        );
        assert!(choices.iter().all(|choice| choice.eligible));
        assert_eq!(choices[0].hint, "Cargo.toml package.authors");
        assert_eq!(choices[2].hint, "pyproject.toml project.authors");
    }

    #[test]
    fn all_files_keep_stable_order_group_sources_and_credit_organisations() {
        let directory = fixture("conflicts");
        let original = FILES.map(|filename| fs::read(directory.path().join(filename)).unwrap());
        for _ in 0..3 {
            let choices = choices(directory.path());
            assert_eq!(
                choices
                    .iter()
                    .map(|choice| choice.name.as_str())
                    .collect::<Vec<_>>(),
                [
                    "Example organisation",
                    "Cargo Person",
                    "Python Person",
                    "example organisation"
                ]
            );
            assert_eq!(
                choices[0].hint,
                "package.json author, Cargo.toml package.authors, and pyproject.toml project.authors"
            );
            assert!(choices.iter().all(|choice| choice.eligible));
        }
        for (filename, original) in FILES.into_iter().zip(original) {
            assert_eq!(fs::read(directory.path().join(filename)).unwrap(), original);
        }
    }

    #[test]
    fn inheritance_and_unrelated_fields_are_not_sources() {
        let directory = fixture("unsupported");
        assert!(names(directory.path()).is_empty());
    }

    #[test]
    fn directly_listed_python_authors_survive_dynamic_declaration() {
        let directory = fixture("python-dynamic");
        assert_eq!(names(directory.path()), ["Static Python Author"]);
    }

    #[test]
    fn unsupported_npm_field_types_contribute_nothing() {
        let directory = tempfile::tempdir().unwrap();
        for author in [
            serde_json::json!(null),
            serde_json::json!(42),
            serde_json::json!(["Array Name"]),
            serde_json::json!({"email": "email@example.com"}),
            serde_json::json!({"name": 42}),
            serde_json::json!({"name": []}),
            serde_json::json!({"name": " \u{2003} "}),
            serde_json::json!({"name": "Unsafe\nName"}),
        ] {
            fs::write(
                directory.path().join("package.json"),
                serde_json::json!({"author": author}).to_string(),
            )
            .unwrap();
            assert!(names(directory.path()).is_empty(), "{author:?}");
        }
    }

    #[test]
    fn malformed_files_and_invalid_utf8_do_not_erase_other_files() {
        let directory = fixture("malformed");
        assert_eq!(names(directory.path()), ["Still Usable"]);
        let target = directory.path().join("package.json");
        fs::write(&target, b"{\"author\":\"\xff\"}").unwrap();
        assert_eq!(names(directory.path()), ["Still Usable"]);
        assert_eq!(fs::read(&target).unwrap(), b"{\"author\":\"\xff\"}");
        fs::remove_file(&target).unwrap();
        assert_eq!(names(directory.path()), ["Still Usable"]);
    }

    #[test]
    fn malformed_toml_field_shapes_are_skipped_without_ecosystem_validation() {
        let directory = tempfile::tempdir().unwrap();
        for content in [
            "[package]\nauthors = 42\n",
            "[package]\nauthors = 'Not an array'\n",
            "[package.authors]\nworkspace = true\n",
            "package = []\n",
            "[workspace.package]\nauthors = ['Workspace Only']\n",
        ] {
            fs::write(directory.path().join("Cargo.toml"), content).unwrap();
            assert!(names(directory.path()).is_empty(), "{content:?}");
        }
        fs::write(
            directory.path().join("Cargo.toml"),
            "[package]\nauthors = ['No package name or version required']\n",
        )
        .unwrap();
        assert_eq!(
            names(directory.path()),
            ["No package name or version required"]
        );
    }

    #[test]
    fn reads_only_the_init_directory_and_never_executes_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let init = directory.path().join("init");
        fs::create_dir(&init).unwrap();
        for location in [
            directory.path().to_path_buf(),
            init.join("child"),
            init.join("node_modules/dependency"),
        ] {
            fs::create_dir_all(&location).unwrap();
            fs::write(location.join("package.json"), r#"{"author":"Not In Init"}"#).unwrap();
        }
        let script = format!(
            "#!/bin/sh\ntouch '{}'\n",
            directory.path().join("unexpected-execution").display()
        );
        for filename in ["setup.py", "npm", "cargo", "python", "backend.py"] {
            let path = init.join(filename);
            fs::write(&path, &script).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
        fs::write(init.join(".npmrc"), "init-author-name=Not A Source\n").unwrap();
        fs::write(
            init.join("pyproject.toml"),
            "[project]\ndynamic = ['authors']\n[build-system]\nrequires = []\nbuild-backend = 'backend'\nbackend-path = ['.']\n",
        )
        .unwrap();
        assert!(names(&init).is_empty());
        assert!(!directory.path().join("unexpected-execution").exists());
        for filename in ["setup.py", "npm", "cargo", "python", "backend.py"] {
            assert_eq!(fs::read_to_string(init.join(filename)).unwrap(), script);
        }
    }

    #[test]
    fn exact_limit_is_accepted_and_one_extra_byte_rejects_the_whole_file() {
        let directory = fixture("conflicts");
        let mut content = br#"{"author":"At Limit"}"#.to_vec();
        content.resize(MAX_FILE_BYTES as usize, b' ');
        fs::write(directory.path().join("package.json"), &content).unwrap();
        assert_eq!(names(directory.path())[0], "At Limit");
        content.push(b' ');
        fs::write(directory.path().join("package.json"), &content).unwrap();
        assert!(
            !names(directory.path())
                .iter()
                .any(|name| name == "At Limit")
        );
        assert_eq!(
            fs::read(directory.path().join("package.json")).unwrap(),
            content
        );
    }

    #[test]
    fn growing_input_stops_at_one_detection_byte_even_without_metadata_rejection() {
        struct GrowingReader {
            consumed: std::rc::Rc<std::cell::Cell<usize>>,
        }
        impl Read for GrowingReader {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                buffer.fill(b' ');
                self.consumed.set(self.consumed.get() + buffer.len());
                Ok(buffer.len())
            }
        }
        let consumed = std::rc::Rc::new(std::cell::Cell::new(0));
        assert!(
            read_bounded(GrowingReader {
                consumed: consumed.clone()
            })
            .is_none()
        );
        assert_eq!(consumed.get() as u64, MAX_FILE_BYTES + 1);
        assert_eq!(
            read_bounded(Cursor::new(b"valid")),
            Some("valid".to_owned())
        );
        assert!(read_bounded(Cursor::new([0xff])).is_none());
    }

    #[test]
    fn file_growth_after_the_opened_handle_check_is_still_bounded() {
        use std::io::Write;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("package.json");
        fs::write(&path, br#"{"author":"Before Growth"}"#).unwrap();
        let reader = open_regular_file(&path).unwrap();
        let mut writer = OpenOptions::new().append(true).open(&path).unwrap();
        writer
            .write_all(&vec![b' '; MAX_FILE_BYTES as usize])
            .unwrap();
        drop(writer);
        assert!(read_bounded(reader).is_none());
    }

    #[test]
    fn read_errors_are_silent_unavailable_input() {
        struct BrokenReader;
        impl Read for BrokenReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("fixture read failure"))
            }
        }
        assert!(read_bounded(BrokenReader).is_none());
    }

    #[test]
    fn directories_are_not_project_files() {
        let directory = fixture("conflicts");
        fs::remove_file(directory.path().join("package.json")).unwrap();
        fs::create_dir(directory.path().join("package.json")).unwrap();
        assert_eq!(names(directory.path())[0], "Cargo Person");
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_files_do_not_erase_other_sources_or_change_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = fixture("conflicts");
        let path = directory.path().join("package.json");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        // Privileged test runners may still read mode-000 files. Only assert
        // omission when this process actually lacks permission to open them.
        let unreadable = File::open(&path).is_err();
        let discovered = names(directory.path());
        assert!(discovered.iter().any(|name| name == "Cargo Person"));
        if unreadable {
            assert_eq!(discovered[0], "Cargo Person");
        }
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0);
        // Restore permissions on our own fixture for deterministic cleanup.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_dangling_links_and_symlink_swaps_are_not_followed() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("private.json");
        let original = br#"{"author":"Never Follow"}"#;
        fs::write(&target, original).unwrap();
        let path = directory.path().join("package.json");
        symlink(&target, &path).unwrap();
        assert!(names(directory.path()).is_empty());
        fs::remove_file(&path).unwrap();
        symlink(outside.path().join("missing"), &path).unwrap();
        assert!(names(directory.path()).is_empty());
        // A prior metadata observation must not allow a later symlink open.
        fs::remove_file(&path).unwrap();
        fs::write(&path, original).unwrap();
        assert!(fs::symlink_metadata(&path).unwrap().is_file());
        fs::remove_file(&path).unwrap();
        symlink(&target, &path).unwrap();
        assert!(read_project_file(&path).is_none());
        assert_eq!(fs::read(target).unwrap(), original);
    }

    #[cfg(unix)]
    #[test]
    fn fifo_and_socket_are_rejected_without_waiting_for_a_writer() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::net::UnixListener;
        use std::sync::mpsc;
        use std::time::Duration;

        let directory = tempfile::tempdir().unwrap();
        let fifo = directory.path().join("package.json");
        let c_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        // SAFETY: c_path is a valid NUL-terminated pathname owned for this call.
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
        let _listener = UnixListener::bind(directory.path().join("Cargo.toml")).unwrap();
        fs::write(
            directory.path().join("pyproject.toml"),
            "[project]\nauthors = [{ name = 'After Nonregular' }]\n",
        )
        .unwrap();
        let path = directory.path().to_owned();
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || sender.send(names(&path)).unwrap());
        let result = receiver.recv_timeout(Duration::from_secs(2));
        if result.is_err() {
            // Unblock a regressed blocking FIFO open before reporting failure.
            use std::os::unix::fs::OpenOptionsExt;
            if let Ok(mut writer) = OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&fifo)
            {
                use std::io::Write;
                let _ = writer.write_all(b"{}");
            }
        }
        assert_eq!(result.unwrap(), ["After Nonregular"]);
        worker.join().unwrap();
    }
}
