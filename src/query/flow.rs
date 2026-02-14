// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::model::{FlowchartAst, ObjectId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReachDirection {
    Out,
    In,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlowNodeDegree {
    pub in_degree: u64,
    pub out_degree: u64,
}

pub fn degrees(ast: &FlowchartAst) -> BTreeMap<ObjectId, FlowNodeDegree> {
    let mut degrees: BTreeMap<ObjectId, FlowNodeDegree> = BTreeMap::new();
    for node_id in ast.nodes().keys() {
        degrees.entry(node_id.clone()).or_default();
    }

    for edge in ast.edges().values() {
        let from_degree = degrees.entry(edge.from_node_id().clone()).or_default();
        from_degree.out_degree = from_degree.out_degree.saturating_add(1);

        let to_degree = degrees.entry(edge.to_node_id().clone()).or_default();
        to_degree.in_degree = to_degree.in_degree.saturating_add(1);
    }

    degrees
}

fn outgoing_adjacency(ast: &FlowchartAst) -> BTreeMap<ObjectId, Vec<ObjectId>> {
    let mut outgoing: BTreeMap<ObjectId, BTreeSet<ObjectId>> = BTreeMap::new();

    for node_id in ast.nodes().keys() {
        outgoing.entry(node_id.clone()).or_default();
    }

    for edge in ast.edges().values() {
        outgoing
            .entry(edge.from_node_id().clone())
            .or_default()
            .insert(edge.to_node_id().clone());
        outgoing.entry(edge.to_node_id().clone()).or_default();
    }

    outgoing
        .into_iter()
        .map(|(node_id, neighbors)| (node_id, neighbors.into_iter().collect()))
        .collect()
}

fn incoming_adjacency(
    outgoing: &BTreeMap<ObjectId, Vec<ObjectId>>,
) -> BTreeMap<ObjectId, Vec<ObjectId>> {
    let mut incoming: BTreeMap<ObjectId, BTreeSet<ObjectId>> = BTreeMap::new();

    for node_id in outgoing.keys() {
        incoming.entry(node_id.clone()).or_default();
    }

    for (from, tos) in outgoing {
        for to in tos {
            incoming.entry(to.clone()).or_default().insert(from.clone());
        }
    }

    incoming
        .into_iter()
        .map(|(node_id, neighbors)| (node_id, neighbors.into_iter().collect()))
        .collect()
}

fn bfs_distances(
    adjacency: &BTreeMap<ObjectId, Vec<ObjectId>>,
    start: &ObjectId,
) -> BTreeMap<ObjectId, usize> {
    let mut dist: BTreeMap<ObjectId, usize> = BTreeMap::new();
    if !adjacency.contains_key(start) {
        return dist;
    }

    let mut queue: VecDeque<ObjectId> = VecDeque::new();
    dist.insert(start.clone(), 0);
    queue.push_back(start.clone());

    while let Some(node_id) = queue.pop_front() {
        let next_distance = dist
            .get(&node_id)
            .copied()
            .expect("node already inserted")
            .saturating_add(1);

        for next_id in adjacency.get(&node_id).into_iter().flatten() {
            if dist.contains_key(next_id) {
                continue;
            }
            dist.insert(next_id.clone(), next_distance);
            queue.push_back(next_id.clone());
        }
    }

    dist
}

fn bfs_reachable(
    adjacency: &BTreeMap<ObjectId, Vec<ObjectId>>,
    start: &ObjectId,
    known_nodes: &BTreeSet<ObjectId>,
) -> BTreeSet<ObjectId> {
    let mut visited: BTreeSet<ObjectId> = BTreeSet::new();
    if !known_nodes.contains(start) {
        return visited;
    }

    let mut queue: VecDeque<ObjectId> = VecDeque::new();

    visited.insert(start.clone());
    queue.push_back(start.clone());

    while let Some(node_id) = queue.pop_front() {
        for next_id in adjacency.get(&node_id).into_iter().flatten() {
            if !known_nodes.contains(next_id) {
                continue;
            }
            if visited.insert(next_id.clone()) {
                queue.push_back(next_id.clone());
            }
        }
    }

    visited
}

pub(crate) fn reachable_with_direction(
    ast: &FlowchartAst,
    from_node_id: &ObjectId,
    direction: ReachDirection,
) -> Vec<ObjectId> {
    let known_nodes = ast.nodes().keys().cloned().collect::<BTreeSet<_>>();
    if !known_nodes.contains(from_node_id) {
        return Vec::new();
    }

    let outgoing = outgoing_adjacency(ast);
    match direction {
        ReachDirection::Out => bfs_reachable(&outgoing, from_node_id, &known_nodes)
            .into_iter()
            .collect(),
        ReachDirection::In => {
            let incoming = incoming_adjacency(&outgoing);
            bfs_reachable(&incoming, from_node_id, &known_nodes)
                .into_iter()
                .collect()
        }
        ReachDirection::Both => {
            let incoming = incoming_adjacency(&outgoing);
            let mut visited = bfs_reachable(&outgoing, from_node_id, &known_nodes);
            visited.extend(bfs_reachable(&incoming, from_node_id, &known_nodes));
            visited.into_iter().collect()
        }
    }
}

pub fn reachable(ast: &FlowchartAst, from_node_id: &ObjectId) -> Vec<ObjectId> {
    reachable_with_direction(ast, from_node_id, ReachDirection::Out)
}

pub fn paths(
    ast: &FlowchartAst,
    from_node_id: &ObjectId,
    to_node_id: &ObjectId,
    limit: usize,
    max_extra_hops: usize,
) -> Vec<Vec<ObjectId>> {
    if limit == 0 {
        return Vec::new();
    }

    let outgoing = outgoing_adjacency(ast);
    if !outgoing.contains_key(from_node_id) || !outgoing.contains_key(to_node_id) {
        return Vec::new();
    }

    if from_node_id == to_node_id {
        return vec![vec![from_node_id.clone()]];
    }

    let dist_from = bfs_distances(&outgoing, from_node_id);
    let shortest_len = match dist_from.get(to_node_id).copied() {
        Some(len) => len,
        None => return Vec::new(),
    };
    let max_len = shortest_len.saturating_add(max_extra_hops);

    let incoming = incoming_adjacency(&outgoing);
    let dist_to = bfs_distances(&incoming, to_node_id);

    let mut results: Vec<Vec<ObjectId>> = Vec::new();
    let mut queue: VecDeque<Vec<ObjectId>> = VecDeque::new();
    queue.push_back(vec![from_node_id.clone()]);

    while let Some(path) = queue.pop_front() {
        if results.len() >= limit {
            break;
        }

        let last = path.last().expect("non-empty path");
        if last == to_node_id {
            results.push(path);
            continue;
        }

        let hops = path.len().saturating_sub(1);
        if hops >= max_len {
            continue;
        }

        let remaining = match dist_to.get(last).copied() {
            Some(remaining) => remaining,
            None => continue,
        };
        if hops.saturating_add(remaining) > max_len {
            continue;
        }

        for next_id in outgoing.get(last).into_iter().flatten() {
            if path.contains(next_id) {
                continue;
            }
            let mut next_path = path.clone();
            next_path.push(next_id.clone());
            queue.push_back(next_path);
        }
    }

    results
}

pub fn cycles(ast: &FlowchartAst) -> Vec<Vec<ObjectId>> {
    let outgoing = outgoing_adjacency(ast);

    let mut index: usize = 0;
    let mut indices: BTreeMap<ObjectId, usize> = BTreeMap::new();
    let mut lowlink: BTreeMap<ObjectId, usize> = BTreeMap::new();
    let mut stack: Vec<ObjectId> = Vec::new();
    let mut on_stack: BTreeSet<ObjectId> = BTreeSet::new();
    let mut sccs: Vec<Vec<ObjectId>> = Vec::new();

    #[allow(clippy::too_many_arguments)]
    fn strongconnect(
        v: ObjectId,
        index: &mut usize,
        outgoing: &BTreeMap<ObjectId, Vec<ObjectId>>,
        indices: &mut BTreeMap<ObjectId, usize>,
        lowlink: &mut BTreeMap<ObjectId, usize>,
        stack: &mut Vec<ObjectId>,
        on_stack: &mut BTreeSet<ObjectId>,
        sccs: &mut Vec<Vec<ObjectId>>,
    ) {
        indices.insert(v.clone(), *index);
        lowlink.insert(v.clone(), *index);
        *index = index.saturating_add(1);

        stack.push(v.clone());
        on_stack.insert(v.clone());

        for w in outgoing.get(&v).into_iter().flatten() {
            if !indices.contains_key(w) {
                strongconnect(
                    w.clone(),
                    index,
                    outgoing,
                    indices,
                    lowlink,
                    stack,
                    on_stack,
                    sccs,
                );
                let v_low = lowlink.get(&v).copied().unwrap_or(usize::MAX);
                let w_low = lowlink.get(w).copied().unwrap_or(usize::MAX);
                lowlink.insert(v.clone(), v_low.min(w_low));
            } else if on_stack.contains(w) {
                let v_low = lowlink.get(&v).copied().unwrap_or(usize::MAX);
                let w_index = indices.get(w).copied().unwrap_or(usize::MAX);
                lowlink.insert(v.clone(), v_low.min(w_index));
            }
        }

        let v_index = indices.get(&v).copied().unwrap_or(usize::MAX);
        let v_low = lowlink.get(&v).copied().unwrap_or(usize::MAX);
        if v_low == v_index {
            let mut scc: Vec<ObjectId> = Vec::new();
            while let Some(w) = stack.pop() {
                on_stack.remove(&w);
                scc.push(w.clone());
                if w == v {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    for v in outgoing.keys() {
        if indices.contains_key(v) {
            continue;
        }
        strongconnect(
            v.clone(),
            &mut index,
            &outgoing,
            &mut indices,
            &mut lowlink,
            &mut stack,
            &mut on_stack,
            &mut sccs,
        );
    }

    let mut cycles: Vec<Vec<ObjectId>> = sccs
        .into_iter()
        .filter_map(|mut scc| {
            scc.sort();
            match scc.as_slice() {
                [] => None,
                [node_id] => outgoing
                    .get(node_id)
                    .into_iter()
                    .flatten()
                    .any(|next_id| next_id == node_id)
                    .then_some(scc),
                _ => Some(scc),
            }
        })
        .collect();

    cycles.sort();
    cycles
}

pub fn dead_ends(ast: &FlowchartAst) -> Vec<ObjectId> {
    let outgoing = outgoing_adjacency(ast);
    outgoing
        .into_iter()
        .filter_map(|(node_id, next_ids)| next_ids.is_empty().then_some(node_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        cycles, dead_ends, degrees, paths, reachable, reachable_with_direction, ReachDirection,
    };
    use crate::model::{FlowEdge, FlowNode, FlowchartAst, ObjectId};

    fn ids(values: &[ObjectId]) -> Vec<String> {
        values.iter().map(|id| id.as_str().to_owned()).collect()
    }

    fn paths_ids(values: &[Vec<ObjectId>]) -> Vec<Vec<String>> {
        values.iter().map(|path| ids(path)).collect()
    }

    fn fixture_ast() -> FlowchartAst {
        let mut ast = FlowchartAst::default();

        for node in [
            "n:a", "n:b", "n:c", "n:d", "n:e", "n:f", "n:x", "n:y", "n:z",
        ] {
            let node_id = ObjectId::new(node).expect("node id");
            ast.nodes_mut()
                .insert(node_id, FlowNode::new(node.to_uppercase()));
        }

        let mut add_edge = |edge_id: &str, from: &str, to: &str| {
            let edge_id = ObjectId::new(edge_id).expect("edge id");
            let from = ObjectId::new(from).expect("from node id");
            let to = ObjectId::new(to).expect("to node id");
            ast.edges_mut().insert(edge_id, FlowEdge::new(from, to));
        };

        add_edge("e:ab", "n:a", "n:b");
        add_edge("e:bc", "n:b", "n:c");
        add_edge("e:ad", "n:a", "n:d");
        add_edge("e:dc", "n:d", "n:c");
        add_edge("e:ce", "n:c", "n:e");
        add_edge("e:bd", "n:b", "n:d");

        add_edge("e:xy", "n:x", "n:y");
        add_edge("e:yx", "n:y", "n:x");
        add_edge("e:zz", "n:z", "n:z");

        ast
    }

    #[test]
    fn reachable_returns_sorted_ids_including_start() {
        let ast = fixture_ast();
        let start = ObjectId::new("n:a").expect("start node id");
        let results = reachable(&ast, &start);
        assert_eq!(ids(&results), vec!["n:a", "n:b", "n:c", "n:d", "n:e"]);
    }

    #[test]
    fn reachable_with_direction_in_returns_sorted_ids_including_start() {
        let ast = fixture_ast();
        let start = ObjectId::new("n:c").expect("start node id");
        let results = reachable_with_direction(&ast, &start, ReachDirection::In);
        assert_eq!(ids(&results), vec!["n:a", "n:b", "n:c", "n:d"]);
    }

    #[test]
    fn reachable_with_direction_both_unions_in_and_out() {
        let ast = fixture_ast();
        let start = ObjectId::new("n:c").expect("start node id");
        let results = reachable_with_direction(&ast, &start, ReachDirection::Both);
        assert_eq!(ids(&results), vec!["n:a", "n:b", "n:c", "n:d", "n:e"]);
    }

    #[test]
    fn degrees_counts_in_and_out_for_each_node() {
        let ast = fixture_ast();
        let degrees = degrees(&ast);

        let a = degrees
            .get(&ObjectId::new("n:a").expect("a"))
            .expect("degree");
        assert_eq!(a.in_degree, 0);
        assert_eq!(a.out_degree, 2);

        let c = degrees
            .get(&ObjectId::new("n:c").expect("c"))
            .expect("degree");
        assert_eq!(c.in_degree, 2);
        assert_eq!(c.out_degree, 1);

        let f = degrees
            .get(&ObjectId::new("n:f").expect("f"))
            .expect("degree");
        assert_eq!(f.in_degree, 0);
        assert_eq!(f.out_degree, 0);
    }

    #[test]
    fn paths_returns_shortest_paths_then_capped_alternates() {
        let ast = fixture_ast();
        let from = ObjectId::new("n:a").expect("from node id");
        let to = ObjectId::new("n:c").expect("to node id");

        let shortest = paths(&ast, &from, &to, 10, 0);
        assert_eq!(
            paths_ids(&shortest),
            vec![vec!["n:a", "n:b", "n:c"], vec!["n:a", "n:d", "n:c"],]
        );

        let alternates = paths(&ast, &from, &to, 10, 1);
        assert_eq!(
            paths_ids(&alternates),
            vec![
                vec!["n:a", "n:b", "n:c"],
                vec!["n:a", "n:d", "n:c"],
                vec!["n:a", "n:b", "n:d", "n:c"],
            ]
        );
    }

    #[test]
    fn cycles_returns_scc_groups_in_deterministic_order() {
        let ast = fixture_ast();
        let results = cycles(&ast);
        assert_eq!(paths_ids(&results), vec![vec!["n:x", "n:y"], vec!["n:z"]]);
    }

    #[test]
    fn dead_ends_returns_terminal_nodes() {
        let ast = fixture_ast();
        let results = dead_ends(&ast);
        assert_eq!(ids(&results), vec!["n:e", "n:f"]);
    }
}
