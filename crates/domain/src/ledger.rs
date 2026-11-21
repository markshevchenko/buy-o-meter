use crate::account::{Account, AccountId, CreateAccount, UpdateAccount};
use crate::category::{
    Applicability, Category, CategoryAmount, CategoryId, CreateCategory, EntryKind, UpdateCategory,
};
use crate::error::DomainError;
use crate::operation::{OperationDraft, RepaymentDirection};

#[derive(Debug)]
pub struct Ledger {
    next_account_id: u64,
    accounts: Vec<Account>,
    operations: Vec<OperationDraft>,
    next_category_id: u64,
    categories: Vec<Category>,
    category_amounts: Vec<CategoryAmount>,
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
            category_amounts: Vec::new(),
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

    pub fn add_category_amount(&mut self, amount: CategoryAmount) -> Result<(), DomainError> {
        self.category_amounts.push(amount);
        Ok(())
    }

    #[must_use]
    pub fn category_amounts(&self) -> &[CategoryAmount] {
        &self.category_amounts
    }

    /// Свои суммы этого вида плюс суммы всех подкатегорий.
    pub fn category_total(&self, id: CategoryId, kind: EntryKind) -> Result<i64, DomainError> {
        self.find_category(id)
            .ok_or(DomainError::CategoryNotFound)?;
        Ok(self.amount_of(id, kind) + self.direct_children_amount(id, kind))
    }

    /// Категории, которые можно выбрать при вводе операции этого вида.
    #[must_use]
    pub fn categories_for(&self, kind: EntryKind) -> Vec<&Category> {
        self.categories
            .iter()
            .filter(|category| applies_to(category.applicability(), kind))
            .collect()
    }

    pub fn add_operation(&mut self, draft: OperationDraft) -> Result<(), DomainError> {
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        self.operations.push(draft);
        Ok(())
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
        if new_parent.parent_id().is_some() || self.has_subcategories(id) {
            return Err(DomainError::CategoryTooDeep);
        }
        Ok(())
    }

    fn has_subcategories(&self, id: CategoryId) -> bool {
        self.categories
            .iter()
            .any(|category| category.parent_id() == Some(id))
    }

    fn amount_of(&self, id: CategoryId, kind: EntryKind) -> i64 {
        self.category_amounts
            .iter()
            .filter(|amount| amount.category_id == id && amount.kind == kind)
            .map(|amount| amount.amount)
            .sum()
    }

    fn direct_children_amount(&self, id: CategoryId, kind: EntryKind) -> i64 {
        self.categories
            .iter()
            .filter(|category| category.parent_id() == Some(id))
            .map(|category| self.amount_of(category.id(), kind))
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
        new_currency: &crate::currency::Currency,
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

fn applies_to(applicability: Applicability, kind: EntryKind) -> bool {
    match kind {
        EntryKind::Expense => {
            matches!(applicability, Applicability::Expense | Applicability::Both)
        }
        EntryKind::Income => matches!(applicability, Applicability::Income | Applicability::Both),
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

        let for_expense = ledger.categories_for(EntryKind::Expense);
        let for_income = ledger.categories_for(EntryKind::Income);
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
        ledger.add_category_amount(CategoryAmount {
            category_id: lunches,
            kind: EntryKind::Expense,
            amount: 1_500,
        })?;

        ledger.move_category(lunches, leisure)?;

        assert_eq!(ledger.category(lunches)?.parent_id(), Some(leisure));
        assert_eq!(ledger.category_total(lunches, EntryKind::Expense)?, 1_500);
        assert_eq!(ledger.category_total(leisure, EntryKind::Expense)?, 1_500);
        assert_eq!(ledger.category_total(food, EntryKind::Expense)?, 0);
        assert_eq!(
            ledger.category_amounts(),
            &[CategoryAmount {
                category_id: lunches,
                kind: EntryKind::Expense,
                amount: 1_500,
            }]
        );
        Ok(())
    }

    #[test]
    fn category_total_adds_own_amounts_and_direct_children() -> Result<(), DomainError> {
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
        ledger.add_category_amount(CategoryAmount {
            category_id: food,
            kind: EntryKind::Expense,
            amount: 400,
        })?;
        ledger.add_category_amount(CategoryAmount {
            category_id: lunches,
            kind: EntryKind::Expense,
            amount: 1_500,
        })?;
        ledger.add_category_amount(CategoryAmount {
            category_id: lunches,
            kind: EntryKind::Income,
            amount: 50,
        })?;

        assert_eq!(ledger.category_total(food, EntryKind::Expense)?, 1_900);
        assert_eq!(ledger.category_total(lunches, EntryKind::Expense)?, 1_500);
        assert_eq!(ledger.category_total(food, EntryKind::Income)?, 50);
        assert_eq!(ledger.category_total(lunches, EntryKind::Income)?, 50);
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
        ledger.add_category_amount(CategoryAmount {
            category_id: id,
            kind: EntryKind::Expense,
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
            ledger.category_amounts(),
            &[CategoryAmount {
                category_id: id,
                kind: EntryKind::Expense,
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
}
