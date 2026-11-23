use super::*;

impl Ledger {
    pub(super) fn expense_form(&self, id: OperationId) -> Result<OperationForm, DomainError> {
        let expense = find_by_id(&self.expenses, id, Expense::id)?;
        Ok(OperationForm::Expense(NewExpense {
            date: expense.date(),
            account_id: Some(expense.account_id()),
            category_id: Some(expense.category_id()),
            amount: Some(expense.amount()),
            comment: expense.comment().to_owned(),
        }))
    }

    pub(super) fn prepare_expense(&self, form: NewExpense) -> Result<Drafted, DomainError> {
        let (account_id, category_id, amount) = super::super::operation::complete_expense(&form)?;
        self.ensure_account_and_category(account_id, category_id)?;
        Ok(Drafted::Expense {
            date: form.date,
            account_id,
            category_id,
            amount,
            comment: form.comment,
        })
    }
}

pub(super) fn saved_expense(
    id: OperationId,
    date: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: String,
) -> Saved {
    Saved::Expense(Expense::new(
        id,
        date,
        account_id,
        category_id,
        amount,
        comment,
    ))
}
