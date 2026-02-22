// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use super::*;

use crate::model::SessionId;
use crate::store::SessionFolder;
use crate::tui::testing::HeadlessTui;
use crossterm::event::KeyCode;
use ratatui::style::Color;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn new_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio runtime")
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("nereid-e2e-{prefix}-{}-{nanos}-{counter}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct CollabHarness {
    _tmp: TempDir,
    folder: SessionFolder,
    agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ui_state: Arc<Mutex<UiState>>,
}

impl CollabHarness {
    fn new(prefix: &str) -> Self {
        let tmp = TempDir::new(prefix);
        let session_dir = tmp.path().join("session");
        std::fs::create_dir_all(&session_dir).expect("create session dir");

        let folder = SessionFolder::new(&session_dir);

        let session = Session::new(SessionId::new("s:e2e-collab").expect("session id"));
        folder.save_session(&session).expect("seed session meta");

        Self {
            _tmp: tmp,
            folder,
            agent_highlights: Arc::new(Mutex::new(BTreeSet::new())),
            ui_state: Arc::new(Mutex::new(UiState::default())),
        }
    }

    fn load_session(&self) -> Session {
        self.folder.load_session().expect("load session")
    }

    fn save_session(&self, session: &Session) {
        self.folder.save_session(session).expect("save session");
    }

    fn server(&self) -> NereidMcp {
        self.mcp(self.load_session())
    }

    fn mcp(&self, session: Session) -> NereidMcp {
        NereidMcp::new_persistent_with_agent_highlights_and_ui_state(
            session,
            self.folder.clone(),
            self.agent_highlights.clone(),
            Some(self.ui_state.clone()),
        )
    }

    fn tui(&self, session: Session) -> HeadlessTui {
        HeadlessTui::new(
            session,
            self.agent_highlights.clone(),
            Some(self.ui_state.clone()),
            Some(self.folder.clone()),
        )
    }
}

#[test]
fn e2e_human_and_agent_collaborate_on_persisted_session() {
    let runtime = new_runtime();

    let harness = CollabHarness::new("collab-session");

    let diagram_id = "d-collab";
    let expected_participant_b_ref = format!("d:{diagram_id}/seq/participant/p:b");

    // Step 1 (agent/MCP): create a new diagram and persist it.
    let server = harness.mcp(harness.load_session());
    let mermaid = "sequenceDiagram\nparticipant a\nparticipant b\n";
    let Json(created) = runtime.block_on(async {
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: mermaid.to_owned(),
                diagram_id: Some(diagram_id.to_owned()),
                name: Some("Collab".to_owned()),
                make_active: Some(true),
            }))
            .await
            .expect("diagram.create_from_mermaid")
    });

    assert_eq!(created.diagram.diagram_id, diagram_id);
    assert_eq!(created.active_diagram_id.as_deref(), Some(diagram_id));

    let reloaded = harness.load_session();
    assert_eq!(reloaded.active_diagram_id().map(|id| id.as_str()), Some(diagram_id));
    assert!(
        reloaded
            .diagrams()
            .contains_key(&DiagramId::new(diagram_id.to_owned()).expect("diagram id")),
        "expected created diagram to be present after reload"
    );

    // Step 2 (human/TUI): load the persisted session and move focus to participant b.
    let mut tui = harness.tui(reloaded);
    tui.press(KeyCode::Char('2')); // toggle+focus Objects
    tui.press(KeyCode::Down); // p:a -> p:b
    let participant_b_ref = tui.selected_ref().expect("tui selection").to_string();
    assert_eq!(participant_b_ref, expected_participant_b_ref);

    let Json(human_attention) = runtime
        .block_on(async { server.attention_human_read().await.expect("attention.human.read") });
    assert_eq!(human_attention.diagram_id.as_deref(), Some(diagram_id));
    assert_eq!(human_attention.object_ref.as_deref(), Some(participant_b_ref.as_str()));

    // Step 3 (agent/MCP): set agent attention to participant b and confirm it renders.
    tui.press(KeyCode::Up); // back to p:a so highlights don't overlap focus
    let Json(update) = runtime.block_on(async {
        server
            .attention_agent_set(Parameters(AttentionAgentSetParams {
                object_ref: participant_b_ref.clone(),
            }))
            .await
            .expect("attention.agent.set")
    });
    assert_eq!(update.object_ref, participant_b_ref);
    assert_eq!(update.diagram_id, diagram_id);

    let text = tui.diagram_text();
    let has_agent_highlight = text
        .lines
        .iter()
        .any(|line| line.spans.iter().any(|span| span.style.bg == Some(Color::LightBlue)));
    assert!(has_agent_highlight, "expected agent highlight to render with bright blue background");

    // Step 4 (human/TUI): toggle multi-selection and ensure it persists to disk.
    tui.press(KeyCode::Down); // p:a -> p:b
    tui.press(KeyCode::Char(' '));

    let reloaded = harness.load_session();
    let selected =
        reloaded.selected_object_refs().iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(selected, vec![participant_b_ref.clone()]);

    // Step 5 (agent/MCP): confirm selection is available via MCP after reload.
    let server = harness.mcp(reloaded);
    let Json(selection) =
        runtime.block_on(async { server.selection_get().await.expect("selection.read") });
    assert_eq!(selection.object_refs, vec![participant_b_ref]);
}

#[test]
fn e2e_selection_and_active_diagram_do_not_drift_between_tui_and_mcp() {
    let runtime = new_runtime();
    let harness = CollabHarness::new("selection-active-diagram-drift");
    let server = harness.server();

    runtime.block_on(async {
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: "flowchart LR\nA --> B\n".to_owned(),
                diagram_id: Some("d-a".to_owned()),
                name: Some("A".to_owned()),
                make_active: Some(true),
            }))
            .await
            .expect("create d-a");
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: "flowchart LR\nX --> Y\n".to_owned(),
                diagram_id: Some("d-b".to_owned()),
                name: Some("B".to_owned()),
                make_active: Some(false),
            }))
            .await
            .expect("create d-b");
        server
            .selection_update(Parameters(SelectionUpdateParams {
                object_refs: vec!["d:d-a/flow/node/n:A".to_owned()],
                mode: UpdateMode::Replace,
            }))
            .await
            .expect("seed stale selection");
    });

    let mut tui = harness.tui(harness.load_session());
    tui.press(KeyCode::Char('d')); // clear selection in current diagram (d-a)
    tui.press(KeyCode::Char(']')); // switch to d-b
    tui.press(KeyCode::Char(' ')); // select current object in d-b
    let selected_ref = tui.selected_ref().expect("tui selected ref").to_string();
    assert!(selected_ref.starts_with("d:d-b/"));

    let Json(current) =
        runtime.block_on(async { server.diagram_current().await.expect("diagram.current") });
    assert_eq!(current.active_diagram_id.as_deref(), Some("d-b"));
    assert_eq!(current.context.human_active_diagram_id.as_deref(), Some("d-b"));

    let Json(selection) =
        runtime.block_on(async { server.selection_get().await.expect("selection.read") });
    assert_eq!(selection.object_refs, vec![selected_ref.clone()]);
    assert!(selection.object_refs.iter().all(|object_ref| object_ref.starts_with("d:d-b/")));
    assert_eq!(selection.context.human_active_diagram_id.as_deref(), Some("d-b"));
    assert_eq!(selection.context.human_active_object_ref.as_deref(), Some(selected_ref.as_str()));

    let Json(human_attention) = runtime
        .block_on(async { server.attention_human_read().await.expect("attention.human.read") });
    assert_eq!(human_attention.diagram_id.as_deref(), Some("d-b"));
    assert_eq!(human_attention.object_ref.as_deref(), Some(selected_ref.as_str()));
    assert_eq!(human_attention.context.human_active_diagram_id.as_deref(), Some("d-b"));

    let reloaded = harness.load_session();
    assert_eq!(reloaded.active_diagram_id().map(|diagram_id| diagram_id.as_str()), Some("d-b"));
    let persisted =
        reloaded.selected_object_refs().iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(persisted, vec![selected_ref]);
}

#[test]
fn e2e_tui_reloads_when_mcp_creates_diagram_and_sets_attention() {
    let runtime = new_runtime();
    let harness = CollabHarness::new("tui-reload-on-create");

    let server = harness.server();
    let mut tui = harness.tui(harness.load_session());

    let diagram_id = "d-live-flow";
    let node_ref = format!("d:{diagram_id}/flow/node/n:A");

    runtime.block_on(async {
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: "flowchart TD\nA[Start] --> B[Done]\n".to_owned(),
                diagram_id: Some(diagram_id.to_owned()),
                name: Some("Live".to_owned()),
                make_active: Some(true),
            }))
            .await
            .expect("diagram.create_from_mermaid");

        server
            .attention_agent_set(Parameters(AttentionAgentSetParams {
                object_ref: node_ref.clone(),
            }))
            .await
            .expect("attention.agent.set");
    });

    tui.sync_from_ui_state();
    assert_eq!(tui.selected_ref().expect("tui selection").to_string(), node_ref);
}

#[test]
fn e2e_diagram_and_sequence_tools_cover_full_surface() {
    let runtime = new_runtime();
    let harness = CollabHarness::new("diagram-seq-tools");
    let server = harness.server();

    let diagram_id = "d-seq-tools";
    let participant_a_ref = format!("d:{diagram_id}/seq/participant/p:a");
    let participant_b_ref = format!("d:{diagram_id}/seq/participant/p:b");

    let mermaid = "sequenceDiagram\nparticipant a\nparticipant b\na->>b: Ping\nb-->>a: Pong\n";
    runtime.block_on(async {
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: mermaid.to_owned(),
                diagram_id: Some(diagram_id.to_owned()),
                name: None,
                make_active: Some(true),
            }))
            .await
            .expect("diagram.create_from_mermaid");
    });

    let Json(view) =
        runtime.block_on(async { server.view_get_state().await.expect("view.read_state") });
    assert_eq!(view.active_diagram_id.as_deref(), Some(diagram_id));

    let Json(diagrams) =
        runtime.block_on(async { server.diagram_list().await.expect("diagram.list") });
    assert!(
        diagrams.diagrams.iter().any(|d| d.diagram_id == diagram_id),
        "expected diagram.list to include created diagram"
    );

    let Json(current) =
        runtime.block_on(async { server.diagram_current().await.expect("diagram.current") });
    assert_eq!(current.active_diagram_id.as_deref(), Some(diagram_id));

    let Json(stat) = runtime.block_on(async {
        server
            .diagram_stat(Parameters(DiagramTargetParams {
                diagram_id: Some(diagram_id.to_owned()),
            }))
            .await
            .expect("diagram.stat")
    });
    assert_eq!(stat.counts.participants, 2);
    assert_eq!(stat.counts.messages, 2);
    assert_eq!(stat.rev, 0);

    let Json(snapshot) = runtime.block_on(async {
        server
            .diagram_read(Parameters(DiagramTargetParams { diagram_id: None }))
            .await
            .expect("diagram.read")
    });
    assert_eq!(snapshot.kind, "Sequence");
    assert!(snapshot.mermaid.contains("sequenceDiagram"));

    let Json(render) = runtime.block_on(async {
        server
            .diagram_render_text(Parameters(DiagramTargetParams { diagram_id: None }))
            .await
            .expect("diagram.render_text")
    });
    assert!(!render.text.trim().is_empty());

    let Json(ast) = runtime.block_on(async {
        server
            .diagram_get_ast(Parameters(DiagramTargetParams {
                diagram_id: Some(diagram_id.to_owned()),
            }))
            .await
            .expect("diagram.get_ast")
    });
    match ast.ast {
        McpDiagramAst::Sequence { participants, messages, blocks: _ } => {
            assert_eq!(participants.len(), 2);
            assert_eq!(messages.len(), 2);
        }
        other => panic!("expected sequence AST, got {other:?}"),
    }

    let Json(message_list) = runtime.block_on(async {
        server
            .seq_messages(Parameters(SeqMessagesParams {
                diagram_id: Some(diagram_id.to_owned()),
                from_participant_id: None,
                to_participant_id: None,
            }))
            .await
            .expect("seq.messages")
    });
    assert_eq!(message_list.messages.len(), 2);
    let message_1_ref = message_list.messages[0].clone();
    let message_2_ref = message_list.messages[1].clone();
    let message_1_id = message_1_ref.rsplit('/').next().expect("message id segment").to_owned();
    let message_2_id = message_2_ref.rsplit('/').next().expect("message id segment").to_owned();

    let Json(slice) = runtime.block_on(async {
        server
            .diagram_get_slice(Parameters(DiagramGetSliceParams {
                diagram_id: Some(diagram_id.to_owned()),
                center_ref: participant_a_ref.clone(),
                radius: Some(1),
                depth: None,
                filters: None,
            }))
            .await
            .expect("diagram.get_slice")
    });
    assert!(slice.objects.contains(&participant_a_ref), "expected slice to include center ref");
    assert!(!slice.edges.is_empty());

    let Json(obj_a) = runtime.block_on(async {
        server
            .object_read(Parameters(ObjectGetParams {
                object_ref: Some(participant_a_ref.clone()),
                object_refs: None,
            }))
            .await
            .expect("object.read participant")
    });
    assert_eq!(obj_a.objects.len(), 1);
    match &obj_a.objects[0].object {
        McpObject::SeqParticipant { mermaid_name, .. } => assert_eq!(mermaid_name, "a"),
        other => panic!("expected SeqParticipant, got {other:?}"),
    }

    let Json(obj_msg_1) = runtime.block_on(async {
        server
            .object_read(Parameters(ObjectGetParams {
                object_ref: Some(message_1_ref.clone()),
                object_refs: None,
            }))
            .await
            .expect("object.read message")
    });
    assert_eq!(obj_msg_1.objects.len(), 1);
    match &obj_msg_1.objects[0].object {
        McpObject::SeqMessage { text, .. } => {
            assert!(text.contains("Ping") || text.contains("Pong"))
        }
        other => panic!("expected SeqMessage, got {other:?}"),
    }

    let Json(search) = runtime.block_on(async {
        server
            .seq_search(Parameters(SeqSearchParams {
                diagram_id: Some(diagram_id.to_owned()),
                needle: "Ping".to_owned(),
                mode: Some("substring".to_owned()),
                case_insensitive: Some(true),
            }))
            .await
            .expect("seq.search")
    });
    assert!(search.messages.iter().any(|m| m == &message_1_ref));

    let Json(trace) = runtime.block_on(async {
        server
            .seq_trace(Parameters(SeqTraceParams {
                diagram_id: Some(diagram_id.to_owned()),
                from_message_id: Some(message_1_id.clone()),
                direction: Some("after".to_owned()),
                limit: Some(10),
            }))
            .await
            .expect("seq.trace")
    });
    assert!(trace.messages.iter().any(|m| m == &message_2_ref));

    let Json(highlighted) = runtime.block_on(async {
        server
            .attention_agent_set(Parameters(AttentionAgentSetParams {
                object_ref: participant_b_ref.clone(),
            }))
            .await
            .expect("attention.agent.set")
    });
    assert_eq!(highlighted.object_ref, participant_b_ref.clone());
    assert_eq!(highlighted.diagram_id, diagram_id);

    let Json(attention) = runtime
        .block_on(async { server.attention_agent_read().await.expect("attention.agent.read") });
    assert_eq!(attention.object_ref.as_deref(), Some(participant_b_ref.as_str()));
    assert_eq!(attention.diagram_id.as_deref(), Some(diagram_id));

    let Json(cleared) = runtime
        .block_on(async { server.attention_agent_clear().await.expect("attention.agent.clear") });
    assert_eq!(cleared.cleared, 1);
    let Json(attention) = runtime.block_on(async {
        server.attention_agent_read().await.expect("attention.agent.read (cleared)")
    });
    assert_eq!(attention.object_ref, None);
    assert_eq!(attention.diagram_id, None);

    let Json(proposed) = runtime.block_on(async {
        server
            .diagram_propose_ops(Parameters(DiagramProposeOpsParams {
                diagram_id: Some(diagram_id.to_owned()),
                base_rev: snapshot.rev,
                ops: vec![McpOp::SeqAddMessage {
                    message_id: "m:0003".to_owned(),
                    from_participant_id: "p:a".to_owned(),
                    to_participant_id: "p:b".to_owned(),
                    kind: MessageKind::Sync,
                    arrow: None,
                    text: "Extra".to_owned(),
                    order_key: 4000,
                }],
            }))
            .await
            .expect("diagram.propose_ops")
    });
    assert_eq!(proposed.new_rev, snapshot.rev + 1);

    let Json(applied) = runtime.block_on(async {
        server
            .diagram_apply_ops(Parameters(ApplyOpsParams {
                diagram_id: None,
                base_rev: snapshot.rev,
                ops: vec![McpOp::SeqAddMessage {
                    message_id: "m:0003".to_owned(),
                    from_participant_id: "p:a".to_owned(),
                    to_participant_id: "p:b".to_owned(),
                    kind: MessageKind::Sync,
                    arrow: None,
                    text: "Extra".to_owned(),
                    order_key: 4000,
                }],
            }))
            .await
            .expect("diagram.apply_ops")
    });
    assert_eq!(applied.new_rev, snapshot.rev + 1);
    assert!(applied.applied >= 1);

    let Json(delta) = runtime.block_on(async {
        server
            .diagram_diff(Parameters(GetDeltaParams {
                diagram_id: Some(diagram_id.to_owned()),
                since_rev: snapshot.rev,
            }))
            .await
            .expect("diagram.diff")
    });
    assert_eq!(delta.from_rev, snapshot.rev);
    assert_eq!(delta.to_rev, applied.new_rev);
    assert!(!delta.changes.is_empty());

    let Json(stat_after) = runtime.block_on(async {
        server
            .diagram_stat(Parameters(DiagramTargetParams { diagram_id: None }))
            .await
            .expect("diagram.stat after apply")
    });
    assert_eq!(stat_after.counts.messages, 3);

    let Json(search) = runtime.block_on(async {
        server
            .seq_search(Parameters(SeqSearchParams {
                diagram_id: None,
                needle: "Extra".to_owned(),
                mode: Some("substring".to_owned()),
                case_insensitive: Some(true),
            }))
            .await
            .expect("seq.search extra")
    });
    assert_eq!(search.messages, vec![format!("d:{diagram_id}/seq/message/m:0003")]);

    let Json(trace) = runtime.block_on(async {
        server
            .seq_trace(Parameters(SeqTraceParams {
                diagram_id: None,
                from_message_id: Some(message_2_id),
                direction: Some("after".to_owned()),
                limit: Some(10),
            }))
            .await
            .expect("seq.trace after second")
    });
    assert!(trace.messages.iter().any(|m| m.ends_with("/m:0003")));

    let new_message_ref = format!("d:{diagram_id}/seq/message/m:0003");
    runtime.block_on(async {
        server
            .selection_update(Parameters(SelectionUpdateParams {
                object_refs: vec![new_message_ref.clone()],
                mode: UpdateMode::Replace,
            }))
            .await
            .expect("selection.update");
    });

    let Json(selection) =
        runtime.block_on(async { server.selection_get().await.expect("selection.read") });
    assert_eq!(selection.object_refs, vec![new_message_ref.clone()]);

    let reloaded = harness.load_session();
    let selected =
        reloaded.selected_object_refs().iter().map(ToString::to_string).collect::<Vec<_>>();
    assert_eq!(selected, vec![new_message_ref.clone()]);

    runtime.block_on(async {
        server
            .attention_agent_set(Parameters(AttentionAgentSetParams {
                object_ref: new_message_ref.clone(),
            }))
            .await
            .expect("attention.agent.set");
    });
    let Json(attention) = runtime
        .block_on(async { server.attention_agent_read().await.expect("attention.agent.read") });
    assert_eq!(attention.object_ref.as_deref(), Some(new_message_ref.as_str()));
    assert_eq!(attention.diagram_id.as_deref(), Some(diagram_id));
}

#[test]
fn e2e_flow_xref_route_and_attention_agent_set_cover_full_surface() {
    let runtime = new_runtime();
    let harness = CollabHarness::new("flow-xref-route-tools");
    let server = harness.server();

    let flow_id = "d-flow-tools";
    let seq_id = "d-seq-tools-2";

    runtime.block_on(async {
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: "flowchart LR\na --> b\nb --> c\nd\n".to_owned(),
                diagram_id: Some(flow_id.to_owned()),
                name: None,
                make_active: Some(true),
            }))
            .await
            .expect("create flow");
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: "sequenceDiagram\nparticipant a\nparticipant b\n".to_owned(),
                diagram_id: Some(seq_id.to_owned()),
                name: None,
                make_active: Some(false),
            }))
            .await
            .expect("create seq");
    });

    let Json(diagrams) =
        runtime.block_on(async { server.diagram_list().await.expect("diagram.list") });
    assert_eq!(diagrams.diagrams.len(), 2);

    let Json(opened) = runtime.block_on(async {
        server
            .diagram_open(Parameters(DiagramOpenParams { diagram_id: flow_id.to_owned() }))
            .await
            .expect("diagram.open")
    });
    assert_eq!(opened.active_diagram_id, flow_id);
    let Json(current) =
        runtime.block_on(async { server.diagram_current().await.expect("diagram.current") });
    assert_eq!(current.active_diagram_id.as_deref(), Some(flow_id));

    let Json(flow_ast) = runtime.block_on(async {
        server
            .diagram_get_ast(Parameters(DiagramTargetParams {
                diagram_id: Some(flow_id.to_owned()),
            }))
            .await
            .expect("diagram.get_ast flow")
    });
    let (first_edge_ref, first_node_ref) = match flow_ast.ast {
        McpDiagramAst::Flowchart { nodes, edges } => {
            assert!(!nodes.is_empty());
            assert!(!edges.is_empty());
            let node_id = &nodes[0].node_id;
            let edge_id = &edges[0].edge_id;
            (format!("d:{flow_id}/flow/edge/{edge_id}"), format!("d:{flow_id}/flow/node/{node_id}"))
        }
        other => panic!("expected flow AST, got {other:?}"),
    };

    let Json(obj_node) = runtime.block_on(async {
        server
            .object_read(Parameters(ObjectGetParams {
                object_ref: Some(first_node_ref.clone()),
                object_refs: None,
            }))
            .await
            .expect("object.read flow node")
    });
    assert_eq!(obj_node.objects.len(), 1);
    match &obj_node.objects[0].object {
        McpObject::FlowNode { label, .. } => assert!(!label.is_empty()),
        other => panic!("expected FlowNode, got {other:?}"),
    }

    let Json(obj_edge) = runtime.block_on(async {
        server
            .object_read(Parameters(ObjectGetParams {
                object_ref: Some(first_edge_ref.clone()),
                object_refs: None,
            }))
            .await
            .expect("object.read flow edge")
    });
    assert_eq!(obj_edge.objects.len(), 1);
    match &obj_edge.objects[0].object {
        McpObject::FlowEdge { from_node_id, .. } => assert!(from_node_id.starts_with("n:")),
        other => panic!("expected FlowEdge, got {other:?}"),
    }

    let Json(render) = runtime.block_on(async {
        server
            .diagram_render_text(Parameters(DiagramTargetParams {
                diagram_id: Some(flow_id.to_owned()),
            }))
            .await
            .expect("diagram.render_text flow")
    });
    assert!(!render.text.trim().is_empty());

    let flow_node_a_ref = format!("d:{flow_id}/flow/node/n:a");
    let Json(slice) = runtime.block_on(async {
        server
            .diagram_get_slice(Parameters(DiagramGetSliceParams {
                diagram_id: Some(flow_id.to_owned()),
                center_ref: flow_node_a_ref.clone(),
                radius: Some(1),
                depth: None,
                filters: None,
            }))
            .await
            .expect("diagram.get_slice flow")
    });
    assert!(slice.objects.contains(&flow_node_a_ref));

    let Json(reachable) = runtime.block_on(async {
        server
            .flow_reachable(Parameters(FlowReachableParams {
                diagram_id: Some(flow_id.to_owned()),
                from_node_id: "n:a".to_owned(),
                direction: Some("out".to_owned()),
            }))
            .await
            .expect("flow.reachable")
    });
    assert!(reachable.nodes.iter().any(|n| n == &format!("d:{flow_id}/flow/node/n:c")));

    let Json(paths) = runtime.block_on(async {
        server
            .flow_paths(Parameters(FlowPathsParams {
                diagram_id: Some(flow_id.to_owned()),
                from_node_id: "n:a".to_owned(),
                to_node_id: "n:c".to_owned(),
                limit: Some(10),
                max_extra_hops: Some(0),
            }))
            .await
            .expect("flow.paths")
    });
    assert_eq!(paths.paths.first().map(|p| p.len()), Some(3), "expected a->b->c path");

    let Json(unreachable) = runtime.block_on(async {
        server
            .flow_unreachable(Parameters(FlowUnreachableParams {
                diagram_id: Some(flow_id.to_owned()),
                start_node_id: Some("n:a".to_owned()),
            }))
            .await
            .expect("flow.unreachable")
    });
    assert!(unreachable.nodes.iter().any(|n| n == &format!("d:{flow_id}/flow/node/n:d")));

    let Json(cycles) = runtime.block_on(async {
        server
            .flow_cycles(Parameters(DiagramTargetParams { diagram_id: Some(flow_id.to_owned()) }))
            .await
            .expect("flow.cycles")
    });
    assert!(cycles.cycles.is_empty());

    let Json(dead_ends) = runtime.block_on(async {
        server
            .flow_dead_ends(Parameters(DiagramTargetParams {
                diagram_id: Some(flow_id.to_owned()),
            }))
            .await
            .expect("flow.dead_ends")
    });
    assert!(dead_ends.nodes.iter().any(|n| n == &format!("d:{flow_id}/flow/node/n:c")));

    let Json(degrees) = runtime.block_on(async {
        server
            .flow_degrees(Parameters(FlowDegreesParams {
                diagram_id: Some(flow_id.to_owned()),
                top: Some(10),
                sort_by: Some("total".to_owned()),
            }))
            .await
            .expect("flow.degrees")
    });
    assert!(!degrees.nodes.is_empty());

    let xref_1 = "x:1";
    let xref_2 = "x:2";
    let flow_node_d_ref = format!("d:{flow_id}/flow/node/n:d");
    let seq_participant_a_ref = format!("d:{seq_id}/seq/participant/p:a");
    let seq_participant_b_ref = format!("d:{seq_id}/seq/participant/p:b");

    runtime.block_on(async {
        server
            .xref_add(Parameters(XRefAddParams {
                xref_id: xref_1.to_owned(),
                from: flow_node_d_ref.clone(),
                to: seq_participant_a_ref.clone(),
                kind: "rel".to_owned(),
                label: Some("connect".to_owned()),
            }))
            .await
            .expect("xref.add");
        server
            .xref_add(Parameters(XRefAddParams {
                xref_id: xref_2.to_owned(),
                from: seq_participant_a_ref.clone(),
                to: seq_participant_b_ref.clone(),
                kind: "rel".to_owned(),
                label: None,
            }))
            .await
            .expect("xref.add 2");
    });

    let Json(xrefs) = runtime.block_on(async {
        server
            .xref_list(Parameters(XRefListParams {
                dangling_only: None,
                status: None,
                kind: None,
                from_ref: None,
                to_ref: None,
                involves_ref: None,
                label_contains: None,
                limit: None,
            }))
            .await
            .expect("xref.list")
    });
    assert_eq!(xrefs.xrefs.len(), 2);

    let Json(neighbors) = runtime.block_on(async {
        server
            .xref_neighbors(Parameters(XRefNeighborsParams {
                object_ref: flow_node_d_ref.clone(),
                direction: Some("out".to_owned()),
            }))
            .await
            .expect("xref.neighbors")
    });
    assert_eq!(neighbors.neighbors, vec![seq_participant_a_ref.clone()]);

    let Json(routes) = runtime.block_on(async {
        server
            .route_find(Parameters(RouteFindParams {
                from_ref: flow_node_d_ref.clone(),
                to_ref: seq_participant_a_ref.clone(),
                limit: Some(3),
                max_hops: Some(10),
                ordering: Some("lexicographic".to_owned()),
            }))
            .await
            .expect("route.find")
    });
    assert!(!routes.routes.is_empty());

    // Start a headless TUI to validate attention.agent.set -> follow-AI sync + xref jumps.
    let mut tui = harness.tui(harness.load_session());
    runtime.block_on(async {
        server
            .attention_agent_set(Parameters(AttentionAgentSetParams {
                object_ref: flow_node_d_ref.clone(),
            }))
            .await
            .expect("attention.agent.set");
    });
    tui.sync_from_ui_state();
    assert_eq!(tui.selected_ref().expect("tui selection").to_string(), flow_node_d_ref);

    tui.press(KeyCode::Tab); // Objects -> XRefs
    tui.press(KeyCode::Char('t')); // jump to xref.to (seq participant a)
    assert_eq!(tui.selected_ref().expect("tui selection").to_string(), seq_participant_a_ref);

    let Json(human_attention) = runtime
        .block_on(async { server.attention_human_read().await.expect("attention.human.read") });
    assert_eq!(human_attention.object_ref.as_deref(), Some(seq_participant_a_ref.as_str()));
    assert_eq!(human_attention.diagram_id.as_deref(), Some(seq_id));

    let Json(removed) = runtime.block_on(async {
        server
            .xref_remove(Parameters(XRefRemoveParams { xref_id: xref_2.to_owned() }))
            .await
            .expect("xref.remove")
    });
    assert!(removed.removed);

    let reloaded = harness.load_session();
    assert_eq!(reloaded.xrefs().len(), 1);
}

#[test]
fn e2e_walkthrough_tools_cover_full_surface() {
    let runtime = new_runtime();
    let harness = CollabHarness::new("walkthrough-tools");

    // Seed a small diagram so walkthrough refs can point somewhere realistic.
    let server = harness.server();
    runtime.block_on(async {
        server
            .diagram_create_from_mermaid(Parameters(DiagramCreateFromMermaidParams {
                mermaid: "sequenceDiagram\nparticipant a\nparticipant b\n".to_owned(),
                diagram_id: Some("d-wt".to_owned()),
                name: None,
                make_active: Some(true),
            }))
            .await
            .expect("create diagram");
    });

    let mut session = harness.load_session();
    let walkthrough_id = WalkthroughId::new("w:1".to_owned()).expect("walkthrough id");
    session
        .walkthroughs_mut()
        .insert(walkthrough_id.clone(), Walkthrough::new(walkthrough_id.clone(), "Walkthrough"));
    harness.save_session(&session);

    let server = harness.server();

    let Json(list) =
        runtime.block_on(async { server.walkthrough_list().await.expect("walkthrough.list") });
    assert_eq!(list.walkthroughs.len(), 1);
    assert_eq!(list.walkthroughs[0].walkthrough_id, walkthrough_id.as_str());

    let Json(opened) = runtime.block_on(async {
        server
            .walkthrough_open(Parameters(WalkthroughOpenParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
            }))
            .await
            .expect("walkthrough.open")
    });
    assert_eq!(opened.active_walkthrough_id, walkthrough_id.as_str());

    let Json(current) = runtime
        .block_on(async { server.walkthrough_current().await.expect("walkthrough.current") });
    assert_eq!(current.active_walkthrough_id.as_deref(), Some(walkthrough_id.as_str()));

    let Json(read) = runtime.block_on(async {
        server
            .walkthrough_read(Parameters(WalkthroughGetParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
            }))
            .await
            .expect("walkthrough.read")
    });
    assert_eq!(read.walkthrough.nodes.len(), 0);
    assert_eq!(read.walkthrough.edges.len(), 0);

    let Json(digest) = runtime.block_on(async {
        server
            .walkthrough_stat(Parameters(WalkthroughGetParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
            }))
            .await
            .expect("walkthrough.stat")
    });
    assert_eq!(digest.digest.rev, 0);

    let Json(applied) = runtime.block_on(async {
        server
            .walkthrough_apply_ops(Parameters(WalkthroughApplyOpsParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
                base_rev: 0,
                ops: vec![
                    McpWalkthroughOp::SetTitle { title: "Updated".to_owned() },
                    McpWalkthroughOp::AddNode {
                        node_id: "n:1".to_owned(),
                        title: "Step 1".to_owned(),
                        body_md: Some("Body".to_owned()),
                        refs: Some(vec!["d:d-wt/seq/participant/p:a".to_owned()]),
                        tags: Some(vec!["tag".to_owned()]),
                        status: Some("todo".to_owned()),
                    },
                    McpWalkthroughOp::AddNode {
                        node_id: "n:2".to_owned(),
                        title: "Step 2".to_owned(),
                        body_md: None,
                        refs: None,
                        tags: None,
                        status: None,
                    },
                    McpWalkthroughOp::AddEdge {
                        from_node_id: "n:1".to_owned(),
                        to_node_id: "n:2".to_owned(),
                        kind: "next".to_owned(),
                        label: Some("continue".to_owned()),
                    },
                ],
            }))
            .await
            .expect("walkthrough.apply_ops")
    });
    assert_eq!(applied.new_rev, 1);
    assert!(applied.applied >= 1);

    let Json(render) = runtime.block_on(async {
        server
            .walkthrough_render_text(Parameters(WalkthroughGetParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
            }))
            .await
            .expect("walkthrough.render_text")
    });
    assert!(!render.text.trim().is_empty());

    let Json(node) = runtime.block_on(async {
        server
            .walkthrough_get_node(Parameters(WalkthroughGetNodeParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
                node_id: "n:1".to_owned(),
            }))
            .await
            .expect("walkthrough.get_node")
    });
    assert_eq!(node.node.node_id, "n:1");
    assert_eq!(node.node.title, "Step 1");

    let Json(delta) = runtime.block_on(async {
        server
            .walkthrough_diff(Parameters(WalkthroughGetDeltaParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
                since_rev: 0,
            }))
            .await
            .expect("walkthrough.diff")
    });
    assert_eq!(delta.from_rev, 0);
    assert_eq!(delta.to_rev, 1);
    assert!(!delta.changes.is_empty());

    let Json(digest) = runtime.block_on(async {
        server
            .walkthrough_stat(Parameters(WalkthroughGetParams {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
            }))
            .await
            .expect("walkthrough.stat after apply")
    });
    assert_eq!(digest.digest.rev, 1);

    let reloaded = harness.load_session();
    let wt = reloaded.walkthroughs().get(&walkthrough_id).expect("walkthrough after reload");
    assert_eq!(wt.title(), "Updated");
    assert_eq!(wt.rev(), 1);
    assert_eq!(wt.nodes().len(), 2);
    assert_eq!(wt.edges().len(), 1);
}
