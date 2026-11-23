use std::cell::RefCell;
use std::collections::HashMap;
use std::future::{Future, ready};
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Revision(u64);

impl Revision {
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFile {
    name: String,
    revision: Revision,
    bytes: Vec<u8>,
}

impl DriveFile {
    #[must_use]
    pub fn new(name: impl Into<String>, revision: Revision, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            revision,
            bytes,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn revision(&self) -> Revision {
        self.revision
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWrite {
    pub name: String,
    pub bytes: Vec<u8>,
    pub seen: Option<Revision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveError {
    Conflict,
    Unavailable,
}

pub trait Drive {
    fn load(&self) -> impl Future<Output = Result<Vec<DriveFile>, DriveError>>;

    /// Пишет все файлы или ни одного. Чужая `seen`-версия — [`DriveError::Conflict`].
    fn write(&mut self, files: Vec<FileWrite>) -> impl Future<Output = Result<(), DriveError>>;
}

#[derive(Debug, Clone)]
struct StoredFile {
    revision: Revision,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MemoryDrive {
    inner: Rc<RefCell<HashMap<String, StoredFile>>>,
}

impl MemoryDrive {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDrive {
    fn default() -> Self {
        Self::new()
    }
}

impl Drive for MemoryDrive {
    fn load(&self) -> impl Future<Output = Result<Vec<DriveFile>, DriveError>> {
        let files = self
            .inner
            .borrow()
            .iter()
            .map(|(name, file)| DriveFile::new(name.clone(), file.revision, file.bytes.clone()))
            .collect();
        ready(Ok(files))
    }

    fn write(&mut self, files: Vec<FileWrite>) -> impl Future<Output = Result<(), DriveError>> {
        ready(self.write_now(files))
    }
}

impl MemoryDrive {
    fn write_now(&self, files: Vec<FileWrite>) -> Result<(), DriveError> {
        let mut inner = self.inner.borrow_mut();
        if files.iter().any(|file| !seen_matches(&inner, file)) {
            return Err(DriveError::Conflict);
        }
        for file in files {
            store(&mut inner, file);
        }
        Ok(())
    }
}

fn seen_matches(files: &HashMap<String, StoredFile>, write: &FileWrite) -> bool {
    files.get(&write.name).map(|file| file.revision) == write.seen
}

fn store(files: &mut HashMap<String, StoredFile>, write: FileWrite) {
    let revision = next_revision(write.seen);
    files.insert(
        write.name,
        StoredFile {
            revision,
            bytes: write.bytes,
        },
    );
}

fn next_revision(seen: Option<Revision>) -> Revision {
    match seen {
        Some(revision) => Revision(revision.as_u64().saturating_add(1)),
        None => Revision::from_u64(1),
    }
}
