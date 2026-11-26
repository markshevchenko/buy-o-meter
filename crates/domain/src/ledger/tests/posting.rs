use super::super::Ledger;
use crate::account::{AccountId, CreateAccount};
use crate::category::{CategoryId, CreateCategory};
use crate::currency::Currency;
use crate::error::{DomainError, MissingFields};
use crate::operation::{NewExpense, NewIncome};
use crate::{add_days, date};

#[test]
fn saved_expense_keeps_date_account_category_amount_and_comment() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let category_id = ledger.create_category(CreateCategory {
        name: "Продукты".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let date = date(2026, 11, 22)?;

    ledger.add_expense(NewExpense {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(450),
        comment: "Хлеб".into(),
    })?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 1);
    assert_eq!(expenses[0].date(), date);
    assert_eq!(expenses[0].account_id(), account_id);
    assert_eq!(expenses[0].category_id(), category_id);
    assert_eq!(expenses[0].amount(), 450);
    assert_eq!(expenses[0].comment(), "Хлеб");
    Ok(())
}

fn ledger_with_cash_and_groceries() -> Result<(Ledger, AccountId, CategoryId), DomainError> {
    ledger_with_cash("Продукты", crate::category::Applicability::Expense)
}

fn assert_expense_rejected(missing: MissingFields) -> Result<(), DomainError> {
    let (mut ledger, account_id, category_id) = ledger_with_cash_and_groceries()?;
    let err = ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: super::omit_if(missing.account, account_id),
        category_id: super::omit_if(missing.category, category_id),
        amount: super::omit_if(missing.amount, 100),
        comment: String::new(),
    });
    assert_eq!(err, Err(DomainError::MissingFields(missing)));
    assert!(ledger.expenses().is_empty());
    assert_eq!(ledger.balance(account_id)?, 10_000);
    assert_eq!(ledger.expense_total(category_id)?, 0);
    Ok(())
}

#[test]
fn expense_without_account_category_or_amount_is_rejected() -> Result<(), DomainError> {
    assert_expense_rejected(MissingFields {
        account: true,
        category: false,
        amount: false,
    })?;
    assert_expense_rejected(MissingFields {
        account: false,
        category: true,
        amount: false,
    })?;
    assert_expense_rejected(MissingFields {
        account: false,
        category: false,
        amount: true,
    })?;
    Ok(())
}

#[test]
fn saved_expense_reduces_only_its_account_balance() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let rub = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let usd = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 100,
        credit_limit: 0,
        note: String::new(),
    })?;
    let category_id = ledger.create_category(CreateCategory {
        name: "Продукты".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;

    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(usd),
        category_id: Some(category_id),
        amount: Some(15),
        comment: String::new(),
    })?;

    assert_eq!(ledger.balance(usd)?, 85);
    assert_eq!(ledger.balance(rub)?, 10_000);
    assert_eq!(ledger.expenses()[0].amount(), 15);
    assert_eq!(ledger.account(usd)?.currency().as_str(), "USD");
    Ok(())
}

#[test]
fn saved_income_increases_only_its_account_balance() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let rub = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let usd = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 100,
        credit_limit: 0,
        note: String::new(),
    })?;
    let category_id = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;

    ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(usd),
        category_id: Some(category_id),
        amount: Some(15),
        comment: String::new(),
    })?;

    assert_eq!(ledger.balance(usd)?, 115);
    assert_eq!(ledger.balance(rub)?, 10_000);
    assert_eq!(ledger.incomes()[0].amount(), 15);
    assert_eq!(ledger.account(usd)?.currency().as_str(), "USD");
    Ok(())
}

#[test]
fn expense_on_parent_counts_only_toward_that_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let groceries = ledger.create_category(CreateCategory {
        name: "Продукты".into(),
        parent_id: Some(food),
        applicability: None,
    })?;

    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(1_200),
        comment: String::new(),
    })?;

    assert_eq!(ledger.expenses()[0].category_id(), food);
    assert_eq!(ledger.expense_total(food)?, 1_200);
    assert_eq!(ledger.expense_total(groceries)?, 0);
    Ok(())
}

#[test]
fn income_on_parent_counts_only_toward_that_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    let bonus = ledger.create_category(CreateCategory {
        name: "Премия".into(),
        parent_id: Some(salary),
        applicability: None,
    })?;

    ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(salary),
        amount: Some(1_200),
        comment: String::new(),
    })?;

    assert_eq!(ledger.incomes()[0].category_id(), salary);
    assert_eq!(ledger.income_total(salary)?, 1_200);
    assert_eq!(ledger.income_total(bonus)?, 0);
    Ok(())
}

#[test]
fn expense_on_earlier_date_appends_to_that_day() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let category_id = ledger.create_category(CreateCategory {
        name: "Продукты".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let yesterday = date(2026, 11, 21)?;
    let today = add_days(yesterday, 1)?;
    let expense = |date, amount, comment: &str| NewExpense {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(amount),
        comment: comment.into(),
    };

    ledger.add_expense(expense(yesterday, 100, "вчера"))?;
    ledger.add_expense(expense(today, 200, "сегодня"))?;
    ledger.add_expense(expense(yesterday, 300, "забытый"))?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 3);
    assert_eq!(expenses[0].date(), yesterday);
    assert_eq!(expenses[0].amount(), 100);
    assert_eq!(expenses[1].date(), yesterday);
    assert_eq!(expenses[1].amount(), 300);
    assert_eq!(expenses[1].comment(), "забытый");
    assert_eq!(expenses[2].date(), today);
    assert_eq!(expenses[2].amount(), 200);
    Ok(())
}

#[test]
fn income_on_earlier_date_appends_to_that_day() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let category_id = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    let yesterday = date(2026, 11, 21)?;
    let today = add_days(yesterday, 1)?;
    let income = |date, amount, comment: &str| NewIncome {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(amount),
        comment: comment.into(),
    };

    ledger.add_income(income(yesterday, 100, "вчера"))?;
    ledger.add_income(income(today, 200, "сегодня"))?;
    ledger.add_income(income(yesterday, 300, "забытый"))?;

    let incomes = ledger.incomes();
    assert_eq!(incomes.len(), 3);
    assert_eq!(incomes[0].date(), yesterday);
    assert_eq!(incomes[0].amount(), 100);
    assert_eq!(incomes[1].date(), yesterday);
    assert_eq!(incomes[1].amount(), 300);
    assert_eq!(incomes[1].comment(), "забытый");
    assert_eq!(incomes[2].date(), today);
    assert_eq!(incomes[2].amount(), 200);
    Ok(())
}

#[test]
fn child_expense_counts_in_the_parent_total() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    })?;
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let groceries = ledger.create_category(CreateCategory {
        name: "Продукты".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(groceries),
        amount: Some(400),
        comment: String::new(),
    })?;

    assert_eq!(ledger.expense_total(groceries)?, 400);
    assert_eq!(ledger.expense_total(food)?, 400);
    Ok(())
}

#[test]
fn child_income_counts_in_the_parent_total() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    let bonus = ledger.create_category(CreateCategory {
        name: "Премия".into(),
        parent_id: Some(salary),
        applicability: None,
    })?;
    ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(bonus),
        amount: Some(400),
        comment: String::new(),
    })?;

    assert_eq!(ledger.income_total(bonus)?, 400);
    assert_eq!(ledger.income_total(salary)?, 400);
    Ok(())
}

#[test]
fn expense_total_rejects_mixed_currencies() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let rub = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    })?;
    let usd = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    })?;
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(rub),
        category_id: Some(food),
        amount: Some(1_000),
        comment: String::new(),
    })?;
    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(usd),
        category_id: Some(food),
        amount: Some(15),
        comment: String::new(),
    })?;

    assert_eq!(ledger.expenses().len(), 2);
    assert_eq!(ledger.balance(rub)?, -1_000);
    assert_eq!(ledger.balance(usd)?, -15);
    assert_eq!(
        ledger.expense_total(food),
        Err(DomainError::MixedCurrencies)
    );
    Ok(())
}

#[test]
fn income_total_rejects_mixed_currencies() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let rub = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    })?;
    let usd = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(rub),
        category_id: Some(salary),
        amount: Some(1_000),
        comment: String::new(),
    })?;
    ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(usd),
        category_id: Some(salary),
        amount: Some(15),
        comment: String::new(),
    })?;

    assert_eq!(ledger.incomes().len(), 2);
    assert_eq!(ledger.balance(rub)?, 1_000);
    assert_eq!(ledger.balance(usd)?, 15);
    assert_eq!(
        ledger.income_total(salary),
        Err(DomainError::MixedCurrencies)
    );
    Ok(())
}

#[test]
fn expense_with_unknown_account_or_category_is_rejected() -> Result<(), DomainError> {
    use crate::account::AccountId;
    use crate::category::CategoryId;

    let (mut ledger, account_id, category_id) = ledger_with_cash_and_groceries()?;
    let unknown_account = ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(AccountId::from_u64(99)),
        category_id: Some(category_id),
        amount: Some(100),
        comment: String::new(),
    });
    assert_eq!(unknown_account, Err(DomainError::AccountNotFound));
    assert!(ledger.expenses().is_empty());
    assert_eq!(ledger.balance(account_id)?, 10_000);

    let unknown_category = ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(CategoryId::from_u64(99)),
        amount: Some(100),
        comment: String::new(),
    });
    assert_eq!(unknown_category, Err(DomainError::CategoryNotFound));
    assert!(ledger.expenses().is_empty());
    assert_eq!(ledger.expense_total(category_id)?, 0);
    Ok(())
}

#[test]
fn income_with_unknown_account_or_category_is_rejected() -> Result<(), DomainError> {
    use crate::account::AccountId;
    use crate::category::CategoryId;

    let (mut ledger, account_id, category_id) = ledger_with_cash_and_salary()?;
    let unknown_account = ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(AccountId::from_u64(99)),
        category_id: Some(category_id),
        amount: Some(100),
        comment: String::new(),
    });
    assert_eq!(unknown_account, Err(DomainError::AccountNotFound));
    assert!(ledger.incomes().is_empty());
    assert_eq!(ledger.balance(account_id)?, 10_000);

    let unknown_category = ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(CategoryId::from_u64(99)),
        amount: Some(100),
        comment: String::new(),
    });
    assert_eq!(unknown_category, Err(DomainError::CategoryNotFound));
    assert!(ledger.incomes().is_empty());
    assert_eq!(ledger.income_total(category_id)?, 0);
    Ok(())
}

#[test]
fn saved_income_keeps_date_account_category_amount_and_comment() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let category_id = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    let date = date(2026, 11, 22)?;

    ledger.add_income(NewIncome {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(450),
        comment: "Оклад".into(),
    })?;

    let incomes = ledger.incomes();
    assert_eq!(incomes.len(), 1);
    assert_eq!(incomes[0].date(), date);
    assert_eq!(incomes[0].account_id(), account_id);
    assert_eq!(incomes[0].category_id(), category_id);
    assert_eq!(incomes[0].amount(), 450);
    assert_eq!(incomes[0].comment(), "Оклад");
    Ok(())
}

fn ledger_with_cash_and_salary() -> Result<(Ledger, AccountId, CategoryId), DomainError> {
    ledger_with_cash("Зарплата", crate::category::Applicability::Income)
}

fn ledger_with_cash(
    category_name: &str,
    applicability: crate::category::Applicability,
) -> Result<(Ledger, AccountId, CategoryId), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
        credit_limit: 0,
        note: String::new(),
    })?;
    let category_id = ledger.create_category(CreateCategory {
        name: category_name.into(),
        parent_id: None,
        applicability: Some(applicability),
    })?;
    Ok((ledger, account_id, category_id))
}

fn assert_income_rejected(missing: MissingFields) -> Result<(), DomainError> {
    let (mut ledger, account_id, category_id) = ledger_with_cash_and_salary()?;
    let err = ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: super::omit_if(missing.account, account_id),
        category_id: super::omit_if(missing.category, category_id),
        amount: super::omit_if(missing.amount, 100),
        comment: String::new(),
    });
    assert_eq!(err, Err(DomainError::MissingFields(missing)));
    assert!(ledger.incomes().is_empty());
    assert_eq!(ledger.balance(account_id)?, 10_000);
    assert_eq!(ledger.income_total(category_id)?, 0);
    Ok(())
}

#[test]
fn income_without_account_category_or_amount_is_rejected() -> Result<(), DomainError> {
    assert_income_rejected(MissingFields {
        account: true,
        category: false,
        amount: false,
    })?;
    assert_income_rejected(MissingFields {
        account: false,
        category: true,
        amount: false,
    })?;
    assert_income_rejected(MissingFields {
        account: false,
        category: false,
        amount: true,
    })?;
    Ok(())
}

#[test]
fn expense_without_any_account_is_rejected() -> Result<(), DomainError> {
    use crate::account::AccountId;
    use crate::category::CategoryId;

    let mut ledger = Ledger::new();
    let err = ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(AccountId::from_u64(1)),
        category_id: Some(CategoryId::from_u64(1)),
        amount: Some(1),
        comment: String::new(),
    });
    assert_eq!(err, Err(DomainError::AccountRequired));
    assert!(ledger.expenses().is_empty());
    Ok(())
}
