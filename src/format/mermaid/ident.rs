// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Validation rules for Mermaid node and participant identifiers.
//!
//! Allowed: non-empty ASCII alphanumeric + `_`, no whitespace or `/` (slash would break
//! `ObjectRef` composition when ids are embedded in paths).

use std::fmt;

/// Why a Mermaid identifier was rejected during parse or export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MermaidIdentError {
    /// Empty token.
    Empty,
    /// Contained whitespace.
    ContainsWhitespace,
    /// Contained `/` (unsafe for path-like ids).
    ContainsSlash,
    /// Character outside `[A-Za-z0-9_]`.
    InvalidChar { ch: char },
}

impl fmt::Display for MermaidIdentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("must not be empty"),
            Self::ContainsWhitespace => f.write_str("must not contain whitespace"),
            Self::ContainsSlash => f.write_str("must not contain '/'"),
            Self::InvalidChar { ch } => write!(f, "contains invalid character: '{ch}'"),
        }
    }
}

pub(super) fn validate_mermaid_ident(ident: &str) -> Result<(), MermaidIdentError> {
    if ident.is_empty() {
        return Err(MermaidIdentError::Empty);
    }
    if ident.chars().any(|c| c.is_whitespace()) {
        return Err(MermaidIdentError::ContainsWhitespace);
    }
    if ident.contains('/') {
        return Err(MermaidIdentError::ContainsSlash);
    }
    if let Some(ch) = ident.chars().find(|c| !c.is_ascii_alphanumeric() && *c != '_') {
        return Err(MermaidIdentError::InvalidChar { ch });
    }
    Ok(())
}
