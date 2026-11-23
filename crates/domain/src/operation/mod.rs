use std::time::SystemTime;

use crate::account::AccountId;
use crate::category::CategoryId;

mod borrow;
mod expense;
mod form;
mod income;
mod lend;
mod repayment;
mod transfer;

#[cfg(test)]
mod tests;

pub use borrow::{Borrow, NewBorrow};
pub use expense::{Expense, NewExpense};
pub use form::OperationForm;
pub use income::{Income, NewIncome};
pub use lend::{Lend, NewLend};
pub use repayment::{NewRepayment, Repayment, RepaymentDirection};
pub use transfer::{NewTransfer, Transfer, TransferAmountFields};

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

impl OperationKind {
    #[must_use]
    #[rustfmt::skip]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Expense,
            Self::Income,
            Self::Transfer,
            Self::Lend,
            Self::Borrow,
            Self::Repayment,
        ]
    }
}

/// Номер внутри дня — место среди записей этой даты, с единицы.
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
