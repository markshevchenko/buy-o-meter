use crate::clock::FixedClock;
use crate::date;
use crate::error::DomainError;

use super::{NewBorrow, NewExpense, NewIncome, NewLend, NewRepayment, NewTransfer};

#[test]
fn open_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let form = NewExpense::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.category_id, None);
    assert_eq!(form.amount, None);
    assert_eq!(form.comment, "");
    Ok(())
}

#[test]
fn open_income_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let form = NewIncome::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.category_id, None);
    assert_eq!(form.amount, None);
    assert_eq!(form.comment, "");
    Ok(())
}

#[test]
fn open_transfer_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
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

#[test]
fn open_lend_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let form = NewLend::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.counterparty, None);
    assert_eq!(form.amount, None);
    Ok(())
}

#[test]
fn open_borrow_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let form = NewBorrow::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.counterparty, None);
    assert_eq!(form.amount, None);
    Ok(())
}

#[test]
fn open_repayment_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let form = NewRepayment::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.counterparty, None);
    assert_eq!(form.direction, None);
    assert_eq!(form.amount, None);
    Ok(())
}
