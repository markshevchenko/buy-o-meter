use std::cell::RefCell;
use std::future::{Future, ready};
use std::rc::Rc;

use super::AuthError;

pub trait Identity {
    fn remembered(&self) -> bool;

    fn sign_in(&mut self, scopes: Vec<String>) -> impl Future<Output = Result<(), AuthError>>;

    fn ensure_access(&mut self) -> impl Future<Output = Result<(), AuthError>>;

    /// Сеанс приложения на этом устройстве. Аккаунт Google в браузере не трогает.
    fn sign_out(&mut self);
}

#[derive(Debug)]
struct IdentityState {
    remembered: bool,
    google_signed_in: bool,
    scopes: Vec<String>,
    sign_ins: u32,
    reject: Option<AuthError>,
    needs_gesture: bool,
}

#[derive(Debug, Clone)]
pub struct MemoryIdentity {
    inner: Rc<RefCell<IdentityState>>,
}

impl MemoryIdentity {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(IdentityState {
                remembered: false,
                google_signed_in: true,
                scopes: Vec::new(),
                sign_ins: 0,
                reject: None,
                needs_gesture: false,
            })),
        }
    }

    #[must_use]
    pub fn google_account_signed_in(&self) -> bool {
        self.inner.borrow().google_signed_in
    }

    #[must_use]
    pub fn requested_scopes(&self) -> Vec<String> {
        self.inner.borrow().scopes.clone()
    }

    #[must_use]
    pub fn sign_in_count(&self) -> u32 {
        self.inner.borrow().sign_ins
    }

    pub fn reject_next_sign_in(&self, error: AuthError) {
        self.inner.borrow_mut().reject = Some(error);
    }

    /// Следующий `ensure_access` просит одно нажатие и больше не просит.
    pub fn expire_access(&self) {
        self.inner.borrow_mut().needs_gesture = true;
    }
}

impl Default for MemoryIdentity {
    fn default() -> Self {
        Self::new()
    }
}

impl Identity for MemoryIdentity {
    fn remembered(&self) -> bool {
        self.inner.borrow().remembered
    }

    fn sign_in(&mut self, scopes: Vec<String>) -> impl Future<Output = Result<(), AuthError>> {
        ready(self.sign_in_now(scopes))
    }

    fn ensure_access(&mut self) -> impl Future<Output = Result<(), AuthError>> {
        ready(self.ensure_now())
    }

    fn sign_out(&mut self) {
        self.inner.borrow_mut().remembered = false;
    }
}

impl MemoryIdentity {
    fn sign_in_now(&self, scopes: Vec<String>) -> Result<(), AuthError> {
        let mut state = self.inner.borrow_mut();
        if let Some(error) = state.reject.take() {
            return Err(error);
        }
        state.scopes = scopes;
        state.sign_ins += 1;
        state.remembered = true;
        state.google_signed_in = true;
        Ok(())
    }

    fn ensure_now(&self) -> Result<(), AuthError> {
        let mut state = self.inner.borrow_mut();
        if state.needs_gesture {
            state.needs_gesture = false;
            return Err(AuthError::NeedsConfirmation);
        }
        Ok(())
    }
}
