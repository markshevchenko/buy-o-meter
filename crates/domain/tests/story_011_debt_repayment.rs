mod common;

use common::create_account;
use domain::{
    Debt, DomainError, FixedClock, Ledger, NewBorrow, NewLend, NewRepayment, RepaymentDirection,
    date,
};

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

fn assert_no_debt(debts: &[Debt], name: &str, currency: &str) {
    assert!(
        !debts
            .iter()
            .any(|d| d.counterparty_name() == name && d.currency().as_str() == currency),
        "expected no debt for {name} {currency}"
    );
}

// 011 «Поля»
#[test]
fn repayment_form_opens_with_today_and_empty_fields() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let clock = FixedClock::new(today);

    let repayment = NewRepayment::open(&clock);
    assert_eq!(repayment.date, today);
    assert_eq!(repayment.account_id, None);
    assert_eq!(repayment.counterparty, None);
    assert_eq!(repayment.direction, None);
    assert_eq!(repayment.amount, None);

    Ok(())
}

// 011 «Частичный возврат мне»
#[test]
fn sequential_received_repayments_reduce_debt_and_increase_balance() -> Result<(), DomainError> {
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

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(2_000),
    })?;
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 8_000);
    assert_eq!(ledger.balance(cash)?, 12_000);

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(3_000),
    })?;
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 5_000);
    assert_eq!(ledger.balance(cash)?, 15_000);

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(5_000),
    })?;
    assert_no_debt(&ledger.debts_to_me(), "Петя", "RUB");
    assert_eq!(ledger.balance(cash)?, 20_000);

    Ok(())
}

// 011 «Мой возврат»
#[test]
fn given_repayment_reduces_my_debt_and_account_balance() -> Result<(), DomainError> {
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

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(card),
        counterparty: Some("Маша".into()),
        direction: Some(RepaymentDirection::Given),
        amount: Some(2_000),
    })?;
    assert_single_debt(&ledger.my_debts(), "Маша", "RUB", 3_000);
    assert_eq!(ledger.balance(card)?, 4_000);

    Ok(())
}

// 011 «Встречные долги»
#[test]
fn given_repayment_to_counterparty_only_reduces_my_side() -> Result<(), DomainError> {
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

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Given),
        amount: Some(3_000),
    })?;
    assert_no_debt(&ledger.my_debts(), "Петя", "RUB");
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 10_000);

    Ok(())
}

// 011 «Полностью погашенный долг»
#[test]
fn fully_repaid_counterparty_disappears_from_debts_to_me() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(10_000),
    })?;
    assert_no_debt(&ledger.debts_to_me(), "Петя", "RUB");

    Ok(())
}

// 011 «Та же валюта»
#[test]
fn repayment_must_use_account_in_debt_currency() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let dollars = create_account(&mut ledger, "Доллары", "USD", 1_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    let rub_balance_before = ledger.balance(cash)?;
    let usd_balance_before = ledger.balance(dollars)?;

    assert_eq!(
        ledger.add_repayment(NewRepayment {
            date: day,
            account_id: Some(dollars),
            counterparty: Some("Петя".into()),
            direction: Some(RepaymentDirection::Received),
            amount: Some(1_000),
        }),
        Err(DomainError::NoDebt)
    );
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 10_000);
    assert_eq!(ledger.balance(cash)?, rub_balance_before);
    assert_eq!(ledger.balance(dollars)?, usd_balance_before);

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(1_000),
    })?;
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 9_000);
    assert_eq!(ledger.balance(cash)?, rub_balance_before + 1_000);

    Ok(())
}

// 011 «Не больше остатка долга»
#[test]
fn repayment_exceeding_debt_is_rejected_and_state_unchanged() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(5_000),
    })?;
    let balance_before = ledger.balance(cash)?;

    assert_eq!(
        ledger.add_repayment(NewRepayment {
            date: day,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            direction: Some(RepaymentDirection::Received),
            amount: Some(6_000),
        }),
        Err(DomainError::RepaymentExceedsDebt { remaining: 5_000 })
    );
    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 5_000);
    assert_eq!(ledger.balance(cash)?, balance_before);

    Ok(())
}

// 011 «Возврат без долга»
#[test]
fn given_repayment_without_my_debt_returns_no_debt_error() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(3_000),
    })?;
    let balance_before = ledger.balance(cash)?;
    assert_single_debt(&ledger.debts_to_me(), "Маша", "RUB", 3_000);
    assert!(ledger.my_debts().is_empty());

    assert_eq!(
        ledger.add_repayment(NewRepayment {
            date: day,
            account_id: Some(cash),
            counterparty: Some("Маша".into()),
            direction: Some(RepaymentDirection::Given),
            amount: Some(1_000),
        }),
        Err(DomainError::NoDebt)
    );
    assert_eq!(ledger.balance(cash)?, balance_before);
    assert_single_debt(&ledger.debts_to_me(), "Маша", "RUB", 3_000);
    assert!(ledger.my_debts().is_empty());

    Ok(())
}