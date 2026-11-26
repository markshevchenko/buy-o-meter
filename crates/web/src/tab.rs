#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Operations,
    Reports,
    Categories,
    Accounts,
    About,
}

impl Tab {
    pub(crate) const ALL: [Tab; 4] = [
        Tab::Operations,
        Tab::Reports,
        Tab::Categories,
        Tab::Accounts,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Tab::Operations => "Операции",
            Tab::Reports => "Отчёты",
            Tab::Categories => "Категории",
            Tab::Accounts => "Счета",
            Tab::About => "О приложении",
        }
    }
}
