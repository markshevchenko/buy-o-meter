use std::time::SystemTime;

use super::Ledger;
use crate::account::AccountId;
use crate::feed::{FeedDay, FeedLine, FeedMoney, MainScreen};
use crate::operation::{
    Borrow, Expense, Income, Lend, OperationEntry, OperationId, OperationKind, Repayment, Transfer,
};

impl Ledger {
    /// Свежие дни сверху, внутри дня — по порядковому номеру.
    #[must_use]
    pub fn main_screen(&self) -> MainScreen {
        MainScreen::new(self.feed_days())
    }

    fn feed_days(&self) -> Vec<FeedDay> {
        let mut days = Vec::new();
        for entry in self.operations() {
            let Some(line) = self.feed_line(&entry) else {
                continue;
            };
            push_line(&mut days, entry.date(), line);
        }
        days.reverse();
        days
    }

    fn feed_line(&self, entry: &OperationEntry) -> Option<FeedLine> {
        let id = entry.id();
        match entry.kind() {
            OperationKind::Expense => self.expense_line(id),
            OperationKind::Income => self.income_line(id),
            OperationKind::Transfer => self.transfer_line(id),
            OperationKind::Lend => self.lend_line(id),
            OperationKind::Borrow => self.borrow_line(id),
            OperationKind::Repayment => self.repayment_line(id),
        }
    }

    fn expense_line(&self, id: OperationId) -> Option<FeedLine> {
        let expense = find_saved(&self.expenses, id, Expense::id)?;
        let money = self.feed_money(expense.account_id(), expense.amount())?;
        let category_name = self.find_category(expense.category_id())?.name().to_owned();
        Some(FeedLine::Expense {
            id,
            money,
            category_name,
        })
    }

    fn income_line(&self, id: OperationId) -> Option<FeedLine> {
        let income = find_saved(&self.incomes, id, Income::id)?;
        let money = self.feed_money(income.account_id(), income.amount())?;
        let category_name = self.find_category(income.category_id())?.name().to_owned();
        Some(FeedLine::Income {
            id,
            money,
            category_name,
        })
    }

    fn transfer_line(&self, id: OperationId) -> Option<FeedLine> {
        let transfer = find_saved(&self.transfers, id, Transfer::id)?;
        let from = self.feed_money(transfer.from_account_id(), transfer.amount())?;
        let to = self.feed_money(transfer.to_account_id(), transfer.to_amount())?;
        Some(FeedLine::Transfer { id, from, to })
    }

    fn lend_line(&self, id: OperationId) -> Option<FeedLine> {
        let lend = find_saved(&self.lends, id, Lend::id)?;
        let money = self.feed_money(lend.account_id(), lend.amount())?;
        Some(FeedLine::Lend {
            id,
            money,
            counterparty: lend.counterparty_name().to_owned(),
        })
    }

    fn borrow_line(&self, id: OperationId) -> Option<FeedLine> {
        let borrow = find_saved(&self.borrows, id, Borrow::id)?;
        let money = self.feed_money(borrow.account_id(), borrow.amount())?;
        Some(FeedLine::Borrow {
            id,
            money,
            counterparty: borrow.counterparty_name().to_owned(),
        })
    }

    fn repayment_line(&self, id: OperationId) -> Option<FeedLine> {
        let repayment = find_saved(&self.repayments, id, Repayment::id)?;
        let money = self.feed_money(repayment.account_id(), repayment.amount())?;
        Some(FeedLine::Repayment {
            id,
            money,
            counterparty: repayment.counterparty_name().to_owned(),
            direction: repayment.direction(),
        })
    }

    fn feed_money(&self, account_id: AccountId, amount: i64) -> Option<FeedMoney> {
        let account = self.find_account(account_id)?;
        Some(FeedMoney::new(
            amount,
            account.currency().clone(),
            account.name().to_owned(),
        ))
    }
}

fn push_line(days: &mut Vec<FeedDay>, date: SystemTime, line: FeedLine) {
    if let Some(day) = days.last_mut().filter(|day| day.date() == date) {
        day.push_line(line);
        return;
    }
    days.push(FeedDay::new(date, vec![line]));
}

fn find_saved<T>(
    items: &[T],
    id: OperationId,
    get_id: impl Fn(&T) -> OperationId,
) -> Option<&T> {
    items.iter().find(|item| get_id(item) == id)
}

#[cfg(test)]
mod tests {
    use super::super::Ledger;
    use crate::account::{CreateAccount, UpdateAccount};
    use crate::category::{Applicability, CreateCategory, UpdateCategory};
    use crate::currency::Currency;
    use crate::feed::FeedLine;
    use crate::operation::{
        NewBorrow, NewExpense, NewIncome, NewLend, NewRepayment, NewTransfer, OperationDraft,
        OperationKind, RepaymentDirection,
    };
    use crate::{DomainError, date};

    fn rub_account(ledger: &mut Ledger, name: &str) -> Result<crate::AccountId, DomainError> {
        ledger.create_account(CreateAccount {
            name: name.into(),
            currency: Currency::new("RUB"),
            opening_balance: 10_000,
        })
    }

    fn expense_category(ledger: &mut Ledger, name: &str) -> Result<crate::CategoryId, DomainError> {
        ledger.create_category(CreateCategory {
            name: name.into(),
            parent_id: None,
            applicability: Some(Applicability::Expense),
        })
    }

    #[test]
    fn empty_ledger_has_empty_main_screen() {
        let ledger = Ledger::new();
        let screen = ledger.main_screen();
        assert!(screen.is_empty());
        assert!(screen.days().is_empty());
    }

    #[test]
    fn expense_line_shows_money_and_current_names() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let account_id = rub_account(&mut ledger, "Наличные")?;
        let category_id = expense_category(&mut ledger, "Продукты")?;
        let day = date(2026, 11, 22)?;
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(450),
            comment: String::new(),
        })?;
        ledger.add_operation(OperationDraft::Expense {
            account_id,
            amount: 100,
        })?;
        ledger.update_account(
            account_id,
            UpdateAccount {
                name: Some("Кошелёк".into()),
                opening_balance: None,
                currency: None,
            },
        )?;
        ledger.update_category(
            category_id,
            UpdateCategory {
                name: Some("Еда".into()),
                applicability: None,
            },
        )?;

        let lines = ledger.main_screen().days()[0].lines().to_vec();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].id(), ledger.operations()[0].id());
        assert_eq!(lines[0].kind(), OperationKind::Expense);
        let FeedLine::Expense {
            money,
            category_name,
            ..
        } = &lines[0]
        else {
            panic!("expected expense");
        };
        assert_eq!(money.amount(), 450);
        assert_eq!(money.currency().as_str(), "RUB");
        assert_eq!(money.account_name(), "Кошелёк");
        assert_eq!(category_name, "Еда");
        Ok(())
    }

    #[test]
    fn days_are_newest_first_and_keep_operation_order() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let account_id = rub_account(&mut ledger, "Наличные")?;
        let category_id = expense_category(&mut ledger, "Продукты")?;
        let older = date(2026, 11, 20)?;
        let newer = date(2026, 11, 22)?;
        for amount in [100, 200] {
            ledger.add_expense(NewExpense {
                date: older,
                account_id: Some(account_id),
                category_id: Some(category_id),
                amount: Some(amount),
                comment: String::new(),
            })?;
        }
        ledger.add_expense(NewExpense {
            date: newer,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(300),
            comment: String::new(),
        })?;

        let screen = ledger.main_screen();
        let days = screen.days();
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].date(), newer);
        assert_eq!(days[1].date(), older);
        let expected: Vec<_> = ledger
            .operations()
            .into_iter()
            .filter(|operation| operation.date() == older)
            .map(|operation| operation.id())
            .collect();
        let actual: Vec<_> = days[1].lines().iter().map(FeedLine::id).collect();
        assert_eq!(actual, expected);
        Ok(())
    }

    fn line_of(ledger: &Ledger, kind: OperationKind) -> FeedLine {
        ledger
            .main_screen()
            .days()
            .iter()
            .flat_map(|day| day.lines().iter().cloned())
            .find(|line| line.kind() == kind)
            .unwrap_or_else(|| panic!("no {kind:?}"))
    }

    #[test]
    fn income_line_shows_money_and_category() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let account_id = rub_account(&mut ledger, "Карта")?;
        let category_id = ledger.create_category(CreateCategory {
            name: "Зарплата".into(),
            parent_id: None,
            applicability: Some(Applicability::Income),
        })?;
        ledger.add_income(NewIncome {
            date: date(2026, 11, 22)?,
            account_id: Some(account_id),
            category_id: Some(category_id),
            amount: Some(80_000),
            comment: String::new(),
        })?;

        let FeedLine::Income {
            money,
            category_name,
            ..
        } = line_of(&ledger, OperationKind::Income)
        else {
            panic!("expected income");
        };
        assert_eq!(money.amount(), 80_000);
        assert_eq!(money.currency().as_str(), "RUB");
        assert_eq!(money.account_name(), "Карта");
        assert_eq!(category_name, "Зарплата");
        Ok(())
    }

    #[test]
    fn transfer_line_shows_both_accounts_when_currencies_differ() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = rub_account(&mut ledger, "Наличные")?;
        let dollars = ledger.create_account(CreateAccount {
            name: "Доллары".into(),
            currency: Currency::new("USD"),
            opening_balance: 50,
        })?;
        ledger.add_transfer(NewTransfer {
            date: date(2026, 11, 22)?,
            from_account_id: Some(cash),
            to_account_id: Some(dollars),
            amount: Some(7_500),
            to_amount: Some(80),
            comment: String::new(),
        })?;

        let FeedLine::Transfer { from, to, .. } = line_of(&ledger, OperationKind::Transfer) else {
            panic!("expected transfer");
        };
        assert_eq!(from.amount(), 7_500);
        assert_eq!(from.currency().as_str(), "RUB");
        assert_eq!(from.account_name(), "Наличные");
        assert_eq!(to.amount(), 80);
        assert_eq!(to.currency().as_str(), "USD");
        assert_eq!(to.account_name(), "Доллары");
        Ok(())
    }

    #[test]
    fn transfer_line_shows_equal_amounts_when_currency_matches() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let cash = rub_account(&mut ledger, "Наличные")?;
        let card = rub_account(&mut ledger, "Карта")?;
        ledger.add_transfer(NewTransfer {
            date: date(2026, 11, 22)?,
            from_account_id: Some(cash),
            to_account_id: Some(card),
            amount: Some(500),
            to_amount: Some(1),
            comment: String::new(),
        })?;

        let FeedLine::Transfer { from, to, .. } = line_of(&ledger, OperationKind::Transfer) else {
            panic!("expected transfer");
        };
        assert_eq!(from.amount(), 500);
        assert_eq!(to.amount(), 500);
        assert_eq!(to.account_name(), "Карта");
        Ok(())
    }

    #[test]
    fn debt_lines_show_copied_counterparty() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let account_id = rub_account(&mut ledger, "Наличные")?;
        let day = date(2026, 11, 22)?;
        ledger.add_lend(NewLend {
            date: day,
            account_id: Some(account_id),
            counterparty: Some("Петя".into()),
            amount: Some(5_000),
        })?;
        ledger.add_borrow(NewBorrow {
            date: day,
            account_id: Some(account_id),
            counterparty: Some("Маша".into()),
            amount: Some(3_000),
        })?;

        let FeedLine::Lend {
            money,
            counterparty,
            ..
        } = line_of(&ledger, OperationKind::Lend)
        else {
            panic!("expected lend");
        };
        assert_eq!(money.amount(), 5_000);
        assert_eq!(money.account_name(), "Наличные");
        assert_eq!(counterparty, "Петя");

        let FeedLine::Borrow {
            money,
            counterparty,
            ..
        } = line_of(&ledger, OperationKind::Borrow)
        else {
            panic!("expected borrow");
        };
        assert_eq!(money.amount(), 3_000);
        assert_eq!(counterparty, "Маша");
        Ok(())
    }

    #[test]
    fn repayment_line_shows_counterparty_and_direction() -> Result<(), DomainError> {
        let mut ledger = Ledger::new();
        let account_id = rub_account(&mut ledger, "Наличные")?;
        let day = date(2026, 11, 22)?;
        ledger.add_lend(NewLend {
            date: day,
            account_id: Some(account_id),
            counterparty: Some("Петя".into()),
            amount: Some(5_000),
        })?;
        ledger.add_repayment(NewRepayment {
            date: day,
            account_id: Some(account_id),
            counterparty: Some("Петя".into()),
            direction: Some(RepaymentDirection::Received),
            amount: Some(1_000),
        })?;

        let FeedLine::Repayment {
            money,
            counterparty,
            direction,
            ..
        } = line_of(&ledger, OperationKind::Repayment)
        else {
            panic!("expected repayment");
        };
        assert_eq!(money.amount(), 1_000);
        assert_eq!(money.currency().as_str(), "RUB");
        assert_eq!(counterparty, "Петя");
        assert_eq!(direction, RepaymentDirection::Received);
        Ok(())
    }
}
