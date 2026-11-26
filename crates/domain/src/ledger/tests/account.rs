use super::super::Ledger;
use crate::account::{AccountId, CreateAccount, UpdateAccount};
use crate::currency::Currency;
use crate::error::DomainError;
use crate::operation::OperationDraft;

#[test]
fn new_ledger_has_no_accounts() {
    let ledger = Ledger::new();
    assert!(ledger.accounts().is_empty());
}

#[test]
fn create_account_assigns_id_and_stores_fields() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 100,
            credit_limit: 0,
            note: String::new(),
    })?;
    let account = ledger.account(id)?;
    assert_eq!(account.opening_balance(), 100);
    assert_eq!(ledger.balance(id)?, 100);
    Ok(())
}

#[test]
fn expense_decreases_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 100,
            credit_limit: 0,
            note: String::new(),
    })?;
    ledger.add_operation(OperationDraft::Expense {
        account_id: id,
        amount: 40,
    })?;
    assert_eq!(ledger.balance(id)?, 60);
    Ok(())
}

#[test]
fn add_operation_without_accounts_fails() {
    let mut ledger = Ledger::new();
    let err = ledger.add_operation(OperationDraft::Expense {
        account_id: AccountId::from_u64(1),
        amount: 1,
    });
    assert_eq!(err, Err(DomainError::AccountRequired));
}

#[test]
fn currency_change_blocked_when_account_has_operations() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
            credit_limit: 0,
            note: String::new(),
    })?;
    ledger.add_operation(OperationDraft::Income {
        account_id: id,
        amount: 1,
    })?;
    let err = ledger.update_account(
        id,
        UpdateAccount {
            name: None,
            opening_balance: None,
            currency: Some(Currency::new("USD")),
            credit_limit: None,
            note: None,
        },
    );
    assert_eq!(err, Err(DomainError::CurrencyChangeNotAllowed));
    Ok(())
}
