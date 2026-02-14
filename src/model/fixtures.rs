// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
use super::ids::ObjectId;

fn oid(value: &str) -> ObjectId {
    ObjectId::new(value).expect("object id")
}

pub(crate) fn flowchart_small_dag() -> FlowchartAst {
    let mut ast = FlowchartAst::default();

    let n_a = oid("n:a");
    let n_b = oid("n:b");
    let n_c = oid("n:c");
    let n_d = oid("n:d");

    ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
    ast.nodes_mut().insert(n_c.clone(), FlowNode::new("C"));
    ast.nodes_mut().insert(n_d.clone(), FlowNode::new("D"));

    ast.edges_mut()
        .insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));
    ast.edges_mut()
        .insert(oid("e:ac"), FlowEdge::new(n_a, n_c.clone()));
    ast.edges_mut()
        .insert(oid("e:bd"), FlowEdge::new(n_b, n_d.clone()));
    ast.edges_mut().insert(oid("e:cd"), FlowEdge::new(n_c, n_d));

    ast
}

#[cfg(test)]
pub(crate) fn flowchart_obstacle_route() -> FlowchartAst {
    let mut ast = FlowchartAst::default();

    let n_a = oid("n:a");
    let n_b = oid("n:b");
    let n_d = oid("n:d");

    ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
    ast.nodes_mut().insert(n_d.clone(), FlowNode::new("D"));

    ast.edges_mut()
        .insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));
    ast.edges_mut()
        .insert(oid("e:bd"), FlowEdge::new(n_b.clone(), n_d.clone()));
    ast.edges_mut().insert(oid("e:ad"), FlowEdge::new(n_a, n_d));

    ast
}

#[cfg(test)]
pub(crate) fn flowchart_node_overlap_avoidance_regression() -> FlowchartAst {
    let mut ast = FlowchartAst::default();

    let n_a = oid("n:a");
    let n_b = oid("n:b");
    let n_c = oid("n:c");
    let n_d = oid("n:d");

    ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    ast.nodes_mut()
        .insert(n_b.clone(), FlowNode::new("WideObstacle"));
    ast.nodes_mut()
        .insert(n_c.clone(), FlowNode::new("AlsoWideObstacle"));
    ast.nodes_mut().insert(n_d.clone(), FlowNode::new("D"));

    ast.edges_mut()
        .insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));
    ast.edges_mut()
        .insert(oid("e:bc"), FlowEdge::new(n_b.clone(), n_c.clone()));
    ast.edges_mut()
        .insert(oid("e:cd"), FlowEdge::new(n_c.clone(), n_d.clone()));
    ast.edges_mut().insert(oid("e:ad"), FlowEdge::new(n_a, n_d));

    ast
}
