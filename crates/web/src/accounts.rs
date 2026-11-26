use leptos::prelude::*;

use domain::{Account, AccountId};

use crate::api::AppApi;

pub(crate) fn accounts_page(api: AppApi) -> impl IntoView {
    let ledger = api.ledger();
    let editing = RwSignal::new(None::<AccountId>);
    let show_create = RwSignal::new(false);
    view! {
        {move || {
            let accounts: Vec<Account> = ledger.with(|ledger| ledger.accounts().to_vec());
            if accounts.is_empty() {
                empty_accounts_view(api.clone(), show_create).into_any()
            } else {
                accounts_table_view(api.clone(), editing, show_create).into_any()
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
    show_create: RwSignal<bool>,
) -> impl IntoView {
    let ledger = api.ledger();
    let api_for_rows = api.clone();
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
                        .map(|account| account_row(api_for_rows.clone(), editing, account))
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
            } else if let Some(id) = editing.get() {
                edit_account::dialog(api.clone(), id, editing).into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

mod create_account {
    use domain::CreateAccount;

    use crate::api::AppApi;
    use leptos::prelude::*;

    #[rustfmt::skip]
    const CURRENCIES: [(&str, &str); 3] = [
        ("RUB", "Рубль"),
        ("USD", "Доллар"),
        ("EUR", "Евро"),
    ];

    #[derive(Clone, Copy)]
    struct Fields {
        name: RwSignal<String>,
        currency: RwSignal<String>,
        opening: RwSignal<String>,
        credit_limit: RwSignal<String>,
        note: RwSignal<String>,
    }

    pub(super) fn dialog(api: AppApi, show: RwSignal<bool>) -> impl IntoView {
        let fields = Fields {
            name: RwSignal::new(String::new()),
            currency: RwSignal::new(CURRENCIES[0].0.to_owned()),
            opening: RwSignal::new("0".to_owned()),
            credit_limit: RwSignal::new(String::new()),
            note: RwSignal::new(String::new()),
        };
        let message = RwSignal::new(None::<String>);
        view! {
            <dialog class="modal" open=true>
                <h2>"Новый счёт"</h2>
                {labeled_field("Название", fields.name, "text")}
                {currency_field(fields.currency)}
                {labeled_field("Начальный остаток", fields.opening, "number")}
                {labeled_field("Кредитный лимит", fields.credit_limit, "number")}
                {labeled_field("Комментарий", fields.note, "text")}
                <p class="error">{move || message.get().unwrap_or_default()}</p>
                <button
                    type="button"
                    on:click=move |_| submit(api.clone(), show, fields, message)
                >
                    "Ок"
                </button>
                <button type="button" on:click=move |_| show.set(false)>"Отмена"</button>
            </dialog>
        }
    }

    fn labeled_field(
        label: &'static str,
        value: RwSignal<String>,
        kind: &'static str,
    ) -> impl IntoView {
        view! {
            <label>
                {label}
                <input
                    type=kind
                    prop:value=move || value.get()
                    on:input=move |event| value.set(event_target_value(&event))
                />
            </label>
        }
    }

    fn currency_field(currency: RwSignal<String>) -> impl IntoView {
        let current = currency.get_untracked();
        view! {
            <label>
                "Валюта"
                <select on:change=move |event| currency.set(event_target_value(&event))>
                    {CURRENCIES
                        .iter()
                        .map(|(code, label)| {
                            let code = *code;
                            view! { <option value=code selected={code == current}>{*label}</option> }
                        })
                        .collect_view()}
                </select>
            </label>
        }
    }

    fn submit(
        api: AppApi,
        show: RwSignal<bool>,
        fields: Fields,
        message: RwSignal<Option<String>>,
    ) {
        let input = match account_input(
            &fields.name.get(),
            &fields.currency.get(),
            &fields.opening.get(),
            &fields.credit_limit.get(),
            &fields.note.get(),
        ) {
            Ok(input) => input,
            Err(error) => {
                message.set(Some(error.to_owned()));
                return;
            }
        };
        api.add_account(input, message, show);
    }

    fn account_input(
        name: &str,
        currency: &str,
        opening: &str,
        credit_limit: &str,
        note: &str,
    ) -> Result<CreateAccount, &'static str> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Укажите название счёта");
        }
        let opening_balance = parse_amount(opening).ok_or("Некорректный остаток")?;
        let credit_limit = parse_credit_limit(credit_limit)?;
        Ok(CreateAccount {
            name: name.to_owned(),
            currency: domain::Currency::new(currency),
            opening_balance,
            credit_limit,
            note: note.to_owned(),
        })
    }

    fn parse_amount(text: &str) -> Option<i64> {
        let text = text.trim();
        if text.is_empty() {
            return Some(0);
        }
        text.parse().ok()
    }

    fn parse_credit_limit(text: &str) -> Result<i64, &'static str> {
        let limit = parse_amount(text).ok_or("Некорректный кредитный лимит")?;
        if limit < 0 {
            return Err("Кредитный лимит не может быть отрицательным");
        }
        Ok(limit)
    }

    #[cfg(test)]
    mod tests {
        use domain::Currency;

        use super::{CURRENCIES, account_input};

        #[test]
        fn name_is_trimmed() {
            let input = account_input("  Наличные  ", "RUB", "0", "0", "").unwrap();
            assert_eq!(input.name, "Наличные");
        }

        #[test]
        fn empty_name_is_rejected() {
            assert!(account_input("   ", "RUB", "0", "0", "").is_err());
        }

        #[test]
        fn negative_opening_balance_is_allowed() {
            let input = account_input("Карта", "RUB", "-500", "0", "").unwrap();
            assert_eq!(input.opening_balance, -500);
        }

        #[test]
        fn empty_amounts_become_zero() {
            let input = account_input("Карта", "RUB", "", "", "").unwrap();
            assert_eq!(input.opening_balance, 0);
            assert_eq!(input.credit_limit, 0);
        }

        #[test]
        fn broken_opening_balance_is_rejected() {
            assert!(account_input("Карта", "RUB", "abc", "0", "").is_err());
        }

        #[test]
        fn negative_credit_limit_is_rejected() {
            assert!(account_input("Карта", "RUB", "0", "-1", "").is_err());
        }

        #[test]
        fn broken_credit_limit_is_rejected() {
            assert!(account_input("Карта", "RUB", "0", "abc", "").is_err());
        }

        #[test]
        fn currency_is_stored_as_code() {
            let input = account_input("Карта", "USD", "0", "0", "").unwrap();
            assert_eq!(input.currency, Currency::new("USD"));
        }

        #[test]
        fn note_is_kept() {
            let input = account_input("Карта", "RUB", "0", "0", "Основная").unwrap();
            assert_eq!(input.note, "Основная");
        }

        #[test]
        fn rub_is_the_default_currency() {
            assert_eq!(CURRENCIES[0].0, "RUB");
        }
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
    }
}

fn toggle(editing: RwSignal<Option<AccountId>>, id: AccountId) {
    if editing.get() == Some(id) {
        editing.set(None);
    } else {
        editing.set(Some(id));
    }
}

mod edit_account {
    use domain::UpdateAccount;

    use crate::api::AppApi;
    use leptos::prelude::*;

    #[rustfmt::skip]
    const CURRENCIES: [(&str, &str); 3] = [
        ("RUB", "Рубль"),
        ("USD", "Доллар"),
        ("EUR", "Евро"),
    ];

    #[derive(Clone, Copy)]
    struct Fields {
        name: RwSignal<String>,
        currency: RwSignal<String>,
        opening: RwSignal<String>,
        credit_limit: RwSignal<String>,
        note: RwSignal<String>,
    }

    pub(super) fn dialog(
        api: AppApi,
        id: domain::AccountId,
        editing: RwSignal<Option<domain::AccountId>>,
    ) -> impl IntoView {
        ().into_any()
    }

    fn labeled_field(
        label: &'static str,
        value: RwSignal<String>,
        kind: &'static str,
    ) -> impl IntoView {
        ().into_any()
    }

    fn currency_field(currency: RwSignal<String>) -> impl IntoView {
        ().into_any()
    }

    fn submit(
        api: AppApi,
        id: domain::AccountId,
        editing: RwSignal<Option<domain::AccountId>>,
        fields: Fields,
        message: RwSignal<Option<String>>,
    ) {
        todo!()
    }

    fn account_input(
        name: &str,
        currency: &str,
        opening: &str,
        credit_limit: &str,
        note: &str,
    ) -> Result<UpdateAccount, &'static str> {
        todo!()
    }

    fn parse_amount(text: &str) -> Option<i64> {
        todo!()
    }

    fn parse_credit_limit(text: &str) -> Result<i64, &'static str> {
        todo!()
    }
}
