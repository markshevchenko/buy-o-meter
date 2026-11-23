use super::super::Ledger;
use crate::account::CreateAccount;
use crate::category::{CategoryId, CreateCategory};
use crate::currency::Currency;
use crate::date;
use crate::error::DomainError;
use crate::operation::{NewExpense, NewIncome, Record};

#[test]
fn transfer_targets_of_missing_category_are_not_found() {
    let ledger = Ledger::new();
    let missing = CategoryId::from_u64(1);
    assert_eq!(
        ledger.transfer_targets(missing),
        Err(DomainError::CategoryNotFound)
    );
}

#[test]
fn transfer_targets_include_parent_peers_and_other_categories() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let lunches = ledger.create_category(CreateCategory {
        name: "Обеды в городе".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    let home_lunches = ledger.create_category(CreateCategory {
        name: "Обед дома".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    let transport = ledger.create_category(CreateCategory {
        name: "Транспорт".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;

    let targets = target_ids(&ledger, lunches)?;

    assert!(!targets.contains(&lunches));
    assert!(targets.contains(&food));
    assert!(targets.contains(&home_lunches));
    assert!(targets.contains(&transport));
    Ok(())
}

fn target_ids(ledger: &Ledger, id: CategoryId) -> Result<Vec<CategoryId>, DomainError> {
    Ok(ledger
        .transfer_targets(id)?
        .iter()
        .map(|category| category.id())
        .collect())
}

#[test]
fn transfer_targets_with_expenses_omit_income_only_categories() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
    })?;
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let lunches = ledger.create_category(CreateCategory {
        name: "Обеды в городе".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    let gifts = ledger.create_category(CreateCategory {
        name: "Подарки".into(),
        parent_id: None,
        applicability: Some(Applicability::Both),
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(lunches),
        amount: Some(450),
        comment: String::new(),
    })?;

    let targets = target_ids(&ledger, lunches)?;

    assert!(targets.contains(&food));
    assert!(targets.contains(&gifts));
    assert!(!targets.contains(&salary));
    Ok(())
}

#[test]
fn transfer_targets_without_expenses_keep_income_categories() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;

    let targets = target_ids(&ledger, food)?;

    assert!(targets.contains(&salary));
    Ok(())
}

#[test]
fn empty_leaf_is_deleted_without_a_transfer_target() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
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

    ledger.delete_category(groceries, None)?;

    assert!(
        !ledger
            .categories()
            .iter()
            .any(|category| category.id() == groceries)
    );
    assert_eq!(
        ledger.category(groceries),
        Err(DomainError::CategoryNotFound)
    );
    assert!(
        !ledger
            .categories_for_expense()
            .iter()
            .any(|category| category.id() == groceries)
    );
    assert!(
        !ledger
            .categories_for_income()
            .iter()
            .any(|category| category.id() == groceries)
    );
    assert_eq!(ledger.category(food)?.name(), "Питание");
    Ok(())
}

#[test]
fn category_with_subcategories_stays_even_with_a_target() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
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
    let transport = ledger.create_category(CreateCategory {
        name: "Транспорт".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(300),
        comment: String::new(),
    })?;
    let categories_before = ledger.categories().to_vec();
    let expenses_before = ledger.expenses().to_vec();

    let err = ledger.delete_category(food, Some(transport));

    assert_eq!(err, Err(DomainError::CategoryHasSubcategories));
    assert_eq!(ledger.categories(), categories_before.as_slice());
    assert_eq!(ledger.expenses(), expenses_before.as_slice());
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));
    assert_eq!(ledger.expense_total(food)?, 300);
    Ok(())
}

#[test]
fn deleting_moves_expenses_to_the_chosen_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
    })?;
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let lunches = ledger.create_category(CreateCategory {
        name: "Обеды в городе".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    let home_lunches = ledger.create_category(CreateCategory {
        name: "Обед дома".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    let day = date(2026, 11, 22)?;
    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account_id),
        category_id: Some(lunches),
        amount: Some(1_500),
        comment: "суп".into(),
    })?;

    ledger.delete_category(lunches, Some(home_lunches))?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 1);
    assert_eq!(expenses[0].date(), day);
    assert_eq!(expenses[0].account_id(), account_id);
    assert_eq!(expenses[0].category_id(), home_lunches);
    assert_eq!(expenses[0].amount(), 1_500);
    assert_eq!(expenses[0].comment(), "суп");
    assert_eq!(ledger.expense_total(home_lunches)?, 1_500);
    assert_eq!(ledger.expense_total(food)?, 1_500);
    assert_eq!(ledger.category(lunches), Err(DomainError::CategoryNotFound));
    assert_eq!(
        ledger.expense_total(lunches),
        Err(DomainError::CategoryNotFound)
    );
    Ok(())
}

#[test]
fn deleting_expenses_without_a_target_keeps_the_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
    })?;
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(800),
        comment: String::new(),
    })?;
    let expenses_before = ledger.expenses().to_vec();

    let err = ledger.delete_category(food, None);

    assert_eq!(err, Err(DomainError::TransferTargetRequired));
    assert_eq!(ledger.category(food)?.name(), "Питание");
    assert_eq!(ledger.expenses(), expenses_before.as_slice());
    assert_eq!(ledger.expense_total(food)?, 800);
    Ok(())
}

#[test]
fn unsuitable_transfer_target_keeps_the_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
    })?;
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(800),
        comment: String::new(),
    })?;
    let expenses_before = ledger.expenses().to_vec();

    assert_eq!(
        ledger.delete_category(food, Some(salary)),
        Err(DomainError::UnsuitableTransferTarget)
    );
    assert_eq!(
        ledger.delete_category(food, Some(food)),
        Err(DomainError::UnsuitableTransferTarget)
    );
    assert_eq!(
        ledger.delete_category(food, Some(CategoryId::from_u64(99))),
        Err(DomainError::UnsuitableTransferTarget)
    );
    assert_eq!(ledger.category(food)?.name(), "Питание");
    assert_eq!(ledger.expenses(), expenses_before.as_slice());
    assert_eq!(ledger.expense_total(food)?, 800);
    Ok(())
}

#[test]
fn deleting_incomes_without_a_target_keeps_the_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    ledger.add_income(NewIncome {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(salary),
        amount: Some(50_000),
        comment: String::new(),
    })?;
    let incomes_before = ledger.incomes().to_vec();

    let err = ledger.delete_category(salary, None);

    assert_eq!(err, Err(DomainError::TransferTargetRequired));
    assert_eq!(ledger.category(salary)?.name(), "Зарплата");
    assert_eq!(ledger.incomes(), incomes_before.as_slice());
    assert_eq!(ledger.income_total(salary)?, 50_000);
    Ok(())
}

#[test]
fn deleting_moves_incomes_to_the_chosen_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let account_id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 10_000,
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    let bonus = ledger.create_category(CreateCategory {
        name: "Премия".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    let day = date(2026, 11, 22)?;
    ledger.add_income(NewIncome {
        date: day,
        account_id: Some(account_id),
        category_id: Some(salary),
        amount: Some(50_000),
        comment: "ноябрь".into(),
    })?;

    ledger.delete_category(salary, Some(bonus))?;

    let incomes = ledger.incomes();
    assert_eq!(incomes.len(), 1);
    assert_eq!(incomes[0].date(), day);
    assert_eq!(incomes[0].account_id(), account_id);
    assert_eq!(incomes[0].category_id(), bonus);
    assert_eq!(incomes[0].amount(), 50_000);
    assert_eq!(incomes[0].comment(), "ноябрь");
    assert_eq!(ledger.income_total(bonus)?, 50_000);
    assert_eq!(ledger.category(salary), Err(DomainError::CategoryNotFound));
    assert_eq!(
        ledger.income_total(salary),
        Err(DomainError::CategoryNotFound)
    );
    Ok(())
}

#[test]
fn deleting_records_without_a_target_keeps_the_category() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let lunches = ledger.create_category(CreateCategory {
        name: "Обеды в городе".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    ledger.add_record(Record::Expense {
        category_id: lunches,
        amount: 1_500,
    })?;
    let records_before = ledger.records().to_vec();

    let err = ledger.delete_category(lunches, None);

    assert_eq!(err, Err(DomainError::TransferTargetRequired));
    assert_eq!(ledger.category(lunches)?.name(), "Обеды в городе");
    assert_eq!(ledger.records(), records_before.as_slice());
    assert_eq!(ledger.expense_total(lunches)?, 1_500);
    assert_eq!(ledger.expense_total(food)?, 1_500);
    Ok(())
}

#[test]
fn deleting_moves_records_and_keeps_parent_total() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let lunches = ledger.create_category(CreateCategory {
        name: "Обеды в городе".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    let home_lunches = ledger.create_category(CreateCategory {
        name: "Обед дома".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    let salary = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    ledger.add_record(Record::Expense {
        category_id: lunches,
        amount: 1_500,
    })?;

    let targets = target_ids(&ledger, lunches)?;
    assert!(!targets.contains(&salary));

    ledger.delete_category(lunches, Some(home_lunches))?;

    assert_eq!(
        ledger.records(),
        [Record::Expense {
            category_id: home_lunches,
            amount: 1_500,
        }]
        .as_slice()
    );
    assert_eq!(ledger.expense_total(home_lunches)?, 1_500);
    assert_eq!(ledger.expense_total(food)?, 1_500);
    assert_eq!(ledger.category(lunches), Err(DomainError::CategoryNotFound));
    Ok(())
}
