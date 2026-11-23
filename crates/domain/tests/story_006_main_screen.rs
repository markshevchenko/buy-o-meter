mod common;

use std::collections::HashSet;
use std::time::SystemTime;

use common::{create_account, create_top};
use domain::{
    Applicability, DomainError, FeedLine, Ledger, NewBorrow, NewExpense, NewIncome, NewLend,
    NewRepayment, NewTransfer, OperationEntry, OperationId, OperationKind, RepaymentDirection,
    add_days, date,
};

fn operations_on_day(ledger: &Ledger, day: SystemTime) -> Vec<OperationEntry> {
    ledger
        .operations()
        .into_iter()
        .filter(|op| op.date() == day)
        .collect()
}

fn feed_lines_on_day(screen: &domain::MainScreen, day: SystemTime) -> &[FeedLine] {
    screen
        .days()
        .iter()
        .find(|feed_day| feed_day.date() == day)
        .map(domain::FeedDay::lines)
        .unwrap_or_else(|| panic!("day {:?} missing from main screen feed", day))
}

fn all_feed_ids(screen: &domain::MainScreen) -> Vec<OperationId> {
    screen
        .days()
        .iter()
        .flat_map(|day| day.lines().iter().map(FeedLine::id))
        .collect()
}

// 006 «Главный экран при открытии»
#[test]
fn main_screen_lists_saved_operations() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day = date(2026, 11, 22)?;

    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account),
        category_id: Some(food),
        amount: Some(450),
        comment: "Хлеб".into(),
    })?;
    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account),
        category_id: Some(food),
        amount: Some(200),
        comment: String::new(),
    })?;

    let screen = ledger.main_screen();
    assert!(!screen.is_empty());
    assert_eq!(all_feed_ids(&screen).len(), ledger.operations().len());
    Ok(())
}

// 006 «Группировка и порядок»
#[test]
fn feed_groups_by_day_newest_first_and_lines_follow_day_sequence() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account = create_account(&mut ledger, "Наличные", "RUB", 50_000)?;
    let food = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day20 = date(2026, 11, 20)?;
    let day21 = date(2026, 11, 21)?;
    let day22 = date(2026, 11, 22)?;

    for amount in [100, 200, 300] {
        ledger.add_expense(NewExpense {
            date: day20,
            account_id: Some(account),
            category_id: Some(food),
            amount: Some(amount),
            comment: String::new(),
        })?;
    }
    ledger.add_expense(NewExpense {
        date: day21,
        account_id: Some(account),
        category_id: Some(food),
        amount: Some(400),
        comment: String::new(),
    })?;
    ledger.add_expense(NewExpense {
        date: day22,
        account_id: Some(account),
        category_id: Some(food),
        amount: Some(500),
        comment: String::new(),
    })?;

    let screen = ledger.main_screen();
    let days = screen.days();
    assert_eq!(days.len(), 3);
    assert_eq!(days[0].date(), day22);
    assert_eq!(days[1].date(), day21);
    assert_eq!(days[2].date(), day20);

    for window in days.windows(2) {
        assert!(window[0].date() > window[1].date());
    }

    let expected_on_20: Vec<_> = operations_on_day(&ledger, day20)
        .iter()
        .map(OperationEntry::id)
        .collect();
    let actual_on_20: Vec<_> = feed_lines_on_day(&screen, day20)
        .iter()
        .map(FeedLine::id)
        .collect();
    assert_eq!(expected_on_20, actual_on_20);

    Ok(())
}

// 006 «Содержимое строки»
#[test]
fn feed_lines_show_kind_amount_currency_account_and_type_specific_fields() -> Result<(), DomainError>
{
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let card = create_account(&mut ledger, "Карта", "RUB", 0)?;
    let dollars = create_account(&mut ledger, "Доллары", "USD", 50)?;
    let food = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let salary = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let day = date(2026, 11, 22)?;

    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(cash),
        category_id: Some(food),
        amount: Some(450),
        comment: String::new(),
    })?;
    ledger.add_income(NewIncome {
        date: day,
        account_id: Some(card),
        category_id: Some(salary),
        amount: Some(80_000),
        comment: String::new(),
    })?;
    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(cash),
        to_account_id: Some(dollars),
        amount: Some(7_500),
        to_amount: Some(80),
        comment: String::new(),
    })?;
    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(5_000),
    })?;
    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(3_000),
    })?;
    ledger.add_repayment(NewRepayment {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        direction: Some(RepaymentDirection::Received),
        amount: Some(1_000),
    })?;

    let screen = ledger.main_screen();
    let lines = feed_lines_on_day(&screen, day);
    assert_eq!(lines.len(), 6);

    let food_name = ledger.category(food)?.name().to_owned();
    let salary_name = ledger.category(salary)?.name().to_owned();

    let line_by_kind = |kind: OperationKind| -> &FeedLine {
        lines
            .iter()
            .find(|line| line.kind() == kind)
            .unwrap_or_else(|| panic!("no {:?} in feed", kind))
    };

    match line_by_kind(OperationKind::Expense) {
        FeedLine::Expense {
            money,
            category_name,
            ..
        } => {
            assert_eq!(money.amount(), 450);
            assert_eq!(money.currency().as_str(), "RUB");
            assert_eq!(money.account_name(), "Наличные");
            assert_eq!(category_name, &food_name);
        }
        other => panic!("expected expense line, got {other:?}"),
    }

    match line_by_kind(OperationKind::Income) {
        FeedLine::Income {
            money,
            category_name,
            ..
        } => {
            assert_eq!(money.amount(), 80_000);
            assert_eq!(money.currency().as_str(), "RUB");
            assert_eq!(money.account_name(), "Карта");
            assert_eq!(category_name, &salary_name);
        }
        other => panic!("expected income line, got {other:?}"),
    }

    match line_by_kind(OperationKind::Transfer) {
        FeedLine::Transfer { from, to, .. } => {
            assert_eq!(from.amount(), 7_500);
            assert_eq!(from.currency().as_str(), "RUB");
            assert_eq!(from.account_name(), "Наличные");
            assert_eq!(to.amount(), 80);
            assert_eq!(to.currency().as_str(), "USD");
            assert_eq!(to.account_name(), "Доллары");
        }
        other => panic!("expected transfer line, got {other:?}"),
    }

    match line_by_kind(OperationKind::Lend) {
        FeedLine::Lend {
            money,
            counterparty,
            ..
        } => {
            assert_eq!(money.amount(), 5_000);
            assert_eq!(money.currency().as_str(), "RUB");
            assert_eq!(money.account_name(), "Наличные");
            assert_eq!(counterparty, "Петя");
        }
        other => panic!("expected lend line, got {other:?}"),
    }

    match line_by_kind(OperationKind::Borrow) {
        FeedLine::Borrow {
            money,
            counterparty,
            ..
        } => {
            assert_eq!(money.amount(), 3_000);
            assert_eq!(money.currency().as_str(), "RUB");
            assert_eq!(money.account_name(), "Наличные");
            assert_eq!(counterparty, "Маша");
        }
        other => panic!("expected borrow line, got {other:?}"),
    }

    match line_by_kind(OperationKind::Repayment) {
        FeedLine::Repayment {
            money,
            counterparty,
            direction,
            ..
        } => {
            assert_eq!(money.amount(), 1_000);
            assert_eq!(money.currency().as_str(), "RUB");
            assert_eq!(money.account_name(), "Наличные");
            assert_eq!(counterparty, "Петя");
            assert_eq!(*direction, RepaymentDirection::Received);
        }
        other => panic!("expected repayment line, got {other:?}"),
    }

    Ok(())
}

// 006 «Быстрое добавление»
#[test]
fn main_screen_exposes_every_operation_kind_for_quick_add() {
    let ledger = Ledger::new();
    let kinds = ledger.main_screen().add_kinds();
    assert_eq!(kinds, OperationKind::all());
}

// 006 «Новая операция в списке»
#[test]
fn newly_added_operation_appears_in_its_day_on_main_screen() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account = create_account(&mut ledger, "Наличные", "RUB", 10_000)?;
    let food = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day = date(2026, 11, 23)?;

    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account),
        category_id: Some(food),
        amount: Some(990),
        comment: "кофе".into(),
    })?;

    let expense_id = ledger.operations()[0].id();
    let screen = ledger.main_screen();
    let lines = feed_lines_on_day(&screen, day);

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].id(), expense_id);
    assert_eq!(lines[0].kind(), OperationKind::Expense);

    Ok(())
}

// 006 «Пустой список»
#[test]
fn main_screen_is_empty_when_ledger_has_no_operations() {
    let ledger = Ledger::new();
    let screen = ledger.main_screen();
    assert!(screen.is_empty());
    assert!(screen.days().is_empty());
}

// 006 «Переход к редактированию»
#[test]
fn feed_line_id_opens_operation_form_with_matching_kind() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account = create_account(&mut ledger, "Наличные", "RUB", 20_000)?;
    let food = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day = date(2026, 11, 22)?;

    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account),
        category_id: Some(food),
        amount: Some(300),
        comment: String::new(),
    })?;

    let screen = ledger.main_screen();
    let line = &feed_lines_on_day(&screen, day)[0];
    let line_id = line.id();
    let line_kind = line.kind();

    assert_eq!(ledger.operation_form(line_id)?.kind(), line_kind);

    Ok(())
}

// 006 «Более ранние операции»
#[test]
fn main_screen_includes_all_operations_not_only_the_latest_window() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let account = create_account(&mut ledger, "Наличные", "RUB", 1_000_000)?;
    let food = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let start = date(2026, 10, 1)?;

    for offset in 0..12 {
        let day = add_days(start, offset)?;
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account),
            category_id: Some(food),
            amount: Some(100 + i64::from(offset)),
            comment: String::new(),
        })?;
    }

    let screen = ledger.main_screen();
    let feed_ids: HashSet<_> = all_feed_ids(&screen).into_iter().collect();
    let operation_ids: HashSet<_> = ledger.operations().into_iter().map(|op| op.id()).collect();

    assert_eq!(feed_ids, operation_ids);
    assert_eq!(feed_ids.len(), 12);

    let days = screen.days();
    assert_eq!(days.len(), 12);
    assert_eq!(days[0].date(), add_days(start, 11)?);
    assert_eq!(days[11].date(), start);

    Ok(())
}
