mod common;

use common::{create_account, create_child, create_top};
use domain::{
    AccountId, Applicability, CategoryId, Currency, DomainError, FixedClock, Ledger, MonthTotal,
    NewBorrow, NewExpense, NewIncome, NewLend, NewRepayment, NewTransfer, PeriodUnit,
    RepaymentDirection, ReportKind, ReportPeriod, add_days, date,
};

fn rub() -> Currency {
    Currency::new("RUB")
}

fn usd() -> Currency {
    Currency::new("USD")
}

fn find_top<'a>(report: &'a domain::CategoryReport, name: &str) -> &'a domain::CategoryReportNode {
    report
        .categories()
        .iter()
        .find(|node| node.name() == name)
        .unwrap_or_else(|| panic!("top-level category {name} not found"))
}

fn find_child<'a>(
    node: &'a domain::CategoryReportNode,
    name: &str,
) -> &'a domain::CategoryReportNode {
    node.children()
        .iter()
        .find(|child| child.name() == name)
        .unwrap_or_else(|| panic!("child category {name} not found"))
}

fn month_key(entry: &MonthTotal) -> (i32, i32) {
    (entry.year(), entry.month())
}

fn food_tree(ledger: &mut Ledger) -> Result<(CategoryId, CategoryId, CategoryId), DomainError> {
    let food = create_top(ledger, "Питание", Applicability::Expense)?;
    let products = create_child(ledger, "Продукты", food, None)?;
    let lunches = create_child(ledger, "Обеды в городе", food, None)?;
    Ok((food, products, lunches))
}

fn expense(
    ledger: &mut Ledger,
    account: AccountId,
    category: CategoryId,
    day: std::time::SystemTime,
    amount: i64,
) -> Result<(), DomainError> {
    ledger.add_expense(NewExpense {
        date: day,
        account_id: Some(account),
        category_id: Some(category),
        amount: Some(amount),
        comment: String::new(),
    })
}

fn income(
    ledger: &mut Ledger,
    account: AccountId,
    category: CategoryId,
    day: std::time::SystemTime,
    amount: i64,
) -> Result<(), DomainError> {
    ledger.add_income(NewIncome {
        date: day,
        account_id: Some(account),
        category_id: Some(category),
        amount: Some(amount),
        comment: String::new(),
    })
}

// 014 «Дерево категорий»
#[test]
fn expense_report_shows_top_level_totals_including_subcategories() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 50_000)?;
    let (food, products, lunches) = food_tree(&mut ledger)?;
    let _transport = create_top(&mut ledger, "Транспорт", Applicability::Expense)?;
    let today = date(2026, 11, 22)?;
    let clock = FixedClock::new(today);

    expense(&mut ledger, cash, products, today, 3_000)?;
    expense(&mut ledger, cash, lunches, today, 1_500)?;

    let period = ReportPeriod::current_month(&clock)?;
    let report = ledger.category_report(ReportKind::Expense, period);
    assert_eq!(report.categories().len(), 1);
    let food_node = find_top(&report, "Питание");
    assert_eq!(food_node.id(), food);
    assert_eq!(food_node.amount(&rub()), 4_500);
    let _ = (products, lunches);

    Ok(())
}

// 014 «Период дерева»
#[test]
fn category_report_respects_month_week_and_adjacent_periods() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let category = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let today = date(2026, 11, 22)?;
    let clock = FixedClock::new(today);
    let october_day = date(2026, 10, 15)?;
    let november_day = date(2026, 11, 2)?;
    let this_week_day = date(2026, 11, 18)?;
    let previous_week_day = date(2026, 11, 10)?;

    expense(&mut ledger, cash, category, october_day, 1_000)?;
    expense(&mut ledger, cash, category, november_day, 2_000)?;
    expense(&mut ledger, cash, category, this_week_day, 300)?;
    expense(&mut ledger, cash, category, previous_week_day, 400)?;

    let default_period = ReportPeriod::current_month(&clock)?;
    assert_eq!(default_period.unit(), PeriodUnit::Month);
    let default_report = ledger.category_report(ReportKind::Expense, default_period);
    assert_eq!(find_top(&default_report, "Продукты").amount(&rub()), 2_700);

    let october = ReportPeriod::month(2026, 10)?;
    assert_eq!(october.unit(), PeriodUnit::Month);
    assert_eq!(
        find_top(
            &ledger.category_report(ReportKind::Expense, october),
            "Продукты"
        )
        .amount(&rub()),
        1_000
    );

    let november = october.next()?;
    assert_eq!(
        find_top(
            &ledger.category_report(ReportKind::Expense, november),
            "Продукты"
        )
        .amount(&rub()),
        2_700
    );

    let this_week = ReportPeriod::week_containing(today)?;
    assert_eq!(this_week.unit(), PeriodUnit::Week);
    assert_eq!(
        find_top(
            &ledger.category_report(ReportKind::Expense, this_week),
            "Продукты"
        )
        .amount(&rub()),
        300
    );

    let previous_week = this_week.previous()?;
    assert_eq!(
        find_top(
            &ledger.category_report(ReportKind::Expense, previous_week),
            "Продукты"
        )
        .amount(&rub()),
        400
    );

    Ok(())
}

// 014 «Раскрытие»
#[test]
fn parent_total_includes_own_operations_not_listed_among_children() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 50_000)?;
    let (food, products, lunches) = food_tree(&mut ledger)?;
    let day = date(2026, 11, 20)?;

    expense(&mut ledger, cash, food, day, 1_000)?;
    expense(&mut ledger, cash, products, day, 2_000)?;
    expense(&mut ledger, cash, lunches, day, 500)?;

    let period = ReportPeriod::month(2026, 11)?;
    let report = ledger.category_report(ReportKind::Expense, period);
    let food_node = find_top(&report, "Питание");
    assert_eq!(food_node.id(), food);
    assert_eq!(food_node.amount(&rub()), 3_500);
    let children_sum = find_child(food_node, "Продукты").amount(&rub())
        + find_child(food_node, "Обеды в городе").amount(&rub());
    assert_eq!(children_sum, 2_500);
    assert!(food_node.amount(&rub()) > children_sum);

    Ok(())
}

// 014 «Расходы или доходы»
#[test]
fn report_kind_limits_operations_to_expenses_or_incomes() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let expense_cat = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let income_cat = create_top(&mut ledger, "Зарплата", Applicability::Income)?;
    let day = date(2026, 11, 22)?;
    let period = ReportPeriod::month(2026, 11)?;

    expense(&mut ledger, cash, expense_cat, day, 5_000)?;
    income(&mut ledger, cash, income_cat, day, 80_000)?;

    let expense_report = ledger.category_report(ReportKind::Expense, period);
    assert_eq!(expense_report.categories().len(), 1);
    assert_eq!(find_top(&expense_report, "Продукты").amount(&rub()), 5_000);

    let income_report = ledger.category_report(ReportKind::Income, period);
    assert_eq!(income_report.categories().len(), 1);
    assert_eq!(find_top(&income_report, "Зарплата").amount(&rub()), 80_000);

    Ok(())
}

// 014 «Помесячно по категории»
#[test]
fn category_months_lists_each_month_for_leaf_category() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let (_food, products, _) = food_tree(&mut ledger)?;
    let today = date(2026, 11, 22)?;

    expense(&mut ledger, cash, products, date(2025, 3, 10)?, 1_000)?;
    expense(&mut ledger, cash, products, date(2025, 5, 1)?, 2_000)?;
    expense(&mut ledger, cash, products, date(2026, 11, 5)?, 500)?;

    let months = ledger.category_months(ReportKind::Expense, products, today)?;
    let keys: Vec<_> = months.months().iter().map(month_key).collect();
    assert_eq!(keys.first(), Some(&(2025, 3)));
    assert_eq!(keys.last(), Some(&(2026, 11)));

    let march = months
        .months()
        .iter()
        .find(|m| month_key(m) == (2025, 3))
        .expect("March 2025");
    assert_eq!(march.amount(&rub()), 1_000);

    let april = months
        .months()
        .iter()
        .find(|m| month_key(m) == (2025, 4))
        .expect("April 2025 gap");
    assert!(april.amounts().is_empty());

    let may = months
        .months()
        .iter()
        .find(|m| month_key(m) == (2025, 5))
        .expect("May 2025");
    assert_eq!(may.amount(&rub()), 2_000);

    let november = months
        .months()
        .iter()
        .find(|m| month_key(m) == (2026, 11))
        .expect("November 2026");
    assert_eq!(november.amount(&rub()), 500);

    Ok(())
}

// 014 «Помесячно по категории верхнего уровня»
#[test]
fn category_months_for_parent_includes_subcategory_operations() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let (food, products, lunches) = food_tree(&mut ledger)?;
    let today = date(2026, 11, 22)?;

    expense(&mut ledger, cash, food, date(2026, 10, 1)?, 700)?;
    expense(&mut ledger, cash, products, date(2026, 10, 15)?, 1_000)?;
    expense(&mut ledger, cash, lunches, date(2026, 11, 1)?, 300)?;

    let months = ledger.category_months(ReportKind::Expense, food, today)?;
    let october = months
        .months()
        .iter()
        .find(|m| month_key(m) == (2026, 10))
        .expect("October 2026");
    assert_eq!(october.amount(&rub()), 1_700);

    let november = months
        .months()
        .iter()
        .find(|m| month_key(m) == (2026, 11))
        .expect("November 2026");
    assert_eq!(november.amount(&rub()), 300);

    let _ = (products, lunches);

    Ok(())
}

// 014 «Все месяцы»
#[test]
fn category_months_span_from_first_operation_through_current_month() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let category = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let today = date(2026, 11, 22)?;

    expense(&mut ledger, cash, category, date(2025, 3, 5)?, 1_000)?;

    let months = ledger.category_months(ReportKind::Expense, category, today)?;
    let keys: Vec<_> = months.months().iter().map(month_key).collect();
    assert_eq!(keys.first(), Some(&(2025, 3)));
    assert_eq!(keys.last(), Some(&(2026, 11)));
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(keys.len(), 21);

    let october_2026 = months
        .months()
        .iter()
        .find(|m| month_key(m) == (2026, 10))
        .expect("empty month inside range");
    assert!(october_2026.amounts().is_empty());

    Ok(())
}

// 014 «Валюты»
#[test]
fn category_report_keeps_each_currency_separate() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let rub_account = create_account(&mut ledger, "Наличные", "RUB", 50_000)?;
    let usd_account = create_account(&mut ledger, "Доллары", "USD", 1_000)?;
    let category = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day = date(2026, 11, 22)?;
    let period = ReportPeriod::month(2026, 11)?;

    expense(&mut ledger, rub_account, category, day, 1_500)?;
    expense(&mut ledger, usd_account, category, day, 40)?;

    let report = ledger.category_report(ReportKind::Expense, period);
    let node = find_top(&report, "Продукты");
    assert_eq!(node.amount(&rub()), 1_500);
    assert_eq!(node.amount(&usd()), 40);
    assert_eq!(node.amounts().len(), 2);
    assert_eq!(node.amounts()[0].currency().as_str(), "RUB");
    assert_eq!(node.amounts()[1].currency().as_str(), "USD");
    assert_ne!(node.amount(&rub()), node.amount(&usd()));

    Ok(())
}

// 014 «Что не учитывается»
#[test]
fn category_report_ignores_transfers_debts_and_repayments() -> Result<(), DomainError> {
    let mut ledger = Ledger::new();
    let cash = create_account(&mut ledger, "Наличные", "RUB", 100_000)?;
    let card = create_account(&mut ledger, "Карта", "RUB", 50_000)?;
    let category = create_top(&mut ledger, "Продукты", Applicability::Expense)?;
    let day = date(2026, 11, 22)?;
    let period = ReportPeriod::month(2026, 11)?;

    expense(&mut ledger, cash, category, day, 2_000)?;

    ledger.add_transfer(NewTransfer {
        date: day,
        from_account_id: Some(card),
        to_account_id: Some(cash),
        amount: Some(90_000),
        to_amount: None,
        comment: String::new(),
    })?;

    ledger.add_lend(NewLend {
        date: day,
        account_id: Some(cash),
        counterparty: Some("Петя".into()),
        amount: Some(30_000),
    })?;

    ledger.add_borrow(NewBorrow {
        date: day,
        account_id: Some(card),
        counterparty: Some("Маша".into()),
        amount: Some(15_000),
    })?;

    ledger.add_repayment(NewRepayment {
        date: add_days(day, 1)?,
        account_id: Some(cash),
        counterparty: Some("Маша".into()),
        amount: Some(5_000),
        direction: Some(RepaymentDirection::Given),
    })?;

    let report = ledger.category_report(ReportKind::Expense, period);
    let node = find_top(&report, "Продукты");
    assert_eq!(node.amount(&rub()), 2_000);

    Ok(())
}
