// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::fmt;
use std::str::FromStr;

use super::object_ref::ObjectRef;

/// Cross-diagram link between two [`ObjectRef`]s (see `docs/protocol-01.md` §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XRef {
    from: ObjectRef,
    to: ObjectRef,
    kind: String,
    label: Option<String>,
    status: XRefStatus,
}

impl XRef {
    pub fn new(
        from: ObjectRef,
        to: ObjectRef,
        kind: impl Into<String>,
        status: XRefStatus,
    ) -> Self {
        Self {
            from,
            to,
            kind: kind.into(),
            label: None,
            status,
        }
    }

    pub fn from(&self) -> &ObjectRef {
        &self.from
    }

    pub fn to(&self) -> &ObjectRef {
        &self.to
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn status(&self) -> XRefStatus {
        self.status
    }

    pub fn set_label(&mut self, label: Option<String>) {
        self.label = label;
    }

    pub fn set_status(&mut self, status: XRefStatus) {
        self.status = status;
    }
}

/// XRef endpoint resolution status (see `docs/protocol-01.md` §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XRefStatus {
    Ok,
    DanglingFrom,
    DanglingTo,
    DanglingBoth,
}

impl XRefStatus {
    pub fn from_flags(from_dangling: bool, to_dangling: bool) -> Self {
        match (from_dangling, to_dangling) {
            (false, false) => Self::Ok,
            (true, false) => Self::DanglingFrom,
            (false, true) => Self::DanglingTo,
            (true, true) => Self::DanglingBoth,
        }
    }

    pub fn is_dangling(self) -> bool {
        self != Self::Ok
    }

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
