use std::time::SystemTime;

use crate::currency::Currency;
use crate::operation::{OperationId, OperationKind, RepaymentDirection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedMoney {
    amount: i64,
    currency: Currency,
    account_name: String,
}

impl FeedMoney {
    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }

    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    #[must_use]
    pub fn account_name(&self) -> &str {
        &self.account_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedLine {
    Expense {
        id: OperationId,
        money: FeedMoney,
        category_name: String,
    },
    Income {
        id: OperationId,
        money: FeedMoney,
        category_name: String,
    },
    Transfer {
        id: OperationId,
        from: FeedMoney,
        to: FeedMoney,
    },
    Lend {
        id: OperationId,
        money: FeedMoney,
        counterparty: String,
    },
    Borrow {
        id: OperationId,
        money: FeedMoney,
        counterparty: String,
    },
    Repayment {
        id: OperationId,
        money: FeedMoney,
        counterparty: String,
        direction: RepaymentDirection,
    },
}

impl FeedLine {
    #[must_use]
    pub fn id(&self) -> OperationId {
        match self {
            Self::Expense { id, .. }
            | Self::Income { id, .. }
            | Self::Transfer { id, .. }
            | Self::Lend { id, .. }
            | Self::Borrow { id, .. }
            | Self::Repayment { id, .. } => *id,
        }
    }

    #[must_use]
    pub fn kind(&self) -> OperationKind {
        match self {
            Self::Expense { .. } => OperationKind::Expense,
            Self::Income { .. } => OperationKind::Income,
            Self::Transfer { .. } => OperationKind::Transfer,
            Self::Lend { .. } => OperationKind::Lend,
            Self::Borrow { .. } => OperationKind::Borrow,
            Self::Repayment { .. } => OperationKind::Repayment,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedDay {
    date: SystemTime,
    lines: Vec<FeedLine>,
}

impl FeedDay {
    #[must_use]
    pub fn date(&self) -> SystemTime {
        self.date
    }

    #[must_use]
    pub fn lines(&self) -> &[FeedLine] {
        &self.lines
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainScreen {
    days: Vec<FeedDay>,
}

impl MainScreen {
    #[must_use]
    pub fn days(&self) -> &[FeedDay] {
        &self.days
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    #[must_use]
    pub fn add_kinds(&self) -> &'static [OperationKind] {
        OperationKind::all()
    }
}
