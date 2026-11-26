use super::super::Ledger;
use crate::account::{AccountId, CreateAccount};
use crate::currency::Currency;
use crate::error::{DomainError, MissingTransferFields};
use crate::operation::{NewTransfer, TransferAmountFields};
use crate::{add_days, date};

#[test]
fn transfer_within_one_currency_moves_the_amount() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 1_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let day = date(2026, 11, 22)?;

    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(card),
        to_account_id: Some(cash),
        amount: Some(5_000),
        to_amount: None,
        comment: "на карман".into(),
    })?;

    assert_eq!(ledger.balance(card)?, 15_000);
    assert_eq!(ledger.balance(cash)?, 6_000);
    let transfers = ledger.transfers();
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].date(), day);
    assert_eq!(transfers[0].from_account_id(), card);
    assert_eq!(transfers[0].to_account_id(), cash);
    assert_eq!(transfers[0].amount(), 5_000);
    assert_eq!(transfers[0].to_amount(), 5_000);
    assert_eq!(transfers[0].comment(), "на карман");
    Ok(())
}

#[test]
fn transfer_between_currencies_moves_each_amount() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let dollars = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 100,
            credit_limit: 0,
            note: String::new(),
    })?;
    let day = date(2026, 11, 22)?;

    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(card),
        to_account_id: Some(dollars),
        amount: Some(7_500),
        to_amount: Some(80),
        comment: String::new(),
    })?;

    assert_eq!(ledger.balance(card)?, 12_500);
    assert_eq!(ledger.balance(dollars)?, 180);
    let transfers = ledger.transfers();
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].amount(), 7_500);
    assert_eq!(transfers[0].to_amount(), 80);
    Ok(())
}

#[test]
fn transfer_amount_fields_follow_account_currencies() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
            credit_limit: 0,
            note: String::new(),
    })?;
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 0,
            credit_limit: 0,
            note: String::new(),
    })?;
    let dollars = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 0,
            credit_limit: 0,
            note: String::new(),
    })?;

    assert_eq!(
        ledger.transfer_amount_fields(card, cash)?,
        TransferAmountFields::One
    );
    assert_eq!(
        ledger.transfer_amount_fields(card, dollars)?,
        TransferAmountFields::Two
    );
    assert_eq!(
        ledger.transfer_amount_fields(card, cash)?,
        TransferAmountFields::One
    );
    Ok(())
}

#[test]
fn transfer_to_the_same_account_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 1_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let day = date(2026, 11, 22)?;
    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(cash),
        to_account_id: Some(card),
        amount: Some(100),
        to_amount: None,
        comment: String::new(),
    })?;
    let saved = ledger.transfers().to_vec();

    let rejected = ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(card),
        to_account_id: Some(card),
        amount: Some(5_000),
        to_amount: None,
        comment: String::new(),
    });

    assert_eq!(rejected, Err(DomainError::SameAccount));
    assert_eq!(ledger.transfers(), saved.as_slice());
    assert_eq!(ledger.balance(card)?, 20_100);
    assert_eq!(ledger.balance(cash)?, 900);
    Ok(())
}

fn card_and_cash() -> Result<(Ledger, AccountId, AccountId), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 1_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    Ok((ledger, card, cash))
}

fn transfer_input(
    from_account_id: Option<AccountId>,
    to_account_id: Option<AccountId>,
    amount: Option<i64>,
    to_amount: Option<i64>,
) -> Result<NewTransfer, DomainError> {
    Ok(NewTransfer {
        date: date(2026, 11, 22)?,
        from_account_id,
        to_account_id,
        amount,
        to_amount,
        comment: String::new(),
    })
}

fn assert_transfer_fields_rejected(missing: MissingTransferFields) -> Result<(), DomainError> {
    let (mut ledger, card, cash) = card_and_cash()?;
    let err = ledger.add_transfer(transfer_input(
        super::omit_if(missing.from_account, card),
        super::omit_if(missing.to_account, cash),
        super::omit_if(missing.amount, 100),
        None,
    )?);
    assert_eq!(err, Err(DomainError::MissingTransferFields(missing)));
    assert!(ledger.transfers().is_empty());
    assert_eq!(ledger.balance(card)?, 20_000);
    assert_eq!(ledger.balance(cash)?, 1_000);
    Ok(())
}

#[test]
fn transfer_without_accounts_or_amount_is_rejected() -> Result<(), DomainError> {
    assert_transfer_fields_rejected(MissingTransferFields {
        from_account: true,
        to_account: false,
        amount: false,
        to_amount: false,
    })?;
    assert_transfer_fields_rejected(MissingTransferFields {
        from_account: false,
        to_account: true,
        amount: false,
        to_amount: false,
    })?;
    assert_transfer_fields_rejected(MissingTransferFields {
        from_account: false,
        to_account: false,
        amount: true,
        to_amount: false,
    })?;
    Ok(())
}

#[test]
fn transfer_between_currencies_without_credited_amount_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let dollars = ledger.create_account(CreateAccount {
        name: "Доллары".into(),
        currency: Currency::new("USD"),
        opening_balance: 100,
            credit_limit: 0,
            note: String::new(),
    })?;

    let err = ledger.add_transfer(transfer_input(
        Some(card),
        Some(dollars),
        Some(7_500),
        None,
    )?);

    assert_eq!(
        err,
        Err(DomainError::MissingTransferFields(MissingTransferFields {
            from_account: false,
            to_account: false,
            amount: false,
            to_amount: true,
        }))
    );
    assert!(ledger.transfers().is_empty());
    assert_eq!(ledger.balance(card)?, 20_000);
    assert_eq!(ledger.balance(dollars)?, 100);
    Ok(())
}

#[test]
fn transfer_without_any_account_is_rejected() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let err = ledger.add_transfer(transfer_input(
        Some(AccountId::from_u64(1)),
        Some(AccountId::from_u64(1)),
        Some(1),
        None,
    )?);
    assert_eq!(err, Err(DomainError::AccountRequired));
    assert!(ledger.transfers().is_empty());
    Ok(())
}

#[test]
fn transfer_with_unknown_account_is_rejected() -> Result<(), DomainError> {
    let (mut ledger, card, cash) = card_and_cash()?;
    let unknown_from = ledger.add_transfer(transfer_input(
        Some(AccountId::from_u64(99)),
        Some(cash),
        Some(100),
        None,
    )?);
    assert_eq!(unknown_from, Err(DomainError::AccountNotFound));
    assert!(ledger.transfers().is_empty());
    assert_eq!(ledger.balance(card)?, 20_000);
    assert_eq!(ledger.balance(cash)?, 1_000);

    let unknown_to = ledger.add_transfer(transfer_input(
        Some(card),
        Some(AccountId::from_u64(99)),
        Some(100),
        None,
    )?);
    assert_eq!(unknown_to, Err(DomainError::AccountNotFound));
    assert!(ledger.transfers().is_empty());
    assert_eq!(ledger.balance(card)?, 20_000);
    Ok(())
}

#[test]
fn transfer_on_earlier_date_appends_to_that_day() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let card = ledger.create_account(CreateAccount {
        name: "Карта".into(),
        currency: Currency::new("RUB"),
        opening_balance: 20_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let cash = ledger.create_account(CreateAccount {
        name: "Наличные".into(),
        currency: Currency::new("RUB"),
        opening_balance: 1_000,
            credit_limit: 0,
            note: String::new(),
    })?;
    let yesterday = date(2026, 11, 21)?;
    let today = add_days(yesterday, 1)?;
    let transfer = |date, amount, comment: &str| NewTransfer {
        date,
        from_account_id: Some(card),
        to_account_id: Some(cash),
        amount: Some(amount),
        to_amount: None,
        comment: comment.into(),
    };

    ledger.add_transfer(transfer(yesterday, 100, "вчера"))?;
    ledger.add_transfer(transfer(today, 200, "сегодня"))?;
    ledger.add_transfer(transfer(yesterday, 300, "забытый"))?;

    let transfers = ledger.transfers();
    assert_eq!(transfers.len(), 3);
    assert_eq!(transfers[0].date(), yesterday);
    assert_eq!(transfers[0].amount(), 100);
    assert_eq!(transfers[1].date(), yesterday);
    assert_eq!(transfers[1].amount(), 300);
    assert_eq!(transfers[1].comment(), "забытый");
    assert_eq!(transfers[2].date(), today);
    assert_eq!(transfers[2].amount(), 200);
    Ok(())
}
