use std::time::SystemTime;

use crate::account::{Account, AccountId, CreateAccount, UpdateAccount};
use crate::category::{Applicability, Category, CategoryId, CreateCategory, UpdateCategory};
use crate::currency::Currency;
use crate::error::{DomainError, MissingFields};
use crate::operation::{
    Expense, Income, NewExpense, NewIncome, NewTransfer, OperationDraft, Record,
    RepaymentDirection, Transfer, TransferAmountFields,
};
use crate::starter::{EXPENSE_TREE, INCOME_TREE, StarterGroup};

#[derive(Debug)]
pub struct Ledger {
    next_account_id: u64,
    accounts: Vec<Account>,
    operations: Vec<OperationDraft>,
    next_category_id: u64,
    categories: Vec<Category>,
    starter_seeded: bool,
    records: Vec<Record>,
    expenses: Vec<Expense>,
    incomes: Vec<Income>,
    transfers: Vec<Transfer>,
}

impl Ledger {
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_account_id: 1,
            accounts: Vec::new(),
            operations: Vec::new(),
            next_category_id: 1,
            categories: Vec::new(),
            starter_seeded: false,
            records: Vec::new(),
            expenses: Vec::new(),
            incomes: Vec::new(),
            transfers: Vec::new(),
        }
    }

    pub fn create_account(&mut self, input: CreateAccount) -> Result<AccountId, DomainError> {
        let id = AccountId::from_u64(self.next_account_id);
        self.next_account_id += 1;
        self.accounts.push(Account::new(id, input));
        Ok(id)
    }

    #[must_use]
    pub fn accounts(&self) -> &[Account] {
        &self.accounts
    }

    pub fn account(&self, id: AccountId) -> Result<&Account, DomainError> {
        self.find_account(id).ok_or(DomainError::AccountNotFound)
    }

    pub fn balance(&self, id: AccountId) -> Result<i64, DomainError> {
        let account = self.find_account(id).ok_or(DomainError::AccountNotFound)?;
        let mut total = account.opening_balance();
        for operation in &self.operations {
            total += balance_effect(id, operation);
        }
        Ok(total)
    }

    pub fn update_account(
        &mut self,
        id: AccountId,
        input: UpdateAccount,
    ) -> Result<(), DomainError> {
        if let Some(currency) = &input.currency {
            self.ensure_currency_change_allowed(id, currency)?;
        }
        self.find_account_mut(id)
            .ok_or(DomainError::AccountNotFound)?
            .update(input);
        Ok(())
    }

    /// Создаёт стартовый набор категорий один раз.
    /// Повторный вызов не добавляет категории и не откатывает правки.
    pub fn seed_starter_categories(&mut self) {
        if self.starter_seeded {
            return;
        }
        self.starter_seeded = true;
        self.seed_tree(EXPENSE_TREE, Applicability::Expense);
        self.seed_tree(INCOME_TREE, Applicability::Income);
    }

    fn seed_tree(&mut self, tree: &[StarterGroup], applicability: Applicability) {
        for group in tree {
            self.seed_group(group, applicability);
        }
    }

    fn seed_group(&mut self, group: &StarterGroup, applicability: Applicability) {
        let Some(parent_id) = self.seed_category(group.name, None, applicability) else {
            return;
        };
        self.seed_children(parent_id, group.children, applicability);
    }

    fn seed_children(
        &mut self,
        parent_id: CategoryId,
        children: &[&str],
        applicability: Applicability,
    ) {
        for child in children {
            let _ = self.seed_category(child, Some(parent_id), applicability);
        }
    }

    fn seed_category(
        &mut self,
        name: &str,
        parent_id: Option<CategoryId>,
        applicability: Applicability,
    ) -> Option<CategoryId> {
        self.create_category(CreateCategory {
            name: name.to_owned(),
            parent_id,
            applicability: Some(applicability),
        })
        .ok()
    }

    pub fn create_category(&mut self, input: CreateCategory) -> Result<CategoryId, DomainError> {
        let input = self.prepare_category(input)?;
        let id = CategoryId::from_u64(self.next_category_id);
        let category = Category::new(id, input)?;
        self.next_category_id += 1;
        self.categories.push(category);
        Ok(id)
    }

    #[must_use]
    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub fn category(&self, id: CategoryId) -> Result<&Category, DomainError> {
        self.find_category(id).ok_or(DomainError::CategoryNotFound)
    }

    pub fn update_category(
        &mut self,
        id: CategoryId,
        input: UpdateCategory,
    ) -> Result<(), DomainError> {
        self.find_category_mut(id)
            .ok_or(DomainError::CategoryNotFound)?
            .update(input);
        Ok(())
    }

    pub fn move_category(
        &mut self,
        id: CategoryId,
        new_parent_id: CategoryId,
    ) -> Result<(), DomainError> {
        self.ensure_move_keeps_two_levels(id, new_parent_id)?;
        self.find_category_mut(id)
            .ok_or(DomainError::CategoryNotFound)?
            .set_parent(new_parent_id);
        Ok(())
    }

    pub fn add_record(&mut self, record: Record) -> Result<(), DomainError> {
        self.records.push(record);
        Ok(())
    }

    #[must_use]
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    pub fn expense_total(&self, id: CategoryId) -> Result<i64, DomainError> {
        self.category_total(id, Self::expense_slice)
    }

    pub fn income_total(&self, id: CategoryId) -> Result<i64, DomainError> {
        self.category_total(id, Self::income_slice)
    }

    #[must_use]
    pub fn categories_for_expense(&self) -> Vec<&Category> {
        self.categories_where(|applicability| {
            matches!(applicability, Applicability::Expense | Applicability::Both)
        })
    }

    #[must_use]
    pub fn categories_for_income(&self) -> Vec<&Category> {
        self.categories_where(|applicability| {
            matches!(applicability, Applicability::Income | Applicability::Both)
        })
    }

    pub fn add_operation(&mut self, draft: OperationDraft) -> Result<(), DomainError> {
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        self.operations.push(draft);
        Ok(())
    }

    /// Пустые счёт, категория или сумма — `DomainError::MissingFields`, список не меняется.
    /// Сумма уменьшает остаток счёта и входит в итог категории.
    pub fn add_expense(&mut self, input: NewExpense) -> Result<(), DomainError> {
        let (account_id, category_id, amount) = complete_expense(&input)?;
        self.ensure_account_and_category(account_id, category_id)?;
        self.place_expense(Expense::new(
            input.date,
            account_id,
            category_id,
            amount,
            input.comment,
        ));
        #[rustfmt::skip]
        self.operations.push(OperationDraft::Expense {
            account_id,
            amount,
        });
        Ok(())
    }

    fn ensure_account_and_category(
        &self,
        account_id: AccountId,
        category_id: CategoryId,
    ) -> Result<(), DomainError> {
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        if self.find_account(account_id).is_none() {
            return Err(DomainError::AccountNotFound);
        }
        if self.find_category(category_id).is_none() {
            return Err(DomainError::CategoryNotFound);
        }
        Ok(())
    }

    /// По дате, внутри дня в порядке добавления.
    #[must_use]
    pub fn expenses(&self) -> &[Expense] {
        &self.expenses
    }

    /// Пустые счёт, категория или сумма — `DomainError::MissingFields`, список не меняется.
    /// Сумма увеличивает остаток счёта и входит в итог категории.
    pub fn add_income(&mut self, input: NewIncome) -> Result<(), DomainError> {
        let (account_id, category_id, amount) = complete_income(&input)?;
        self.ensure_account_and_category(account_id, category_id)?;
        self.place_income(Income::new(
            input.date,
            account_id,
            category_id,
            amount,
            input.comment,
        ));
        #[rustfmt::skip]
        self.operations.push(OperationDraft::Income {
            account_id,
            amount,
        });
        Ok(())
    }

    /// По дате, внутри дня в порядке добавления.
    #[must_use]
    pub fn incomes(&self) -> &[Income] {
        &self.incomes
    }

    /// Одна валюта: списывается и зачисляется `amount`, `to_amount` пустой.
    /// Разные валюты: списывается `amount`, зачисляется `to_amount`.
    /// Один и тот же счёт — `DomainError::SameAccount`, список не меняется.
    /// В итоги категорий перевод не входит.
    pub fn add_transfer(&mut self, input: NewTransfer) -> Result<(), DomainError> {
        let (from_account_id, to_account_id, amount) = complete_transfer(&input)?;
        if from_account_id == to_account_id {
            return Err(DomainError::SameAccount);
        }
        let to_amount =
            self.credited_amount(from_account_id, to_account_id, amount, input.to_amount)?;
        self.place_transfer(Transfer::new(
            input.date,
            from_account_id,
            to_account_id,
            amount,
            to_amount,
            input.comment,
        ));
        #[rustfmt::skip]
        self.operations.push(OperationDraft::Transfer {
            from_account_id,
            from_amount: amount,
            to_account_id,
            to_amount,
        });
        Ok(())
    }

    /// По дате, внутри дня в порядке добавления.
    #[must_use]
    pub fn transfers(&self) -> &[Transfer] {
        &self.transfers
    }

    /// `One`, если валюты счетов совпадают, иначе `Two`.
    pub fn transfer_amount_fields(
        &self,
        from_account_id: AccountId,
        to_account_id: AccountId,
    ) -> Result<TransferAmountFields, DomainError> {
        if self.accounts_share_currency(from_account_id, to_account_id)? {
            Ok(TransferAmountFields::One)
        } else {
            Ok(TransferAmountFields::Two)
        }
    }

    fn credited_amount(
        &self,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
        to_amount: Option<i64>,
    ) -> Result<i64, DomainError> {
        if self.accounts_share_currency(from_account_id, to_account_id)? {
            return Ok(amount);
        }
        Ok(to_amount.unwrap_or(amount))
    }

    fn accounts_share_currency(
        &self,
        from_account_id: AccountId,
        to_account_id: AccountId,
    ) -> Result<bool, DomainError> {
        Ok(self.account(from_account_id)?.currency() == self.account(to_account_id)?.currency())
    }

    fn place_expense(&mut self, expense: Expense) {
        place_on_date(&mut self.expenses, expense, Expense::date);
    }

    fn place_income(&mut self, income: Income) {
        place_on_date(&mut self.incomes, income, Income::date);
    }

    fn place_transfer(&mut self, transfer: Transfer) {
        place_on_date(&mut self.transfers, transfer, Transfer::date);
    }

    fn prepare_category(&self, mut input: CreateCategory) -> Result<CreateCategory, DomainError> {
        let Some(parent_id) = input.parent_id else {
            return Ok(input);
        };
        let parent = self
            .find_category(parent_id)
            .ok_or(DomainError::CategoryNotFound)?;
        if parent.parent_id().is_some() {
            return Err(DomainError::CategoryTooDeep);
        }
        if input.applicability.is_none() {
            input.applicability = Some(parent.applicability());
        }
        Ok(input)
    }

    fn ensure_move_keeps_two_levels(
        &self,
        id: CategoryId,
        new_parent_id: CategoryId,
    ) -> Result<(), DomainError> {
        if self.find_category(id).is_none() {
            return Err(DomainError::CategoryNotFound);
        }
        let new_parent = self
            .find_category(new_parent_id)
            .ok_or(DomainError::CategoryNotFound)?;
        if id == new_parent_id || new_parent.parent_id().is_some() || self.has_subcategories(id) {
            return Err(DomainError::CategoryTooDeep);
        }
        Ok(())
    }

    fn has_subcategories(&self, id: CategoryId) -> bool {
        self.categories
            .iter()
            .any(|category| category.parent_id() == Some(id))
    }

    fn category_total(
        &self,
        id: CategoryId,
        slice: fn(&Self, CategoryId, &mut Option<Currency>) -> Result<i64, DomainError>,
    ) -> Result<i64, DomainError> {
        self.find_category(id)
            .ok_or(DomainError::CategoryNotFound)?;
        let mut currency = None;
        let mut total = slice(self, id, &mut currency)?;
        for child in self.direct_children(id) {
            total += slice(self, child, &mut currency)?;
        }
        Ok(total)
    }

    fn direct_children(&self, id: CategoryId) -> Vec<CategoryId> {
        self.categories
            .iter()
            .filter(|category| category.parent_id() == Some(id))
            .map(|category| category.id())
            .collect()
    }

    fn categories_where(&self, allows: impl Fn(Applicability) -> bool) -> Vec<&Category> {
        self.categories
            .iter()
            .filter(|category| allows(category.applicability()))
            .collect()
    }

    fn expense_slice(
        &self,
        id: CategoryId,
        currency: &mut Option<Currency>,
    ) -> Result<i64, DomainError> {
        self.saved_total(
            id,
            currency,
            self.expense_of(id),
            &self.expenses,
            expense_parts,
        )
    }

    fn income_slice(
        &self,
        id: CategoryId,
        currency: &mut Option<Currency>,
    ) -> Result<i64, DomainError> {
        self.saved_total(
            id,
            currency,
            self.income_of(id),
            &self.incomes,
            income_parts,
        )
    }

    fn saved_total<T>(
        &self,
        id: CategoryId,
        currency: &mut Option<Currency>,
        recorded: i64,
        saved: &[T],
        parts: impl Fn(&T) -> (CategoryId, AccountId, i64),
    ) -> Result<i64, DomainError> {
        let mut total = recorded;
        for item in saved {
            let (category_id, account_id, amount) = parts(item);
            if category_id != id {
                continue;
            }
            let account = self
                .find_account(account_id)
                .ok_or(DomainError::AccountNotFound)?;
            note_currency(currency, account.currency())?;
            total += amount;
        }
        Ok(total)
    }

    fn expense_of(&self, id: CategoryId) -> i64 {
        self.sum_of(id, |record| match record {
            Record::Expense {
                category_id,
                amount,
            } => Some((*category_id, *amount)),
            Record::Income { .. } => None,
        })
    }

    fn income_of(&self, id: CategoryId) -> i64 {
        self.sum_of(id, |record| match record {
            Record::Income {
                category_id,
                amount,
            } => Some((*category_id, *amount)),
            Record::Expense { .. } => None,
        })
    }

    fn sum_of(&self, id: CategoryId, parts: impl Fn(&Record) -> Option<(CategoryId, i64)>) -> i64 {
        self.records
            .iter()
            .filter_map(parts)
            .filter(|(category_id, _)| *category_id == id)
            .map(|(_, amount)| amount)
            .sum()
    }

    fn find_category(&self, id: CategoryId) -> Option<&Category> {
        self.categories.iter().find(|category| category.id() == id)
    }

    fn find_category_mut(&mut self, id: CategoryId) -> Option<&mut Category> {
        self.categories
            .iter_mut()
            .find(|category| category.id() == id)
    }

    fn find_account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.iter().find(|account| account.id() == id)
    }

    fn find_account_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.accounts.iter_mut().find(|account| account.id() == id)
    }

    fn ensure_currency_change_allowed(
        &self,
        id: AccountId,
        new_currency: &Currency,
    ) -> Result<(), DomainError> {
        let account = self.find_account(id).ok_or(DomainError::AccountNotFound)?;
        if account.currency() == new_currency {
            return Ok(());
        }
        if self.account_has_operations(id) {
            return Err(DomainError::CurrencyChangeNotAllowed);
        }
        Ok(())
    }

    fn account_has_operations(&self, id: AccountId) -> bool {
        self.operations
            .iter()
            .any(|operation| operation_touches_account(operation, id))
    }
}

fn expense_parts(expense: &Expense) -> (CategoryId, AccountId, i64) {
    (
        expense.category_id(),
        expense.account_id(),
        expense.amount(),
    )
}

#[rustfmt::skip]
fn income_parts(income: &Income) -> (CategoryId, AccountId, i64) {
    (
        income.category_id(),
        income.account_id(),
        income.amount(),
    )
}

fn note_currency(seen: &mut Option<Currency>, currency: &Currency) -> Result<(), DomainError> {
    match seen {
        None => *seen = Some(currency.clone()),
        Some(existing) if existing == currency => {}
        Some(_) => return Err(DomainError::MixedCurrencies),
    }
    Ok(())
}

fn place_on_date<T>(items: &mut Vec<T>, item: T, date_of: impl Fn(&T) -> SystemTime) {
    let date = date_of(&item);
    let index = items.partition_point(|existing| date_of(existing) <= date);
    items.insert(index, item);
}

fn complete_transfer(input: &NewTransfer) -> Result<(AccountId, AccountId, i64), DomainError> {
    match (input.from_account_id, input.to_account_id, input.amount) {
        (Some(from_account_id), Some(to_account_id), Some(amount)) => {
            Ok((from_account_id, to_account_id, amount))
        }
        _ => Err(DomainError::AccountNotFound),
    }
}

fn complete_income(input: &NewIncome) -> Result<(AccountId, CategoryId, i64), DomainError> {
    complete_fields(input.account_id, input.category_id, input.amount)
}

fn complete_expense(input: &NewExpense) -> Result<(AccountId, CategoryId, i64), DomainError> {
    complete_fields(input.account_id, input.category_id, input.amount)
}

fn complete_fields(
    account_id: Option<AccountId>,
    category_id: Option<CategoryId>,
    amount: Option<i64>,
) -> Result<(AccountId, CategoryId, i64), DomainError> {
    match (account_id, category_id, amount) {
        (Some(account_id), Some(category_id), Some(amount)) => {
            Ok((account_id, category_id, amount))
        }
        (account_id, category_id, amount) => Err(DomainError::MissingFields(MissingFields {
            account: account_id.is_none(),
            category: category_id.is_none(),
            amount: amount.is_none(),
        })),
    }
}

fn balance_effect(account_id: AccountId, operation: &OperationDraft) -> i64 {
    match operation {
        OperationDraft::Expense {
            account_id: id,
            amount,
        } if *id == account_id => -amount,
        OperationDraft::Income {
            account_id: id,
            amount,
        } if *id == account_id => *amount,
        OperationDraft::Lend {
            account_id: id,
            amount,
        } if *id == account_id => -amount,
        OperationDraft::Borrow {
            account_id: id,
            amount,
        } if *id == account_id => *amount,
        OperationDraft::Repayment {
            account_id: id,
            amount,
            direction: RepaymentDirection::Received,
        } if *id == account_id => *amount,
        OperationDraft::Repayment {
            account_id: id,
            amount,
            direction: RepaymentDirection::Given,
        } if *id == account_id => -amount,
        OperationDraft::Transfer {
            from_account_id,
            from_amount,
            to_account_id,
            to_amount,
        } => transfer_effect(
            account_id,
            *from_account_id,
            *from_amount,
            *to_account_id,
            *to_amount,
        ),
        _ => 0,
    }
}

fn transfer_effect(
    account_id: AccountId,
    from_account_id: AccountId,
    from_amount: i64,
    to_account_id: AccountId,
    to_amount: i64,
) -> i64 {
    let mut delta = 0;
    if from_account_id == account_id {
        delta -= from_amount;
    }
    if to_account_id == account_id {
        delta += to_amount;
    }
    delta
}

fn operation_touches_account(operation: &OperationDraft, id: AccountId) -> bool {
    match operation {
        OperationDraft::Expense { account_id, .. }
        | OperationDraft::Income { account_id, .. }
        | OperationDraft::Lend { account_id, .. }
        | OperationDraft::Borrow { account_id, .. }
        | OperationDraft::Repayment { account_id, .. } => *account_id == id,
        OperationDraft::Transfer {
            from_account_id,
            to_account_id,
            ..
        } => *from_account_id == id || *to_account_id == id,
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::Currency;
    use crate::error::MissingFields;
    use crate::{add_days, date};

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
    fn new_ledger_has_no_accounts() {
        let ledger = Ledger::new();
        assert!(ledger.accounts().is_empty());
    }

    #[test]
    fn create_account_assigns_id_and_stores_fields() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 100,
        })?;
        let account = ledger.account(id)?;
        assert_eq!(account.opening_balance(), 100);
        assert_eq!(ledger.balance(id)?, 100);
        Ok(())
    }

    #[test]
    fn expense_decreases_balance() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 100,
        })?;
        ledger.add_operation(OperationDraft::Expense {
            account_id: id,
            amount: 40,
        })?;
        assert_eq!(ledger.balance(id)?, 60);
        Ok(())
    }

    #[test]
    fn add_operation_without_accounts_fails() {
        let mut ledger = Ledger::new();
        let err = ledger.add_operation(OperationDraft::Expense {
            account_id: AccountId::from_u64(1),
            amount: 1,
        });
        assert_eq!(err, Err(DomainError::AccountRequired));
    }

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

    #[test]
    fn currency_change_blocked_when_account_has_operations() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })?;
        ledger.add_operation(OperationDraft::Income {
            account_id: id,
            amount: 1,
        })?;
        let err = ledger.update_account(
            id,
            UpdateAccount {
                name: None,
                opening_balance: None,
                currency: Some(Currency::new("USD")),
            },
        );
        assert_eq!(err, Err(DomainError::CurrencyChangeNotAllowed));
        Ok(())
    }

    #[test]
    fn saved_expense_keeps_date_account_category_amount_and_comment() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let account_id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })?;
        let category_id = ledger.create_category(CreateCategory {
            name: "Продукты".into(),
            parent_id: None,
            applicability: Some(Applicability::Expense),
        })?;
        let date = date(2026, 11, 22)?;

        ledger.add_expense(NewExpense {
            date,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(450),
            comment: "Хлеб".into(),
        })?;

        let expenses = ledger.expenses();
        assert_eq!(expenses.len(), 1);
        assert_eq!(expenses[0].date(), date);
        assert_eq!(expenses[0].account_id(), account_id);
        assert_eq!(expenses[0].category_id(), category_id);
        assert_eq!(expenses[0].amount(), 450);
        assert_eq!(expenses[0].comment(), "Хлеб");
        Ok(())
    }

    fn ledger_with_cash_and_groceries() -> Result<(Ledger, AccountId, CategoryId), DomainError> {
        ledger_with_cash("Продукты", crate::category::Applicability::Expense)
    }

    fn omit_if<T>(missing: bool, value: T) -> Option<T> {
        if missing { None } else { Some(value) }
    }

    fn assert_expense_rejected(missing: MissingFields) -> Result<(), DomainError> {
        let (mut ledger, account_id, category_id) = ledger_with_cash_and_groceries()?;
        let err = ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: omit_if(missing.account, account_id),
            category_id: omit_if(missing.category, category_id),
            amount: omit_if(missing.amount, 100),
            comment: String::new(),
        });
        assert_eq!(err, Err(DomainError::MissingFields(missing)));
        assert!(ledger.expenses().is_empty());
        assert_eq!(ledger.balance(account_id)?, 10_000);
        assert_eq!(ledger.expense_total(category_id)?, 0);
        Ok(())
    }

    #[test]
    fn expense_without_account_category_or_amount_is_rejected() -> Result<(), DomainError> {
        assert_expense_rejected(MissingFields {
            account: true,
            category: false,
            amount: false,
        })?;
        assert_expense_rejected(MissingFields {
            account: false,
            category: true,
            amount: false,
        })?;
        assert_expense_rejected(MissingFields {
            account: false,
            category: false,
            amount: true,
        })?;
        Ok(())
    }

    #[test]
    fn saved_expense_reduces_only_its_account_balance() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let rub = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })?;
        let usd = ledger.create_account(CreateAccount {
            name: "Доллары".into(),
            currency: Currency::new("USD"),
            opening_balance: 100,
        })?;
        let category_id = ledger.create_category(CreateCategory {
            name: "Продукты".into(),
            parent_id: None,
            applicability: Some(Applicability::Expense),
        })?;

        ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(usd),
            category_id: Some(category_id),
            amount: Some(15),
            comment: String::new(),
        })?;

        assert_eq!(ledger.balance(usd)?, 85);
        assert_eq!(ledger.balance(rub)?, 10_000);
        assert_eq!(ledger.expenses()[0].amount(), 15);
        assert_eq!(ledger.account(usd)?.currency().as_str(), "USD");
        Ok(())
    }

    #[test]
    fn saved_income_increases_only_its_account_balance() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let rub = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })?;
        let usd = ledger.create_account(CreateAccount {
            name: "Доллары".into(),
            currency: Currency::new("USD"),
            opening_balance: 100,
        })?;
        let category_id = ledger.create_category(CreateCategory {
            name: "Зарплата".into(),
            parent_id: None,
            applicability: Some(Applicability::Income),
        })?;

        ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(usd),
            category_id: Some(category_id),
            amount: Some(15),
            comment: String::new(),
        })?;

        assert_eq!(ledger.balance(usd)?, 115);
        assert_eq!(ledger.balance(rub)?, 10_000);
        assert_eq!(ledger.incomes()[0].amount(), 15);
        assert_eq!(ledger.account(usd)?.currency().as_str(), "USD");
        Ok(())
    }

    #[test]
    fn expense_on_parent_counts_only_toward_that_category() -> Result<(), DomainError> {
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

        ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(account_id),
            category_id: Some(food),
            amount: Some(1_200),
            comment: String::new(),
        })?;

        assert_eq!(ledger.expenses()[0].category_id(), food);
        assert_eq!(ledger.expense_total(food)?, 1_200);
        assert_eq!(ledger.expense_total(groceries)?, 0);
        Ok(())
    }

    #[test]
    fn income_on_parent_counts_only_toward_that_category() -> Result<(), DomainError> {
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
            parent_id: Some(salary),
            applicability: None,
        })?;

        ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(account_id),
            category_id: Some(salary),
            amount: Some(1_200),
            comment: String::new(),
        })?;

        assert_eq!(ledger.incomes()[0].category_id(), salary);
        assert_eq!(ledger.income_total(salary)?, 1_200);
        assert_eq!(ledger.income_total(bonus)?, 0);
        Ok(())
    }

    #[test]
    fn expense_on_earlier_date_appends_to_that_day() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let account_id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })?;
        let category_id = ledger.create_category(CreateCategory {
            name: "Продукты".into(),
            parent_id: None,
            applicability: Some(Applicability::Expense),
        })?;
        let yesterday = date(2026, 11, 21)?;
        let today = add_days(yesterday, 1)?;
        let expense = |date, amount, comment: &str| NewExpense {
            date,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(amount),
            comment: comment.into(),
        };

        ledger.add_expense(expense(yesterday, 100, "вчера"))?;
        ledger.add_expense(expense(today, 200, "сегодня"))?;
        ledger.add_expense(expense(yesterday, 300, "забытый"))?;

        let expenses = ledger.expenses();
        assert_eq!(expenses.len(), 3);
        assert_eq!(expenses[0].date(), yesterday);
        assert_eq!(expenses[0].amount(), 100);
        assert_eq!(expenses[1].date(), yesterday);
        assert_eq!(expenses[1].amount(), 300);
        assert_eq!(expenses[1].comment(), "забытый");
        assert_eq!(expenses[2].date(), today);
        assert_eq!(expenses[2].amount(), 200);
        Ok(())
    }

    #[test]
    fn income_on_earlier_date_appends_to_that_day() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let account_id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })?;
        let category_id = ledger.create_category(CreateCategory {
            name: "Зарплата".into(),
            parent_id: None,
            applicability: Some(Applicability::Income),
        })?;
        let yesterday = date(2026, 11, 21)?;
        let today = add_days(yesterday, 1)?;
        let income = |date, amount, comment: &str| NewIncome {
            date,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(amount),
            comment: comment.into(),
        };

        ledger.add_income(income(yesterday, 100, "вчера"))?;
        ledger.add_income(income(today, 200, "сегодня"))?;
        ledger.add_income(income(yesterday, 300, "забытый"))?;

        let incomes = ledger.incomes();
        assert_eq!(incomes.len(), 3);
        assert_eq!(incomes[0].date(), yesterday);
        assert_eq!(incomes[0].amount(), 100);
        assert_eq!(incomes[1].date(), yesterday);
        assert_eq!(incomes[1].amount(), 300);
        assert_eq!(incomes[1].comment(), "забытый");
        assert_eq!(incomes[2].date(), today);
        assert_eq!(incomes[2].amount(), 200);
        Ok(())
    }

    #[test]
    fn child_expense_counts_in_the_parent_total() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let account_id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
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
        ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(account_id),
            category_id: Some(groceries),
            amount: Some(400),
            comment: String::new(),
        })?;

        assert_eq!(ledger.expense_total(groceries)?, 400);
        assert_eq!(ledger.expense_total(food)?, 400);
        Ok(())
    }

    #[test]
    fn child_income_counts_in_the_parent_total() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let account_id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })?;
        let salary = ledger.create_category(CreateCategory {
            name: "Зарплата".into(),
            parent_id: None,
            applicability: Some(Applicability::Income),
        })?;
        let bonus = ledger.create_category(CreateCategory {
            name: "Премия".into(),
            parent_id: Some(salary),
            applicability: None,
        })?;
        ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(account_id),
            category_id: Some(bonus),
            amount: Some(400),
            comment: String::new(),
        })?;

        assert_eq!(ledger.income_total(bonus)?, 400);
        assert_eq!(ledger.income_total(salary)?, 400);
        Ok(())
    }

    #[test]
    fn expense_total_rejects_mixed_currencies() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let rub = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })?;
        let usd = ledger.create_account(CreateAccount {
            name: "Доллары".into(),
            currency: Currency::new("USD"),
            opening_balance: 0,
        })?;
        let food = ledger.create_category(CreateCategory {
            name: "Питание".into(),
            parent_id: None,
            applicability: Some(Applicability::Expense),
        })?;
        ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(rub),
            category_id: Some(food),
            amount: Some(1_000),
            comment: String::new(),
        })?;
        ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(usd),
            category_id: Some(food),
            amount: Some(15),
            comment: String::new(),
        })?;

        assert_eq!(ledger.expenses().len(), 2);
        assert_eq!(ledger.balance(rub)?, -1_000);
        assert_eq!(ledger.balance(usd)?, -15);
        assert_eq!(
            ledger.expense_total(food),
            Err(DomainError::MixedCurrencies)
        );
        Ok(())
    }

    #[test]
    fn income_total_rejects_mixed_currencies() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let rub = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })?;
        let usd = ledger.create_account(CreateAccount {
            name: "Доллары".into(),
            currency: Currency::new("USD"),
            opening_balance: 0,
        })?;
        let salary = ledger.create_category(CreateCategory {
            name: "Зарплата".into(),
            parent_id: None,
            applicability: Some(Applicability::Income),
        })?;
        ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(rub),
            category_id: Some(salary),
            amount: Some(1_000),
            comment: String::new(),
        })?;
        ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(usd),
            category_id: Some(salary),
            amount: Some(15),
            comment: String::new(),
        })?;

        assert_eq!(ledger.incomes().len(), 2);
        assert_eq!(ledger.balance(rub)?, 1_000);
        assert_eq!(ledger.balance(usd)?, 15);
        assert_eq!(
            ledger.income_total(salary),
            Err(DomainError::MixedCurrencies)
        );
        Ok(())
    }

    #[test]
    fn expense_with_unknown_account_or_category_is_rejected() -> Result<(), DomainError> {
        use crate::account::AccountId;
        use crate::category::CategoryId;

        let (mut ledger, account_id, category_id) = ledger_with_cash_and_groceries()?;
        let unknown_account = ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(AccountId::from_u64(99)),
            category_id: Some(category_id),
            amount: Some(100),
            comment: String::new(),
        });
        assert_eq!(unknown_account, Err(DomainError::AccountNotFound));
        assert!(ledger.expenses().is_empty());
        assert_eq!(ledger.balance(account_id)?, 10_000);

        let unknown_category = ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(account_id),
            category_id: Some(CategoryId::from_u64(99)),
            amount: Some(100),
            comment: String::new(),
        });
        assert_eq!(unknown_category, Err(DomainError::CategoryNotFound));
        assert!(ledger.expenses().is_empty());
        assert_eq!(ledger.expense_total(category_id)?, 0);
        Ok(())
    }

    #[test]
    fn income_with_unknown_account_or_category_is_rejected() -> Result<(), DomainError> {
        use crate::account::AccountId;
        use crate::category::CategoryId;

        let (mut ledger, account_id, category_id) = ledger_with_cash_and_salary()?;
        let unknown_account = ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(AccountId::from_u64(99)),
            category_id: Some(category_id),
            amount: Some(100),
            comment: String::new(),
        });
        assert_eq!(unknown_account, Err(DomainError::AccountNotFound));
        assert!(ledger.incomes().is_empty());
        assert_eq!(ledger.balance(account_id)?, 10_000);

        let unknown_category = ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(account_id),
            category_id: Some(CategoryId::from_u64(99)),
            amount: Some(100),
            comment: String::new(),
        });
        assert_eq!(unknown_category, Err(DomainError::CategoryNotFound));
        assert!(ledger.incomes().is_empty());
        assert_eq!(ledger.income_total(category_id)?, 0);
        Ok(())
    }

    #[test]
    fn saved_income_keeps_date_account_category_amount_and_comment() -> Result<(), DomainError> {
        use crate::category::Applicability;

        let mut ledger = Ledger::new();
        let account_id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })?;
        let category_id = ledger.create_category(CreateCategory {
            name: "Зарплата".into(),
            parent_id: None,
            applicability: Some(Applicability::Income),
        })?;
        let date = date(2026, 11, 22)?;

        ledger.add_income(NewIncome {
            date,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(450),
            comment: "Оклад".into(),
        })?;

        let incomes = ledger.incomes();
        assert_eq!(incomes.len(), 1);
        assert_eq!(incomes[0].date(), date);
        assert_eq!(incomes[0].account_id(), account_id);
        assert_eq!(incomes[0].category_id(), category_id);
        assert_eq!(incomes[0].amount(), 450);
        assert_eq!(incomes[0].comment(), "Оклад");
        Ok(())
    }

    fn ledger_with_cash_and_salary() -> Result<(Ledger, AccountId, CategoryId), DomainError> {
        ledger_with_cash("Зарплата", crate::category::Applicability::Income)
    }

    fn ledger_with_cash(
        category_name: &str,
        applicability: crate::category::Applicability,
    ) -> Result<(Ledger, AccountId, CategoryId), DomainError> {
        let mut ledger = Ledger::new();
        let account_id = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })?;
        let category_id = ledger.create_category(CreateCategory {
            name: category_name.into(),
            parent_id: None,
            applicability: Some(applicability),
        })?;
        Ok((ledger, account_id, category_id))
    }

    fn assert_income_rejected(missing: MissingFields) -> Result<(), DomainError> {
        let (mut ledger, account_id, category_id) = ledger_with_cash_and_salary()?;
        let err = ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: omit_if(missing.account, account_id),
            category_id: omit_if(missing.category, category_id),
            amount: omit_if(missing.amount, 100),
            comment: String::new(),
        });
        assert_eq!(err, Err(DomainError::MissingFields(missing)));
        assert!(ledger.incomes().is_empty());
        assert_eq!(ledger.balance(account_id)?, 10_000);
        assert_eq!(ledger.income_total(category_id)?, 0);
        Ok(())
    }

    #[test]
    fn income_without_account_category_or_amount_is_rejected() -> Result<(), DomainError> {
        assert_income_rejected(MissingFields {
            account: true,
            category: false,
            amount: false,
        })?;
        assert_income_rejected(MissingFields {
            account: false,
            category: true,
            amount: false,
        })?;
        assert_income_rejected(MissingFields {
            account: false,
            category: false,
            amount: true,
        })?;
        Ok(())
    }

    #[test]
    fn expense_without_any_account_is_rejected() -> Result<(), DomainError> {
        use crate::account::AccountId;
        use crate::category::CategoryId;

        let mut ledger = Ledger::new();
        let err = ledger.add_expense(NewExpense {
            date: date(2026, 11, 22)?,
            account_id: Some(AccountId::from_u64(1)),
            category_id: Some(CategoryId::from_u64(1)),
            amount: Some(1),
            comment: String::new(),
        });
        assert_eq!(err, Err(DomainError::AccountRequired));
        assert!(ledger.expenses().is_empty());
        Ok(())
    }

    #[test]
    fn transfer_within_one_currency_moves_the_amount() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let card = ledger.create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 1_000,
        })?;
        let day = date(2026, 11, 22)?;

        ledger.add_transfer(NewTransfer {
            date: day,
            from_account_id: Some(card),
            to_account_id: Some(cash),
            amount: Some(5_000),
            to_amount: None,
            comment: "на карман".into(),
        })?;

        assert_eq!(ledger.balance(card)?, 15_000);
        assert_eq!(ledger.balance(cash)?, 6_000);
        let transfers = ledger.transfers();
        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].date(), day);
        assert_eq!(transfers[0].from_account_id(), card);
        assert_eq!(transfers[0].to_account_id(), cash);
        assert_eq!(transfers[0].amount(), 5_000);
        assert_eq!(transfers[0].to_amount(), 5_000);
        assert_eq!(transfers[0].comment(), "на карман");
        Ok(())
    }

    #[test]
    fn transfer_between_currencies_moves_each_amount() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let card = ledger.create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        let dollars = ledger.create_account(CreateAccount {
            name: "Доллары".into(),
            currency: Currency::new("USD"),
            opening_balance: 100,
        })?;
        let day = date(2026, 11, 22)?;

        ledger.add_transfer(NewTransfer {
            date: day,
            from_account_id: Some(card),
            to_account_id: Some(dollars),
            amount: Some(7_500),
            to_amount: Some(80),
            comment: String::new(),
        })?;

        assert_eq!(ledger.balance(card)?, 12_500);
        assert_eq!(ledger.balance(dollars)?, 180);
        let transfers = ledger.transfers();
        assert_eq!(transfers.len(), 1);
        assert_eq!(transfers[0].amount(), 7_500);
        assert_eq!(transfers[0].to_amount(), 80);
        Ok(())
    }

    #[test]
    fn transfer_amount_fields_follow_account_currencies() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let card = ledger.create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })?;
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })?;
        let dollars = ledger.create_account(CreateAccount {
            name: "Доллары".into(),
            currency: Currency::new("USD"),
            opening_balance: 0,
        })?;

        assert_eq!(
            ledger.transfer_amount_fields(card, cash)?,
            TransferAmountFields::One
        );
        assert_eq!(
            ledger.transfer_amount_fields(card, dollars)?,
            TransferAmountFields::Two
        );
        assert_eq!(
            ledger.transfer_amount_fields(card, cash)?,
            TransferAmountFields::One
        );
        Ok(())
    }

    #[test]
    fn transfer_to_the_same_account_is_rejected() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let card = ledger.create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 1_000,
        })?;
        let day = date(2026, 11, 22)?;
        ledger.add_transfer(NewTransfer {
            date: day,
            from_account_id: Some(cash),
            to_account_id: Some(card),
            amount: Some(100),
            to_amount: None,
            comment: String::new(),
        })?;
        let saved = ledger.transfers().to_vec();

        let rejected = ledger.add_transfer(NewTransfer {
            date: day,
            from_account_id: Some(card),
            to_account_id: Some(card),
            amount: Some(5_000),
            to_amount: None,
            comment: String::new(),
        });

        assert_eq!(rejected, Err(DomainError::SameAccount));
        assert_eq!(ledger.transfers(), saved.as_slice());
        assert_eq!(ledger.balance(card)?, 20_100);
        assert_eq!(ledger.balance(cash)?, 900);
        Ok(())
    }

    #[test]
    fn transfer_on_earlier_date_appends_to_that_day() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let card = ledger.create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 1_000,
        })?;
        let yesterday = date(2026, 11, 21)?;
        let today = add_days(yesterday, 1)?;
        let transfer = |date, amount, comment: &str| NewTransfer {
            date,
            from_account_id: Some(card),
            to_account_id: Some(cash),
            amount: Some(amount),
            to_amount: None,
            comment: comment.into(),
        };

        ledger.add_transfer(transfer(yesterday, 100, "вчера"))?;
        ledger.add_transfer(transfer(today, 200, "сегодня"))?;
        ledger.add_transfer(transfer(yesterday, 300, "забытый"))?;

        let transfers = ledger.transfers();
        assert_eq!(transfers.len(), 3);
        assert_eq!(transfers[0].date(), yesterday);
        assert_eq!(transfers[0].amount(), 100);
        assert_eq!(transfers[1].date(), yesterday);
        assert_eq!(transfers[1].amount(), 300);
        assert_eq!(transfers[1].comment(), "забытый");
        assert_eq!(transfers[2].date(), today);
        assert_eq!(transfers[2].amount(), 200);
        Ok(())
    }
}
