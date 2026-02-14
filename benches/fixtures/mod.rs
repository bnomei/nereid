// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

#![allow(dead_code)]

// Shared deterministic benchmark fixtures (no RNG).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nereid::model::{
    CategoryPath, Diagram, DiagramAst, DiagramId, FlowEdge, FlowNode, FlowchartAst, ObjectId,
    ObjectRef, SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant, Session,
    SessionId, Walkthrough, WalkthroughEdge, WalkthroughId, WalkthroughNode, WalkthroughNodeId,
    XRef, XRefId, XRefStatus,
};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> Self {
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);

        let mut path = std::env::temp_dir();
        path.push(format!("nereid_bench_{prefix}_{pid}_{nanos}_{counter}"));
        std::fs::create_dir_all(&path).expect("create temp dir");

        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn ascii_repeat_to_len(prefix: &str, fill: char, target_len: usize) -> String {
    if prefix.len() >= target_len {
        return prefix[..target_len].to_owned();
    }

    let mut out = String::with_capacity(target_len);
    out.push_str(prefix);
    while out.len() < target_len {
        out.push(fill);
    }
    out
}

fn category_path_2(a: &'static str, b: &'static str) -> CategoryPath {
    CategoryPath::new(vec![a.to_owned(), b.to_owned()]).expect("valid category path")
}

pub fn checksum_flowchart(ast: &FlowchartAst) -> u64 {
    let mut acc = 0u64;
    for (node_id, node) in ast.nodes() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(node_id.as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(node.label().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(node.shape().len() as u64);
    }
    for (edge_id, edge) in ast.edges() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(edge_id.as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(edge.from_node_id().as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(edge.to_node_id().as_str().len() as u64);
        if let Some(label) = edge.label() {
            acc = acc.wrapping_mul(131).wrapping_add(label.len() as u64);
        }
        if let Some(style) = edge.style() {
            acc = acc.wrapping_mul(131).wrapping_add(style.len() as u64);
        }
    }
    acc
}

pub fn checksum_sequence(ast: &SequenceAst) -> u64 {
    let mut acc = 0u64;
    for (participant_id, participant) in ast.participants() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(participant_id.as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(participant.mermaid_name().len() as u64);
        if let Some(role) = participant.role() {
            acc = acc.wrapping_mul(131).wrapping_add(role.len() as u64);
        }
    }
    for msg in ast.messages() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(msg.message_id().as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(msg.from_participant_id().as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(msg.to_participant_id().as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add((msg.kind() as u8) as u64);
        acc = acc.wrapping_mul(131).wrapping_add(msg.text().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(msg.order_key().unsigned_abs());
    }
    for note in ast.notes() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(note.note_id().as_str().len() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(note.text().len() as u64);
    }
    acc
}

pub fn checksum_session(session: &Session) -> u64 {
    let mut acc = 0u64;
    acc = acc
        .wrapping_mul(131)
        .wrapping_add(session.session_id().as_str().len() as u64);

    for (diagram_id, diagram) in session.diagrams() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(diagram_id.as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(diagram.name().len() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(diagram.rev());
        acc = match diagram.ast() {
            DiagramAst::Flowchart(ast) => {
                acc.wrapping_mul(131).wrapping_add(checksum_flowchart(ast))
            }
            DiagramAst::Sequence(ast) => acc.wrapping_mul(131).wrapping_add(checksum_sequence(ast)),
        };
    }

    for (walkthrough_id, walkthrough) in session.walkthroughs() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(walkthrough_id.as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(walkthrough.title().len() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(walkthrough.rev());
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(walkthrough.nodes().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(walkthrough.edges().len() as u64);
    }

    for (xref_id, xref) in session.xrefs() {
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(xref_id.as_str().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(xref.from().to_string().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add(xref.to().to_string().len() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(xref.kind().len() as u64);
        acc = acc
            .wrapping_mul(131)
            .wrapping_add((xref.status() as u8) as u64);
        if let Some(label) = xref.label() {
            acc = acc.wrapping_mul(131).wrapping_add(label.len() as u64);
        }
    }

    acc
}

pub mod flow {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DagParams {
        pub layers: usize,
        pub nodes_per_layer: usize,
        pub fanout: usize,
        pub cross_edges_per_node: usize,
        pub label_len: usize,
    }

    impl DagParams {
        pub const fn new(
            layers: usize,
            nodes_per_layer: usize,
            fanout: usize,
            cross_edges_per_node: usize,
            label_len: usize,
        ) -> Self {
            Self {
                layers,
                nodes_per_layer,
                fanout,
                cross_edges_per_node,
                label_len,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Case {
        Small,
        MediumDense,
        LargeLongLabels,
    }

    impl Case {
        pub const fn id(self) -> &'static str {
            match self {
                Self::Small => "small",
                Self::MediumDense => "medium_dense",
                Self::LargeLongLabels => "large_long_labels",
            }
        }

        pub const fn params(self) -> DagParams {
            match self {
                Self::Small => DagParams::new(6, 10, 2, 0, 12),
                Self::MediumDense => DagParams::new(12, 20, 4, 1, 12),
                Self::LargeLongLabels => DagParams::new(24, 35, 4, 2, 64),
            }
        }
    }

    fn node_mermaid_id(layer: usize, idx: usize) -> String {
        format!("l{layer:02}_n{idx:04}")
    }

    fn node_id(layer: usize, idx: usize) -> ObjectId {
        let mermaid = node_mermaid_id(layer, idx);
        ObjectId::new(format!("n:{mermaid}")).expect("valid node id")
    }

    fn edge_id(index: usize) -> ObjectId {
        ObjectId::new(format!("e:{index:06}")).expect("valid edge id")
    }

    /// Deterministic layered DAG generator.
    ///
    /// - All edges go from lower → higher layers (acyclic by construction).
    /// - Node ids are Mermaid-compatible (exportable) and stable.
    pub fn dag(params: DagParams) -> FlowchartAst {
        assert!(params.layers >= 2, "layers must be >= 2");
        assert!(params.nodes_per_layer >= 1, "nodes_per_layer must be >= 1");
        assert!(params.fanout >= 1, "fanout must be >= 1");

        let mut ast = FlowchartAst::default();

        let mut node_ids = Vec::<Vec<ObjectId>>::with_capacity(params.layers);
        for layer in 0..params.layers {
            let mut layer_ids = Vec::<ObjectId>::with_capacity(params.nodes_per_layer);
            for idx in 0..params.nodes_per_layer {
                let id = node_id(layer, idx);
                let base = format!("Node_{}", node_mermaid_id(layer, idx));
                let label = ascii_repeat_to_len(&base, 'x', params.label_len);
                ast.nodes_mut().insert(id.clone(), FlowNode::new(label));
                layer_ids.push(id);
            }
            node_ids.push(layer_ids);
        }

        let mut next_edge = 0usize;
        let fanout = params.fanout.min(params.nodes_per_layer);

        for layer in 0..params.layers.saturating_sub(1) {
            for idx in 0..params.nodes_per_layer {
                let from = node_ids[layer][idx].clone();

                for k in 0..fanout {
                    let to_idx = (idx + k) % params.nodes_per_layer;
                    let to = node_ids[layer + 1][to_idx].clone();
                    ast.edges_mut()
                        .insert(edge_id(next_edge), FlowEdge::new(from.clone(), to));
                    next_edge += 1;
                }

                if layer + 2 >= params.layers {
                    continue;
                }
                let max_target_layers = params.layers - (layer + 2);
                for k in 0..params.cross_edges_per_node {
                    let target_layer = layer + 2 + (k % max_target_layers);
                    let to_idx = (idx + 1 + k.saturating_mul(3)) % params.nodes_per_layer;
                    let to = node_ids[target_layer][to_idx].clone();
                    ast.edges_mut()
                        .insert(edge_id(next_edge), FlowEdge::new(from.clone(), to));
                    next_edge += 1;
                }
            }
        }

        ast
    }

    pub fn fixture(case: Case) -> FlowchartAst {
        dag(case.params())
    }
}

pub mod seq {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Params {
        pub participants: usize,
        pub messages: usize,
        pub long_text: bool,
    }

    impl Params {
        pub const fn new(participants: usize, messages: usize, long_text: bool) -> Self {
            Self {
                participants,
                messages,
                long_text,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Case {
        Small,
        SmallLongText,
        Medium,
        LargeLongText,
    }

    impl Case {
        pub const fn id(self) -> &'static str {
            match self {
                Self::Small => "small",
                Self::SmallLongText => "small_long_text",
                Self::Medium => "medium",
                Self::LargeLongText => "large_long_text",
            }
        }

        pub const fn params(self) -> Params {
            match self {
                Self::Small => Params::new(8, 40, false),
                Self::SmallLongText => Params::new(8, 40, true),
                Self::Medium => Params::new(20, 200, false),
                Self::LargeLongText => Params::new(40, 800, true),
            }
        }
    }

    fn participant_name(idx: usize) -> String {
        format!("P{idx:03}")
    }

    fn participant_id(name: &str) -> ObjectId {
        ObjectId::new(format!("p:{name}")).expect("valid participant id")
    }

    fn message_id(idx: usize) -> ObjectId {
        ObjectId::new(format!("m:{idx:06}")).expect("valid message id")
    }

    fn message_text(idx: usize, long_text: bool) -> String {
        if long_text {
            let prefix = format!("msg_{idx:06}_");
            ascii_repeat_to_len(&prefix, 'y', 160)
        } else {
            format!("m{idx:04}")
        }
    }

    pub fn diagram(params: Params) -> SequenceAst {
        assert!(params.participants >= 2, "participants must be >= 2");

        let mut ast = SequenceAst::default();

        let mut participant_ids = Vec::<ObjectId>::with_capacity(params.participants);
        for idx in 0..params.participants {
            let name = participant_name(idx);
            let id = participant_id(&name);
            ast.participants_mut()
                .insert(id.clone(), SequenceParticipant::new(name));
            participant_ids.push(id);
        }

        for idx in 0..params.messages {
            let from = participant_ids[idx % params.participants].clone();
            let to = participant_ids[(idx + 1) % params.participants].clone();
            let kind = match idx % 3 {
                0 => SequenceMessageKind::Sync,
                1 => SequenceMessageKind::Async,
                _ => SequenceMessageKind::Return,
            };
            let text = message_text(idx, params.long_text);
            let order_key = (idx as i64) * 1000;
            ast.messages_mut().push(SequenceMessage::new(
                message_id(idx),
                from,
                to,
                kind,
                text,
                order_key,
            ));
        }

        ast
    }

    pub fn fixture(case: Case) -> SequenceAst {
        diagram(case.params())
    }
}

pub mod session {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Params {
        pub flow_diagrams: usize,
        pub seq_diagrams: usize,
        pub flow: flow::DagParams,
        pub seq: seq::Params,
        pub include_walkthroughs: bool,
        pub include_xrefs: bool,
    }

    impl Params {
        pub const fn new(
            flow_diagrams: usize,
            seq_diagrams: usize,
            flow: flow::DagParams,
            seq: seq::Params,
            include_walkthroughs: bool,
            include_xrefs: bool,
        ) -> Self {
            Self {
                flow_diagrams,
                seq_diagrams,
                flow,
                seq,
                include_walkthroughs,
                include_xrefs,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Case {
        SessionSmall,
        SessionMedium,
        SessionLarge,
        Session25Touch1,
    }

    impl Case {
        pub const fn id(self) -> &'static str {
            match self {
                Self::SessionSmall => "session_small",
                Self::SessionMedium => "session_medium",
                Self::SessionLarge => "session_large",
                Self::Session25Touch1 => "session_25_touch_1",
            }
        }

        pub const fn params(self) -> Params {
            match self {
                Self::SessionSmall => Params::new(
                    1,
                    1,
                    flow::Case::Small.params(),
                    seq::Case::Small.params(),
                    false,
                    false,
                ),
                Self::SessionMedium => Params::new(
                    4,
                    3,
                    flow::Case::MediumDense.params(),
                    seq::Case::Medium.params(),
                    true,
                    true,
                ),
                Self::SessionLarge => Params::new(
                    10,
                    8,
                    flow::Case::LargeLongLabels.params(),
                    seq::Case::LargeLongText.params(),
                    true,
                    true,
                ),
                Self::Session25Touch1 => Params::new(
                    13,
                    12,
                    flow::Case::Small.params(),
                    seq::Case::Small.params(),
                    true,
                    true,
                ),
            }
        }
    }

    fn diagram_id(kind: &str, idx: usize) -> DiagramId {
        DiagramId::new(format!("{kind}_{idx:03}")).expect("valid diagram id")
    }

    fn walkthrough_id(idx: usize) -> WalkthroughId {
        WalkthroughId::new(format!("wt_{idx:03}")).expect("valid walkthrough id")
    }

    fn walkthrough_node_id(idx: usize) -> WalkthroughNodeId {
        WalkthroughNodeId::new(format!("wtn_{idx:04}")).expect("valid walkthrough node id")
    }

    fn xref_id(idx: usize) -> XRefId {
        XRefId::new(format!("xref_{idx:04}")).expect("valid xref id")
    }

    fn first_flow_node_ref(diagram_id: &DiagramId, ast: &FlowchartAst) -> Option<ObjectRef> {
        let node_id = ast.nodes().keys().next()?.clone();
        Some(ObjectRef::new(
            diagram_id.clone(),
            category_path_2("flow", "node"),
            node_id,
        ))
    }

    fn first_seq_participant_ref(diagram_id: &DiagramId, ast: &SequenceAst) -> Option<ObjectRef> {
        let participant_id = ast.participants().keys().next()?.clone();
        Some(ObjectRef::new(
            diagram_id.clone(),
            category_path_2("seq", "participant"),
            participant_id,
        ))
    }

    pub fn build(case_id: &'static str, params: Params) -> Session {
        let session_id = SessionId::new(case_id).expect("valid session id");
        let mut session = Session::new(session_id);

        let mut flow_diagram_ids = Vec::<DiagramId>::with_capacity(params.flow_diagrams);
        for idx in 0..params.flow_diagrams {
            let id = diagram_id("flow", idx);
            let ast = flow::dag(params.flow);
            let diagram = Diagram::new(
                id.clone(),
                format!("Flow {idx:03}"),
                DiagramAst::Flowchart(ast),
            );
            session.diagrams_mut().insert(id.clone(), diagram);
            flow_diagram_ids.push(id);
        }

        let mut seq_diagram_ids = Vec::<DiagramId>::with_capacity(params.seq_diagrams);
        for idx in 0..params.seq_diagrams {
            let id = diagram_id("seq", idx);
            let ast = seq::diagram(params.seq);
            let diagram = Diagram::new(
                id.clone(),
                format!("Seq {idx:03}"),
                DiagramAst::Sequence(ast),
            );
            session.diagrams_mut().insert(id.clone(), diagram);
            seq_diagram_ids.push(id);
        }

        if let Some(first) = session.diagrams().keys().next().cloned() {
            session.set_active_diagram_id(Some(first));
        }

        if params.include_walkthroughs {
            let wt_id = walkthrough_id(0);
            let mut wt = Walkthrough::new(wt_id.clone(), "Walkthrough");

            let n0 = walkthrough_node_id(0);
            let n1 = walkthrough_node_id(1);
            let n2 = walkthrough_node_id(2);

            wt.nodes_mut()
                .push(WalkthroughNode::new(n0.clone(), "Start"));
            wt.nodes_mut()
                .push(WalkthroughNode::new(n1.clone(), "Middle"));
            wt.nodes_mut().push(WalkthroughNode::new(n2.clone(), "End"));

            wt.edges_mut()
                .push(WalkthroughEdge::new(n0.clone(), n1.clone(), "next"));
            wt.edges_mut()
                .push(WalkthroughEdge::new(n1.clone(), n2.clone(), "next"));

            session.walkthroughs_mut().insert(wt_id.clone(), wt);
            session.set_active_walkthrough_id(Some(wt_id));
        }

        if params.include_xrefs {
            let mut next_xref = 0usize;

            if let (Some(flow_id), Some(seq_id)) =
                (flow_diagram_ids.first(), seq_diagram_ids.first())
            {
                if let (Some(flow_diagram), Some(seq_diagram)) = (
                    session.diagrams().get(flow_id),
                    session.diagrams().get(seq_id),
                ) {
                    let from = match flow_diagram.ast() {
                        DiagramAst::Flowchart(ast) => first_flow_node_ref(flow_id, ast),
                        _ => None,
                    };
                    let to = match seq_diagram.ast() {
                        DiagramAst::Sequence(ast) => first_seq_participant_ref(seq_id, ast),
                        _ => None,
                    };

                    if let (Some(from), Some(to)) = (from, to) {
                        let xref = XRef::new(from, to, "links", XRefStatus::Ok);
                        session.xrefs_mut().insert(xref_id(next_xref), xref);
                        next_xref += 1;
                    }
                }
            }

            // Deterministic intra-diagram xref (flow node → flow node) if possible.
            if next_xref < 2 {
                if let Some(flow_id) = flow_diagram_ids.first() {
                    if let Some(diagram) = session.diagrams().get(flow_id) {
                        if let DiagramAst::Flowchart(ast) = diagram.ast() {
                            let mut nodes = ast.nodes().keys();
                            let a = nodes.next().cloned();
                            let b = nodes.next().cloned();
                            if let (Some(a), Some(b)) = (a, b) {
                                let from = ObjectRef::new(
                                    flow_id.clone(),
                                    category_path_2("flow", "node"),
                                    a,
                                );
                                let to = ObjectRef::new(
                                    flow_id.clone(),
                                    category_path_2("flow", "node"),
                                    b,
                                );
                                let xref = XRef::new(from, to, "next", XRefStatus::Ok);
                                session.xrefs_mut().insert(xref_id(next_xref), xref);
                            }
                        }
                    }
                }
            }
        }

        session
    }

    pub fn fixture(case: Case) -> Session {
        build(case.id(), case.params())
    }
}
