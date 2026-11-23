use std::time::SystemTime;

use domain::{
    AccountId, CategoryId, DomainError, Ledger, NewBorrow, NewExpense, NewIncome, NewLend,
    NewRepayment, NewTransfer, OperationForm, OperationId, OperationKind, RepaymentDirection,
    TransferAmountFields,
};

use crate::text::{input_date, today_input};

#[derive(Clone)]
pub(crate) struct Draft {
    pub(crate) date: SystemTime,
    pub(crate) account_id: Option<AccountId>,
    pub(crate) category_id: Option<CategoryId>,
    pub(crate) amount: Option<i64>,
    pub(crate) to_account_id: Option<AccountId>,
    pub(crate) to_amount: Option<i64>,
    pub(crate) comment: String,
    pub(crate) counterparty: Option<String>,
    pub(crate) direction: Option<RepaymentDirection>,
}

pub(crate) struct Loaded {
    pub(crate) kind: OperationKind,
    pub(crate) date: String,
    pub(crate) account: String,
    pub(crate) category: String,
    pub(crate) amount: String,
    pub(crate) to_account: String,
    pub(crate) to_amount: String,
    pub(crate) comment: String,
    pub(crate) counterparty: String,
    pub(crate) direction: String,
}

pub(crate) fn load_editor(
    ledger: &Ledger,
    editing: Option<OperationId>,
    kind: OperationKind,
) -> Loaded {
    let Some(id) = editing else {
        return Loaded::blank(kind);
    };
    ledger
        .operation_form(id)
        .map(Loaded::from_form)
        .unwrap_or_else(|_| Loaded::blank(kind))
}

pub(crate) fn apply(
    ledger: &mut Ledger,
    editing: Option<OperationId>,
    kind: OperationKind,
    draft: Draft,
) -> Result<(), DomainError> {
    let draft = adjusted(ledger, draft);
    match editing {
        Some(id) => ledger.update_operation(id, form_from(kind, &draft)),
        None => create_new(ledger, kind, &draft),
    }
}

pub(crate) fn needs_credited_amount(ledger: &Ledger, from: &str, to: &str) -> bool {
    let (Some(from_id), Some(to_id)) = (parse_account(from), parse_account(to)) else {
        return false;
    };
    matches!(
        ledger.transfer_amount_fields(from_id, to_id),
        Ok(TransferAmountFields::Two)
    )
}

pub(crate) fn parse_account(text: &str) -> Option<AccountId> {
    parse_u64(text).map(AccountId::from_u64)
}

pub(crate) fn parse_category(text: &str) -> Option<CategoryId> {
    parse_u64(text).map(CategoryId::from_u64)
}

pub(crate) fn parse_amount(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    text.parse().ok()
}

pub(crate) fn parse_name(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

pub(crate) fn parse_direction(text: &str) -> Option<RepaymentDirection> {
    match text {
        "received" => Some(RepaymentDirection::Received),
        "given" => Some(RepaymentDirection::Given),
        _ => None,
    }
}

impl Loaded {
    fn blank(kind: OperationKind) -> Self {
        Self {
            kind,
            date: today_input(),
            account: String::new(),
            category: String::new(),
            amount: String::new(),
            to_account: String::new(),
            to_amount: String::new(),
            comment: String::new(),
            counterparty: String::new(),
            direction: String::new(),
        }
    }

    fn from_form(form: OperationForm) -> Self {
        match form {
            OperationForm::Expense(form) => money_loaded(
                OperationKind::Expense,
                form.date,
                form.account_id,
                form.category_id,
                form.amount,
                form.comment,
            ),
            OperationForm::Income(form) => money_loaded(
                OperationKind::Income,
                form.date,
                form.account_id,
                form.category_id,
                form.amount,
                form.comment,
            ),
            OperationForm::Transfer(form) => transfer_loaded(form),
            OperationForm::Lend(form) => debt_loaded(
                OperationKind::Lend,
                form.date,
                form.account_id,
                form.counterparty,
                form.amount,
                None,
            ),
            OperationForm::Borrow(form) => debt_loaded(
                OperationKind::Borrow,
                form.date,
                form.account_id,
                form.counterparty,
                form.amount,
                None,
            ),
            OperationForm::Repayment(form) => debt_loaded(
                OperationKind::Repayment,
                form.date,
                form.account_id,
                form.counterparty,
                form.amount,
                form.direction,
            ),
        }
    }
}

fn money_loaded(
    kind: OperationKind,
    date: SystemTime,
    account_id: Option<AccountId>,
    category_id: Option<CategoryId>,
    amount: Option<i64>,
    comment: String,
) -> Loaded {
    Loaded {
        kind,
        date: input_date(date),
        account: account_text(account_id),
        category: category_text(category_id),
        amount: amount_text(amount),
        to_account: String::new(),
        to_amount: String::new(),
        comment,
        counterparty: String::new(),
        direction: String::new(),
    }
}

fn transfer_loaded(form: NewTransfer) -> Loaded {
    Loaded {
        kind: OperationKind::Transfer,
        date: input_date(form.date),
        account: account_text(form.from_account_id),
        category: String::new(),
        amount: amount_text(form.amount),
        to_account: account_text(form.to_account_id),
        to_amount: amount_text(form.to_amount),
        comment: form.comment,
        counterparty: String::new(),
        direction: String::new(),
    }
}

fn debt_loaded(
    kind: OperationKind,
    date: SystemTime,
    account_id: Option<AccountId>,
    counterparty: Option<String>,
    amount: Option<i64>,
    direction: Option<RepaymentDirection>,
) -> Loaded {
    Loaded {
        kind,
        date: input_date(date),
        account: account_text(account_id),
        category: String::new(),
        amount: amount_text(amount),
        to_account: String::new(),
        to_amount: String::new(),
        comment: String::new(),
        counterparty: counterparty.unwrap_or_default(),
        direction: direction_text(direction),
    }
}

fn adjusted(ledger: &Ledger, mut draft: Draft) -> Draft {
    draft.to_amount = credited(ledger, &draft);
    draft
}

fn credited(ledger: &Ledger, draft: &Draft) -> Option<i64> {
    let (Some(from_id), Some(to_id)) = (draft.account_id, draft.to_account_id) else {
        return draft.to_amount;
    };
    if matches!(
        ledger.transfer_amount_fields(from_id, to_id),
        Ok(TransferAmountFields::One)
    ) {
        return None;
    }
    draft.to_amount
}

fn create_new(ledger: &mut Ledger, kind: OperationKind, draft: &Draft) -> Result<(), DomainError> {
    match kind {
        OperationKind::Expense => ledger.add_expense(expense_from(draft)),
        OperationKind::Income => ledger.add_income(income_from(draft)),
        OperationKind::Transfer => ledger.add_transfer(transfer_from(draft)),
        OperationKind::Lend => ledger.add_lend(lend_from(draft)),
        OperationKind::Borrow => ledger.add_borrow(borrow_from(draft)),
        OperationKind::Repayment => ledger.add_repayment(repayment_from(draft)),
    }
}

fn form_from(kind: OperationKind, draft: &Draft) -> OperationForm {
    match kind {
        OperationKind::Expense => OperationForm::Expense(expense_from(draft)),
        OperationKind::Income => OperationForm::Income(income_from(draft)),
        OperationKind::Transfer => OperationForm::Transfer(transfer_from(draft)),
        OperationKind::Lend => OperationForm::Lend(lend_from(draft)),
        OperationKind::Borrow => OperationForm::Borrow(borrow_from(draft)),
        OperationKind::Repayment => OperationForm::Repayment(repayment_from(draft)),
    }
}

fn expense_from(draft: &Draft) -> NewExpense {
    NewExpense {
        date: draft.date,
        account_id: draft.account_id,
        category_id: draft.category_id,
        amount: draft.amount,
        comment: draft.comment.clone(),
    }
}

fn income_from(draft: &Draft) -> NewIncome {
    NewIncome {
        date: draft.date,
        account_id: draft.account_id,
        category_id: draft.category_id,
        amount: draft.amount,
        comment: draft.comment.clone(),
    }
}

fn transfer_from(draft: &Draft) -> NewTransfer {
    NewTransfer {
        date: draft.date,
        from_account_id: draft.account_id,
        to_account_id: draft.to_account_id,
        amount: draft.amount,
        to_amount: draft.to_amount,
        comment: draft.comment.clone(),
    }
}

fn lend_from(draft: &Draft) -> NewLend {
    NewLend {
        date: draft.date,
        account_id: draft.account_id,
        counterparty: draft.counterparty.clone(),
        amount: draft.amount,
    }
}

fn borrow_from(draft: &Draft) -> NewBorrow {
    NewBorrow {
        date: draft.date,
        account_id: draft.account_id,
        counterparty: draft.counterparty.clone(),
        amount: draft.amount,
    }
}

fn repayment_from(draft: &Draft) -> NewRepayment {
    NewRepayment {
        date: draft.date,
        account_id: draft.account_id,
        counterparty: draft.counterparty.clone(),
        direction: draft.direction,
        amount: draft.amount,
    }
}

fn account_text(id: Option<AccountId>) -> String {
    id.map(|id| id.as_u64().to_string()).unwrap_or_default()
}

fn category_text(id: Option<CategoryId>) -> String {
    id.map(|id| id.as_u64().to_string()).unwrap_or_default()
}

fn amount_text(amount: Option<i64>) -> String {
    amount.map(|amount| amount.to_string()).unwrap_or_default()
}

fn direction_text(direction: Option<RepaymentDirection>) -> String {
    match direction {
        Some(RepaymentDirection::Received) => "received".to_owned(),
        Some(RepaymentDirection::Given) => "given".to_owned(),
        None => String::new(),
    }
}

fn parse_u64(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        text.parse().ok()
    }
}
