// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

use crate::model::{CategoryPath, DiagramAst, DiagramId, ObjectId, ObjectRef, Session};

#[derive(Clone, Debug)]
pub struct SessionRouteAdjacency {
    adjacency: BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
}

impl SessionRouteAdjacency {
    pub fn derive(session: &Session) -> Self {
        Self {
            adjacency: derive_adjacency(session),
        }
    }
}

fn flow_node_category() -> CategoryPath {
    CategoryPath::new(vec!["flow".to_owned(), "node".to_owned()]).expect("static category")
}

fn flow_edge_category() -> CategoryPath {
    CategoryPath::new(vec!["flow".to_owned(), "edge".to_owned()]).expect("static category")
}

fn seq_message_category() -> CategoryPath {
    CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()]).expect("static category")
}

fn seq_participant_category() -> CategoryPath {
    CategoryPath::new(vec!["seq".to_owned(), "participant".to_owned()]).expect("static category")
}

fn seq_block_category() -> CategoryPath {
    CategoryPath::new(vec!["seq".to_owned(), "block".to_owned()]).expect("static category")
}

fn seq_section_category() -> CategoryPath {
    CategoryPath::new(vec!["seq".to_owned(), "section".to_owned()]).expect("static category")
}

fn flow_node_ref(diagram_id: &DiagramId, node_id: &ObjectId) -> ObjectRef {
    ObjectRef::new(diagram_id.clone(), flow_node_category(), node_id.clone())
}

fn flow_edge_ref(diagram_id: &DiagramId, edge_id: &ObjectId) -> ObjectRef {
    ObjectRef::new(diagram_id.clone(), flow_edge_category(), edge_id.clone())
}

fn seq_message_ref(diagram_id: &DiagramId, message_id: &ObjectId) -> ObjectRef {
    ObjectRef::new(
        diagram_id.clone(),
        seq_message_category(),
        message_id.clone(),
    )
}

fn seq_participant_ref(diagram_id: &DiagramId, participant_id: &ObjectId) -> ObjectRef {
    ObjectRef::new(
        diagram_id.clone(),
        seq_participant_category(),
        participant_id.clone(),
    )
}

fn seq_block_ref(diagram_id: &DiagramId, block_id: &ObjectId) -> ObjectRef {
    ObjectRef::new(diagram_id.clone(), seq_block_category(), block_id.clone())
}

fn seq_section_ref(diagram_id: &DiagramId, section_id: &ObjectId) -> ObjectRef {
    ObjectRef::new(
        diagram_id.clone(),
        seq_section_category(),
        section_id.clone(),
    )
}

fn insert_node(adjacency: &mut BTreeMap<ObjectRef, BTreeSet<ObjectRef>>, node: ObjectRef) {
    adjacency.entry(node).or_default();
}

fn insert_edge(
    adjacency: &mut BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
    from: ObjectRef,
    to: ObjectRef,
) {
    adjacency.entry(from).or_default().insert(to);
}

fn derive_adjacency(session: &Session) -> BTreeMap<ObjectRef, BTreeSet<ObjectRef>> {
    let mut adjacency: BTreeMap<ObjectRef, BTreeSet<ObjectRef>> = BTreeMap::new();

    for diagram in session.diagrams().values() {
        let diagram_id = diagram.diagram_id();
        match diagram.ast() {
            DiagramAst::Flowchart(ast) => {
                for node_id in ast.nodes().keys() {
                    insert_node(&mut adjacency, flow_node_ref(diagram_id, node_id));
                }
                for (edge_id, edge) in ast.edges().iter() {
                    let edge_ref = flow_edge_ref(diagram_id, edge_id);
                    insert_node(&mut adjacency, edge_ref.clone());

                    let from = flow_node_ref(diagram_id, edge.from_node_id());
                    let to = flow_node_ref(diagram_id, edge.to_node_id());

                    insert_node(&mut adjacency, from.clone());
                    insert_node(&mut adjacency, to.clone());

                    insert_edge(&mut adjacency, from.clone(), to.clone());

                    insert_edge(&mut adjacency, from.clone(), edge_ref.clone());
                    insert_edge(&mut adjacency, to.clone(), edge_ref.clone());
                    insert_edge(&mut adjacency, edge_ref.clone(), from);
                    insert_edge(&mut adjacency, edge_ref, to);
                }
            }
            DiagramAst::Sequence(ast) => {
                for participant_id in ast.participants().keys() {
                    insert_node(
                        &mut adjacency,
                        seq_participant_ref(diagram_id, participant_id),
                    );
                }

                let messages = ast.messages_in_order();

                let message_refs = messages
                    .iter()
                    .map(|msg| seq_message_ref(diagram_id, msg.message_id()))
                    .collect::<Vec<_>>();

                for reference in &message_refs {
                    insert_node(&mut adjacency, reference.clone());
                }

                for pair in message_refs.windows(2) {
                    let from = pair[0].clone();
                    let to = pair[1].clone();
                    insert_edge(&mut adjacency, from, to);
                }

                for msg in &messages {
                    let message_ref = seq_message_ref(diagram_id, msg.message_id());
                    let from_participant =
                        seq_participant_ref(diagram_id, msg.from_participant_id());
                    let to_participant = seq_participant_ref(diagram_id, msg.to_participant_id());

                    insert_node(&mut adjacency, message_ref.clone());
                    insert_node(&mut adjacency, from_participant.clone());
                    insert_node(&mut adjacency, to_participant.clone());

                    insert_edge(
                        &mut adjacency,
                        from_participant.clone(),
                        message_ref.clone(),
                    );
                    insert_edge(&mut adjacency, message_ref.clone(), from_participant);
                    insert_edge(&mut adjacency, to_participant.clone(), message_ref.clone());
                    insert_edge(&mut adjacency, message_ref, to_participant);
                }

                fn add_block(
                    diagram_id: &DiagramId,
                    block: &crate::model::seq_ast::SequenceBlock,
                    adjacency: &mut BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
                    parent: Option<ObjectRef>,
                ) {
                    let block_ref = seq_block_ref(diagram_id, block.block_id());
                    insert_node(adjacency, block_ref.clone());

                    if let Some(parent_ref) = parent.as_ref() {
                        insert_edge(adjacency, parent_ref.clone(), block_ref.clone());
                        insert_edge(adjacency, block_ref.clone(), parent_ref.clone());
                    }

                    for section in block.sections() {
                        let section_ref = seq_section_ref(diagram_id, section.section_id());
                        insert_node(adjacency, section_ref.clone());
                        insert_edge(adjacency, block_ref.clone(), section_ref.clone());
                        insert_edge(adjacency, section_ref.clone(), block_ref.clone());

                        for message_id in section.message_ids() {
                            let message_ref = seq_message_ref(diagram_id, message_id);
                            insert_node(adjacency, message_ref.clone());
                            insert_edge(adjacency, section_ref.clone(), message_ref.clone());
                            insert_edge(adjacency, message_ref, section_ref.clone());
                        }
                    }

                    for child in block.blocks() {
                        add_block(diagram_id, child, adjacency, Some(block_ref.clone()));
                    }
                }

                for block in ast.blocks() {
                    add_block(diagram_id, block, &mut adjacency, None);
                }
            }
        }
    }

    for xref in session.xrefs().values() {
        let a = xref.from().clone();
        let b = xref.to().clone();
        insert_edge(&mut adjacency, a.clone(), b.clone());
        insert_edge(&mut adjacency, b, a);
    }

    adjacency
}

fn reconstruct_path(
    mut current: ObjectRef,
    start: &ObjectRef,
    previous: &BTreeMap<ObjectRef, ObjectRef>,
) -> Vec<ObjectRef> {
    let mut reversed = vec![current.clone()];
    while &current != start {
        let Some(prev) = previous.get(&current) else {
            break;
        };
        current = prev.clone();
        reversed.push(current.clone());
    }
    reversed.reverse();
    reversed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutesOrdering {
    FewestHops,
    Lexicographic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateRoute {
    hops: u64,
    path: Vec<ObjectRef>,
}

impl Ord for CandidateRoute {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.hops, &self.path).cmp(&(other.hops, &other.path))
    }
}

impl PartialOrd for CandidateRoute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn find_routes_with_adjacency(
    adjacency: &SessionRouteAdjacency,
    from: &ObjectRef,
    to: &ObjectRef,
    limit: u64,
    max_hops: Option<u64>,
    ordering: RoutesOrdering,
) -> Vec<Vec<ObjectRef>> {
    if limit == 0 {
        return Vec::new();
    }

    if from == to {
        return vec![vec![from.clone()]];
    }

    let max_hops = max_hops.unwrap_or(u64::MAX);
    let mut candidates: BinaryHeap<Reverse<CandidateRoute>> = BinaryHeap::new();
    candidates.push(Reverse(CandidateRoute {
        hops: 0,
        path: vec![from.clone()],
    }));

    let mut routes = Vec::new();
    while let Some(Reverse(candidate)) = candidates.pop() {
        let Some(last) = candidate.path.last() else {
            continue;
        };

        if last == to {
            routes.push(candidate.path);
            if routes.len() as u64 >= limit {
                break;
            }
            continue;
        }

        if candidate.hops >= max_hops {
            continue;
        }

        let next_hops = candidate.hops + 1;
        for next in adjacency.adjacency.get(last).into_iter().flatten() {
            if candidate.path.contains(next) {
                continue;
            }

            let mut next_path = candidate.path.clone();
            next_path.push(next.clone());
            candidates.push(Reverse(CandidateRoute {
                hops: next_hops,
                path: next_path,
            }));
        }
    }

    match ordering {
        RoutesOrdering::FewestHops => routes,
        RoutesOrdering::Lexicographic => {
            routes.sort();
            routes
        }
    }
}

pub fn find_routes(
    session: &Session,
    from: &ObjectRef,
    to: &ObjectRef,
    limit: u64,
    max_hops: Option<u64>,
    ordering: RoutesOrdering,
) -> Vec<Vec<ObjectRef>> {
    let adjacency = SessionRouteAdjacency::derive(session);
    find_routes_with_adjacency(&adjacency, from, to, limit, max_hops, ordering)
}

pub fn find_route_with_adjacency(
    adjacency: &SessionRouteAdjacency,
    from: &ObjectRef,
    to: &ObjectRef,
) -> Option<Vec<ObjectRef>> {
    if from == to {
        return Some(vec![from.clone()]);
    }

    let start = from.clone();
    let goal = to.clone();

    let mut visited: BTreeSet<ObjectRef> = BTreeSet::new();
    let mut previous: BTreeMap<ObjectRef, ObjectRef> = BTreeMap::new();
    let mut queue: VecDeque<ObjectRef> = VecDeque::new();

    visited.insert(start.clone());
    queue.push_back(start.clone());

    while let Some(node) = queue.pop_front() {
        for next in adjacency.adjacency.get(&node).into_iter().flatten() {
            if visited.contains(next) {
                continue;
            }

            visited.insert(next.clone());
            previous.insert(next.clone(), node.clone());

            if next == &goal {
                return Some(reconstruct_path(goal.clone(), &start, &previous));
            }

            queue.push_back(next.clone());
        }
    }

    None
}

pub fn find_route(session: &Session, from: &ObjectRef, to: &ObjectRef) -> Option<Vec<ObjectRef>> {
    let adjacency = SessionRouteAdjacency::derive(session);
    find_route_with_adjacency(&adjacency, from, to)
}

#[cfg(test)]
mod tests {
    use super::{
        find_route, find_route_with_adjacency, find_routes, find_routes_with_adjacency,
        RoutesOrdering, SessionRouteAdjacency,
    };
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceSection, SequenceSectionKind,
    };
    use crate::model::{
        Diagram, DiagramAst, DiagramId, FlowEdge, FlowNode, FlowchartAst, ObjectRef, SequenceAst,
        SequenceMessage, SequenceMessageKind, Session, SessionId, XRef, XRefId, XRefStatus,
    };
    use crate::model::{ObjectId, SequenceParticipant};

    fn ref_strings(path: &[ObjectRef]) -> Vec<String> {
        path.iter().map(|r| r.to_string()).collect()
    }

    #[test]
    fn finds_simple_cross_diagram_route_via_xref() {
        let mut session = Session::new(SessionId::new("s1").expect("session id"));

        let flow_id = DiagramId::new("flow").expect("diagram id");
        let mut flow_ast = FlowchartAst::default();
        for node in ["n:a", "n:b"] {
            let node_id = ObjectId::new(node).expect("node id");
            flow_ast
                .nodes_mut()
                .insert(node_id, FlowNode::new(node.to_uppercase()));
        }
        let edge_id = ObjectId::new("e:ab").expect("edge id");
        let from = ObjectId::new("n:a").expect("from node id");
        let to = ObjectId::new("n:b").expect("to node id");
        flow_ast
            .edges_mut()
            .insert(edge_id, FlowEdge::new(from.clone(), to.clone()));

        session.diagrams_mut().insert(
            flow_id.clone(),
            Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
        );

        let seq_id = DiagramId::new("seq").expect("diagram id");
        let mut seq_ast = SequenceAst::default();
        let p1 = ObjectId::new("p:one").expect("participant id");
        let p2 = ObjectId::new("p:two").expect("participant id");
        seq_ast
            .participants_mut()
            .insert(p1.clone(), SequenceParticipant::new("One"));
        seq_ast
            .participants_mut()
            .insert(p2.clone(), SequenceParticipant::new("Two"));

        let m1 = ObjectId::new("m:1").expect("message id");
        let m2 = ObjectId::new("m:2").expect("message id");
        // Insert out of order to validate deterministic ordering by order_key + id.
        seq_ast.messages_mut().push(SequenceMessage::new(
            m2.clone(),
            p1.clone(),
            p2.clone(),
            SequenceMessageKind::Sync,
            "Second",
            200,
        ));
        seq_ast.messages_mut().push(SequenceMessage::new(
            m1.clone(),
            p1.clone(),
            p2.clone(),
            SequenceMessageKind::Sync,
            "First",
            100,
        ));

        session.diagrams_mut().insert(
            seq_id.clone(),
            Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)),
        );

        let xref_id = XRefId::new("x:1").expect("xref id");
        let xref_from: ObjectRef = "d:flow/flow/node/n:b".parse().expect("from ref");
        let xref_to: ObjectRef = "d:seq/seq/message/m:1".parse().expect("to ref");
        session.xrefs_mut().insert(
            xref_id,
            XRef::new(xref_from, xref_to, "relates", XRefStatus::Ok),
        );

        let start: ObjectRef = "d:flow/flow/node/n:a".parse().expect("start ref");
        let goal: ObjectRef = "d:seq/seq/message/m:2".parse().expect("goal ref");

        let route = find_route(&session, &start, &goal).expect("route");
        let adjacency = SessionRouteAdjacency::derive(&session);
        let cached_route =
            find_route_with_adjacency(&adjacency, &start, &goal).expect("cached route");
        assert_eq!(cached_route, route);

        assert_eq!(
            ref_strings(&route),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/node/n:b",
                "d:seq/seq/message/m:1",
                "d:seq/seq/message/m:2",
            ]
        );
    }

    #[test]
    fn finds_flow_edge_and_endpoint_node_routes() {
        let mut session = Session::new(SessionId::new("s-edge").expect("session id"));

        let flow_id = DiagramId::new("flow").expect("diagram id");
        let mut flow_ast = FlowchartAst::default();
        for node in ["n:a", "n:b"] {
            let node_id = ObjectId::new(node).expect("node id");
            flow_ast
                .nodes_mut()
                .insert(node_id, FlowNode::new(node.to_uppercase()));
        }

        flow_ast
            .edges_mut()
            .insert(oid("e:ab"), FlowEdge::new(oid("n:a"), oid("n:b")));

        session.diagrams_mut().insert(
            flow_id.clone(),
            Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
        );

        let edge: ObjectRef = "d:flow/flow/edge/e:ab".parse().expect("edge ref");
        let from: ObjectRef = "d:flow/flow/node/n:a".parse().expect("from node ref");
        let to: ObjectRef = "d:flow/flow/node/n:b".parse().expect("to node ref");

        assert_eq!(
            ref_strings(&find_route(&session, &edge, &from).expect("edge->from route")),
            vec!["d:flow/flow/edge/e:ab", "d:flow/flow/node/n:a"]
        );
        assert_eq!(
            ref_strings(&find_route(&session, &edge, &to).expect("edge->to route")),
            vec!["d:flow/flow/edge/e:ab", "d:flow/flow/node/n:b"]
        );
        assert_eq!(
            ref_strings(&find_route(&session, &from, &edge).expect("from->edge route")),
            vec!["d:flow/flow/node/n:a", "d:flow/flow/edge/e:ab"]
        );
        assert_eq!(
            ref_strings(&find_route(&session, &to, &edge).expect("to->edge route")),
            vec!["d:flow/flow/node/n:b", "d:flow/flow/edge/e:ab"]
        );
    }

    #[test]
    fn finds_seq_participant_and_message_routes() {
        let mut session = Session::new(SessionId::new("s-participant").expect("session id"));

        let seq_id = DiagramId::new("seq").expect("diagram id");
        let mut seq_ast = SequenceAst::default();
        let p1 = ObjectId::new("p:one").expect("participant id");
        let p2 = ObjectId::new("p:two").expect("participant id");
        seq_ast
            .participants_mut()
            .insert(p1.clone(), SequenceParticipant::new("One"));
        seq_ast
            .participants_mut()
            .insert(p2.clone(), SequenceParticipant::new("Two"));

        let m1 = ObjectId::new("m:1").expect("message id");
        let m2 = ObjectId::new("m:2").expect("message id");
        seq_ast.messages_mut().push(SequenceMessage::new(
            m1.clone(),
            p1.clone(),
            p2.clone(),
            SequenceMessageKind::Sync,
            "First",
            100,
        ));
        seq_ast.messages_mut().push(SequenceMessage::new(
            m2.clone(),
            p1.clone(),
            p2.clone(),
            SequenceMessageKind::Sync,
            "Second",
            200,
        ));

        session.diagrams_mut().insert(
            seq_id.clone(),
            Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)),
        );

        let participant: ObjectRef = "d:seq/seq/participant/p:one"
            .parse()
            .expect("participant ref");
        let message: ObjectRef = "d:seq/seq/message/m:2".parse().expect("message ref");
        assert_eq!(
            ref_strings(
                &find_route(&session, &participant, &message).expect("participant->message route")
            ),
            vec!["d:seq/seq/participant/p:one", "d:seq/seq/message/m:2"]
        );

        let participant_to: ObjectRef = "d:seq/seq/participant/p:two"
            .parse()
            .expect("participant ref");
        let message_from: ObjectRef = "d:seq/seq/message/m:1".parse().expect("message ref");
        assert_eq!(
            ref_strings(
                &find_route(&session, &message_from, &participant_to)
                    .expect("message->participant route")
            ),
            vec!["d:seq/seq/message/m:1", "d:seq/seq/participant/p:two"]
        );
    }

    #[test]
    fn finds_seq_block_to_message_route() {
        let mut session = Session::new(SessionId::new("s-block-route").expect("session id"));

        let seq_id = DiagramId::new("seq").expect("diagram id");
        let mut seq_ast = SequenceAst::default();
        let p1 = ObjectId::new("p:one").expect("participant id");
        let p2 = ObjectId::new("p:two").expect("participant id");
        seq_ast
            .participants_mut()
            .insert(p1.clone(), SequenceParticipant::new("One"));
        seq_ast
            .participants_mut()
            .insert(p2.clone(), SequenceParticipant::new("Two"));

        let m1 = ObjectId::new("m:1").expect("message id");
        seq_ast.messages_mut().push(SequenceMessage::new(
            m1.clone(),
            p1.clone(),
            p2.clone(),
            SequenceMessageKind::Sync,
            "First",
            100,
        ));

        seq_ast.blocks_mut().push(SequenceBlock::new(
            ObjectId::new("b:0000").expect("block id"),
            SequenceBlockKind::Alt,
            None,
            vec![SequenceSection::new(
                ObjectId::new("sec:0000:00").expect("section id"),
                SequenceSectionKind::Main,
                None,
                vec![m1.clone()],
            )],
            Vec::new(),
        ));

        session.diagrams_mut().insert(
            seq_id.clone(),
            Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)),
        );

        let block: ObjectRef = "d:seq/seq/block/b:0000".parse().expect("block ref");
        let message: ObjectRef = "d:seq/seq/message/m:1".parse().expect("message ref");
        assert_eq!(
            ref_strings(&find_route(&session, &block, &message).expect("block->message route")),
            vec![
                "d:seq/seq/block/b:0000",
                "d:seq/seq/section/sec:0000:00",
                "d:seq/seq/message/m:1",
            ]
        );
    }

    #[test]
    fn finds_cross_diagram_route_via_edge_and_participant_xref() {
        let mut session = Session::new(SessionId::new("s-xref").expect("session id"));

        let flow_id = DiagramId::new("flow").expect("diagram id");
        let mut flow_ast = FlowchartAst::default();
        for node in ["n:a", "n:b"] {
            let node_id = ObjectId::new(node).expect("node id");
            flow_ast
                .nodes_mut()
                .insert(node_id, FlowNode::new(node.to_uppercase()));
        }
        flow_ast
            .edges_mut()
            .insert(oid("e:ab"), FlowEdge::new(oid("n:a"), oid("n:b")));
        session.diagrams_mut().insert(
            flow_id.clone(),
            Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
        );

        let seq_id = DiagramId::new("seq").expect("diagram id");
        let mut seq_ast = SequenceAst::default();
        let p1 = ObjectId::new("p:one").expect("participant id");
        let p2 = ObjectId::new("p:two").expect("participant id");
        seq_ast
            .participants_mut()
            .insert(p1.clone(), SequenceParticipant::new("One"));
        seq_ast
            .participants_mut()
            .insert(p2.clone(), SequenceParticipant::new("Two"));

        let m1 = ObjectId::new("m:1").expect("message id");
        let m2 = ObjectId::new("m:2").expect("message id");
        seq_ast.messages_mut().push(SequenceMessage::new(
            m1.clone(),
            p1.clone(),
            p2.clone(),
            SequenceMessageKind::Sync,
            "First",
            100,
        ));
        seq_ast.messages_mut().push(SequenceMessage::new(
            m2.clone(),
            p1.clone(),
            p2.clone(),
            SequenceMessageKind::Sync,
            "Second",
            200,
        ));

        session.diagrams_mut().insert(
            seq_id.clone(),
            Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)),
        );

        let xref_id = XRefId::new("x:1").expect("xref id");
        let xref_from: ObjectRef = "d:flow/flow/edge/e:ab".parse().expect("from ref");
        let xref_to: ObjectRef = "d:seq/seq/participant/p:one".parse().expect("to ref");
        session.xrefs_mut().insert(
            xref_id,
            XRef::new(xref_from, xref_to, "relates", XRefStatus::Ok),
        );

        let start: ObjectRef = "d:flow/flow/node/n:a".parse().expect("start ref");
        let goal: ObjectRef = "d:seq/seq/message/m:2".parse().expect("goal ref");
        let route = find_route(&session, &start, &goal).expect("route");
        assert_eq!(
            ref_strings(&route),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/edge/e:ab",
                "d:seq/seq/participant/p:one",
                "d:seq/seq/message/m:2",
            ]
        );
    }

    #[test]
    fn finds_multiple_routes_deterministically_and_supports_ordering() {
        let mut session = Session::new(SessionId::new("s2").expect("session id"));

        let flow_id = DiagramId::new("flow").expect("diagram id");
        let mut flow_ast = FlowchartAst::default();
        for node in ["n:a", "n:b", "n:c", "n:d"] {
            let node_id = ObjectId::new(node).expect("node id");
            flow_ast
                .nodes_mut()
                .insert(node_id, FlowNode::new(node.to_uppercase()));
        }

        flow_ast
            .edges_mut()
            .insert(oid("e:ab"), FlowEdge::new(oid("n:a"), oid("n:b")));
        flow_ast
            .edges_mut()
            .insert(oid("e:ac"), FlowEdge::new(oid("n:a"), oid("n:c")));
        flow_ast
            .edges_mut()
            .insert(oid("e:bd"), FlowEdge::new(oid("n:b"), oid("n:d")));
        flow_ast
            .edges_mut()
            .insert(oid("e:cd"), FlowEdge::new(oid("n:c"), oid("n:d")));
        flow_ast
            .edges_mut()
            .insert(oid("e:bc"), FlowEdge::new(oid("n:b"), oid("n:c")));

        session.diagrams_mut().insert(
            flow_id.clone(),
            Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
        );

        let start: ObjectRef = "d:flow/flow/node/n:a".parse().expect("start ref");
        let goal: ObjectRef = "d:flow/flow/node/n:d".parse().expect("goal ref");

        let adjacency = SessionRouteAdjacency::derive(&session);
        let routes = find_routes_with_adjacency(
            &adjacency,
            &start,
            &goal,
            10,
            Some(3),
            RoutesOrdering::FewestHops,
        );
        let repeat = find_routes(
            &session,
            &start,
            &goal,
            10,
            Some(3),
            RoutesOrdering::FewestHops,
        );
        assert_eq!(routes, repeat);

        assert_eq!(routes.len(), 7);
        assert_eq!(
            ref_strings(&routes[0]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/node/n:b",
                "d:flow/flow/node/n:d",
            ]
        );
        assert_eq!(
            ref_strings(&routes[1]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/node/n:c",
                "d:flow/flow/node/n:d",
            ]
        );
        assert_eq!(
            ref_strings(&routes[2]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/edge/e:ab",
                "d:flow/flow/node/n:b",
                "d:flow/flow/node/n:d",
            ]
        );
        assert_eq!(
            ref_strings(&routes[3]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/edge/e:ac",
                "d:flow/flow/node/n:c",
                "d:flow/flow/node/n:d",
            ]
        );
        assert_eq!(
            ref_strings(&routes[4]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/node/n:b",
                "d:flow/flow/edge/e:bd",
                "d:flow/flow/node/n:d",
            ]
        );
        assert_eq!(
            ref_strings(&routes[5]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/node/n:b",
                "d:flow/flow/node/n:c",
                "d:flow/flow/node/n:d",
            ]
        );
        assert_eq!(
            ref_strings(&routes[6]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/node/n:c",
                "d:flow/flow/edge/e:cd",
                "d:flow/flow/node/n:d",
            ]
        );

        let lex_routes = find_routes(
            &session,
            &start,
            &goal,
            10,
            Some(3),
            RoutesOrdering::Lexicographic,
        );
        assert_eq!(
            ref_strings(&lex_routes[0]),
            vec![
                "d:flow/flow/node/n:a",
                "d:flow/flow/edge/e:ab",
                "d:flow/flow/node/n:b",
                "d:flow/flow/node/n:d",
            ]
        );
    }

    fn oid(raw: &str) -> ObjectId {
        ObjectId::new(raw).expect("object id")
    }
}
