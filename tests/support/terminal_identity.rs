use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use portable_pty::CommandBuilder;
use serde_json::Value;
use tempfile::{TempDir, tempdir};

pub const DEFAULT_AUTHOR: &str = "Terminal fixture author";

// This directory is deliberately a sibling fixture, never inside the init target.
// Clearing the child environment is essential: removing a few known token names
// alone would still expose real credentials, Git includes, and account settings.
pub struct IdentityFixture {
    directory: TempDir,
}

impl IdentityFixture {
    pub fn new() -> Self {
        let fixture = Self {
            directory: tempdir().unwrap(),
        };
        for directory in ["bin", "home", "xdg", "gh"] {
            fs::create_dir(fixture.path().join(directory)).unwrap();
        }
        fs::write(fixture.path().join("gitconfig"), "").unwrap();
        fixture.executable("git", "exit 1\n");
        fixture.executable("gh", "exit 1\n");
        fixture
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    fn executable(&self, name: &str, body: &str) {
        let path = self.path().join("bin").join(name);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >>\"$HOME/../{name}-calls\"\n/usr/bin/env >\"$HOME/../{name}-environment\"\n{body}"
            ),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    pub fn github(&self, profile: Value, organisations: Value) {
        fs::write(self.path().join("profile.json"), profile.to_string()).unwrap();
        fs::write(
            self.path().join("organisations.json"),
            organisations.to_string(),
        )
        .unwrap();
        self.executable(
            "gh",
            "case \"$*\" in\n*graphql*) /bin/cat \"$HOME/../organisations.json\";;\n*user*) /bin/cat \"$HOME/../profile.json\";;\n*) exit 1;;\nesac\n",
        );
    }

    pub fn without_tools(&self) {
        for name in ["git", "gh"] {
            fs::remove_file(self.path().join("bin").join(name)).unwrap();
        }
    }

    pub fn apply(&self, command: &mut CommandBuilder) {
        self.apply_with_profile(command, std::env::var_os("LLVM_PROFILE_FILE"));
    }

    fn apply_with_profile(&self, command: &mut CommandBuilder, profile: Option<OsString>) {
        command.env_clear();
        // Keep the instrumented child's counters without inheriting identity or
        // config. LLVM, not this fixture, interprets placeholders such as %p/%m.
        if let Some(profile) = profile {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        command.env("PATH", self.path().join("bin"));
        command.env("HOME", self.path().join("home"));
        command.env("XDG_CONFIG_HOME", self.path().join("xdg"));
        command.env("GH_CONFIG_DIR", self.path().join("gh"));
        command.env("GIT_CONFIG_NOSYSTEM", "1");
        command.env("GIT_CONFIG_GLOBAL", self.path().join("gitconfig"));
        command.env("GIT_AUTHOR_NAME", DEFAULT_AUTHOR);
        command.env("LANG", "en_US.UTF-8");
        command.env("LC_ALL", "en_US.UTF-8");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
    use std::process::Command;
    use std::time::{Duration, Instant};

    #[test]
    fn profile_forwarding_is_verbatim_and_keeps_terminal_identity_isolated() {
        let fixture = IdentityFixture::new();
        for profile in [
            Some(OsString::from_vec(
                b"/profiles/raw-\xff-%p-%m.profraw".to_vec(),
            )),
            Some(OsString::new()),
            None,
        ] {
            let mut command = CommandBuilder::new("unused");
            command.env("GH_TOKEN", "inherited-private-token");
            command.env("GITHUB_TOKEN", "inherited-private-token");
            command.env("GH_CONFIG_DIR", "/private/gh-config");
            command.env("GIT_CONFIG_GLOBAL", "/private/git-config");
            command.env("GIT_CONFIG_COUNT", "1");
            command.env("LLVM_PROFILE_FILE", "inherited-profile-must-be-cleared");
            fixture.apply_with_profile(&mut command, profile.clone());
            assert_eq!(command.get_env("LLVM_PROFILE_FILE"), profile.as_deref());
            for key in ["GH_TOKEN", "GITHUB_TOKEN", "GIT_CONFIG_COUNT"] {
                assert_eq!(command.get_env(key), None, "{key}");
            }
            assert_eq!(
                command.get_env("GH_CONFIG_DIR"),
                Some(fixture.path().join("gh").as_os_str())
            );
            assert_eq!(
                command.get_env("GIT_CONFIG_GLOBAL"),
                Some(fixture.path().join("gitconfig").as_os_str())
            );
            assert_eq!(
                command.get_env("PATH"),
                Some(fixture.path().join("bin").as_os_str())
            );
        }
    }

    #[test]
    fn instrumented_terminal_child_writes_forwarded_profile_outside_init_target() {
        let fixture = IdentityFixture::new();
        let target = fixture.path().join("init-target");
        let profiles = fixture.path().join("profiles");
        fs::create_dir(&target).unwrap();
        fs::create_dir(&profiles).unwrap();
        let source = fixture.path().join("profile_probe.rs");
        let executable = fixture.path().join("profile-probe");
        fs::write(
            &source,
            r#"fn main() {
    assert_eq!(std::env::var_os("LLVM_PROFILE_FILE"), std::env::args_os().nth(1));
    assert!(std::env::var_os("GH_TOKEN").is_none());
    assert!(std::env::var_os("GITHUB_TOKEN").is_none());
    assert!(std::env::var_os("GIT_CONFIG_COUNT").is_none());
}"#,
        )
        .unwrap();
        let compiled = Command::new("rustc")
            .args(["-C", "instrument-coverage"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(compiled.status.success(), "{compiled:?}");
        let profile = profiles.join("terminal-%p-%m.profraw").into_os_string();
        let mut command = CommandBuilder::new(&executable);
        command.arg(&profile);
        command.cwd(&target);
        command.env("GH_TOKEN", "inherited-private-token");
        command.env("GITHUB_TOKEN", "inherited-private-token");
        command.env("GIT_CONFIG_COUNT", "1");
        fixture.apply_with_profile(&mut command, Some(profile.clone()));
        assert_eq!(
            command.get_env("LLVM_PROFILE_FILE"),
            Some(profile.as_os_str())
        );
        let _session = crate::TERMINAL_SESSION
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let terminal = portable_pty::native_pty_system()
            .openpty(portable_pty::PtySize::default())
            .unwrap();
        let mut child = terminal.slave.spawn_command(command).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("instrumented terminal child did not exit");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(status.success(), "{status:?}");
        let files: Vec<_> = fs::read_dir(&profiles).unwrap().collect();
        assert_eq!(files.len(), 1);
        let output = files[0].as_ref().unwrap();
        let filename = output.file_name().into_string().unwrap();
        assert!(filename.starts_with("terminal-") && filename.ends_with(".profraw"));
        assert!(!filename.contains('%'), "{filename}");
        let bytes = output.metadata().unwrap().len();
        assert!(bytes > 0);
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        println!(
            "Terminal forwarding proof: {bytes} profile bytes outside init target; no default profile in target"
        );
    }
}
