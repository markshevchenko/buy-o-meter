mod drive;
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
}

impl<I, D> Session<I, D> {
    #[must_use]
    pub fn new(identity: I, drive: D) -> Self {
        Self {
            identity,
            drive,
            place: Place::SignIn { message: None },
            ledger: None,
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
        let _ = self.ports_mut();
        pending()
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
}

#[cfg(test)]
mod tests {
    use super::{MemoryDrive, MemoryIdentity, Place, Session};
    use pollster::block_on;

    #[test]
    fn fresh_open_shows_sign_in_without_ledger() {
        let mut session = Session::new(MemoryIdentity::new(), MemoryDrive::new());
        let place = block_on(session.open());
        assert_eq!(place, Place::SignIn { message: None });
        assert_eq!(session.place(), Place::SignIn { message: None });
        assert!(session.ledger().is_none());
    }
}

fn pending<T>() -> impl Future<Output = T> {
    poll_fn(|_| todo!())
}
