use crate::account::AccountId;

/// Направление возврата долга.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepaymentDirection {
    /// Мне вернули — остаток счёта растёт.
    Received,
    /// Я вернул — остаток счёта уменьшается.
    Given,
}

/// Черновик операции, достаточный для пересчёта остатков счетов.
/// Полные поля операций появятся в историях ввода.
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
    /// Перевод: списание с `from`, зачисление на `to` (суммы могут отличаться при разных валютах).
    Transfer {
        from_account_id: AccountId,
        from_amount: i64,
        to_account_id: AccountId,
        to_amount: i64,
    },
    /// Выданный долг — остаток счёта уменьшается.
    Lend {
        account_id: AccountId,
        amount: i64,
    },
    /// Взятый долг — остаток счёта растёт.
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
