// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Entity-relationship diagram AST: entities and cardinality relationships.
//!
//! Stable ids key entities (`er/entity`) and relationships (`er/relationship`). Attribute boxes
//! are out of scope for this limited Mermaid subset; notes are sidecar-only.

use std::collections::BTreeMap;

use super::ids::ObjectId;

/// ER diagram content keyed by stable object IDs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ErAst {
    entities: BTreeMap<ObjectId, ErEntity>,
    relationships: BTreeMap<ObjectId, ErRelationship>,
}

impl ErAst {
    /// Entities keyed by stable id.
    pub fn entities(&self) -> &BTreeMap<ObjectId, ErEntity> {
        &self.entities
    }

    /// Mutable entity map.
    pub fn entities_mut(&mut self) -> &mut BTreeMap<ObjectId, ErEntity> {
        &mut self.entities
    }

    /// Relationships keyed by stable id.
    pub fn relationships(&self) -> &BTreeMap<ObjectId, ErRelationship> {
        &self.relationships
    }

    /// Mutable relationship map.
    pub fn relationships_mut(&mut self) -> &mut BTreeMap<ObjectId, ErRelationship> {
        &mut self.relationships
    }
}

/// Named entity box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErEntity {
    name: String,
    /// Top-level note (sidecar / UI); not part of Mermaid export.
    note: Option<String>,
}

impl ErEntity {
    /// Entity with name only.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), note: None }
    }

    /// Sidecar/UI note; not part of Mermaid export.
    pub fn set_note<T: Into<String>>(&mut self, note: Option<T>) {
        self.note = note.map(Into::into);
    }

    /// Entity display name (Mermaid identity).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Rename the entity.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Sidecar/UI note text.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::ErEntity;

    #[test]
    fn er_entity_note_can_be_set_and_cleared() {
        let mut entity = ErEntity::new("CUSTOMER");
        assert_eq!(entity.note(), None);

        entity.set_note(Some("billing party"));
        assert_eq!(entity.note(), Some("billing party"));

        entity.set_note::<&str>(None);
        assert_eq!(entity.note(), None);
    }
}

/// Cardinality at one end of a relationship (Mermaid 2-char tokens fold into these).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErCardinality {
    /// `||`
    ExactlyOne,
    /// `|o` / `o|`
    ZeroOrOne,
    /// `|{` / `}|`
    OneOrMore,
    /// `}o` / `o{`
    ZeroOrMore,
}

/// Identifying (solid `--`) vs non-identifying (dashed `..`) relationship stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ErStroke {
    #[default]
    Identifying,
    NonIdentifying,
}

/// Directed relationship between two entities with endpoint cardinalities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErRelationship {
    from_entity_id: ObjectId,
    to_entity_id: ObjectId,
    from_card: ErCardinality,
    to_card: ErCardinality,
    stroke: ErStroke,
    label: Option<String>,
    /// Original Mermaid connector body for lossless export.
    raw_connector: Option<String>,
}

impl ErRelationship {
    /// Identifying solid stroke, no label/raw connector.
    pub fn new(
        from_entity_id: ObjectId,
        to_entity_id: ObjectId,
        from_card: ErCardinality,
        to_card: ErCardinality,
    ) -> Self {
        Self {
            from_entity_id,
            to_entity_id,
            from_card,
            to_card,
            stroke: ErStroke::Identifying,
            label: None,
            raw_connector: None,
        }
    }

    /// Builder: identifying vs non-identifying stroke.
    pub fn with_stroke(mut self, stroke: ErStroke) -> Self {
        self.stroke = stroke;
        self
    }

    /// Builder: optional relationship label.
    pub fn with_label(mut self, label: Option<impl Into<String>>) -> Self {
        self.label = label.map(Into::into);
        self
    }

    /// Builder: preserve original Mermaid connector body for export.
    pub fn with_raw_connector(mut self, raw: Option<impl Into<String>>) -> Self {
        self.raw_connector = raw.map(Into::into);
        self
    }

    /// Source entity id.
    pub fn from_entity_id(&self) -> &ObjectId {
        &self.from_entity_id
    }

    /// Target entity id.
    pub fn to_entity_id(&self) -> &ObjectId {
        &self.to_entity_id
    }

    /// Cardinality at the source end.
    pub fn from_card(&self) -> ErCardinality {
        self.from_card
    }

    /// Cardinality at the target end.
    pub fn to_card(&self) -> ErCardinality {
        self.to_card
    }

    /// Identifying (solid) vs non-identifying (dashed).
    pub fn stroke(&self) -> ErStroke {
        self.stroke
    }

    /// Optional relationship label.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Set or clear the relationship label.
    pub fn set_label<T: Into<String>>(&mut self, label: Option<T>) {
        self.label = label.map(Into::into);
    }

    /// Original Mermaid connector body when known.
    pub fn raw_connector(&self) -> Option<&str> {
        self.raw_connector.as_deref()
    }
}
