pub(crate) struct StarterGroup {
    pub(crate) name: &'static str,
    pub(crate) children: &'static [&'static str],
}

#[rustfmt::skip]
pub(crate) const EXPENSE_TREE: &[StarterGroup] = &[
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
pub(crate) const INCOME_TREE: &[StarterGroup] = &[
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
