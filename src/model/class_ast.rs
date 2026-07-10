// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Class-diagram AST: classes with members and typed relations.

use std::collections::BTreeMap;

use super::ids::ObjectId;

/// Class diagram content keyed by stable object IDs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassAst {
    classes: BTreeMap<ObjectId, ClassNode>,
    relations: BTreeMap<ObjectId, ClassRelation>,
}

impl ClassAst {
    pub fn classes(&self) -> &BTreeMap<ObjectId, ClassNode> {
        &self.classes
    }

    pub fn classes_mut(&mut self) -> &mut BTreeMap<ObjectId, ClassNode> {
        &mut self.classes
    }

    pub fn relations(&self) -> &BTreeMap<ObjectId, ClassRelation> {
        &self.relations
    }

    pub fn relations_mut(&mut self) -> &mut BTreeMap<ObjectId, ClassRelation> {
        &mut self.relations
    }
}

/// A class box with optional attribute and method compartments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassNode {
    name: String,
    attributes: Vec<String>,
    methods: Vec<String>,
    /// Top-level note (sidecar / UI); not part of Mermaid export.
    note: Option<String>,
}

impl ClassNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), attributes: Vec::new(), methods: Vec::new(), note: None }
    }

    pub fn with_attributes(mut self, attributes: Vec<String>) -> Self {
        self.attributes = attributes;
        self
    }

    pub fn with_methods(mut self, methods: Vec<String>) -> Self {
        self.methods = methods;
        self
    }

    pub fn set_note<T: Into<String>>(&mut self, note: Option<T>) {
        self.note = note.map(Into::into);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn attributes(&self) -> &[String] {
        &self.attributes
    }

    pub fn methods(&self) -> &[String] {
        &self.methods
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn attributes_mut(&mut self) -> &mut Vec<String> {
        &mut self.attributes
    }

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

/// Directed class relation with optional label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassRelation {
    from_class_id: ObjectId,
    to_class_id: ObjectId,
    kind: ClassRelationKind,
    /// True when the Mermaid token points left (e.g. `<|--` means to inherits from from).
    /// Stored as source→target in AST; kind encodes the geometric cap placement on lower.
    label: Option<String>,
    /// Raw Mermaid connector for lossless export when known.
    raw_connector: Option<String>,
}

impl ClassRelation {
    pub fn new(from_class_id: ObjectId, to_class_id: ObjectId, kind: ClassRelationKind) -> Self {
        Self { from_class_id, to_class_id, kind, label: None, raw_connector: None }
    }

    pub fn with_label(mut self, label: Option<impl Into<String>>) -> Self {
        self.label = label.map(Into::into);
        self
    }

    pub fn with_raw_connector(mut self, raw: Option<impl Into<String>>) -> Self {
        self.raw_connector = raw.map(Into::into);
        self
    }

    pub fn from_class_id(&self) -> &ObjectId {
        &self.from_class_id
    }

    pub fn to_class_id(&self) -> &ObjectId {
        &self.to_class_id
    }

    pub fn kind(&self) -> ClassRelationKind {
        self.kind
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn set_label<T: Into<String>>(&mut self, label: Option<T>) {
        self.label = label.map(Into::into);
    }

    pub fn raw_connector(&self) -> Option<&str> {
        self.raw_connector.as_deref()
    }
}
