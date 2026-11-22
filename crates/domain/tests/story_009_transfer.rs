use std::time::SystemTime;

mod common;

use common::create_account;
use domain::{AccountId, DomainError, FixedClock, Ledger, NewTransfer, TransferAmountFields, date};

fn assert_transfer(
    ledger: &Ledger,
    day: SystemTime,
    from_account_id: AccountId,
    to_account_id: AccountId,
    amount: i64,
    to_amount: i64,
    comment: &str,
) {
    let transfers = ledger.transfers();
    assert_eq!(transfers.len(), 1);
    let transfer = &transfers[0];
    assert_eq!(transfer.date(), day);
    assert_eq!(transfer.from_account_id(), from_account_id);
    assert_eq!(transfer.to_account_id(), to_account_id);
    assert_eq!(transfer.amount(), amount);
    assert_eq!(transfer.to_amount(), to_amount);
    assert_eq!(transfer.comment(), comment);
}

// 009 «Поля»
#[test]
fn new_transfer_opens_with_today_and_empty_fields() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let form = NewTransfer::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.from_account_id, None);
    assert_eq!(form.to_account_id, None);
    assert_eq!(form.amount, None);
    assert_eq!(form.to_amount, None);
    assert_eq!(form.comment, "");
    Ok(())
}

// 009 «Одна валюта»
#[test]
fn transfer_within_one_currency_moves_single_amount() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = create_account(&mut ledger, "Карта", "RUB", 20_000)?;
    let cash = create_account(&mut ledger, "Наличные", "RUB", 1_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(card),
        to_account_id: Some(cash),
        amount: Some(5_000),
        to_amount: None,
        comment: "на карман".into(),
    })?;

    assert_eq!(ledger.balance(card)?, 15_000);
    assert_eq!(ledger.balance(cash)?, 6_000);
    assert_transfer(&ledger, day, card, cash, 5_000, 5_000, "на карман");
    Ok(())
}

// 009 «Разные валюты»
#[test]
fn transfer_between_currencies_moves_each_amount() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = create_account(&mut ledger, "Карта", "RUB", 20_000)?;
    let dollars = create_account(&mut ledger, "Доллары", "USD", 100)?;
    let day = date(2026, 11, 22)?;

    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(card),
        to_account_id: Some(dollars),
        amount: Some(7_500),
        to_amount: Some(80),
        comment: String::new(),
    })?;

    assert_eq!(ledger.balance(card)?, 12_500);
    assert_eq!(ledger.balance(dollars)?, 180);
    assert_transfer(&ledger, day, card, dollars, 7_500, 80, "");
    Ok(())
}

// 009 «Поля сумм зависят от счетов»
#[test]
fn transfer_amount_fields_follow_account_currencies() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = create_account(&mut ledger, "Карта", "RUB", 0)?;
    let cash = create_account(&mut ledger, "Наличные", "RUB", 0)?;
    let dollars = create_account(&mut ledger, "Доллары", "USD", 0)?;

    assert_eq!(
        ledger.transfer_amount_fields(card, cash)?,
        TransferAmountFields::One
    );
    assert_eq!(
        ledger.transfer_amount_fields(card, dollars)?,
        TransferAmountFields::Two
    );
    assert_eq!(
        ledger.transfer_amount_fields(card, cash)?,
        TransferAmountFields::One
    );
    Ok(())
}

// 009 «Один и тот же счёт»
#[test]
fn transfer_to_the_same_account_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = create_account(&mut ledger, "Карта", "RUB", 20_000)?;
    let cash = create_account(&mut ledger, "Наличные", "RUB", 1_000)?;
    let day = date(2026, 11, 22)?;
    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(cash),
        to_account_id: Some(card),
        amount: Some(100),
        to_amount: None,
        comment: String::new(),
    })?;
    let saved = ledger.transfers().to_vec();

    let rejected = ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(card),
        to_account_id: Some(card),
        amount: Some(5_000),
        to_amount: None,
        comment: String::new(),
    });

    assert_eq!(rejected, Err(DomainError::SameAccount));
    assert_eq!(ledger.transfers(), saved.as_slice());
    assert_eq!(ledger.balance(card)?, 20_100);
    assert_eq!(ledger.balance(cash)?, 900);
    Ok(())
}
