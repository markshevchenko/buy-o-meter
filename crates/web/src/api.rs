use std::sync::{Arc, Mutex as StdMutex};

use futures::lock::Mutex;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use domain::{
    AccountId, AuthError, CreateAccount, Currency, DomainError, Ledger, OperationId, OperationKind,
    Place, Session, SessionError,
};

use crate::draft::{Draft, apply};
use crate::drive_api::GoogleDrive;
use crate::google::GoogleIdentity;
use crate::mode::Mode;
use crate::text::error_text;
use crate::token::Tokens;

#[derive(Clone, Copy)]
pub(crate) struct AccountSlots {
    pub(crate) account: RwSignal<String>,
    pub(crate) to_account: RwSignal<String>,
    pub(crate) name: RwSignal<String>,
}

#[derive(Clone)]
pub(crate) struct NewAccount {
    pub(crate) name: String,
    pub(crate) currency: String,
    pub(crate) opening: i64,
    pub(crate) kind: OperationKind,
    pub(crate) slots: AccountSlots,
    pub(crate) note: RwSignal<String>,
}

enum Pending {
    Apply {
        editing: Option<OperationId>,
        kind: OperationKind,
        draft: Draft,
        message: RwSignal<Option<String>>,
    },
    Delete {
        id: OperationId,
        message: RwSignal<Option<String>>,
    },
    Account(NewAccount),
}

#[derive(Clone)]
pub(crate) struct AppApi {
    session: Arc<Mutex<Session<GoogleIdentity, GoogleDrive>>>,
    place: RwSignal<Place>,
    ledger: RwSignal<Ledger>,
    mode: RwSignal<Mode>,
    needs_confirm: RwSignal<bool>,
    pending: RwSignal<Option<Pending>>,
}

impl AppApi {
    pub(crate) fn new() -> Self {
        let tokens = Tokens::new();
        let session = Session::new(
            GoogleIdentity::new(tokens.clone()),
            GoogleDrive::new(tokens),
        );
        Self {
            session: Arc::new(Mutex::new(session)),
            place: RwSignal::new(Place::SignIn { message: None }),
            ledger: RwSignal::new(Ledger::new()),
            mode: RwSignal::new(Mode::Feed),
            needs_confirm: RwSignal::new(false),
            pending: RwSignal::new(None),
        }
    }

    pub(crate) fn place(&self) -> RwSignal<Place> {
        self.place
    }

    pub(crate) fn ledger(&self) -> RwSignal<Ledger> {
        self.ledger
    }

    pub(crate) fn mode(&self) -> RwSignal<Mode> {
        self.mode
    }

    pub(crate) fn needs_confirm(&self) -> RwSignal<bool> {
        self.needs_confirm
    }

    pub(crate) fn open(&self) {
        let api = self.clone();
        spawn_local(async move {
            let mut session = api.session.lock().await;
            let place = session.open().await;
            let ledger = session.ledger().cloned();
            drop(session);
            api.publish(place, ledger);
        });
    }

    pub(crate) fn sign_in(&self) {
        let api = self.clone();
        spawn_local(async move {
            let mut session = api.session.lock().await;
            let _ = session.sign_in().await;
            let place = session.place();
            let ledger = session.ledger().cloned();
            drop(session);
            api.publish(place, ledger);
        });
    }

    pub(crate) fn sign_out(&self) {
        let api = self.clone();
        spawn_local(async move {
            let mut session = api.session.lock().await;
            let place = session.sign_out();
            drop(session);
            api.publish(place, None);
        });
    }

    pub(crate) fn confirm(&self) {
        let api = self.clone();
        spawn_local(async move {
            let ready = confirm_session(&api).await;
            if !ready {
                return;
            }
            api.needs_confirm.set(false);
            if let Some(pending) = take_pending(api.pending) {
                api.retry(pending);
            }
        });
    }

    pub(crate) fn watch_visibility(&self) {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let api = self.clone();
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            if document_visible() {
                api.on_return();
            }
        });
        if document
            .add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref())
            .is_ok()
        {
            closure.forget();
        }
    }

    pub(crate) fn on_return(&self) {
        if self.place.get_untracked() != Place::Main {
            return;
        }
        let api = self.clone();
        spawn_local(async move {
            let mut session = api.session.lock().await;
            let result = session.on_return().await;
            let place = session.place();
            let ledger = session.ledger().cloned();
            drop(session);
            if result.is_ok() {
                api.publish(place, ledger);
            }
        });
    }

    pub(crate) fn save_operation(
        &self,
        editing: Option<OperationId>,
        kind: OperationKind,
        draft: Draft,
        message: RwSignal<Option<String>>,
    ) {
        let pending = Pending::Apply {
            editing,
            kind,
            draft: draft.clone(),
            message,
        };
        let api = self.clone();
        let on_ok = self.clone();
        self.edit(
            move |ledger| apply(ledger, editing, kind, draft),
            move |result| {
                api.settle(result, message, pending, move || {
                    on_ok.mode.set(Mode::Feed);
                });
            },
        );
    }

    pub(crate) fn delete_operation(&self, id: OperationId, message: RwSignal<Option<String>>) {
        let pending = Pending::Delete { id, message };
        let api = self.clone();
        let on_ok = self.clone();
        self.edit(
            move |ledger| ledger.delete_operation(id),
            move |result| {
                api.settle(result, message, pending, move || {
                    on_ok.mode.set(Mode::Feed);
                });
            },
        );
    }

    pub(crate) fn create_account(&self, input: NewAccount) {
        let created = Arc::new(StdMutex::new(None));
        let created_edit = created.clone();
        let name = input.name.clone();
        let currency = input.currency.clone();
        let opening = input.opening;
        let pending = Pending::Account(input.clone());
        let api = self.clone();
        self.edit(
            move |ledger| store_account(&created_edit, ledger, name, currency, opening),
            move |result| api.finish_account(result, input, created, pending),
        );
    }

    fn edit(
        &self,
        edit: impl FnOnce(&mut Ledger) -> Result<(), DomainError> + 'static,
        done: impl FnOnce(Result<Place, SessionError>) + 'static,
    ) {
        let api = self.clone();
        spawn_local(async move {
            let mut session = api.session.lock().await;
            let result = session.change(edit).await;
            let place = session.place();
            let ledger = session.ledger().cloned();
            drop(session);
            api.place.set(place);
            if let Some(ledger) = ledger {
                api.ledger.set(ledger);
            }
            done(result);
        });
    }

    fn settle(
        &self,
        result: Result<Place, SessionError>,
        message: RwSignal<Option<String>>,
        pending: Pending,
        on_ok: impl FnOnce(),
    ) {
        match result {
            Ok(_) => {
                self.needs_confirm.set(false);
                on_ok();
            }
            Err(SessionError::Auth(AuthError::NeedsConfirmation)) => {
                self.needs_confirm.set(true);
                self.pending.set(Some(pending));
            }
            Err(SessionError::Domain(error)) => message.set(Some(error_text(&error))),
            Err(_) => message.set(Some("Не удалось сохранить".to_owned())),
        }
    }

    fn finish_account(
        &self,
        result: Result<Place, SessionError>,
        input: NewAccount,
        created: Arc<StdMutex<Option<AccountId>>>,
        pending: Pending,
    ) {
        let note = input.note;
        let slots = input.slots;
        let kind = input.kind;
        match result {
            Ok(_) => {
                self.needs_confirm.set(false);
                place_account(&created, kind, &slots);
                slots.name.set(String::new());
                note.set(String::new());
            }
            Err(SessionError::Auth(AuthError::NeedsConfirmation)) => {
                self.needs_confirm.set(true);
                self.pending.set(Some(pending));
            }
            Err(SessionError::Domain(error)) => note.set(error_text(&error)),
            Err(_) => note.set("Не удалось сохранить".to_owned()),
        }
    }

    fn retry(&self, pending: Pending) {
        match pending {
            Pending::Apply {
                editing,
                kind,
                draft,
                message,
            } => self.save_operation(editing, kind, draft, message),
            Pending::Delete { id, message } => self.delete_operation(id, message),
            Pending::Account(input) => self.create_account(input),
        }
    }

    fn publish(&self, place: Place, ledger: Option<Ledger>) {
        self.place.set(place);
        if place == Place::Main {
            if let Some(ledger) = ledger {
                self.ledger.set(ledger);
            }
            return;
        }
        self.ledger.set(Ledger::new());
        self.mode.set(Mode::Feed);
        self.needs_confirm.set(false);
        self.pending.set(None);
    }
}

async fn confirm_session(api: &AppApi) -> bool {
    let mut session = api.session.lock().await;
    let result = session.confirm().await;
    let place = session.place();
    let ledger = session.ledger().cloned();
    drop(session);
    api.publish(place, ledger);
    result.is_ok()
}

fn store_account(
    created: &StdMutex<Option<AccountId>>,
    ledger: &mut Ledger,
    name: String,
    currency: String,
    opening: i64,
) -> Result<(), DomainError> {
    let id = ledger.create_account(CreateAccount {
        name,
        currency: Currency::new(currency),
        opening_balance: opening,
    })?;
    if let Ok(mut slot) = created.lock() {
        *slot = Some(id);
    }
    Ok(())
}

fn take_pending(pending: RwSignal<Option<Pending>>) -> Option<Pending> {
    pending.try_update(Option::take).flatten()
}

pub(crate) fn select_created_account(kind: OperationKind, slots: &AccountSlots, id: String) {
    if slots.account.get().is_empty() {
        slots.account.set(id);
        return;
    }
    if kind == OperationKind::Transfer && slots.to_account.get().is_empty() {
        slots.to_account.set(id);
    }
}

fn document_visible() -> bool {
    web_sys::window()
        .and_then(|window| window.document())
        .is_some_and(|document| document.visibility_state() == web_sys::VisibilityState::Visible)
}

fn place_account(created: &StdMutex<Option<AccountId>>, kind: OperationKind, slots: &AccountSlots) {
    let Ok(slot) = created.lock() else {
        return;
    };
    if let Some(id) = *slot {
        select_created_account(kind, slots, id.as_u64().to_string());
    }
}
