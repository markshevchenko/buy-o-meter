mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::create_account;
use domain::{
    Drive, DriveError, DriveFile, FileWrite, MemoryDrive, MemoryIdentity, Place, Session,
    SessionError,
};

/// Drive, которому можно отключить чтение, не теряя уже записанное.
#[derive(Clone)]
struct FlakyDrive {
    inner: MemoryDrive,
    failure: Rc<Cell<Option<DriveError>>>,
    loads: Rc<Cell<u32>>,
}

impl FlakyDrive {
    fn new(inner: MemoryDrive) -> Self {
        Self {
            inner,
            failure: Rc::new(Cell::new(None)),
            loads: Rc::new(Cell::new(0)),
        }
    }

    fn break_reads(&self, error: DriveError) {
        self.failure.set(Some(error));
    }

    fn repair_reads(&self) {
        self.failure.set(None);
    }

    fn loads(&self) -> u32 {
        self.loads.get()
    }
}

impl Drive for FlakyDrive {
    async fn load(&self) -> Result<Vec<DriveFile>, DriveError> {
        self.loads.set(self.loads.get() + 1);
        match self.failure.get() {
            Some(error) => Err(error),
            None => self.inner.load().await,
        }
    }

    async fn write(&mut self, files: Vec<FileWrite>) -> Result<(), DriveError> {
        match self.failure.get() {
            Some(error) => Err(error),
            None => self.inner.write(files).await,
        }
    }
}

fn signed_in_with_account(drive: FlakyDrive) -> Session<MemoryIdentity, FlakyDrive> {
    let mut session = Session::new(MemoryIdentity::new(), drive);
    pollster::block_on(session.open());
    pollster::block_on(session.sign_in()).expect("вход");
    pollster::block_on(session.change(|ledger| {
        create_account(ledger, "Наличные", "RUB", 1_000)?;
        Ok(())
    }))
    .expect("счёт");
    session
}

fn account_names(session: &Session<MemoryIdentity, FlakyDrive>) -> Vec<String> {
    session
        .ledger()
        .expect("журнал")
        .accounts()
        .iter()
        .map(|account| account.name().to_owned())
        .collect()
}

// 016 «Сбой обновления при возврате к приложению»
#[test]
fn failed_return_keeps_feed_and_reports_the_failure() {
    let drive = FlakyDrive::new(MemoryDrive::new());
    let mut session = signed_in_with_account(drive.clone());
    assert!(session.sync_failure().is_none());

    drive.break_reads(DriveError::Unavailable);
    let error = pollster::block_on(session.on_return()).expect_err("обновление должно отказать");

    assert_eq!(error, SessionError::Drive(DriveError::Unavailable));
    assert_eq!(
        session.sync_failure(),
        Some(&SessionError::Drive(DriveError::Unavailable))
    );
    assert_eq!(session.place(), Place::Main);
    assert_eq!(account_names(&session), vec!["Наличные".to_owned()]);
}

// 016 «Причина сбоя названа»
#[test]
fn reported_failure_carries_the_drive_reason() {
    #[rustfmt::skip]
    let reasons = [
        DriveError::Unavailable,
        DriveError::Conflict,
    ];

    for reason in reasons {
        let drive = FlakyDrive::new(MemoryDrive::new());
        let mut session = signed_in_with_account(drive.clone());

        drive.break_reads(reason);
        pollster::block_on(session.on_return()).expect_err("обновление должно отказать");

        assert_eq!(session.sync_failure(), Some(&SessionError::Drive(reason)));
    }
}

// 016 «Сообщение уходит после успеха»
#[test]
fn successful_return_clears_the_failure() {
    let drive = FlakyDrive::new(MemoryDrive::new());
    let mut session = signed_in_with_account(drive.clone());

    drive.break_reads(DriveError::Unavailable);
    pollster::block_on(session.on_return()).expect_err("обновление должно отказать");
    assert!(session.sync_failure().is_some());

    drive.repair_reads();
    pollster::block_on(session.on_return()).expect("обновление прошло");

    assert!(session.sync_failure().is_none());
}

// 016 «Повторить попытку»
#[test]
fn retry_after_failure_reads_drive_again() {
    let memory = MemoryDrive::new();
    let drive = FlakyDrive::new(memory.clone());
    let mut session = signed_in_with_account(drive.clone());

    let mut phone = Session::new(MemoryIdentity::new(), memory);
    pollster::block_on(phone.open());
    pollster::block_on(phone.sign_in()).expect("вход с телефона");
    pollster::block_on(phone.change(|ledger| {
        create_account(ledger, "Карта", "RUB", 0)?;
        Ok(())
    }))
    .expect("счёт с телефона");

    drive.break_reads(DriveError::Unavailable);
    pollster::block_on(session.on_return()).expect_err("обновление должно отказать");
    let loads = drive.loads();

    drive.repair_reads();
    pollster::block_on(session.on_return()).expect("повтор прошёл");

    assert!(drive.loads() > loads, "повтор не обратился к Drive");
    assert!(session.sync_failure().is_none());
    let mut names = account_names(&session);
    names.sort();
    assert_eq!(names, vec!["Карта".to_owned(), "Наличные".to_owned()]);
}
