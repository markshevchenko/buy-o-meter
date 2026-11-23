use std::time::SystemTime;

use super::OperationId;
use crate::account::AccountId;
use crate::category::CategoryId;
use crate::clock::Clock;

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
