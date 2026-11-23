use std::time::SystemTime;

use crate::account::AccountId;
use crate::category::CategoryId;
use crate::clock::Clock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperationId(u64);

impl OperationId {
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Expense,
    Income,
    Transfer,
    Lend,
    Borrow,
    Repayment,
}

/// Операция в ленте. Номер внутри дня — место среди записей этой даты, с единицы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationEntry {
    id: OperationId,
    date: SystemTime,
    kind: OperationKind,
}

impl OperationEntry {
    pub(crate) fn new(id: OperationId, date: SystemTime, kind: OperationKind) -> Self {
        Self { id, date, kind }
    }

    #[must_use]
    pub fn id(&self) -> OperationId {
        self.id
    }

    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn kind(&self) -> OperationKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepaymentDirection {
    Received,
    Given,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationDraft {
    Expense {
        account_id: AccountId,
        amount: i64,
    },
    Income {
        account_id: AccountId,
        amount: i64,
    },
    Transfer {
        from_account_id: AccountId,
        from_amount: i64,
        to_account_id: AccountId,
        to_amount: i64,
    },
    Lend {
        account_id: AccountId,
        amount: i64,
    },
    Borrow {
        account_id: AccountId,
        amount: i64,
    },
    Repayment {
        account_id: AccountId,
        amount: i64,
        direction: RepaymentDirection,
    },
}

/// Название категории в запись не копируется.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    Expense {
        category_id: CategoryId,
        amount: i64,
    },
    Income {
        category_id: CategoryId,
        amount: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExpense {
    pub date: SystemTime,
    pub account_id: Option<AccountId>,
    pub category_id: Option<CategoryId>,
    pub amount: Option<i64>,
    pub comment: String,
}

impl NewExpense {
    #[must_use]
    pub fn open(clock: &impl Clock) -> Self {
        Self {
            date: clock.today(),
            account_id: None,
            category_id: None,
            amount: None,
            comment: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expense {
    id: OperationId,
    date: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: String,
}

impl Expense {
    pub(crate) fn new(
        id: OperationId,
        date: SystemTime,
        account_id: AccountId,
        category_id: CategoryId,
        amount: i64,
        comment: String,
    ) -> Self {
        Self {
            id,
            date,
            account_id,
            category_id,
            amount,
            comment,
        }
    }

    pub(crate) fn id(&self) -> OperationId {
        self.id
    }

    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    #[must_use]
    pub fn category_id(&self) -> CategoryId {
        self.category_id
    }

    pub(crate) fn set_category_id(&mut self, category_id: CategoryId) {
        self.category_id = category_id;
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }

    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewIncome {
    pub date: SystemTime,
    pub account_id: Option<AccountId>,
    pub category_id: Option<CategoryId>,
    pub amount: Option<i64>,
    pub comment: String,
}

impl NewIncome {
    #[must_use]
    pub fn open(clock: &impl Clock) -> Self {
        Self {
            date: clock.today(),
            account_id: None,
            category_id: None,
            amount: None,
            comment: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Income {
    id: OperationId,
    date: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: String,
}

impl Income {
    pub(crate) fn new(
        id: OperationId,
        date: SystemTime,
        account_id: AccountId,
        category_id: CategoryId,
        amount: i64,
        comment: String,
    ) -> Self {
        Self {
            id,
            date,
            account_id,
            category_id,
            amount,
            comment,
        }
    }

    pub(crate) fn id(&self) -> OperationId {
        self.id
    }

    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    #[must_use]
    pub fn category_id(&self) -> CategoryId {
        self.category_id
    }

    pub(crate) fn set_category_id(&mut self, category_id: CategoryId) {
        self.category_id = category_id;
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }

    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferAmountFields {
    One,
    Two,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTransfer {
    pub date: SystemTime,
    pub from_account_id: Option<AccountId>,
    pub to_account_id: Option<AccountId>,
    pub amount: Option<i64>,
    pub to_amount: Option<i64>,
    pub comment: String,
}

impl NewTransfer {
    #[must_use]
    pub fn open(clock: &impl Clock) -> Self {
        Self {
            date: clock.today(),
            from_account_id: None,
            to_account_id: None,
            amount: None,
            to_amount: None,
            comment: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    id: OperationId,
    date: SystemTime,
    from_account_id: AccountId,
    to_account_id: AccountId,
    amount: i64,
    to_amount: i64,
    comment: String,
}

impl Transfer {
    pub(crate) fn new(
        id: OperationId,
        date: SystemTime,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
        to_amount: i64,
        comment: String,
    ) -> Self {
        Self {
            id,
            date,
            from_account_id,
            to_account_id,
            amount,
            to_amount,
            comment,
        }
    }

    pub(crate) fn id(&self) -> OperationId {
        self.id
    }

    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn from_account_id(&self) -> AccountId {
        self.from_account_id
    }

    #[must_use]
    pub fn to_account_id(&self) -> AccountId {
        self.to_account_id
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }

    #[must_use]
    pub fn to_amount(&self) -> i64 {
        self.to_amount
    }

    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLend {
    pub date: SystemTime,
    pub account_id: Option<AccountId>,
    pub counterparty: Option<String>,
    pub amount: Option<i64>,
}

impl NewLend {
    #[must_use]
    pub fn open(clock: &impl Clock) -> Self {
        Self {
            date: clock.today(),
            account_id: None,
            counterparty: None,
            amount: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lend {
    id: OperationId,
    date: SystemTime,
    account_id: AccountId,
    counterparty_name: String,
    amount: i64,
}

impl Lend {
    pub(crate) fn new(
        id: OperationId,
        date: SystemTime,
        account_id: AccountId,
        counterparty_name: String,
        amount: i64,
    ) -> Self {
        Self {
            id,
            date,
            account_id,
            counterparty_name,
            amount,
        }
    }

    pub(crate) fn id(&self) -> OperationId {
        self.id
    }

    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    #[must_use]
    pub fn counterparty_name(&self) -> &str {
        &self.counterparty_name
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBorrow {
    pub date: SystemTime,
    pub account_id: Option<AccountId>,
    pub counterparty: Option<String>,
    pub amount: Option<i64>,
}

impl NewBorrow {
    #[must_use]
    pub fn open(clock: &impl Clock) -> Self {
        Self {
            date: clock.today(),
            account_id: None,
            counterparty: None,
            amount: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Borrow {
    id: OperationId,
    date: SystemTime,
    account_id: AccountId,
    counterparty_name: String,
    amount: i64,
}

impl Borrow {
    pub(crate) fn new(
        id: OperationId,
        date: SystemTime,
        account_id: AccountId,
        counterparty_name: String,
        amount: i64,
    ) -> Self {
        Self {
            id,
            date,
            account_id,
            counterparty_name,
            amount,
        }
    }

    pub(crate) fn id(&self) -> OperationId {
        self.id
    }

    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    #[must_use]
    pub fn counterparty_name(&self) -> &str {
        &self.counterparty_name
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRepayment {
    pub date: SystemTime,
    pub account_id: Option<AccountId>,
    pub counterparty: Option<String>,
    pub direction: Option<RepaymentDirection>,
    pub amount: Option<i64>,
}

impl NewRepayment {
    #[must_use]
    pub fn open(clock: &impl Clock) -> Self {
        Self {
            date: clock.today(),
            account_id: None,
            counterparty: None,
            direction: None,
            amount: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repayment {
    id: OperationId,
    date: SystemTime,
    account_id: AccountId,
    counterparty_name: String,
    direction: RepaymentDirection,
    amount: i64,
}

impl Repayment {
    pub(crate) fn new(
        id: OperationId,
        date: SystemTime,
        account_id: AccountId,
        counterparty_name: String,
        direction: RepaymentDirection,
        amount: i64,
    ) -> Self {
        Self {
            id,
            date,
            account_id,
            counterparty_name,
            direction,
            amount,
        }
    }

    pub(crate) fn id(&self) -> OperationId {
        self.id
    }

    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    #[must_use]
    pub fn counterparty_name(&self) -> &str {
        &self.counterparty_name
    }

    #[must_use]
    pub fn direction(&self) -> RepaymentDirection {
        self.direction
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

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

    /// Совпадающие поля сохраняют значения, остальные у нового типа пустые.
    /// Дата и сумма есть у всех. Счёт тоже: у перевода это счёт списания,
    /// счёт зачисления и вторая сумма при переходе на перевод пустые.
    /// Категория сохраняется между расходом и доходом.
    /// Комментарий сохраняется между расходом, доходом и переводом.
    /// Контрагент сохраняется между выдачей, займом и возвратом.
    /// Направление есть только у возврата и при переходе на него пустое.
    /// Тот же тип возвращает форму без изменений.
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

#[cfg(test)]
mod tests {
    use crate::clock::FixedClock;
    use crate::date;
    use crate::error::DomainError;

    use super::{NewBorrow, NewExpense, NewIncome, NewLend, NewRepayment, NewTransfer};

    #[test]
    fn open_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
        let today = date(2026, 11, 22)?;
        let form = NewExpense::open(&FixedClock::new(today));

        assert_eq!(form.date, today);
        assert_eq!(form.account_id, None);
        assert_eq!(form.category_id, None);
        assert_eq!(form.amount, None);
        assert_eq!(form.comment, "");
        Ok(())
    }

    #[test]
    fn open_income_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
        let today = date(2026, 11, 22)?;
        let form = NewIncome::open(&FixedClock::new(today));

        assert_eq!(form.date, today);
        assert_eq!(form.account_id, None);
        assert_eq!(form.category_id, None);
        assert_eq!(form.amount, None);
        assert_eq!(form.comment, "");
        Ok(())
    }

    #[test]
    fn open_transfer_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
        let today = date(2026, 11, 22)?;
        let form = NewTransfer::open(&FixedClock::new(today));

        assert_eq!(form.date, today);
        assert_eq!(form.from_account_id, None);
        assert_eq!(form.to_account_id, None);
        assert_eq!(form.amount, None);
        assert_eq!(form.to_amount, None);
        assert_eq!(form.comment, "");
        Ok(())
    }

    #[test]
    fn open_lend_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
        let today = date(2026, 11, 22)?;
        let form = NewLend::open(&FixedClock::new(today));

        assert_eq!(form.date, today);
        assert_eq!(form.account_id, None);
        assert_eq!(form.counterparty, None);
        assert_eq!(form.amount, None);
        Ok(())
    }

    #[test]
    fn open_borrow_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
        let today = date(2026, 11, 22)?;
        let form = NewBorrow::open(&FixedClock::new(today));

        assert_eq!(form.date, today);
        assert_eq!(form.account_id, None);
        assert_eq!(form.counterparty, None);
        assert_eq!(form.amount, None);
        Ok(())
    }

    #[test]
    fn open_repayment_sets_today_and_leaves_fields_empty() -> Result<(), DomainError> {
        let today = date(2026, 11, 22)?;
        let form = NewRepayment::open(&FixedClock::new(today));

        assert_eq!(form.date, today);
        assert_eq!(form.account_id, None);
        assert_eq!(form.counterparty, None);
        assert_eq!(form.direction, None);
        assert_eq!(form.amount, None);
        Ok(())
    }
}
