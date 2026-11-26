use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::Ledger;
use super::edit::FeedItem;
use crate::account::Account;
use crate::category::Category;
use crate::counterparty::Counterparty;
use crate::operation::{
    Borrow, Expense, Income, Lend, OperationDraft, OperationId, Record, Repayment, Transfer,
};

#[derive(Debug)]
pub(crate) struct StoreError;

pub(crate) struct LedgerFile {
    pub(crate) name: &'static str,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct AccountsFile {
    next_account_id: u64,
    accounts: Vec<Account>,
}

#[derive(Serialize, Deserialize)]
struct CategoriesFile {
    next_category_id: u64,
    starter_seeded: bool,
    categories: Vec<Category>,
}

#[derive(Serialize, Deserialize)]
struct OperationsFile {
    next_operation_id: u64,
    operations: Vec<OperationDraft>,
    operation_links: Vec<Option<OperationId>>,
    feed: Vec<FeedItem>,
    records: Vec<Record>,
    expenses: Vec<Expense>,
    incomes: Vec<Income>,
    transfers: Vec<Transfer>,
    lends: Vec<Lend>,
    borrows: Vec<Borrow>,
    repayments: Vec<Repayment>,
}

#[derive(Serialize, Deserialize)]
struct CounterpartiesFile {
    next_counterparty_id: u64,
    counterparties: Vec<Counterparty>,
}

pub(crate) fn encode_ledger(ledger: &Ledger) -> Result<Vec<LedgerFile>, StoreError> {
    Ok(vec![
        to_file("accounts.json", &accounts_file(ledger))?,
        to_file("categories.json", &categories_file(ledger))?,
        to_file("operations.json", &operations_file(ledger))?,
        to_file("counterparties.json", &counterparties_file(ledger))?,
    ])
}

pub(crate) fn decode_ledger(files: &[(&str, &[u8])]) -> Result<Ledger, StoreError> {
    let accounts = parse(files, "accounts.json")?;
    let categories = parse(files, "categories.json")?;
    let operations = parse(files, "operations.json")?;
    let counterparties = parse(files, "counterparties.json")?;
    Ok(assemble(accounts, categories, operations, counterparties))
}

fn accounts_file(ledger: &Ledger) -> AccountsFile {
    AccountsFile {
        next_account_id: ledger.next_account_id,
        accounts: ledger.accounts.clone(),
    }
}

fn categories_file(ledger: &Ledger) -> CategoriesFile {
    CategoriesFile {
        next_category_id: ledger.next_category_id,
        starter_seeded: ledger.starter_seeded,
        categories: ledger.categories.clone(),
    }
}

fn operations_file(ledger: &Ledger) -> OperationsFile {
    OperationsFile {
        next_operation_id: ledger.next_operation_id,
        operations: ledger.operations.clone(),
        operation_links: ledger.operation_links.clone(),
        feed: ledger.feed.clone(),
        records: ledger.records.clone(),
        expenses: ledger.expenses.clone(),
        incomes: ledger.incomes.clone(),
        transfers: ledger.transfers.clone(),
        lends: ledger.lends.clone(),
        borrows: ledger.borrows.clone(),
        repayments: ledger.repayments.clone(),
    }
}

fn counterparties_file(ledger: &Ledger) -> CounterpartiesFile {
    CounterpartiesFile {
        next_counterparty_id: ledger.next_counterparty_id,
        counterparties: ledger.counterparties.clone(),
    }
}

fn to_file<T: Serialize>(name: &'static str, value: &T) -> Result<LedgerFile, StoreError> {
    let bytes = serde_json::to_vec(value).map_err(|_| StoreError)?;
    Ok(LedgerFile { name, bytes })
}

fn parse<T: DeserializeOwned>(files: &[(&str, &[u8])], name: &str) -> Result<T, StoreError> {
    let bytes = files
        .iter()
        .find(|(file_name, _)| *file_name == name)
        .map(|(_, bytes)| *bytes)
        .ok_or(StoreError)?;
    serde_json::from_slice(bytes).map_err(|_| StoreError)
}

fn assemble(
    accounts: AccountsFile,
    categories: CategoriesFile,
    operations: OperationsFile,
    counterparties: CounterpartiesFile,
) -> Ledger {
    Ledger {
        next_account_id: accounts.next_account_id,
        accounts: accounts.accounts,
        operations: operations.operations,
        operation_links: operations.operation_links,
        next_operation_id: operations.next_operation_id,
        feed: operations.feed,
        next_category_id: categories.next_category_id,
        categories: categories.categories,
        starter_seeded: categories.starter_seeded,
        records: operations.records,
        expenses: operations.expenses,
        incomes: operations.incomes,
        transfers: operations.transfers,
        lends: operations.lends,
        borrows: operations.borrows,
        repayments: operations.repayments,
        next_counterparty_id: counterparties.next_counterparty_id,
        counterparties: counterparties.counterparties,
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_ledger, encode_ledger};
    use crate::account::{AccountId, CreateAccount};
    use crate::category::Applicability;
    use crate::clock::date;
    use crate::currency::Currency;
    use crate::ledger::Ledger;
    use crate::operation::{NewExpense, NewLend, OperationForm, OperationKind, Record};

    #[test]
    fn ledger_roundtrip_keeps_private_state() {
        let ledger = filled_ledger();
        let account = ledger.accounts()[0].id();
        let encoded = encode_ledger(&ledger).expect("кодирование");
        let views: Vec<_> = encoded
            .iter()
            .map(|file| (file.name, file.bytes.as_slice()))
            .collect();
        let loaded = decode_ledger(&views).expect("декодирование");
        assert_loaded(&ledger, &loaded, account);
    }

    fn filled_ledger() -> Ledger {
        let mut ledger = Ledger::new();
        ledger.seed_starter_categories();
        let account = ledger
            .create_account(CreateAccount {
                name: "Карта".into(),
                currency: Currency::new("RUB"),
                opening_balance: 5_000,
            credit_limit: 0,
            note: String::new(),
            })
            .expect("счёт");
        let food = ledger
            .categories()
            .iter()
            .find(|category| category.name() == "Продукты")
            .expect("категория")
            .id();
        let day = date(1969, 12, 31).expect("дата");
        ledger
            .add_expense(NewExpense {
                date: day,
                account_id: Some(account),
                category_id: Some(food),
                amount: Some(350),
                comment: "Такси".into(),
            })
            .expect("расход");
        ledger
            .add_lend(NewLend {
                date: day,
                account_id: Some(account),
                counterparty: Some("Анна".into()),
                amount: Some(80),
            })
            .expect("долг");
        ledger
            .add_record(Record::Expense {
                category_id: food,
                amount: 15,
            })
            .expect("проводка");
        ledger
    }

    fn assert_loaded(ledger: &Ledger, loaded: &Ledger, account: AccountId) {
        let day = date(1969, 12, 31).expect("дата");
        assert_eq!(loaded.accounts().len(), 1);
        assert_eq!(loaded.accounts()[0].id(), account);
        assert_eq!(loaded.accounts()[0].name(), "Карта");
        assert_eq!(loaded.accounts()[0].opening_balance(), 5_000);
        assert_eq!(loaded.categories().len(), ledger.categories().len());
        assert!(loaded.categories().iter().any(|category| {
            category.name() == "Продукты" && category.applicability() == Applicability::Expense
        }));
        let mut again = loaded.clone();
        again.seed_starter_categories();
        assert_eq!(again.categories().len(), loaded.categories().len());
        assert_eq!(loaded.operations().len(), 2);
        let expense = loaded
            .operations()
            .into_iter()
            .find(|operation| operation.kind() == OperationKind::Expense)
            .expect("операция");
        let OperationForm::Expense(form) = loaded.operation_form(expense.id()).expect("форма")
        else {
            panic!("ожидался расход");
        };
        assert_eq!(form.comment, "Такси");
        assert_eq!(form.date, day);
        assert_eq!(form.account_id, Some(account));
        assert!(
            loaded
                .counterparties()
                .iter()
                .any(|item| item.name() == "Анна")
        );
        assert_eq!(loaded.records().len(), 1);
        assert_eq!(
            loaded.balance(account).expect("остаток"),
            ledger.balance(account).expect("остаток")
        );
    }
}
