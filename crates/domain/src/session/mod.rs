mod drive;
mod files;
mod identity;

use std::future::Future;

use crate::error::DomainError;
use crate::ledger::Ledger;

pub use drive::{Drive, DriveError, DriveFile, FileWrite, MemoryDrive, Revision};
pub use identity::{Identity, MemoryIdentity};

/// Область доступа только к папке приложения в Drive, не к остальным файлам.
pub const APPDATA_SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";

#[rustfmt::skip]
pub const DATA_FILES: &[&str] = &[
    "accounts.json",
    "categories.json",
    "operations.json",
    "counterparties.json",
];

pub const SIGN_IN_REQUIRED: &str = "Без входа приложение не работает";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    Cancelled,
    Denied,
    NeedsConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    SignIn {
        message: Option<&'static str>,
    },
    /// Одно нажатие, чтобы обновить доступ. Пароль и новое разрешение не нужны.
    Confirm,
    Main,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    Auth(AuthError),
    Drive(DriveError),
    Domain(DomainError),
}

#[derive(Debug)]
pub struct Session<I, D> {
    identity: I,
    drive: D,
    place: Place,
    ledger: Option<Ledger>,
    seen: files::Seen,
}

impl<I, D> Session<I, D> {
    #[must_use]
    pub fn new(identity: I, drive: D) -> Self {
        Self {
            identity,
            drive,
            place: Place::SignIn { message: None },
            ledger: None,
            seen: files::empty_seen(),
        }
    }

    #[must_use]
    pub fn identity(&self) -> &I {
        &self.identity
    }

    #[must_use]
    pub fn drive(&self) -> &D {
        &self.drive
    }
}

impl<I: Identity, D: Drive> Session<I, D> {
    #[must_use]
    pub fn place(&self) -> Place {
        self.place
    }

    #[must_use]
    pub fn ledger(&self) -> Option<&Ledger> {
        self.ledger.as_ref()
    }

    /// Пока сбой не снят, журнал на экране мог устареть.
    #[must_use]
    pub fn sync_failure(&self) -> Option<&SessionError> {
        todo!("история 016")
    }

    pub fn open(&mut self) -> impl Future<Output = Place> {
        self.open_place()
    }

    /// Запрашивает только [`APPDATA_SCOPE`].
    pub fn sign_in(&mut self) -> impl Future<Output = Result<Place, SessionError>> {
        self.sign_in_place()
    }

    /// Забывает сеанс только на этом устройстве.
    pub fn sign_out(&mut self) -> Place {
        self.identity.sign_out();
        self.seen = files::empty_seen();
        self.show_sign_in(None);
        self.place
    }

    /// Если файлы в Drive новее, журнал заменяется данными оттуда.
    pub fn on_return(&mut self) -> impl Future<Output = Result<Place, SessionError>> {
        self.reload_on_return()
    }

    /// Сначала перечитывает Drive и только потом применяет правку.
    ///
    /// `AuthError::NeedsConfirmation` — правка не применяется. После [`Session::confirm`]
    /// её вызывают снова с теми же данными.
    pub fn change(
        &mut self,
        edit: impl FnOnce(&mut Ledger) -> Result<(), DomainError>,
    ) -> impl Future<Output = Result<Place, SessionError>> {
        self.change_ledger(edit)
    }

    /// Повторный доступ после `NeedsConfirmation`, без пароля и без нового разрешения.
    pub fn confirm(&mut self) -> impl Future<Output = Result<Place, SessionError>> {
        self.confirm_place()
    }

    async fn open_place(&mut self) -> Place {
        if !self.identity.remembered() {
            self.show_sign_in(None);
            return self.place;
        }
        self.resume_remembered().await;
        self.place
    }

    async fn sign_in_place(&mut self) -> Result<Place, SessionError> {
        let scopes = vec![APPDATA_SCOPE.to_owned()];
        if let Err(error) = self.identity.sign_in(scopes).await {
            self.show_sign_in(Some(SIGN_IN_REQUIRED));
            return Err(SessionError::Auth(error));
        }
        self.enter_main().await
    }

    async fn reload_on_return(&mut self) -> Result<Place, SessionError> {
        if self.place != Place::Main {
            return Ok(self.place);
        }
        self.ready_for_edit().await?;
        self.refresh_if_newer().await?;
        Ok(self.place)
    }

    async fn change_ledger(
        &mut self,
        edit: impl FnOnce(&mut Ledger) -> Result<(), DomainError>,
    ) -> Result<Place, SessionError> {
        self.ready_for_edit().await?;
        self.refresh_if_newer().await?;
        let mut edited = self.editable()?;
        edit(&mut edited).map_err(SessionError::Domain)?;
        self.save_ledger(&edited).await?;
        self.ledger = Some(edited);
        Ok(self.place)
    }

    async fn confirm_place(&mut self) -> Result<Place, SessionError> {
        self.ready_for_edit().await?;
        if self.ledger.is_none() {
            self.load_or_seed().await?;
        }
        self.place = Place::Main;
        Ok(self.place)
    }

    async fn resume_remembered(&mut self) {
        match self.identity.ensure_access().await {
            Ok(()) => self.enter_or_sign_in().await,
            Err(AuthError::NeedsConfirmation) => self.place = Place::Confirm,
            Err(_) => self.show_sign_in(Some(SIGN_IN_REQUIRED)),
        }
    }

    async fn enter_or_sign_in(&mut self) {
        if self.enter_main().await.is_err() {
            self.show_sign_in(Some(SIGN_IN_REQUIRED));
        }
    }

    fn show_sign_in(&mut self, message: Option<&'static str>) {
        self.place = Place::SignIn { message };
        self.ledger = None;
    }

    async fn enter_main(&mut self) -> Result<Place, SessionError> {
        self.load_or_seed().await?;
        self.place = Place::Main;
        Ok(self.place)
    }

    async fn load_or_seed(&mut self) -> Result<(), SessionError> {
        let loaded = self.drive.load().await.map_err(SessionError::Drive)?;
        if !files::has_data(&loaded) {
            return self.seed_and_save().await;
        }
        self.install(&loaded)
    }

    async fn seed_and_save(&mut self) -> Result<(), SessionError> {
        let mut ledger = Ledger::new();
        ledger.seed_starter_categories();
        self.ledger = Some(ledger);
        self.seen = files::empty_seen();
        self.save().await
    }

    fn install(&mut self, loaded: &[DriveFile]) -> Result<(), SessionError> {
        self.ledger = Some(files::decode(loaded)?);
        self.seen = files::seen_from(loaded);
        Ok(())
    }

    async fn save(&mut self) -> Result<(), SessionError> {
        let ledger = self.editable()?;
        self.save_ledger(&ledger).await
    }

    async fn save_ledger(&mut self, ledger: &Ledger) -> Result<(), SessionError> {
        let writes = files::writes(ledger, &self.seen)?;
        self.drive
            .write(writes)
            .await
            .map_err(SessionError::Drive)?;
        self.remember_versions().await;
        Ok(())
    }

    /// Поле `version` у Drive не равно нашему шагу. Если перечитать не вышло, сдвигаем локально.
    async fn remember_versions(&mut self) {
        match self.drive.load().await {
            Ok(loaded) => self.seen = files::seen_from(&loaded),
            Err(_) => files::advance(&mut self.seen),
        }
    }

    fn editable(&self) -> Result<Ledger, SessionError> {
        self.ledger
            .clone()
            .ok_or(SessionError::Drive(DriveError::Unavailable))
    }

    async fn ready_for_edit(&mut self) -> Result<(), SessionError> {
        self.identity
            .ensure_access()
            .await
            .map_err(SessionError::Auth)
    }

    async fn refresh_if_newer(&mut self) -> Result<(), SessionError> {
        let loaded = self.drive.load().await.map_err(SessionError::Drive)?;
        if !files::remote_is_newer(&loaded, &self.seen) {
            return Ok(());
        }
        self.install(&loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        APPDATA_SCOPE, AuthError, Drive, DriveError, DriveFile, FileWrite, MemoryDrive,
        MemoryIdentity, Place, SIGN_IN_REQUIRED, Session, SessionError,
    };
    use pollster::block_on;

    #[test]
    fn fresh_open_shows_sign_in_without_ledger() {
        let mut session = Session::new(MemoryIdentity::new(), MemoryDrive::new());
        let place = block_on(session.open());
        assert_eq!(place, Place::SignIn { message: None });
        assert_eq!(session.place(), Place::SignIn { message: None });
        assert!(session.ledger().is_none());
    }

    #[test]
    fn sign_in_seeds_empty_drive_with_only_app_data_scope() {
        let identity = MemoryIdentity::new();
        let drive = MemoryDrive::new();
        let mut session = Session::new(identity.clone(), drive.clone());
        block_on(session.open());
        let place = block_on(session.sign_in()).expect("вход");
        assert_eq!(place, Place::Main);
        assert_eq!(identity.requested_scopes(), vec![APPDATA_SCOPE.to_owned()]);
        let count = session.ledger().expect("журнал").categories().len();
        assert!(count > 0);
        assert!(session.ledger().expect("журнал").accounts().is_empty());

        let mut other = Session::new(MemoryIdentity::new(), drive);
        block_on(other.open());
        block_on(other.sign_in()).expect("второй вход");
        let loaded = other.ledger().expect("журнал");
        assert_eq!(loaded.categories().len(), count);
        assert!(loaded.accounts().is_empty());
    }

    #[test]
    fn cancelled_sign_in_stays_on_sign_in() {
        let identity = MemoryIdentity::new();
        let mut session = Session::new(identity.clone(), MemoryDrive::new());
        identity.reject_next_sign_in(AuthError::Cancelled);
        block_on(session.open());
        let error = block_on(session.sign_in()).expect_err("отказ");
        assert_eq!(error, SessionError::Auth(AuthError::Cancelled));
        assert_eq!(
            session.place(),
            Place::SignIn {
                message: Some(SIGN_IN_REQUIRED),
            }
        );
        assert!(session.ledger().is_none());
    }

    #[test]
    fn change_reloads_newer_drive_before_edit() {
        use crate::account::CreateAccount;
        use crate::currency::Currency;

        let drive = MemoryDrive::new();
        let mut phone = Session::new(MemoryIdentity::new(), drive.clone());
        let mut computer = Session::new(MemoryIdentity::new(), drive);
        block_on(phone.open());
        block_on(phone.sign_in()).expect("телефон");
        block_on(computer.open());
        block_on(computer.sign_in()).expect("компьютер");
        block_on(phone.change(|ledger| {
            ledger.create_account(CreateAccount {
                name: "Наличные".into(),
                currency: Currency::new("RUB"),
                opening_balance: 0,
            })?;
            Ok(())
        }))
        .expect("телефон пишет");
        block_on(computer.on_return()).expect("возврат");
        assert_eq!(
            computer.ledger().expect("журнал").accounts()[0].name(),
            "Наличные"
        );
        block_on(computer.change(|ledger| {
            ledger.create_account(CreateAccount {
                name: "Карта".into(),
                currency: Currency::new("RUB"),
                opening_balance: 0,
            })?;
            Ok(())
        }))
        .expect("компьютер пишет");
        let names: Vec<_> = computer
            .ledger()
            .expect("журнал")
            .accounts()
            .iter()
            .map(|account| account.name().to_owned())
            .collect();
        assert_eq!(names, vec!["Наличные".to_owned(), "Карта".to_owned()]);
    }

    #[test]
    fn expired_open_confirms_without_new_sign_in() {
        let identity = MemoryIdentity::new();
        let drive = MemoryDrive::new();
        let mut session = Session::new(identity.clone(), drive.clone());
        block_on(session.open());
        block_on(session.sign_in()).expect("вход");
        session.sign_out();
        assert_eq!(session.place(), Place::SignIn { message: None });
        assert!(session.ledger().is_none());

        let mut session = Session::new(identity.clone(), drive.clone());
        block_on(session.open());
        block_on(session.sign_in()).expect("повторный вход");
        let sign_ins = identity.sign_in_count();
        identity.expire_access();
        let mut session = Session::new(identity.clone(), drive);
        assert_eq!(block_on(session.open()), Place::Confirm);
        assert_eq!(identity.sign_in_count(), sign_ins);
        let place = block_on(session.confirm()).expect("подтверждение");
        assert_eq!(place, Place::Main);
        assert_eq!(identity.sign_in_count(), sign_ins);
    }

    #[test]
    fn failed_write_drops_the_edit() {
        use crate::account::CreateAccount;
        use crate::currency::Currency;

        let memory = MemoryDrive::new();
        let drive = LimitedDrive {
            inner: memory.clone(),
            remaining: 1,
        };
        let mut session = Session::new(MemoryIdentity::new(), drive);
        block_on(session.open());
        block_on(session.sign_in()).expect("вход");
        let error = block_on(session.change(|ledger| {
            ledger.create_account(CreateAccount {
                name: "Наличные".into(),
                currency: Currency::new("RUB"),
                opening_balance: 0,
            })?;
            Ok(())
        }))
        .expect_err("запись");
        assert_eq!(error, SessionError::Drive(DriveError::Unavailable));
        assert!(session.ledger().expect("журнал").accounts().is_empty());

        let mut other = Session::new(MemoryIdentity::new(), memory);
        block_on(other.open());
        block_on(other.sign_in()).expect("чтение");
        assert!(other.ledger().expect("журнал").accounts().is_empty());
    }

    #[test]
    fn expired_return_stays_on_main() {
        let identity = MemoryIdentity::new();
        let mut session = Session::new(identity.clone(), MemoryDrive::new());
        block_on(session.open());
        block_on(session.sign_in()).expect("вход");
        let sign_ins = identity.sign_in_count();
        identity.expire_access();
        let error = block_on(session.on_return()).expect_err("нужно подтверждение");
        assert_eq!(error, SessionError::Auth(AuthError::NeedsConfirmation));
        assert_eq!(session.place(), Place::Main);
        assert!(session.ledger().is_some());
        assert_eq!(identity.sign_in_count(), sign_ins);
        block_on(session.confirm()).expect("подтверждение");
        assert_eq!(session.place(), Place::Main);
        assert!(session.ledger().is_some());
        assert_eq!(identity.sign_in_count(), sign_ins);
    }

    struct LimitedDrive {
        inner: MemoryDrive,
        remaining: u32,
    }

    impl Drive for LimitedDrive {
        async fn load(&self) -> Result<Vec<DriveFile>, DriveError> {
            self.inner.load().await
        }

        async fn write(&mut self, files: Vec<FileWrite>) -> Result<(), DriveError> {
            if self.remaining == 0 {
                return Err(DriveError::Unavailable);
            }
            self.remaining -= 1;
            self.inner.write(files).await
        }
    }
}
