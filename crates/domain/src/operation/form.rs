use std::time::SystemTime;

use super::OperationKind;
use super::borrow::NewBorrow;
use super::expense::NewExpense;
use super::income::NewIncome;
use super::lend::NewLend;
use super::repayment::NewRepayment;
use super::transfer::NewTransfer;
use crate::account::AccountId;
use crate::category::CategoryId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationForm {
    Expense(NewExpense),
    Income(NewIncome),
    Transfer(NewTransfer),
    Lend(NewLend),
    Borrow(NewBorrow),
    Repayment(NewRepayment),
}

impl OperationForm {
    #[must_use]
    pub fn kind(&self) -> OperationKind {
        match self {
            Self::Expense(_) => OperationKind::Expense,
            Self::Income(_) => OperationKind::Income,
            Self::Transfer(_) => OperationKind::Transfer,
            Self::Lend(_) => OperationKind::Lend,
            Self::Borrow(_) => OperationKind::Borrow,
            Self::Repayment(_) => OperationKind::Repayment,
        }
    }

    #[must_use]
    pub fn switch_kind(self, kind: OperationKind) -> Self {
        if self.kind() == kind {
            return self;
        }
        form_with_shared_fields(shared_fields(self), kind)
    }
}

struct SharedFields {
    date: SystemTime,
    account_id: Option<AccountId>,
    amount: Option<i64>,
    comment: Option<String>,
    category_id: Option<CategoryId>,
    counterparty: Option<String>,
}

fn shared_fields(form: OperationForm) -> SharedFields {
    match form {
        OperationForm::Expense(form) => money_fields(
            form.date,
            form.account_id,
            form.amount,
            form.comment,
            form.category_id,
        ),
        OperationForm::Income(form) => money_fields(
            form.date,
            form.account_id,
            form.amount,
            form.comment,
            form.category_id,
        ),
        OperationForm::Transfer(form) => money_fields(
            form.date,
            form.from_account_id,
            form.amount,
            form.comment,
            None,
        ),
        OperationForm::Lend(form) => {
            debt_fields(form.date, form.account_id, form.amount, form.counterparty)
        }
        OperationForm::Borrow(form) => {
            debt_fields(form.date, form.account_id, form.amount, form.counterparty)
        }
        OperationForm::Repayment(form) => {
            debt_fields(form.date, form.account_id, form.amount, form.counterparty)
        }
    }
}

fn money_fields(
    date: SystemTime,
    account_id: Option<AccountId>,
    amount: Option<i64>,
    comment: String,
    category_id: Option<CategoryId>,
) -> SharedFields {
    SharedFields {
        date,
        account_id,
        amount,
        comment: Some(comment),
        category_id,
        counterparty: None,
    }
}

fn debt_fields(
    date: SystemTime,
    account_id: Option<AccountId>,
    amount: Option<i64>,
    counterparty: Option<String>,
) -> SharedFields {
    SharedFields {
        date,
        account_id,
        amount,
        comment: None,
        category_id: None,
        counterparty,
    }
}

fn form_with_shared_fields(fields: SharedFields, kind: OperationKind) -> OperationForm {
    let comment = fields.comment.unwrap_or_default();
    match kind {
        OperationKind::Expense => OperationForm::Expense(NewExpense {
            date: fields.date,
            account_id: fields.account_id,
            category_id: fields.category_id,
            amount: fields.amount,
            comment,
        }),
        OperationKind::Income => OperationForm::Income(NewIncome {
            date: fields.date,
            account_id: fields.account_id,
            category_id: fields.category_id,
            amount: fields.amount,
            comment,
        }),
        OperationKind::Transfer => OperationForm::Transfer(NewTransfer {
            date: fields.date,
            from_account_id: fields.account_id,
            to_account_id: None,
            amount: fields.amount,
            to_amount: None,
            comment,
        }),
        OperationKind::Lend => OperationForm::Lend(NewLend {
            date: fields.date,
            account_id: fields.account_id,
            counterparty: fields.counterparty,
            amount: fields.amount,
        }),
        OperationKind::Borrow => OperationForm::Borrow(NewBorrow {
            date: fields.date,
            account_id: fields.account_id,
            counterparty: fields.counterparty,
            amount: fields.amount,
        }),
        OperationKind::Repayment => OperationForm::Repayment(NewRepayment {
            date: fields.date,
            account_id: fields.account_id,
            counterparty: fields.counterparty,
            direction: None,
            amount: fields.amount,
        }),
    }
}
