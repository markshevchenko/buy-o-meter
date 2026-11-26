use crate::account::CreateAccount;
use crate::category::{Applicability, CreateCategory};
use crate::currency::Currency;
use crate::operation::{
    NewExpense, NewLend, NewRepayment, OperationDraft, OperationForm, OperationKind,
    RepaymentDirection,
};
use crate::{DomainError, Ledger, date};

fn rub_account(ledger: &mut Ledger, opening_balance: i64) -> Result<crate::AccountId, DomainError> {
    ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance,
        credit_limit: 0,
        note: String::new(),
    })
}

#[test]
fn balance_draft_is_not_listed() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = rub_account(&mut ledger, 1_000)?;
    ledger.add_operation(OperationDraft::Expense {
        account_id,
        amount: 100,
    })?;

    assert!(ledger.operations().is_empty());
    assert_eq!(ledger.balance(account_id)?, 900);
    Ok(())
}

#[test]
fn renaming_counterparty_on_lend_keeps_previous_name() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = rub_account(&mut ledger, 10_000)?;
    let day = date(2026, 11, 20)?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(account_id),
        counterparty: Some("Вася".into()),
        amount: Some(100),
    })?;
    let id = ledger.operations()[0].id();
    let mut form = match ledger.operation_form(id)? {
        OperationForm::Lend(form) => form,
        _ => panic!("lend"),
    };
    form.counterparty = Some("Петя".into());
    ledger.update_operation(id, OperationForm::Lend(form))?;

    let names: Vec<_> = ledger
        .counterparties()
        .iter()
        .map(|counterparty| counterparty.name())
        .collect();
    assert_eq!(names, ["Вася", "Петя"]);
    Ok(())
}

#[test]
fn editing_amount_keeps_the_same_day_slot() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = rub_account(&mut ledger, 10_000)?;
    let category_id = ledger.create_category(CreateCategory {
        name: "Продукты".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let day = date(2026, 11, 20)?;
    for amount in [100, 200] {
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(amount),
            comment: String::new(),
        })?;
    }
    let first = ledger.operations()[0].id();
    let mut form = match ledger.operation_form(first)? {
        OperationForm::Expense(form) => form,
        _ => panic!("expense"),
    };
    form.amount = Some(150);
    ledger.update_operation(first, OperationForm::Expense(form))?;

    let ids: Vec<_> = ledger.operations().iter().map(|entry| entry.id()).collect();
    assert_eq!(ids, [first, ledger.operations()[1].id()]);
    assert_eq!(ledger.expenses()[0].amount(), 150);
    assert_eq!(ledger.expenses()[1].amount(), 200);
    Ok(())
}

#[test]
fn editing_repayment_does_not_restore_deleted_counterparty() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = rub_account(&mut ledger, 10_000)?;
    let day = date(2026, 11, 20)?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(account_id),
        counterparty: Some("Вася".into()),
        amount: Some(1_000),
    })?;
    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(account_id),
        counterparty: Some("Вася".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(1_000),
    })?;
    let counterparty_id = ledger.counterparties()[0].id();
    ledger.delete_counterparty(counterparty_id)?;

    let repayment_id = ledger
        .operations()
        .iter()
        .find(|entry| entry.kind() == OperationKind::Repayment)
        .map(|entry| entry.id())
        .unwrap_or_else(|| panic!("repayment"));
    let mut form = match ledger.operation_form(repayment_id)? {
        OperationForm::Repayment(form) => form,
        _ => panic!("repayment"),
    };
    form.date = date(2026, 11, 21)?;
    ledger.update_operation(repayment_id, OperationForm::Repayment(form))?;

    assert!(ledger.counterparties().is_empty());
    Ok(())
}

#[test]
fn replacing_lend_that_backs_repayments_does_not_report_negative_remaining()
-> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = rub_account(&mut ledger, 20_000)?;
    let day = date(2026, 11, 20)?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(account_id),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(account_id),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(8_000),
    })?;
    let lend_id = ledger.operations()[0].id();
    let mut form = match ledger
        .operation_form(lend_id)?
        .switch_kind(OperationKind::Repayment)
    {
        OperationForm::Repayment(form) => form,
        _ => panic!("repayment"),
    };
    form.direction = Some(RepaymentDirection::Received);
    let err = ledger.update_operation(lend_id, OperationForm::Repayment(form));

    assert_eq!(err, Err(DomainError::SubsequentRepaymentsDoNotMatch));
    assert_eq!(ledger.lends()[0].amount(), 10_000);
    assert_eq!(ledger.repayments()[0].amount(), 8_000);
    Ok(())
}
