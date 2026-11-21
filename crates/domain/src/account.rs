use crate::currency::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountId(u64);

impl AccountId {
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    name: String,
    currency: Currency,
    opening_balance: i64,
}

impl Account {
    pub(crate) fn new(id: AccountId, input: CreateAccount) -> Self {
        Self {
            id,
            name: input.name,
            currency: input.currency,
            opening_balance: input.opening_balance,
        }
    }

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
    pub fn opening_balance(&self) -> i64 {
        self.opening_balance
    }

    pub(crate) fn update(&mut self, input: UpdateAccount) {
        if let Some(name) = input.name {
            self.name = name;
        }
        if let Some(opening_balance) = input.opening_balance {
            self.opening_balance = opening_balance;
        }
        if let Some(currency) = input.currency {
            self.currency = currency;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAccount {
    pub name: String,
    pub currency: Currency,
    pub opening_balance: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAccount {
    pub name: Option<String>,
    pub opening_balance: Option<i64>,
    pub currency: Option<Currency>,
}
