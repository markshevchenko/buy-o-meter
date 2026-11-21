/// Ошибки предметной области.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    AccountNotFound,
    /// Валюту нельзя менять: по счёту уже есть операции.
    CurrencyChangeNotAllowed,
    /// Нельзя добавить операцию: нет ни одного счёта.
    AccountRequired,
}
