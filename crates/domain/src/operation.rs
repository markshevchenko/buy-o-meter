use crate::account::AccountId;
use crate::category::CategoryId;
use crate::clock::Clock;
use crate::date::Date;

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

/// Сумма по категории для итогов. Название категории в запись не копируется.
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

/// Форма расхода. Пустые счёт, категория и сумма ещё не заполнены.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExpense {
    pub date: Date,
    pub account_id: Option<AccountId>,
    pub category_id: Option<CategoryId>,
    pub amount: Option<i64>,
    pub comment: String,
}

impl NewExpense {
    /// Дата — сегодня, счёт, категория и сумма пустые, комментарий пустой.
    #[must_use]
    pub fn open(clock: &impl Clock) -> Self {
        let _ = clock;
        todo!()
    }
}

/// Сохранённый расход. Сумма в валюте счёта.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expense {
    date: Date,
    number_in_day: u32,
    account_id: AccountId,
    category_id: CategoryId,
    amount: i64,
    comment: String,
}

impl Expense {
    #[must_use]
    pub fn date(&self) -> Date {
        self.date
    }

    /// Порядковый номер внутри дня, с 1.
    #[must_use]
    pub fn number_in_day(&self) -> u32 {
        self.number_in_day
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
