// 017 «Логика при старте и создание счёта»

use domain::{CreateAccount, Currency, DomainError, Ledger};

// Критерий: валидация названия счёта
#[test]
fn account_name_cannot_be_empty() {
    let mut ledger = Ledger::new();
    let result = ledger.create_account(CreateAccount {
        name: "".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    });
    assert_eq!(result, Err(DomainError::EmptyAccountName));
}

// Критерий: валидация названия счёта (пробелы)
#[test]
fn account_name_cannot_be_only_whitespace() {
    let mut ledger = Ledger::new();
    let result = ledger.create_account(CreateAccount {
        name: "   ".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
        credit_limit: 0,
        note: String::new(),
    });
    assert_eq!(result, Err(DomainError::EmptyAccountName));
}

// Критерий: создание счёта с новыми полями
#[test]
fn account_with_credit_limit_and_note() {
    let mut ledger = Ledger::new();
    let id = ledger
        .create_account(CreateAccount {
            name: "Кредитная карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
            credit_limit: 50_000,
            note: "Основная карта".into(),
        })
        .unwrap();
    
    let account = ledger.account(id).unwrap();
    assert_eq!(account.credit_limit(), 50_000);
    assert_eq!(account.note(), "Основная карта");
}
