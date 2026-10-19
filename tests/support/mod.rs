use std::io;
use std::path::{Path, PathBuf};

/// Own an isolated directory whose final component is a valid layer-name default.
/// tempfile's generated `.tmp...` name is intentionally not a valid layer name.
pub struct TestDirectory {
    _parent: tempfile::TempDir,
    path: PathBuf,
}

impl TestDirectory {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn tempdir() -> io::Result<TestDirectory> {
    let parent = tempfile::tempdir()?;
    let path = parent.path().join("project");
    std::fs::create_dir(&path)?;
    Ok(TestDirectory {
        _parent: parent,
        path,
    })
}
