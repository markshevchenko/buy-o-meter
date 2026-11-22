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
];
