use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::Ledger;
use crate::account::AccountId;
use crate::category::CategoryId;
use crate::currency::Currency;
use crate::error::DomainError;
use crate::operation::{
    Borrow, Expense, Income, Lend, NewBorrow, NewExpense, NewIncome, NewLend, NewRepayment,
    NewTransfer, OperationDraft, OperationEntry, OperationForm, OperationId, OperationKind,
    Repayment, RepaymentDirection, Transfer,
};

mod borrow;
mod expense;
mod income;
mod lend;
mod repayment;
mod transfer;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct FeedItem {
    id: OperationId,
    #[serde(with = "crate::epoch")]
    date: SystemTime,
    kind: OperationKind,
}

type DebtTotals = Vec<(String, Currency, i64)>;

struct DebtEffect {
    name: String,
    currency: Currency,
    direction: RepaymentDirection,
    amount: i64,
}

struct RepaymentFit {
    name: String,
    currency: Currency,
    direction: RepaymentDirection,
    amount: i64,
}

enum Drafted {
    Expense {
        date: SystemTime,
        account_id: AccountId,
        category_id: CategoryId,
        amount: i64,
        comment: String,
    },
    Income {
        date: SystemTime,
        account_id: AccountId,
        category_id: CategoryId,
        amount: i64,
        comment: String,
    },
    Transfer {
        date: SystemTime,
        from_account_id: AccountId,
        to_account_id: AccountId,
        amount: i64,
        to_amount: i64,
        comment: String,
    },
    Lend {
        date: SystemTime,
        account_id: AccountId,
        name: String,
        amount: i64,
    },
    Borrow {
        date: SystemTime,
        account_id: AccountId,
        name: String,
        amount: i64,
    },
    Repayment {
        date: SystemTime,
        account_id: AccountId,
        name: String,
        direction: RepaymentDirection,
        amount: i64,
    },
}

enum Saved {
    Expense(Expense),
    Income(Income),
    Transfer(Transfer),
    Lend(Lend),
    Borrow(Borrow),
    Repayment(Repayment),
}

impl Saved {
    fn date(&self) -> SystemTime {
        match self {
            Self::Expense(operation) => operation.date(),
            Self::Income(operation) => operation.date(),
            Self::Transfer(operation) => operation.date(),
            Self::Lend(operation) => operation.date(),
            Self::Borrow(operation) => operation.date(),
            Self::Repayment(operation) => operation.date(),
        }
    }

    fn kind(&self) -> OperationKind {
        match self {
            Self::Expense(_) => OperationKind::Expense,
            Self::Income(_) => OperationKind::Income,
            Self::Transfer(_) => OperationKind::Transfer,
            Self::Lend(_) => OperationKind::Lend,
            Self::Borrow(_) => OperationKind::Borrow,
            Self::Repayment(_) => OperationKind::Repayment,
        }
    }
}

impl Drafted {
    fn balance_draft(&self) -> OperationDraft {
        match self {
            Self::Expense {
                account_id, amount, ..
            } => OperationDraft::Expense {
                account_id: *account_id,
                amount: *amount,
            },
            Self::Income {
                account_id, amount, ..
            } => OperationDraft::Income {
                account_id: *account_id,
                amount: *amount,
            },
            Self::Transfer {
                from_account_id,
                to_account_id,
                amount,
                to_amount,
                ..
            } => OperationDraft::Transfer {
                from_account_id: *from_account_id,
                from_amount: *amount,
                to_account_id: *to_account_id,
                to_amount: *to_amount,
            },
            Self::Lend {
                account_id, amount, ..
            } => OperationDraft::Lend {
                account_id: *account_id,
                amount: *amount,
            },
            Self::Borrow {
                account_id, amount, ..
            } => OperationDraft::Borrow {
                account_id: *account_id,
                amount: *amount,
            },
            Self::Repayment {
                account_id,
                direction,
                amount,
                ..
            } => OperationDraft::Repayment {
                account_id: *account_id,
                amount: *amount,
                direction: *direction,
            },
        }
    }

    fn counterparty_name(&self) -> Option<String> {
        match self {
            Self::Lend { name, .. } | Self::Borrow { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    fn into_saved(self, id: OperationId) -> Saved {
        match self {
            Self::Expense {
                date,
                account_id,
                category_id,
                amount,
                comment,
            } => expense::saved_expense(id, date, account_id, category_id, amount, comment),
            Self::Income {
                date,
                account_id,
                category_id,
                amount,
                comment,
            } => income::saved_income(id, date, account_id, category_id, amount, comment),
            Self::Transfer {
                date,
                from_account_id,
                to_account_id,
                amount,
                to_amount,
                comment,
            } => transfer::saved_transfer(
                id,
                date,
                from_account_id,
                to_account_id,
                amount,
                to_amount,
                comment,
            ),
            Self::Lend {
                date,
                account_id,
                name,
                amount,
            } => Saved::Lend(Lend::new(id, date, account_id, name, amount)),
            Self::Borrow {
                date,
                account_id,
                name,
                amount,
            } => Saved::Borrow(Borrow::new(id, date, account_id, name, amount)),
            Self::Repayment {
                date,
                account_id,
                name,
                direction,
                amount,
            } => Saved::Repayment(Repayment::new(
                id, date, account_id, name, direction, amount,
            )),
        }
    }
}

impl Ledger {
    pub(super) fn allocate_operation_id(&mut self) -> OperationId {
        let id = OperationId::from_u64(self.next_operation_id);
        self.next_operation_id += 1;
        id
    }

    pub(super) fn track_operation(
        &mut self,
        id: OperationId,
        date: SystemTime,
        kind: OperationKind,
        draft: OperationDraft,
    ) {
        self.insert_feed(FeedItem { id, date, kind });
        self.track_draft_link(Some(id), draft);
    }

    pub(super) fn track_draft(&mut self, draft: OperationDraft) {
        self.track_draft_link(None, draft);
    }

    /// Номер внутри дня — место среди операций этой даты, с единицы.
    /// Черновики `add_operation` в ленту не входят.
    #[must_use]
    pub fn operations(&self) -> Vec<OperationEntry> {
        self.feed
            .iter()
            .map(|item| OperationEntry::new(item.id, item.date, item.kind))
            .collect()
    }

    pub fn operation_form(&self, id: OperationId) -> Result<OperationForm, DomainError> {
        let kind = self
            .feed_item(id)
            .ok_or(DomainError::OperationNotFound)?
            .kind;
        self.form_for(id, kind)
    }

    pub fn update_operation(
        &mut self,
        id: OperationId,
        form: OperationForm,
    ) -> Result<(), DomainError> {
        let old = self.feed_item(id).ok_or(DomainError::OperationNotFound)?;
        let drafted = self.prepare_form(form)?;
        self.ensure_can_replace(id, &drafted)?;
        let draft = drafted.balance_draft();
        let counterparty = drafted.counterparty_name();
        self.store(id, old, drafted.into_saved(id), draft, counterparty);
        Ok(())
    }

    pub fn delete_operation(&mut self, id: OperationId) -> Result<(), DomainError> {
        let item = self.feed_item(id).ok_or(DomainError::OperationNotFound)?;
        self.ensure_delete_keeps_debts(id)?;
        self.remove_saved(id, item.kind);
        self.remove_feed(id);
        self.remove_draft(id);
        Ok(())
    }

    fn track_draft_link(&mut self, id: Option<OperationId>, draft: OperationDraft) {
        self.operations.push(draft);
        self.operation_links.push(id);
    }

    fn insert_feed(&mut self, item: FeedItem) {
        let index = self
            .feed
            .partition_point(|existing| existing.date <= item.date);
        self.feed.insert(index, item);
    }

    fn feed_item(&self, id: OperationId) -> Option<FeedItem> {
        self.feed.iter().copied().find(|item| item.id == id)
    }

    fn form_for(&self, id: OperationId, kind: OperationKind) -> Result<OperationForm, DomainError> {
        match kind {
            OperationKind::Expense => self.expense_form(id),
            OperationKind::Income => self.income_form(id),
            OperationKind::Transfer => self.transfer_form(id),
            OperationKind::Lend => self.lend_form(id),
            OperationKind::Borrow => self.borrow_form(id),
            OperationKind::Repayment => self.repayment_form(id),
        }
    }

    fn prepare_form(&self, form: OperationForm) -> Result<Drafted, DomainError> {
        match form {
            OperationForm::Expense(form) => self.prepare_expense(form),
            OperationForm::Income(form) => self.prepare_income(form),
            OperationForm::Transfer(form) => self.prepare_transfer(form),
            OperationForm::Lend(form) => self.prepare_lend(form),
            OperationForm::Borrow(form) => self.prepare_borrow(form),
            OperationForm::Repayment(form) => self.prepare_repayment(form),
        }
    }

    fn ensure_can_replace(&self, id: OperationId, drafted: &Drafted) -> Result<(), DomainError> {
        let (mut received, mut given) = self.totals_without(id)?;
        reject_repayment_that_does_not_fit(self, &received, &given, drafted)?;
        if let Some(effect) = debt_of(self, drafted)? {
            add_effect(&mut received, &mut given, &effect);
        }
        reject_negative(&received, &given)
    }

    fn ensure_delete_keeps_debts(&self, id: OperationId) -> Result<(), DomainError> {
        let (received, given) = self.totals_without(id)?;
        reject_negative(&received, &given)
    }

    fn totals_without(&self, id: OperationId) -> Result<(DebtTotals, DebtTotals), DomainError> {
        let mut received = self.received_raw();
        let mut given = self.given_raw();
        if let Some(mut old) = self.effect_of_saved(id)? {
            old.amount = -old.amount;
            add_effect(&mut received, &mut given, &old);
        }
        Ok((received, given))
    }

    fn received_raw(&self) -> DebtTotals {
        let mut totals = super::debt::balance_totals(self, &self.lends, super::debt::lend_parts);
        super::debt::subtract_repayments(&mut totals, self, RepaymentDirection::Received);
        totals
    }

    fn given_raw(&self) -> DebtTotals {
        let mut totals =
            super::debt::balance_totals(self, &self.borrows, super::debt::borrow_parts);
        super::debt::subtract_repayments(&mut totals, self, RepaymentDirection::Given);
        totals
    }

    fn effect_of_saved(&self, id: OperationId) -> Result<Option<DebtEffect>, DomainError> {
        let Some(item) = self.feed_item(id) else {
            return Err(DomainError::OperationNotFound);
        };
        match item.kind {
            OperationKind::Expense | OperationKind::Income | OperationKind::Transfer => Ok(None),
            OperationKind::Lend => self.lend_effect(id).map(Some),
            OperationKind::Borrow => self.borrow_effect(id).map(Some),
            OperationKind::Repayment => self.saved_repayment_effect(id).map(Some),
        }
    }

    fn store(
        &mut self,
        id: OperationId,
        old: FeedItem,
        saved: Saved,
        draft: OperationDraft,
        counterparty: Option<String>,
    ) {
        let date = saved.date();
        let kind = saved.kind();
        if old.kind == kind && old.date == date {
            self.overwrite(id, saved);
        } else {
            self.remove_saved(id, old.kind);
            self.insert_saved(saved);
            self.move_feed(id, date, kind);
        }
        self.replace_draft(id, draft);
        if let Some(name) = counterparty {
            self.remember_counterparty(&name);
        }
    }

    fn overwrite(&mut self, id: OperationId, saved: Saved) {
        match saved {
            Saved::Expense(expense) => replace_in(&mut self.expenses, id, expense, Expense::id),
            Saved::Income(income) => replace_in(&mut self.incomes, id, income, Income::id),
            Saved::Transfer(transfer) => {
                replace_in(&mut self.transfers, id, transfer, Transfer::id);
            }
            Saved::Lend(lend) => replace_in(&mut self.lends, id, lend, Lend::id),
            Saved::Borrow(borrow) => replace_in(&mut self.borrows, id, borrow, Borrow::id),
            Saved::Repayment(repayment) => {
                replace_in(&mut self.repayments, id, repayment, Repayment::id);
            }
        }
    }

    fn insert_saved(&mut self, saved: Saved) {
        match saved {
            Saved::Expense(expense) => self.place_expense(expense),
            Saved::Income(income) => self.place_income(income),
            Saved::Transfer(transfer) => self.place_transfer(transfer),
            Saved::Lend(lend) => self.place_lend(lend),
            Saved::Borrow(borrow) => self.place_borrow(borrow),
            Saved::Repayment(repayment) => self.place_repayment(repayment),
        }
    }

    fn remove_saved(&mut self, id: OperationId, kind: OperationKind) {
        match kind {
            OperationKind::Expense => remove_id(&mut self.expenses, id, Expense::id),
            OperationKind::Income => remove_id(&mut self.incomes, id, Income::id),
            OperationKind::Transfer => remove_id(&mut self.transfers, id, Transfer::id),
            OperationKind::Lend => remove_id(&mut self.lends, id, Lend::id),
            OperationKind::Borrow => remove_id(&mut self.borrows, id, Borrow::id),
            OperationKind::Repayment => remove_id(&mut self.repayments, id, Repayment::id),
        }
    }

    fn move_feed(&mut self, id: OperationId, date: SystemTime, kind: OperationKind) {
        let Some(index) = self.feed.iter().position(|item| item.id == id) else {
            return;
        };
        if self.feed[index].date == date {
            self.feed[index].kind = kind;
            return;
        }
        self.feed.remove(index);
        self.insert_feed(FeedItem { id, date, kind });
    }

    fn remove_feed(&mut self, id: OperationId) {
        self.feed.retain(|item| item.id != id);
    }

    fn replace_draft(&mut self, id: OperationId, draft: OperationDraft) {
        if let Some(index) = self
            .operation_links
            .iter()
            .position(|link| *link == Some(id))
        {
            self.operations[index] = draft;
        }
    }

    fn remove_draft(&mut self, id: OperationId) {
        if let Some(index) = self
            .operation_links
            .iter()
            .position(|link| *link == Some(id))
        {
            self.operation_links.remove(index);
            self.operations.remove(index);
        }
    }
}

fn find_by_id<T>(
    items: &[T],
    id: OperationId,
    get_id: impl Fn(&T) -> OperationId,
) -> Result<&T, DomainError> {
    items
        .iter()
        .find(|item| get_id(item) == id)
        .ok_or(DomainError::OperationNotFound)
}

fn replace_in<T>(
    items: &mut [T],
    id: OperationId,
    replacement: T,
    get_id: impl Fn(&T) -> OperationId,
) {
    if let Some(slot) = items.iter_mut().find(|item| get_id(item) == id) {
        *slot = replacement;
    }
}

fn remove_id<T>(items: &mut Vec<T>, id: OperationId, get_id: impl Fn(&T) -> OperationId) {
    if let Some(index) = items.iter().position(|item| get_id(item) == id) {
        items.remove(index);
    }
}

fn debt_of(ledger: &Ledger, drafted: &Drafted) -> Result<Option<DebtEffect>, DomainError> {
    match drafted {
        Drafted::Lend {
            account_id,
            name,
            amount,
            ..
        } => Ok(Some(debt_principal(
            ledger,
            *account_id,
            name,
            *amount,
            RepaymentDirection::Received,
        )?)),
        Drafted::Borrow {
            account_id,
            name,
            amount,
            ..
        } => Ok(Some(debt_principal(
            ledger,
            *account_id,
            name,
            *amount,
            RepaymentDirection::Given,
        )?)),
        Drafted::Repayment {
            account_id,
            name,
            direction,
            amount,
            ..
        } => {
            let mut effect = debt_principal(ledger, *account_id, name, *amount, *direction)?;
            effect.amount = -effect.amount;
            Ok(Some(effect))
        }
        _ => Ok(None),
    }
}

fn repayment_fit(ledger: &Ledger, drafted: &Drafted) -> Result<Option<RepaymentFit>, DomainError> {
    let Drafted::Repayment {
        account_id,
        name,
        direction,
        amount,
        ..
    } = drafted
    else {
        return Ok(None);
    };
    Ok(Some(RepaymentFit {
        name: name.clone(),
        currency: ledger.account(*account_id)?.currency().clone(),
        direction: *direction,
        amount: *amount,
    }))
}

fn debt_principal(
    ledger: &Ledger,
    account_id: AccountId,
    name: &str,
    amount: i64,
    direction: RepaymentDirection,
) -> Result<DebtEffect, DomainError> {
    Ok(DebtEffect {
        name: name.to_owned(),
        currency: ledger.account(account_id)?.currency().clone(),
        direction,
        amount,
    })
}

fn reject_repayment_that_does_not_fit(
    ledger: &Ledger,
    received: &DebtTotals,
    given: &DebtTotals,
    drafted: &Drafted,
) -> Result<(), DomainError> {
    let Some(fit) = repayment_fit(ledger, drafted)? else {
        return Ok(());
    };
    let remaining = remaining_of(&fit, received, given);
    if remaining < 0 {
        return Err(DomainError::SubsequentRepaymentsDoNotMatch);
    }
    if remaining == 0 {
        return Err(DomainError::NoDebt);
    }
    if fit.amount > remaining {
        return Err(DomainError::RepaymentExceedsDebt { remaining });
    }
    Ok(())
}

fn remaining_of(fit: &RepaymentFit, received: &DebtTotals, given: &DebtTotals) -> i64 {
    let totals = match fit.direction {
        RepaymentDirection::Received => received,
        RepaymentDirection::Given => given,
    };
    bucket(totals, &fit.name, &fit.currency)
}

fn reject_negative(received: &DebtTotals, given: &DebtTotals) -> Result<(), DomainError> {
    if goes_negative(received) || goes_negative(given) {
        return Err(DomainError::SubsequentRepaymentsDoNotMatch);
    }
    Ok(())
}

fn goes_negative(totals: &DebtTotals) -> bool {
    totals.iter().any(|(_, _, amount)| *amount < 0)
}

fn bucket(totals: &DebtTotals, name: &str, currency: &Currency) -> i64 {
    totals
        .iter()
        .find(|(existing, existing_currency, _)| existing == name && existing_currency == currency)
        .map(|(_, _, amount)| *amount)
        .unwrap_or(0)
}

fn add_effect(received: &mut DebtTotals, given: &mut DebtTotals, effect: &DebtEffect) {
    let totals = match effect.direction {
        RepaymentDirection::Received => received,
        RepaymentDirection::Given => given,
    };
    super::debt::add_balance(totals, &effect.name, effect.currency.clone(), effect.amount);
}
