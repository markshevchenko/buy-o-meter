use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::OperationId;
use crate::account::AccountId;
use crate::clock::Clock;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transfer {
    id: OperationId,
    #[serde(with = "crate::epoch")]
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
