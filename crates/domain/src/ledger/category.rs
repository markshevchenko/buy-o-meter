use super::Ledger;
use crate::account::AccountId;
use crate::category::{Applicability, Category, CategoryId, CreateCategory, UpdateCategory};
use crate::currency::Currency;
use crate::error::DomainError;
use crate::operation::{Expense, Income, Record};
use crate::starter::{EXPENSE_TREE, INCOME_TREE, StarterGroup};

impl Ledger {
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
