use domain::{
    Applicability, Category, CategoryId, CreateCategory, DomainError, Ledger, UpdateCategory,
};

struct StarterGroup {
    name: &'static str,
    children: &'static [&'static str],
}

#[rustfmt::skip]
const EXPENSE_TREE: &[StarterGroup] = &[
    StarterGroup {
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
    StarterGroup {
        name: "Питание",
        children: &[
            "Продукты",
            "Кафе и рестораны",
            "Рабочие обеды",
        ],
    },
    StarterGroup {
        name: "Транспорт",
        children: &[
            "Общественный транспорт",
            "Бензин",
            "Такси",
            "Парковка и штрафы",
            "Обслуживание авто",
        ],
    },
    StarterGroup {
        name: "Здоровье",
        children: &[
            "Лекарства",
            "Врачи и клиники",
            "Страховка",
            "Спорт и фитнес",
        ],
    },
    StarterGroup {
        name: "Личные расходы",
        children: &[
            "Одежда и обувь",
            "Гигиена и красота",
            "Подписки и сервисы",
        ],
    },
    StarterGroup {
        name: "Семья и дети",
        children: &[
            "Образование",
            "Одежда и товары",
            "Секции и кружки",
            "Питомцы",
        ],
    },
    StarterGroup {
        name: "Развлечения и развитие",
        children: &[
            "Досуг (кино, концерты)",
            "Путешествия",
            "Образование (курсы, книги)",
            "Хобби",
        ],
    },
    StarterGroup {
        name: "Обязательные платежи",
        children: &[
            "Кредиты и займы",
            "Налоги",
            "Алименты",
            "Банковские комиссии",
        ],
    },
    StarterGroup {
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
const INCOME_TREE: &[StarterGroup] = &[
    StarterGroup {
        name: "Активный доход",
        children: &[
            "Зарплата (оклад)",
            "Премии и бонусы",
            "Подработка",
        ],
    },
    StarterGroup {
        name: "Пассивный доход",
        children: &[
            "Проценты по вкладам",
            "Кэшбэк",
        ],
    },
    StarterGroup {
        name: "Прочее",
        children: &[
            "Прочие доходы",
            "Купоны по облигациям",
            "Подарки",
        ],
    },
];

fn flag(check: bool, applicability: Applicability) -> Option<Applicability> {
    check.then_some(applicability)
}

fn assert_placed(
    categories: &[Category],
    index: usize,
    name: &str,
    parent_id: Option<CategoryId>,
    applicability: Option<Applicability>,
) -> CategoryId {
    assert!(
        index < categories.len(),
        "category {name} is missing at position {index}"
    );
    let category = &categories[index];
    assert_eq!(category.name(), name, "position {index}");
    assert_eq!(category.parent_id(), parent_id, "{name}");
    if let Some(applicability) = applicability {
        assert_eq!(category.applicability(), applicability, "{name}");
    }
    category.id()
}

fn place_tree(
    categories: &[Category],
    index: &mut usize,
    tree: &[StarterGroup],
    applicability: Option<Applicability>,
) {
    for node in tree {
        let parent_id = assert_placed(categories, *index, node.name, None, applicability);
        *index += 1;
        for child in node.children {
            assert_placed(categories, *index, child, Some(parent_id), applicability);
            *index += 1;
        }
    }
}

fn assert_starter_set(categories: &[Category], check_applicability: bool) {
    let mut index = 0;
    place_tree(
        categories,
        &mut index,
        EXPENSE_TREE,
        flag(check_applicability, Applicability::Expense),
    );
    place_tree(
        categories,
        &mut index,
        INCOME_TREE,
        flag(check_applicability, Applicability::Income),
    );
    assert_eq!(index, categories.len());
}

fn require_id(ledger: &Ledger, name: &str, parent_id: Option<CategoryId>) -> CategoryId {
    for category in ledger.categories() {
        if category.name() == name && category.parent_id() == parent_id {
            return category.id();
        }
    }
    panic!("category {name} is missing");
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

// 002 «Набор создаётся при первом входе»
#[test]
fn first_entry_creates_the_two_level_starter_tree() {
    let mut ledger = Ledger::new();
    assert!(ledger.categories().is_empty());

    ledger.seed_starter_categories();

    assert_starter_set(ledger.categories(), false);
}

// 002 «Признаки применимости»
#[test]
fn starter_categories_have_expense_income_or_both_applicability() {
    let mut ledger = Ledger::new();
    ledger.seed_starter_categories();

    assert_starter_set(ledger.categories(), true);
}

// 002 «Набор можно менять»
#[test]
fn starter_category_keeps_rename_applicability_change_and_move() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    ledger.seed_starter_categories();

    let housing = require_id(&ledger, "Жильё", None);
    let utilities = require_id(&ledger, "ЖКХ", Some(housing));
    let food = require_id(&ledger, "Питание", None);

    ledger.update_category(
        utilities,
        UpdateCategory {
            name: Some("Коммунальные".into()),
            applicability: Some(Applicability::Both),
        },
    )?;
    ledger.move_category(utilities, food)?;

    let updated = ledger.category(utilities)?;
    assert_eq!(updated.id(), utilities);
    assert_eq!(updated.name(), "Коммунальные");
    assert_eq!(updated.applicability(), Applicability::Both);
    assert_eq!(updated.parent_id(), Some(food));
    assert!(offered_for_expense(&ledger, utilities));
    assert!(offered_for_income(&ledger, utilities));
    assert_eq!(ledger.category(housing)?.name(), "Жильё");
    assert_eq!(ledger.category(food)?.parent_id(), None);
    Ok(())
}

// 002 «Набор не создаётся повторно»
#[test]
fn seeding_again_keeps_edits_and_does_not_duplicate_the_starter_set() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    ledger.seed_starter_categories();

    let housing = require_id(&ledger, "Жильё", None);
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

    assert_eq!(ledger.category(housing)?.name(), "Дом");
    assert_eq!(
        ledger.category(housing)?.applicability(),
        Applicability::Both
    );
    assert_eq!(ledger.category(custom)?.name(), "Свои траты");
    assert_eq!(ledger.category(custom)?.parent_id(), None);
    assert_eq!(ledger.categories(), configured.as_slice());
    Ok(())
}
