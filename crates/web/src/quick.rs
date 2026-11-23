use leptos::prelude::*;

use domain::{AccountBalance, Debt, Ledger};

use crate::mode::Mode;

pub(crate) fn quick_page(ledger: RwSignal<Ledger>, mode: RwSignal<Mode>) -> impl IntoView {
    view! {
        <button type="button" on:click=move |_| mode.set(Mode::Feed)>"К списку"</button>
        {move || quick_body(ledger)}
    }
}

fn quick_body(ledger: RwSignal<Ledger>) -> impl IntoView {
    let access = ledger.with(Ledger::quick_access);
    view! {
        <h2>"Счета"</h2>
        {balance_lines(access.accounts())}
        <h2>"Мои долги"</h2>
        {debt_lines(access.my_debts())}
        <h2>"Долги мне"</h2>
        {debt_lines(access.debts_to_me())}
    }
}

fn balance_lines(accounts: &[AccountBalance]) -> AnyView {
    if accounts.is_empty() {
        return empty_line();
    }
    accounts
        .iter()
        .map(|account| view! { <p>{balance_text(account)}</p> })
        .collect_view()
        .into_any()
}

fn debt_lines(debts: &[Debt]) -> AnyView {
    if debts.is_empty() {
        return empty_line();
    }
    debts
        .iter()
        .map(|debt| view! { <p>{debt_text(debt)}</p> })
        .collect_view()
        .into_any()
}

fn empty_line() -> AnyView {
    view! { <p class="quiet">"Нет"</p> }.into_any()
}

fn balance_text(account: &AccountBalance) -> String {
    format!(
        "{} — {} {}",
        account.name(),
        account.amount(),
        account.currency().as_str()
    )
}

fn debt_text(debt: &Debt) -> String {
    format!(
        "{} — {} {}",
        debt.counterparty_name(),
        debt.amount(),
        debt.currency().as_str()
    )
}
