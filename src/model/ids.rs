// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Newtype string ids for sessions, diagrams, objects, walkthroughs, and xrefs.
//!
//! Construction validates non-empty path-segment strings (no `/`) so ids can appear inside
//! `ObjectRef`s, session folders, and JSON sidecars without further escaping. UUID shape is not
//! required; callers choose human-friendly or opaque stable ids.

use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

/// Phantom-tagged stable string id shared by protocol and store layers.
///
/// Validates only non-empty path segments (no `/`); does not enforce UUID format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id<T> {
    value: String,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Construct after validating a non-empty path segment (no `/`).
    pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        validate_id_segment(&value)?;
        Ok(Self { value, _marker: PhantomData })
    }

    /// Borrow the raw id string.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Consume into the owned string.
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

/// Invalid id segment construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdError {
    /// Empty string is not a path segment.
    Empty,
    /// Slash would break `ObjectRef` / path composition.
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

/// Phantom tag for [`SessionId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SessionIdTag {}
/// Session container identifier.
pub type SessionId = Id<SessionIdTag>;

/// Phantom tag for [`DiagramId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagramIdTag {}
/// Diagram identifier within a session.
pub type DiagramId = Id<DiagramIdTag>;

/// Phantom tag for [`WalkthroughId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WalkthroughIdTag {}
/// Walkthrough graph identifier within a session.
pub type WalkthroughId = Id<WalkthroughIdTag>;

/// Phantom tag for [`WalkthroughNodeId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WalkthroughNodeIdTag {}
/// Node identifier within a walkthrough graph.
pub type WalkthroughNodeId = Id<WalkthroughNodeIdTag>;

/// Phantom tag for [`XRefId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum XRefIdTag {}
/// Cross-reference link identifier within a session.
pub type XRefId = Id<XRefIdTag>;

/// Phantom tag for [`ObjectId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectIdTag {}
/// Diagram object identifier (node, edge, message, participant, etc.).
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
