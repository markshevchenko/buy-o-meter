#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    AccountNotFound,
    CurrencyChangeNotAllowed,
    AccountRequired,
}
