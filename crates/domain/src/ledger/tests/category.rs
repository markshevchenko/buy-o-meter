use super::super::Ledger;
use crate::category::{Category, CategoryId, CreateCategory, UpdateCategory};
use crate::error::DomainError;
use crate::operation::Record;

struct ExpectedGroup {
    name: &'static str,
    children: &'static [&'static str],
}

#[rustfmt::skip]
const EXPECTED_EXPENSES: &[ExpectedGroup] = &[
    ExpectedGroup {
        name: "Жильё",
        children: &[
            "Аренда/покупка/ипотека",
            "ЖКХ",
            "Интернет и связь",
            "Ремонт и мебель",
            "Техника",
            "Хозтовары",
        ],
    },
    ExpectedGroup {
        name: "Питание",
        children: &[
            "Продукты",
            "Кафе и рестораны",
            "Рабочие обеды",
        ],
    },
    ExpectedGroup {
        name: "Транспорт",
        children: &[
            "Общественный транспорт",
            "Бензин",
            "Такси",
            "Парковка и штрафы",
            "Обслуживание авто",
        ],
    },
    ExpectedGroup {
        name: "Здоровье",
        children: &[
            "Лекарства",
            "Врачи и клиники",
            "Страховка",
            "Спорт и фитнес",
        ],
    },
    ExpectedGroup {
        name: "Личные расходы",
        children: &[
            "Одежда и обувь",
            "Гигиена и красота",
            "Подписки и сервисы",
        ],
    },
    ExpectedGroup {
        name: "Семья и дети",
        children: &[
            "Образование",
            "Одежда и товары",
            "Секции и кружки",
            "Питомцы",
        ],
    },
    ExpectedGroup {
        name: "Развлечения и развитие",
        children: &[
            "Досуг (кино, концерты)",
            "Путешествия",
            "Образование (курсы, книги)",
            "Хобби",
        ],
    },
    ExpectedGroup {
        name: "Обязательные платежи",
        children: &[
            "Кредиты и займы",
            "Налоги",
            "Алименты",
            "Банковские комиссии",
        ],
    },
    ExpectedGroup {
        name: "Прочее",
        children: &[
            "Непредвиденные расходы",
            "Прочие траты",
            "Потери",
            "Подарки",
        ],
    },
];

#[rustfmt::skip]
const EXPECTED_INCOMES: &[ExpectedGroup] = &[
    ExpectedGroup {
        name: "Активный доход",
        children: &[
            "Зарплата (оклад)",
            "Премии и бонусы",
            "Подработка",
        ],
    },
    ExpectedGroup {
        name: "Пассивный доход",
        children: &[
            "Проценты по вкладам",
            "Кэшбэк",
        ],
    },
    ExpectedGroup {
        name: "Прочее",
        children: &[
            "Прочие доходы",
            "Купоны по облигациям",
            "Подарки",
        ],
    },
];

#[test]
fn move_that_would_exceed_two_levels_is_rejected() -> Result<(), DomainError> {
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
    let leisure = ledger.create_category(CreateCategory {
        name: "Досуг".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let before = ledger.categories().to_vec();

    let err = ledger.move_category(food, leisure);
    assert_eq!(err, Err(DomainError::CategoryTooDeep));
    assert_eq!(ledger.categories(), before.as_slice());
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));

    let taxi = ledger.create_category(CreateCategory {
        name: "Такси".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let err = ledger.move_category(taxi, groceries);
    assert_eq!(err, Err(DomainError::CategoryTooDeep));
    assert_eq!(ledger.category(taxi)?.parent_id(), None);

    ledger.add_record(Record::Expense {
        category_id: taxi,
        amount: 100,
    })?;
    let err = ledger.move_category(taxi, taxi);
    assert_eq!(err, Err(DomainError::CategoryTooDeep));
    assert_eq!(ledger.category(taxi)?.parent_id(), None);
    assert_eq!(ledger.expense_total(taxi)?, 100);
    Ok(())
}

#[test]
fn categories_for_kind_include_that_kind_and_both() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let expense = ledger.create_category(CreateCategory {
        name: "Транспорт".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    let income = ledger.create_category(CreateCategory {
        name: "Зарплата".into(),
        parent_id: None,
        applicability: Some(Applicability::Income),
    })?;
    let both = ledger.create_category(CreateCategory {
        name: "Подарки".into(),
        parent_id: None,
        applicability: Some(Applicability::Both),
    })?;

    let for_expense = ledger.categories_for_expense();
    let for_income = ledger.categories_for_income();
    assert!(for_expense.iter().any(|category| category.id() == expense));
    assert!(for_expense.iter().any(|category| category.id() == both));
    assert!(!for_expense.iter().any(|category| category.id() == income));
    assert!(for_income.iter().any(|category| category.id() == income));
    assert!(for_income.iter().any(|category| category.id() == both));
    assert!(!for_income.iter().any(|category| category.id() == expense));
    Ok(())
}

#[test]
fn moving_category_counts_its_amounts_under_the_new_parent() -> Result<(), DomainError> {
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
    let leisure = ledger.create_category(CreateCategory {
        name: "Досуг".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    ledger.add_record(Record::Expense {
        category_id: lunches,
        amount: 1_500,
    })?;

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

#[test]
fn totals_add_own_records_and_direct_children() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let food = ledger.create_category(CreateCategory {
        name: "Питание".into(),
        parent_id: None,
        applicability: Some(Applicability::Both),
    })?;
    let lunches = ledger.create_category(CreateCategory {
        name: "Обеды".into(),
        parent_id: Some(food),
        applicability: None,
    })?;
    ledger.add_record(Record::Expense {
        category_id: food,
        amount: 400,
    })?;
    ledger.add_record(Record::Expense {
        category_id: lunches,
        amount: 1_500,
    })?;
    ledger.add_record(Record::Income {
        category_id: lunches,
        amount: 50,
    })?;

    assert_eq!(ledger.expense_total(food)?, 1_900);
    assert_eq!(ledger.expense_total(lunches)?, 1_500);
    assert_eq!(ledger.income_total(food)?, 50);
    assert_eq!(ledger.income_total(lunches)?, 50);
    Ok(())
}

#[test]
fn renaming_category_keeps_amounts_on_the_same_id() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let id = ledger.create_category(CreateCategory {
        name: "Кафе".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;
    ledger.add_record(Record::Expense {
        category_id: id,
        amount: 450,
    })?;

    ledger.update_category(
        id,
        UpdateCategory {
            name: Some("Кофейни".into()),
            applicability: None,
        },
    )?;

    assert_eq!(ledger.category(id)?.name(), "Кофейни");
    assert_eq!(
        ledger.records(),
        &[Record::Expense {
            category_id: id,
            amount: 450,
        }]
    );
    Ok(())
}

#[test]
fn update_category_changes_only_provided_fields() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let id = ledger.create_category(CreateCategory {
        name: "Кафе".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;

    ledger.update_category(
        id,
        UpdateCategory {
            name: Some("Кофейни".into()),
            applicability: None,
        },
    )?;
    assert_eq!(ledger.category(id)?.name(), "Кофейни");
    assert_eq!(ledger.category(id)?.applicability(), Applicability::Expense);

    ledger.update_category(
        id,
        UpdateCategory {
            name: None,
            applicability: Some(Applicability::Both),
        },
    )?;
    assert_eq!(ledger.category(id)?.name(), "Кофейни");
    assert_eq!(ledger.category(id)?.applicability(), Applicability::Both);
    assert_eq!(ledger.category(id)?.parent_id(), None);
    Ok(())
}

#[test]
fn subcategory_of_subcategory_is_rejected() -> Result<(), DomainError> {
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

    let before = ledger.categories().to_vec();
    let err = ledger.create_category(CreateCategory {
        name: "Молоко".into(),
        parent_id: Some(groceries),
        applicability: None,
    });
    assert_eq!(err, Err(DomainError::CategoryTooDeep));
    assert_eq!(ledger.categories(), before.as_slice());
    Ok(())
}

#[test]
fn subcategory_inherits_parent_applicability_unless_overridden() -> Result<(), DomainError> {
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
    assert_eq!(ledger.category(groceries)?.parent_id(), Some(food));
    assert_eq!(
        ledger.category(groceries)?.applicability(),
        Applicability::Expense
    );

    let cafes = ledger.create_category(CreateCategory {
        name: "Кафе".into(),
        parent_id: Some(food),
        applicability: Some(Applicability::Income),
    })?;
    assert_eq!(
        ledger.category(cafes)?.applicability(),
        Applicability::Income
    );
    Ok(())
}

#[test]
fn create_top_level_category_stores_name_and_applicability() -> Result<(), DomainError> {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    let id = ledger.create_category(CreateCategory {
        name: "Транспорт".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;

    assert_eq!(ledger.categories().len(), 1);
    assert_eq!(ledger.category(id)?.id(), id);
    assert_eq!(ledger.category(id)?.name(), "Транспорт");
    assert_eq!(ledger.category(id)?.applicability(), Applicability::Expense);
    assert_eq!(ledger.category(id)?.parent_id(), None);
    Ok(())
}

#[test]
fn seed_starter_categories_creates_housing_group() {
    let mut ledger = Ledger::new();
    assert!(ledger.categories().is_empty());

    ledger.seed_starter_categories();

    let categories = ledger.categories();
    let housing_id = assert_expense_at(categories, 0, "Жильё", None);
    assert_expense_at(categories, 1, "Аренда/покупка/ипотека", Some(housing_id));
    assert_expense_at(categories, 2, "ЖКХ", Some(housing_id));
    assert_expense_at(categories, 3, "Интернет и связь", Some(housing_id));
    assert_expense_at(categories, 4, "Ремонт и мебель", Some(housing_id));
    assert_expense_at(categories, 5, "Техника", Some(housing_id));
    assert_expense_at(categories, 6, "Хозтовары", Some(housing_id));
}

#[test]
fn seed_starter_categories_creates_the_expense_tree() {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    ledger.seed_starter_categories();

    let mut index = 0;
    place_expected(
        ledger.categories(),
        &mut index,
        EXPECTED_EXPENSES,
        Applicability::Expense,
    );
    assert_eq!(index, group_len(EXPECTED_EXPENSES));
}

#[test]
fn seed_starter_categories_appends_the_income_tree() {
    use crate::category::Applicability;

    let mut ledger = Ledger::new();
    ledger.seed_starter_categories();

    let mut index = 0;
    place_expected(
        ledger.categories(),
        &mut index,
        EXPECTED_EXPENSES,
        Applicability::Expense,
    );
    place_expected(
        ledger.categories(),
        &mut index,
        EXPECTED_INCOMES,
        Applicability::Income,
    );
    assert_eq!(index, ledger.categories().len());
}

#[test]
fn seed_starter_categories_again_keeps_edits() -> Result<(), DomainError> {
    use crate::category::{Applicability, CreateCategory, UpdateCategory};

    let mut ledger = Ledger::new();
    ledger.seed_starter_categories();

    let housing = ledger.categories()[0].id();
    ledger.update_category(
        housing,
        UpdateCategory {
            name: Some("Дом".into()),
            applicability: Some(Applicability::Both),
        },
    )?;
    let custom = ledger.create_category(CreateCategory {
        name: "Свои траты".into(),
        parent_id: None,
        applicability: Some(Applicability::Expense),
    })?;

    let configured = ledger.categories().to_vec();
    ledger.seed_starter_categories();

    assert_eq!(ledger.categories(), configured.as_slice());
    assert_eq!(ledger.category(housing)?.name(), "Дом");
    assert_eq!(ledger.category(custom)?.name(), "Свои траты");
    Ok(())
}

fn place_expected(
    categories: &[Category],
    index: &mut usize,
    groups: &[ExpectedGroup],
    applicability: crate::category::Applicability,
) {
    for group in groups {
        let parent_id = assert_placed(categories, *index, group.name, None, applicability);
        *index += 1;
        place_children(categories, index, parent_id, group.children, applicability);
    }
}

fn place_children(
    categories: &[Category],
    index: &mut usize,
    parent_id: CategoryId,
    children: &[&str],
    applicability: crate::category::Applicability,
) {
    for child in children {
        assert_placed(categories, *index, child, Some(parent_id), applicability);
        *index += 1;
    }
}

fn group_len(groups: &[ExpectedGroup]) -> usize {
    groups.iter().map(|group| 1 + group.children.len()).sum()
}

fn assert_expense_at(
    categories: &[Category],
    index: usize,
    name: &str,
    parent_id: Option<CategoryId>,
) -> CategoryId {
    use crate::category::Applicability;

    assert_placed(categories, index, name, parent_id, Applicability::Expense)
}

fn assert_placed(
    categories: &[Category],
    index: usize,
    name: &str,
    parent_id: Option<CategoryId>,
    applicability: crate::category::Applicability,
) -> CategoryId {
    let category = &categories[index];
    assert_eq!(category.name(), name);
    assert_eq!(category.parent_id(), parent_id);
    assert_eq!(category.applicability(), applicability);
    category.id()
}
