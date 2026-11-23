use super::{Ledger, place_on_date};
use crate::account::AccountId;
use crate::category::CategoryId;
use crate::error::{DomainError, MissingFields, MissingTransferFields};
use crate::operation::{
    Expense, Income, NewExpense, NewIncome, NewTransfer, OperationDraft, OperationKind, Transfer,
    TransferAmountFields,
};

impl Ledger {
    pub fn add_operation(&mut self, draft: OperationDraft) -> Result<(), DomainError> {
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        self.track_draft(draft);
        Ok(())
    }

    pub fn add_expense(&mut self, input: NewExpense) -> Result<(), DomainError> {
        let (account_id, category_id, amount) = complete_expense(&input)?;
        self.ensure_account_and_category(account_id, category_id)?;
        let id = self.allocate_operation_id();
        let date = input.date;
        self.place_expense(Expense::new(
            id,
            date,
            account_id,
            category_id,
            amount,
            input.comment,
        ));
        #[rustfmt::skip]
            self.track_operation(id, date, OperationKind::Expense, OperationDraft::Expense {
                account_id,
                amount,
            });
        Ok(())
    }

    pub(super) fn ensure_account_and_category(
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

    #[must_use]
    pub fn expenses(&self) -> &[Expense] {
        &self.expenses
    }

    pub fn add_income(&mut self, input: NewIncome) -> Result<(), DomainError> {
        let (account_id, category_id, amount) = complete_income(&input)?;
        self.ensure_account_and_category(account_id, category_id)?;
        let id = self.allocate_operation_id();
        let date = input.date;
        self.place_income(Income::new(
            id,
            date,
            account_id,
            category_id,
            amount,
            input.comment,
        ));
        #[rustfmt::skip]
            self.track_operation(id, date, OperationKind::Income, OperationDraft::Income {
                account_id,
                amount,
            });
        Ok(())
    }

    #[must_use]
    pub fn incomes(&self) -> &[Income] {
        &self.incomes
    }

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
        let id = self.allocate_operation_id();
        let date = input.date;
        self.place_transfer(Transfer::new(
            id,
            date,
            from_account_id,
            to_account_id,
            amount,
            to_amount,
            input.comment,
        ));
        #[rustfmt::skip]
            self.track_operation(id, date, OperationKind::Transfer, OperationDraft::Transfer {
                from_account_id,
                from_amount: amount,
                to_account_id,
                to_amount,
            });
        Ok(())
    }

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

    pub(super) fn credited_amount(
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

    pub(super) fn place_expense(&mut self, expense: Expense) {
        place_on_date(&mut self.expenses, expense, Expense::date);
    }

    pub(super) fn place_income(&mut self, income: Income) {
        place_on_date(&mut self.incomes, income, Income::date);
    }

    pub(super) fn place_transfer(&mut self, transfer: Transfer) {
        place_on_date(&mut self.transfers, transfer, Transfer::date);
    }
}

pub(super) fn complete_transfer(
    input: &NewTransfer,
) -> Result<(AccountId, AccountId, i64), DomainError> {
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

pub(super) fn complete_income(
    input: &NewIncome,
) -> Result<(AccountId, CategoryId, i64), DomainError> {
    complete_fields(input.account_id, input.category_id, input.amount)
}

pub(super) fn complete_expense(
    input: &NewExpense,
) -> Result<(AccountId, CategoryId, i64), DomainError> {
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
