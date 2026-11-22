mod common;

use common::{create_account, create_child, create_top};
use domain::{Applicability, CategoryId, DomainError, Ledger, NewExpense, date};

fn target_ids(ledger: &Ledger, id: CategoryId) -> Result<Vec<CategoryId>, DomainError> {
    Ok(ledger
        .transfer_targets(id)?
        .iter()
        .map(|category| category.id())
        .collect())
}

fn listed_among_categories(ledger: &Ledger, id: CategoryId) -> bool {
    ledger
        .categories()
        .iter()
        .any(|category| category.id() == id)
}

fn offered_for_expense(ledger: &Ledger, id: CategoryId) -> bool {
    ledger
        .categories_for_expense()
        .iter()
        .any(|category| category.id() == id)
}

fn offered_for_income(ledger: &Ledger, id: CategoryId) -> bool {
    ledger
        .categories_for_income()
        .iter()
        .any(|category| category.id() == id)
}

// 004 «Категория без операций»
#[test]
fn empty_leaf_category_is_removed_without_transfer_target() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let groceries = create_child(&mut ledger, "Продукты", food, None)?;

    ledger.delete_category(groceries, None)?;

    assert!(!listed_among_categories(&ledger, groceries));
    assert_eq!(
        ledger.category(groceries),
        Err(DomainError::CategoryNotFound)
    );
    assert!(!offered_for_expense(&ledger, groceries));
    assert!(!offered_for_income(&ledger, groceries));
    Ok(())
}

// 004 «Выбор категории для переноса»
#[test]
fn transfer_targets_for_category_with_expenses_include_parent_and_peers() -> Result<(), DomainError>
{
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let lunches = create_child(&mut ledger, "Обеды в городе", food, None)?;
    let home_lunches = create_child(&mut ledger, "Обед дома", food, None)?;
    let transport = create_top(&mut ledger, "Транспорт", Applicability::Expense)?;

    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(lunches),
        amount: Some(1_500),
        comment: String::new(),
    })?;

    let targets = target_ids(&ledger, lunches)?;

    assert!(!targets.contains(&lunches));
    assert!(targets.contains(&food));
    assert!(targets.contains(&home_lunches));
    assert!(targets.contains(&transport));
    Ok(())
}

// 004 «Перенос операций»
#[test]
fn deleting_transfers_expenses_and_preserves_report_totals() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let lunches = create_child(&mut ledger, "Обеды в городе", food, None)?;
    let home_lunches = create_child(&mut ledger, "Обед дома", food, None)?;

    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(lunches),
        amount: Some(1_500),
        comment: String::new(),
    })?;

    let moved_total = ledger.expense_total(lunches)?;

    ledger.delete_category(lunches, Some(home_lunches))?;

    let expenses = ledger.expenses();
    assert_eq!(expenses.len(), 1);
    assert_eq!(expenses[0].category_id(), home_lunches);
    assert_eq!(expenses[0].amount(), 1_500);
    assert_eq!(ledger.expense_total(home_lunches)?, moved_total);
    assert_eq!(ledger.expense_total(food)?, moved_total);
    assert!(!listed_among_categories(&ledger, lunches));
    assert_eq!(ledger.category(lunches), Err(DomainError::CategoryNotFound));
    assert_eq!(
        ledger.expense_total(lunches),
        Err(DomainError::CategoryNotFound)
    );
    assert!(!offered_for_expense(&ledger, lunches));
    assert!(!offered_for_income(&ledger, lunches));
    Ok(())
}

// 004 «Категория верхнего уровня»
#[test]
fn top_level_category_with_expenses_can_transfer_to_peer_subcategory() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let leisure = create_top(&mut ledger, "Досуг", Applicability::Expense)?;
    let cinema = create_child(&mut ledger, "Кино", leisure, None)?;

    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(800),
        comment: String::new(),
    })?;

    let targets = target_ids(&ledger, food)?;
    assert!(!targets.contains(&food));
    assert!(targets.contains(&cinema));

    let moved_total = ledger.expense_total(food)?;
    ledger.delete_category(food, Some(cinema))?;

    assert_eq!(ledger.expenses()[0].category_id(), cinema);
    assert_eq!(ledger.expense_total(cinema)?, moved_total);
    assert_eq!(ledger.expense_total(leisure)?, moved_total);
    assert!(!listed_among_categories(&ledger, food));
    assert_eq!(ledger.category(food), Err(DomainError::CategoryNotFound));
    Ok(())
}

// 004 «Подходящие категории»
#[test]
fn transfer_targets_with_expenses_exclude_income_only_categories() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let lunches = create_child(&mut ledger, "Обеды в городе", food, None)?;
    let gifts = create_top(&mut ledger, "Подарки", Applicability::Both)?;
    let salary = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let cashback = create_child(&mut ledger, "Кэшбэк", food, Some(Applicability::Income))?;

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
    assert!(!targets.contains(&cashback));
    for id in targets {
        let applicability = ledger.category(id)?.applicability();
        assert!(matches!(
            applicability,
            Applicability::Expense | Applicability::Both
        ));
    }
    Ok(())
}

// 004 «Категория с подкатегориями»
#[test]
fn category_with_subcategories_cannot_be_deleted() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account_id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let groceries = create_child(&mut ledger, "Продукты", food, None)?;
    let transport = create_top(&mut ledger, "Транспорт", Applicability::Expense)?;

    ledger.add_expense(NewExpense {
        date: date(2026, 11, 22)?,
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(300),
        comment: String::new(),
    })?;

    let categories_before = ledger.categories().to_vec();
    let expenses_before = ledger.expenses().to_vec();

    assert_eq!(
        ledger.delete_category(food, Some(transport)),
        Err(DomainError::CategoryHasSubcategories)
    );

    assert_eq!(ledger.categories(), categories_before.as_slice());
    assert_eq!(ledger.expenses(), expenses_before.as_slice());
    assert_eq!(ledger.category(food)?.name(), "Питание");
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));
    assert_eq!(ledger.expense_total(food)?, 300);
    Ok(())
}
