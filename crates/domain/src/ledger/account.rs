use super::Ledger;
use crate::account::{Account, AccountId, CreateAccount, UpdateAccount};
use crate::currency::Currency;
use crate::error::DomainError;
use crate::operation::{OperationDraft, RepaymentDirection};
use crate::quick_access::AccountBalance;

impl Ledger {
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
        let opening = account.opening_balance();
        Ok(opening_plus_effects(id, opening, &self.operations))
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

    pub(super) fn ensure_account(&self, account_id: AccountId) -> Result<(), DomainError> {
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        if self.find_account(account_id).is_none() {
            return Err(DomainError::AccountNotFound);
        }
        Ok(())
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

pub(super) fn account_lines(
    accounts: &[Account],
    operations: &[OperationDraft],
) -> Vec<AccountBalance> {
    accounts
        .iter()
        .map(|account| {
            AccountBalance::new(
                account.id(),
                account.name().to_owned(),
                account.currency().clone(),
                opening_plus_effects(account.id(), account.opening_balance(), operations),
            )
        })
        .collect()
}

fn opening_plus_effects(account_id: AccountId, opening: i64, operations: &[OperationDraft]) -> i64 {
    let mut total = opening;
    for operation in operations {
        total += balance_effect(account_id, operation);
    }
    total
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
