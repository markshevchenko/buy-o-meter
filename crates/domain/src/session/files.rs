use super::{DATA_FILES, DriveError, DriveFile, FileWrite, Revision, SessionError};
use crate::ledger::{Ledger, LedgerFile, StoreError, decode_ledger, encode_ledger};

pub(super) type Seen = [Option<Revision>; DATA_FILES.len()];

pub(super) const fn empty_seen() -> Seen {
    [None; DATA_FILES.len()]
}

pub(super) fn has_data(files: &[DriveFile]) -> bool {
    files.iter().any(|file| data_index(file.name()).is_some())
}

pub(super) fn remote_is_newer(files: &[DriveFile], seen: &Seen) -> bool {
    files.iter().any(|file| newer_than_seen(file, seen))
}

pub(super) fn seen_from(files: &[DriveFile]) -> Seen {
    let mut seen = empty_seen();
    for file in files {
        if let Some(index) = data_index(file.name()) {
            seen[index] = Some(file.revision());
        }
    }
    seen
}

pub(super) fn advance(seen: &mut Seen) {
    for slot in seen {
        *slot = Some(next_revision(*slot));
    }
}

pub(super) fn writes(ledger: &Ledger, seen: &Seen) -> Result<Vec<FileWrite>, SessionError> {
    encode_ledger(ledger)
        .map_err(store_error)
        .map(|files| files.into_iter().map(|file| write(file, seen)).collect())
}

pub(super) fn decode(files: &[DriveFile]) -> Result<Ledger, SessionError> {
    let views: Vec<_> = files
        .iter()
        .map(|file| (file.name(), file.bytes()))
        .collect();
    decode_ledger(&views).map_err(store_error)
}

fn newer_than_seen(file: &DriveFile, seen: &Seen) -> bool {
    let Some(index) = data_index(file.name()) else {
        return false;
    };
    match seen[index] {
        Some(revision) => file.revision() > revision,
        None => true,
    }
}

fn write(file: LedgerFile, seen: &Seen) -> FileWrite {
    FileWrite {
        seen: data_index(file.name).and_then(|index| seen[index]),
        name: file.name.to_owned(),
        bytes: file.bytes,
    }
}

fn next_revision(seen: Option<Revision>) -> Revision {
    match seen {
        Some(revision) => Revision::from_u64(revision.as_u64().saturating_add(1)),
        None => Revision::from_u64(1),
    }
}

fn data_index(name: &str) -> Option<usize> {
    DATA_FILES.iter().position(|file| *file == name)
}

fn store_error(_: StoreError) -> SessionError {
    SessionError::Drive(DriveError::Unavailable)
}
