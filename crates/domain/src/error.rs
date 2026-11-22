#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingFields {
    pub account: bool,
    pub category: bool,
    pub amount: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingTransferFields {
    pub from_account: bool,
    pub to_account: bool,
    pub amount: bool,
    pub to_amount: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    AccountNotFound,
    CurrencyChangeNotAllowed,
    AccountRequired,
    CategoryNotFound,
    ApplicabilityRequired,
    CategoryTooDeep,
    CategoryHasSubcategories,
    TransferTargetRequired,
    UnsuitableTransferTarget,
    MissingFields(MissingFields),
    MissingTransferFields(MissingTransferFields),
    MixedCurrencies,
    InvalidDate,
    SameAccount,
}
