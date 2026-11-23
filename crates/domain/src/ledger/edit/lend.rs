use super::*;

impl Ledger {
    pub(super) fn lend_form(&self, id: OperationId) -> Result<OperationForm, DomainError> {
        let lend = find_by_id(&self.lends, id, Lend::id)?;
        Ok(OperationForm::Lend(NewLend {
            date: lend.date(),
            account_id: Some(lend.account_id()),
            counterparty: Some(lend.counterparty_name().to_owned()),
            amount: Some(lend.amount()),
        }))
    }

    pub(super) fn prepare_lend(&self, form: NewLend) -> Result<Drafted, DomainError> {
        let (account_id, name, amount) =
            super::super::debt::complete_debt(form.account_id, form.counterparty, form.amount)?;
        self.ensure_account(account_id)?;
        Ok(Drafted::Lend {
            date: form.date,
            account_id,
            name,
            amount,
        })
    }

    pub(super) fn lend_effect(&self, id: OperationId) -> Result<DebtEffect, DomainError> {
        let lend = find_by_id(&self.lends, id, Lend::id)?;
        debt_principal(
            self,
            lend.account_id(),
            lend.counterparty_name(),
            lend.amount(),
            RepaymentDirection::Received,
        )
    }
}
