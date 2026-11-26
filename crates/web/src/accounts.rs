use leptos::prelude::*;

use domain::{Account, AccountId, UpdateAccount};

use crate::api::AppApi;

pub(crate) fn accounts_page(api: AppApi) -> impl IntoView {
    let ledger = api.ledger();
    let editing = RwSignal::new(None::<AccountId>);
    let list = api.clone();
    let show_create = RwSignal::new(false);
    view! {
        {move || {
            let accounts: Vec<Account> = ledger.with(|ledger| ledger.accounts().to_vec());
            if accounts.is_empty() {
                empty_accounts_view(api.clone(), show_create).into_any()
            } else {
                accounts_table_view(list.clone(), editing, api.clone(), show_create).into_any()
            }
        }}
    }
}

fn empty_accounts_view(api: AppApi, show_create: RwSignal<bool>) -> impl IntoView {
    view! {
        <p class="invite">
            "Для начала заведите ваши счета: один счёт для каждой наличной валюты, "
            "которой вы пользуетесь, по одному счёту на каждую карту и на каждый открытый счёт в банке."
        </p>
        <button type="button" on:click=move |_| show_create.set(true)>
            "Создать счёт"
        </button>
        {move || {
            if show_create.get() {
                create_account::dialog(api.clone(), show_create).into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

fn accounts_table_view(
    api: AppApi,
    editing: RwSignal<Option<AccountId>>,
    list: AppApi,
    show_create: RwSignal<bool>,
) -> impl IntoView {
    let ledger = api.ledger();
    view! {
        <table class="accounts">
            <thead>
                <tr>
                    <th>"Название"</th>
                    <th>"Остаток"</th>
                </tr>
            </thead>
            <tbody>
                {move || {
                    let accounts: Vec<Account> = ledger.with(|ledger| ledger.accounts().to_vec());
                    accounts
                        .into_iter()
                        .map(|account| account_row(list.clone(), editing, account))
                        .collect_view()
                }}
            </tbody>
        </table>
        <button type="button" on:click=move |_| show_create.set(true)>
            "Создать счёт"
        </button>
        {move || {
            if show_create.get() {
                create_account::dialog(api.clone(), show_create).into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

mod create_account {
    use crate::api::AppApi;
    use leptos::prelude::*;

    pub(super) fn dialog(api: AppApi, show: RwSignal<bool>) -> impl IntoView {
        view! { <dialog>"Модальное окно создания счёта (заглушка)"</dialog> }
    }
}

fn account_row(
    api: AppApi,
    editing: RwSignal<Option<AccountId>>,
    account: Account,
) -> impl IntoView {
    let id = account.id();
    let ledger = api.ledger();
    let name = account.name().to_owned();
    let currency = account.currency().as_str().to_owned();
    view! {
        <tr>
            <td>
                <button type="button" class="line" on:click=move |_| toggle(editing, id)>
                    {name.clone()}
                </button>
            </td>
            <td>
                {move || {
                    let balance = ledger.with(|ledger| ledger.balance(id).unwrap_or(0));
                    format!("{balance} {currency}")
                }}
            </td>
        </tr>
        {move || {
            if editing.get() == Some(id) {
                edit_panel(api.clone(), id).into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

fn toggle(editing: RwSignal<Option<AccountId>>, id: AccountId) {
    if editing.get() == Some(id) {
        editing.set(None);
    } else {
        editing.set(Some(id));
    }
}

fn edit_panel(api: AppApi, id: AccountId) -> impl IntoView {
    let ledger = api.ledger();
    let Some((name, opening, currency)) = ledger.with_untracked(|ledger| {
        ledger.account(id).ok().map(|account| {
            (
                account.name().to_owned(),
                account.opening_balance(),
                account.currency().as_str().to_owned(),
            )
        })
    }) else {
        return ().into_any();
    };
    let name = RwSignal::new(name);
    let opening = RwSignal::new(opening.to_string());
    let currency = RwSignal::new(currency);
    let message = RwSignal::new(None::<String>);
    view! {
        <fieldset>
            <label>
                "Название"
                <input
                    type="text"
                    prop:value=move || name.get()
                    on:input=move |event| name.set(event_target_value(&event))
                />
            </label>
            <label>
                "Начальный остаток"
                <input
                    type="number"
                    prop:value=move || opening.get()
                    on:input=move |event| opening.set(event_target_value(&event))
                />
            </label>
            <label>
                "Валюта"
                <input
                    type="text"
                    prop:value=move || currency.get()
                    on:input=move |event| currency.set(event_target_value(&event))
                />
            </label>
            <button
                type="button"
                on:click=move |_| save_account(api.clone(), id, name, opening, currency, message)
            >
                "Сохранить"
            </button>
            <p class="error">{move || message.get().unwrap_or_default()}</p>
        </fieldset>
    }
    .into_any()
}

fn save_account(
    api: AppApi,
    id: AccountId,
    name: RwSignal<String>,
    opening: RwSignal<String>,
    currency: RwSignal<String>,
    message: RwSignal<Option<String>>,
) {
    let Ok(opening_balance) = opening.get().trim().parse::<i64>() else {
        message.set(Some("Некорректный остаток".to_owned()));
        return;
    };
    api.update_account(
        id,
        UpdateAccount {
            name: Some(name.get()),
            opening_balance: Some(opening_balance),
            currency: Some(domain::Currency::new(currency.get())),
            credit_limit: None,
            note: None,
        },
        message,
    );
}
