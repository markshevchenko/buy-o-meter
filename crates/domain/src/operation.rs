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

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }

    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use crate::clock::FixedClock;

    use super::NewExpense;

    fn midnight(days_after_epoch: u32) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(24 * 60 * 60) * days_after_epoch
    }

    #[test]
    fn open_sets_today_and_leaves_fields_empty() {
        let today = midnight(4);
        let form = NewExpense::open(&FixedClock::new(today));

        assert_eq!(form.date, today);
        assert_eq!(form.account_id, None);
        assert_eq!(form.category_id, None);
        assert_eq!(form.amount, None);
        assert_eq!(form.comment, "");
    }
}
