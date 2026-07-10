// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Graph-family layout entry points (flow / class / er).
//!
//! Commit-1 bridges to [`super::layout_flowchart`] via the scene→flowchart AST bridge. Later
//! commits can layout directly from [`crate::render::scene::GraphModel`] metrics (variable box
//! heights, compartments).
//!
//! Layering note: scene types currently live under `render::scene`, so `layout` depends on
//! `render` for the model types. That inversion is temporary; a neutral `scene` module is
//! preferred once paint stops bridging through domain ASTs.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{FlowchartAst, ObjectId};
use crate::render::scene::GraphModel;

use super::flowchart::{layout_flowchart, FlowEdgeEndpoint, FlowchartLayout, FlowchartLayoutError};

/// Place a graph-family scene.
pub fn layout_graph(model: &GraphModel) -> Result<FlowchartLayout, FlowchartLayoutError> {
    layout_flowchart(&model.to_flowchart_ast())
}

/// Place a general graph while retaining all original edges for paint.
///
/// Class and ER relations commonly contain cycles and self-relations. The layered flowchart
/// placer only needs an acyclic backbone to choose node coordinates, so this function builds a
/// deterministic maximal DAG from the scene and lets paint route every edge against that layout.
pub fn layout_general_graph(model: &GraphModel) -> Result<FlowchartLayout, FlowchartLayoutError> {
    let source = model.to_flowchart_ast();
    let mut backbone = FlowchartAst::default();
    backbone.nodes_mut().extend(source.nodes().clone());

    let mut outgoing = BTreeMap::<ObjectId, BTreeSet<ObjectId>>::new();
    for node_id in source.nodes().keys() {
        outgoing.insert(node_id.clone(), BTreeSet::new());
    }

    fn reaches(
        outgoing: &BTreeMap<ObjectId, BTreeSet<ObjectId>>,
        start: &ObjectId,
        target: &ObjectId,
    ) -> bool {
        let mut pending = vec![start.clone()];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if &current == target {
                return true;
            }
            if !seen.insert(current.clone()) {
                continue;
            }
            pending.extend(outgoing.get(&current).into_iter().flatten().rev().cloned());
        }
        false
    }

    for (edge_id, edge) in source.edges() {
        let from = edge.from_node_id();
        let to = edge.to_node_id();
        if !source.nodes().contains_key(from) {
            return Err(FlowchartLayoutError::UnknownNode {
                edge_id: edge_id.clone(),
                endpoint: FlowEdgeEndpoint::From,
                node_id: from.clone(),
            });
        }
        if !source.nodes().contains_key(to) {
            return Err(FlowchartLayoutError::UnknownNode {
                edge_id: edge_id.clone(),
                endpoint: FlowEdgeEndpoint::To,
                node_id: to.clone(),
            });
        }

        if from == to || reaches(&outgoing, to, from) {
            continue;
        }
        outgoing.get_mut(from).expect("validated graph node").insert(to.clone());
        backbone.edges_mut().insert(edge_id.clone(), edge.clone());
    }

    layout_flowchart(&backbone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FlowEdge, FlowNode};
    use crate::render::lower::lower_flowchart;

    fn oid(value: &str) -> ObjectId {
        ObjectId::new(value).expect("object id")
    }

    #[test]
    fn general_graph_layout_accepts_cycles_and_self_relations() {
        let mut ast = FlowchartAst::default();
        for id in ["n:a", "n:b", "n:c"] {
            ast.nodes_mut().insert(oid(id), FlowNode::new(id));
        }
        for (edge_id, from, to) in [
            ("e:ab", "n:a", "n:b"),
            ("e:bc", "n:b", "n:c"),
            ("e:ca", "n:c", "n:a"),
            ("e:aa", "n:a", "n:a"),
        ] {
            ast.edges_mut().insert(oid(edge_id), FlowEdge::new(oid(from), oid(to)));
        }

        let layout = layout_general_graph(&lower_flowchart(&ast)).expect("general graph layout");
        assert_eq!(layout.node_placements().len(), 3);
    }
}
