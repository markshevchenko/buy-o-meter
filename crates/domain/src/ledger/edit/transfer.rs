use super::*;

impl Ledger {
    pub(super) fn transfer_form(&self, id: OperationId) -> Result<OperationForm, DomainError> {
        let transfer = find_by_id(&self.transfers, id, Transfer::id)?;
        Ok(OperationForm::Transfer(NewTransfer {
            date: transfer.date(),
            from_account_id: Some(transfer.from_account_id()),
            to_account_id: Some(transfer.to_account_id()),
            amount: Some(transfer.amount()),
            to_amount: Some(transfer.to_amount()),
            comment: transfer.comment().to_owned(),
        }))
    }

    pub(super) fn prepare_transfer(&self, form: NewTransfer) -> Result<Drafted, DomainError> {
        let (from_account_id, to_account_id, amount) =
            super::super::operation::complete_transfer(&form)?;
        if self.accounts.is_empty() {
            return Err(DomainError::AccountRequired);
        }
        if from_account_id == to_account_id {
            return Err(DomainError::SameAccount);
        }
        let to_amount =
            self.credited_amount(from_account_id, to_account_id, amount, form.to_amount)?;
        Ok(Drafted::Transfer {
            date: form.date,
            from_account_id,
            to_account_id,
            amount,
            to_amount,
            comment: form.comment,
        })
    }
}

pub(super) fn saved_transfer(
    id: OperationId,
    date: SystemTime,
    from_account_id: AccountId,
    to_account_id: AccountId,
    amount: i64,
    to_amount: i64,
    comment: String,
) -> Saved {
    Saved::Transfer(Transfer::new(
        id,
        date,
        from_account_id,
        to_account_id,
        amount,
        to_amount,
        comment,
    ))
}
