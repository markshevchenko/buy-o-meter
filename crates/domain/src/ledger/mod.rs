use std::time::SystemTime;

use crate::account::{Account, AccountId};
use crate::category::{Category, CategoryId};
use crate::counterparty::{Counterparty, CounterpartyId};
use crate::operation::{
    Borrow, Expense, Income, Lend, OperationDraft, OperationId, Record, Repayment, Transfer,
};

mod account;
mod category;
mod debt;
mod edit;
mod feed;
mod operation;
mod report;
mod snapshot;

pub(crate) use snapshot::{LedgerFile, StoreError, decode_ledger, encode_ledger};

pub use report::{
    CategoryMonths, CategoryReport, CategoryReportNode, CurrencyAmount, MonthTotal, PeriodUnit,
    ReportKind, ReportPeriod,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct Ledger {
    next_account_id: u64,
    accounts: Vec<Account>,
    operations: Vec<OperationDraft>,
    operation_links: Vec<Option<OperationId>>,
    next_operation_id: u64,
    feed: Vec<edit::FeedItem>,
    next_category_id: u64,
    categories: Vec<Category>,
    starter_seeded: bool,
    records: Vec<Record>,
    expenses: Vec<Expense>,
    incomes: Vec<Income>,
    transfers: Vec<Transfer>,
    lends: Vec<Lend>,
    borrows: Vec<Borrow>,
    repayments: Vec<Repayment>,
    next_counterparty_id: u64,
    counterparties: Vec<Counterparty>,
}

impl Ledger {
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_account_id: 1,
            accounts: Vec::new(),
            operations: Vec::new(),
            operation_links: Vec::new(),
            next_operation_id: 1,
            feed: Vec::new(),
            next_category_id: 1,
            categories: Vec::new(),
            starter_seeded: false,
            records: Vec::new(),
            expenses: Vec::new(),
            incomes: Vec::new(),
            transfers: Vec::new(),
            lends: Vec::new(),
            borrows: Vec::new(),
            repayments: Vec::new(),
            next_counterparty_id: 1,
            counterparties: Vec::new(),
        }
    }

    fn find_category(&self, id: CategoryId) -> Option<&Category> {
        self.categories.iter().find(|category| category.id() == id)
    }

    fn find_category_mut(&mut self, id: CategoryId) -> Option<&mut Category> {
        self.categories
            .iter_mut()
            .find(|category| category.id() == id)
    }

    fn find_account(&self, id: AccountId) -> Option<&Account> {
        self.accounts.iter().find(|account| account.id() == id)
    }

    fn find_account_mut(&mut self, id: AccountId) -> Option<&mut Account> {
        self.accounts.iter_mut().find(|account| account.id() == id)
    }

    fn find_counterparty_mut(&mut self, id: CounterpartyId) -> Option<&mut Counterparty> {
        self.counterparties
            .iter_mut()
            .find(|counterparty| counterparty.id() == id)
    }
}

fn place_on_date<T>(items: &mut Vec<T>, item: T, date_of: impl Fn(&T) -> SystemTime) {
    let date = date_of(&item);
    let index = items.partition_point(|existing| date_of(existing) <= date);
    items.insert(index, item);
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}
