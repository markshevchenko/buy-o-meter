use std::time::SystemTime;

use time::{Date, UtcDateTime};

use super::Ledger;
use crate::account::AccountId;
use crate::category::{Category, CategoryId};
use crate::clock::{self, Clock};
use crate::currency::Currency;
use crate::error::DomainError;

/// Расходы или доходы в отчёте по категориям.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Expense,
    Income,
}

/// Месяц или неделя с понедельника по воскресенье.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodUnit {
    Month,
    Week,
}

/// Период дерева отчёта. По умолчанию — текущий месяц.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportPeriod {
    unit: PeriodUnit,
    start: SystemTime,
}

impl ReportPeriod {
    pub fn current_month(clock: &impl Clock) -> Result<Self, DomainError> {
        let today = calendar_date(clock.today())?;
        Ok(Self {
            unit: PeriodUnit::Month,
            start: month_start(today.year(), month_index(today.month()))?,
        })
    }

    pub fn month(year: i32, month: i32) -> Result<Self, DomainError> {
        Ok(Self {
            unit: PeriodUnit::Month,
            start: month_start(year, month)?,
        })
    }

    pub fn week_containing(day: SystemTime) -> Result<Self, DomainError> {
        let date = calendar_date(day)?;
        let back = i64::from(date.weekday().number_days_from_monday());
        Ok(Self {
            unit: PeriodUnit::Week,
            start: clock::add_days(day, -back)?,
        })
    }

    pub fn previous(self) -> Result<Self, DomainError> {
        todo!("{self:?}")
    }

    pub fn next(self) -> Result<Self, DomainError> {
        Ok(Self {
            unit: self.unit,
            start: self.exclusive_end()?,
        })
    }

    #[must_use]
    pub fn unit(self) -> PeriodUnit {
        self.unit
    }

    #[must_use]
    pub fn start(self) -> SystemTime {
        self.start
    }

    fn contains(self, moment: SystemTime) -> bool {
        let Ok(end) = self.exclusive_end() else {
            return false;
        };
        moment >= self.start && moment < end
    }

    fn exclusive_end(self) -> Result<SystemTime, DomainError> {
        match self.unit {
            PeriodUnit::Month => {
                let start = calendar_date(self.start)?;
                let (year, month) = shift_month(start.year(), month_index(start.month()), 1)?;
                month_start(year, month)
            }
            PeriodUnit::Week => clock::add_days(self.start, 7),
        }
    }

    /// Конец периода, не включая этот момент.
    pub fn end(self) -> Result<SystemTime, DomainError> {
        todo!("{self:?}")
    }
}

/// Сумма в одной валюте. Разные валюты не складываются.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyAmount {
    currency: Currency,
    amount: i64,
}

impl CurrencyAmount {
    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

/// Категория в дереве отчёта.
///
/// Сумма включает операции этой категории и всех её подкатегорий.
/// В `children` только подкатегории со своей суммой: операции самого родителя
/// отдельной строкой не показываются. Строки без сумм за период отсутствуют.
/// Валюты идут по коду, категории — в порядке создания.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryReportNode {
    id: CategoryId,
    name: String,
    amounts: Vec<CurrencyAmount>,
    children: Vec<CategoryReportNode>,
}

impl CategoryReportNode {
    #[must_use]
    pub fn id(&self) -> CategoryId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn amounts(&self) -> &[CurrencyAmount] {
        &self.amounts
    }

    #[must_use]
    pub fn amount(&self, currency: &Currency) -> i64 {
        amount_of(&self.amounts, currency)
    }

    #[must_use]
    pub fn children(&self) -> &[CategoryReportNode] {
        &self.children
    }
}

/// Дерево категорий верхнего уровня за период.
///
/// Учитываются только операции выбранного вида. Переводы, долги и возвраты
/// в отчёт не входят.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryReport {
    categories: Vec<CategoryReportNode>,
}

impl CategoryReport {
    #[must_use]
    pub fn categories(&self) -> &[CategoryReportNode] {
        &self.categories
    }
}

/// Суммы одного месяца. Пустой `amounts` — операций в этом месяце не было.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthTotal {
    year: i32,
    month: i32,
    amounts: Vec<CurrencyAmount>,
}

impl MonthTotal {
    #[must_use]
    pub fn year(&self) -> i32 {
        self.year
    }

    #[must_use]
    pub fn month(&self) -> i32 {
        self.month
    }

    #[must_use]
    pub fn amounts(&self) -> &[CurrencyAmount] {
        &self.amounts
    }

    #[must_use]
    pub fn amount(&self, currency: &Currency) -> i64 {
        amount_of(&self.amounts, currency)
    }
}

/// Помесячные суммы категории: от месяца первой операции этого вида
/// до месяца `today` включительно, по возрастанию. Последний месяц — текущий.
/// Для категории верхнего уровня каждый месяц включает её операции и операции
/// подкатегорий. Если таких операций нет, список пуст.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryMonths {
    months: Vec<MonthTotal>,
}

impl CategoryMonths {
    #[must_use]
    pub fn months(&self) -> &[MonthTotal] {
        &self.months
    }
}

impl Ledger {
    #[must_use]
    pub fn category_report(&self, kind: ReportKind, period: ReportPeriod) -> CategoryReport {
        let _ = kind;
        CategoryReport {
            categories: self.report_roots(&self.expense_totals(period)),
        }
    }

    fn report_roots(
        &self,
        totals: &[(CategoryId, Vec<CurrencyAmount>)],
    ) -> Vec<CategoryReportNode> {
        self.categories
            .iter()
            .filter(|category| category.parent_id().is_none())
            .filter_map(|category| self.report_node(category, totals))
            .collect()
    }

    fn report_node(
        &self,
        category: &Category,
        totals: &[(CategoryId, Vec<CurrencyAmount>)],
    ) -> Option<CategoryReportNode> {
        let children = self.child_nodes(category.id(), totals);
        let amounts = rolled_amounts(direct_amounts(totals, category.id()), &children);
        if amounts.is_empty() {
            return None;
        }
        Some(CategoryReportNode {
            id: category.id(),
            name: category.name().to_owned(),
            amounts,
            children,
        })
    }

    fn child_nodes(
        &self,
        parent_id: CategoryId,
        totals: &[(CategoryId, Vec<CurrencyAmount>)],
    ) -> Vec<CategoryReportNode> {
        self.categories
            .iter()
            .filter(|category| category.parent_id() == Some(parent_id))
            .filter_map(|category| self.report_node(category, totals))
            .collect()
    }

    fn expense_totals(&self, period: ReportPeriod) -> Vec<(CategoryId, Vec<CurrencyAmount>)> {
        let mut totals = Vec::new();
        for expense in &self.expenses {
            if !period.contains(expense.date()) {
                continue;
            }
            let Some(currency) = self.currency_of(expense.account_id()) else {
                continue;
            };
            add_total(
                &mut totals,
                expense.category_id(),
                currency,
                expense.amount(),
            );
        }
        totals
    }

    fn currency_of(&self, account_id: AccountId) -> Option<Currency> {
        self.find_account(account_id)
            .map(|account| account.currency().clone())
    }

    pub fn category_months(
        &self,
        kind: ReportKind,
        category_id: CategoryId,
        today: SystemTime,
    ) -> Result<CategoryMonths, DomainError> {
        let _ = self
            .categories
            .iter()
            .any(|category| category.id() == category_id);
        todo!("{kind:?} {category_id:?} {today:?}")
    }
}

fn calendar_date(time: SystemTime) -> Result<Date, DomainError> {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| DomainError::InvalidDate)?;
    let seconds = i64::try_from(duration.as_secs()).map_err(|_| DomainError::InvalidDate)?;
    UtcDateTime::from_unix_timestamp(seconds)
        .map(|moment| moment.date())
        .map_err(|_| DomainError::InvalidDate)
}

fn month_start(year: i32, month: i32) -> Result<SystemTime, DomainError> {
    clock::date(year, month, 1)
}

fn month_index(month: time::Month) -> i32 {
    i32::from(u8::from(month))
}

fn shift_month(year: i32, month: i32, delta: i32) -> Result<(i32, i32), DomainError> {
    let index = year
        .checked_mul(12)
        .and_then(|months| months.checked_add(month - 1))
        .and_then(|months| months.checked_add(delta))
        .ok_or(DomainError::InvalidDate)?;
    Ok((index.div_euclid(12), index.rem_euclid(12) + 1))
}

fn direct_amounts(
    totals: &[(CategoryId, Vec<CurrencyAmount>)],
    id: CategoryId,
) -> Vec<CurrencyAmount> {
    totals
        .iter()
        .find(|(category_id, _)| *category_id == id)
        .map(|(_, amounts)| amounts.clone())
        .unwrap_or_default()
}

fn rolled_amounts(
    mut amounts: Vec<CurrencyAmount>,
    children: &[CategoryReportNode],
) -> Vec<CurrencyAmount> {
    for child in children {
        for item in child.amounts() {
            add_amount(&mut amounts, item.currency().clone(), item.amount());
        }
    }
    amounts.sort_by(|left, right| left.currency().as_str().cmp(right.currency().as_str()));
    amounts
}

fn add_total(
    totals: &mut Vec<(CategoryId, Vec<CurrencyAmount>)>,
    category_id: CategoryId,
    currency: Currency,
    amount: i64,
) {
    if let Some((_, amounts)) = totals.iter_mut().find(|(id, _)| *id == category_id) {
        add_amount(amounts, currency, amount);
        return;
    }
    totals.push((category_id, vec![CurrencyAmount { currency, amount }]));
}

fn add_amount(amounts: &mut Vec<CurrencyAmount>, currency: Currency, amount: i64) {
    if let Some(existing) = amounts.iter_mut().find(|item| item.currency() == &currency) {
        existing.amount += amount;
        return;
    }
    amounts.push(CurrencyAmount { currency, amount });
}

fn amount_of(amounts: &[CurrencyAmount], currency: &Currency) -> i64 {
    amounts
        .iter()
        .find(|item| item.currency() == currency)
        .map_or(0, CurrencyAmount::amount)
}

#[cfg(test)]
mod tests {
    use crate::account::{AccountId, CreateAccount};
    use crate::category::{Applicability, CategoryId, CreateCategory};
    use crate::clock::{date, FixedClock};
    use crate::currency::Currency;
    use crate::error::DomainError;
    use crate::operation::NewExpense;

    use super::{Ledger, PeriodUnit, ReportKind, ReportPeriod};

    #[test]
    fn current_month_starts_on_the_first() -> Result<(), DomainError> {
        let clock = FixedClock::new(date(2026, 11, 22)?);
        let period = ReportPeriod::current_month(&clock)?;
        assert_eq!(period.unit(), PeriodUnit::Month);
        assert_eq!(period.start(), date(2026, 11, 1)?);
        Ok(())
    }

    #[test]
    fn expense_report_rolls_subcategories_into_top_level() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = account(&mut ledger)?;
        let food = category(&mut ledger, "Питание", None)?;
        let products = category(&mut ledger, "Продукты", Some(food))?;
        let lunches = category(&mut ledger, "Обеды", Some(food))?;
        let _transport = category(&mut ledger, "Транспорт", None)?;
        let day = date(2026, 11, 22)?;
        spend(&mut ledger, cash, products, day, 3_000)?;
        spend(&mut ledger, cash, lunches, day, 1_500)?;

        let period = ReportPeriod::current_month(&FixedClock::new(day))?;
        let report = ledger.category_report(ReportKind::Expense, period);
        assert_eq!(report.categories().len(), 1);
        let food_node = &report.categories()[0];
        assert_eq!(food_node.id(), food);
        assert_eq!(food_node.amount(&Currency::new("RUB")), 4_500);
        Ok(())
    }

    #[test]
    fn current_month_report_skips_other_months() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = account(&mut ledger)?;
        let products = category(&mut ledger, "Продукты", None)?;
        let today = date(2026, 11, 22)?;
        spend(&mut ledger, cash, products, date(2026, 10, 15)?, 1_000)?;
        spend(&mut ledger, cash, products, today, 2_000)?;

        let period = ReportPeriod::current_month(&FixedClock::new(today))?;
        let report = ledger.category_report(ReportKind::Expense, period);
        assert_eq!(report.categories()[0].amount(&Currency::new("RUB")), 2_000);
        Ok(())
    }

    #[test]
    fn month_period_counts_only_that_month() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = account(&mut ledger)?;
        let products = category(&mut ledger, "Продукты", None)?;
        spend(&mut ledger, cash, products, date(2026, 10, 15)?, 1_000)?;
        spend(&mut ledger, cash, products, date(2026, 11, 2)?, 2_000)?;

        let october = ReportPeriod::month(2026, 10)?;
        assert_eq!(october.unit(), PeriodUnit::Month);
        assert_eq!(october.start(), date(2026, 10, 1)?);
        assert_eq!(spent(&ledger, october), 1_000);
        assert_eq!(ReportPeriod::month(2026, 13), Err(DomainError::InvalidDate));
        Ok(())
    }

    #[test]
    fn next_month_follows_the_calendar() -> Result<(), DomainError> {
        let october = ReportPeriod::month(2026, 10)?;
        let november = october.next()?;
        assert_eq!(november.unit(), PeriodUnit::Month);
        assert_eq!(november.start(), date(2026, 11, 1)?);

        let january = ReportPeriod::month(2026, 12)?.next()?;
        assert_eq!(january.start(), date(2027, 1, 1)?);
        Ok(())
    }

    #[test]
    fn week_containing_starts_on_monday() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = account(&mut ledger)?;
        let products = category(&mut ledger, "Продукты", None)?;
        spend(&mut ledger, cash, products, date(2026, 11, 2)?, 2_000)?;
        spend(&mut ledger, cash, products, date(2026, 11, 18)?, 300)?;
        spend(&mut ledger, cash, products, date(2026, 11, 10)?, 400)?;

        let today = date(2026, 11, 22)?;
        let this_week = ReportPeriod::week_containing(today)?;
        assert_eq!(this_week.unit(), PeriodUnit::Week);
        assert_eq!(this_week.start(), date(2026, 11, 16)?);
        assert_eq!(spent(&ledger, this_week), 300);
        assert_eq!(
            ReportPeriod::week_containing(date(2026, 11, 16)?)?.start(),
            date(2026, 11, 16)?
        );
        Ok(())
    }

    fn spent(ledger: &Ledger, period: ReportPeriod) -> i64 {
        ledger
            .category_report(ReportKind::Expense, period)
            .categories()[0]
            .amount(&Currency::new("RUB"))
    }

    fn account(ledger: &mut Ledger) -> Result<AccountId, DomainError> {
        ledger.create_account(CreateAccount {
            name: "Наличные".to_owned(),
            currency: Currency::new("RUB"),
            opening_balance: 50_000,
        })
    }

    fn category(
        ledger: &mut Ledger,
        name: &str,
        parent_id: Option<CategoryId>,
    ) -> Result<CategoryId, DomainError> {
        ledger.create_category(CreateCategory {
            name: name.to_owned(),
            parent_id,
            applicability: Some(Applicability::Expense),
        })
    }

    fn spend(
        ledger: &mut Ledger,
        account_id: AccountId,
        category_id: CategoryId,
        day: std::time::SystemTime,
        amount: i64,
    ) -> Result<(), DomainError> {
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(amount),
            comment: String::new(),
        })
    }
}
