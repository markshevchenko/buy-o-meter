use crate::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CategoryId(u64);

impl CategoryId {
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    Expense,
    Income,
    Both,
}

/// Вид операции, для которого выбирают категорию: расход или доход.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Expense,
    Income,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    id: CategoryId,
    name: String,
    applicability: Applicability,
    parent_id: Option<CategoryId>,
}

impl Category {
    pub(crate) fn new(id: CategoryId, input: CreateCategory) -> Result<Self, DomainError> {
        let Some(applicability) = input.applicability else {
            return Err(DomainError::ApplicabilityRequired);
        };
        Ok(Self {
            id,
            name: input.name,
            applicability,
            parent_id: input.parent_id,
        })
    }

    #[must_use]
    pub fn id(&self) -> CategoryId {
        self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn applicability(&self) -> Applicability {
        self.applicability
    }

    #[must_use]
    pub fn parent_id(&self) -> Option<CategoryId> {
        self.parent_id
    }

    pub(crate) fn update(&mut self, input: UpdateCategory) {
        if let Some(name) = input.name {
            self.name = name;
        }
        if let Some(applicability) = input.applicability {
            self.applicability = applicability;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<CategoryId>,
    /// `None` у подкатегории — признак родителя. Для верхнего уровня обязателен.
    pub applicability: Option<Applicability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub applicability: Option<Applicability>,
}

/// Сумма, отнесённая к категории. Имя категории в сумму не копируется.
/// Полная операция (дата, счёт, комментарий) появится в историях ввода.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryAmount {
    pub category_id: CategoryId,
    pub kind: EntryKind,
    pub amount: i64,
}
