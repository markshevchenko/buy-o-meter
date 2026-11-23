use std::time::SystemTime;

use super::OperationId;
use crate::account::AccountId;
use crate::clock::Clock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepaymentDirection {
    Received,
    Given,
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
