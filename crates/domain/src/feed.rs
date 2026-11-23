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
        pub id: OperationId,
        pub money: FeedMoney,
        pub category_name: String,
    },
    Income {
        pub id: OperationId,
        pub money: FeedMoney,
        pub category_name: String,
    },
    Transfer {
        pub id: OperationId,
        pub from: FeedMoney,
        pub to: FeedMoney,
    },
    Lend {
        pub id: OperationId,
        pub money: FeedMoney,
        pub counterparty: String,
    },
    Borrow {
        pub id: OperationId,
        pub money: FeedMoney,
        pub counterparty: String,
    },
    Repayment {
        pub id: OperationId,
        pub money: FeedMoney,
        pub counterparty: String,
        pub direction: RepaymentDirection,
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
