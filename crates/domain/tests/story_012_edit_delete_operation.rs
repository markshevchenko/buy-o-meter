mod common;

use common::{create_account, create_top};
use domain::{
    AccountId, Applicability, CategoryId, Debt, DomainError, Ledger, MissingRepaymentFields,
    NewExpense, NewLend, NewRepayment, OperationEntry, OperationForm, OperationId, OperationKind,
    RepaymentDirection, date,
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

fn operations_on_day(ledger: &Ledger, day: SystemTime) -> Vec<OperationEntry> {
    ledger
        .operations()
        .into_iter()
        .filter(|op| op.date() == day)
        .collect()
}

fn operation_amount(ledger: &Ledger, id: OperationId) -> Result<i64, DomainError> {
    let amount = match ledger.operation_form(id)? {
        OperationForm::Expense(f) => f.amount,
        OperationForm::Income(f) => f.amount,
        OperationForm::Transfer(f) => f.amount,
        OperationForm::Lend(f) => f.amount,
        OperationForm::Borrow(f) => f.amount,
        OperationForm::Repayment(f) => f.amount,
    };
    Ok(amount.unwrap_or_else(|| panic!("amount")))
}

fn id_on_day(ledger: &Ledger, day: SystemTime, kind: OperationKind) -> OperationId {
    for operation in operations_on_day(ledger, day) {
        if operation.kind() == kind {
            return operation.id();
        }
    }
    panic!("no {kind:?} on the day");
}

fn day_slot(ledger: &Ledger, day: SystemTime, id: OperationId) -> usize {
    for (index, operation) in operations_on_day(ledger, day).iter().enumerate() {
        if operation.id() == id {
            return index;
        }
    }
    panic!("operation is missing from the day");
}

fn assert_expense_moved(
    ledger: &Ledger,
    new_day: SystemTime,
    new_account: AccountId,
    new_category: CategoryId,
    old_account: AccountId,
    old_category: CategoryId,
) -> Result<(), DomainError> {
    let updated = ledger.expenses();
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].date(), new_day);
    assert_eq!(updated[0].account_id(), new_account);
    assert_eq!(updated[0].category_id(), new_category);
    assert_eq!(updated[0].amount(), 1_100);
    assert_eq!(updated[0].comment(), "стало");
    assert_eq!(ledger.balance(old_account)?, 16_000);
    assert_eq!(ledger.balance(new_account)?, 3_900);
    assert_eq!(ledger.expense_total(old_category)?, 0);
    assert_eq!(ledger.expense_total(new_category)?, 1_100);
    Ok(())
}

fn rename_lend(ledger: &mut Ledger, day: SystemTime, name: &str) -> Result<(), DomainError> {
    let lend_id = id_on_day(ledger, day, OperationKind::Lend);
    let mut lend_form = match ledger.operation_form(lend_id)? {
        OperationForm::Lend(lend) => lend,
        _ => panic!("expected lend form"),
    };
    lend_form.counterparty = Some(name.into());
    ledger.update_operation(lend_id, OperationForm::Lend(lend_form))
}

fn assert_income_keeps_shared_fields(
    form: &OperationForm,
    day: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: &str,
) {
    match form {
        OperationForm::Income(income) => {
            assert_eq!(income.date, day);
            assert_eq!(income.account_id, Some(account_id));
            assert_eq!(income.category_id, Some(category_id));
            assert_eq!(income.amount, Some(amount));
            assert_eq!(income.comment, comment);
        }
        _ => panic!("expected income form after switch"),
    }
}

fn assert_repayment_needs_counterparty(
    form: &OperationForm,
    day: SystemTime,
    account_id: AccountId,
    amount: i64,
) {
    match form {
        OperationForm::Repayment(repayment) => {
            assert_eq!(repayment.date, day);
            assert_eq!(repayment.account_id, Some(account_id));
            assert_eq!(repayment.amount, Some(amount));
            assert_eq!(repayment.counterparty, None);
            assert_eq!(repayment.direction, None);
        }
        _ => panic!("expected repayment form after switch"),
    }
}

fn assert_expense_became_income(
    ledger: &Ledger,
    day: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: &str,
) -> Result<(), DomainError> {
    assert_eq!(ledger.expenses().len(), 1);
    assert_eq!(ledger.expenses()[0].amount(), 100);
    let incomes = ledger.incomes();
    assert_eq!(incomes.len(), 1);
    assert_eq!(incomes[0].date(), day);
    assert_eq!(incomes[0].account_id(), account_id);
    assert_eq!(incomes[0].category_id(), category_id);
    assert_eq!(incomes[0].amount(), amount);
    assert_eq!(incomes[0].comment(), comment);
    assert_eq!(ledger.balance(account_id)?, 10_000 - 100 + amount);
    assert_eq!(ledger.expense_total(category_id)?, 100);
    assert_eq!(ledger.income_total(category_id)?, amount);
    Ok(())
}

fn amounts_on_day(ledger: &Ledger, day: SystemTime) -> Result<Vec<i64>, DomainError> {
    operations_on_day(ledger, day)
        .iter()
        .map(|op| operation_amount(ledger, op.id()))
        .collect()
}

// 012 «Редактирование»
#[test]
fn updating_expense_amount_recalculates_balance_and_category_total() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day = date(2026, 11, 20)?;

    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(1_250),
        comment: String::new(),
    })?;

    assert_eq!(ledger.balance(account_id)?, 8_750);
    assert_eq!(ledger.expense_total(category_id)?, 1_250);

    let expense_id = operations_on_day(&ledger, day)[0].id();
    let mut form = match ledger.operation_form(expense_id)? {
        OperationForm::Expense(expense) => expense,
        _ => panic!("expected expense form"),
    };
    form.amount = Some(1_520);
    ledger.update_operation(expense_id, OperationForm::Expense(form))?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 1);
    assert_eq!(expenses[0].amount(), 1_520);
    assert_eq!(ledger.balance(account_id)?, 8_480);
    assert_eq!(ledger.expense_total(category_id)?, 1_520);

    Ok(())
}

// 012 «Любые поля»
#[test]
fn update_preserves_changed_expense_fields_and_lend_counterparty() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let old_account = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let new_account = create_account(&mut ledger, "Карта", "RUB", 5_000)?;
    let old_category = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let new_category = create_top(&mut ledger, "Транспорт", Applicability::Expense)?;
    let old_day = date(2026, 11, 18)?;
    let new_day = date(2026, 11, 22)?;

    ledger.add_expense(NewExpense {
        date: old_day,
        account_id: Some(old_account),
        category_id: Some(old_category),
        amount: Some(800),
        comment: "было".into(),
    })?;
    assert_eq!(ledger.balance(old_account)?, 19_200);
    assert_eq!(ledger.expense_total(old_category)?, 800);
    assert_eq!(ledger.expense_total(new_category)?, 0);

    ledger.add_lend(NewLend {
        date: old_day,
        account_id: Some(old_account),
        counterparty: Some("Вася".into()),
        amount: Some(4_000),
    })?;
    assert_single_debt(&ledger.debts_to_me(), "Вася", "RUB", 4_000);
    assert_eq!(ledger.balance(old_account)?, 15_200);

    let expense_id = id_on_day(&ledger, old_day, OperationKind::Expense);
    ledger.update_operation(
        expense_id,
        OperationForm::Expense(NewExpense {
            date: new_day,
            account_id: Some(new_account),
            category_id: Some(new_category),
            amount: Some(1_100),
            comment: "стало".into(),
        }),
    )?;
    assert_expense_moved(
        &ledger,
        new_day,
        new_account,
        new_category,
        old_account,
        old_category,
    )?;

    rename_lend(&mut ledger, old_day, "Петя")?;

    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 4_000);
    assert_no_debt(&ledger.debts_to_me(), "Вася", "RUB");
    assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");

    Ok(())
}

// 012 «Смена типа»
#[test]
fn expense_can_become_income_preserving_day_slot_and_rejecting_incomplete_repayment()
-> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Подарки", Applicability::Both)?;
    let day = date(2026, 11, 20)?;
    let amount = 2_500;
    let comment = "ошибка типа";

    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(amount),
        comment: comment.into(),
    })?;
    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(100),
        comment: String::new(),
    })?;

    let expense_id = operations_on_day(&ledger, day)[0].id();
    let slot_before = day_slot(&ledger, day, expense_id);
    assert_eq!(slot_before, 0);

    let original = ledger.operation_form(expense_id)?;
    let income_form = original.clone().switch_kind(OperationKind::Income);
    assert_income_keeps_shared_fields(&income_form, day, account_id, category_id, amount, comment);

    let repayment_form = original.switch_kind(OperationKind::Repayment);
    assert_repayment_needs_counterparty(&repayment_form, day, account_id, amount);

    assert_eq!(
        ledger.update_operation(expense_id, repayment_form),
        Err(DomainError::MissingRepaymentFields(
            MissingRepaymentFields {
                account: false,
                counterparty: true,
                direction: true,
                amount: false,
            }
        ))
    );
    assert_eq!(ledger.expenses().len(), 2);
    assert_eq!(ledger.expenses()[0].amount(), amount);

    ledger.update_operation(expense_id, income_form)?;
    assert_expense_became_income(&ledger, day, account_id, category_id, amount, comment)?;

    let ops_on_day = operations_on_day(&ledger, day);
    let slot_after = day_slot(&ledger, day, expense_id);
    assert_eq!(slot_after, slot_before);
    assert_eq!(ops_on_day[slot_after].date(), day);
    assert_eq!(ops_on_day[slot_after].kind(), OperationKind::Income);

    Ok(())
}

// 012 «Смена даты»
#[test]
fn moving_operation_to_another_day_reorders_numbers_within_each_day() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 50_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day20 = date(2026, 11, 20)?;
    let day21 = date(2026, 11, 21)?;

    for amount in [100, 200, 300] {
        ledger.add_expense(NewExpense {
            date: day20,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(amount),
            comment: String::new(),
        })?;
    }
    for amount in [400, 500] {
        ledger.add_expense(NewExpense {
            date: day21,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(amount),
            comment: String::new(),
        })?;
    }

    let second_on_20 = operations_on_day(&ledger, day20)[1].id();
    let mut form = match ledger.operation_form(second_on_20)? {
        OperationForm::Expense(expense) => expense,
        _ => panic!("expected expense form"),
    };
    form.date = day21;
    ledger.update_operation(second_on_20, OperationForm::Expense(form))?;

    assert_eq!(amounts_on_day(&ledger, day20)?, vec![100, 300]);
    assert_eq!(amounts_on_day(&ledger, day21)?, vec![400, 500, 200]);

    Ok(())
}

// 012 «Удаление»
#[test]
fn deleting_middle_operation_shifts_later_entries_and_recalculates_totals()
-> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 30_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day = date(2026, 11, 20)?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(account_id),
        counterparty: Some("Олег".into()),
        amount: Some(5_000),
    })?;
    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(1_250),
        comment: String::new(),
    })?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(account_id),
        counterparty: Some("Игорь".into()),
        amount: Some(3_000),
    })?;

    assert_eq!(amounts_on_day(&ledger, day)?, vec![5_000, 1_250, 3_000]);
    assert_eq!(ledger.expense_total(category_id)?, 1_250);
    assert_single_debt(&ledger.debts_to_me(), "Олег", "RUB", 5_000);
    assert_single_debt(&ledger.debts_to_me(), "Игорь", "RUB", 3_000);
    let balance_before = ledger.balance(account_id)?;

    let expense_id = operations_on_day(&ledger, day)[1].id();
    ledger.delete_operation(expense_id)?;

    assert_eq!(amounts_on_day(&ledger, day)?, vec![5_000, 3_000]);
    assert!(ledger.expenses().is_empty());
    assert_eq!(ledger.expense_total(category_id)?, 0);
    assert_eq!(ledger.balance(account_id)?, balance_before + 1_250);
    assert_single_debt(&ledger.debts_to_me(), "Олег", "RUB", 5_000);
    assert_single_debt(&ledger.debts_to_me(), "Игорь", "RUB", 3_000);

    Ok(())
}

// 012 «Долги»
#[test]
fn deleting_received_repayment_restores_debt_and_reduces_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
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
        amount: Some(3_000),
    })?;

    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 7_000);
    assert_eq!(ledger.balance(account_id)?, 13_000);
    assert_eq!(ledger.repayments().len(), 1);

    let repayment_id = id_on_day(&ledger, day, OperationKind::Repayment);
    ledger.delete_operation(repayment_id)?;

    assert_single_debt(&ledger.debts_to_me(), "Петя", "RUB", 10_000);
    assert!(ledger.repayments().is_empty());
    assert_eq!(ledger.balance(account_id)?, 10_000);

    Ok(())
}

// 012 «Правка долга не должна ломать возвраты»
#[test]
fn lend_change_blocked_when_subsequent_repayments_exceed_new_principal() -> Result<(), DomainError>
{
    let day = date(2026, 11, 20)?;
    let cash = |ledger: &mut Ledger| create_account(ledger, "Наличные", "RUB", 20_000);

    let mut delete_ledger = Ledger::new();
    let delete_cash = cash(&mut delete_ledger)?;
    delete_ledger.add_lend(NewLend {
        date: day,
        account_id: Some(delete_cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    delete_ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(delete_cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(8_000),
    })?;
    let lend_id = operations_on_day(&delete_ledger, day)[0].id();
    assert_eq!(
        delete_ledger.delete_operation(lend_id),
        Err(DomainError::SubsequentRepaymentsDoNotMatch)
    );

    let mut reduce_ledger = Ledger::new();
    let reduce_cash = cash(&mut reduce_ledger)?;
    reduce_ledger.add_lend(NewLend {
        date: day,
        account_id: Some(reduce_cash),
        counterparty: Some("Петя".into()),
        amount: Some(10_000),
    })?;
    reduce_ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(reduce_cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(8_000),
    })?;
    let reduce_lend_id = operations_on_day(&reduce_ledger, day)[0].id();
    let mut lend_form = match reduce_ledger.operation_form(reduce_lend_id)? {
        OperationForm::Lend(lend) => lend,
        _ => panic!("expected lend form"),
    };
    lend_form.amount = Some(5_000);
    assert_eq!(
        reduce_ledger.update_operation(reduce_lend_id, OperationForm::Lend(lend_form)),
        Err(DomainError::SubsequentRepaymentsDoNotMatch)
    );

    assert_single_debt(&delete_ledger.debts_to_me(), "Петя", "RUB", 2_000);
    assert_eq!(delete_ledger.lends()[0].amount(), 10_000);
    assert_eq!(delete_ledger.repayments()[0].amount(), 8_000);
    assert_eq!(delete_ledger.balance(delete_cash)?, 18_000);

    assert_single_debt(&reduce_ledger.debts_to_me(), "Петя", "RUB", 2_000);
    assert_eq!(reduce_ledger.lends()[0].amount(), 10_000);
    assert_eq!(reduce_ledger.repayments()[0].amount(), 8_000);
    assert_eq!(reduce_ledger.balance(reduce_cash)?, 18_000);

    Ok(())
}
