use crate::account::AccountId;
use crate::counterparty::Debt;
use crate::currency::Currency;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBalance {
    id: AccountId,
    name: String,
    currency: Currency,
    amount: i64,
}

impl AccountBalance {
    #[must_use]
    pub fn id(&self) -> AccountId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    #[must_use]
    pub fn amount(&self) -> i64 {
        self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickAccess {
    accounts: Vec<AccountBalance>,
    my_debts: Vec<Debt>,
    debts_to_me: Vec<Debt>,
}

impl QuickAccess {
    #[must_use]
    pub fn accounts(&self) -> &[AccountBalance] {
        &self.accounts
    }

    #[must_use]
    pub fn my_debts(&self) -> &[Debt] {
        &self.my_debts
    }

    #[must_use]
    pub fn debts_to_me(&self) -> &[Debt] {
        &self.debts_to_me
    }
}
