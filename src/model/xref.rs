// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Cross-diagram links between `ObjectRef` endpoints.
//!
//! An xref is a directed edge in session space (not inside a single diagram AST). Status tracks
//! dangling endpoints after deletes/replaces so agents can treat broken links as TODOs.

use std::fmt;
use std::str::FromStr;

use super::object_ref::ObjectRef;

/// Directed link between two [`ObjectRef`] endpoints with free-form kind and optional label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XRef {
    from: ObjectRef,
    to: ObjectRef,
    kind: String,
    label: Option<String>,
    status: XRefStatus,
}

impl XRef {
    /// Build with empty label; status is usually recomputed via [`XRefStatus::from_flags`].
    pub fn new(
        from: ObjectRef,
        to: ObjectRef,
        kind: impl Into<String>,
        status: XRefStatus,
    ) -> Self {
        Self { from, to, kind: kind.into(), label: None, status }
    }

    /// Source endpoint.
    pub fn from(&self) -> &ObjectRef {
        &self.from
    }

    /// Target endpoint.
    pub fn to(&self) -> &ObjectRef {
        &self.to
    }

    /// Free-form relation kind (protocol does not enumerate values).
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Optional human-readable label.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Current endpoint liveness status.
    pub fn status(&self) -> XRefStatus {
        self.status
    }

    /// Set or clear the display label.
    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label;
    }

    /// Update dangling status after session mutation or revalidation.
    pub fn set_status(&mut self, status: XRefStatus) {
        self.status = status;
    }
}

/// Endpoint resolution status for an [`XRef`] (`ok` / `dangling_*` wire strings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XRefStatus {
    /// Both endpoints resolve via [`Session::object_ref_exists`](crate::model::Session::object_ref_exists).
    Ok,
    /// Only the `from` endpoint is missing.
    DanglingFrom,
    /// Only the `to` endpoint is missing.
    DanglingTo,
    /// Both endpoints are missing.
    DanglingBoth,
}

impl XRefStatus {
    /// Map independent endpoint-missing flags to a status variant.
    pub fn from_flags(from_dangling: bool, to_dangling: bool) -> Self {
        match (from_dangling, to_dangling) {
            (false, false) => Self::Ok,
            (true, false) => Self::DanglingFrom,
            (false, true) => Self::DanglingTo,
            (true, true) => Self::DanglingBoth,
        }
    }

    /// True when either endpoint is missing.
    pub fn is_dangling(self) -> bool {
        self != Self::Ok
    }

    /// Protocol wire string (`ok`, `dangling_from`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::DanglingFrom => "dangling_from",
            Self::DanglingTo => "dangling_to",
            Self::DanglingBoth => "dangling_both",
        }
    }
}

impl fmt::Display for XRefStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Unknown [`XRefStatus`] wire token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseXRefStatusError;

impl fmt::Display for ParseXRefStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid xref status")
    }
}

impl std::error::Error for ParseXRefStatusError {}

impl FromStr for XRefStatus {
    type Err = ParseXRefStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ok" => Ok(Self::Ok),
            "dangling_from" => Ok(Self::DanglingFrom),
            "dangling_to" => Ok(Self::DanglingTo),
            "dangling_both" => Ok(Self::DanglingBoth),
            _ => Err(ParseXRefStatusError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::XRefStatus;

    #[test]
    fn xref_status_roundtrips_via_str() {
        let cases = [
            XRefStatus::Ok,
            XRefStatus::DanglingFrom,
            XRefStatus::DanglingTo,
            XRefStatus::DanglingBoth,
        ];

        for status in cases {
            let s = status.as_str();
            let parsed: XRefStatus = s.parse().expect("parse");
            assert_eq!(parsed, status);
            assert_eq!(parsed.to_string(), s);
        }
    }
}
