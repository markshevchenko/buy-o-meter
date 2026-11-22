use std::time::{Duration, SystemTime};

use domain::{
    AccountId, Applicability, CategoryId, CreateAccount, CreateCategory, Currency, DomainError,
    FixedClock, Ledger, MissingFields, NewExpense,
};

fn midnight(days_after_epoch: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(days_after_epoch * 24 * 60 * 60)
}

fn create_account(
    ledger: &mut Ledger,
    name: &str,
    currency: &str,
    opening_balance: i64,
) -> Result<AccountId, DomainError> {
    ledger.create_account(CreateAccount {
        name: name.into(),
        currency: Currency::new(currency),
        opening_balance,
    })
}

fn create_top(
    ledger: &mut Ledger,
    name: &str,
    applicability: Applicability,
) -> Result<CategoryId, DomainError> {
    ledger.create_category(CreateCategory {
        name: name.into(),
        parent_id: None,
        applicability: Some(applicability),
    })
}

fn create_child(
    ledger: &mut Ledger,
    name: &str,
    parent_id: CategoryId,
    applicability: Option<Applicability>,
) -> Result<CategoryId, DomainError> {
    ledger.create_category(CreateCategory {
        name: name.into(),
        parent_id: Some(parent_id),
        applicability,
    })
}

// 007 «Поля»
#[test]
fn saved_expense_stores_date_account_category_amount_and_comment() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let date = midnight(0);

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

// 007 «Значения по умолчанию»
#[test]
fn new_expense_opens_with_today_and_empty_fields() -> Result<(), DomainError> {
    let today = midnight(0);
    let form = NewExpense::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.category_id, None);
    assert_eq!(form.amount, None);
    assert_eq!(form.comment, "");
    Ok(())
}

// 007 «Выбор категории»
#[test]
fn expense_categories_include_expense_and_both_with_subcategories() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let groceries = create_child(&mut ledger, "Продукты", food, None)?;
    let cashback = create_child(&mut ledger, "Кэшбэк", food, Some(Applicability::Income))?;
    let gifts = create_top(&mut ledger, "Подарки", Applicability::Both)?;
    let salary = create_top(&mut ledger, "Зарплата", Applicability::Income)?;

    assert_eq!(
        ledger.category(groceries)?.applicability(),
        Applicability::Expense
    );
    assert_eq!(
        ledger.category(cashback)?.applicability(),
        Applicability::Income
    );

    let offered = ledger.categories_for_expense();
    assert!(offered.iter().any(|category| category.id() == food));
    assert!(offered.iter().any(|category| category.id() == groceries));
    assert!(offered.iter().any(|category| category.id() == gifts));
    assert!(offered.iter().all(|category| category.id() != salary));
    assert!(offered.iter().all(|category| category.id() != cashback));
    assert!(offered.iter().all(|category| {
        matches!(
            category.applicability(),
            Applicability::Expense | Applicability::Both
        )
    }));
    assert_eq!(
        offered
            .iter()
            .find(|category| category.id() == groceries)
            .map(|category| category.parent_id()),
        Some(Some(food))
    );
    Ok(())
}

// 007 «Категория верхнего уровня»
#[test]
fn expense_on_top_level_category_does_not_use_subcategory() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let groceries = create_child(&mut ledger, "Продукты", food, None)?;
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));

    ledger.add_expense(NewExpense {
        date: midnight(0),
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(1_200),
        comment: String::new(),
    })?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 1);
    assert_eq!(expenses[0].category_id(), food);
    assert_ne!(expenses[0].category_id(), groceries);
    assert_eq!(ledger.expense_total(food)?, 1_200);
    assert_eq!(ledger.expense_total(groceries)?, 0);
    Ok(())
}

// 007 «Сохранение»
#[test]
fn saving_expenses_assigns_day_numbers_and_reduces_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let date = midnight(0);

    ledger.add_expense(NewExpense {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(300),
        comment: String::new(),
    })?;
    ledger.add_expense(NewExpense {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(700),
        comment: String::new(),
    })?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 2);
    assert_eq!(expenses[0].date(), date);
    assert_eq!(expenses[0].amount(), 300);
    assert_eq!(expenses[1].date(), date);
    assert_eq!(expenses[1].amount(), 700);
    assert_eq!(ledger.balance(account_id)?, 9_000);
    Ok(())
}

// 007 «Валюта»
#[test]
fn expense_amount_stays_in_selected_account_currency() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let rub = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let usd = create_account(&mut ledger, "Доллары", "USD", 100)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;

    ledger.add_expense(NewExpense {
        date: midnight(0),
        account_id: Some(usd),
        category_id: Some(category_id),
        amount: Some(15),
        comment: String::new(),
    })?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 1);
    assert_eq!(expenses[0].account_id(), usd);
    assert_eq!(expenses[0].amount(), 15);
    assert_eq!(ledger.balance(usd)?, 85);
    assert_eq!(ledger.account(usd)?.currency().as_str(), "USD");
    assert_eq!(ledger.balance(rub)?, 10_000);
    assert_eq!(ledger.account(rub)?.currency().as_str(), "RUB");
    Ok(())
}

// 007 «Обязательные поля»
#[test]
fn expense_missing_account_category_or_amount_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let date = midnight(0);

    let missing_account = ledger.add_expense(NewExpense {
        date,
        account_id: None,
        category_id: Some(category_id),
        amount: Some(100),
        comment: String::new(),
    });
    assert_eq!(
        missing_account,
        Err(DomainError::MissingFields(MissingFields {
            account: true,
            category: false,
            amount: false,
        }))
    );

    let missing_category = ledger.add_expense(NewExpense {
        date,
        account_id: Some(account_id),
        category_id: None,
        amount: Some(100),
        comment: String::new(),
    });
    assert_eq!(
        missing_category,
        Err(DomainError::MissingFields(MissingFields {
            account: false,
            category: true,
            amount: false,
        }))
    );

    let missing_amount = ledger.add_expense(NewExpense {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: None,
        comment: String::new(),
    });
    assert_eq!(
        missing_amount,
        Err(DomainError::MissingFields(MissingFields {
            account: false,
            category: false,
            amount: true,
        }))
    );

    assert!(ledger.expenses().is_empty());
    assert_eq!(ledger.balance(account_id)?, 10_000);
    Ok(())
}

// 007 «Комментарий необязателен»
#[test]
fn expense_with_empty_comment_is_saved() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;

    ledger.add_expense(NewExpense {
        date: midnight(0),
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(100),
        comment: String::new(),
    })?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 1);
    assert_eq!(expenses[0].comment(), "");
    Ok(())
}

// 007 «Прошедшая дата»
#[test]
fn expense_on_past_date_appends_to_that_day() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let yesterday = midnight(0);
    let today = yesterday + Duration::from_secs(24 * 60 * 60);

    ledger.add_expense(NewExpense {
        date: yesterday,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(100),
        comment: "вчера".into(),
    })?;
    ledger.add_expense(NewExpense {
        date: today,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(200),
        comment: "сегодня".into(),
    })?;
    ledger.add_expense(NewExpense {
        date: yesterday,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(300),
        comment: "забытый".into(),
    })?;

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
