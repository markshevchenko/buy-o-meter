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
}

impl<I, D> Session<I, D> {
    #[must_use]
    pub fn new(identity: I, drive: D) -> Self {
        Self { identity, drive }
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
        let _ = (&self.identity, &self.drive);
        todo!()
    }

    #[must_use]
    pub fn ledger(&self) -> Option<&Ledger> {
        let _ = (&self.identity, &self.drive);
        todo!()
    }

    pub fn open(&mut self) -> impl Future<Output = Place> {
        let _ = self.ports_mut();
        pending()
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
}

fn pending<T>() -> impl Future<Output = T> {
    poll_fn(|_| todo!())
}
