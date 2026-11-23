use std::time::SystemTime;

use super::Ledger;
use crate::category::CategoryId;
use crate::clock::Clock;
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
        let _ = clock.today();
        todo!()
    }

    pub fn month(year: i32, month: i32) -> Result<Self, DomainError> {
        todo!("month {year}-{month}")
    }

    pub fn week_containing(day: SystemTime) -> Result<Self, DomainError> {
        todo!("{day:?}")
    }

    pub fn previous(self) -> Result<Self, DomainError> {
        todo!("{self:?}")
    }

    pub fn next(self) -> Result<Self, DomainError> {
        todo!("{self:?}")
    }

    #[must_use]
    pub fn unit(self) -> PeriodUnit {
        self.unit
    }

    #[must_use]
    pub fn start(self) -> SystemTime {
        self.start
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
        todo!("{kind:?} {period:?} categories={}", self.categories.len())
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

fn amount_of(amounts: &[CurrencyAmount], currency: &Currency) -> i64 {
    amounts
        .iter()
        .find(|item| item.currency() == currency)
        .map_or(0, CurrencyAmount::amount)
}
