mod drive;
mod files;
mod identity;

use std::future::{Future, poll_fn};

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

    pub fn open(&mut self) -> impl Future<Output = Place> {
        async move {
            if self.identity.remembered() {
                todo!()
            }
            self.show_sign_in(None);
            self.place
        }
    }

    /// Запрашивает только [`APPDATA_SCOPE`].
    pub fn sign_in(&mut self) -> impl Future<Output = Result<Place, SessionError>> {
        async move {
            let scopes = vec![APPDATA_SCOPE.to_owned()];
            if let Err(error) = self.identity.sign_in(scopes).await {
                self.show_sign_in(Some(SIGN_IN_REQUIRED));
                return Err(SessionError::Auth(error));
            }
            self.enter_main().await
        }
    }

    /// Забывает сеанс только на этом устройстве.
    pub fn sign_out(&mut self) -> Place {
        let _ = self.ports_mut();
        todo!()
    }

    /// Если файлы в Drive новее, журнал заменяется данными оттуда.
    pub fn on_return(&mut self) -> impl Future<Output = Result<Place, SessionError>> {
        let _ = self.ports_mut();
        pending()
    }

    /// Сначала перечитывает Drive и только потом применяет правку.
    ///
    /// `AuthError::NeedsConfirmation` — правка не применяется. После [`Session::confirm`]
    /// её вызывают снова с теми же данными.
    pub fn change(
        &mut self,
        edit: impl FnOnce(&mut Ledger) -> Result<(), DomainError>,
    ) -> impl Future<Output = Result<Place, SessionError>> {
        let _ = (self.ports_mut(), edit);
        pending()
    }

    /// Повторный доступ после `NeedsConfirmation`, без пароля и без нового разрешения.
    pub fn confirm(&mut self) -> impl Future<Output = Result<Place, SessionError>> {
        let _ = self.ports_mut();
        pending()
    }

    fn ports_mut(&mut self) -> (&mut I, &mut D) {
        (&mut self.identity, &mut self.drive)
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
        let writes = self.pending_writes()?;
        self.drive
            .write(writes)
            .await
            .map_err(SessionError::Drive)?;
        files::advance(&mut self.seen);
        Ok(())
    }

    fn pending_writes(&self) -> Result<Vec<FileWrite>, SessionError> {
        let ledger = self
            .ledger
            .as_ref()
            .ok_or(SessionError::Drive(DriveError::Unavailable))?;
        files::writes(ledger, &self.seen)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        APPDATA_SCOPE, AuthError, MemoryDrive, MemoryIdentity, Place, SIGN_IN_REQUIRED, Session,
        SessionError,
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
}

fn pending<T>() -> impl Future<Output = T> {
    poll_fn(|_| todo!())
}
