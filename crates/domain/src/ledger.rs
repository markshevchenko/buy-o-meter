use crate::account::{Account, AccountId, CreateAccount, UpdateAccount};
use crate::error::DomainError;
use crate::operation::OperationDraft;

/// Книга учёта: счета и операции. Хранится в памяти; персистентность — за трейтами позже.
#[derive(Debug)]
pub struct Ledger;

impl Ledger {
    #[must_use]
    pub fn new() -> Self {
        todo!()
    }

    /// Создаёт счёт. Отдельного поля «тип» нет — тип отражается в `name`.
    pub fn create_account(&mut self, input: CreateAccount) -> Result<AccountId, DomainError> {
        let _ = input;
        todo!()
    }

    #[must_use]
    pub fn accounts(&self) -> Vec<Account> {
        todo!()
    }

    pub fn account(&self, id: AccountId) -> Result<Account, DomainError> {
        let _ = id;
        todo!()
    }

    /// Текущий остаток счёта в его валюте: начальный остаток ± операции по этому счёту.
    pub fn balance(&self, id: AccountId) -> Result<i64, DomainError> {
        let _ = id;
        todo!()
    }

    pub fn update_account(
        &mut self,
        id: AccountId,
        input: UpdateAccount,
    ) -> Result<(), DomainError> {
        let _ = (id, input);
        todo!()
    }

    /// Добавляет операцию. Если счетов нет — [`DomainError::AccountRequired`].
    pub fn add_operation(&mut self, draft: OperationDraft) -> Result<(), DomainError> {
        let _ = draft;
        todo!()
    }
}
