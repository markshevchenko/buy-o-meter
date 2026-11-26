mod common;

use common::create_account;
use domain::{
    AccountId, CreateAccount, Currency, DomainError, Ledger, OperationDraft, RepaymentDirection,
    UpdateAccount,
};

// 005 «Создание счёта»
#[test]
fn creating_account_adds_it_with_balance_equal_to_opening() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;

    let accounts = ledger.accounts();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id(), id);
    assert_eq!(accounts[0].name(), "Наличные");
    assert_eq!(accounts[0].currency(), &Currency::new("RUB"));
    assert_eq!(accounts[0].opening_balance(), 10_000);
    assert_eq!(ledger.balance(id)?, 10_000);
    Ok(())
}

// 005 «Разные валюты»
#[test]
fn balances_stay_in_account_currency_and_are_not_combined() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let rub = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let usd = create_account(&mut ledger, "Dollar cash", "USD", 100)?;

    assert_eq!(ledger.balance(rub)?, 10_000);
    assert_eq!(ledger.account(rub)?.currency(), &Currency::new("RUB"));
    assert_eq!(ledger.balance(usd)?, 100);
    assert_eq!(ledger.account(usd)?.currency(), &Currency::new("USD"));
    assert_ne!(
        ledger.account(rub)?.currency(),
        ledger.account(usd)?.currency()
    );
    Ok(())
}

// 005 «Тип в названии»
#[test]
fn account_has_no_type_field_type_is_reflected_in_name() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_account(&mut ledger, "Карта Сбер", "RUB", 0)?;

    let account = ledger.account(id)?;
    assert_eq!(account.name(), "Карта Сбер");
    // CreateAccount / Account содержат только name, currency, opening_balance — без поля типа.
    let _ = CreateAccount {
        name: "Вклад".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
            credit_limit: 0,
            note: String::new(),
    };
    Ok(())
}

// 005 «Остаток»
#[test]
fn balance_applies_all_operation_kinds_to_opening() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let card = create_account(&mut ledger, "Карта", "RUB", 0)?;

    ledger.add_operation(OperationDraft::Income {
        account_id: cash,
        amount: 5_000,
    })?;
    ledger.add_operation(OperationDraft::Expense {
        account_id: cash,
        amount: 1_000,
    })?;
    ledger.add_operation(OperationDraft::Transfer {
        from_account_id: cash,
        from_amount: 2_000,
        to_account_id: card,
        to_amount: 2_000,
    })?;
    ledger.add_operation(OperationDraft::Lend {
        account_id: cash,
        amount: 500,
    })?;
    ledger.add_operation(OperationDraft::Borrow {
        account_id: cash,
        amount: 300,
    })?;
    ledger.add_operation(OperationDraft::Repayment {
        account_id: cash,
        amount: 100,
        direction: RepaymentDirection::Received,
    })?;
    ledger.add_operation(OperationDraft::Repayment {
        account_id: cash,
        amount: 50,
        direction: RepaymentDirection::Given,
    })?;

    // 10_000 + 5_000 - 1_000 - 2_000 - 500 + 300 + 100 - 50 = 11_850
    assert_eq!(ledger.balance(cash)?, 11_850);
    assert_eq!(ledger.balance(card)?, 2_000);
    Ok(())
}

// 005 «Редактирование»
#[test]
fn updating_name_or_opening_balance_persists_and_recalculates_balance() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    ledger.add_operation(OperationDraft::Expense {
        account_id: id,
        amount: 1_000,
    })?;

    ledger.update_account(
        id,
        UpdateAccount {
            name: Some("Карта Сбер".into()),
            opening_balance: Some(20_000),
            currency: None,
                credit_limit: None,
                note: None,
        },
    )?;

    let account = ledger.account(id)?;
    assert_eq!(account.name(), "Карта Сбер");
    assert_eq!(account.opening_balance(), 20_000);
    assert_eq!(ledger.balance(id)?, 19_000);
    Ok(())
}

// 005 «Валюта счёта с операциями»
#[test]
fn currency_cannot_change_after_operations_but_can_before() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let id = create_account(&mut ledger, "Наличные", "RUB", 0)?;

    ledger.update_account(
        id,
        UpdateAccount {
            name: None,
            opening_balance: None,
            currency: Some(Currency::new("USD")),
            credit_limit: None,
            note: None,
        },
    )?;
    assert_eq!(ledger.account(id)?.currency(), &Currency::new("USD"));

    ledger.add_operation(OperationDraft::Income {
        account_id: id,
        amount: 10,
    })?;

    let err = ledger.update_account(
        id,
        UpdateAccount {
            name: None,
            opening_balance: None,
            currency: Some(Currency::new("RUB")),
            credit_limit: None,
            note: None,
        },
    );
    assert_eq!(err, Err(DomainError::CurrencyChangeNotAllowed));
    assert_eq!(ledger.account(id)?.currency(), &Currency::new("USD"));
    Ok(())
}

// 005 «Счёт обязателен для операций»
#[test]
fn operation_without_accounts_requires_creating_account_first() {
    let mut ledger = Ledger::new();
    // Нет счетов: любая операция должна требовать сначала создать счёт.
    let err = ledger.add_operation(OperationDraft::Expense {
        account_id: AccountId::from_u64(1),
        amount: 100,
    });
    assert_eq!(err, Err(DomainError::AccountRequired));
}
