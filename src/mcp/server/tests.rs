// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::*;
use crate::model::{
    seq_ast::{SequenceBlock, SequenceBlockKind, SequenceSection, SequenceSectionKind},
    DiagramAst, FlowEdge, FlowNode, FlowchartAst, ObjectRef, SequenceAst, SequenceMessage,
    SequenceMessageKind, SequenceParticipant, SessionId, Walkthrough, WalkthroughEdge,
    WalkthroughId, WalkthroughNode, WalkthroughNodeId, XRef, XRefId, XRefStatus,
};
use std::str::FromStr;

fn temp_session_dir(test_name: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut dir = std::env::temp_dir();
    let pid = std::process::id();
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).expect("clock is monotonic").as_nanos();
    dir.push(format!("nereid-{test_name}-{pid}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn oid(value: &str) -> ObjectId {
    ObjectId::new(value).expect("object id")
}

fn oref(diagram_id: &str, object_id: &str) -> ObjectRef {
    ObjectRef::from_str(&format!("d:{diagram_id}/obj/{object_id}")).expect("object ref")
}

fn xref_list_params() -> XRefListParams {
    XRefListParams {
        dangling_only: None,
        status: None,
        kind: None,
        from_ref: None,
        to_ref: None,
        involves_ref: None,
        label_contains: None,
        limit: None,
    }
}

fn demo_session() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-demo").expect("session id"));

    let seq_id = DiagramId::new("d-seq").expect("diagram id");
    let mut seq_ast = SequenceAst::default();
    let p_a = oid("p:a");
    let p_b = oid("p:b");
    seq_ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
    seq_ast.participants_mut().insert(p_b.clone(), SequenceParticipant::new("B"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:1"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Hi",
        1000,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)));

    let flow_id = DiagramId::new("d-flow").expect("diagram id");
    let mut flow_ast = FlowchartAst::default();
    let n_a = oid("n:a");
    let n_b = oid("n:b");
    flow_ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
    flow_ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a, n_b));
    session
        .diagrams_mut()
        .insert(flow_id.clone(), Diagram::new(flow_id, "Flow", DiagramAst::Flowchart(flow_ast)));

    session.set_active_diagram_id(Some(seq_id));
    session
}

fn demo_session_with_seq_blocks() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-seq-blocks").expect("session id"));

    let seq_id = DiagramId::new("d-seq-blocks").expect("diagram id");
    let mut seq_ast = SequenceAst::default();
    let p_a = oid("p:a");
    let p_b = oid("p:b");
    seq_ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
    seq_ast.participants_mut().insert(p_b.clone(), SequenceParticipant::new("B"));

    let m_main = oid("m:1");
    let m_else = oid("m:2");
    seq_ast.messages_mut().push(SequenceMessage::new(
        m_main.clone(),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Main",
        1000,
    ));
    seq_ast.messages_mut().push(SequenceMessage::new(
        m_else.clone(),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Else",
        2000,
    ));

    seq_ast.blocks_mut().push(SequenceBlock::new(
        oid("b:0000"),
        SequenceBlockKind::Alt,
        Some("guard".to_owned()),
        vec![
            SequenceSection::new(
                oid("sec:0000:00"),
                SequenceSectionKind::Main,
                Some("ok".to_owned()),
                vec![m_main],
            ),
            SequenceSection::new(
                oid("sec:0000:01"),
                SequenceSectionKind::Else,
                Some("fallback".to_owned()),
                vec![m_else],
            ),
        ],
        Vec::new(),
    ));

    session.diagrams_mut().insert(
        seq_id.clone(),
        Diagram::new(seq_id.clone(), "Seq Blocks", DiagramAst::Sequence(seq_ast)),
    );
    session.set_active_diagram_id(Some(seq_id));
    session
}

fn demo_session_for_seq_trace() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-seq-trace").expect("session id"));

    let seq_id = DiagramId::new("d-seq-trace").expect("diagram id");
    let mut seq_ast = SequenceAst::default();
    let p_a = oid("p:a");
    let p_b = oid("p:b");
    seq_ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
    seq_ast.participants_mut().insert(p_b.clone(), SequenceParticipant::new("B"));

    // Intentionally insert out of order to validate deterministic ordering.
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:0003"),
        p_b.clone(),
        p_a.clone(),
        SequenceMessageKind::Return,
        "Third",
        2000,
    ));
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:0002"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "First",
        1000,
    ));
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:0001"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Second",
        2000,
    ));
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:0004"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Async,
        "Fourth",
        3000,
    ));

    session.diagrams_mut().insert(
        seq_id.clone(),
        Diagram::new(seq_id.clone(), "Seq Trace", DiagramAst::Sequence(seq_ast)),
    );

    session.set_active_diagram_id(Some(seq_id));
    session
}

fn demo_session_for_flow_reachable() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-flow-reachable").expect("session id"));

    let flow_id = DiagramId::new("d-flow-reach").expect("diagram id");
    let mut flow_ast = FlowchartAst::default();
    let n_a = oid("n:a");
    let n_b = oid("n:b");
    let n_c = oid("n:c");
    flow_ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
    flow_ast.nodes_mut().insert(n_c.clone(), FlowNode::new("C"));
    flow_ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));
    flow_ast.edges_mut().insert(oid("e:bc"), FlowEdge::new(n_b.clone(), n_c.clone()));

    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow Reach", DiagramAst::Flowchart(flow_ast)),
    );

    session.set_active_diagram_id(Some(flow_id));
    session
}

fn demo_session_for_flow_paths() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-flow-paths").expect("session id"));

    let flow_id = DiagramId::new("d-flow-paths").expect("diagram id");
    let flow_ast = crate::model::fixtures::flowchart_small_dag();

    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow Paths", DiagramAst::Flowchart(flow_ast)),
    );

    session.set_active_diagram_id(Some(flow_id));
    session
}

fn demo_session_for_flow_degrees() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-flow-degrees").expect("session id"));

    let flow_id = DiagramId::new("d-flow-degrees").expect("diagram id");
    let mut flow_ast = FlowchartAst::default();
    let n_a = oid("n:a");
    let n_b = oid("n:b");
    let n_c = oid("n:c");
    let n_d = oid("n:d");
    flow_ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
    flow_ast.nodes_mut().insert(n_c.clone(), FlowNode::new("C"));
    flow_ast.nodes_mut().insert(n_d.clone(), FlowNode::new("D"));
    flow_ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));
    flow_ast.edges_mut().insert(oid("e:ac"), FlowEdge::new(n_a.clone(), n_c.clone()));
    flow_ast.edges_mut().insert(oid("e:ad"), FlowEdge::new(n_a, n_d.clone()));
    flow_ast.edges_mut().insert(oid("e:cb"), FlowEdge::new(n_c, n_b.clone()));
    flow_ast.edges_mut().insert(oid("e:db"), FlowEdge::new(n_d, n_b));

    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow Degrees", DiagramAst::Flowchart(flow_ast)),
    );

    session.set_active_diagram_id(Some(flow_id));
    session
}

fn demo_session_for_flow_cycles() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-flow-cycles").expect("session id"));

    let seq_id = DiagramId::new("d-seq-cycles").expect("diagram id");
    session.diagrams_mut().insert(
        seq_id.clone(),
        Diagram::new(seq_id, "Seq", DiagramAst::Sequence(SequenceAst::default())),
    );

    let flow_id = DiagramId::new("d-flow-cycles").expect("diagram id");
    let mut flow_ast = FlowchartAst::default();
    flow_ast.nodes_mut().insert(oid("n:e"), FlowNode::new("E"));
    flow_ast.nodes_mut().insert(oid("n:f"), FlowNode::new("F"));
    flow_ast.nodes_mut().insert(oid("n:x"), FlowNode::new("X"));
    flow_ast.nodes_mut().insert(oid("n:y"), FlowNode::new("Y"));
    flow_ast.nodes_mut().insert(oid("n:z"), FlowNode::new("Z"));

    flow_ast.edges_mut().insert(oid("e:xy"), FlowEdge::new(oid("n:x"), oid("n:y")));
    flow_ast.edges_mut().insert(oid("e:yx"), FlowEdge::new(oid("n:y"), oid("n:x")));
    flow_ast.edges_mut().insert(oid("e:zz"), FlowEdge::new(oid("n:z"), oid("n:z")));

    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow Cycles", DiagramAst::Flowchart(flow_ast)),
    );

    session.set_active_diagram_id(Some(flow_id));
    session
}

fn demo_session_for_flow_unreachable() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-flow-unreachable").expect("session id"));

    let flow_id = DiagramId::new("d-flow-unreach").expect("diagram id");
    let mut flow_ast = FlowchartAst::default();

    // Insert intentionally out of order to validate deterministic ordering.
    flow_ast.nodes_mut().insert(oid("n:y"), FlowNode::new("Y"));
    flow_ast.nodes_mut().insert(oid("n:a"), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(oid("n:x"), FlowNode::new("X"));
    flow_ast.nodes_mut().insert(oid("n:c"), FlowNode::new("C"));
    flow_ast.nodes_mut().insert(oid("n:b"), FlowNode::new("B"));

    flow_ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(oid("n:a"), oid("n:b")));
    flow_ast.edges_mut().insert(oid("e:bc"), FlowEdge::new(oid("n:b"), oid("n:c")));
    flow_ast.edges_mut().insert(oid("e:xy"), FlowEdge::new(oid("n:x"), oid("n:y")));
    flow_ast.edges_mut().insert(oid("e:yx"), FlowEdge::new(oid("n:y"), oid("n:x")));

    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow Unreachable", DiagramAst::Flowchart(flow_ast)),
    );

    session.set_active_diagram_id(Some(flow_id));
    session
}

fn demo_session_with_xrefs() -> Session {
    let mut session = demo_session();

    session.xrefs_mut().insert(
        XRefId::new("x:2").expect("xref id"),
        XRef::new(oref("d-seq", "p:a"), oref("d-flow", "n:a"), "relates_to", XRefStatus::Ok),
    );

    session.xrefs_mut().insert(
        XRefId::new("x:1").expect("xref id"),
        XRef::new(
            oref("d-seq", "p:b"),
            oref("d-flow", "n:missing"),
            "relates_to",
            XRefStatus::DanglingTo,
        ),
    );

    session
}

fn demo_session_with_xrefs_varied() -> Session {
    let mut session = demo_session();

    let mut x2 =
        XRef::new(oref("d-seq", "p:a"), oref("d-flow", "n:a"), "relates_to", XRefStatus::Ok);
    x2.set_label(Some("Alpha".to_owned()));
    session.xrefs_mut().insert(XRefId::new("x:2").expect("xref id"), x2);

    let mut x1 = XRef::new(
        oref("d-seq", "p:b"),
        oref("d-flow", "n:missing"),
        "relates_to",
        XRefStatus::DanglingTo,
    );
    x1.set_label(Some("Beta".to_owned()));
    session.xrefs_mut().insert(XRefId::new("x:1").expect("xref id"), x1);

    let mut x3 = XRef::new(
        oref("d-seq", "p:a"),
        oref("d-flow", "n:b"),
        "implements",
        XRefStatus::DanglingFrom,
    );
    x3.set_label(Some("Auth step".to_owned()));
    session.xrefs_mut().insert(XRefId::new("x:3").expect("xref id"), x3);

    session
}

fn demo_session_with_neighbors_xrefs() -> Session {
    let mut session = demo_session();

    session.xrefs_mut().insert(
        XRefId::new("x:n1").expect("xref id"),
        XRef::new(
            ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("from ref"),
            ObjectRef::from_str("d:d-flow/flow/node/n:a").expect("to ref"),
            "relates_to",
            XRefStatus::Ok,
        ),
    );

    session.xrefs_mut().insert(
        XRefId::new("x:n2").expect("xref id"),
        XRef::new(
            ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("from ref"),
            ObjectRef::from_str("d:d-flow/flow/node/n:b").expect("to ref"),
            "relates_to",
            XRefStatus::Ok,
        ),
    );

    session.xrefs_mut().insert(
        XRefId::new("x:n3").expect("xref id"),
        XRef::new(
            ObjectRef::from_str("d:d-flow/flow/node/n:a").expect("from ref"),
            ObjectRef::from_str("d:d-seq/seq/participant/p:b").expect("to ref"),
            "relates_to",
            XRefStatus::Ok,
        ),
    );

    session
}

fn demo_session_with_route() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-route").expect("session id"));

    let seq_id = DiagramId::new("d-seq").expect("diagram id");
    let mut seq_ast = SequenceAst::default();
    let p_a = oid("p:a");
    let p_b = oid("p:b");
    seq_ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
    seq_ast.participants_mut().insert(p_b.clone(), SequenceParticipant::new("B"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:1"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "First",
        1000,
    ));
    seq_ast.messages_mut().push(SequenceMessage::new(
        oid("m:2"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Second",
        2000,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)));

    let flow_id = DiagramId::new("d-flow").expect("diagram id");
    let mut flow_ast = FlowchartAst::default();
    let n_a = oid("n:a");
    let n_b = oid("n:b");
    flow_ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
    flow_ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a, n_b));
    let flow_diagram = Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast));
    session.diagrams_mut().insert(flow_id, flow_diagram);

    session.xrefs_mut().insert(
        XRefId::new("x:route").expect("xref id"),
        XRef::new(
            "d:d-flow/flow/node/n:b".parse().expect("from ref"),
            "d:d-seq/seq/message/m:1".parse().expect("to ref"),
            "relates_to",
            XRefStatus::Ok,
        ),
    );

    session
}

fn demo_session_with_multiple_routes() -> Session {
    let mut session = demo_session_with_route();

    session.xrefs_mut().insert(
        XRefId::new("x:route-direct").expect("xref id"),
        XRef::new(
            "d:d-flow/flow/node/n:a".parse().expect("from ref"),
            "d:d-seq/seq/message/m:1".parse().expect("to ref"),
            "relates_to",
            XRefStatus::Ok,
        ),
    );

    session
}

fn demo_session_with_walkthroughs() -> Session {
    let mut session = Session::new(SessionId::new("s:mcp-walkthroughs").expect("session id"));

    let mut wt_2 =
        Walkthrough::new(WalkthroughId::new("w:2").expect("walkthrough id"), "Second walkthrough");
    wt_2.bump_rev();
    wt_2.bump_rev();
    wt_2.nodes_mut().push(WalkthroughNode::new(
        WalkthroughNodeId::new("wn:1").expect("walkthrough node id"),
        "Only node",
    ));

    let mut wt_1 =
        Walkthrough::new(WalkthroughId::new("w:1").expect("walkthrough id"), "First walkthrough");
    let w1_n1 = WalkthroughNodeId::new("wn:2").expect("walkthrough node id");
    let w1_n2 = WalkthroughNodeId::new("wn:3").expect("walkthrough node id");
    let mut start = WalkthroughNode::new(w1_n1.clone(), "Start");
    start.set_body_md(Some("Start body".to_owned()));
    start.refs_mut().push(ObjectRef::from_str("d:d-seq/seq/message/m:1").expect("ref"));
    start.refs_mut().push(ObjectRef::from_str("d:d-flow/flow/node/n:a").expect("ref"));
    start.tags_mut().push("intro".to_owned());
    start.tags_mut().push("evidence".to_owned());
    start.set_status(Some("draft".to_owned()));
    wt_1.nodes_mut().push(start);

    let mut end = WalkthroughNode::new(w1_n2.clone(), "End");
    end.refs_mut().push(ObjectRef::from_str("d:d-flow/flow/edge/e:ab").expect("ref"));
    wt_1.nodes_mut().push(end);

    let mut edge = WalkthroughEdge::new(w1_n1, w1_n2, "next");
    edge.set_label(Some("continue".to_owned()));
    wt_1.edges_mut().push(edge);

    session.walkthroughs_mut().insert(wt_2.walkthrough_id().clone(), wt_2);
    session.walkthroughs_mut().insert(wt_1.walkthrough_id().clone(), wt_1);

    session
}

#[test]
fn tools_advertise_descriptions_and_schemas() {
    let tools = NereidMcp::tool_router().list_all();
    assert!(!tools.is_empty(), "expected at least one tool");

    let mut missing_description = Vec::new();
    let mut missing_output_schema = Vec::new();
    let mut non_object_input_schema = Vec::new();
    let mut non_object_output_schema = Vec::new();

    let mut seen_names = BTreeSet::new();

    for tool in tools {
        let name = tool.name.to_string();
        assert!(seen_names.insert(name.clone()), "duplicate tool name: {name}");

        let desc_missing =
            tool.description.as_deref().map(|desc| desc.trim().is_empty()).unwrap_or(true);
        if desc_missing {
            missing_description.push(name.clone());
        }

        if tool.input_schema.get("type").and_then(|v| v.as_str()) != Some("object") {
            non_object_input_schema.push(name.clone());
        }

        match tool.output_schema.as_ref() {
            None => missing_output_schema.push(name.clone()),
            Some(schema) => {
                if schema.get("type").and_then(|v| v.as_str()) != Some("object") {
                    non_object_output_schema.push(name.clone());
                }
            }
        }
    }

    assert!(missing_description.is_empty(), "tools missing description: {missing_description:?}");
    assert!(
        missing_output_schema.is_empty(),
        "tools missing output_schema: {missing_output_schema:?}"
    );
    assert!(
        non_object_input_schema.is_empty(),
        "tools with non-object input_schema: {non_object_input_schema:?}"
    );
    assert!(
        non_object_output_schema.is_empty(),
        "tools with non-object output_schema: {non_object_output_schema:?}"
    );
}

#[tokio::test]
async fn attention_human_and_follow_ai_read_return_stable_defaults_without_ui_state() {
    let server = NereidMcp::new(demo_session());
    let Json(attention) = server.attention_human_read().await.expect("attention.human.read");

    assert_eq!(attention.object_ref, None);
    assert_eq!(attention.diagram_id, None);
    assert_eq!(attention.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(attention.context.human_active_diagram_id, None);
    assert_eq!(attention.context.human_active_object_ref, None);
    assert_eq!(attention.context.follow_ai, None);
    assert_eq!(attention.context.ui_rev, None);
    assert_eq!(attention.context.ui_session_rev, None);

    let Json(follow_ai) = server.follow_ai_read().await.expect("follow_ai.read");
    assert!(follow_ai.enabled);
    assert_eq!(follow_ai.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(follow_ai.context.human_active_diagram_id, None);
    assert_eq!(follow_ai.context.human_active_object_ref, None);
    assert_eq!(follow_ai.context.follow_ai, None);
    assert_eq!(follow_ai.context.ui_rev, None);
    assert_eq!(follow_ai.context.ui_session_rev, None);
}

#[tokio::test]
async fn follow_ai_set_updates_shared_ui_state_when_available() {
    let ui_state = Arc::new(Mutex::new(UiState::default()));
    let server = NereidMcp::new_with_agent_highlights_and_ui_state(
        demo_session(),
        Arc::new(Mutex::new(BTreeSet::new())),
        Some(ui_state.clone()),
    );

    let Json(initial) = server.follow_ai_read().await.expect("follow_ai.read initial");
    assert!(initial.enabled);
    assert_eq!(initial.context.follow_ai, Some(true));

    let Json(updated) = server
        .follow_ai_set(Parameters(FollowAiSetParams { enabled: false }))
        .await
        .expect("follow_ai.set");
    assert!(!updated.enabled);

    let Json(current) = server.follow_ai_read().await.expect("follow_ai.read current");
    assert!(!current.enabled);
    assert_eq!(current.context.follow_ai, Some(false));
    assert!(!ui_state.lock().await.follow_ai());
}

#[tokio::test]
async fn view_get_state_returns_stable_defaults() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server.view_get_state().await.expect("view state");

    assert_eq!(result.active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(result.scroll.x, 0.0);
    assert_eq!(result.scroll.y, 0.0);
    assert!(result.panes.is_empty());
    assert_eq!(result.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(result.context.human_active_diagram_id, None);
    assert_eq!(result.context.human_active_object_ref, None);
    assert_eq!(result.context.follow_ai, None);
    assert_eq!(result.context.ui_rev, None);
    assert_eq!(result.context.ui_session_rev, None);
}

#[tokio::test]
async fn selection_update_get_ignores_missing_and_is_deterministic() {
    let server = NereidMcp::new(demo_session());

    let Json(initial) = server.selection_get().await.expect("get initial selection");
    assert!(initial.object_refs.is_empty());
    assert_eq!(initial.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(initial.context.human_active_diagram_id, None);
    assert_eq!(initial.context.human_active_object_ref, None);
    assert_eq!(initial.context.follow_ai, None);
    assert_eq!(initial.context.ui_rev, None);
    assert_eq!(initial.context.ui_session_rev, None);

    let Json(result) = server
        .selection_update(Parameters(SelectionUpdateParams {
            object_refs: vec![
                "d:d-seq/seq/participant/p:a".to_owned(),
                "d:d-seq/seq/message/m:999".to_owned(),
                "d:d-flow/flow/edge/e:ab".to_owned(),
                "d:d-missing/seq/participant/p:a".to_owned(),
            ],
            mode: UpdateMode::Replace,
        }))
        .await
        .expect("set selection");

    assert_eq!(
        result.applied,
        vec!["d:d-flow/flow/edge/e:ab".to_owned(), "d:d-seq/seq/participant/p:a".to_owned(),]
    );
    assert_eq!(
        result.ignored,
        vec!["d:d-missing/seq/participant/p:a".to_owned(), "d:d-seq/seq/message/m:999".to_owned(),]
    );

    let Json(get) = server.selection_get().await.expect("get selection");
    assert_eq!(get.object_refs, result.applied);

    server
        .selection_update(Parameters(SelectionUpdateParams {
            object_refs: vec![
                "d:d-seq/seq/message/m:1".to_owned(),
                "d:d-seq/seq/message/m:missing".to_owned(),
            ],
            mode: UpdateMode::Add,
        }))
        .await
        .expect("add selection");

    let Json(get_after_add) = server.selection_get().await.expect("get selection after add");
    assert_eq!(
        get_after_add.object_refs,
        vec![
            "d:d-flow/flow/edge/e:ab".to_owned(),
            "d:d-seq/seq/message/m:1".to_owned(),
            "d:d-seq/seq/participant/p:a".to_owned(),
        ]
    );

    server
        .selection_update(Parameters(SelectionUpdateParams {
            object_refs: vec!["d:d-seq/seq/participant/p:a".to_owned()],
            mode: UpdateMode::Remove,
        }))
        .await
        .expect("remove selection");

    let Json(get_after_remove) = server.selection_get().await.expect("get selection after remove");
    assert_eq!(
        get_after_remove.object_refs,
        vec!["d:d-flow/flow/edge/e:ab".to_owned(), "d:d-seq/seq/message/m:1".to_owned(),]
    );
}

#[tokio::test]
async fn selection_get_includes_human_active_diagram_when_ui_state_is_shared() {
    let ui_state = Arc::new(Mutex::new(UiState::default()));
    ui_state
        .lock()
        .await
        .set_human_selection(Some(DiagramId::new("d-flow").expect("diagram id")), None);

    let server = NereidMcp::new_with_agent_highlights_and_ui_state(
        demo_session(),
        Arc::new(Mutex::new(BTreeSet::new())),
        Some(ui_state),
    );

    let Json(selection) = server.selection_get().await.expect("selection.read");
    assert_eq!(selection.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(selection.context.human_active_diagram_id.as_deref(), Some("d-flow"));
    assert_eq!(selection.context.human_active_object_ref, None);
    assert_eq!(selection.context.follow_ai, Some(true));
    assert_eq!(selection.context.ui_rev, Some(1));
    assert_eq!(selection.context.ui_session_rev, Some(0));
}

#[tokio::test]
async fn attention_agent_set_read_clear_validates_refs() {
    let server = NereidMcp::new(demo_session());

    let Json(initial) = server.attention_agent_read().await.expect("attention.agent.read initial");
    assert_eq!(initial.object_ref, None);
    assert_eq!(initial.diagram_id, None);
    assert_eq!(initial.context.session_active_diagram_id.as_deref(), Some("d-seq"));

    let err = match server
        .attention_agent_set(Parameters(AttentionAgentSetParams {
            object_ref: "d:d-seq/seq/message/m:999".to_owned(),
        }))
        .await
    {
        Ok(_) => panic!("attention.agent.set should reject missing object"),
        Err(err) => err,
    };
    assert!(err.message.contains("object not found"));

    let object_ref = "d:d-flow/flow/edge/e:ab".to_owned();
    let Json(set) = server
        .attention_agent_set(Parameters(AttentionAgentSetParams { object_ref: object_ref.clone() }))
        .await
        .expect("attention.agent.set");
    assert_eq!(set.object_ref, object_ref);
    assert_eq!(set.diagram_id, "d-flow");

    let Json(read) = server.attention_agent_read().await.expect("attention.agent.read");
    assert_eq!(read.object_ref.as_deref(), Some("d:d-flow/flow/edge/e:ab"));
    assert_eq!(read.diagram_id.as_deref(), Some("d-flow"));
    assert_eq!(read.context.session_active_diagram_id.as_deref(), Some("d-seq"));

    let Json(cleared) = server.attention_agent_clear().await.expect("attention.agent.clear");
    assert_eq!(cleared.cleared, 1);

    let Json(after_clear) =
        server.attention_agent_read().await.expect("attention.agent.read after clear");
    assert_eq!(after_clear.object_ref, None);
    assert_eq!(after_clear.diagram_id, None);
    assert_eq!(after_clear.context.session_active_diagram_id.as_deref(), Some("d-seq"));
}

#[tokio::test]
async fn attention_agent_set_overwrites_previous_value() {
    let server = NereidMcp::new(demo_session());

    let Json(first) = server
        .attention_agent_set(Parameters(AttentionAgentSetParams {
            object_ref: "d:d-seq/seq/participant/p:a".to_owned(),
        }))
        .await
        .expect("attention.agent.set first");
    assert_eq!(first.object_ref, "d:d-seq/seq/participant/p:a");
    assert_eq!(first.diagram_id, "d-seq");

    let Json(second) = server
        .attention_agent_set(Parameters(AttentionAgentSetParams {
            object_ref: "d:d-seq/seq/message/m:1".to_owned(),
        }))
        .await
        .expect("attention.agent.set second");
    assert_eq!(second.object_ref, "d:d-seq/seq/message/m:1");
    assert_eq!(second.diagram_id, "d-seq");

    let Json(read) = server.attention_agent_read().await.expect("attention.agent.read");
    assert_eq!(read.object_ref.as_deref(), Some("d:d-seq/seq/message/m:1"));
    assert_eq!(read.diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(read.context.session_active_diagram_id.as_deref(), Some("d-seq"));

    let Json(cleared) = server.attention_agent_clear().await.expect("attention.agent.clear");
    assert_eq!(cleared.cleared, 1);

    let Json(cleared_again) =
        server.attention_agent_clear().await.expect("attention.agent.clear again");
    assert_eq!(cleared_again.cleared, 0);
}

#[tokio::test]
async fn streamable_http_tools_call_updates_shared_agent_attention_state() {
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use rmcp::transport::{
        streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
        StreamableHttpService,
    };

    let agent_highlights = Arc::new(Mutex::new(BTreeSet::new()));
    let server = NereidMcp::new_with_agent_highlights(demo_session(), agent_highlights.clone());

    let config = StreamableHttpServerConfig {
        stateful_mode: false,
        sse_keep_alive: None,
        ..StreamableHttpServerConfig::default()
    };

    let session_manager = Arc::new(LocalSessionManager::default());
    let service = {
        let server = server.clone();
        StreamableHttpService::new(move || Ok(server.clone()), session_manager, config)
    };

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "attention.agent.set",
            "arguments": {
                "object_ref": "d:d-seq/seq/participant/p:a",
            }
        }
    })
    .to_string();

    let response = service
        .handle(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(axum::http::header::ACCEPT, "application/json, text/event-stream")
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .expect("request"),
        )
        .await;

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert!(response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/event-stream")));

    let response_body = Body::new(response.into_body());
    let bytes = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        to_bytes(response_body, usize::MAX),
    )
    .await
    .expect("timeout collecting response body")
    .expect("collect response body");
    assert!(!bytes.is_empty());

    let highlights = agent_highlights.lock().await;
    assert_eq!(highlights.len(), 1);
    assert!(highlights.iter().any(|oref| oref.to_string() == "d:d-seq/seq/participant/p:a"));
}

#[tokio::test]
async fn list_diagrams_returns_deterministic_order() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server.diagram_list().await.expect("list");
    let ids = result.diagrams.iter().map(|d| d.diagram_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["d-flow", "d-seq"]);
    assert_eq!(result.context.session_active_diagram_id.as_deref(), Some("d-seq"));
}

#[tokio::test]
async fn walkthrough_list_returns_deterministic_order_and_counts() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let Json(result) = server.walkthrough_list().await.expect("walkthrough list");

    let ids = result.walkthroughs.iter().map(|w| w.walkthrough_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["w:1", "w:2"]);

    assert_eq!(result.walkthroughs[0].title, "First walkthrough");
    assert_eq!(result.walkthroughs[0].rev, 0);
    assert_eq!(result.walkthroughs[0].nodes, 2);
    assert_eq!(result.walkthroughs[0].edges, 1);

    assert_eq!(result.walkthroughs[1].title, "Second walkthrough");
    assert_eq!(result.walkthroughs[1].rev, 2);
    assert_eq!(result.walkthroughs[1].nodes, 1);
    assert_eq!(result.walkthroughs[1].edges, 0);
    assert_eq!(result.context.session_active_diagram_id, None);
}

#[tokio::test]
async fn walkthrough_read_returns_nodes_edges_and_refs() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let Json(result) = server
        .walkthrough_read(Parameters(WalkthroughGetParams { walkthrough_id: "w:1".into() }))
        .await
        .expect("walkthrough read");

    assert_eq!(result.walkthrough.walkthrough_id, "w:1");
    assert_eq!(result.walkthrough.title, "First walkthrough");
    assert_eq!(result.walkthrough.rev, 0);

    assert_eq!(result.walkthrough.nodes.len(), 2);
    assert_eq!(result.walkthrough.nodes[0].node_id, "wn:2");
    assert_eq!(result.walkthrough.nodes[0].title, "Start");
    assert_eq!(result.walkthrough.nodes[0].body_md.as_deref(), Some("Start body"));
    assert_eq!(
        result.walkthrough.nodes[0].refs,
        vec!["d:d-seq/seq/message/m:1", "d:d-flow/flow/node/n:a"]
    );
    assert_eq!(result.walkthrough.nodes[0].tags, vec!["intro", "evidence"]);
    assert_eq!(result.walkthrough.nodes[0].status.as_deref(), Some("draft"));

    assert_eq!(result.walkthrough.nodes[1].node_id, "wn:3");
    assert_eq!(result.walkthrough.nodes[1].title, "End");
    assert_eq!(result.walkthrough.nodes[1].body_md, None);
    assert_eq!(result.walkthrough.nodes[1].refs, vec!["d:d-flow/flow/edge/e:ab"]);
    assert!(result.walkthrough.nodes[1].tags.is_empty());
    assert_eq!(result.walkthrough.nodes[1].status, None);

    assert_eq!(result.walkthrough.edges.len(), 1);
    assert_eq!(result.walkthrough.edges[0].from_node_id, "wn:2");
    assert_eq!(result.walkthrough.edges[0].to_node_id, "wn:3");
    assert_eq!(result.walkthrough.edges[0].kind, "next");
    assert_eq!(result.walkthrough.edges[0].label.as_deref(), Some("continue"));
    assert_eq!(result.context.session_active_diagram_id, None);
}

#[tokio::test]
async fn walkthrough_get_node_returns_single_node() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let Json(result) = server
        .walkthrough_get_node(Parameters(WalkthroughGetNodeParams {
            walkthrough_id: "w:1".into(),
            node_id: "wn:2".into(),
        }))
        .await
        .expect("walkthrough get node");

    assert_eq!(result.node.node_id, "wn:2");
    assert_eq!(result.node.title, "Start");
    assert_eq!(result.node.body_md.as_deref(), Some("Start body"));
    assert_eq!(result.node.refs, vec!["d:d-seq/seq/message/m:1", "d:d-flow/flow/node/n:a"]);
    assert_eq!(result.node.tags, vec!["intro", "evidence"]);
    assert_eq!(result.node.status.as_deref(), Some("draft"));
    assert_eq!(result.context.session_active_diagram_id, None);
}

#[tokio::test]
async fn walkthrough_get_node_rejects_invalid_walkthrough_id() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_get_node(Parameters(WalkthroughGetNodeParams {
            walkthrough_id: "w/1".into(),
            node_id: "wn:2".into(),
        }))
        .await
    {
        Ok(_) => panic!("expected invalid id error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn walkthrough_get_node_rejects_invalid_node_id() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_get_node(Parameters(WalkthroughGetNodeParams {
            walkthrough_id: "w:1".into(),
            node_id: "wn/2".into(),
        }))
        .await
    {
        Ok(_) => panic!("expected invalid id error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn walkthrough_get_node_returns_not_found_when_missing_node() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_get_node(Parameters(WalkthroughGetNodeParams {
            walkthrough_id: "w:1".into(),
            node_id: "wn:missing".into(),
        }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn walkthrough_read_rejects_invalid_ids() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_read(Parameters(WalkthroughGetParams { walkthrough_id: "w/1".into() }))
        .await
    {
        Ok(_) => panic!("expected invalid id error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn walkthrough_read_returns_not_found_when_missing() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_read(Parameters(WalkthroughGetParams { walkthrough_id: "w:missing".into() }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn walkthrough_stat_returns_rev_and_counts() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let Json(result) = server
        .walkthrough_stat(Parameters(WalkthroughGetParams { walkthrough_id: "w:1".into() }))
        .await
        .expect("walkthrough stat");

    assert_eq!(result.digest.rev, 0);
    assert_eq!(result.digest.counts.nodes, 2);
    assert_eq!(result.digest.counts.edges, 1);
    assert_eq!(result.context.session_active_diagram_id, None);
}

#[tokio::test]
async fn walkthrough_stat_rejects_invalid_ids() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_stat(Parameters(WalkthroughGetParams { walkthrough_id: "w/1".into() }))
        .await
    {
        Ok(_) => panic!("expected invalid id error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn walkthrough_stat_returns_not_found_when_missing() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_stat(Parameters(WalkthroughGetParams { walkthrough_id: "w:missing".into() }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn walkthrough_render_text_renders_walkthrough() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let Json(result) = server
        .walkthrough_render_text(Parameters(WalkthroughGetParams { walkthrough_id: "w:1".into() }))
        .await
        .expect("walkthrough render");

    assert_eq!(result.text, "┌───────┐    ┌─────┐\n│ Start │───▶│ End │\n└───────┘    └─────┘");
    assert_eq!(result.context.session_active_diagram_id, None);
}

#[tokio::test]
async fn walkthrough_render_text_rejects_invalid_ids() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_render_text(Parameters(WalkthroughGetParams { walkthrough_id: "w/1".into() }))
        .await
    {
        Ok(_) => panic!("expected invalid id error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn walkthrough_render_text_returns_not_found_when_missing() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_render_text(Parameters(WalkthroughGetParams {
            walkthrough_id: "w:missing".into(),
        }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn walkthrough_apply_ops_conflicts_on_stale_base_rev() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
            walkthrough_id: "w:1".into(),
            base_rev: 123,
            ops: vec![McpWalkthroughOp::SetTitle { title: "Updated".into() }],
        }))
        .await
    {
        Ok(_) => panic!("expected conflict error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
    let data = err.data.expect("error data");
    assert_eq!(data["snapshot_tool"].as_str().unwrap(), "walkthrough.stat");
    assert_eq!(data["digest"]["rev"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn walkthrough_apply_ops_bumps_rev_and_returns_delta_for_add_update_remove() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());

    let Json(result) = server
        .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
            walkthrough_id: "w:1".into(),
            base_rev: 0,
            ops: vec![McpWalkthroughOp::AddNode {
                node_id: "wn:4".into(),
                title: "New".into(),
                body_md: None,
                refs: None,
                tags: None,
                status: None,
            }],
        }))
        .await
        .expect("add node");

    assert_eq!(result.new_rev, 1);
    assert_eq!(result.applied, 1);
    assert_eq!(result.delta.added, vec!["w:w:1/node/wn:4"]);
    assert!(result.delta.removed.is_empty());
    assert!(result.delta.updated.is_empty());

    let Json(result) = server
        .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
            walkthrough_id: "w:1".into(),
            base_rev: 1,
            ops: vec![McpWalkthroughOp::UpdateNode {
                node_id: "wn:4".into(),
                title: Some("Newer".into()),
                body_md: None,
                refs: None,
                tags: None,
                status: None,
            }],
        }))
        .await
        .expect("update node");

    assert_eq!(result.new_rev, 2);
    assert_eq!(result.applied, 1);
    assert!(result.delta.added.is_empty());
    assert!(result.delta.removed.is_empty());
    assert_eq!(result.delta.updated, vec!["w:w:1/node/wn:4"]);

    let Json(result) = server
        .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
            walkthrough_id: "w:1".into(),
            base_rev: 2,
            ops: vec![McpWalkthroughOp::RemoveNode { node_id: "wn:2".into() }],
        }))
        .await
        .expect("remove node");

    assert_eq!(result.new_rev, 3);
    assert_eq!(result.applied, 1);
    assert!(result.delta.added.is_empty());
    assert_eq!(result.delta.removed, vec!["w:w:1/edge/wn:2/wn:3/next", "w:w:1/node/wn:2"]);
    assert!(result.delta.updated.is_empty());
}

#[tokio::test]
async fn walkthrough_diff_spans_multiple_revisions() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());

    server
        .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
            walkthrough_id: "w:1".into(),
            base_rev: 0,
            ops: vec![McpWalkthroughOp::AddNode {
                node_id: "wn:4".into(),
                title: "New".into(),
                body_md: None,
                refs: None,
                tags: None,
                status: None,
            }],
        }))
        .await
        .expect("add node");

    server
        .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
            walkthrough_id: "w:1".into(),
            base_rev: 1,
            ops: vec![McpWalkthroughOp::SetTitle { title: "Updated".into() }],
        }))
        .await
        .expect("set title");

    let Json(delta) = server
        .walkthrough_diff(Parameters(WalkthroughGetDeltaParams {
            walkthrough_id: "w:1".into(),
            since_rev: 0,
        }))
        .await
        .expect("delta");
    assert_eq!(delta.from_rev, 0);
    assert_eq!(delta.to_rev, 2);

    let added = delta
        .changes
        .iter()
        .find(|change| change.kind == DeltaChangeKind::Added)
        .expect("added change");
    assert_eq!(added.refs, vec!["w:w:1/node/wn:4"]);

    let updated = delta
        .changes
        .iter()
        .find(|change| change.kind == DeltaChangeKind::Updated)
        .expect("updated change");
    assert_eq!(updated.refs, vec!["w:w:1/meta"]);

    assert!(
        delta.changes.iter().all(|change| change.kind != DeltaChangeKind::Removed),
        "should not include a removed change"
    );
}

#[tokio::test]
async fn route_find_returns_path_when_route_exists() {
    let server = NereidMcp::new(demo_session_with_route());
    let Json(result) = server
        .route_find(Parameters(RouteFindParams {
            from_ref: "d:d-flow/flow/node/n:a".into(),
            to_ref: "d:d-seq/seq/message/m:2".into(),
            limit: None,
            max_hops: None,
            ordering: None,
        }))
        .await
        .expect("routes");

    assert_eq!(result.routes.len(), 1);
    assert_eq!(
        result.routes[0],
        vec![
            "d:d-flow/flow/node/n:a",
            "d:d-flow/flow/node/n:b",
            "d:d-seq/seq/message/m:1",
            "d:d-seq/seq/message/m:2",
        ]
    );
}

#[tokio::test]
async fn route_find_returns_empty_when_not_found() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .route_find(Parameters(RouteFindParams {
            from_ref: "d:d-flow/flow/node/n:a".into(),
            to_ref: "d:d-seq/seq/message/m:1".into(),
            limit: None,
            max_hops: None,
            ordering: None,
        }))
        .await
        .expect("routes");

    assert!(result.routes.is_empty());
}

#[tokio::test]
async fn route_find_honors_limit_zero() {
    let server = NereidMcp::new(demo_session_with_route());
    let Json(result) = server
        .route_find(Parameters(RouteFindParams {
            from_ref: "d:d-flow/flow/node/n:a".into(),
            to_ref: "d:d-seq/seq/message/m:2".into(),
            limit: Some(0),
            max_hops: None,
            ordering: None,
        }))
        .await
        .expect("routes");

    assert!(result.routes.is_empty());
}

#[tokio::test]
async fn route_find_honors_max_hops() {
    let server = NereidMcp::new(demo_session_with_route());
    let Json(result) = server
        .route_find(Parameters(RouteFindParams {
            from_ref: "d:d-flow/flow/node/n:a".into(),
            to_ref: "d:d-seq/seq/message/m:2".into(),
            limit: None,
            max_hops: Some(2),
            ordering: None,
        }))
        .await
        .expect("routes");

    assert!(result.routes.is_empty());
}

#[tokio::test]
async fn route_find_can_return_multiple_routes_and_defaults_to_fewest_hops() {
    let server = NereidMcp::new(demo_session_with_multiple_routes());
    let Json(result) = server
        .route_find(Parameters(RouteFindParams {
            from_ref: "d:d-flow/flow/node/n:a".into(),
            to_ref: "d:d-seq/seq/message/m:2".into(),
            limit: Some(2),
            max_hops: None,
            ordering: None,
        }))
        .await
        .expect("routes");

    assert_eq!(result.routes.len(), 2);
    assert_eq!(
        result.routes[0],
        vec!["d:d-flow/flow/node/n:a", "d:d-seq/seq/message/m:1", "d:d-seq/seq/message/m:2",]
    );
    assert_eq!(
        result.routes[1],
        vec![
            "d:d-flow/flow/node/n:a",
            "d:d-flow/flow/node/n:b",
            "d:d-seq/seq/message/m:1",
            "d:d-seq/seq/message/m:2",
        ]
    );
}

#[tokio::test]
async fn route_find_supports_lexicographic_ordering() {
    let server = NereidMcp::new(demo_session_with_multiple_routes());
    let Json(result) = server
        .route_find(Parameters(RouteFindParams {
            from_ref: "d:d-flow/flow/node/n:a".into(),
            to_ref: "d:d-seq/seq/message/m:2".into(),
            limit: Some(2),
            max_hops: None,
            ordering: Some("lexicographic".to_owned()),
        }))
        .await
        .expect("routes");

    assert_eq!(result.routes.len(), 2);
    assert_eq!(
        result.routes[0],
        vec![
            "d:d-flow/flow/node/n:a",
            "d:d-flow/flow/node/n:b",
            "d:d-seq/seq/message/m:1",
            "d:d-seq/seq/message/m:2",
        ]
    );
}

#[tokio::test]
async fn route_find_rejects_invalid_ordering_param() {
    let server = NereidMcp::new(demo_session_with_multiple_routes());
    let err = match server
        .route_find(Parameters(RouteFindParams {
            from_ref: "d:d-flow/flow/node/n:a".into(),
            to_ref: "d:d-seq/seq/message/m:2".into(),
            limit: Some(10),
            max_hops: None,
            ordering: Some("bad".to_owned()),
        }))
        .await
    {
        Ok(_) => panic!("expected error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn xref_list_returns_all_xrefs_ordered_by_id() {
    let server = NereidMcp::new(demo_session_with_xrefs());
    let Json(result) = server.xref_list(Parameters(xref_list_params())).await.expect("xref list");

    let ids = result.xrefs.iter().map(|x| x.xref_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["x:1", "x:2"]);
    assert_eq!(result.xrefs.len(), 2);
}

#[tokio::test]
async fn xref_list_can_filter_to_dangling_only() {
    let server = NereidMcp::new(demo_session_with_xrefs());
    let mut params = xref_list_params();
    params.dangling_only = Some(true);
    let Json(result) = server.xref_list(Parameters(params)).await.expect("xref list");

    assert_eq!(result.xrefs.len(), 1);
    assert_ne!(result.xrefs[0].status, "ok");
}

#[tokio::test]
async fn xref_list_can_filter_by_status() {
    let server = NereidMcp::new(demo_session_with_xrefs_varied());

    let mut params = xref_list_params();
    params.status = Some("ok".into());
    let Json(ok_only) = server.xref_list(Parameters(params)).await.expect("xref list");
    assert_eq!(ok_only.xrefs.len(), 1);
    assert_eq!(ok_only.xrefs[0].xref_id, "x:2");

    let mut params = xref_list_params();
    params.status = Some("dangling_*".into());
    let Json(dangling) = server.xref_list(Parameters(params)).await.expect("xref list");
    let ids = dangling.xrefs.iter().map(|x| x.xref_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["x:1", "x:3"]);
}

#[tokio::test]
async fn xref_list_can_filter_by_kind_and_endpoints_and_label() {
    let server = NereidMcp::new(demo_session_with_xrefs_varied());

    let mut params = xref_list_params();
    params.kind = Some("implements".into());
    let Json(kind_only) = server.xref_list(Parameters(params)).await.expect("xref list");
    assert_eq!(kind_only.xrefs.len(), 1);
    assert_eq!(kind_only.xrefs[0].xref_id, "x:3");

    let mut params = xref_list_params();
    params.from_ref = Some("d:d-seq/obj/p:a".into());
    let Json(from_filtered) = server.xref_list(Parameters(params)).await.expect("xref list");
    let ids = from_filtered.xrefs.iter().map(|x| x.xref_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["x:2", "x:3"]);

    let mut params = xref_list_params();
    params.to_ref = Some("d:d-flow/obj/n:missing".into());
    let Json(to_filtered) = server.xref_list(Parameters(params)).await.expect("xref list");
    assert_eq!(to_filtered.xrefs.len(), 1);
    assert_eq!(to_filtered.xrefs[0].xref_id, "x:1");

    let mut params = xref_list_params();
    params.involves_ref = Some("d:d-flow/obj/n:a".into());
    let Json(involves) = server.xref_list(Parameters(params)).await.expect("xref list");
    let ids = involves.xrefs.iter().map(|x| x.xref_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["x:2"]);

    let mut params = xref_list_params();
    params.label_contains = Some("Auth".into());
    let Json(label_filtered) = server.xref_list(Parameters(params)).await.expect("xref list");
    assert_eq!(label_filtered.xrefs.len(), 1);
    assert_eq!(label_filtered.xrefs[0].xref_id, "x:3");
}

#[tokio::test]
async fn xref_list_applies_limit_after_sort() {
    let server = NereidMcp::new(demo_session_with_xrefs_varied());
    let mut params = xref_list_params();
    params.limit = Some(2);
    let Json(result) = server.xref_list(Parameters(params)).await.expect("xref list");
    let ids = result.xrefs.iter().map(|x| x.xref_id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["x:1", "x:2"]);
}

#[tokio::test]
async fn xref_neighbors_returns_out_neighbors_sorted() {
    let server = NereidMcp::new(demo_session_with_neighbors_xrefs());
    let Json(result) = server
        .xref_neighbors(Parameters(XRefNeighborsParams {
            object_ref: "d:d-seq/seq/participant/p:a".into(),
            direction: Some("out".into()),
        }))
        .await
        .expect("xref neighbors");

    assert_eq!(result.neighbors, vec!["d:d-flow/flow/node/n:a", "d:d-flow/flow/node/n:b"]);
}

#[tokio::test]
async fn xref_neighbors_returns_in_neighbors_sorted() {
    let server = NereidMcp::new(demo_session_with_neighbors_xrefs());
    let Json(result) = server
        .xref_neighbors(Parameters(XRefNeighborsParams {
            object_ref: "d:d-flow/flow/node/n:a".into(),
            direction: Some("in".into()),
        }))
        .await
        .expect("xref neighbors");

    assert_eq!(result.neighbors, vec!["d:d-seq/seq/participant/p:a"]);
}

#[tokio::test]
async fn xref_neighbors_unions_in_and_out_by_default() {
    let server = NereidMcp::new(demo_session_with_neighbors_xrefs());
    let Json(result) = server
        .xref_neighbors(Parameters(XRefNeighborsParams {
            object_ref: "d:d-flow/flow/node/n:a".into(),
            direction: None,
        }))
        .await
        .expect("xref neighbors");

    assert_eq!(
        result.neighbors,
        vec!["d:d-seq/seq/participant/p:a", "d:d-seq/seq/participant/p:b"]
    );
}

#[tokio::test]
async fn xref_neighbors_rejects_invalid_direction() {
    let server = NereidMcp::new(demo_session_with_neighbors_xrefs());
    let err = match server
        .xref_neighbors(Parameters(XRefNeighborsParams {
            object_ref: "d:d-seq/seq/participant/p:a".into(),
            direction: Some("sideways".into()),
        }))
        .await
    {
        Ok(_) => panic!("expected invalid direction error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn xref_add_returns_ok_status_when_endpoints_exist() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .xref_add(Parameters(XRefAddParams {
            xref_id: "x:new".into(),
            from: "d:d-seq/seq/participant/p:a".into(),
            to: "d:d-flow/flow/node/n:a".into(),
            kind: "relates_to".into(),
            label: None,
        }))
        .await
        .expect("xref add");

    assert_eq!(result.xref_id, "x:new");
    assert_eq!(result.status, "ok");

    let Json(list) = server.xref_list(Parameters(xref_list_params())).await.expect("xref list");
    assert_eq!(list.xrefs.len(), 1);
    assert_eq!(list.xrefs[0].xref_id, "x:new");
    assert_eq!(list.xrefs[0].status, "ok");
}

#[tokio::test]
async fn xref_add_marks_dangling_to_when_target_missing() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .xref_add(Parameters(XRefAddParams {
            xref_id: "x:dangling".into(),
            from: "d:d-seq/seq/participant/p:a".into(),
            to: "d:d-flow/flow/node/n:missing".into(),
            kind: "relates_to".into(),
            label: None,
        }))
        .await
        .expect("xref add");

    assert_eq!(result.status, "dangling_to");
}

#[tokio::test]
async fn xref_remove_deletes_existing_xref() {
    let server = NereidMcp::new(demo_session_with_xrefs());
    let Json(result) = server
        .xref_remove(Parameters(XRefRemoveParams { xref_id: "x:1".into() }))
        .await
        .expect("xref remove");

    assert!(result.removed);

    let Json(list) = server.xref_list(Parameters(xref_list_params())).await.expect("xref list");
    assert_eq!(list.xrefs.len(), 1);
    assert_eq!(list.xrefs[0].xref_id, "x:2");
}

#[tokio::test]
async fn object_read_returns_seq_participant() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-seq/seq/participant/p:a".into()),
            object_refs: None,
        }))
        .await
        .expect("object read");

    assert_eq!(result.objects.len(), 1);
    assert_eq!(result.objects[0].object_ref, "d:d-seq/seq/participant/p:a");
    match &result.objects[0].object {
        McpObject::SeqParticipant { mermaid_name, role } => {
            assert_eq!(mermaid_name, "A");
            assert_eq!(role.as_deref(), None);
        }
        _ => panic!("unexpected object kind"),
    }
}

#[tokio::test]
async fn object_read_returns_seq_message() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-seq/seq/message/m:1".into()),
            object_refs: None,
        }))
        .await
        .expect("object read");

    assert_eq!(result.objects.len(), 1);
    assert_eq!(result.objects[0].object_ref, "d:d-seq/seq/message/m:1");
    match &result.objects[0].object {
        McpObject::SeqMessage {
            from_participant_id,
            to_participant_id,
            kind,
            arrow,
            text,
            order_key,
        } => {
            assert_eq!(from_participant_id, "p:a");
            assert_eq!(to_participant_id, "p:b");
            assert_eq!(*kind, MessageKind::Sync);
            assert_eq!(arrow.as_deref(), None);
            assert_eq!(text, "Hi");
            assert_eq!(*order_key, 1000);
        }
        _ => panic!("unexpected object kind"),
    }
}

#[tokio::test]
async fn object_read_returns_seq_block() {
    let server = NereidMcp::new(demo_session_with_seq_blocks());
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-seq-blocks/seq/block/b:0000".into()),
            object_refs: None,
        }))
        .await
        .expect("object read");

    assert_eq!(result.objects.len(), 1);
    match &result.objects[0].object {
        McpObject::SeqBlock { kind, header, section_ids, child_block_ids } => {
            assert_eq!(*kind, McpSeqBlockKind::Alt);
            assert_eq!(header.as_deref(), Some("guard"));
            assert_eq!(
                section_ids,
                &vec![String::from("sec:0000:00"), String::from("sec:0000:01")]
            );
            assert!(child_block_ids.is_empty());
        }
        _ => panic!("unexpected object kind"),
    }
}

#[tokio::test]
async fn object_read_returns_seq_section() {
    let server = NereidMcp::new(demo_session_with_seq_blocks());
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-seq-blocks/seq/section/sec:0000:00".into()),
            object_refs: None,
        }))
        .await
        .expect("object read");

    assert_eq!(result.objects.len(), 1);
    match &result.objects[0].object {
        McpObject::SeqSection { kind, header, message_ids } => {
            assert_eq!(*kind, McpSeqSectionKind::Main);
            assert_eq!(header.as_deref(), Some("ok"));
            assert_eq!(message_ids, &vec![String::from("m:1")]);
        }
        _ => panic!("unexpected object kind"),
    }
}

#[tokio::test]
async fn seq_trace_after_from_message_returns_following_messages() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let Json(result) = server
        .seq_trace(Parameters(SeqTraceParams {
            diagram_id: None,
            from_message_id: Some("m:0001".into()),
            direction: Some("after".into()),
            limit: Some(10),
        }))
        .await
        .expect("seq trace");

    assert_eq!(
        result.messages,
        vec!["d:d-seq-trace/seq/message/m:0003", "d:d-seq-trace/seq/message/m:0004",]
    );
}

#[tokio::test]
async fn seq_trace_before_from_message_returns_preceding_messages() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let Json(result) = server
        .seq_trace(Parameters(SeqTraceParams {
            diagram_id: None,
            from_message_id: Some("m:0003".into()),
            direction: Some("before".into()),
            limit: Some(2),
        }))
        .await
        .expect("seq trace");

    assert_eq!(
        result.messages,
        vec!["d:d-seq-trace/seq/message/m:0002", "d:d-seq-trace/seq/message/m:0001",]
    );
}

#[tokio::test]
async fn seq_trace_from_message_omitted_returns_first_or_last_messages() {
    let server = NereidMcp::new(demo_session_for_seq_trace());

    let Json(result) = server
        .seq_trace(Parameters(SeqTraceParams {
            diagram_id: None,
            from_message_id: None,
            direction: None,
            limit: Some(2),
        }))
        .await
        .expect("seq trace");

    assert_eq!(
        result.messages,
        vec!["d:d-seq-trace/seq/message/m:0002", "d:d-seq-trace/seq/message/m:0001",]
    );

    let Json(result) = server
        .seq_trace(Parameters(SeqTraceParams {
            diagram_id: None,
            from_message_id: None,
            direction: Some("before".into()),
            limit: Some(2),
        }))
        .await
        .expect("seq trace");

    assert_eq!(
        result.messages,
        vec!["d:d-seq-trace/seq/message/m:0003", "d:d-seq-trace/seq/message/m:0004",]
    );
}

#[tokio::test]
async fn seq_trace_rejects_invalid_direction() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let err = match server
        .seq_trace(Parameters(SeqTraceParams {
            diagram_id: None,
            from_message_id: None,
            direction: Some("sideways".into()),
            limit: None,
        }))
        .await
    {
        Ok(_) => panic!("expected invalid direction error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn seq_trace_returns_not_found_when_from_message_missing() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let err = match server
        .seq_trace(Parameters(SeqTraceParams {
            diagram_id: None,
            from_message_id: Some("m:9999".into()),
            direction: Some("after".into()),
            limit: None,
        }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn seq_search_returns_matches_in_deterministic_order() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let Json(result) = server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: None,
            needle: "d".into(),
            mode: None,
            case_insensitive: None,
        }))
        .await
        .expect("seq search");

    assert_eq!(
        result.messages,
        vec!["d:d-seq-trace/seq/message/m:0001", "d:d-seq-trace/seq/message/m:0003",]
    );
}

#[tokio::test]
async fn seq_search_returns_empty_when_no_match() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let Json(result) = server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: None,
            needle: "zz".into(),
            mode: None,
            case_insensitive: None,
        }))
        .await
        .expect("seq search");

    assert!(result.messages.is_empty());
}

#[tokio::test]
async fn seq_search_supports_regex_mode_with_default_case_insensitive() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let Json(result) = server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: None,
            needle: "^third$".into(),
            mode: Some("regex".into()),
            case_insensitive: None,
        }))
        .await
        .expect("seq search");

    assert_eq!(result.messages, vec!["d:d-seq-trace/seq/message/m:0003"]);
}

#[tokio::test]
async fn seq_search_honors_case_insensitive_false_in_regex_mode() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let Json(result) = server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: None,
            needle: "^third$".into(),
            mode: Some("regex".into()),
            case_insensitive: Some(false),
        }))
        .await
        .expect("seq search");

    assert!(result.messages.is_empty());
}

#[tokio::test]
async fn seq_search_rejects_invalid_mode() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let err = match server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: None,
            needle: "d".into(),
            mode: Some("glob".into()),
            case_insensitive: None,
        }))
        .await
    {
        Ok(_) => panic!("expected invalid mode error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn seq_search_rejects_invalid_regex() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let err = match server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: None,
            needle: "(".into(),
            mode: Some("regex".into()),
            case_insensitive: None,
        }))
        .await
    {
        Ok(_) => panic!("expected invalid regex error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn seq_search_rejects_invalid_diagram_id() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let err = match server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: Some("d/invalid".into()),
            needle: "d".into(),
            mode: None,
            case_insensitive: None,
        }))
        .await
    {
        Ok(_) => panic!("expected invalid diagram id error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn seq_search_rejects_non_sequence_diagram() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: Some("d-flow".into()),
            needle: "A".into(),
            mode: None,
            case_insensitive: None,
        }))
        .await
    {
        Ok(_) => panic!("expected non-sequence diagram error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn seq_search_rejects_empty_needle() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let err = match server
        .seq_search(Parameters(SeqSearchParams {
            diagram_id: None,
            needle: "".into(),
            mode: None,
            case_insensitive: None,
        }))
        .await
    {
        Ok(_) => panic!("expected empty needle error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn seq_messages_filters_and_orders_deterministically() {
    let server = NereidMcp::new(demo_session_for_seq_trace());

    let params =
        SeqMessagesParams { diagram_id: None, from_participant_id: None, to_participant_id: None };
    let Json(all) = server.seq_messages(Parameters(params.clone())).await.expect("seq messages");
    assert_eq!(
        all.messages,
        vec![
            "d:d-seq-trace/seq/message/m:0002",
            "d:d-seq-trace/seq/message/m:0001",
            "d:d-seq-trace/seq/message/m:0003",
            "d:d-seq-trace/seq/message/m:0004",
        ]
    );

    let Json(all_again) = server.seq_messages(Parameters(params)).await.expect("seq messages");
    assert_eq!(all_again.messages, all.messages);

    let Json(from_filtered) = server
        .seq_messages(Parameters(SeqMessagesParams {
            diagram_id: None,
            from_participant_id: Some("p:a".into()),
            to_participant_id: None,
        }))
        .await
        .expect("seq messages");
    assert_eq!(
        from_filtered.messages,
        vec![
            "d:d-seq-trace/seq/message/m:0002",
            "d:d-seq-trace/seq/message/m:0001",
            "d:d-seq-trace/seq/message/m:0004",
        ]
    );

    let Json(to_filtered) = server
        .seq_messages(Parameters(SeqMessagesParams {
            diagram_id: None,
            from_participant_id: None,
            to_participant_id: Some("p:a".into()),
        }))
        .await
        .expect("seq messages");
    assert_eq!(to_filtered.messages, vec!["d:d-seq-trace/seq/message/m:0003"]);

    let Json(both_filtered) = server
        .seq_messages(Parameters(SeqMessagesParams {
            diagram_id: None,
            from_participant_id: Some("p:a".into()),
            to_participant_id: Some("p:b".into()),
        }))
        .await
        .expect("seq messages");
    assert_eq!(both_filtered.messages, from_filtered.messages);
}

#[tokio::test]
async fn seq_messages_rejects_non_sequence_diagram() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .seq_messages(Parameters(SeqMessagesParams {
            diagram_id: Some("d-flow".into()),
            from_participant_id: None,
            to_participant_id: None,
        }))
        .await
    {
        Ok(_) => panic!("expected non-sequence diagram error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn flow_reachable_out_returns_reachable_nodes() {
    let server = NereidMcp::new(demo_session_for_flow_reachable());
    let Json(result) = server
        .flow_reachable(Parameters(FlowReachableParams {
            diagram_id: None,
            from_node_id: "n:b".into(),
            direction: Some("out".into()),
        }))
        .await
        .expect("flow reachable");

    assert_eq!(result.nodes, vec!["d:d-flow-reach/flow/node/n:b", "d:d-flow-reach/flow/node/n:c"]);
}

#[tokio::test]
async fn flow_reachable_in_returns_inbound_reachable_nodes() {
    let server = NereidMcp::new(demo_session_for_flow_reachable());
    let Json(result) = server
        .flow_reachable(Parameters(FlowReachableParams {
            diagram_id: None,
            from_node_id: "n:b".into(),
            direction: Some("in".into()),
        }))
        .await
        .expect("flow reachable");

    assert_eq!(result.nodes, vec!["d:d-flow-reach/flow/node/n:a", "d:d-flow-reach/flow/node/n:b"]);
}

#[tokio::test]
async fn flow_reachable_both_unions_in_and_out() {
    let server = NereidMcp::new(demo_session_for_flow_reachable());
    let Json(result) = server
        .flow_reachable(Parameters(FlowReachableParams {
            diagram_id: None,
            from_node_id: "n:b".into(),
            direction: Some("both".into()),
        }))
        .await
        .expect("flow reachable");

    assert_eq!(
        result.nodes,
        vec![
            "d:d-flow-reach/flow/node/n:a",
            "d:d-flow-reach/flow/node/n:b",
            "d:d-flow-reach/flow/node/n:c",
        ]
    );
}

#[tokio::test]
async fn flow_reachable_rejects_invalid_direction() {
    let server = NereidMcp::new(demo_session_for_flow_reachable());
    let err = match server
        .flow_reachable(Parameters(FlowReachableParams {
            diagram_id: None,
            from_node_id: "n:b".into(),
            direction: Some("sideways".into()),
        }))
        .await
    {
        Ok(_) => panic!("expected invalid direction error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn flow_paths_returns_multiple_paths_in_deterministic_order() {
    let server = NereidMcp::new(demo_session_for_flow_paths());
    let Json(result) = server
        .flow_paths(Parameters(FlowPathsParams {
            diagram_id: None,
            from_node_id: "n:a".into(),
            to_node_id: "n:d".into(),
            limit: None,
            max_extra_hops: None,
        }))
        .await
        .expect("flow paths");

    assert_eq!(
        result.paths,
        vec![
            vec![
                "d:d-flow-paths/flow/node/n:a",
                "d:d-flow-paths/flow/node/n:b",
                "d:d-flow-paths/flow/node/n:d",
            ],
            vec![
                "d:d-flow-paths/flow/node/n:a",
                "d:d-flow-paths/flow/node/n:c",
                "d:d-flow-paths/flow/node/n:d",
            ],
        ]
    );
}

#[tokio::test]
async fn flow_paths_returns_empty_when_no_path_exists() {
    let server = NereidMcp::new(demo_session_for_flow_paths());
    let Json(result) = server
        .flow_paths(Parameters(FlowPathsParams {
            diagram_id: None,
            from_node_id: "n:d".into(),
            to_node_id: "n:a".into(),
            limit: None,
            max_extra_hops: None,
        }))
        .await
        .expect("flow paths");

    assert!(result.paths.is_empty());
}

#[tokio::test]
async fn flow_paths_rejects_invalid_node_id() {
    let server = NereidMcp::new(demo_session_for_flow_paths());
    let err = match server
        .flow_paths(Parameters(FlowPathsParams {
            diagram_id: None,
            from_node_id: "n/a".into(),
            to_node_id: "n:d".into(),
            limit: None,
            max_extra_hops: None,
        }))
        .await
    {
        Ok(_) => panic!("expected invalid node id error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn flow_paths_returns_not_found_when_node_missing() {
    let server = NereidMcp::new(demo_session_for_flow_paths());
    let err = match server
        .flow_paths(Parameters(FlowPathsParams {
            diagram_id: None,
            from_node_id: "n:missing".into(),
            to_node_id: "n:d".into(),
            limit: None,
            max_extra_hops: None,
        }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn flow_cycles_returns_self_loops_and_multi_node_cycles() {
    let server = NereidMcp::new(demo_session_for_flow_cycles());
    let Json(result) = server
        .flow_cycles(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("flow cycles");

    assert_eq!(
        result.cycles,
        vec![
            vec!["d:d-flow-cycles/flow/node/n:x", "d:d-flow-cycles/flow/node/n:y",],
            vec!["d:d-flow-cycles/flow/node/n:z"],
        ]
    );
}

#[tokio::test]
async fn flow_dead_ends_returns_terminal_nodes() {
    let server = NereidMcp::new(demo_session_for_flow_cycles());
    let Json(result) = server
        .flow_dead_ends(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("flow dead ends");

    assert_eq!(
        result.nodes,
        vec!["d:d-flow-cycles/flow/node/n:e", "d:d-flow-cycles/flow/node/n:f",]
    );
}

#[tokio::test]
async fn flow_degrees_defaults_to_sort_by_out_and_truncates() {
    let server = NereidMcp::new(demo_session_for_flow_degrees());
    let Json(result) = server
        .flow_degrees(Parameters(FlowDegreesParams {
            diagram_id: None,
            top: Some(2),
            sort_by: None,
        }))
        .await
        .expect("flow degrees");

    let nodes = result
        .nodes
        .iter()
        .map(|node| (node.node_ref.as_str(), node.label.as_str(), node.in_degree, node.out_degree))
        .collect::<Vec<_>>();
    assert_eq!(
        nodes,
        vec![
            ("d:d-flow-degrees/flow/node/n:a", "A", 0, 3),
            ("d:d-flow-degrees/flow/node/n:c", "C", 1, 1),
        ]
    );
}

#[tokio::test]
async fn flow_degrees_sort_by_in_orders_by_degree() {
    let server = NereidMcp::new(demo_session_for_flow_degrees());
    let Json(result) = server
        .flow_degrees(Parameters(FlowDegreesParams {
            diagram_id: None,
            top: Some(3),
            sort_by: Some("in".into()),
        }))
        .await
        .expect("flow degrees");

    let nodes = result
        .nodes
        .iter()
        .map(|node| (node.node_ref.as_str(), node.in_degree, node.out_degree))
        .collect::<Vec<_>>();
    assert_eq!(
        nodes,
        vec![
            ("d:d-flow-degrees/flow/node/n:b", 3, 0),
            ("d:d-flow-degrees/flow/node/n:c", 1, 1),
            ("d:d-flow-degrees/flow/node/n:d", 1, 1),
        ]
    );
}

#[tokio::test]
async fn flow_degrees_sort_by_total_orders_by_degree() {
    let server = NereidMcp::new(demo_session_for_flow_degrees());
    let Json(result) = server
        .flow_degrees(Parameters(FlowDegreesParams {
            diagram_id: None,
            top: Some(3),
            sort_by: Some("total".into()),
        }))
        .await
        .expect("flow degrees");

    let nodes = result
        .nodes
        .iter()
        .map(|node| (node.node_ref.as_str(), node.in_degree, node.out_degree))
        .collect::<Vec<_>>();
    assert_eq!(
        nodes,
        vec![
            ("d:d-flow-degrees/flow/node/n:a", 0, 3),
            ("d:d-flow-degrees/flow/node/n:b", 3, 0),
            ("d:d-flow-degrees/flow/node/n:c", 1, 1),
        ]
    );
}

#[tokio::test]
async fn flow_degrees_top_zero_returns_empty() {
    let server = NereidMcp::new(demo_session_for_flow_degrees());
    let Json(result) = server
        .flow_degrees(Parameters(FlowDegreesParams {
            diagram_id: None,
            top: Some(0),
            sort_by: None,
        }))
        .await
        .expect("flow degrees");

    assert!(result.nodes.is_empty());
}

#[tokio::test]
async fn flow_degrees_rejects_invalid_sort_by() {
    let server = NereidMcp::new(demo_session_for_flow_degrees());
    let err = match server
        .flow_degrees(Parameters(FlowDegreesParams {
            diagram_id: None,
            top: None,
            sort_by: Some("bad".into()),
        }))
        .await
    {
        Ok(_) => panic!("expected invalid params error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn flow_degrees_rejects_non_flowchart_diagram() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .flow_degrees(Parameters(FlowDegreesParams {
            diagram_id: Some("d-seq".into()),
            top: None,
            sort_by: None,
        }))
        .await
    {
        Ok(_) => panic!("expected non-flowchart error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn flow_cycles_rejects_invalid_diagram_id() {
    let server = NereidMcp::new(demo_session_for_flow_cycles());
    let err = match server
        .flow_cycles(Parameters(DiagramTargetParams { diagram_id: Some("d/invalid".into()) }))
        .await
    {
        Ok(_) => panic!("expected invalid diagram id error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn flow_dead_ends_rejects_non_flowchart_diagram() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .flow_dead_ends(Parameters(DiagramTargetParams { diagram_id: Some("d-seq".into()) }))
        .await
    {
        Ok(_) => panic!("expected non-flowchart error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn flow_unreachable_returns_unreachable_nodes_in_deterministic_order() {
    let server = NereidMcp::new(demo_session_for_flow_unreachable());

    let params = FlowUnreachableParams { diagram_id: None, start_node_id: None };
    let Json(result) =
        server.flow_unreachable(Parameters(params.clone())).await.expect("flow unreachable");
    assert_eq!(
        result.nodes,
        vec!["d:d-flow-unreach/flow/node/n:x", "d:d-flow-unreach/flow/node/n:y",]
    );

    let Json(again) = server.flow_unreachable(Parameters(params)).await.expect("flow unreachable");
    assert_eq!(again.nodes, result.nodes);
}

#[tokio::test]
async fn flow_unreachable_rejects_non_flowchart_diagram() {
    let server = NereidMcp::new(demo_session_for_seq_trace());
    let err = match server
        .flow_unreachable(Parameters(FlowUnreachableParams {
            diagram_id: None,
            start_node_id: None,
        }))
        .await
    {
        Ok(_) => panic!("expected non-flowchart error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn object_read_returns_flow_node() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-flow/flow/node/n:a".into()),
            object_refs: None,
        }))
        .await
        .expect("object read");

    assert_eq!(result.objects.len(), 1);
    assert_eq!(result.objects[0].object_ref, "d:d-flow/flow/node/n:a");
    assert_eq!(result.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(result.context.human_active_diagram_id, None);
    assert_eq!(result.context.human_active_object_ref, None);
    assert_eq!(result.context.follow_ai, None);
    assert_eq!(result.context.ui_rev, None);
    assert_eq!(result.context.ui_session_rev, None);
    match &result.objects[0].object {
        McpObject::FlowNode { label, shape, mermaid_id } => {
            assert_eq!(label, "A");
            assert_eq!(shape, "rect");
            assert_eq!(mermaid_id.as_deref(), None);
        }
        _ => panic!("unexpected object kind"),
    }
}

#[tokio::test]
async fn object_read_includes_shared_ui_context_when_available() {
    let ui_state = Arc::new(Mutex::new(UiState::default()));
    {
        let mut ui = ui_state.lock().await;
        ui.set_human_selection(
            Some(DiagramId::new("d-flow").expect("diagram id")),
            Some(ObjectRef::from_str("d:d-flow/flow/node/n:b").expect("object ref")),
        );
        ui.set_follow_ai(false);
        ui.bump_session_rev();
    }

    let server = NereidMcp::new_with_agent_highlights_and_ui_state(
        demo_session(),
        Arc::new(Mutex::new(BTreeSet::new())),
        Some(ui_state),
    );
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-seq/seq/participant/p:a".into()),
            object_refs: None,
        }))
        .await
        .expect("object read");

    assert_eq!(result.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(result.context.human_active_diagram_id.as_deref(), Some("d-flow"));
    assert_eq!(result.context.human_active_object_ref.as_deref(), Some("d:d-flow/flow/node/n:b"));
    assert_eq!(result.context.follow_ai, Some(false));
    assert_eq!(result.context.ui_rev, Some(3));
    assert_eq!(result.context.ui_session_rev, Some(1));
}

#[tokio::test]
async fn object_read_returns_flow_edge() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-flow/flow/edge/e:ab".into()),
            object_refs: None,
        }))
        .await
        .expect("object read");

    assert_eq!(result.objects.len(), 1);
    assert_eq!(result.objects[0].object_ref, "d:d-flow/flow/edge/e:ab");
    match &result.objects[0].object {
        McpObject::FlowEdge { from_node_id, to_node_id, label, connector, style } => {
            assert_eq!(from_node_id, "n:a");
            assert_eq!(to_node_id, "n:b");
            assert_eq!(label.as_deref(), None);
            assert_eq!(connector.as_deref(), None);
            assert_eq!(style.as_deref(), None);
        }
        _ => panic!("unexpected object kind"),
    }
}

#[tokio::test]
async fn object_read_accepts_object_refs_array() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .object_read(Parameters(ObjectGetParams {
            object_ref: None,
            object_refs: Some(vec![
                "d:d-seq/seq/participant/p:a".into(),
                "d:d-flow/flow/edge/e:ab".into(),
            ]),
        }))
        .await
        .expect("object read");

    assert_eq!(result.objects.len(), 2);
    assert_eq!(result.objects[0].object_ref, "d:d-seq/seq/participant/p:a");
    assert_eq!(result.objects[1].object_ref, "d:d-flow/flow/edge/e:ab");
    assert!(matches!(&result.objects[0].object, McpObject::SeqParticipant { .. }));
    assert!(matches!(&result.objects[1].object, McpObject::FlowEdge { .. }));
}

#[tokio::test]
async fn object_read_rejects_when_object_ref_and_object_refs_are_both_set() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .object_read(Parameters(ObjectGetParams {
            object_ref: Some("d:d-seq/seq/participant/p:a".into()),
            object_refs: Some(vec![String::from("d:d-flow/flow/edge/e:ab")]),
        }))
        .await
    {
        Ok(_) => panic!("expected invalid_params"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn diagram_current_returns_null_when_unset() {
    let session = Session::new(SessionId::new("s:mcp-unset").expect("session id"));
    let server = NereidMcp::new(session);
    let Json(result) = server.diagram_current().await.expect("get active diagram");
    assert_eq!(result.active_diagram_id, None);
    assert_eq!(result.context.session_active_diagram_id, None);
    assert_eq!(result.context.human_active_diagram_id, None);
    assert_eq!(result.context.human_active_object_ref, None);
    assert_eq!(result.context.follow_ai, None);
    assert_eq!(result.context.ui_rev, None);
    assert_eq!(result.context.ui_session_rev, None);
}

#[tokio::test]
async fn diagram_current_returns_id_when_set() {
    let server = NereidMcp::new(demo_session());
    server
        .diagram_open(Parameters(DiagramOpenParams { diagram_id: "d-flow".into() }))
        .await
        .expect("set active diagram");

    let Json(result) = server.diagram_current().await.expect("get active diagram");
    assert_eq!(result.active_diagram_id.as_deref(), Some("d-flow"));
    assert_eq!(result.context.session_active_diagram_id.as_deref(), Some("d-flow"));
    assert_eq!(result.context.human_active_diagram_id, None);
    assert_eq!(result.context.human_active_object_ref, None);
    assert_eq!(result.context.follow_ai, None);
    assert_eq!(result.context.ui_rev, None);
    assert_eq!(result.context.ui_session_rev, None);
}

#[tokio::test]
async fn walkthrough_current_returns_null_when_unset() {
    let session = Session::new(SessionId::new("s:mcp-walkthrough-unset").expect("session id"));
    let server = NereidMcp::new(session);
    let Json(result) = server.walkthrough_current().await.expect("get active walkthrough");
    assert_eq!(result.active_walkthrough_id, None);
    assert_eq!(result.context.session_active_diagram_id, None);
}

#[tokio::test]
async fn walkthrough_open_then_current_returns_id() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let Json(result) = server
        .walkthrough_open(Parameters(WalkthroughOpenParams { walkthrough_id: "w:1".into() }))
        .await
        .expect("set active walkthrough");
    assert_eq!(result.active_walkthrough_id, "w:1");

    let Json(after) = server.walkthrough_current().await.expect("get active walkthrough");
    assert_eq!(after.active_walkthrough_id.as_deref(), Some("w:1"));
    assert_eq!(after.context.session_active_diagram_id, None);
}

#[tokio::test]
async fn walkthrough_open_rejects_invalid_id() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_open(Parameters(WalkthroughOpenParams { walkthrough_id: "w/1".into() }))
        .await
    {
        Ok(_) => panic!("expected invalid id error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn walkthrough_open_returns_not_found_when_missing() {
    let server = NereidMcp::new(demo_session_with_walkthroughs());
    let err = match server
        .walkthrough_open(Parameters(WalkthroughOpenParams { walkthrough_id: "w:missing".into() }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn diagram_open_sets_active_diagram() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .diagram_open(Parameters(DiagramOpenParams { diagram_id: "d-flow".into() }))
        .await
        .expect("set active diagram");
    assert_eq!(result.active_diagram_id, "d-flow");

    let Json(digest) = server
        .diagram_stat(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("stat");
    assert_eq!(digest.counts.participants, 0);
    assert_eq!(digest.counts.messages, 0);
    assert_eq!(digest.counts.nodes, 2);
    assert_eq!(digest.counts.edges, 1);
}

#[tokio::test]
async fn diagram_create_from_mermaid_rejects_unrenderable_flowchart() {
    let session = Session::new(SessionId::new("s:mcp-create-unrenderable").expect("session id"));
    let server = NereidMcp::new(session);

    let err = match server
        .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
            mermaid: "flowchart TD\nC --> D\nD --> C\n".into(),
            diagram_id: Some("d-cycle".into()),
            name: Some("Cycle".into()),
            make_active: Some(true),
        }))
        .await
    {
        Ok(_) => panic!("expected render preflight failure"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
    assert!(
        err.message.contains("cannot render Mermaid diagram"),
        "unexpected message: {}",
        err.message
    );
    assert!(err.message.contains("flowchart layout error"), "unexpected message: {}", err.message);
    assert!(err.message.contains("contains a cycle"), "unexpected message: {}", err.message);

    let Json(diagrams) = server.diagram_list().await.expect("diagram list");
    assert!(diagrams.diagrams.is_empty());

    let Json(current) = server.diagram_current().await.expect("diagram current");
    assert!(current.active_diagram_id.is_none());
}

#[tokio::test]
async fn diagram_delete_rejects_invalid_id() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_delete(Parameters(DiagramDeleteParams { diagram_id: "d/flow".into() }))
        .await
    {
        Ok(_) => panic!("expected invalid id error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn diagram_delete_returns_not_found_when_missing() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_delete(Parameters(DiagramDeleteParams { diagram_id: "d-missing".into() }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn diagram_delete_retargets_active_and_prunes_selection() {
    let mut session = demo_session();
    session.xrefs_mut().insert(
        XRefId::new("x:flow-target").expect("xref id"),
        XRef::new(
            ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("from ref"),
            ObjectRef::from_str("d:d-flow/flow/node/n:a").expect("to ref"),
            "relates_to",
            XRefStatus::Ok,
        ),
    );
    let server = NereidMcp::new(session);

    server
        .diagram_open(Parameters(DiagramOpenParams { diagram_id: "d-flow".into() }))
        .await
        .expect("set active diagram");

    server
        .selection_update(Parameters(SelectionUpdateParams {
            object_refs: vec![
                "d:d-flow/flow/edge/e:ab".into(),
                "d:d-seq/seq/participant/p:a".into(),
            ],
            mode: UpdateMode::Replace,
        }))
        .await
        .expect("set selection");

    server
        .attention_agent_set(Parameters(AttentionAgentSetParams {
            object_ref: "d:d-flow/flow/node/n:a".into(),
        }))
        .await
        .expect("set agent attention");

    let Json(result) = server
        .diagram_delete(Parameters(DiagramDeleteParams { diagram_id: "d-flow".into() }))
        .await
        .expect("delete diagram");
    assert_eq!(result.deleted_diagram_id, "d-flow");
    assert_eq!(result.active_diagram_id.as_deref(), Some("d-seq"));

    let Json(diagrams) = server.diagram_list().await.expect("diagram list");
    assert_eq!(diagrams.diagrams.len(), 1);
    assert_eq!(diagrams.diagrams[0].diagram_id, "d-seq");

    let Json(current) = server.diagram_current().await.expect("diagram current");
    assert_eq!(current.active_diagram_id.as_deref(), Some("d-seq"));

    let Json(selection) = server.selection_get().await.expect("selection");
    assert_eq!(selection.object_refs, vec!["d:d-seq/seq/participant/p:a".to_owned()]);

    let Json(agent) = server.attention_agent_read().await.expect("agent attention");
    assert!(agent.object_ref.is_none());
    assert!(agent.diagram_id.is_none());

    let Json(xrefs) = server.xref_list(Parameters(xref_list_params())).await.expect("xref list");
    assert_eq!(xrefs.xrefs.len(), 1);
    assert_eq!(xrefs.xrefs[0].status, "dangling_to");
}

#[tokio::test]
async fn diagram_delete_last_diagram_clears_active_diagram() {
    let mut session = Session::new(SessionId::new("s:mcp-single-diagram").expect("session id"));
    let diagram_id = DiagramId::new("d-only").expect("diagram id");
    let mut ast = FlowchartAst::default();
    ast.nodes_mut().insert(oid("n:a"), FlowNode::new("A"));
    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Only", DiagramAst::Flowchart(ast)),
    );
    session.set_active_diagram_id(Some(diagram_id));

    let server = NereidMcp::new(session);
    let Json(result) = server
        .diagram_delete(Parameters(DiagramDeleteParams { diagram_id: "d-only".into() }))
        .await
        .expect("delete diagram");
    assert_eq!(result.deleted_diagram_id, "d-only");
    assert_eq!(result.active_diagram_id, None);

    let Json(diagrams) = server.diagram_list().await.expect("diagram list");
    assert!(diagrams.diagrams.is_empty());

    let Json(current) = server.diagram_current().await.expect("diagram current");
    assert_eq!(current.active_diagram_id, None);
}

#[tokio::test]
async fn diagram_stat_uses_active_diagram_when_diagram_id_is_null() {
    let server = NereidMcp::new(demo_session());
    server
        .diagram_open(Parameters(DiagramOpenParams { diagram_id: "d-flow".into() }))
        .await
        .expect("set active diagram");

    let params: DiagramTargetParams =
        serde_json::from_value(serde_json::json!({ "diagram_id": null })).expect("params");
    let Json(digest) = server.diagram_stat(Parameters(params)).await.expect("stat");
    assert_eq!(digest.counts.nodes, 2);
    assert_eq!(digest.counts.edges, 1);
    assert_eq!(digest.context.session_active_diagram_id.as_deref(), Some("d-flow"));
    assert_eq!(digest.context.human_active_diagram_id, None);
    assert_eq!(digest.context.human_active_object_ref, None);
    assert_eq!(digest.context.follow_ai, None);
    assert_eq!(digest.context.ui_rev, None);
    assert_eq!(digest.context.ui_session_rev, None);
}

#[tokio::test]
async fn diagram_stat_uses_active_diagram_when_omitted() {
    let server = NereidMcp::new(demo_session());
    let Json(digest) = server
        .diagram_stat(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("stat");
    assert_eq!(digest.counts.participants, 2);
    assert_eq!(digest.counts.messages, 1);
    assert_eq!(digest.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(digest.context.human_active_diagram_id, None);
    assert_eq!(digest.context.human_active_object_ref, None);
    assert_eq!(digest.context.follow_ai, None);
    assert_eq!(digest.context.ui_rev, None);
    assert_eq!(digest.context.ui_session_rev, None);
}

#[tokio::test]
async fn diagram_read_returns_mermaid_and_kind() {
    let server = NereidMcp::new(demo_session());
    let Json(snapshot) = server
        .diagram_read(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("read");
    assert_eq!(snapshot.rev, 0);
    assert_eq!(snapshot.kind, "Sequence");
    assert!(snapshot.mermaid.contains("sequenceDiagram"));
    assert_eq!(snapshot.context.session_active_diagram_id.as_deref(), Some("d-seq"));
    assert_eq!(snapshot.context.human_active_diagram_id, None);
    assert_eq!(snapshot.context.human_active_object_ref, None);
    assert_eq!(snapshot.context.follow_ai, None);
    assert_eq!(snapshot.context.ui_rev, None);
    assert_eq!(snapshot.context.ui_session_rev, None);
}

#[tokio::test]
async fn diagram_get_ast_returns_sorted_sequence_ast() {
    let mut session = Session::new(SessionId::new("s:mcp-get-ast-seq").expect("session id"));

    let diagram_id = DiagramId::new("d-seq-ast").expect("diagram id");
    let mut ast = SequenceAst::default();
    let p_a = oid("p:a");
    let p_b = oid("p:b");

    let mut participant_b = SequenceParticipant::new("B");
    participant_b.set_note(Some("note-b"));
    ast.participants_mut().insert(p_b.clone(), participant_b);
    ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));

    ast.messages_mut().push(SequenceMessage::new(
        oid("m:0002"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Second",
        2000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        oid("m:0001"),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "First",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        oid("m:0000"),
        p_b.clone(),
        p_a.clone(),
        SequenceMessageKind::Return,
        "Tie-break",
        2000,
    ));

    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Seq AST", DiagramAst::Sequence(ast)),
    );
    session.set_active_diagram_id(Some(diagram_id));

    let server = NereidMcp::new(session);
    let Json(result) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("diagram ast");

    assert_eq!(result.diagram_id, "d-seq-ast");
    assert_eq!(result.kind, "Sequence");
    assert_eq!(result.rev, 0);

    let McpDiagramAst::Sequence { participants, messages, blocks } = result.ast else {
        panic!("expected sequence ast");
    };

    let participant_ids =
        participants.iter().map(|p| p.participant_id.as_str()).collect::<Vec<_>>();
    assert_eq!(participant_ids, vec!["p:a", "p:b"]);
    assert!(participants[0].note.is_none());
    assert_eq!(participants[1].note.as_deref(), Some("note-b"));

    let message_ids =
        messages.iter().map(|m| (m.order_key, m.message_id.as_str())).collect::<Vec<_>>();
    assert_eq!(message_ids, vec![(1000, "m:0001"), (2000, "m:0000"), (2000, "m:0002")]);
    assert!(blocks.is_empty());
}

#[tokio::test]
async fn diagram_get_ast_includes_sequence_blocks_and_sections() {
    let mut session = Session::new(SessionId::new("s:mcp-get-ast-seq-blocks").expect("session id"));

    let diagram_id = DiagramId::new("d-seq-ast-blocks").expect("diagram id");
    let mut ast = SequenceAst::default();
    let p_a = oid("p:a");
    let p_b = oid("p:b");
    ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(p_b.clone(), SequenceParticipant::new("B"));

    let m_main = oid("m:0001");
    let m_else = oid("m:0002");
    ast.messages_mut().push(SequenceMessage::new(
        m_main.clone(),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Main",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        m_else.clone(),
        p_a.clone(),
        p_b.clone(),
        SequenceMessageKind::Sync,
        "Else",
        2000,
    ));

    ast.blocks_mut().push(SequenceBlock::new(
        oid("b:0000"),
        SequenceBlockKind::Alt,
        Some("guard".to_owned()),
        vec![
            SequenceSection::new(
                oid("sec:0000:00"),
                SequenceSectionKind::Main,
                Some("ok".to_owned()),
                vec![m_main],
            ),
            SequenceSection::new(
                oid("sec:0000:01"),
                SequenceSectionKind::Else,
                Some("fallback".to_owned()),
                vec![m_else],
            ),
        ],
        Vec::new(),
    ));

    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Seq AST Blocks", DiagramAst::Sequence(ast)),
    );
    session.set_active_diagram_id(Some(diagram_id));

    let server = NereidMcp::new(session);
    let Json(result) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("diagram ast");

    let McpDiagramAst::Sequence { blocks, .. } = result.ast else {
        panic!("expected sequence ast");
    };

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].block_id, "b:0000");
    assert_eq!(blocks[0].kind, McpSeqBlockKind::Alt);
    assert_eq!(blocks[0].header.as_deref(), Some("guard"));
    assert!(blocks[0].blocks.is_empty());
    assert_eq!(blocks[0].sections.len(), 2);

    assert_eq!(blocks[0].sections[0].section_id, "sec:0000:00");
    assert_eq!(blocks[0].sections[0].kind, McpSeqSectionKind::Main);
    assert_eq!(blocks[0].sections[0].header.as_deref(), Some("ok"));
    assert_eq!(blocks[0].sections[0].message_ids, vec!["m:0001"]);

    assert_eq!(blocks[0].sections[1].section_id, "sec:0000:01");
    assert_eq!(blocks[0].sections[1].kind, McpSeqSectionKind::Else);
    assert_eq!(blocks[0].sections[1].header.as_deref(), Some("fallback"));
    assert_eq!(blocks[0].sections[1].message_ids, vec!["m:0002"]);
}

#[tokio::test]
async fn diagram_get_ast_returns_sorted_flowchart_ast() {
    let mut session = Session::new(SessionId::new("s:mcp-get-ast-flow").expect("session id"));

    let diagram_id = DiagramId::new("d-flow-ast").expect("diagram id");
    let mut ast = FlowchartAst::default();
    let n_a = oid("n:a");
    let n_b = oid("n:b");

    let mut node_b = FlowNode::new("B");
    node_b.set_note(Some("note-b"));
    ast.nodes_mut().insert(n_b.clone(), node_b);
    ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    ast.edges_mut().insert(oid("e:002"), FlowEdge::new(n_a.clone(), n_b.clone()));
    ast.edges_mut().insert(oid("e:001"), FlowEdge::new(n_b, n_a));

    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Flow AST", DiagramAst::Flowchart(ast)),
    );
    session.set_active_diagram_id(Some(diagram_id.clone()));

    let server = NereidMcp::new(session);
    let Json(result) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: Some("d-flow-ast".into()) }))
        .await
        .expect("diagram ast");

    assert_eq!(result.diagram_id, "d-flow-ast");
    assert_eq!(result.kind, "Flowchart");
    assert_eq!(result.rev, 0);

    let McpDiagramAst::Flowchart { nodes, edges } = result.ast else {
        panic!("expected flowchart ast");
    };

    let node_ids = nodes.iter().map(|n| n.node_id.as_str()).collect::<Vec<_>>();
    assert_eq!(node_ids, vec!["n:a", "n:b"]);
    assert!(nodes[0].note.is_none());
    assert_eq!(nodes[1].note.as_deref(), Some("note-b"));

    let edge_ids = edges.iter().map(|e| e.edge_id.as_str()).collect::<Vec<_>>();
    assert_eq!(edge_ids, vec!["e:001", "e:002"]);
}

#[tokio::test]
async fn diagram_get_slice_flow_node_defaults_radius_to_one() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: None,
            center_ref: "d:d-flow/flow/node/n:a".into(),
            radius: None,
            depth: None,
            filters: None,
        }))
        .await
        .expect("diagram slice");

    assert_eq!(result.objects, vec!["d:d-flow/flow/node/n:a", "d:d-flow/flow/node/n:b"]);
    assert_eq!(result.edges, vec!["d:d-flow/flow/edge/e:ab"]);
}

#[tokio::test]
async fn diagram_get_slice_depth_overrides_radius() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: None,
            center_ref: "d:d-flow/flow/node/n:a".into(),
            radius: Some(0),
            depth: Some(1),
            filters: None,
        }))
        .await
        .expect("diagram slice");

    assert_eq!(result.objects, vec!["d:d-flow/flow/node/n:a", "d:d-flow/flow/node/n:b"]);
    assert_eq!(result.edges, vec!["d:d-flow/flow/edge/e:ab"]);
}

#[tokio::test]
async fn diagram_get_slice_filters_include_categories() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: None,
            center_ref: "d:d-flow/flow/node/n:a".into(),
            radius: None,
            depth: None,
            filters: Some(DiagramSliceFilters {
                include_categories: Some(vec![String::from("flow/node")]),
                exclude_categories: None,
            }),
        }))
        .await
        .expect("diagram slice");

    assert_eq!(result.objects, vec!["d:d-flow/flow/node/n:a", "d:d-flow/flow/node/n:b"]);
    assert!(result.edges.is_empty());
}

#[tokio::test]
async fn diagram_get_slice_seq_message_radius_zero_includes_endpoints() {
    let server = NereidMcp::new(demo_session());
    let Json(result) = server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: None,
            center_ref: "d:d-seq/seq/message/m:1".into(),
            radius: Some(0),
            depth: None,
            filters: None,
        }))
        .await
        .expect("diagram slice");

    assert_eq!(result.objects, vec!["d:d-seq/seq/participant/p:a", "d:d-seq/seq/participant/p:b"]);
    assert_eq!(result.edges, vec!["d:d-seq/seq/message/m:1"]);
}

#[tokio::test]
async fn diagram_get_slice_seq_block_includes_sections_and_messages() {
    let server = NereidMcp::new(demo_session_with_seq_blocks());
    let Json(result) = server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: None,
            center_ref: "d:d-seq-blocks/seq/block/b:0000".into(),
            radius: None,
            depth: Some(2),
            filters: None,
        }))
        .await
        .expect("diagram slice");

    assert!(result.objects.contains(&"d:d-seq-blocks/seq/block/b:0000".into()));
    assert!(result.objects.contains(&"d:d-seq-blocks/seq/section/sec:0000:00".into()));
    assert!(result.objects.contains(&"d:d-seq-blocks/seq/section/sec:0000:01".into()));
    assert!(result.edges.contains(&"d:d-seq-blocks/seq/message/m:1".into()));
    assert!(result.edges.contains(&"d:d-seq-blocks/seq/message/m:2".into()));
}

#[tokio::test]
async fn diagram_get_slice_rejects_invalid_diagram_id() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: Some("d/invalid".into()),
            center_ref: "d:d-flow/flow/node/n:a".into(),
            radius: None,
            depth: None,
            filters: None,
        }))
        .await
    {
        Ok(_) => panic!("expected invalid diagram id error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn diagram_get_slice_rejects_invalid_center_ref() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: None,
            center_ref: "x:bad".into(),
            radius: None,
            depth: None,
            filters: None,
        }))
        .await
    {
        Ok(_) => panic!("expected invalid center_ref error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn diagram_get_slice_returns_not_found_when_center_missing() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: None,
            center_ref: "d:d-flow/flow/node/n:missing".into(),
            radius: None,
            depth: None,
            filters: None,
        }))
        .await
    {
        Ok(_) => panic!("expected not found error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);
}

#[tokio::test]
async fn diagram_get_slice_rejects_mismatched_center_ref_diagram_id() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_get_slice(Parameters(DiagramGetSliceParams {
            diagram_id: Some("d-seq".into()),
            center_ref: "d:d-flow/flow/node/n:a".into(),
            radius: None,
            depth: None,
            filters: None,
        }))
        .await
    {
        Ok(_) => panic!("expected mismatch error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn diagram_render_text_renders_diagram_and_matches_renderer() {
    let session = demo_session();
    let expected = {
        let diagram_id = DiagramId::new("d-seq").expect("diagram id");
        let diagram = session.diagrams().get(&diagram_id).expect("diagram");
        render_diagram_unicode(diagram).expect("render")
    };

    let server = NereidMcp::new(session);
    let Json(result) = server
        .diagram_render_text(Parameters(DiagramTargetParams { diagram_id: Some("d-seq".into()) }))
        .await
        .expect("diagram render");

    assert_eq!(result.text, expected);
}

#[tokio::test]
async fn diagram_render_text_uses_active_diagram_when_diagram_id_is_omitted() {
    let session = demo_session();
    let expected = {
        let diagram_id = DiagramId::new("d-flow").expect("diagram id");
        let diagram = session.diagrams().get(&diagram_id).expect("diagram");
        render_diagram_unicode(diagram).expect("render")
    };

    let server = NereidMcp::new(session);
    server
        .diagram_open(Parameters(DiagramOpenParams { diagram_id: "d-flow".into() }))
        .await
        .expect("set active diagram");

    let Json(result) = server
        .diagram_render_text(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("diagram render");

    assert_eq!(result.text, expected);
}

#[tokio::test]
async fn apply_ops_conflicts_on_stale_base_rev() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 123,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:new".into(),
                mermaid_name: "New".into(),
            }],
        }))
        .await
    {
        Ok(_) => panic!("expected conflict error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
    let data = err.data.expect("error data");
    assert_eq!(data["snapshot_tool"].as_str().unwrap(), "diagram.stat");
    assert_eq!(data["digest"]["rev"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn apply_ops_maps_kind_mismatch_to_invalid_params() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::FlowUpdateNode {
                node_id: "n:a".into(),
                label: Some("A2".into()),
                shape: None,
            }],
        }))
        .await
    {
        Ok(_) => panic!("expected kind mismatch error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn apply_ops_maps_already_exists_to_invalid_params() {
    let server = NereidMcp::new(demo_session());
    let err = match server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:a".into(),
                mermaid_name: "A".into(),
            }],
        }))
        .await
    {
        Ok(_) => panic!("expected already exists error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn apply_ops_rejects_unrenderable_result_and_preserves_state() {
    let server = NereidMcp::new(demo_session());

    let err = match server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: Some("d-flow".into()),
            base_rev: 0,
            ops: vec![McpOp::FlowAddEdge {
                edge_id: "e:ba".into(),
                from_node_id: "n:b".into(),
                to_node_id: "n:a".into(),
                label: None,
                connector: None,
                style: None,
            }],
        }))
        .await
    {
        Ok(_) => panic!("expected renderability validation error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
    assert!(
        err.message.contains("cannot render diagram after apply_ops"),
        "unexpected message: {}",
        err.message
    );
    assert!(err.message.contains("contains a cycle"), "unexpected message: {}", err.message);

    let Json(stat) = server
        .diagram_stat(Parameters(DiagramTargetParams { diagram_id: Some("d-flow".into()) }))
        .await
        .expect("stat");
    assert_eq!(stat.rev, 0);
    assert_eq!(stat.counts.edges, 1);

    let Json(ast) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: Some("d-flow".into()) }))
        .await
        .expect("ast");
    let McpDiagramAst::Flowchart { edges, .. } = ast.ast else {
        panic!("expected flowchart ast");
    };
    assert_eq!(edges.len(), 1);
}

#[tokio::test]
async fn apply_ops_supports_setting_and_clearing_sequence_participant_note() {
    let server = NereidMcp::new(demo_session());

    let Json(result) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::SeqSetParticipantNote {
                participant_id: "p:a".into(),
                note: Some("invariant".into()),
            }],
        }))
        .await
        .expect("apply");

    assert_eq!(result.new_rev, 1);
    assert!(result.delta.added.is_empty());
    assert!(result.delta.removed.is_empty());
    assert_eq!(result.delta.updated, vec!["d:d-seq/seq/participant/p:a".to_owned()]);

    let Json(ast) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("ast");
    let McpDiagramAst::Sequence { participants, .. } = ast.ast else {
        panic!("expected sequence ast");
    };
    assert_eq!(participants[0].participant_id, "p:a");
    assert_eq!(participants[0].note.as_deref(), Some("invariant"));

    let Json(result) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 1,
            ops: vec![McpOp::SeqSetParticipantNote { participant_id: "p:a".into(), note: None }],
        }))
        .await
        .expect("apply clear");

    assert_eq!(result.new_rev, 2);
    assert_eq!(result.delta.updated, vec!["d:d-seq/seq/participant/p:a".to_owned()]);

    let Json(ast) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("ast");
    let McpDiagramAst::Sequence { participants, .. } = ast.ast else {
        panic!("expected sequence ast");
    };
    assert_eq!(participants[0].participant_id, "p:a");
    assert!(participants[0].note.is_none());
}

#[tokio::test]
async fn apply_ops_supports_setting_flow_node_note() {
    let server = NereidMcp::new(demo_session());

    let Json(result) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: Some("d-flow".into()),
            base_rev: 0,
            ops: vec![McpOp::FlowSetNodeNote {
                node_id: "n:a".into(),
                note: Some("invariant".into()),
            }],
        }))
        .await
        .expect("apply");

    assert_eq!(result.new_rev, 1);
    assert!(result.delta.added.is_empty());
    assert!(result.delta.removed.is_empty());
    assert_eq!(result.delta.updated, vec!["d:d-flow/flow/node/n:a".to_owned()]);

    let Json(ast) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: Some("d-flow".into()) }))
        .await
        .expect("ast");
    let McpDiagramAst::Flowchart { nodes, .. } = ast.ast else {
        panic!("expected flowchart ast");
    };
    assert_eq!(nodes[0].node_id, "n:a");
    assert_eq!(nodes[0].note.as_deref(), Some("invariant"));
}

#[tokio::test]
async fn apply_ops_supports_setting_flow_node_mermaid_id() {
    let server = NereidMcp::new(demo_session());

    let Json(result) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: Some("d-flow".into()),
            base_rev: 0,
            ops: vec![McpOp::FlowSetNodeMermaidId {
                node_id: "n:a".into(),
                mermaid_id: Some("authz".into()),
            }],
        }))
        .await
        .expect("apply");

    assert_eq!(result.new_rev, 1);
    assert_eq!(result.delta.updated, vec!["d:d-flow/flow/node/n:a".to_owned()]);

    let Json(ast) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: Some("d-flow".into()) }))
        .await
        .expect("ast");
    let McpDiagramAst::Flowchart { nodes, .. } = ast.ast else {
        panic!("expected flowchart ast");
    };
    let node = nodes.iter().find(|node| node.node_id == "n:a").expect("flow node n:a");
    assert_eq!(node.mermaid_id.as_deref(), Some("authz"));
}

#[tokio::test]
async fn apply_ops_rejects_invalid_flow_node_mermaid_id() {
    let server = NereidMcp::new(demo_session());

    let err = match server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: Some("d-flow".into()),
            base_rev: 0,
            ops: vec![McpOp::FlowSetNodeMermaidId {
                node_id: "n:a".into(),
                mermaid_id: Some("bad-id".into()),
            }],
        }))
        .await
    {
        Ok(_) => panic!("expected invalid params error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
}

#[tokio::test]
async fn propose_ops_supports_flow_node_mermaid_id_without_mutating_state() {
    let server = NereidMcp::new(demo_session());

    let Json(proposed) = server
        .diagram_propose_ops(Parameters(DiagramProposeOpsParams {
            diagram_id: Some("d-flow".into()),
            base_rev: 0,
            ops: vec![McpOp::FlowSetNodeMermaidId {
                node_id: "n:a".into(),
                mermaid_id: Some("authz".into()),
            }],
        }))
        .await
        .expect("propose");
    assert_eq!(proposed.new_rev, 1);
    assert_eq!(proposed.delta.updated, vec!["d:d-flow/flow/node/n:a".to_owned()]);

    let Json(ast) = server
        .diagram_get_ast(Parameters(DiagramTargetParams { diagram_id: Some("d-flow".into()) }))
        .await
        .expect("ast");
    let McpDiagramAst::Flowchart { nodes, .. } = ast.ast else {
        panic!("expected flowchart ast");
    };
    let node = nodes.iter().find(|node| node.node_id == "n:a").expect("flow node n:a");
    assert!(node.mermaid_id.is_none());
}

#[tokio::test]
async fn propose_ops_rejects_unrenderable_result_without_mutating_state() {
    let server = NereidMcp::new(demo_session());

    let err = match server
        .diagram_propose_ops(Parameters(DiagramProposeOpsParams {
            diagram_id: Some("d-flow".into()),
            base_rev: 0,
            ops: vec![McpOp::FlowAddEdge {
                edge_id: "e:ba".into(),
                from_node_id: "n:b".into(),
                to_node_id: "n:a".into(),
                label: None,
                connector: None,
                style: None,
            }],
        }))
        .await
    {
        Ok(_) => panic!("expected renderability validation error"),
        Err(err) => err,
    };
    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
    assert!(
        err.message.contains("cannot render diagram after propose_ops"),
        "unexpected message: {}",
        err.message
    );
    assert!(err.message.contains("contains a cycle"), "unexpected message: {}", err.message);

    let Json(stat) = server
        .diagram_stat(Parameters(DiagramTargetParams { diagram_id: Some("d-flow".into()) }))
        .await
        .expect("stat");
    assert_eq!(stat.rev, 0);
    assert_eq!(stat.counts.edges, 1);
}

#[tokio::test]
async fn propose_ops_does_not_mutate_state() {
    let server = NereidMcp::new(demo_session());

    let Json(before) = server
        .diagram_stat(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("before");
    assert_eq!(before.rev, 0);

    let Json(proposed) = server
        .diagram_propose_ops(Parameters(DiagramProposeOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:proposed".into(),
                mermaid_name: "Proposed".into(),
            }],
        }))
        .await
        .expect("propose");
    assert_eq!(proposed.new_rev, 1);
    assert_eq!(proposed.applied, 1);

    let Json(after) = server
        .diagram_stat(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("after");
    assert_eq!(after.rev, 0);
}

#[tokio::test]
async fn propose_ops_delta_matches_apply_ops_for_same_input() {
    let server = NereidMcp::new(demo_session());
    let params = DiagramProposeOpsParams {
        diagram_id: None,
        base_rev: 0,
        ops: vec![McpOp::SeqAddParticipant {
            participant_id: "p:match".into(),
            mermaid_name: "Match".into(),
        }],
    };

    let Json(proposed) =
        server.diagram_propose_ops(Parameters(params.clone())).await.expect("propose");

    let Json(applied) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: params.diagram_id.clone(),
            base_rev: params.base_rev,
            ops: params.ops.clone(),
        }))
        .await
        .expect("apply");

    assert_eq!(proposed.new_rev, applied.new_rev);
    assert_eq!(proposed.applied, applied.applied);
    assert_eq!(proposed.delta.added, applied.delta.added);
    assert_eq!(proposed.delta.removed, applied.delta.removed);
    assert_eq!(proposed.delta.updated, applied.delta.updated);
}

#[tokio::test]
async fn apply_ops_bumps_rev_and_diff_returns_cached_last_delta() {
    let server = NereidMcp::new(demo_session());

    let Json(before) = server
        .diagram_stat(Parameters(DiagramTargetParams { diagram_id: None }))
        .await
        .expect("before");
    assert_eq!(before.rev, 0);

    let Json(result) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:new".into(),
                mermaid_name: "New".into(),
            }],
        }))
        .await
        .expect("apply");
    assert_eq!(result.new_rev, 1);
    assert_eq!(result.applied, 1);
    assert_eq!(result.delta.added.len(), 1);

    let Json(delta) = server
        .diagram_diff(Parameters(GetDeltaParams { diagram_id: None, since_rev: 0 }))
        .await
        .expect("delta");
    assert_eq!(delta.from_rev, 0);
    assert_eq!(delta.to_rev, 1);
    assert!(delta.changes.iter().any(|c| c.kind == DeltaChangeKind::Added && !c.refs.is_empty()));
}

#[tokio::test]
async fn diagram_diff_can_span_multiple_revisions_within_history_window() {
    let server = NereidMcp::new(demo_session());

    server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:new1".into(),
                mermaid_name: "New1".into(),
            }],
        }))
        .await
        .expect("apply1");

    server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 1,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:new2".into(),
                mermaid_name: "New2".into(),
            }],
        }))
        .await
        .expect("apply2");

    let Json(delta) = server
        .diagram_diff(Parameters(GetDeltaParams { diagram_id: None, since_rev: 0 }))
        .await
        .expect("delta");
    assert_eq!(delta.from_rev, 0);
    assert_eq!(delta.to_rev, 2);
}

#[tokio::test]
async fn diagram_diff_errors_with_supported_since_rev_and_snapshot_tool_outside_history_window() {
    let server = NereidMcp::new(demo_session());

    let mut base_rev = 0_u64;
    for idx in 0..(DELTA_HISTORY_LIMIT as u64 + 2) {
        let Json(result) = server
            .diagram_apply_ops(Parameters(ApplyOpsParams {
                diagram_id: Some("d-flow".into()),
                base_rev,
                ops: vec![McpOp::FlowUpdateNode {
                    node_id: "n:a".into(),
                    label: Some(format!("A{idx}")),
                    shape: None,
                }],
            }))
            .await
            .expect("apply");
        base_rev = result.new_rev;
    }

    let err = match server
        .diagram_diff(Parameters(GetDeltaParams {
            diagram_id: Some("d-flow".into()),
            since_rev: 0,
        }))
        .await
    {
        Ok(_) => panic!("expected delta unavailable error"),
        Err(err) => err,
    };

    assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_REQUEST);
    let data = err.data.expect("error data");
    assert_eq!(data["current_rev"].as_u64().unwrap(), base_rev);
    assert_eq!(data["snapshot_tool"].as_str().unwrap(), "diagram.read");
    assert!(data["supported_since_rev"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn diagram_diff_is_empty_when_since_rev_equals_current_rev() {
    let server = NereidMcp::new(demo_session());
    let Json(delta) = server
        .diagram_diff(Parameters(GetDeltaParams { diagram_id: None, since_rev: 0 }))
        .await
        .expect("delta");
    assert_eq!(delta.from_rev, 0);
    assert_eq!(delta.to_rev, 0);
    assert!(delta.changes.is_empty());
}

#[tokio::test]
async fn diagram_apply_ops_persists_new_rev_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-apply-ops");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    let Json(result) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:new".into(),
                mermaid_name: "New".into(),
            }],
        }))
        .await
        .expect("apply ops");
    assert_eq!(result.new_rev, 1);

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    let diagram_id = DiagramId::new("d-seq").expect("diagram id");
    let diagram = loaded.diagrams().get(&diagram_id).expect("diagram");
    assert_eq!(diagram.rev(), 1);
}

#[tokio::test]
async fn diagram_diff_history_survives_external_selection_only_updates() {
    let dir = temp_session_dir("mcp-delta-history-survives-selection-updates");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    let Json(applied) = server
        .diagram_apply_ops(Parameters(ApplyOpsParams {
            diagram_id: None,
            base_rev: 0,
            ops: vec![McpOp::SeqAddParticipant {
                participant_id: "p:new".into(),
                mermaid_name: "New".into(),
            }],
        }))
        .await
        .expect("apply ops");
    assert_eq!(applied.new_rev, 1);

    let mut external =
        SessionFolder::new(dir_str.clone()).load_session().expect("load session externally");
    external.set_selected_object_refs(
        [ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("object ref")]
            .into_iter()
            .collect(),
    );
    SessionFolder::new(dir_str)
        .save_selected_object_refs(&external)
        .expect("persist external selection");

    let Json(delta) = server
        .diagram_diff(Parameters(GetDeltaParams { diagram_id: None, since_rev: 0 }))
        .await
        .expect("diagram diff");
    assert_eq!(delta.from_rev, 0);
    assert_eq!(delta.to_rev, 1);
}

#[tokio::test]
async fn walkthrough_apply_ops_persists_new_rev_and_title_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-walkthrough-apply-ops");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session_with_walkthroughs();
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    let Json(result) = server
        .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
            walkthrough_id: "w:1".into(),
            base_rev: 0,
            ops: vec![McpWalkthroughOp::SetTitle { title: "Updated".into() }],
        }))
        .await
        .expect("apply ops");
    assert_eq!(result.new_rev, 1);

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    let walkthrough_id = WalkthroughId::new("w:1").expect("walkthrough id");
    let walkthrough = loaded.walkthroughs().get(&walkthrough_id).expect("walkthrough");
    assert_eq!(walkthrough.rev(), 1);
    assert_eq!(walkthrough.title(), "Updated");
}

#[tokio::test]
async fn diagram_open_persists_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-set-active-diagram");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let mut session = demo_session();
    assert_eq!(session.active_diagram_id().map(|diagram_id| diagram_id.as_str()), Some("d-seq"));
    let walkthrough_id = WalkthroughId::new("w:1").expect("walkthrough id");
    let walkthrough = Walkthrough::new(walkthrough_id.clone(), "Walkthrough");
    session.walkthroughs_mut().insert(walkthrough_id, walkthrough);
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    server
        .diagram_open(Parameters(DiagramOpenParams { diagram_id: "d-flow".into() }))
        .await
        .expect("set active diagram");

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    assert_eq!(loaded.active_diagram_id().map(|diagram_id| diagram_id.as_str()), Some("d-flow"));
}

#[tokio::test]
async fn diagram_current_refreshes_from_session_folder() {
    let dir = temp_session_dir("mcp-refresh-current-diagram");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");
    let server = NereidMcp::new_persistent(session, folder);

    let mut external =
        SessionFolder::new(dir_str.clone()).load_session().expect("load session externally");
    external.set_active_diagram_id(Some(DiagramId::new("d-flow").expect("diagram id")));
    SessionFolder::new(dir_str)
        .save_active_diagram_id(&external)
        .expect("persist active diagram externally");

    let Json(current) = server.diagram_current().await.expect("diagram.current");
    assert_eq!(current.active_diagram_id.as_deref(), Some("d-flow"));
}

#[tokio::test]
async fn diagram_list_refreshes_from_session_folder() {
    let dir = temp_session_dir("mcp-refresh-diagram-list");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");
    let server = NereidMcp::new_persistent(session, folder);

    let mut external =
        SessionFolder::new(dir_str.clone()).load_session().expect("load session externally");
    let extra_id = DiagramId::new("d-external").expect("diagram id");
    let mut flow = FlowchartAst::default();
    flow.nodes_mut().insert(oid("n:start"), FlowNode::new("Start"));
    external.diagrams_mut().insert(
        extra_id.clone(),
        Diagram::new(extra_id.clone(), "External".to_owned(), DiagramAst::Flowchart(flow)),
    );
    SessionFolder::new(dir_str).save_session(&external).expect("persist external diagram");

    let Json(list) = server.diagram_list().await.expect("diagram.list");
    assert!(list.diagrams.iter().any(|diagram| diagram.diagram_id == extra_id.as_str()));
}

#[tokio::test]
async fn diagram_delete_persists_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-delete-diagram");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let mut session = demo_session();
    session.set_selected_object_refs(
        [
            ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("object ref"),
            ObjectRef::from_str("d:d-flow/flow/node/n:a").expect("object ref"),
        ]
        .into_iter()
        .collect(),
    );
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    let Json(result) = server
        .diagram_delete(Parameters(DiagramDeleteParams { diagram_id: "d-seq".into() }))
        .await
        .expect("delete diagram");
    assert_eq!(result.deleted_diagram_id, "d-seq");
    assert_eq!(result.active_diagram_id.as_deref(), Some("d-flow"));

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    assert_eq!(loaded.diagrams().len(), 1);
    assert!(loaded.diagrams().contains_key(&DiagramId::new("d-flow").expect("diagram id")));
    assert_eq!(loaded.active_diagram_id().map(|diagram_id| diagram_id.as_str()), Some("d-flow"));
    let selected =
        loaded.selected_object_refs().iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(selected, vec!["d:d-flow/flow/node/n:a".to_owned()]);
}

#[tokio::test]
async fn walkthrough_open_persists_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-set-active-walkthrough");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let mut session = demo_session();
    assert!(session.active_walkthrough_id().is_none());
    let walkthrough_id = WalkthroughId::new("w:1").expect("walkthrough id");
    let walkthrough = Walkthrough::new(walkthrough_id.clone(), "Walkthrough");
    session.walkthroughs_mut().insert(walkthrough_id, walkthrough);
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    server
        .walkthrough_open(Parameters(WalkthroughOpenParams { walkthrough_id: "w:1".into() }))
        .await
        .expect("set active walkthrough");

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    assert_eq!(
        loaded.active_walkthrough_id().map(|walkthrough_id| walkthrough_id.as_str()),
        Some("w:1")
    );
}

#[tokio::test]
async fn selection_update_persists_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-multi-selection");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    server
        .selection_update(Parameters(SelectionUpdateParams {
            object_refs: vec!["d:d-seq/seq/participant/p:a".to_owned()],
            mode: UpdateMode::Replace,
        }))
        .await
        .expect("set selection");

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    let expected = ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("object ref");
    assert_eq!(loaded.selected_object_refs().len(), 1);
    assert!(loaded.selected_object_refs().contains(&expected));
}

#[tokio::test]
async fn selection_get_refreshes_from_session_folder_meta() {
    let dir = temp_session_dir("mcp-selection-read-refreshes-meta");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);

    let mut on_disk = SessionFolder::new(dir_str.clone())
        .load_session()
        .expect("reload session for external update");
    on_disk.set_selected_object_refs(
        [
            ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("object ref"),
            ObjectRef::from_str("d:d-flow/flow/edge/e:ab").expect("object ref"),
        ]
        .into_iter()
        .collect(),
    );
    SessionFolder::new(dir_str)
        .save_selected_object_refs(&on_disk)
        .expect("persist external selection");

    let Json(selection) = server.selection_get().await.expect("selection.read");
    assert_eq!(
        selection.object_refs,
        vec!["d:d-flow/flow/edge/e:ab".to_owned(), "d:d-seq/seq/participant/p:a".to_owned(),]
    );
}

#[tokio::test]
async fn selection_update_add_uses_latest_meta_selection_in_persistent_mode() {
    let dir = temp_session_dir("mcp-selection-update-add-uses-meta");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");
    let server = NereidMcp::new_persistent(session, folder);

    let mut on_disk = SessionFolder::new(dir_str.clone())
        .load_session()
        .expect("reload session for external update");
    on_disk.set_selected_object_refs(
        [ObjectRef::from_str("d:d-seq/seq/participant/p:a").expect("object ref")]
            .into_iter()
            .collect(),
    );
    SessionFolder::new(dir_str.clone())
        .save_selected_object_refs(&on_disk)
        .expect("persist external selection");

    server
        .selection_update(Parameters(SelectionUpdateParams {
            object_refs: vec!["d:d-flow/flow/edge/e:ab".to_owned()],
            mode: UpdateMode::Add,
        }))
        .await
        .expect("selection.update add");

    let loaded = SessionFolder::new(dir_str).load_session().expect("load merged selection");
    let selected =
        loaded.selected_object_refs().iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(
        selected,
        vec!["d:d-flow/flow/edge/e:ab".to_owned(), "d:d-seq/seq/participant/p:a".to_owned(),]
    );
}

#[tokio::test]
async fn xref_add_persists_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-xref-add");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session();
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    server
        .xref_add(Parameters(XRefAddParams {
            xref_id: "x:new".into(),
            from: "d:d-seq/seq/participant/p:a".into(),
            to: "d:d-flow/flow/node/n:a".into(),
            kind: "relates_to".into(),
            label: None,
        }))
        .await
        .expect("xref add");

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    let xref_id = XRefId::new("x:new").expect("xref id");
    assert!(loaded.xrefs().contains_key(&xref_id));
}

#[tokio::test]
async fn xref_remove_persists_to_session_folder() {
    let dir = temp_session_dir("mcp-persist-xref-remove");
    let dir_str = dir.to_string_lossy().to_string();
    let folder = SessionFolder::new(dir_str.clone());

    let session = demo_session_with_xrefs();
    folder.save_session(&session).expect("save initial session");

    let server = NereidMcp::new_persistent(session, folder);
    server
        .xref_remove(Parameters(XRefRemoveParams { xref_id: "x:1".into() }))
        .await
        .expect("xref remove");

    let loaded = SessionFolder::new(dir_str).load_session().expect("load session");
    let removed_id = XRefId::new("x:1").expect("xref id");
    assert!(!loaded.xrefs().contains_key(&removed_id));
}
