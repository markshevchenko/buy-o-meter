use std::time::SystemTime;

use crate::account::AccountId;
use crate::category::CategoryId;
use crate::clock::Clock;

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
    date: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: String,
}

impl Expense {
    pub(crate) fn new(
        date: SystemTime,
        account_id: AccountId,
        category_id: CategoryId,
        amount: i64,
        comment: String,
    ) -> Self {
        Self {
            date,
            account_id,
            category_id,
            amount,
            comment,
        }
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
    date: SystemTime,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: String,
}

impl Income {
    pub(crate) fn new(
        date: SystemTime,
        account_id: AccountId,
        category_id: CategoryId,
        amount: i64,
        comment: String,
    ) -> Self {
        Self {
            date,
            account_id,
            category_id,
            amount,
            comment,
        }
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
    date: SystemTime,
    from_account_id: AccountId,
    to_account_id: AccountId,
    amount: i64,
    to_amount: i64,
    comment: String,
}

impl Transfer {
    pub(crate) fn new(
        date: SystemTime,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
        to_amount: i64,
        comment: String,
    ) -> Self {
        Self {
            date,
            from_account_id,
            to_account_id,
            amount,
            to_amount,
            comment,
        }
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

#[cfg(test)]
mod tests {
    use crate::clock::FixedClock;
    use crate::date;
    use crate::error::DomainError;

    use super::{NewExpense, NewIncome, NewTransfer};

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
}
