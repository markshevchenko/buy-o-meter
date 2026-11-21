use domain::{
    CreateAccount, Currency, DomainError, Ledger, OperationDraft, RepaymentDirection, UpdateAccount,
};

// 005 «Создание счёта»
#[test]
fn creating_account_adds_it_with_balance_equal_to_opening() {
    let mut ledger = Ledger::new();
    let id = ledger
        .create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })
        .unwrap();

    let accounts = ledger.accounts();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, id);
    assert_eq!(accounts[0].name, "Наличные");
    assert_eq!(accounts[0].currency, Currency::new("RUB"));
    assert_eq!(accounts[0].opening_balance, 10_000);
    assert_eq!(ledger.balance(id).unwrap(), 10_000);
}

// 005 «Разные валюты»
#[test]
fn balances_stay_in_account_currency_and_are_not_combined() {
    let mut ledger = Ledger::new();
    let rub = ledger
        .create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })
        .unwrap();
    let usd = ledger
        .create_account(CreateAccount {
            name: "Dollar cash".into(),
            currency: Currency::new("USD"),
            opening_balance: 100,
        })
        .unwrap();

    assert_eq!(ledger.balance(rub).unwrap(), 10_000);
    assert_eq!(ledger.account(rub).unwrap().currency, Currency::new("RUB"));
    assert_eq!(ledger.balance(usd).unwrap(), 100);
    assert_eq!(ledger.account(usd).unwrap().currency, Currency::new("USD"));
    assert_ne!(
        ledger.account(rub).unwrap().currency,
        ledger.account(usd).unwrap().currency
    );
}

// 005 «Тип в названии»
#[test]
fn account_has_no_type_field_type_is_reflected_in_name() {
    let mut ledger = Ledger::new();
    let id = ledger
        .create_account(CreateAccount {
            name: "Карта Сбер".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })
        .unwrap();

    let account = ledger.account(id).unwrap();
    assert_eq!(account.name, "Карта Сбер");
    // CreateAccount / Account содержат только name, currency, opening_balance — без поля типа.
    let _ = CreateAccount {
        name: "Вклад".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
    };
}

// 005 «Остаток»
#[test]
fn balance_applies_all_operation_kinds_to_opening() {
    let mut ledger = Ledger::new();
    let cash = ledger
        .create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })
        .unwrap();
    let card = ledger
        .create_account(CreateAccount {
            name: "Карта".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })
        .unwrap();

    ledger
        .add_operation(OperationDraft::Income {
            account_id: cash,
            amount: 5_000,
        })
        .unwrap();
    ledger
        .add_operation(OperationDraft::Expense {
            account_id: cash,
            amount: 1_000,
        })
        .unwrap();
    ledger
        .add_operation(OperationDraft::Transfer {
            from_account_id: cash,
            from_amount: 2_000,
            to_account_id: card,
            to_amount: 2_000,
        })
        .unwrap();
    ledger
        .add_operation(OperationDraft::Lend {
            account_id: cash,
            amount: 500,
        })
        .unwrap();
    ledger
        .add_operation(OperationDraft::Borrow {
            account_id: cash,
            amount: 300,
        })
        .unwrap();
    ledger
        .add_operation(OperationDraft::Repayment {
            account_id: cash,
            amount: 100,
            direction: RepaymentDirection::Received,
        })
        .unwrap();
    ledger
        .add_operation(OperationDraft::Repayment {
            account_id: cash,
            amount: 50,
            direction: RepaymentDirection::Given,
        })
        .unwrap();

    // 10_000 + 5_000 - 1_000 - 2_000 - 500 + 300 + 100 - 50 = 11_850
    assert_eq!(ledger.balance(cash).unwrap(), 11_850);
    assert_eq!(ledger.balance(card).unwrap(), 2_000);
}

// 005 «Редактирование»
#[test]
fn updating_name_or_opening_balance_persists_and_recalculates_balance() {
    let mut ledger = Ledger::new();
    let id = ledger
        .create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })
        .unwrap();
    ledger
        .add_operation(OperationDraft::Expense {
            account_id: id,
            amount: 1_000,
        })
        .unwrap();

    ledger
        .update_account(
            id,
            UpdateAccount {
                name: Some("Карта Сбер".into()),
                opening_balance: Some(20_000),
                currency: None,
            },
        )
        .unwrap();

    let account = ledger.account(id).unwrap();
    assert_eq!(account.name, "Карта Сбер");
    assert_eq!(account.opening_balance, 20_000);
    assert_eq!(ledger.balance(id).unwrap(), 19_000);
}

// 005 «Валюта счёта с операциями»
#[test]
fn currency_cannot_change_after_operations_but_can_before() {
    let mut ledger = Ledger::new();
    let id = ledger
        .create_account(CreateAccount {
            name: "Наличные".into(),
            currency: Currency::new("RUB"),
            opening_balance: 0,
        })
        .unwrap();

    ledger
        .update_account(
            id,
            UpdateAccount {
                name: None,
                opening_balance: None,
                currency: Some(Currency::new("USD")),
            },
        )
        .unwrap();
    assert_eq!(ledger.account(id).unwrap().currency, Currency::new("USD"));

    ledger
        .add_operation(OperationDraft::Income {
            account_id: id,
            amount: 10,
        })
        .unwrap();

    let err = ledger
        .update_account(
            id,
            UpdateAccount {
                name: None,
                opening_balance: None,
                currency: Some(Currency::new("RUB")),
            },
        )
        .unwrap_err();
    assert_eq!(err, DomainError::CurrencyChangeNotAllowed);
    assert_eq!(ledger.account(id).unwrap().currency, Currency::new("USD"));
}

// 005 «Счёт обязателен для операций»
#[test]
fn operation_without_accounts_requires_creating_account_first() {
    let mut ledger = Ledger::new();
    // Нет счетов: любая операция должна требовать сначала создать счёт.
    let err = ledger
        .add_operation(OperationDraft::Expense {
            account_id: domain::AccountId::from_u64(1),
            amount: 100,
        })
        .unwrap_err();
    assert_eq!(err, DomainError::AccountRequired);
}
