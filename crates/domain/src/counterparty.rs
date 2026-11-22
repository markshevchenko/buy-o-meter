use crate::currency::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CounterpartyId(u64);

impl CounterpartyId {
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
pub struct Counterparty {
    id: CounterpartyId,
    name: String,
}

impl Counterparty {
    pub(crate) fn new(id: CounterpartyId, name: String) -> Self {
        Self { id, name }
    }

    #[must_use]
    pub fn id(&self) -> CounterpartyId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Debt {
    counterparty_name: String,
    currency: Currency,
    amount: i64,
}

impl Debt {
    pub(crate) fn new(counterparty_name: String, currency: Currency, amount: i64) -> Self {
        Self {
            counterparty_name,
            currency,
            amount,
        }
    }

    #[must_use]
    pub fn counterparty_name(&self) -> &str {
        &self.counterparty_name
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
