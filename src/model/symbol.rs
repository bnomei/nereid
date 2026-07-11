// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Frigg `stable_symbol_id` anchors on participants and flow nodes.
//!
//! Sidecar-persisted metadata (not Mermaid source) linking diagram objects to code symbols.
//! Format: `sym-` + non-empty lowercase hex. Optional `repository_id` scopes multi-repo sessions.

use std::fmt;

/// Code-symbol link attached to a participant or flow node (sidecar / UI only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolAnchor {
    stable_symbol_id: String,
    repository_id: Option<String>,
}

impl SymbolAnchor {
    /// Validate `stable_symbol_id` (`sym-` + lowercase hex) and retain optional repository scope.
    pub fn new(
        stable_symbol_id: impl Into<String>,
        repository_id: Option<String>,
    ) -> Result<Self, SymbolAnchorError> {
        let stable_symbol_id = stable_symbol_id.into();
        validate_stable_symbol_id(&stable_symbol_id)?;
        Ok(Self { stable_symbol_id, repository_id })
    }

    /// Frigg stable symbol id (`sym-<hex>`).
    pub fn stable_symbol_id(&self) -> &str {
        &self.stable_symbol_id
    }

    /// Optional repository scope when the session spans multiple Frigg repos.
    pub fn repository_id(&self) -> Option<&str> {
        self.repository_id.as_deref()
    }
}

/// Invalid [`SymbolAnchor`] construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolAnchorError {
    /// Id was empty, missing `sym-`, or used non-lowercase-hex digits.
    InvalidStableSymbolId { stable_symbol_id: String },
}

impl fmt::Display for SymbolAnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableSymbolId { stable_symbol_id } => {
                write!(f, "invalid stable_symbol_id '{stable_symbol_id}'")
            }
        }
    }
}

impl std::error::Error for SymbolAnchorError {}

fn validate_stable_symbol_id(stable_symbol_id: &str) -> Result<(), SymbolAnchorError> {
    let Some(hex) = stable_symbol_id.strip_prefix("sym-") else {
        return Err(SymbolAnchorError::InvalidStableSymbolId {
            stable_symbol_id: stable_symbol_id.to_owned(),
        });
    };

    if hex.is_empty()
        || !hex.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(SymbolAnchorError::InvalidStableSymbolId {
            stable_symbol_id: stable_symbol_id.to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SymbolAnchor;

    #[test]
    fn symbol_anchor_accepts_lowercase_hex_symbol_ids() {
        let anchor = SymbolAnchor::new("sym-16c57df0026ced40", Some("repo".to_owned()))
            .expect("valid symbol anchor");

        assert_eq!(anchor.stable_symbol_id(), "sym-16c57df0026ced40");
        assert_eq!(anchor.repository_id(), Some("repo"));
    }

    #[test]
    fn symbol_anchor_rejects_invalid_symbol_ids() {
        for value in ["", "abc", "sym-", "sym-XYZ", "sym-123G"] {
            assert!(SymbolAnchor::new(value, None).is_err(), "{value} should be invalid");
        }
    }
}
