// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::BTreeMap;

use super::ids::ObjectId;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlowchartAst {
    nodes: BTreeMap<ObjectId, FlowNode>,
    edges: BTreeMap<ObjectId, FlowEdge>,
    default_edge_style: Option<String>,
    groups: BTreeMap<ObjectId, FlowGroup>,
    node_groups: BTreeMap<ObjectId, ObjectId>,
}

impl FlowchartAst {
    pub fn nodes(&self) -> &BTreeMap<ObjectId, FlowNode> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut BTreeMap<ObjectId, FlowNode> {
        &mut self.nodes
    }

    pub fn edges(&self) -> &BTreeMap<ObjectId, FlowEdge> {
        &self.edges
    }

    pub fn edges_mut(&mut self) -> &mut BTreeMap<ObjectId, FlowEdge> {
        &mut self.edges
    }

    pub fn default_edge_style(&self) -> Option<&str> {
        self.default_edge_style.as_deref()
    }

    pub fn set_default_edge_style<T: Into<String>>(&mut self, style: Option<T>) {
        self.default_edge_style = style.map(Into::into);
    }

    pub fn groups(&self) -> &BTreeMap<ObjectId, FlowGroup> {
        &self.groups
    }

    pub fn groups_mut(&mut self) -> &mut BTreeMap<ObjectId, FlowGroup> {
        &mut self.groups
    }

    pub fn node_groups(&self) -> &BTreeMap<ObjectId, ObjectId> {
        &self.node_groups
    }

    pub fn node_groups_mut(&mut self) -> &mut BTreeMap<ObjectId, ObjectId> {
        &mut self.node_groups
    }

    pub fn node_group(&self, node_id: &ObjectId) -> Option<&ObjectId> {
        self.node_groups.get(node_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowNode {
    mermaid_id: Option<String>,
    label: String,
    shape: String,
    note: Option<String>,
}

impl FlowNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            mermaid_id: None,
            label: label.into(),
            shape: "rect".to_owned(),
            note: None,
        }
    }

    pub fn new_with(
        label: impl Into<String>,
        shape: impl Into<String>,
        mermaid_id: Option<String>,
    ) -> Self {
        Self {
            mermaid_id,
            label: label.into(),
            shape: shape.into(),
            note: None,
        }
    }

    pub fn set_mermaid_id<T: Into<String>>(&mut self, mermaid_id: Option<T>) {
        self.mermaid_id = mermaid_id.map(Into::into);
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    pub fn set_shape(&mut self, shape: impl Into<String>) {
        self.shape = shape.into();
    }

    pub fn set_note<T: Into<String>>(&mut self, note: Option<T>) {
        self.note = note.map(Into::into);
    }

    pub fn mermaid_id(&self) -> Option<&str> {
        self.mermaid_id.as_deref()
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn shape(&self) -> &str {
        &self.shape
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowEdge {
    from_node_id: ObjectId,
    to_node_id: ObjectId,
    label: Option<String>,
    connector: Option<String>,
    style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowGroup {
    mermaid_id: Option<String>,
    label: String,
}

impl FlowGroup {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            mermaid_id: None,
            label: label.into(),
        }
    }

    pub fn new_with(label: impl Into<String>, mermaid_id: Option<String>) -> Self {
        Self {
            mermaid_id,
            label: label.into(),
        }
    }

    pub fn set_mermaid_id<T: Into<String>>(&mut self, mermaid_id: Option<T>) {
        self.mermaid_id = mermaid_id.map(Into::into);
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    pub fn mermaid_id(&self) -> Option<&str> {
        self.mermaid_id.as_deref()
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

impl FlowEdge {
    pub fn new(from_node_id: ObjectId, to_node_id: ObjectId) -> Self {
        Self {
            from_node_id,
            to_node_id,
            label: None,
            connector: None,
            style: None,
        }
    }

    pub fn new_with(
        from_node_id: ObjectId,
        to_node_id: ObjectId,
        label: Option<String>,
        style: Option<String>,
    ) -> Self {
        Self {
            from_node_id,
            to_node_id,
            label,
            connector: None,
            style,
        }
    }

    pub fn set_label<T: Into<String>>(&mut self, label: Option<T>) {
        self.label = label.map(Into::into);
    }

    pub fn set_connector<T: Into<String>>(&mut self, connector: Option<T>) {
        self.connector = connector.map(Into::into);
    }

    pub fn set_style<T: Into<String>>(&mut self, style: Option<T>) {
        self.style = style.map(Into::into);
    }

    pub fn from_node_id(&self) -> &ObjectId {
        &self.from_node_id
    }

    pub fn to_node_id(&self) -> &ObjectId {
        &self.to_node_id
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn connector(&self) -> Option<&str> {
        self.connector.as_deref()
    }

    pub fn style(&self) -> Option<&str> {
        self.style.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::{FlowEdge, FlowNode};
    use crate::model::ObjectId;

    #[test]
    fn flow_node_can_be_constructed_and_updated() {
        let mut node = FlowNode::new("Hello");
        assert_eq!(node.mermaid_id(), None);
        assert_eq!(node.label(), "Hello");
        assert_eq!(node.shape(), "rect");
        assert_eq!(node.note(), None);

        node.set_mermaid_id(Some("n1"));
        node.set_label("World");
        node.set_shape("circle");
        node.set_note(Some("invariant"));

        assert_eq!(node.mermaid_id(), Some("n1"));
        assert_eq!(node.label(), "World");
        assert_eq!(node.shape(), "circle");
        assert_eq!(node.note(), Some("invariant"));

        node.set_mermaid_id::<&str>(None);
        assert_eq!(node.mermaid_id(), None);

        node.set_note::<&str>(None);
        assert_eq!(node.note(), None);
    }

    #[test]
    fn flow_node_can_be_constructed_with_explicit_mermaid_fields() {
        let node = FlowNode::new_with("Start", "stadium", Some("start".to_owned()));

        assert_eq!(node.mermaid_id(), Some("start"));
        assert_eq!(node.label(), "Start");
        assert_eq!(node.shape(), "stadium");
    }

    #[test]
    fn flow_edge_can_be_constructed_and_updated() {
        let from = ObjectId::new("n1").expect("from node id");
        let to = ObjectId::new("n2").expect("to node id");
        let mut edge = FlowEdge::new(from.clone(), to.clone());

        assert_eq!(edge.from_node_id(), &from);
        assert_eq!(edge.to_node_id(), &to);
        assert_eq!(edge.label(), None);
        assert_eq!(edge.connector(), None);
        assert_eq!(edge.style(), None);

        edge.set_label(Some("yes"));
        edge.set_connector(Some("-.->"));
        edge.set_style(Some("dashed"));

        assert_eq!(edge.label(), Some("yes"));
        assert_eq!(edge.connector(), Some("-.->"));
        assert_eq!(edge.style(), Some("dashed"));

        edge.set_label::<&str>(None);
        edge.set_connector::<&str>(None);
        edge.set_style::<&str>(None);

        assert_eq!(edge.label(), None);
        assert_eq!(edge.connector(), None);
        assert_eq!(edge.style(), None);
    }

    #[test]
    fn flow_edge_can_be_constructed_with_explicit_mermaid_fields() {
        let from = ObjectId::new("n1").expect("from node id");
        let to = ObjectId::new("n2").expect("to node id");
        let edge = FlowEdge::new_with(
            from.clone(),
            to.clone(),
            Some("maybe".to_owned()),
            Some("thick".to_owned()),
        );

        assert_eq!(edge.from_node_id(), &from);
        assert_eq!(edge.to_node_id(), &to);
        assert_eq!(edge.label(), Some("maybe"));
        assert_eq!(edge.style(), Some("thick"));
    }
}
