// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Canonical `ObjectRef` paths for stable UI, MCP, and xref addressing.
//!
//! Wire form: `d:<diagram_id>/<category...>/<object_id>` (e.g. `d:flow/flow/node/n:a`).
//! Category segments are free-form so unknown kinds remain parseable; existence is checked later
//! by [`Session::object_ref_exists`](crate::model::Session::object_ref_exists).

use std::fmt;
use std::str::FromStr;

use super::ids::{DiagramId, IdError, ObjectId};

/// Category path segments under a diagram (`seq/message`, `flow/node`, …).
///
/// At least one non-empty segment is required. Unknown categories are intentionally representable
/// so protocol clients can pass forward-compatible refs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CategoryPath {
    segments: Vec<String>,
}

impl CategoryPath {
    /// Build from ordered segments; rejects empty path or empty segment strings.
    pub fn new(segments: Vec<String>) -> Result<Self, CategoryPathError> {
        if segments.is_empty() {
            return Err(CategoryPathError::Empty);
        }
        if segments.iter().any(|s| s.is_empty()) {
            return Err(CategoryPathError::EmptySegment);
        }
        Ok(Self { segments })
    }

    /// Ordered category segments as stored (not joined).
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

/// Invalid [`CategoryPath`] construction (empty path or empty segment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryPathError {
    /// No segments provided.
    Empty,
    /// At least one segment was the empty string.
    EmptySegment,
}

impl fmt::Display for CategoryPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("category path must not be empty"),
            Self::EmptySegment => f.write_str("category path must not contain empty segments"),
        }
    }
}

impl std::error::Error for CategoryPathError {}

/// Stable cross-layer object address: diagram + category path + object id.
///
/// Canonical wire form: `d:<diagram_id>/<category...>/<object_id>`. Used by TUI selection, MCP
/// tools, walkthrough evidence, and xrefs. Does not itself prove the object still exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectRef {
    diagram_id: DiagramId,
    category: CategoryPath,
    object_id: ObjectId,
}

impl ObjectRef {
    /// Assemble a ref from already-validated id pieces (no existence check).
    pub fn new(diagram_id: DiagramId, category: CategoryPath, object_id: ObjectId) -> Self {
        Self { diagram_id, category, object_id }
    }

    /// Diagram that owns the referenced object.
    pub fn diagram_id(&self) -> &DiagramId {
        &self.diagram_id
    }

    /// Category path within the diagram (`seq/message`, `gantt/lane`, …).
    pub fn category(&self) -> &CategoryPath {
        &self.category
    }

    /// Stable object id (node, edge, message, …).
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Parse the canonical `d:…/…/…` wire form.
    ///
    /// Category is everything between the diagram id and the final segment (object id).
    pub fn parse(input: &str) -> Result<Self, ParseObjectRefError> {
        const PREFIX: &str = "d:";
        let rest = input.strip_prefix(PREFIX).ok_or(ParseObjectRefError::MissingPrefix)?;

        let (diagram_id_str, remainder) =
            rest.split_once('/').ok_or(ParseObjectRefError::MissingCategory)?;

        if diagram_id_str.is_empty() {
            return Err(ParseObjectRefError::MissingDiagramId);
        }
        let diagram_id = DiagramId::new(diagram_id_str.to_owned())
            .map_err(ParseObjectRefError::InvalidDiagramId)?;

        if remainder.is_empty() {
            return Err(ParseObjectRefError::MissingCategory);
        }

        let (category_str, object_id_str) =
            remainder.rsplit_once('/').ok_or(ParseObjectRefError::MissingObjectId)?;

        if category_str.is_empty() {
            return Err(ParseObjectRefError::MissingCategory);
        }
        if object_id_str.is_empty() {
            return Err(ParseObjectRefError::MissingObjectId);
        }

        let category_segments = category_str.split('/').map(|s| s.to_owned()).collect::<Vec<_>>();
        let category =
            CategoryPath::new(category_segments).map_err(ParseObjectRefError::InvalidCategory)?;

        let object_id = ObjectId::new(object_id_str.to_owned())
            .map_err(ParseObjectRefError::InvalidObjectId)?;

        Ok(Self { diagram_id, category, object_id })
    }
}

impl fmt::Display for ObjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("d:")?;
        write!(f, "{}", self.diagram_id)?;
        for seg in self.category.segments() {
            write!(f, "/{}", seg)?;
        }
        write!(f, "/{}", self.object_id)
    }
}

impl FromStr for ObjectRef {
    type Err = ParseObjectRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Failure parsing an [`ObjectRef`] wire string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseObjectRefError {
    /// Input did not start with `d:`.
    MissingPrefix,
    /// Empty diagram id after the prefix.
    MissingDiagramId,
    /// Missing or empty category path between diagram and object id.
    MissingCategory,
    /// Missing final object-id segment.
    MissingObjectId,
    /// Diagram id failed segment validation.
    InvalidDiagramId(IdError),
    /// Category path failed validation.
    InvalidCategory(CategoryPathError),
    /// Object id failed segment validation.
    InvalidObjectId(IdError),
}

impl fmt::Display for ParseObjectRefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPrefix => f.write_str("object ref must start with 'd:'"),
            Self::MissingDiagramId => f.write_str("object ref is missing diagram id"),
            Self::MissingCategory => f.write_str("object ref is missing category path"),
            Self::MissingObjectId => f.write_str("object ref is missing object id"),
            Self::InvalidDiagramId(err) => write!(f, "invalid diagram id: {err}"),
            Self::InvalidCategory(err) => write!(f, "invalid category path: {err}"),
            Self::InvalidObjectId(err) => write!(f, "invalid object id: {err}"),
        }
    }
}

impl std::error::Error for ParseObjectRefError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidDiagramId(err) => Some(err),
            Self::InvalidCategory(err) => Some(err),
            Self::InvalidObjectId(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjectRef, ParseObjectRefError};

    #[test]
    fn parses_and_formats_protocol_examples() {
        let cases = [
            "d:7b1d0000-0000-0000-0000-000000000000/seq/participant/p:alice",
            "d:7b1d0000-0000-0000-0000-000000000000/seq/message/m:0042",
            "d:91aa0000-0000-0000-0000-000000000000/flow/node/n:authorize",
            "d:91aa0000-0000-0000-0000-000000000000/flow/edge/e:13",
        ];

        for s in cases {
            let parsed: ObjectRef = s.parse().expect("parse");
            assert_eq!(parsed.to_string(), s);
            let reparsed: ObjectRef = parsed.to_string().parse().expect("reparse");
            assert_eq!(reparsed, parsed);
        }
    }

    #[test]
    fn accepts_unknown_categories_with_multiple_segments() {
        let s = "d:diag/custom/category/kind/o:1";
        let parsed: ObjectRef = s.parse().expect("parse");
        assert_eq!(parsed.to_string(), s);
        assert_eq!(
            parsed.category().segments(),
            &["custom".to_owned(), "category".to_owned(), "kind".to_owned()]
        );
        assert_eq!(parsed.object_id().as_str(), "o:1");
        assert_eq!(parsed.diagram_id().as_str(), "diag");
    }

    #[test]
    fn rejects_missing_prefix() {
        let err = "x:diag/seq/participant/p:alice".parse::<ObjectRef>().unwrap_err();
        assert_eq!(err, ParseObjectRefError::MissingPrefix);
    }

    #[test]
    fn rejects_missing_diagram_id() {
        let err = "d:/seq/participant/p:alice".parse::<ObjectRef>().unwrap_err();
        assert_eq!(err, ParseObjectRefError::MissingDiagramId);
    }

    #[test]
    fn rejects_missing_category() {
        let err = "d:diag/seq".parse::<ObjectRef>().unwrap_err();
        assert_eq!(err, ParseObjectRefError::MissingObjectId);

        let err = "d:diag/".parse::<ObjectRef>().unwrap_err();
        assert_eq!(err, ParseObjectRefError::MissingCategory);
    }

    #[test]
    fn rejects_missing_object_id() {
        let err = "d:diag/seq/participant/".parse::<ObjectRef>().unwrap_err();
        assert_eq!(err, ParseObjectRefError::MissingObjectId);
    }

    #[test]
    fn rejects_empty_category_segments() {
        let err = "d:diag/seq//p:alice".parse::<ObjectRef>().unwrap_err();
        assert!(matches!(err, ParseObjectRefError::InvalidCategory(_)));
    }
}
