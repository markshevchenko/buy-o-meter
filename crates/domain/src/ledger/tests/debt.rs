use super::super::Ledger;
use crate::account::CreateAccount;
use crate::currency::Currency;
use crate::error::DomainError;
use crate::operation::{NewBorrow, NewLend, NewRepayment, OperationDraft, RepaymentDirection};
use crate::{add_days, date};

#[test]
fn saving_lend_adds_new_counterparty() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;

    assert!(ledger.counterparties().is_empty());

    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(1_000),
    })?;

    assert_eq!(ledger.counterparties().len(), 1);
    assert_eq!(ledger.counterparties()[0].name(), "Маша");
    Ok(())
}

#[test]
fn lend_keeps_counterparty_name_after_list_grows() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;

    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");

    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(500),
    })?;
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
    Ok(())
}

#[test]
fn counterparty_with_lend_cannot_be_renamed_or_deleted() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;

    let id = ledger.counterparties()[0].id();
    let before = ledger.counterparties().to_vec();
    assert_eq!(
        ledger.rename_counterparty(id, "Пётр".into()),
        Err(DomainError::CounterpartyHasDebts)
    );
    assert_eq!(
        ledger.delete_counterparty(id),
        Err(DomainError::CounterpartyHasDebts)
    );
    assert_eq!(ledger.counterparties(), before.as_slice());
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
    assert_eq!(ledger.lends()[0].amount(), 10_000);
    Ok(())
}

#[test]
fn lending_reduces_account_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    assert_eq!(ledger.balance(cash)?, 10_000);
    Ok(())
}

#[test]
fn lending_adds_debt_to_me_in_account_currency() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;

    let debts = ledger.debts_to_me();
    assert_eq!(debts.len(), 1);
    assert_eq!(debts[0].counterparty_name(), "Петя");
    assert_eq!(debts[0].currency().as_str(), "RUB");
    assert_eq!(debts[0].amount(), 10_000);
    assert!(ledger.my_debts().is_empty());
    Ok(())
}

#[test]
fn second_lend_to_same_name_keeps_one_counterparty() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 50_000,
    })?;
    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(3_000),
    })?;

    assert_eq!(ledger.counterparties().len(), 1);
    assert_eq!(ledger.counterparties()[0].name(), "Петя");
    Ok(())
}

#[test]
fn borrowing_increases_account_balance_and_my_debt() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 1_000,
    })?;
    ledger.add_borrow(NewBorrow {
        date: date(2026, 11, 22)?,
        account_id: Some(card),
        counterparty: Some("Маша".into()),
        amount: Some(5_000),
    })?;

    assert_eq!(ledger.balance(card)?, 6_000);
    assert_eq!(ledger.my_debts().len(), 1);
    assert_eq!(ledger.my_debts()[0].counterparty_name(), "Маша");
    assert_eq!(ledger.my_debts()[0].currency().as_str(), "RUB");
    assert_eq!(ledger.my_debts()[0].amount(), 5_000);
    assert!(ledger.debts_to_me().is_empty());
    assert_eq!(ledger.borrows()[0].counterparty_name(), "Маша");
    Ok(())
}

#[test]
fn counterparty_with_borrow_cannot_be_renamed_or_deleted() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 1_000,
    })?;
    ledger.add_borrow(NewBorrow {
        date: date(2026, 11, 22)?,
        account_id: Some(card),
        counterparty: Some("Маша".into()),
        amount: Some(5_000),
    })?;

    let id = ledger.counterparties()[0].id();
    let before = ledger.counterparties().to_vec();
    assert_eq!(
        ledger.rename_counterparty(id, "Мария".into()),
        Err(DomainError::CounterpartyHasDebts)
    );
    assert_eq!(
        ledger.delete_counterparty(id),
        Err(DomainError::CounterpartyHasDebts)
    );
    assert_eq!(ledger.counterparties(), before.as_slice());
    assert_eq!(ledger.borrows()[0].counterparty_name(), "Маша");
    assert_eq!(ledger.borrows()[0].amount(), 5_000);
    Ok(())
}

#[test]
fn received_repayment_reduces_debt_to_me_and_increases_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
    ledger.add_lend(NewLend {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;

    ledger.add_repayment(NewRepayment {
        date: date(2026, 11, 22)?,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(2_000),
    })?;

    assert_eq!(ledger.balance(cash)?, 12_000);
    let debts = ledger.debts_to_me();
    assert_eq!(debts.len(), 1);
    assert_eq!(debts[0].counterparty_name(), "Петя");
    assert_eq!(debts[0].currency().as_str(), "RUB");
    assert_eq!(debts[0].amount(), 8_000);
    assert!(ledger.my_debts().is_empty());
    assert_eq!(ledger.repayments()[0].counterparty_name(), "Петя");
    assert_eq!(
        ledger.repayments()[0].direction(),
        RepaymentDirection::Received
    );
    assert_eq!(ledger.repayments()[0].amount(), 2_000);
    Ok(())
}

#[test]
fn given_repayment_reduces_my_debt_and_leaves_debt_to_me() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 50_000,
    })?;
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

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Given),
        amount: Some(2_000),
    })?;

    assert_eq!(ledger.balance(cash)?, 41_000);
    assert_eq!(ledger.my_debts().len(), 1);
    assert_eq!(ledger.my_debts()[0].counterparty_name(), "Петя");
    assert_eq!(ledger.my_debts()[0].currency().as_str(), "RUB");
    assert_eq!(ledger.my_debts()[0].amount(), 1_000);
    assert_eq!(ledger.debts_to_me().len(), 1);
    assert_eq!(ledger.debts_to_me()[0].amount(), 10_000);
    assert_eq!(ledger.repayments()[0].counterparty_name(), "Петя");
    assert_eq!(
        ledger.repayments()[0].direction(),
        RepaymentDirection::Given
    );

    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Given),
        amount: Some(1_000),
    })?;

    assert!(ledger.my_debts().is_empty());
    assert_eq!(ledger.debts_to_me()[0].amount(), 10_000);
    assert_eq!(ledger.balance(cash)?, 40_000);
    Ok(())
}

#[test]
fn repayment_above_remaining_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
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
    assert_eq!(ledger.debts_to_me()[0].amount(), 5_000);
    assert_eq!(ledger.balance(cash)?, balance_before);
    assert!(ledger.repayments().is_empty());
    Ok(())
}

#[test]
fn given_repayment_without_my_debt_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
    let day = date(2026, 11, 22)?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(3_000),
    })?;
    let balance_before = ledger.balance(cash)?;

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
    assert_eq!(ledger.debts_to_me()[0].amount(), 3_000);
    assert!(ledger.my_debts().is_empty());
    assert!(ledger.repayments().is_empty());
    Ok(())
}

#[test]
fn repayment_in_another_currency_is_no_debt() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 100_000,
    })?;
    let dollars = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 1_000,
    })?;
    let day = date(2026, 11, 22)?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    let rub_before = ledger.balance(cash)?;
    let usd_before = ledger.balance(dollars)?;

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
    assert_eq!(ledger.debts_to_me().len(), 1);
    assert_eq!(ledger.debts_to_me()[0].amount(), 10_000);
    assert_eq!(ledger.debts_to_me()[0].currency().as_str(), "RUB");
    assert_eq!(ledger.balance(cash)?, rub_before);
    assert_eq!(ledger.balance(dollars)?, usd_before);
    assert!(ledger.repayments().is_empty());
    Ok(())
}

#[test]
fn repaid_counterparty_can_be_renamed_and_deleted() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
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
    assert!(ledger.debts_to_me().is_empty());

    let id = ledger.counterparties()[0].id();
    ledger.rename_counterparty(id, "Пётр".into())?;
    assert_eq!(ledger.counterparties()[0].name(), "Пётр");
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
    assert_eq!(ledger.repayments()[0].counterparty_name(), "Петя");

    ledger.delete_counterparty(id)?;
    assert!(ledger.counterparties().is_empty());
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
    assert_eq!(ledger.repayments()[0].counterparty_name(), "Петя");
    Ok(())
}

#[test]
fn repayment_on_earlier_date_appends_to_that_day() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
    let yesterday = date(2026, 11, 21)?;
    let today = add_days(yesterday, 1)?;
    ledger.add_lend(NewLend {
        date: yesterday,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    let repay = |when, amount| NewRepayment {
        date: when,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(amount),
    };

    ledger.add_repayment(repay(yesterday, 100))?;
    ledger.add_repayment(repay(today, 200))?;
    ledger.add_repayment(repay(yesterday, 300))?;

    let repayments = ledger.repayments();
    assert_eq!(repayments.len(), 3);
    assert_eq!(repayments[0].date(), yesterday);
    assert_eq!(repayments[0].amount(), 100);
    assert_eq!(repayments[1].date(), yesterday);
    assert_eq!(repayments[1].amount(), 300);
    assert_eq!(repayments[2].date(), today);
    assert_eq!(repayments[2].amount(), 200);
    Ok(())
}

#[test]
fn quick_access_lists_accounts_in_creation_order_with_their_balances() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
    })?;
    let dollars = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 1_000,
    })?;
    ledger.add_operation(OperationDraft::Expense {
        account_id: cash,
        amount: 3_000,
    })?;

    let snapshot = ledger.quick_access();
    let accounts = snapshot.accounts();
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].id(), cash);
    assert_eq!(accounts[0].name(), "Наличные");
    assert_eq!(accounts[0].currency().as_str(), "RUB");
    assert_eq!(accounts[0].amount(), ledger.balance(cash)?);
    assert_eq!(accounts[1].id(), dollars);
    assert_eq!(accounts[1].name(), "Доллары");
    assert_eq!(accounts[1].currency().as_str(), "USD");
    assert_eq!(accounts[1].amount(), ledger.balance(dollars)?);
    Ok(())
}

#[test]
fn quick_access_my_debts_matches_ledger_my_debts() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 1_000,
    })?;
    let day = date(2026, 11, 22)?;
    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(card),
        counterparty: Some("Маша".into()),
        amount: Some(3_000),
    })?;

    let snapshot = ledger.quick_access();
    assert!(snapshot.debts_to_me().is_empty());
    assert_eq!(snapshot.my_debts(), ledger.my_debts().as_slice());
    Ok(())
}

#[test]
fn quick_access_debts_to_me_matches_ledger_debts_to_me() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 100_000,
    })?;
    let dollars = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 1_000,
    })?;
    let day = date(2026, 11, 22)?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(5_000),
    })?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(dollars),
        counterparty: Some("Петя".into()),
        amount: Some(100),
    })?;

    let snapshot = ledger.quick_access();
    assert!(snapshot.my_debts().is_empty());
    assert_eq!(snapshot.debts_to_me(), ledger.debts_to_me().as_slice());
    Ok(())
}
