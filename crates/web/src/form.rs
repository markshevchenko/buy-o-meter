use leptos::prelude::*;

use domain::{Account, Category, CreateAccount, Currency, Ledger, OperationId, OperationKind};

use crate::draft::{
    Draft, apply, load_editor, needs_credited_amount, parse_account, parse_amount, parse_category,
    parse_direction, parse_name,
};
use crate::mode::Mode;
use crate::text::{error_text, kind_from_name, kind_name, parse_date};

#[derive(Clone, Copy)]
struct Fields {
    date: RwSignal<String>,
    account: RwSignal<String>,
    category: RwSignal<String>,
    amount: RwSignal<String>,
    to_account: RwSignal<String>,
    to_amount: RwSignal<String>,
    comment: RwSignal<String>,
    counterparty: RwSignal<String>,
    direction: RwSignal<String>,
}

#[component]
pub(crate) fn Editor(
    ledger: RwSignal<Ledger>,
    mode: RwSignal<Mode>,
    editing: Option<OperationId>,
    initial_kind: OperationKind,
) -> impl IntoView {
    let loaded = ledger.with_untracked(|ledger| load_editor(ledger, editing, initial_kind));
    let kind = RwSignal::new(loaded.kind);
    let fields = fields_from(&loaded);
    let message = RwSignal::new(None::<String>);
    let title = if editing.is_some() {
        "Редактирование"
    } else {
        "Новая операция"
    };
    view! {
        <form on:submit=move |event| {
            event.prevent_default();
            save_form(ledger, mode, editing, kind, fields, message);
        }>
            <h2>{title}</h2>
            {kind_select(kind)}
            {move || fields_for(kind.get(), ledger, fields)}
            <AccountMaker ledger=ledger selected=fields.account />
            <p class="error">{move || message.get().unwrap_or_default()}</p>
            <button type="submit">"Сохранить"</button>
            {delete_button(editing, ledger, mode, message)}
            <button type="button" on:click=move |_| mode.set(Mode::Feed)>"К списку"</button>
        </form>
    }
}

fn fields_from(loaded: &crate::draft::Loaded) -> Fields {
    Fields {
        date: RwSignal::new(loaded.date.clone()),
        account: RwSignal::new(loaded.account.clone()),
        category: RwSignal::new(loaded.category.clone()),
        amount: RwSignal::new(loaded.amount.clone()),
        to_account: RwSignal::new(loaded.to_account.clone()),
        to_amount: RwSignal::new(loaded.to_amount.clone()),
        comment: RwSignal::new(loaded.comment.clone()),
        counterparty: RwSignal::new(loaded.counterparty.clone()),
        direction: RwSignal::new(loaded.direction.clone()),
    }
}

fn save_form(
    ledger: RwSignal<Ledger>,
    mode: RwSignal<Mode>,
    editing: Option<OperationId>,
    kind: RwSignal<OperationKind>,
    fields: Fields,
    message: RwSignal<Option<String>>,
) {
    let draft = match read_draft(fields) {
        Ok(draft) => draft,
        Err(error) => {
            message.set(Some(error_text(&error).to_owned()));
            return;
        }
    };
    let kind = kind.get();
    let outcome = ledger.try_update(|ledger| apply(ledger, editing, kind, draft));
    match outcome {
        Some(Ok(())) => mode.set(Mode::Feed),
        Some(Err(error)) => message.set(Some(error_text(&error).to_owned())),
        None => {}
    }
}

fn read_draft(fields: Fields) -> Result<Draft, domain::DomainError> {
    Ok(Draft {
        date: parse_date(&fields.date.get())?,
        account_id: parse_account(&fields.account.get()),
        category_id: parse_category(&fields.category.get()),
        amount: parse_amount(&fields.amount.get()),
        to_account_id: parse_account(&fields.to_account.get()),
        to_amount: parse_amount(&fields.to_amount.get()),
        comment: fields.comment.get(),
        counterparty: parse_name(&fields.counterparty.get()),
        direction: parse_direction(&fields.direction.get()),
    })
}

fn fields_for(kind: OperationKind, ledger: RwSignal<Ledger>, fields: Fields) -> AnyView {
    match kind {
        OperationKind::Expense | OperationKind::Income => {
            money_fields(ledger, fields, kind).into_any()
        }
        OperationKind::Transfer => transfer_fields(ledger, fields).into_any(),
        OperationKind::Lend | OperationKind::Borrow => debt_fields(ledger, fields).into_any(),
        OperationKind::Repayment => repayment_fields(ledger, fields).into_any(),
    }
}

fn money_fields(ledger: RwSignal<Ledger>, fields: Fields, kind: OperationKind) -> impl IntoView {
    view! {
        {date_field(fields.date)}
        {account_select(ledger, fields.account, "Счёт")}
        {category_select(ledger, fields.category, kind)}
        {labeled_input("Сумма", fields.amount, "number", "")}
        {labeled_input("Комментарий", fields.comment, "text", "quiet")}
    }
}

fn transfer_fields(ledger: RwSignal<Ledger>, fields: Fields) -> impl IntoView {
    view! {
        {date_field(fields.date)}
        {account_select(ledger, fields.account, "Откуда")}
        {account_select(ledger, fields.to_account, "Куда")}
        {labeled_input("Сумма", fields.amount, "number", "")}
        {credited_field(ledger, fields)}
        {labeled_input("Комментарий", fields.comment, "text", "quiet")}
    }
}

fn debt_fields(ledger: RwSignal<Ledger>, fields: Fields) -> impl IntoView {
    view! {
        {date_field(fields.date)}
        {account_select(ledger, fields.account, "Счёт")}
        {counterparty_input(ledger, fields.counterparty)}
        {labeled_input("Сумма", fields.amount, "number", "")}
    }
}

fn repayment_fields(ledger: RwSignal<Ledger>, fields: Fields) -> impl IntoView {
    view! {
        {date_field(fields.date)}
        {account_select(ledger, fields.account, "Счёт")}
        {counterparty_input(ledger, fields.counterparty)}
        {direction_select(fields.direction)}
        {labeled_input("Сумма", fields.amount, "number", "")}
    }
}

fn credited_field(ledger: RwSignal<Ledger>, fields: Fields) -> impl IntoView {
    move || {
        let from = fields.account.get();
        let to = fields.to_account.get();
        let show = ledger.with(|ledger| needs_credited_amount(ledger, &from, &to));
        if show {
            labeled_input("Сумма зачисления", fields.to_amount, "number", "").into_any()
        } else {
            ().into_any()
        }
    }
}

fn date_field(value: RwSignal<String>) -> impl IntoView {
    labeled_input("Дата", value, "date", "")
}

fn labeled_input(
    label: &'static str,
    value: RwSignal<String>,
    kind: &'static str,
    class: &'static str,
) -> impl IntoView {
    view! {
        <label class=class>
            {label}
            <input
                type=kind
                prop:value=move || value.get()
                on:input=move |event| value.set(event_target_value(&event))
            />
        </label>
    }
}

fn account_select(
    ledger: RwSignal<Ledger>,
    selected: RwSignal<String>,
    label: &'static str,
) -> impl IntoView {
    view! {
        <label>
            {label}
            {move || {
                let options = ledger.with(account_options);
                choice_select(selected, "Счёт", options)
            }}
        </label>
    }
}

fn category_select(
    ledger: RwSignal<Ledger>,
    selected: RwSignal<String>,
    kind: OperationKind,
) -> impl IntoView {
    view! {
        <label>
            "Категория"
            {move || {
                let options = ledger.with(|ledger| category_options(ledger, kind));
                choice_select(selected, "Категория", options)
            }}
        </label>
    }
}

fn choice_select(
    selected: RwSignal<String>,
    placeholder: &'static str,
    options: Vec<(String, String)>,
) -> impl IntoView {
    let current = selected.get();
    view! {
        <select on:change=move |event| selected.set(event_target_value(&event))>
            <option value="">{placeholder}</option>
            {options
                .into_iter()
                .map(|(id, label)| {
                    let picked = id == current;
                    view! { <option value=id selected=picked>{label}</option> }
                })
                .collect_view()}
        </select>
    }
}

fn account_options(ledger: &Ledger) -> Vec<(String, String)> {
    ledger.accounts().iter().map(account_option).collect()
}

fn account_option(account: &Account) -> (String, String) {
    (
        account.id().as_u64().to_string(),
        format!("{}, {}", account.name(), account.currency().as_str()),
    )
}

fn category_options(ledger: &Ledger, kind: OperationKind) -> Vec<(String, String)> {
    let categories = match kind {
        OperationKind::Income => ledger.categories_for_income(),
        _ => ledger.categories_for_expense(),
    };
    categories.into_iter().map(category_option).collect()
}

fn category_option(category: &Category) -> (String, String) {
    (
        category.id().as_u64().to_string(),
        category.name().to_owned(),
    )
}

fn counterparty_input(ledger: RwSignal<Ledger>, value: RwSignal<String>) -> impl IntoView {
    view! {
        <label>
            "Контрагент"
            {move || names_input(ledger, value)}
        </label>
    }
}

fn names_input(ledger: RwSignal<Ledger>, value: RwSignal<String>) -> impl IntoView {
    let names = ledger.with(|ledger| {
        ledger
            .counterparties()
            .iter()
            .map(|item| item.name().to_owned())
            .collect::<Vec<_>>()
    });
    view! {
        <input
            type="text"
            list="counterparties"
            prop:value=move || value.get()
            on:input=move |event| value.set(event_target_value(&event))
        />
        <datalist id="counterparties">
            {names
                .into_iter()
                .map(|name| view! { <option value=name /> })
                .collect_view()}
        </datalist>
    }
}

fn direction_select(direction: RwSignal<String>) -> impl IntoView {
    view! {
        <label>
            "Направление"
            {move || direction_options(direction)}
        </label>
    }
}

fn direction_options(direction: RwSignal<String>) -> impl IntoView {
    let current = direction.get();
    view! {
        <select on:change=move |event| direction.set(event_target_value(&event))>
            <option value="">"Выберите"</option>
            <option value="received" selected={current == "received"}>
                "Мне вернули"
            </option>
            <option value="given" selected={current == "given"}>
                "Я вернул"
            </option>
        </select>
    }
}

fn kind_select(kind: RwSignal<OperationKind>) -> impl IntoView {
    view! {
        <label>
            "Тип"
            {move || kind_options(kind)}
        </label>
    }
}

fn kind_options(kind: RwSignal<OperationKind>) -> impl IntoView {
    let current = kind.get();
    view! {
        <select on:change=move |event| {
            if let Some(next) = kind_from_name(&event_target_value(&event)) {
                kind.set(next);
            }
        }>
            {OperationKind::all()
                .iter()
                .copied()
                .map(|item| kind_option(item, current))
                .collect_view()}
        </select>
    }
}

fn kind_option(item: OperationKind, current: OperationKind) -> impl IntoView {
    let name = kind_name(item);
    view! { <option value=name selected={item == current}>{name}</option> }
}

fn delete_button(
    editing: Option<OperationId>,
    ledger: RwSignal<Ledger>,
    mode: RwSignal<Mode>,
    message: RwSignal<Option<String>>,
) -> AnyView {
    let Some(id) = editing else {
        return ().into_any();
    };
    view! {
        <button type="button" on:click=move |_| remove_operation(ledger, mode, id, message)>
            "Удалить"
        </button>
    }
    .into_any()
}

fn remove_operation(
    ledger: RwSignal<Ledger>,
    mode: RwSignal<Mode>,
    id: OperationId,
    message: RwSignal<Option<String>>,
) {
    let outcome = ledger.try_update(|ledger| ledger.delete_operation(id));
    match outcome {
        Some(Ok(())) => mode.set(Mode::Feed),
        Some(Err(error)) => message.set(Some(error_text(&error).to_owned())),
        None => {}
    }
}

#[component]
fn AccountMaker(ledger: RwSignal<Ledger>, selected: RwSignal<String>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let currency = RwSignal::new("RUB".to_owned());
    let opening = RwSignal::new("0".to_owned());
    let note = RwSignal::new(String::new());
    view! {
        <fieldset>
            <legend>"Новый счёт"</legend>
            {labeled_input("Название", name, "text", "")}
            {labeled_input("Валюта", currency, "text", "")}
            {labeled_input("Начальный остаток", opening, "number", "")}
            <button
                type="button"
                on:click=move |_| create_account(ledger, selected, name, currency, opening, note)
            >
                "Создать счёт"
            </button>
            <p class="error">{move || note.get()}</p>
        </fieldset>
    }
}

fn create_account(
    ledger: RwSignal<Ledger>,
    selected: RwSignal<String>,
    name: RwSignal<String>,
    currency: RwSignal<String>,
    opening: RwSignal<String>,
    note: RwSignal<String>,
) {
    let account_name = name.get().trim().to_owned();
    if account_name.is_empty() {
        note.set("Укажите название счёта".to_owned());
        return;
    }
    let Some(opening_balance) = parse_opening(&opening.get()) else {
        note.set("Некорректный остаток".to_owned());
        return;
    };
    let currency_code = currency.get().trim().to_owned();
    if currency_code.is_empty() {
        note.set("Укажите валюту".to_owned());
        return;
    }
    let created = ledger.try_update(|ledger| {
        ledger.create_account(CreateAccount {
            name: account_name,
            currency: Currency::new(currency_code),
            opening_balance,
        })
    });
    if let Some(Ok(id)) = created {
        selected.set(id.as_u64().to_string());
        name.set(String::new());
        note.set(String::new());
    }
}

fn parse_opening(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() {
        return Some(0);
    }
    text.parse().ok()
}
