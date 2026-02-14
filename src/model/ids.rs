// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

/// A stable identifier used across the model and protocol surfaces.
///
/// This is intentionally std-only and does not enforce a UUID format; it only
/// enforces that the id is a non-empty *path segment* (i.e. contains no `/`),
/// because IDs appear inside canonical `ObjectRef`s like `d:<diagram_id>/...`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id<T> {
    value: String,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        validate_id_segment(&value)?;
        Ok(Self {
            value,
            _marker: PhantomData,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn into_string(self) -> String {
        self.value
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

impl<T> AsRef<str> for Id<T> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<T> Borrow<str> for Id<T> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<T> FromStr for Id<T> {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_owned())
    }
}

impl<T> TryFrom<String> for Id<T> {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    Empty,
    ContainsSlash,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("id must not be empty"),
            Self::ContainsSlash => f.write_str("id must not contain '/'"),
        }
    }
}

impl std::error::Error for IdError {}

fn validate_id_segment(value: &str) -> Result<(), IdError> {
    if value.is_empty() {
        return Err(IdError::Empty);
    }
    if value.contains('/') {
        return Err(IdError::ContainsSlash);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SessionIdTag {}
pub type SessionId = Id<SessionIdTag>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagramIdTag {}
pub type DiagramId = Id<DiagramIdTag>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WalkthroughIdTag {}
pub type WalkthroughId = Id<WalkthroughIdTag>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WalkthroughNodeIdTag {}
pub type WalkthroughNodeId = Id<WalkthroughNodeIdTag>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum XRefIdTag {}
pub type XRefId = Id<XRefIdTag>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectIdTag {}
pub type ObjectId = Id<ObjectIdTag>;

#[cfg(test)]
mod tests {
    use super::{Id, IdError};

    #[test]
    fn id_rejects_empty() {
        let result: Result<Id<()>, _> = Id::new("");
        assert_eq!(result, Err(IdError::Empty));
    }

    #[test]
    fn id_rejects_slash() {
        let result: Result<Id<()>, _> = Id::new("a/b");
        assert_eq!(result, Err(IdError::ContainsSlash));
    }
}
