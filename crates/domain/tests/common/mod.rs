// Каждая история — отдельный крейт и вызывает не все функции.
#![allow(dead_code)]

use domain::{
    AccountId, Applicability, CategoryId, CreateAccount, CreateCategory, Currency, DomainError,
    Ledger,
};

pub fn create_account(
    ledger: &mut Ledger,
    name: &str,
    currency: &str,
    opening_balance: i64,
) -> Result<AccountId, DomainError> {
    ledger.create_account(CreateAccount {
        name: name.into(),
        currency: Currency::new(currency),
        opening_balance,
        credit_limit: 0,
        note: String::new(),
    })
}

pub fn create_top(
    ledger: &mut Ledger,
    name: &str,
    applicability: Applicability,
) -> Result<CategoryId, DomainError> {
    ledger.create_category(CreateCategory {
        name: name.into(),
        parent_id: None,
        applicability: Some(applicability),
    })
}

pub fn create_child(
    ledger: &mut Ledger,
    name: &str,
    parent_id: CategoryId,
    applicability: Option<Applicability>,
) -> Result<CategoryId, DomainError> {
    ledger.create_category(CreateCategory {
        name: name.into(),
        parent_id: Some(parent_id),
        applicability,
    })
}
