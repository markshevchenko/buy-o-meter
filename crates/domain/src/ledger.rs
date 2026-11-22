use std::time::SystemTime;

use crate::account::{Account, AccountId, CreateAccount, UpdateAccount};
use crate::category::{Applicability, Category, CategoryId, CreateCategory, UpdateCategory};
use crate::counterparty::{Counterparty, CounterpartyId, Debt};
use crate::currency::Currency;
use crate::error::{
    DomainError, MissingDebtFields, MissingFields, MissingRepaymentFields, MissingTransferFields,
};
use crate::operation::{
    Borrow, Expense, Income, Lend, NewBorrow, NewExpense, NewIncome, NewLend, NewRepayment,
    NewTransfer, OperationDraft, Record, Repayment, RepaymentDirection, Transfer,
    TransferAmountFields,
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
    lends: Vec<Lend>,
    borrows: Vec<Borrow>,
    repayments: Vec<Repayment>,
    next_counterparty_id: u64,
    counterparties: Vec<Counterparty>,
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
            lends: Vec::new(),
            borrows: Vec::new(),
            repayments: Vec::new(),
            next_counterparty_id: 1,
            counterparties: Vec::new(),
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

    pub fn transfer_targets(&self, id: CategoryId) -> Result<Vec<&Category>, DomainError> {
        self.find_category(id)
            .ok_or(DomainError::CategoryNotFound)?;
        let keeps_income = !self.has_expenses(id);
        Ok(self
            .categories
            .iter()
            .filter(|category| {
                category.id() != id
                    && (keeps_income || category.applicability() != Applicability::Income)
            })
            .collect())
    }

    fn has_expenses(&self, id: CategoryId) -> bool {
        saved_expense(&self.expenses, id) || recorded_expense(&self.records, id)
    }

    pub fn delete_category(
        &mut self,
        id: CategoryId,
        transfer_to: Option<CategoryId>,
    ) -> Result<(), DomainError> {
        self.find_category(id)
            .ok_or(DomainError::CategoryNotFound)?;
        if self.has_subcategories(id) {
            return Err(DomainError::CategoryHasSubcategories);
        }
        self.relocate_operations(id, transfer_to)?;
        self.remove_category(id);
        Ok(())
    }

    fn has_incomes(&self, id: CategoryId) -> bool {
        saved_income(&self.incomes, id) || recorded_income(&self.records, id)
    }

    fn relocate_operations(
        &mut self,
        id: CategoryId,
        transfer_to: Option<CategoryId>,
    ) -> Result<(), DomainError> {
        if !self.has_expenses(id) && !self.has_incomes(id) {
            return Ok(());
        }
        let target = transfer_to.ok_or(DomainError::TransferTargetRequired)?;
        self.ensure_transfer_target(id, target)?;
        self.move_operations(id, target);
        Ok(())
    }

    fn ensure_transfer_target(
        &self,
        id: CategoryId,
        target: CategoryId,
    ) -> Result<(), DomainError> {
        let suitable = self
            .transfer_targets(id)?
            .iter()
            .any(|category| category.id() == target);
        if suitable {
            Ok(())
        } else {
            Err(DomainError::UnsuitableTransferTarget)
        }
    }

    fn remove_category(&mut self, id: CategoryId) {
        self.categories.retain(|category| category.id() != id);
    }

    fn move_operations(&mut self, from: CategoryId, to: CategoryId) {
        for expense in &mut self.expenses {
            if expense.category_id() == from {
                expense.set_category_id(to);
            }
        }
        for income in &mut self.incomes {
            if income.category_id() == from {
                income.set_category_id(to);
            }
        }
        move_records(&mut self.records, from, to);
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
    pub fn add_transfer(&mut self, input: NewTransfer) -> Result<(), DomainError> {
        let (from_account_id, to_account_id, amount) = complete_transfer(&input)?;
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
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

    /// Пустые счёт, контрагент или сумма — `DomainError::MissingDebtFields`, списки не меняются.
    /// Имя копируется в операцию. Новое имя попадает в список контрагентов, известное не дублируется.
    /// Остаток счёта уменьшается. Долг мне этому имени растёт в валюте счёта.
    pub fn add_lend(&mut self, input: NewLend) -> Result<(), DomainError> {
        let (account_id, name, amount) =
            complete_debt(input.account_id, input.counterparty, input.amount)?;
        self.ensure_account(account_id)?;
        self.remember_counterparty(&name);
        self.place_lend(Lend::new(input.date, account_id, name, amount));
        #[rustfmt::skip]
        self.operations.push(OperationDraft::Lend {
            account_id,
            amount,
        });
        Ok(())
    }

    /// По дате, внутри дня в порядке добавления.
    #[must_use]
    pub fn lends(&self) -> &[Lend] {
        &self.lends
    }

    /// Пустые счёт, контрагент или сумма — `DomainError::MissingDebtFields`, списки не меняются.
    /// Имя копируется в операцию. Новое имя попадает в список контрагентов, известное не дублируется.
    /// Остаток счёта увеличивается. Мой долг этому имени растёт в валюте счёта.
    pub fn add_borrow(&mut self, input: NewBorrow) -> Result<(), DomainError> {
        let (account_id, name, amount) =
            complete_debt(input.account_id, input.counterparty, input.amount)?;
        self.ensure_account(account_id)?;
        self.remember_counterparty(&name);
        self.place_borrow(Borrow::new(input.date, account_id, name, amount));
        #[rustfmt::skip]
        self.operations.push(OperationDraft::Borrow {
            account_id,
            amount,
        });
        Ok(())
    }

    /// По дате, внутри дня в порядке добавления.
    #[must_use]
    pub fn borrows(&self) -> &[Borrow] {
        &self.borrows
    }

    /// Пустые счёт, контрагент, направление или сумма —
    /// `DomainError::MissingRepaymentFields`, списки не меняются.
    /// Нет открытого долга этому имени в выбранном направлении и валюте счёта —
    /// `DomainError::NoDebt`.
    /// Сумма больше остатка — `DomainError::RepaymentExceedsDebt`,
    /// в `remaining` остаток долга; операция не сохраняется.
    /// `RepaymentDirection::Received` («мне вернули») уменьшает долг мне и увеличивает остаток счёта.
    /// `RepaymentDirection::Given` («я вернул») уменьшает мой долг и уменьшает остаток счёта.
    /// Встречное направление не меняется. Имя копируется в операцию.
    pub fn add_repayment(&mut self, input: NewRepayment) -> Result<(), DomainError> {
        let (account_id, name, direction, amount) = complete_repayment(
            input.account_id,
            input.counterparty,
            input.direction,
            input.amount,
        )?;
        self.ensure_account(account_id)?;
        let remaining = self.debt_remaining(account_id, &name, direction)?;
        if amount > remaining {
            return Err(DomainError::RepaymentExceedsDebt { remaining });
        }
        #[rustfmt::skip]
        self.place_repayment(Repayment::new(
            input.date,
            account_id,
            name,
            direction,
            amount,
        ));
        #[rustfmt::skip]
        self.operations.push(OperationDraft::Repayment {
            account_id,
            amount,
            direction,
        });
        Ok(())
    }

    /// По дате, внутри дня в порядке добавления.
    #[must_use]
    pub fn repayments(&self) -> &[Repayment] {
        &self.repayments
    }

    /// Имена для выбора, в порядке появления.
    #[must_use]
    pub fn counterparties(&self) -> &[Counterparty] {
        &self.counterparties
    }

    /// Меняет имя только в списке. В операциях остаётся прежнее.
    /// Нет такого id — `CounterpartyNotFound`.
    /// Ненулевой долг по текущему имени в любую сторону — `CounterpartyHasDebts`, список не меняется.
    /// Нулевой остаток после возврата не мешает.
    pub fn rename_counterparty(
        &mut self,
        id: CounterpartyId,
        name: String,
    ) -> Result<(), DomainError> {
        self.reject_if_counterparty_has_debts(id)?;
        let _ = name;
        todo!()
    }

    /// Убирает имя из списка. В операциях оно остаётся.
    /// Нет такого id — `CounterpartyNotFound`.
    /// Ненулевой долг по текущему имени в любую сторону — `CounterpartyHasDebts`, список не меняется.
    /// Нулевой остаток после возврата не мешает.
    pub fn delete_counterparty(&mut self, id: CounterpartyId) -> Result<(), DomainError> {
        self.reject_if_counterparty_has_debts(id)?;
        todo!()
    }

    /// Кому я дал и ещё не получил обратно: выдачи минус возвраты «мне вернули».
    /// Одна запись на имя и валюту, сумма больше нуля.
    /// Встречный долг в этот список не сворачивается.
    #[must_use]
    pub fn debts_to_me(&self) -> Vec<Debt> {
        self.open_debts(&self.lends, lend_parts, RepaymentDirection::Received)
    }

    /// У кого я взял и ещё не отдал: займы минус возвраты «я вернул».
    /// Одна запись на имя и валюту, сумма больше нуля.
    /// Встречный долг в этот список не сворачивается.
    #[must_use]
    pub fn my_debts(&self) -> Vec<Debt> {
        self.open_debts(&self.borrows, borrow_parts, RepaymentDirection::Given)
    }

    fn debt_remaining(
        &self,
        account_id: AccountId,
        name: &str,
        direction: RepaymentDirection,
    ) -> Result<i64, DomainError> {
        let currency = self.account(account_id)?.currency().clone();
        let debts = match direction {
            RepaymentDirection::Received => self.debts_to_me(),
            RepaymentDirection::Given => self.my_debts(),
        };
        Ok(debts
            .iter()
            .find(|debt| debt.counterparty_name() == name && debt.currency() == &currency)
            .map(Debt::amount)
            .unwrap_or(0))
    }

    fn open_debts<T>(
        &self,
        items: &[T],
        parts: impl Fn(&T) -> (&str, AccountId, i64),
        repaid_by: RepaymentDirection,
    ) -> Vec<Debt> {
        let mut totals = balance_totals(self, items, parts);
        subtract_repayments(&mut totals, self, repaid_by);
        positive_debts(totals)
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
        to_amount.ok_or(DomainError::MissingTransferFields(MissingTransferFields {
            from_account: false,
            to_account: false,
            amount: false,
            to_amount: true,
        }))
    }

    fn accounts_share_currency(
        &self,
        from_account_id: AccountId,
        to_account_id: AccountId,
    ) -> Result<bool, DomainError> {
        Ok(self.account(from_account_id)?.currency() == self.account(to_account_id)?.currency())
    }

    fn ensure_account(&self, account_id: AccountId) -> Result<(), DomainError> {
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        if self.find_account(account_id).is_none() {
            return Err(DomainError::AccountNotFound);
        }
        Ok(())
    }

    fn reject_if_counterparty_has_debts(&self, id: CounterpartyId) -> Result<(), DomainError> {
        let name = self
            .counterparties
            .iter()
            .find(|counterparty| counterparty.id() == id)
            .map(Counterparty::name)
            .ok_or(DomainError::CounterpartyNotFound)?;
        if has_open_debt(&self.debts_to_me(), name) || has_open_debt(&self.my_debts(), name) {
            return Err(DomainError::CounterpartyHasDebts);
        }
        Ok(())
    }

    fn remember_counterparty(&mut self, name: &str) {
        if self
            .counterparties
            .iter()
            .any(|counterparty| counterparty.name() == name)
        {
            return;
        }
        let id = CounterpartyId::from_u64(self.next_counterparty_id);
        self.next_counterparty_id += 1;
        self.counterparties
            .push(Counterparty::new(id, name.to_owned()));
    }

    fn place_lend(&mut self, lend: Lend) {
        place_on_date(&mut self.lends, lend, Lend::date);
    }

    fn place_borrow(&mut self, borrow: Borrow) {
        place_on_date(&mut self.borrows, borrow, Borrow::date);
    }

    fn place_repayment(&mut self, repayment: Repayment) {
        place_on_date(&mut self.repayments, repayment, Repayment::date);
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
        (from_account_id, to_account_id, amount) => {
            Err(DomainError::MissingTransferFields(MissingTransferFields {
                from_account: from_account_id.is_none(),
                to_account: to_account_id.is_none(),
                amount: amount.is_none(),
                to_amount: false,
            }))
        }
    }
}

fn saved_expense(expenses: &[Expense], id: CategoryId) -> bool {
    expenses.iter().any(|expense| expense.category_id() == id)
}

fn saved_income(incomes: &[Income], id: CategoryId) -> bool {
    incomes.iter().any(|income| income.category_id() == id)
}

fn recorded_expense(records: &[Record], id: CategoryId) -> bool {
    records.iter().any(|record| match record {
        Record::Expense { category_id, .. } => *category_id == id,
        Record::Income { .. } => false,
    })
}

fn recorded_income(records: &[Record], id: CategoryId) -> bool {
    records.iter().any(|record| match record {
        Record::Income { category_id, .. } => *category_id == id,
        Record::Expense { .. } => false,
    })
}

fn move_records(records: &mut [Record], from: CategoryId, to: CategoryId) {
    for record in records {
        let category_id = match record {
            Record::Expense { category_id, .. } | Record::Income { category_id, .. } => category_id,
        };
        if *category_id == from {
            *category_id = to;
        }
    }
}

fn has_open_debt(debts: &[Debt], name: &str) -> bool {
    debts.iter().any(|debt| debt.counterparty_name() == name)
}

fn balance_totals<T>(
    ledger: &Ledger,
    items: &[T],
    parts: impl Fn(&T) -> (&str, AccountId, i64),
) -> Vec<(String, Currency, i64)> {
    let mut totals = Vec::new();
    for item in items {
        let (name, account_id, amount) = parts(item);
        add_account_amount(ledger, &mut totals, name, account_id, amount);
    }
    totals
}

fn subtract_repayments(
    totals: &mut Vec<(String, Currency, i64)>,
    ledger: &Ledger,
    direction: RepaymentDirection,
) {
    for repayment in &ledger.repayments {
        if repayment.direction() != direction {
            continue;
        }
        add_account_amount(
            ledger,
            totals,
            repayment.counterparty_name(),
            repayment.account_id(),
            -repayment.amount(),
        );
    }
}

fn add_account_amount(
    ledger: &Ledger,
    totals: &mut Vec<(String, Currency, i64)>,
    name: &str,
    account_id: AccountId,
    amount: i64,
) {
    let Some(currency) = ledger
        .find_account(account_id)
        .map(|account| account.currency().clone())
    else {
        return;
    };
    add_balance(totals, name, currency, amount);
}

fn positive_debts(totals: Vec<(String, Currency, i64)>) -> Vec<Debt> {
    totals
        .into_iter()
        .filter(|(_, _, amount)| *amount > 0)
        .map(|(name, currency, amount)| Debt::new(name, currency, amount))
        .collect()
}

fn add_balance(
    totals: &mut Vec<(String, Currency, i64)>,
    name: &str,
    currency: Currency,
    amount: i64,
) {
    if let Some((_, _, total)) = totals
        .iter_mut()
        .find(|(existing, existing_currency, _)| existing == name && existing_currency == &currency)
    {
        *total += amount;
        return;
    }
    totals.push((name.to_owned(), currency, amount));
}

#[rustfmt::skip]
fn lend_parts(lend: &Lend) -> (&str, AccountId, i64) {
    (
        lend.counterparty_name(),
        lend.account_id(),
        lend.amount(),
    )
}

#[rustfmt::skip]
fn borrow_parts(borrow: &Borrow) -> (&str, AccountId, i64) {
    (
        borrow.counterparty_name(),
        borrow.account_id(),
        borrow.amount(),
    )
}

fn complete_repayment(
    account_id: Option<AccountId>,
    counterparty: Option<String>,
    direction: Option<RepaymentDirection>,
    amount: Option<i64>,
) -> Result<(AccountId, String, RepaymentDirection, i64), DomainError> {
    match (account_id, counterparty, direction, amount) {
        (Some(account_id), Some(counterparty), Some(direction), Some(amount)) => {
            Ok((account_id, counterparty, direction, amount))
        }
        (account_id, counterparty, direction, amount) => Err(DomainError::MissingRepaymentFields(
            MissingRepaymentFields {
                account: account_id.is_none(),
                counterparty: counterparty.is_none(),
                direction: direction.is_none(),
                amount: amount.is_none(),
            },
        )),
    }
}

fn complete_debt(
    account_id: Option<AccountId>,
    counterparty: Option<String>,
    amount: Option<i64>,
) -> Result<(AccountId, String, i64), DomainError> {
    match (account_id, counterparty, amount) {
        (Some(account_id), Some(counterparty), Some(amount)) => {
            Ok((account_id, counterparty, amount))
        }
        (account_id, counterparty, amount) => {
            Err(DomainError::MissingDebtFields(MissingDebtFields {
                account: account_id.is_none(),
                counterparty: counterparty.is_none(),
                amount: amount.is_none(),
            }))
        }
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
    use crate::error::{MissingFields, MissingTransferFields};
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

    fn card_and_cash() -> Result<(Ledger, AccountId, AccountId), DomainError> {
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
        Ok((ledger, card, cash))
    }

    fn transfer_input(
        from_account_id: Option<AccountId>,
        to_account_id: Option<AccountId>,
        amount: Option<i64>,
        to_amount: Option<i64>,
    ) -> Result<NewTransfer, DomainError> {
        Ok(NewTransfer {
            date: date(2026, 11, 22)?,
            from_account_id,
            to_account_id,
            amount,
            to_amount,
            comment: String::new(),
        })
    }

    fn assert_transfer_fields_rejected(missing: MissingTransferFields) -> Result<(), DomainError> {
        let (mut ledger, card, cash) = card_and_cash()?;
        let err = ledger.add_transfer(transfer_input(
            omit_if(missing.from_account, card),
            omit_if(missing.to_account, cash),
            omit_if(missing.amount, 100),
            None,
        )?);
        assert_eq!(err, Err(DomainError::MissingTransferFields(missing)));
        assert!(ledger.transfers().is_empty());
        assert_eq!(ledger.balance(card)?, 20_000);
        assert_eq!(ledger.balance(cash)?, 1_000);
        Ok(())
    }

    #[test]
    fn transfer_without_accounts_or_amount_is_rejected() -> Result<(), DomainError> {
        assert_transfer_fields_rejected(MissingTransferFields {
            from_account: true,
            to_account: false,
            amount: false,
            to_amount: false,
        })?;
        assert_transfer_fields_rejected(MissingTransferFields {
            from_account: false,
            to_account: true,
            amount: false,
            to_amount: false,
        })?;
        assert_transfer_fields_rejected(MissingTransferFields {
            from_account: false,
            to_account: false,
            amount: true,
            to_amount: false,
        })?;
        Ok(())
    }

    #[test]
    fn transfer_between_currencies_without_credited_amount_is_rejected() -> Result<(), DomainError>
    {
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

        let err = ledger.add_transfer(transfer_input(
            Some(card),
            Some(dollars),
            Some(7_500),
            None,
        )?);

        assert_eq!(
            err,
            Err(DomainError::MissingTransferFields(MissingTransferFields {
                from_account: false,
                to_account: false,
                amount: false,
                to_amount: true,
            }))
        );
        assert!(ledger.transfers().is_empty());
        assert_eq!(ledger.balance(card)?, 20_000);
        assert_eq!(ledger.balance(dollars)?, 100);
        Ok(())
    }

    #[test]
    fn transfer_without_any_account_is_rejected() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let err = ledger.add_transfer(transfer_input(
            Some(AccountId::from_u64(1)),
            Some(AccountId::from_u64(1)),
            Some(1),
            None,
        )?);
        assert_eq!(err, Err(DomainError::AccountRequired));
        assert!(ledger.transfers().is_empty());
        Ok(())
    }

    #[test]
    fn transfer_with_unknown_account_is_rejected() -> Result<(), DomainError> {
        let (mut ledger, card, cash) = card_and_cash()?;
        let unknown_from = ledger.add_transfer(transfer_input(
            Some(AccountId::from_u64(99)),
            Some(cash),
            Some(100),
            None,
        )?);
        assert_eq!(unknown_from, Err(DomainError::AccountNotFound));
        assert!(ledger.transfers().is_empty());
        assert_eq!(ledger.balance(card)?, 20_000);
        assert_eq!(ledger.balance(cash)?, 1_000);

        let unknown_to = ledger.add_transfer(transfer_input(
            Some(card),
            Some(AccountId::from_u64(99)),
            Some(100),
            None,
        )?);
        assert_eq!(unknown_to, Err(DomainError::AccountNotFound));
        assert!(ledger.transfers().is_empty());
        assert_eq!(ledger.balance(card)?, 20_000);
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

    #[test]
    fn saving_lend_adds_new_counterparty() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;

        assert!(ledger.counterparties().is_empty());

        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Маша".into()),
            amount: Some(1_000),
        })?;

        assert_eq!(ledger.counterparties().len(), 1);
        assert_eq!(ledger.counterparties()[0].name(), "Маша");
        Ok(())
    }

    #[test]
    fn lend_keeps_counterparty_name_after_list_grows() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;

        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(10_000),
        })?;
        assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");

        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Маша".into()),
            amount: Some(500),
        })?;
        assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
        Ok(())
    }

    #[test]
    fn counterparty_with_lend_cannot_be_renamed_or_deleted() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(10_000),
        })?;

        let id = ledger.counterparties()[0].id();
        let before = ledger.counterparties().to_vec();
        assert_eq!(
            ledger.rename_counterparty(id, "Пётр".into()),
            Err(DomainError::CounterpartyHasDebts)
        );
        assert_eq!(
            ledger.delete_counterparty(id),
            Err(DomainError::CounterpartyHasDebts)
        );
        assert_eq!(ledger.counterparties(), before.as_slice());
        assert_eq!(ledger.lends()[0].counterparty_name(), "Петя");
        assert_eq!(ledger.lends()[0].amount(), 10_000);
        Ok(())
    }

    #[test]
    fn lending_reduces_account_balance() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(10_000),
        })?;
        assert_eq!(ledger.balance(cash)?, 10_000);
        Ok(())
    }

    #[test]
    fn lending_adds_debt_to_me_in_account_currency() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(10_000),
        })?;

        let debts = ledger.debts_to_me();
        assert_eq!(debts.len(), 1);
        assert_eq!(debts[0].counterparty_name(), "Петя");
        assert_eq!(debts[0].currency().as_str(), "RUB");
        assert_eq!(debts[0].amount(), 10_000);
        assert!(ledger.my_debts().is_empty());
        Ok(())
    }

    #[test]
    fn second_lend_to_same_name_keeps_one_counterparty() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 50_000,
        })?;
        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(10_000),
        })?;
        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(3_000),
        })?;

        assert_eq!(ledger.counterparties().len(), 1);
        assert_eq!(ledger.counterparties()[0].name(), "Петя");
        Ok(())
    }

    #[test]
    fn borrowing_increases_account_balance_and_my_debt() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let card = ledger.create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 1_000,
        })?;
        ledger.add_borrow(NewBorrow {
            date: date(2026, 11, 22)?,
            account_id: Some(card),
            counterparty: Some("Маша".into()),
            amount: Some(5_000),
        })?;

        assert_eq!(ledger.balance(card)?, 6_000);
        assert_eq!(ledger.my_debts().len(), 1);
        assert_eq!(ledger.my_debts()[0].counterparty_name(), "Маша");
        assert_eq!(ledger.my_debts()[0].currency().as_str(), "RUB");
        assert_eq!(ledger.my_debts()[0].amount(), 5_000);
        assert!(ledger.debts_to_me().is_empty());
        assert_eq!(ledger.borrows()[0].counterparty_name(), "Маша");
        Ok(())
    }

    #[test]
    fn counterparty_with_borrow_cannot_be_renamed_or_deleted() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let card = ledger.create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 1_000,
        })?;
        ledger.add_borrow(NewBorrow {
            date: date(2026, 11, 22)?,
            account_id: Some(card),
            counterparty: Some("Маша".into()),
            amount: Some(5_000),
        })?;

        let id = ledger.counterparties()[0].id();
        let before = ledger.counterparties().to_vec();
        assert_eq!(
            ledger.rename_counterparty(id, "Мария".into()),
            Err(DomainError::CounterpartyHasDebts)
        );
        assert_eq!(
            ledger.delete_counterparty(id),
            Err(DomainError::CounterpartyHasDebts)
        );
        assert_eq!(ledger.counterparties(), before.as_slice());
        assert_eq!(ledger.borrows()[0].counterparty_name(), "Маша");
        assert_eq!(ledger.borrows()[0].amount(), 5_000);
        Ok(())
    }

    #[test]
    fn received_repayment_reduces_debt_to_me_and_increases_balance() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        ledger.add_lend(NewLend {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(10_000),
        })?;

        ledger.add_repayment(NewRepayment {
            date: date(2026, 11, 22)?,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            direction: Some(RepaymentDirection::Received),
            amount: Some(2_000),
        })?;

        assert_eq!(ledger.balance(cash)?, 12_000);
        let debts = ledger.debts_to_me();
        assert_eq!(debts.len(), 1);
        assert_eq!(debts[0].counterparty_name(), "Петя");
        assert_eq!(debts[0].currency().as_str(), "RUB");
        assert_eq!(debts[0].amount(), 8_000);
        assert!(ledger.my_debts().is_empty());
        assert_eq!(ledger.repayments()[0].counterparty_name(), "Петя");
        assert_eq!(
            ledger.repayments()[0].direction(),
            RepaymentDirection::Received
        );
        assert_eq!(ledger.repayments()[0].amount(), 2_000);
        Ok(())
    }

    #[test]
    fn given_repayment_reduces_my_debt_and_leaves_debt_to_me() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 50_000,
        })?;
        let day = date(2026, 11, 22)?;
        ledger.add_lend(NewLend {
            date: day,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(10_000),
        })?;
        ledger.add_borrow(NewBorrow {
            date: day,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(3_000),
        })?;

        ledger.add_repayment(NewRepayment {
            date: day,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            direction: Some(RepaymentDirection::Given),
            amount: Some(2_000),
        })?;

        assert_eq!(ledger.balance(cash)?, 41_000);
        assert_eq!(ledger.my_debts().len(), 1);
        assert_eq!(ledger.my_debts()[0].counterparty_name(), "Петя");
        assert_eq!(ledger.my_debts()[0].currency().as_str(), "RUB");
        assert_eq!(ledger.my_debts()[0].amount(), 1_000);
        assert_eq!(ledger.debts_to_me().len(), 1);
        assert_eq!(ledger.debts_to_me()[0].amount(), 10_000);
        assert_eq!(ledger.repayments()[0].counterparty_name(), "Петя");
        assert_eq!(
            ledger.repayments()[0].direction(),
            RepaymentDirection::Given
        );

        ledger.add_repayment(NewRepayment {
            date: day,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            direction: Some(RepaymentDirection::Given),
            amount: Some(1_000),
        })?;

        assert!(ledger.my_debts().is_empty());
        assert_eq!(ledger.debts_to_me()[0].amount(), 10_000);
        assert_eq!(ledger.balance(cash)?, 40_000);
        Ok(())
    }

    #[test]
    fn repayment_above_remaining_is_rejected() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = ledger.create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 20_000,
        })?;
        let day = date(2026, 11, 22)?;
        ledger.add_lend(NewLend {
            date: day,
            account_id: Some(cash),
            counterparty: Some("Петя".into()),
            amount: Some(5_000),
        })?;
        let balance_before = ledger.balance(cash)?;

        assert_eq!(
            ledger.add_repayment(NewRepayment {
                date: day,
                account_id: Some(cash),
                counterparty: Some("Петя".into()),
                direction: Some(RepaymentDirection::Received),
                amount: Some(6_000),
            }),
            Err(DomainError::RepaymentExceedsDebt { remaining: 5_000 })
        );
        assert_eq!(ledger.debts_to_me()[0].amount(), 5_000);
        assert_eq!(ledger.balance(cash)?, balance_before);
        assert!(ledger.repayments().is_empty());
        Ok(())
    }
}
