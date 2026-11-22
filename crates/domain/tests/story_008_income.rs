mod common;

use common::{create_account, create_child, create_top};
use domain::{
    AccountId, Applicability, CategoryId, DomainError, FixedClock, Ledger, MissingFields,
    NewIncome, date,
};

// 008 «Поля»
#[test]
fn new_income_opens_with_today_and_empty_fields() -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    let form = NewIncome::open(&FixedClock::new(today));

    assert_eq!(form.date, today);
    assert_eq!(form.account_id, None);
    assert_eq!(form.category_id, None);
    assert_eq!(form.amount, None);
    assert_eq!(form.comment, "");
    Ok(())
}

struct IncomeChoice {
    account_id: AccountId,
    salary: CategoryId,
    bonus: CategoryId,
    deduction: CategoryId,
    gifts: CategoryId,
    food: CategoryId,
}

fn income_choice(ledger: &mut Ledger) -> Result<IncomeChoice, DomainError> {
    let account_id = create_account(ledger, "Наличные", "RUB", 10_000)?;
    let salary = create_top(ledger, "Зарплата", Applicability::Income)?;
    let bonus = create_child(ledger, "Премия", salary, None)?;
    let deduction = create_child(ledger, "Удержание", salary, Some(Applicability::Expense))?;
    let gifts = create_top(ledger, "Подарки", Applicability::Both)?;
    let food = create_top(ledger, "Питание", Applicability::Expense)?;
    Ok(IncomeChoice {
        account_id,
        salary,
        bonus,
        deduction,
        gifts,
        food,
    })
}

fn assert_offered_income_categories(
    ledger: &Ledger,
    choice: &IncomeChoice,
) -> Result<(), DomainError> {
    assert_eq!(
        ledger.category(choice.bonus)?.applicability(),
        Applicability::Income
    );
    assert_eq!(
        ledger.category(choice.deduction)?.applicability(),
        Applicability::Expense
    );
    assert_eq!(
        ledger.category(choice.bonus)?.parent_id(),
        Some(choice.salary)
    );

    let offered = ledger.categories_for_income();
    assert!(
        offered
            .iter()
            .any(|category| category.id() == choice.salary)
    );
    assert!(offered.iter().any(|category| category.id() == choice.bonus));
    assert!(offered.iter().any(|category| category.id() == choice.gifts));
    assert!(offered.iter().all(|category| category.id() != choice.food));
    assert!(
        offered
            .iter()
            .all(|category| category.id() != choice.deduction)
    );
    assert!(offered.iter().all(|category| {
        matches!(
            category.applicability(),
            Applicability::Income | Applicability::Both
        )
    }));
    assert_eq!(
        offered
            .iter()
            .find(|category| category.id() == choice.bonus)
            .map(|category| category.parent_id()),
        Some(Some(choice.salary))
    );
    Ok(())
}

fn assert_income_on_parent_and_child(
    ledger: &mut Ledger,
    choice: &IncomeChoice,
) -> Result<(), DomainError> {
    let today = date(2026, 11, 22)?;
    ledger.add_income(NewIncome {
        date: today,
        account_id: Some(choice.account_id),
        category_id: Some(choice.bonus),
        amount: Some(500),
        comment: String::new(),
    })?;
    ledger.add_income(NewIncome {
        date: today,
        account_id: Some(choice.account_id),
        category_id: Some(choice.salary),
        amount: Some(1_000),
        comment: String::new(),
    })?;

    let incomes = ledger.incomes();
    assert_eq!(incomes.len(), 2);
    assert_eq!(incomes[0].category_id(), choice.bonus);
    assert_eq!(incomes[0].amount(), 500);
    assert_eq!(incomes[1].category_id(), choice.salary);
    assert_ne!(incomes[1].category_id(), choice.bonus);
    assert_eq!(incomes[1].amount(), 1_000);
    assert_eq!(ledger.income_total(choice.bonus)?, 500);
    assert_eq!(ledger.income_total(choice.salary)?, 1_500);
    Ok(())
}

// 008 «Выбор категории»
#[test]
fn income_categories_include_income_and_both_and_accept_parent_or_child() -> Result<(), DomainError>
{
    let mut ledger = Ledger::new();
    let choice = income_choice(&mut ledger)?;
    assert_offered_income_categories(&ledger, &choice)?;
    assert_income_on_parent_and_child(&mut ledger, &choice)
}

// 008 «Сохранение»
#[test]
fn saving_incomes_assigns_day_numbers_and_increases_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let date = date(2026, 11, 22)?;

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
    assert_eq!(ledger.balance(account_id)?, 11_000);
    Ok(())
}

// 008 «Обязательные поля»
#[test]
fn income_missing_account_category_or_amount_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let category_id = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let date = date(2026, 11, 22)?;

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
