use super::*;

impl Ledger {
    pub(super) fn borrow_form(&self, id: OperationId) -> Result<OperationForm, DomainError> {
        let borrow = find_by_id(&self.borrows, id, Borrow::id)?;
        Ok(OperationForm::Borrow(NewBorrow {
            date: borrow.date(),
            account_id: Some(borrow.account_id()),
            counterparty: Some(borrow.counterparty_name().to_owned()),
            amount: Some(borrow.amount()),
        }))
    }

    pub(super) fn prepare_borrow(&self, form: NewBorrow) -> Result<Drafted, DomainError> {
        let (account_id, name, amount) =
            super::super::debt::complete_debt(form.account_id, form.counterparty, form.amount)?;
        self.ensure_account(account_id)?;
        Ok(Drafted::Borrow {
            date: form.date,
            account_id,
            name,
            amount,
        })
    }

    pub(super) fn borrow_effect(&self, id: OperationId) -> Result<DebtEffect, DomainError> {
        let borrow = find_by_id(&self.borrows, id, Borrow::id)?;
        debt_principal(
            self,
            borrow.account_id(),
            borrow.counterparty_name(),
            borrow.amount(),
            RepaymentDirection::Given,
        )
    }
}
