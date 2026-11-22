use std::time::{Duration, SystemTime};

use domain::{
    AccountId, Applicability, CategoryId, CreateAccount, CreateCategory, Currency, DomainError,
    FixedClock, Ledger, MissingFields, NewIncome,
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

// 008 «Поля»
#[test]
fn new_income_opens_with_today_and_empty_fields() -> Result<(), DomainError> {
    let today = midnight(0);
    let form = NewIncome::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.category_id, None);
    assert_eq!(form.amount, None);
    assert_eq!(form.comment, "");
    Ok(())
}

// 008 «Выбор категории»
#[test]
fn income_categories_include_income_and_both_and_accept_parent_or_child() -> Result<(), DomainError>
{
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let salary = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let bonus = create_child(&mut ledger, "Премия", salary, None)?;
    let deduction = create_child(
        &mut ledger,
        "Удержание",
        salary,
        Some(Applicability::Expense),
    )?;
    let gifts = create_top(&mut ledger, "Подарки", Applicability::Both)?;
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;

    assert_eq!(
        ledger.category(bonus)?.applicability(),
        Applicability::Income
    );
    assert_eq!(
        ledger.category(deduction)?.applicability(),
        Applicability::Expense
    );
    assert_eq!(ledger.category(bonus)?.parent_id(), Some(salary));

    let offered = ledger.categories_for_income();
    assert!(offered.iter().any(|category| category.id() == salary));
    assert!(offered.iter().any(|category| category.id() == bonus));
    assert!(offered.iter().any(|category| category.id() == gifts));
    assert!(offered.iter().all(|category| category.id() != food));
    assert!(offered.iter().all(|category| category.id() != deduction));
    assert!(offered.iter().all(|category| {
        matches!(
            category.applicability(),
            Applicability::Income | Applicability::Both
        )
    }));
    assert_eq!(
        offered
            .iter()
            .find(|category| category.id() == bonus)
            .map(|category| category.parent_id()),
        Some(Some(salary))
    );

    ledger.add_income(NewIncome {
        date: midnight(0),
        account_id: Some(account_id),
        category_id: Some(bonus),
        amount: Some(500),
        comment: String::new(),
    })?;
    ledger.add_income(NewIncome {
        date: midnight(0),
        account_id: Some(account_id),
        category_id: Some(salary),
        amount: Some(1_000),
        comment: String::new(),
    })?;

    let incomes = ledger.incomes();
    assert_eq!(incomes.len(), 2);
    assert_eq!(incomes[0].category_id(), bonus);
    assert_eq!(incomes[0].amount(), 500);
    assert_eq!(incomes[1].category_id(), salary);
    assert_ne!(incomes[1].category_id(), bonus);
    assert_eq!(incomes[1].amount(), 1_000);
    assert_eq!(ledger.income_total(bonus)?, 500);
    assert_eq!(ledger.income_total(salary)?, 1_500);
    Ok(())
}

// 008 «Сохранение»
#[test]
fn saving_incomes_assigns_day_numbers_and_increases_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let date = midnight(0);

    ledger.add_income(NewIncome {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(300),
        comment: String::new(),
    })?;
    ledger.add_income(NewIncome {
        date,
        account_id: Some(account_id),
        category_id: Some(category_id),
        amount: Some(700),
        comment: String::new(),
    })?;

    let incomes = ledger.incomes();
    assert_eq!(incomes.len(), 2);
    assert_eq!(incomes[0].date(), date);
    assert_eq!(incomes[0].amount(), 300);
    assert_eq!(incomes[1].date(), date);
    assert_eq!(incomes[1].amount(), 700);
    assert_eq!(ledger.balance(account_id)?, 10_700);
    Ok(())
}

// 008 «Обязательные поля»
#[test]
fn income_missing_account_category_or_amount_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let date = midnight(0);

    let missing_account = ledger.add_income(NewIncome {
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

    let missing_category = ledger.add_income(NewIncome {
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

    let missing_amount = ledger.add_income(NewIncome {
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

    assert!(ledger.incomes().is_empty());
    assert_eq!(ledger.balance(account_id)?, 10_000);
    Ok(())
}
