use super::*;

impl Ledger {
    pub(super) fn repayment_form(&self, id: OperationId) -> Result<OperationForm, DomainError> {
        let repayment = find_by_id(&self.repayments, id, Repayment::id)?;
        Ok(OperationForm::Repayment(NewRepayment {
            date: repayment.date(),
            account_id: Some(repayment.account_id()),
            counterparty: Some(repayment.counterparty_name().to_owned()),
            direction: Some(repayment.direction()),
            amount: Some(repayment.amount()),
        }))
    }

    pub(super) fn prepare_repayment(&self, form: NewRepayment) -> Result<Drafted, DomainError> {
        let (account_id, name, direction, amount) = super::super::debt::complete_repayment(
            form.account_id,
            form.counterparty,
            form.direction,
            form.amount,
        )?;
        self.ensure_account(account_id)?;
        Ok(Drafted::Repayment {
            date: form.date,
            account_id,
            name,
            direction,
            amount,
        })
    }

    pub(super) fn saved_repayment_effect(
        &self,
        id: OperationId,
    ) -> Result<DebtEffect, DomainError> {
        let repayment = find_by_id(&self.repayments, id, Repayment::id)?;
        let mut effect = debt_principal(
            self,
            repayment.account_id(),
            repayment.counterparty_name(),
            repayment.amount(),
            repayment.direction(),
        )?;
        effect.amount = -effect.amount;
        Ok(effect)
    }
}
