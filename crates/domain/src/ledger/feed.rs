use std::time::SystemTime;

use super::Ledger;
use crate::account::AccountId;
use crate::feed::{FeedDay, FeedLine, FeedMoney, MainScreen};
use crate::operation::{Expense, OperationEntry, OperationId, OperationKind};

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
        match entry.kind() {
            OperationKind::Expense => self.expense_line(entry.id()),
            OperationKind::Income
            | OperationKind::Transfer
            | OperationKind::Lend
            | OperationKind::Borrow
            | OperationKind::Repayment => None,
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
    use crate::operation::{NewExpense, OperationDraft, OperationKind};
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
}
