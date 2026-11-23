use serde::{Deserialize, Serialize};

use crate::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Applicability {
    Expense,
    Income,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub(crate) fn set_parent(&mut self, parent_id: CategoryId) {
        self.parent_id = Some(parent_id);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<CategoryId>,
    pub applicability: Option<Applicability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub applicability: Option<Applicability>,
}
