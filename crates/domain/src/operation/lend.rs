use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::OperationId;
use crate::account::AccountId;
use crate::clock::Clock;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lend {
    id: OperationId,
    #[serde(with = "crate::epoch")]
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
