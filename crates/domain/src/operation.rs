use crate::account::AccountId;

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
