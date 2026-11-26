use leptos::prelude::*;

use domain::{Account, AccountId, UpdateAccount};

use crate::api::AppApi;

pub(crate) fn accounts_page(api: AppApi) -> impl IntoView {
    let ledger = api.ledger();
    let editing = RwSignal::new(None::<AccountId>);
    let list = api.clone();
    view! {
        <div class="tree">
            {move || {
                let accounts: Vec<Account> = ledger.with(|ledger| ledger.accounts().to_vec());
                accounts
                    .into_iter()
                    .map(|account| account_row(list.clone(), editing, account))
                    .collect_view()
            }}
        </div>
        {add_form(api)}
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
        <button type="button" class="line" on:click=move |_| toggle(editing, id)>
            {move || {
                let balance = ledger.with(|ledger| ledger.balance(id).unwrap_or(0));
                format!("{name}, {currency}: {balance}")
            }}
        </button>
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
        },
        message,
    );
}

fn add_form(api: AppApi) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let currency = RwSignal::new("RUB".to_owned());
    let opening = RwSignal::new("0".to_owned());
    let message = RwSignal::new(None::<String>);
    view! {
        <fieldset>
            <legend>"Новый счёт"</legend>
            <label>
                "Название"
                <input
                    type="text"
                    prop:value=move || name.get()
                    on:input=move |event| name.set(event_target_value(&event))
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
            <label>
                "Начальный остаток"
                <input
                    type="number"
                    prop:value=move || opening.get()
                    on:input=move |event| opening.set(event_target_value(&event))
                />
            </label>
            <button
                type="button"
                on:click=move |_| add_account(api.clone(), name, currency, opening, message)
            >
                "Создать счёт"
            </button>
            <p class="error">{move || message.get().unwrap_or_default()}</p>
        </fieldset>
    }
}

fn add_account(
    api: AppApi,
    name: RwSignal<String>,
    currency: RwSignal<String>,
    opening: RwSignal<String>,
    message: RwSignal<Option<String>>,
) {
    let account_name = name.get().trim().to_owned();
    if account_name.is_empty() {
        message.set(Some("Укажите название счёта".to_owned()));
        return;
    }
    let Ok(opening_balance) = opening.get().trim().parse::<i64>() else {
        message.set(Some("Некорректный остаток".to_owned()));
        return;
    };
    let currency_code = currency.get().trim().to_owned();
    if currency_code.is_empty() {
        message.set(Some("Укажите валюту".to_owned()));
        return;
    }
    api.add_account(account_name, currency_code, opening_balance, message);
    name.set(String::new());
}
