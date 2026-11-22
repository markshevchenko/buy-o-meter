mod common;

use common::{create_child, create_top};
use domain::{Applicability, CategoryId, DomainError, Ledger, Record, UpdateCategory};

fn add_expense(
    ledger: &mut Ledger,
    category_id: CategoryId,
    amount: i64,
) -> Result<(), DomainError> {
    ledger.add_record(Record::Expense {
        category_id,
        amount,
    })
}

fn add_income(
    ledger: &mut Ledger,
    category_id: CategoryId,
    amount: i64,
) -> Result<(), DomainError> {
    ledger.add_record(Record::Income {
        category_id,
        amount,
    })
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

// 003 «Категория верхнего уровня»
#[test]
fn creating_top_level_category_adds_it_to_the_tree() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_top(&mut ledger, "Транспорт", Applicability::Expense)?;

    let categories = ledger.categories();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].id(), id);
    assert_eq!(categories[0].name(), "Транспорт");
    assert_eq!(categories[0].applicability(), Applicability::Expense);
    assert_eq!(categories[0].parent_id(), None);
    Ok(())
}

// 003 «Подкатегория наследует настройки»
#[test]
fn subcategory_inherits_applicability_and_can_be_changed() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;

    let groceries = create_child(&mut ledger, "Продукты", food, None)?;
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));
    assert_eq!(
        ledger.category(groceries)?.applicability(),
        Applicability::Expense
    );

    let cafes = create_child(&mut ledger, "Кафе", food, Some(Applicability::Income))?;
    assert_eq!(ledger.category(cafes)?.parent_id(), Some(food));
    assert_eq!(
        ledger.category(cafes)?.applicability(),
        Applicability::Income
    );

    ledger.update_category(
        groceries,
        UpdateCategory {
            name: None,
            applicability: Some(Applicability::Both),
        },
    )?;
    assert_eq!(ledger.category(groceries)?.name(), "Продукты");
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));
    assert_eq!(
        ledger.category(groceries)?.applicability(),
        Applicability::Both
    );
    Ok(())
}

// 003 «Не больше двух уровней»
#[test]
fn adding_subcategory_to_a_subcategory_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let groceries = create_child(&mut ledger, "Продукты", food, None)?;
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));

    let snapshot = ledger.categories().to_vec();
    let err = create_child(&mut ledger, "Молоко", groceries, None);
    assert_eq!(err, Err(DomainError::CategoryTooDeep));
    assert_eq!(ledger.categories(), snapshot.as_slice());
    Ok(())
}

// 003 «Переименование»
#[test]
fn renaming_category_shows_new_name_for_existing_amounts() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_top(&mut ledger, "Кафе", Applicability::Expense)?;
    add_expense(&mut ledger, id, 450)?;
    assert_eq!(ledger.category(id)?.name(), "Кафе");

    ledger.update_category(
        id,
        UpdateCategory {
            name: Some("Кофейни".into()),
            applicability: None,
        },
    )?;

    assert_eq!(ledger.category(id)?.name(), "Кофейни");
    assert_eq!(ledger.category(id)?.applicability(), Applicability::Expense);
    assert_eq!(
        ledger.records(),
        &[Record::Expense {
            category_id: id,
            amount: 450,
        }]
    );
    Ok(())
}

// 003 «Перемещение»
#[test]
fn moving_subcategory_counts_its_amounts_in_the_new_parent() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let lunches = create_child(&mut ledger, "Обеды в городе", food, None)?;
    let leisure = create_top(&mut ledger, "Досуг", Applicability::Expense)?;
    add_expense(&mut ledger, lunches, 1_500)?;

    assert_eq!(ledger.category(lunches)?.parent_id(), Some(food));
    assert_eq!(ledger.category(leisure)?.parent_id(), None);
    assert_eq!(ledger.expense_total(food)?, 1_500);

    ledger.move_category(lunches, leisure)?;

    assert_eq!(ledger.category(lunches)?.parent_id(), Some(leisure));
    assert_eq!(ledger.expense_total(lunches)?, 1_500);
    assert_eq!(ledger.expense_total(leisure)?, 1_500);
    assert_eq!(ledger.expense_total(food)?, 0);
    assert_eq!(
        ledger.records(),
        &[Record::Expense {
            category_id: lunches,
            amount: 1_500,
        }]
    );
    Ok(())
}

// 003 «Изменение признака»
#[test]
fn changing_applicability_to_both_offers_category_for_income() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_top(&mut ledger, "Подарки", Applicability::Expense)?;
    assert_eq!(ledger.category(id)?.applicability(), Applicability::Expense);
    assert!(!offered_for_income(&ledger, id));

    ledger.update_category(
        id,
        UpdateCategory {
            name: None,
            applicability: Some(Applicability::Both),
        },
    )?;

    assert_eq!(ledger.category(id)?.applicability(), Applicability::Both);
    assert!(offered_for_income(&ledger, id));
    assert!(offered_for_expense(&ledger, id));
    Ok(())
}

// 003 «Признак не затрагивает внесённые операции»
#[test]
fn changing_applicability_keeps_recorded_income() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_top(&mut ledger, "Продажи", Applicability::Both)?;
    add_income(&mut ledger, id, 2_000)?;

    ledger.update_category(
        id,
        UpdateCategory {
            name: None,
            applicability: Some(Applicability::Expense),
        },
    )?;

    assert_eq!(ledger.category(id)?.applicability(), Applicability::Expense);
    assert_eq!(
        ledger.records(),
        &[Record::Income {
            category_id: id,
            amount: 2_000,
        }]
    );
    assert_eq!(ledger.income_total(id)?, 2_000);
    assert!(!offered_for_income(&ledger, id));
    assert!(offered_for_expense(&ledger, id));
    Ok(())
}

// 003 «Выбор категории при вводе»
#[test]
fn income_only_category_is_not_offered_for_expense() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_top(&mut ledger, "Зарплата", Applicability::Income)?;

    assert_eq!(ledger.category(id)?.applicability(), Applicability::Income);
    assert!(!offered_for_expense(&ledger, id));
    assert!(offered_for_income(&ledger, id));
    Ok(())
}

// 003 «Категорию с подкатегориями нельзя вложить в другую»
#[test]
fn category_with_subcategories_cannot_be_nested_into_another() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let food = create_top(&mut ledger, "Питание", Applicability::Expense)?;
    let groceries = create_child(&mut ledger, "Продукты", food, None)?;
    let leisure = create_top(&mut ledger, "Досуг", Applicability::Expense)?;
    assert_eq!(ledger.category(food)?.parent_id(), None);
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));

    let snapshot = ledger.categories().to_vec();
    let err = ledger.move_category(food, leisure);
    assert_eq!(err, Err(DomainError::CategoryTooDeep));
    assert_eq!(ledger.categories(), snapshot.as_slice());
    Ok(())
}
