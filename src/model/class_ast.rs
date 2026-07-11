// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Class-diagram AST: classes with members and typed relations.
//!
//! Stable ids key classes (`class/class`) and relations (`class/relation`). Notes and raw Mermaid
//! connectors are sidecar / lossless-export metadata, not required for layout.

use std::collections::BTreeMap;

use super::ids::ObjectId;

/// Class diagram content keyed by stable object IDs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassAst {
    classes: BTreeMap<ObjectId, ClassNode>,
    relations: BTreeMap<ObjectId, ClassRelation>,
}

impl ClassAst {
    /// Classes keyed by stable id.
    pub fn classes(&self) -> &BTreeMap<ObjectId, ClassNode> {
        &self.classes
    }

    /// Mutable class map.
    pub fn classes_mut(&mut self) -> &mut BTreeMap<ObjectId, ClassNode> {
        &mut self.classes
    }

    /// Relations keyed by stable id.
    pub fn relations(&self) -> &BTreeMap<ObjectId, ClassRelation> {
        &self.relations
    }

    /// Mutable relation map.
    pub fn relations_mut(&mut self) -> &mut BTreeMap<ObjectId, ClassRelation> {
        &mut self.relations
    }
}

/// Class box with optional attribute and method compartments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassNode {
    name: String,
    attributes: Vec<String>,
    methods: Vec<String>,
    /// Top-level note (sidecar / UI); not part of Mermaid export.
    note: Option<String>,
}

impl ClassNode {
    /// Empty compartments and no note.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), attributes: Vec::new(), methods: Vec::new(), note: None }
    }

    /// Builder: replace attribute lines.
    pub fn with_attributes(mut self, attributes: Vec<String>) -> Self {
        self.attributes = attributes;
        self
    }

    /// Builder: replace method lines.
    pub fn with_methods(mut self, methods: Vec<String>) -> Self {
        self.methods = methods;
        self
    }

    /// Sidecar/UI note; not part of Mermaid export.
    pub fn set_note<T: Into<String>>(&mut self, note: Option<T>) {
        self.note = note.map(Into::into);
    }

    /// Class display name (also Mermaid identity).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Rename the class.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Attribute compartment lines.
    pub fn attributes(&self) -> &[String] {
        &self.attributes
    }

    /// Method compartment lines.
    pub fn methods(&self) -> &[String] {
        &self.methods
    }

    /// Sidecar/UI note text.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// Mutable attributes.
    pub fn attributes_mut(&mut self) -> &mut Vec<String> {
        &mut self.attributes
    }

    /// Mutable methods.
    pub fn methods_mut(&mut self) -> &mut Vec<String> {
        &mut self.methods
    }
}

#[cfg(test)]
mod tests {
    use super::ClassNode;

    #[test]
    fn class_node_note_can_be_set_and_cleared() {
        let mut node = ClassNode::new("Foo");
        assert_eq!(node.note(), None);

        node.set_note(Some("domain aggregate root"));
        assert_eq!(node.note(), Some("domain aggregate root"));

        node.set_note::<&str>(None);
        assert_eq!(node.note(), None);
    }
}

/// Relation kinds supported in the Mermaid classDiagram subset.
///
/// Direction in the AST is always left→right as written; geometric cap side is recovered from
/// `raw_connector` when present, else a default right-facing token for the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassRelationKind {
    Inheritance,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Realization,
    Link,
}

/// Directed class relation (left→right as written) with optional label and raw connector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassRelation {
    from_class_id: ObjectId,
    to_class_id: ObjectId,
    kind: ClassRelationKind,
    label: Option<String>,
    /// Raw Mermaid connector for lossless export when known (cap side lives in the token).
    raw_connector: Option<String>,
}

impl ClassRelation {
    /// Relation without label or raw connector.
    pub fn new(from_class_id: ObjectId, to_class_id: ObjectId, kind: ClassRelationKind) -> Self {
        Self { from_class_id, to_class_id, kind, label: None, raw_connector: None }
    }

    /// Builder: optional relation label.
    pub fn with_label(mut self, label: Option<impl Into<String>>) -> Self {
        self.label = label.map(Into::into);
        self
    }

    /// Builder: preserve original Mermaid connector for export.
    pub fn with_raw_connector(mut self, raw: Option<impl Into<String>>) -> Self {
        self.raw_connector = raw.map(Into::into);
        self
    }

    /// Source class id (left endpoint as written).
    pub fn from_class_id(&self) -> &ObjectId {
        &self.from_class_id
    }

    /// Target class id (right endpoint as written).
    pub fn to_class_id(&self) -> &ObjectId {
        &self.to_class_id
    }

    /// Semantic relation kind.
    pub fn kind(&self) -> ClassRelationKind {
        self.kind
    }

    /// Optional relation label.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Set or clear the relation label.
    pub fn set_label<T: Into<String>>(&mut self, label: Option<T>) {
        self.label = label.map(Into::into);
    }

    /// Original Mermaid connector token when known.
    pub fn raw_connector(&self) -> Option<&str> {
        self.raw_connector.as_deref()
    }
}
