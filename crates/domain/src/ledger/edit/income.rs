use super::*;

impl Ledger {
    pub(super) fn income_form(&self, id: OperationId) -> Result<OperationForm, DomainError> {
        let income = find_by_id(&self.incomes, id, Income::id)?;
        Ok(OperationForm::Income(NewIncome {
            date: income.date(),
            account_id: Some(income.account_id()),
            category_id: Some(income.category_id()),
            amount: Some(income.amount()),
            comment: income.comment().to_owned(),
        }))
    }

    pub(super) fn prepare_income(&self, form: NewIncome) -> Result<Drafted, DomainError> {
        let (account_id, category_id, amount) = super::super::operation::complete_income(&form)?;
        self.ensure_account_and_category(account_id, category_id)?;
        Ok(Drafted::Income {
            date: form.date,
            account_id,
            category_id,
            amount,
            comment: form.comment,
        })
    }
}

pub(super) fn saved_income(
    id: OperationId,
    date: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: String,
) -> Saved {
    Saved::Income(Income::new(
        id,
        date,
        account_id,
        category_id,
        amount,
        comment,
    ))
}
