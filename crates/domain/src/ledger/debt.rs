use super::{Ledger, place_on_date};
use crate::account::AccountId;
use crate::counterparty::{Counterparty, CounterpartyId, Debt};
use crate::currency::Currency;
use crate::error::{DomainError, MissingDebtFields, MissingRepaymentFields};
use crate::operation::{
    Borrow, Lend, NewBorrow, NewLend, NewRepayment, OperationDraft, OperationKind, Repayment,
    RepaymentDirection,
};
use crate::quick_access::QuickAccess;

impl Ledger {
    pub fn add_lend(&mut self, input: NewLend) -> Result<(), DomainError> {
        let (account_id, name, amount) =
            complete_debt(input.account_id, input.counterparty, input.amount)?;
        self.ensure_account(account_id)?;
        self.remember_counterparty(&name);
        let id = self.allocate_operation_id();
        let date = input.date;
        self.place_lend(Lend::new(id, date, account_id, name, amount));
        #[rustfmt::skip]
            self.track_operation(id, date, OperationKind::Lend, OperationDraft::Lend {
                account_id,
                amount,
            });
        Ok(())
    }

    #[must_use]
    pub fn lends(&self) -> &[Lend] {
        &self.lends
    }

    pub fn add_borrow(&mut self, input: NewBorrow) -> Result<(), DomainError> {
        let (account_id, name, amount) =
            complete_debt(input.account_id, input.counterparty, input.amount)?;
        self.ensure_account(account_id)?;
        self.remember_counterparty(&name);
        let id = self.allocate_operation_id();
        let date = input.date;
        self.place_borrow(Borrow::new(id, date, account_id, name, amount));
        #[rustfmt::skip]
            self.track_operation(id, date, OperationKind::Borrow, OperationDraft::Borrow {
                account_id,
                amount,
            });
        Ok(())
    }

    #[must_use]
    pub fn borrows(&self) -> &[Borrow] {
        &self.borrows
    }

    pub fn add_repayment(&mut self, input: NewRepayment) -> Result<(), DomainError> {
        let (account_id, name, direction, amount) = complete_repayment(
            input.account_id,
            input.counterparty,
            input.direction,
            input.amount,
        )?;
        self.ensure_account(account_id)?;
        let remaining = self.debt_remaining(account_id, &name, direction)?;
        if remaining == 0 {
            return Err(DomainError::NoDebt);
        }
        if amount > remaining {
            return Err(DomainError::RepaymentExceedsDebt { remaining });
        }
        let id = self.allocate_operation_id();
        let date = input.date;
        #[rustfmt::skip]
            self.place_repayment(Repayment::new(
                id,
                date,
                account_id,
                name,
                direction,
                amount,
            ));
        #[rustfmt::skip]
            self.track_operation(id, date, OperationKind::Repayment, OperationDraft::Repayment {
                account_id,
                amount,
                direction,
            });
        Ok(())
    }

    #[must_use]
    pub fn repayments(&self) -> &[Repayment] {
        &self.repayments
    }

    #[must_use]
    pub fn counterparties(&self) -> &[Counterparty] {
        &self.counterparties
    }

    /// Меняет имя только в списке. В операциях остаётся прежнее.
    pub fn rename_counterparty(
        &mut self,
        id: CounterpartyId,
        name: String,
    ) -> Result<(), DomainError> {
        self.reject_if_counterparty_has_debts(id)?;
        self.find_counterparty_mut(id)
            .ok_or(DomainError::CounterpartyNotFound)?
            .set_name(name);
        Ok(())
    }

    /// Убирает имя из списка. В операциях оно остаётся.
    pub fn delete_counterparty(&mut self, id: CounterpartyId) -> Result<(), DomainError> {
        self.reject_if_counterparty_has_debts(id)?;
        self.counterparties
            .retain(|counterparty| counterparty.id() != id);
        Ok(())
    }

    /// Встречный долг в этот список не сворачивается.
    #[must_use]
    pub fn debts_to_me(&self) -> Vec<Debt> {
        self.open_debts(&self.lends, lend_parts, RepaymentDirection::Received)
    }

    /// Встречный долг в этот список не сворачивается.
    #[must_use]
    pub fn my_debts(&self) -> Vec<Debt> {
        self.open_debts(&self.borrows, borrow_parts, RepaymentDirection::Given)
    }

    #[must_use]
    pub fn quick_access(&self) -> QuickAccess {
        let accounts = super::account::account_lines(&self.accounts, &self.operations);
        QuickAccess::new(accounts, self.my_debts(), self.debts_to_me())
    }

    fn debt_remaining(
        &self,
        account_id: AccountId,
        name: &str,
        direction: RepaymentDirection,
    ) -> Result<i64, DomainError> {
        let currency = self.account(account_id)?.currency().clone();
        let debts = match direction {
            RepaymentDirection::Received => self.debts_to_me(),
            RepaymentDirection::Given => self.my_debts(),
        };
        Ok(debts
            .iter()
            .find(|debt| debt.counterparty_name() == name && debt.currency() == &currency)
            .map(Debt::amount)
            .unwrap_or(0))
    }

    fn open_debts<T>(
        &self,
        items: &[T],
        parts: impl Fn(&T) -> (&str, AccountId, i64),
        repaid_by: RepaymentDirection,
    ) -> Vec<Debt> {
        let mut totals = balance_totals(self, items, parts);
        subtract_repayments(&mut totals, self, repaid_by);
        positive_debts(totals)
    }

    fn reject_if_counterparty_has_debts(&self, id: CounterpartyId) -> Result<(), DomainError> {
        let name = self
            .counterparties
            .iter()
            .find(|counterparty| counterparty.id() == id)
            .map(Counterparty::name)
            .ok_or(DomainError::CounterpartyNotFound)?;
        if has_open_debt(&self.debts_to_me(), name) || has_open_debt(&self.my_debts(), name) {
            return Err(DomainError::CounterpartyHasDebts);
        }
        Ok(())
    }

    pub(super) fn remember_counterparty(&mut self, name: &str) {
        if self
            .counterparties
            .iter()
            .any(|counterparty| counterparty.name() == name)
        {
            return;
        }
        let id = CounterpartyId::from_u64(self.next_counterparty_id);
        self.next_counterparty_id += 1;
        self.counterparties
            .push(Counterparty::new(id, name.to_owned()));
    }

    pub(super) fn place_lend(&mut self, lend: Lend) {
        place_on_date(&mut self.lends, lend, Lend::date);
    }

    pub(super) fn place_borrow(&mut self, borrow: Borrow) {
        place_on_date(&mut self.borrows, borrow, Borrow::date);
    }

    pub(super) fn place_repayment(&mut self, repayment: Repayment) {
        place_on_date(&mut self.repayments, repayment, Repayment::date);
    }
}

fn has_open_debt(debts: &[Debt], name: &str) -> bool {
    debts.iter().any(|debt| debt.counterparty_name() == name)
}

pub(super) fn balance_totals<T>(
    ledger: &Ledger,
    items: &[T],
    parts: impl Fn(&T) -> (&str, AccountId, i64),
) -> Vec<(String, Currency, i64)> {
    let mut totals = Vec::new();
    for item in items {
        let (name, account_id, amount) = parts(item);
        add_account_amount(ledger, &mut totals, name, account_id, amount);
    }
    totals
}

pub(super) fn subtract_repayments(
    totals: &mut Vec<(String, Currency, i64)>,
    ledger: &Ledger,
    direction: RepaymentDirection,
) {
    for repayment in &ledger.repayments {
        if repayment.direction() != direction {
            continue;
        }
        add_account_amount(
            ledger,
            totals,
            repayment.counterparty_name(),
            repayment.account_id(),
            -repayment.amount(),
        );
    }
}

fn add_account_amount(
    ledger: &Ledger,
    totals: &mut Vec<(String, Currency, i64)>,
    name: &str,
    account_id: AccountId,
    amount: i64,
) {
    let Some(currency) = ledger
        .find_account(account_id)
        .map(|account| account.currency().clone())
    else {
        return;
    };
    add_balance(totals, name, currency, amount);
}

fn positive_debts(totals: Vec<(String, Currency, i64)>) -> Vec<Debt> {
    totals
        .into_iter()
        .filter(|(_, _, amount)| *amount > 0)
        .map(|(name, currency, amount)| Debt::new(name, currency, amount))
        .collect()
}

pub(super) fn add_balance(
    totals: &mut Vec<(String, Currency, i64)>,
    name: &str,
    currency: Currency,
    amount: i64,
) {
    if let Some((_, _, total)) = totals
        .iter_mut()
        .find(|(existing, existing_currency, _)| existing == name && existing_currency == &currency)
    {
        *total += amount;
        return;
    }
    totals.push((name.to_owned(), currency, amount));
}

#[rustfmt::skip]
pub(super) fn lend_parts(lend: &Lend) -> (&str, AccountId, i64) {
    (
        lend.counterparty_name(),
        lend.account_id(),
        lend.amount(),
    )
}

#[rustfmt::skip]
pub(super) fn borrow_parts(borrow: &Borrow) -> (&str, AccountId, i64) {
    (
        borrow.counterparty_name(),
        borrow.account_id(),
        borrow.amount(),
    )
}

pub(super) fn complete_repayment(
    account_id: Option<AccountId>,
    counterparty: Option<String>,
    direction: Option<RepaymentDirection>,
    amount: Option<i64>,
) -> Result<(AccountId, String, RepaymentDirection, i64), DomainError> {
    match (account_id, counterparty, direction, amount) {
        (Some(account_id), Some(counterparty), Some(direction), Some(amount)) => {
            Ok((account_id, counterparty, direction, amount))
        }
        (account_id, counterparty, direction, amount) => Err(DomainError::MissingRepaymentFields(
            MissingRepaymentFields {
                account: account_id.is_none(),
                counterparty: counterparty.is_none(),
                direction: direction.is_none(),
                amount: amount.is_none(),
            },
        )),
    }
}

pub(super) fn complete_debt(
    account_id: Option<AccountId>,
    counterparty: Option<String>,
    amount: Option<i64>,
) -> Result<(AccountId, String, i64), DomainError> {
    match (account_id, counterparty, amount) {
        (Some(account_id), Some(counterparty), Some(amount)) => {
            Ok((account_id, counterparty, amount))
        }
        (account_id, counterparty, amount) => {
            Err(DomainError::MissingDebtFields(MissingDebtFields {
                account: account_id.is_none(),
                counterparty: counterparty.is_none(),
                amount: amount.is_none(),
            }))
        }
    }
}
