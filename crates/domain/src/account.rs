use crate::currency::Currency;

/// Идентификатор счёта.
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

/// Счёт: место, где лежат деньги. Типа счёта отдельным полем нет — он в названии.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub currency: Currency,
    pub opening_balance: i64,
}

/// Данные для создания счёта.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAccount {
    pub name: String,
    pub currency: Currency,
    pub opening_balance: i64,
}

/// Данные для изменения счёта. `None` — поле не менять.
/// Валюту можно менять, только если по счёту ещё нет операций.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAccount {
    pub name: Option<String>,
    pub opening_balance: Option<i64>,
    pub currency: Option<Currency>,
}
