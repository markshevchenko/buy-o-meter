mod common;

use common::create_account;
use domain::{
    Counterparty, CounterpartyId, Debt, DomainError, FixedClock, Ledger, NewBorrow, NewLend, date,
};

fn counterparty_id_by_name(ledger: &Ledger, name: &str) -> CounterpartyId {
    ledger
        .counterparties()
        .iter()
        .find(|c| c.name() == name)
        .map(Counterparty::id)
        .unwrap_or_else(|| panic!("counterparty {name:?} not found"))
}

fn assert_single_debt(debts: &[Debt], name: &str, currency: &str, amount: i64) {
    let matching: Vec<_> = debts
        .iter()
        .filter(|d| d.counterparty_name() == name && d.currency().as_str() == currency)
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "expected one debt for {name} {currency}, got {}",
        matching.len()
    );
    assert_eq!(matching[0].amount(), amount);
}

// 010 «Поля»
#[test]
fn lend_and_borrow_forms_open_with_today_and_empty_fields() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let clock = FixedClock::new(today);

    let lend = NewLend::open(&clock);
    assert_eq!(lend.date, today);
    assert_eq!(lend.account_id, None);
    assert_eq!(lend.counterparty, None);
    assert_eq!(lend.amount, None);

    let borrow = NewBorrow::open(&clock);
    assert_eq!(borrow.date, today);
    assert_eq!(borrow.account_id, None);
    assert_eq!(borrow.counterparty, None);
    assert_eq!(borrow.amount, None);

    Ok(())
}

// 010 «Новый контрагент»
#[test]
fn saving_operation_adds_new_counterparty_to_list() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let day = date(2026, 11, 22)?;

    assert!(!ledger.counterparties().iter().any(|c| c.name() == "Маша"));

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(1_000),
    })?;

    assert!(ledger.counterparties().iter().any(|c| c.name() == "Маша"));
    Ok(())
}

// 010 «Имя копируется в операцию»
#[test]
fn counterparty_name_is_stored_on_operation_and_unchanged_by_list_growth() -> Result<(), DomainError>
{
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;

    assert_eq!(ledger.lends().len(), 1);
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(500),
    })?;

    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
    Ok(())
}

// 010 «Контрагент с долгом»
#[test]
fn counterparty_with_debt_cannot_be_renamed_or_deleted() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;

    let petya_id = counterparty_id_by_name(&ledger, "Петя");
    let counterparties_before: Vec<_> = ledger
        .counterparties()
        .iter()
        .map(|c| (c.id(), c.name().to_owned()))
        .collect();
    let lend_before = ledger.lends().to_vec();

    assert_eq!(
        ledger.rename_counterparty(petya_id, "Пётр".into()),
        Err(DomainError::CounterpartyHasDebts)
    );
    assert_eq!(
        ledger.delete_counterparty(petya_id),
        Err(DomainError::CounterpartyHasDebts)
    );

    let counterparties_after: Vec<_> = ledger
        .counterparties()
        .iter()
        .map(|c| (c.id(), c.name().to_owned()))
        .collect();
    assert_eq!(counterparties_after, counterparties_before);
    assert_eq!(ledger.lends().len(), lend_before.len());
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
    assert_eq!(ledger.lends()[0].amount(), lend_before[0].amount());
    Ok(())
}

// 010 «Дать в долг»
#[test]
fn lending_reduces_account_balance_and_adds_debt_to_me() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;

    assert_eq!(ledger.balance(cash)?, 10_000);
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 10_000);
    assert!(ledger.my_debts().is_empty());
    Ok(())
}

// 010 «Взять в долг»
#[test]
fn borrowing_increases_account_balance_and_adds_my_debt() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = create_account(&mut ledger, "Карта", "RUB", 1_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(card),
        counterparty: Some("Маша".into()),
        amount: Some(5_000),
    })?;

    assert_eq!(ledger.balance(card)?, 6_000);
    assert_single_debt(&ledger.my_debts(), "Маша", "RUB", 5_000);
    assert!(ledger.debts_to_me().is_empty());
    Ok(())
}

// 010 «Повторный долг тому же человеку»
#[test]
fn second_lend_to_same_person_increases_debt_total() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 50_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(3_000),
    })?;

    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 13_000);
    Ok(())
}

// 010 «Валюта долга»
#[test]
fn debt_currency_matches_account_and_splits_by_currency_for_same_name() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let dollars = create_account(&mut ledger, "Доллары", "USD", 1_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(dollars),
        counterparty: Some("Петя".into()),
        amount: Some(50),
    })?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;

    assert_single_debt(&ledger.debts_to_me(), "Петя", "USD", 50);
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 10_000);
    Ok(())
}

// 010 «Встречные долги не сворачиваются»
#[test]
fn lend_and_borrow_with_same_counterparty_stay_in_separate_lists() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 50_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(3_000),
    })?;

    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 10_000);
    assert_single_debt(&ledger.my_debts(), "Петя", "RUB", 3_000);
    Ok(())
}
