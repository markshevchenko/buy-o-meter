use crate::account::{Account, AccountId, CreateAccount, UpdateAccount};
use crate::category::{
    Category, CategoryAmount, CategoryId, CreateCategory, EntryKind, UpdateCategory,
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
        let _ = (id, input);
        todo!()
    }

    pub fn move_category(
        &mut self,
        id: CategoryId,
        new_parent_id: CategoryId,
    ) -> Result<(), DomainError> {
        let _ = (id, new_parent_id);
        todo!()
    }

    pub fn add_category_amount(&mut self, amount: CategoryAmount) -> Result<(), DomainError> {
        let _ = amount;
        todo!()
    }

    #[must_use]
    pub fn category_amounts(&self) -> &[CategoryAmount] {
        todo!()
    }

    /// Свои суммы этого вида плюс суммы всех подкатегорий.
    pub fn category_total(&self, id: CategoryId, kind: EntryKind) -> Result<i64, DomainError> {
        let _ = (id, kind);
        todo!()
    }

    /// Категории, которые можно выбрать при вводе операции этого вида.
    #[must_use]
    pub fn categories_for(&self, kind: EntryKind) -> Vec<&Category> {
        let _ = kind;
        todo!()
    }

    pub fn add_operation(&mut self, draft: OperationDraft) -> Result<(), DomainError> {
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        self.operations.push(draft);
        Ok(())
    }

    fn find_category(&self, id: CategoryId) -> Option<&Category> {
        self.categories.iter().find(|category| category.id() == id)
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
