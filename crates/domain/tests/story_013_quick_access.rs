mod common;

use common::{create_account, create_top};
use domain::{
    AccountBalance, AccountId, Debt, DomainError, Ledger, NewBorrow, NewExpense, NewLend,
    NewRepayment, OperationForm, OperationId, OperationKind, RepaymentDirection, date,
};
use std::time::SystemTime;

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

fn assert_account_line(
    row: &AccountBalance,
    id: AccountId,
    name: &str,
    currency: &str,
    amount: i64,
) {
    assert_eq!(row.id(), id);
    assert_eq!(row.name(), name);
    assert_eq!(row.currency().as_str(), currency);
    assert_eq!(row.amount(), amount);
}

fn operations_on_day(ledger: &Ledger, day: SystemTime) -> Vec<domain::OperationEntry> {
    ledger
        .operations()
        .into_iter()
        .filter(|op| op.date() == day)
        .collect()
}

fn id_on_day(ledger: &Ledger, day: SystemTime, kind: OperationKind) -> OperationId {
    for operation in operations_on_day(ledger, day) {
        if operation.kind() == kind {
            return operation.id();
        }
    }
    panic!("no {kind:?} on the day");
}

// 013 «Остатки по счетам»
#[test]
fn quick_access_lists_each_account_balance_in_its_currency() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let card = create_account(&mut ledger, "Карта", "RUB", 5_000)?;
    let dollars = create_account(&mut ledger, "Доллары", "USD", 1_000)?;
    let category = create_top(&mut ledger, "Продукты", domain::Applicability::Expense)?;
    let day = date(2026, 11, 22)?;

    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(cash),
        category_id: Some(category),
        amount: Some(3_000),
        comment: String::new(),
    })?;

    let snapshot = ledger.quick_access();
    let accounts = snapshot.accounts();
    assert_eq!(accounts.len(), 3);
    assert_account_line(&accounts[0], cash, "Наличные", "RUB", 17_000);
    assert_account_line(&accounts[1], card, "Карта", "RUB", 5_000);
    assert_account_line(&accounts[2], dollars, "Доллары", "USD", 1_000);

    Ok(())
}

// 013 «Мои долги»
#[test]
fn quick_access_my_debts_shows_borrow_to_counterparty() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = create_account(&mut ledger, "Карта", "RUB", 1_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(card),
        counterparty: Some("Маша".into()),
        amount: Some(3_000),
    })?;

    let snapshot = ledger.quick_access();
    assert!(snapshot.debts_to_me().is_empty());
    assert_single_debt(snapshot.my_debts(), "Маша", "RUB", 3_000);

    Ok(())
}

// 013 «Долги мне»
#[test]
fn quick_access_debts_to_me_shows_each_currency_separately() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let dollars = create_account(&mut ledger, "Доллары", "USD", 1_000)?;
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
    assert_single_debt(snapshot.debts_to_me(), "Петя", "RUB", 5_000);
    assert_single_debt(snapshot.debts_to_me(), "Петя", "USD", 100);

    Ok(())
}

// 013 «Погашенные долги»
#[test]
fn quick_access_omits_fully_repaid_counterparties() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let day = date(2026, 11, 22)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(5_000),
    })?;
    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(5_000),
    })?;

    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(3_000),
    })?;
    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        direction: Some(RepaymentDirection::Given),
        amount: Some(3_000),
    })?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Олег".into()),
        amount: Some(1_000),
    })?;
    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Вася".into()),
        amount: Some(2_000),
    })?;

    let snapshot = ledger.quick_access();
    assert_no_debt(snapshot.debts_to_me(), "Петя", "RUB");
    assert_no_debt(snapshot.my_debts(), "Маша", "RUB");
    assert_single_debt(snapshot.debts_to_me(), "Олег", "RUB", 1_000);
    assert_single_debt(snapshot.my_debts(), "Вася", "RUB", 2_000);

    Ok(())
}

// 013 «Актуальность»
#[test]
fn quick_access_reflects_add_update_and_delete_operations() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let day = date(2026, 11, 20)?;

    let before = ledger.quick_access();
    assert_account_line(&before.accounts()[0], cash, "Наличные", "RUB", 10_000);
    assert!(before.my_debts().is_empty());
    assert!(before.debts_to_me().is_empty());

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Олег".into()),
        amount: Some(5_000),
    })?;

    let after_add = ledger.quick_access();
    assert_account_line(&after_add.accounts()[0], cash, "Наличные", "RUB", 5_000);
    assert_single_debt(after_add.debts_to_me(), "Олег", "RUB", 5_000);

    let lend_id = id_on_day(&ledger, day, OperationKind::Lend);
    let mut lend_form = match ledger.operation_form(lend_id)? {
        OperationForm::Lend(lend) => lend,
        _ => panic!("expected lend form"),
    };
    lend_form.amount = Some(7_000);
    ledger.update_operation(lend_id, OperationForm::Lend(lend_form))?;

    let after_update = ledger.quick_access();
    assert_account_line(&after_update.accounts()[0], cash, "Наличные", "RUB", 3_000);
    assert_single_debt(after_update.debts_to_me(), "Олег", "RUB", 7_000);

    ledger.delete_operation(lend_id)?;

    let after_delete = ledger.quick_access();
    assert_account_line(&after_delete.accounts()[0], cash, "Наличные", "RUB", 10_000);
    assert!(after_delete.debts_to_me().is_empty());

    Ok(())
}
