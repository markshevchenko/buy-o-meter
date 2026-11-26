use std::sync::{Arc, Mutex as StdMutex};

use futures::lock::Mutex;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use domain::{
    AccountId, AuthError, CategoryId, CreateAccount, CreateCategory, Currency, DomainError, Ledger,
    OperationId, OperationKind, Place, Session, SessionError, UpdateAccount, UpdateCategory,
};

use crate::draft::{Draft, apply};
use crate::drive_api::GoogleDrive;
use crate::google::GoogleIdentity;
use crate::mode::Mode;
use crate::tab::Tab;
use crate::text::{error_text, session_error_text};
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
    CreateCategory {
        input: CreateCategory,
        message: RwSignal<Option<String>>,
    },
    UpdateCategory {
        id: CategoryId,
        input: UpdateCategory,
        message: RwSignal<Option<String>>,
    },
    MoveCategory {
        id: CategoryId,
        new_parent_id: CategoryId,
        message: RwSignal<Option<String>>,
    },
    DeleteCategory {
        id: CategoryId,
        transfer_to: Option<CategoryId>,
        message: RwSignal<Option<String>>,
    },
    NewAccount {
        input: CreateAccount,
        message: RwSignal<Option<String>>,
        show: RwSignal<bool>,
    },
    UpdateAccount {
        id: AccountId,
        input: UpdateAccount,
        message: RwSignal<Option<String>>,
    },
}

#[derive(Clone)]
pub(crate) struct AppApi {
    session: Arc<Mutex<Session<GoogleIdentity, GoogleDrive>>>,
    place: RwSignal<Place>,
    ledger: RwSignal<Ledger>,
    mode: RwSignal<Mode>,
    tab: RwSignal<Tab>,
    needs_confirm: RwSignal<bool>,
    confirm_error: RwSignal<Option<String>>,
    sync_error: RwSignal<Option<String>>,
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
            tab: RwSignal::new(Tab::Operations),
            needs_confirm: RwSignal::new(false),
            confirm_error: RwSignal::new(None),
            sync_error: RwSignal::new(None),
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

    pub(crate) fn tab(&self) -> RwSignal<Tab> {
        self.tab
    }

    pub(crate) fn needs_confirm(&self) -> RwSignal<bool> {
        self.needs_confirm
    }

    pub(crate) fn confirm_error(&self) -> RwSignal<Option<String>> {
        self.confirm_error
    }

    pub(crate) fn sync_error(&self) -> RwSignal<Option<String>> {
        self.sync_error
    }

    pub(crate) fn open(&self) {
        let api = self.clone();
        spawn_local(async move {
            let mut session = api.session.lock().await;
            let place = session.open().await;
            let ledger = session.ledger().cloned();
            drop(session);
            api.publish(place, ledger, true);
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
            api.publish(place, ledger, true);
        });
    }

    pub(crate) fn sign_out(&self) {
        let api = self.clone();
        spawn_local(async move {
            let mut session = api.session.lock().await;
            let place = session.sign_out();
            drop(session);
            api.publish(place, None, false);
        });
    }

    pub(crate) fn confirm(&self) {
        let api = self.clone();
        spawn_local(async move {
            if let Err(error) = confirm_session(&api).await {
                api.confirm_error.set(Some(session_error_text(&error)));
                return;
            }
            api.confirm_error.set(None);
            api.needs_confirm.set(false);
            if let Some(pending) = take_pending(api.pending) {
                api.retry(pending);
                return;
            }
            if api.place.get_untracked() == Place::Main {
                api.on_return();
            }
        });
    }

    pub(crate) fn abandon(&self) {
        self.needs_confirm.set(false);
        self.confirm_error.set(None);
        self.pending.set(None);
        self.mode.set(Mode::Feed);
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
            let failure = session.sync_failure().map(session_error_text);
            drop(session);
            api.sync_error.set(failure);
            match result {
                Ok(_) => api.publish(place, ledger, false),
                Err(SessionError::Auth(AuthError::NeedsConfirmation)) => {
                    api.needs_confirm.set(true);
                }
                // Причину уже держит sync_error, её показывает баннер устаревших данных.
                Err(_) => {}
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
        let Some(pending) = self.hold(pending) else {
            return;
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
        let Some(pending) = self.hold(pending) else {
            return;
        };
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
        let Some(pending) = self.hold(pending) else {
            return;
        };
        let api = self.clone();
        self.edit(
            move |ledger| store_account(&created_edit, ledger, name, currency, opening),
            move |result| api.finish_account(result, input, created, pending),
        );
    }

    pub(crate) fn create_category(&self, input: CreateCategory, message: RwSignal<Option<String>>) {
        let pending = Pending::CreateCategory {
            input: input.clone(),
            message,
        };
        let Some(pending) = self.hold(pending) else {
            return;
        };
        let api = self.clone();
        self.edit(
            move |ledger| ledger.create_category(input).map(|_| ()),
            move |result| api.settle(result, message, pending, || {}),
        );
    }

    pub(crate) fn update_category(
        &self,
        id: CategoryId,
        input: UpdateCategory,
        message: RwSignal<Option<String>>,
    ) {
        let pending = Pending::UpdateCategory {
            id,
            input: input.clone(),
            message,
        };
        let Some(pending) = self.hold(pending) else {
            return;
        };
        let api = self.clone();
        self.edit(
            move |ledger| ledger.update_category(id, input),
            move |result| api.settle(result, message, pending, || {}),
        );
    }

    pub(crate) fn move_category(
        &self,
        id: CategoryId,
        new_parent_id: CategoryId,
        message: RwSignal<Option<String>>,
    ) {
        let pending = Pending::MoveCategory {
            id,
            new_parent_id,
            message,
        };
        let Some(pending) = self.hold(pending) else {
            return;
        };
        let api = self.clone();
        self.edit(
            move |ledger| ledger.move_category(id, new_parent_id),
            move |result| api.settle(result, message, pending, || {}),
        );
    }

    pub(crate) fn delete_category(
        &self,
        id: CategoryId,
        transfer_to: Option<CategoryId>,
        message: RwSignal<Option<String>>,
    ) {
        let pending = Pending::DeleteCategory {
            id,
            transfer_to,
            message,
        };
        let Some(pending) = self.hold(pending) else {
            return;
        };
        let api = self.clone();
        self.edit(
            move |ledger| ledger.delete_category(id, transfer_to),
            move |result| api.settle(result, message, pending, || {}),
        );
    }

    pub(crate) fn add_account(
        &self,
        input: CreateAccount,
        message: RwSignal<Option<String>>,
        show: RwSignal<bool>,
    ) {
        let pending = Pending::NewAccount {
            input: input.clone(),
            message,
            show,
        };
        let Some(pending) = self.hold(pending) else {
            return;
        };
        let api = self.clone();
        self.edit(
            move |ledger| ledger.create_account(input).map(|_| ()),
            move |result| api.settle(result, message, pending, move || show.set(false)),
        );
    }

    pub(crate) fn update_account(
        &self,
        id: AccountId,
        input: UpdateAccount,
        message: RwSignal<Option<String>>,
    ) {
        let pending = Pending::UpdateAccount {
            id,
            input: input.clone(),
            message,
        };
        let Some(pending) = self.hold(pending) else {
            return;
        };
        let api = self.clone();
        self.edit(
            move |ledger| ledger.update_account(id, input),
            move |result| api.settle(result, message, pending, || {}),
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
            let ledger = result.as_ref().ok().and_then(|_| session.ledger().cloned());
            drop(session);
            if result.is_ok() {
                api.place.set(place);
                api.sync_error.set(None);
                if let Some(ledger) = ledger {
                    api.ledger.set(ledger);
                }
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
                self.pending.set(None);
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
                self.pending.set(None);
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

    fn hold(&self, pending: Pending) -> Option<Pending> {
        if !self.needs_confirm.get_untracked() {
            return Some(pending);
        }
        self.pending.set(Some(pending));
        None
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
            Pending::CreateCategory { input, message } => self.create_category(input, message),
            Pending::UpdateCategory { id, input, message } => {
                self.update_category(id, input, message);
            }
            Pending::MoveCategory {
                id,
                new_parent_id,
                message,
            } => self.move_category(id, new_parent_id, message),
            Pending::DeleteCategory {
                id,
                transfer_to,
                message,
            } => self.delete_category(id, transfer_to, message),
            Pending::NewAccount {
                input,
                message,
                show,
            } => self.add_account(input, message, show),
            Pending::UpdateAccount { id, input, message } => {
                self.update_account(id, input, message);
            }
        }
    }

    fn publish(&self, place: Place, ledger: Option<Ledger>, is_initial_load: bool) {
        self.place.set(place);
        if place == Place::Main {
            if let Some(ledger) = ledger {
                if is_initial_load && ledger.accounts().is_empty() {
                    self.tab.set(Tab::Accounts);
                }
                self.ledger.set(ledger);
            }
            return;
        }
        self.ledger.set(Ledger::new());
        self.mode.set(Mode::Feed);
        self.tab.set(Tab::Operations);
        self.needs_confirm.set(false);
        self.confirm_error.set(None);
        self.sync_error.set(None);
        self.pending.set(None);
    }
}

async fn confirm_session(api: &AppApi) -> Result<(), SessionError> {
    let mut session = api.session.lock().await;
    let result = session.confirm().await;
    let place = session.place();
    let ledger = session.ledger().cloned();
    drop(session);
    result?;
    api.publish(place, ledger, true);
    Ok(())
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
        credit_limit: 0,
        note: String::new(),
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
