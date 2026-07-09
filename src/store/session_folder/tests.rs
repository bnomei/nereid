// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Session folder persistence regression tests and round-trip fixtures.

use std::env;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use rstest::{fixture, rstest};

use super::{
    DiagramMeta, DiagramStableIdMap, DiagramXRef, SessionFolder, SessionMeta, SessionMetaDiagram,
    StoreError, XRefStatus as StoreXRefStatus,
};
use crate::format::mermaid::{export_flowchart, export_sequence_diagram};
use crate::layout::{layout_flowchart, layout_sequence};
use crate::model::{
    CategoryPath, Diagram, DiagramAst, DiagramId, DiagramKind, FlowEdge, FlowNode, FlowchartAst,
    ObjectId, ObjectRef, SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    Session, SessionId, SymbolAnchor, Walkthrough, WalkthroughEdge, WalkthroughId, WalkthroughNode,
    WalkthroughNodeId, XRef, XRefId, XRefStatus as ModelXRefStatus,
};
use crate::render::{
    render_flowchart_unicode, render_sequence_unicode, render_walkthrough_unicode,
};

static TEMP_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDir {
    path: std::path::PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = env::temp_dir();
        path.push(format!("nereid-{prefix}-{}-{nanos}-{counter}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct SessionFolderTestCtx {
    tmp: TempDir,
    session_dir: std::path::PathBuf,
    folder: SessionFolder,
}

impl SessionFolderTestCtx {
    fn new(prefix: &str) -> Self {
        let tmp = TempDir::new(prefix);
        let session_dir = tmp.path().join("my-session");
        std::fs::create_dir_all(&session_dir).unwrap();
        let folder = SessionFolder::new(&session_dir);
        Self { tmp, session_dir, folder }
    }
}

#[fixture]
fn ctx() -> SessionFolderTestCtx {
    SessionFolderTestCtx::new("session-folder")
}

#[rstest]
fn save_stores_relative_paths_and_load_resolves_them(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let diagram_id = DiagramId::new("d1").unwrap();
    let meta = SessionMeta {
        session_id: SessionId::new("s1").unwrap(),
        active_diagram_id: Some(diagram_id.clone()),
        active_walkthrough_id: None,
        walkthrough_ids: None,
        diagrams: vec![SessionMetaDiagram {
            diagram_id,
            name: "Auth Flow".to_owned(),
            kind: DiagramKind::Flowchart,
            mmd_path: session_dir.join("diagrams/auth-flow.mmd"),
            rev: 0,
        }],
        xrefs: Vec::new(),
        selected_object_refs: Vec::new(),
    };

    folder.save_meta(&meta).unwrap();

    let meta_path = folder.meta_path();
    let meta_str = std::fs::read_to_string(&meta_path).unwrap();
    let meta_json: serde_json::Value = serde_json::from_str(&meta_str).unwrap();

    let stored_path = meta_json["diagrams"][0]["mmd_path"].as_str().unwrap();
    assert_eq!(stored_path, "diagrams/auth-flow.mmd");

    let loaded = folder.load_meta().unwrap();
    assert_eq!(loaded, meta);
}

#[rstest]
fn load_rejects_parent_traversal(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let meta_path = folder.meta_path();
    std::fs::write(
        &meta_path,
        r#"{
  "session_id": "s1",
  "active_diagram_id": null,
  "diagrams": [
    {
      "diagram_id": "d1",
      "name": "Bad",
      "kind": "sequence",
      "mmd_path": "../escape.mmd"
    }
  ]
}"#,
    )
    .unwrap();

    let err = folder.load_meta().unwrap_err();
    match err {
        StoreError::InvalidRelativePath { .. } => {}
        other => panic!("expected InvalidRelativePath, got: {other:?}"),
    }
}

#[rstest]
fn load_or_init_session_creates_seed_diagram_when_meta_is_missing(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;
    let meta_path = folder.meta_path();
    assert!(!meta_path.exists());

    let session = folder.load_or_init_session().unwrap();
    assert_eq!(session.session_id(), &SessionId::new("s:my-session").unwrap());
    assert!(meta_path.is_file());

    let diagram_id = DiagramId::new("flow").unwrap();
    assert_eq!(session.active_diagram_id(), Some(&diagram_id));
    let diagram = session.diagrams().get(&diagram_id).expect("seed diagram");
    match diagram.ast() {
        DiagramAst::Flowchart(ast) => {
            assert_eq!(ast.nodes().len(), 1);
            let node_id = ObjectId::new("n:hello").unwrap();
            let node = ast.nodes().get(&node_id).expect("seed node");
            assert_eq!(node.label(), "Hello");
        }
        other => panic!("expected flowchart seed diagram, got: {other:?}"),
    }

    assert!(session.walkthroughs().is_empty());
    assert!(session.xrefs().is_empty());

    let loaded = folder.load_session().unwrap();
    assert_eq!(loaded.session_id(), session.session_id());
    assert_eq!(loaded.active_diagram_id(), session.active_diagram_id());
    assert_eq!(loaded.walkthroughs(), session.walkthroughs());
    assert_eq!(loaded.xrefs(), session.xrefs());
    assert_eq!(loaded.diagrams().len(), 1);
}

#[rstest]
fn load_or_init_session_refuses_to_seed_when_diagram_files_exist(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s:prior").unwrap());
    let d1 = DiagramId::new("d1").unwrap();
    let mut ast = FlowchartAst::default();
    ast.nodes_mut().insert(ObjectId::new("n:keep").unwrap(), FlowNode::new("Keep"));
    session
        .diagrams_mut()
        .insert(d1.clone(), Diagram::new(d1.clone(), "Prior", DiagramAst::Flowchart(ast)));
    folder.save_session(&session).unwrap();

    let meta_path = folder.meta_path();
    assert!(meta_path.is_file());
    std::fs::remove_file(&meta_path).unwrap();
    assert!(ctx.session_dir.join("diagrams/d1.mmd").is_file());

    let err = folder.load_or_init_session().unwrap_err();
    match err {
        StoreError::MetaMissingWithExistingDiagrams { meta_path: reported, .. } => {
            assert_eq!(reported, meta_path);
        }
        other => panic!("expected MetaMissingWithExistingDiagrams, got: {other:?}"),
    }

    assert!(!meta_path.exists(), "load_or_init must not write a seed meta when diagrams exist");
}

#[rstest]
fn load_or_init_session_does_not_hide_missing_diagram_errors(ctx: SessionFolderTestCtx) {
    let missing_mmd_path = ctx.session_dir.join("diagrams/missing.mmd");
    let meta = SessionMeta {
        session_id: SessionId::new("s1").unwrap(),
        active_diagram_id: None,
        active_walkthrough_id: None,
        walkthrough_ids: Some(Vec::new()),
        diagrams: vec![SessionMetaDiagram {
            diagram_id: DiagramId::new("d1").unwrap(),
            name: "Missing diagram".to_owned(),
            kind: DiagramKind::Flowchart,
            mmd_path: missing_mmd_path.clone(),
            rev: 0,
        }],
        xrefs: Vec::new(),
        selected_object_refs: Vec::new(),
    };
    ctx.folder.save_meta(&meta).unwrap();

    let err = ctx.folder.load_or_init_session().unwrap_err();
    match err {
        StoreError::Io { path, source } => {
            assert_eq!(path, missing_mmd_path);
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        }
        other => panic!("expected Io NotFound, got: {other:?}"),
    }
}

#[rstest]
fn save_active_diagram_id_updates_meta_and_loads_back(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let d1 = DiagramId::new("d1").unwrap();
    let mut d1_ast = FlowchartAst::default();
    d1_ast.nodes_mut().insert(ObjectId::new("n:start").unwrap(), FlowNode::new("Start"));
    session
        .diagrams_mut()
        .insert(d1.clone(), Diagram::new(d1.clone(), "Diagram 1", DiagramAst::Flowchart(d1_ast)));

    let d2 = DiagramId::new("d2").unwrap();
    let mut d2_ast = FlowchartAst::default();
    d2_ast.nodes_mut().insert(ObjectId::new("n:end").unwrap(), FlowNode::new("End"));
    session
        .diagrams_mut()
        .insert(d2.clone(), Diagram::new(d2.clone(), "Diagram 2", DiagramAst::Flowchart(d2_ast)));

    session.set_active_diagram_id(Some(d1));
    folder.save_session(&session).unwrap();

    session.set_active_diagram_id(Some(d2.clone()));
    folder.save_active_diagram_id(&session).unwrap();

    let meta = folder.load_meta().unwrap();
    assert_eq!(meta.active_diagram_id, Some(d2.clone()));

    let loaded = folder.load_session().unwrap();
    assert_eq!(loaded.active_diagram_id(), Some(&d2));
}

#[rstest]
fn save_active_diagram_id_does_not_drop_concurrent_save_session_additions(
    ctx: SessionFolderTestCtx,
) {
    use std::sync::Arc;
    use std::sync::Barrier;

    const DIAGRAMS: usize = 60;

    fn diagram(id: &DiagramId, node: &str) -> Diagram {
        let mut ast = FlowchartAst::default();
        ast.nodes_mut().insert(ObjectId::new(node).unwrap(), FlowNode::new("N"));
        Diagram::new(id.clone(), "D", DiagramAst::Flowchart(ast))
    }

    let mut session = Session::new(SessionId::new("s:concurrent").unwrap());
    let first = DiagramId::new("d-000").unwrap();
    session.diagrams_mut().insert(first.clone(), diagram(&first, "n:000"));
    session.set_active_diagram_id(Some(first));
    ctx.folder.save_session(&session).unwrap();

    let writer_folder = SessionFolder::new(&ctx.session_dir);
    let patcher_folder = SessionFolder::new(&ctx.session_dir);
    let observer_folder = SessionFolder::new(&ctx.session_dir);
    let barrier = Arc::new(Barrier::new(3));
    let writer_barrier = barrier.clone();
    let observer_barrier = barrier.clone();
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observer_done = done.clone();

    let writer = std::thread::spawn(move || {
        writer_barrier.wait();
        let mut session = session;
        for i in 1..DIAGRAMS {
            let id = DiagramId::new(format!("d-{i:03}")).unwrap();
            session.diagrams_mut().insert(id.clone(), diagram(&id, &format!("n:{i:03}")));
            session.set_active_diagram_id(Some(id));
            writer_folder.save_session(&session).unwrap();
        }
    });

    let patcher = std::thread::spawn(move || {
        barrier.wait();
        for _ in 0..DIAGRAMS * 4 {
            let on_disk = patcher_folder.load_session().unwrap();
            patcher_folder.save_active_diagram_id(&on_disk).unwrap();
        }
    });

    let observer = std::thread::spawn(move || {
        observer_barrier.wait();
        let mut high_water = 0usize;
        while !observer_done.load(std::sync::atomic::Ordering::Relaxed) {
            let count = observer_folder.load_meta().unwrap().diagrams.len();
            assert!(
                count >= high_water,
                "meta diagram count dropped from {high_water} to {count} during concurrent saves",
            );
            high_water = count;
        }
    });

    writer.join().unwrap();
    patcher.join().unwrap();
    done.store(true, std::sync::atomic::Ordering::Relaxed);
    observer.join().unwrap();

    let meta = ctx.folder.load_meta().unwrap();
    let indexed: std::collections::BTreeSet<_> =
        meta.diagrams.iter().map(|d| d.diagram_id.clone()).collect();
    for i in 0..DIAGRAMS {
        let id = DiagramId::new(format!("d-{i:03}")).unwrap();
        assert!(
            indexed.contains(&id),
            "diagram {id} missing from meta index after concurrent saves"
        );
    }
    let loaded = ctx.folder.load_session().unwrap();
    assert_eq!(loaded.diagrams().len(), DIAGRAMS);
}

#[rstest]
fn session_update_waits_to_load_until_prior_writer_commits(ctx: SessionFolderTestCtx) {
    fn diagram(id: &DiagramId, node_id: &str, label: &str) -> Diagram {
        let mut ast = FlowchartAst::default();
        ast.nodes_mut().insert(ObjectId::new(node_id).unwrap(), FlowNode::new(label));
        Diagram::new(id.clone(), label, DiagramAst::Flowchart(ast))
    }

    fn add_node(session: &mut Session, diagram_id: &DiagramId, node_id: &str, label: &str) {
        let mut diagram = session.diagrams().get(diagram_id).cloned().expect("diagram");
        let DiagramAst::Flowchart(mut ast) = diagram.ast().clone() else {
            panic!("expected flowchart");
        };
        ast.nodes_mut().insert(ObjectId::new(node_id).unwrap(), FlowNode::new(label));
        diagram.set_ast(DiagramAst::Flowchart(ast)).unwrap();
        diagram.bump_rev();
        session.diagrams_mut().insert(diagram_id.clone(), diagram);
    }

    let d_a = DiagramId::new("d-a").unwrap();
    let d_b = DiagramId::new("d-b").unwrap();
    let mut session = Session::new(SessionId::new("s:tx").unwrap());
    session.diagrams_mut().insert(d_a.clone(), diagram(&d_a, "n:a0", "A"));
    session.diagrams_mut().insert(d_b.clone(), diagram(&d_b, "n:b0", "B"));
    ctx.folder.save_session(&session).unwrap();

    let first_folder = SessionFolder::new(&ctx.session_dir);
    let mut first_update = first_folder.begin_session_update().unwrap();
    add_node(first_update.session_mut(), &d_b, "n:b1", "B1");

    let second_folder = SessionFolder::new(&ctx.session_dir);
    let d_a_for_thread = d_a.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let second_writer = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let mut second_update = second_folder.begin_session_update().unwrap();
        add_node(second_update.session_mut(), &d_a_for_thread, "n:a1", "A1");
        second_update.commit().unwrap();
    });

    started_rx.recv().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    first_update.commit().unwrap();
    second_writer.join().unwrap();

    let loaded = ctx.folder.load_session().unwrap();
    let DiagramAst::Flowchart(ast_a) = loaded.diagrams().get(&d_a).unwrap().ast() else {
        panic!("expected d-a flowchart");
    };
    assert!(ast_a.nodes().contains_key(&ObjectId::new("n:a1").unwrap()));
    let DiagramAst::Flowchart(ast_b) = loaded.diagrams().get(&d_b).unwrap().ast() else {
        panic!("expected d-b flowchart");
    };
    assert!(ast_b.nodes().contains_key(&ObjectId::new("n:b1").unwrap()));
}

#[rstest]
fn save_diagram_meta_stores_relative_paths_and_load_resolves_them(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let mmd_path = session_dir.join("diagrams/auth-flow.mmd");

    let mut by_mermaid_id = std::collections::BTreeMap::new();
    by_mermaid_id.insert("authorize".to_owned(), "n:authorize".to_owned());

    let meta = DiagramMeta {
        diagram_id: DiagramId::new("d1").unwrap(),
        mmd_path: mmd_path.clone(),
        stable_id_map: DiagramStableIdMap {
            by_mermaid_id,
            by_name: std::collections::BTreeMap::new(),
        },
        xrefs: vec![DiagramXRef {
            xref_id: "x1".to_owned(),
            from: "d:d1/flow/node/n:authorize".to_owned(),
            to: "d:d2/seq/message/m:0042".to_owned(),
            kind: "implements".to_owned(),
            label: Some("Auth step".to_owned()),
            status: StoreXRefStatus::DanglingTo,
        }],
        flow_edges: Vec::new(),
        sequence_messages: Vec::new(),
        sequence_blocks: Vec::new(),
        default_symbol_repository_id: None,
        flow_node_notes: Default::default(),
        sequence_participant_notes: Default::default(),
        flow_node_symbols: Default::default(),
        sequence_participant_symbols: Default::default(),
    };

    folder.save_diagram_meta(&meta).unwrap();

    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();
    assert_eq!(sidecar_path, session_dir.join("diagrams/auth-flow.meta.json"));

    let meta_str = std::fs::read_to_string(&sidecar_path).unwrap();
    let meta_json: serde_json::Value = serde_json::from_str(&meta_str).unwrap();
    assert_eq!(meta_json["mmd_path"].as_str().unwrap(), "diagrams/auth-flow.mmd");
    assert!(meta_json.get("stable_id_map").is_some());
    assert!(meta_json.get("xrefs").is_some());

    let loaded = folder.load_diagram_meta(&mmd_path).unwrap();
    assert_eq!(loaded, meta);
}

#[rstest]
fn save_diagram_meta_rejects_paths_outside_session(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let meta = DiagramMeta {
        diagram_id: DiagramId::new("d1").unwrap(),
        mmd_path: ctx.tmp.path().join("escape.mmd"),
        stable_id_map: DiagramStableIdMap::default(),
        xrefs: Vec::new(),
        flow_edges: Vec::new(),
        sequence_messages: Vec::new(),
        sequence_blocks: Vec::new(),
        default_symbol_repository_id: None,
        flow_node_notes: Default::default(),
        sequence_participant_notes: Default::default(),
        flow_node_symbols: Default::default(),
        sequence_participant_symbols: Default::default(),
    };

    let err = folder.save_diagram_meta(&meta).unwrap_err();
    match err {
        StoreError::PathOutsideSession { .. } => {}
        other => panic!("expected PathOutsideSession, got: {other:?}"),
    }
}

#[rstest]
fn load_diagram_meta_rejects_parent_traversal(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;
    std::fs::create_dir_all(session_dir.join("diagrams")).unwrap();

    let mmd_path = session_dir.join("diagrams/auth-flow.mmd");
    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();

    std::fs::write(
        &sidecar_path,
        r#"{
  "diagram_id": "d1",
  "mmd_path": "../escape.mmd",
  "stable_id_map": { "by_mermaid_id": {}, "by_name": {} },
  "xrefs": []
}"#,
    )
    .unwrap();

    let err = folder.load_diagram_meta(&mmd_path).unwrap_err();
    match err {
        StoreError::InvalidRelativePath { .. } => {}
        other => panic!("expected InvalidRelativePath, got: {other:?}"),
    }
}

#[rstest]
fn save_session_exports_canonical_mmd_and_text_unicode(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    // Sequence diagram
    let seq_id = DiagramId::new("d1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:alice").unwrap();
    let p_bob = ObjectId::new("p:bob").unwrap();
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").unwrap(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));
    let seq_diagram =
        Diagram::new(seq_id.clone(), "Seq Example", DiagramAst::Sequence(seq_ast.clone()));
    let seq_expected_mmd = export_sequence_diagram(&seq_ast).unwrap();
    let seq_layout = layout_sequence(&seq_ast).unwrap();
    let mut seq_expected_text = render_sequence_unicode(&seq_ast, &seq_layout).unwrap();
    seq_expected_text.push('\n');

    // Flowchart diagram
    let flow_id = DiagramId::new("d2").unwrap();
    let mut flow_ast = FlowchartAst::default();
    let n_start = ObjectId::new("n:start").unwrap();
    let n_end = ObjectId::new("n:end").unwrap();
    flow_ast.nodes_mut().insert(n_start.clone(), FlowNode::new("Start"));
    flow_ast.nodes_mut().insert(n_end.clone(), FlowNode::new("End"));
    flow_ast
        .edges_mut()
        .insert(ObjectId::new("e:0001").unwrap(), FlowEdge::new(n_start.clone(), n_end.clone()));
    let flow_diagram =
        Diagram::new(flow_id.clone(), "Flow Example", DiagramAst::Flowchart(flow_ast.clone()));
    let flow_expected_mmd = export_flowchart(&flow_ast).unwrap();
    let flow_layout = layout_flowchart(&flow_ast).unwrap();
    let mut flow_expected_text = render_flowchart_unicode(&flow_ast, &flow_layout).unwrap();
    flow_expected_text.push('\n');

    session.diagrams_mut().insert(seq_id.clone(), seq_diagram);
    session.diagrams_mut().insert(flow_id.clone(), flow_diagram);
    session.set_active_diagram_id(Some(seq_id.clone()));

    folder.save_session(&session).unwrap();
    folder.flush_ascii_exports();

    // Session meta stored relative paths.
    let meta_str = std::fs::read_to_string(folder.meta_path()).unwrap();
    let meta_json: serde_json::Value = serde_json::from_str(&meta_str).unwrap();
    assert_eq!(meta_json["session_id"].as_str().unwrap(), "s1");
    assert_eq!(meta_json["active_diagram_id"].as_str().unwrap(), "d1");

    let mut mmd_paths = meta_json["diagrams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["mmd_path"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    mmd_paths.sort();
    assert_eq!(mmd_paths, vec!["diagrams/d1.mmd", "diagrams/d2.mmd"]);

    // Exported files are written under the session folder.
    let seq_mmd_path = session_dir.join("diagrams/d1.mmd");
    let seq_text_path = session_dir.join("diagrams/d1.ascii.txt");
    assert_eq!(std::fs::read_to_string(&seq_mmd_path).unwrap(), seq_expected_mmd);
    assert_eq!(std::fs::read_to_string(&seq_text_path).unwrap(), seq_expected_text);

    let flow_mmd_path = session_dir.join("diagrams/d2.mmd");
    let flow_text_path = session_dir.join("diagrams/d2.ascii.txt");
    assert_eq!(std::fs::read_to_string(&flow_mmd_path).unwrap(), flow_expected_mmd);
    assert_eq!(std::fs::read_to_string(&flow_text_path).unwrap(), flow_expected_text);
}

#[rstest]
fn save_and_load_session_round_trips_symbol_anchors_without_changing_mmd(
    ctx: SessionFolderTestCtx,
) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s:symbols").unwrap());
    let diagram_id = DiagramId::new("read-search-hybrid-flow").unwrap();
    let participant_id = ObjectId::new("p:CLI").unwrap();
    let mut ast = SequenceAst::default();
    let mut participant = SequenceParticipant::new("CLI");
    participant.set_note(Some("Entry point"));
    participant.set_symbol(Some(SymbolAnchor::new("sym-16c57df0026ced40", None).expect("symbol")));
    ast.participants_mut().insert(participant_id.clone(), participant);

    let expected_mmd = export_sequence_diagram(&ast).unwrap();
    let mut diagram =
        Diagram::new(diagram_id.clone(), "ASK Read Search", DiagramAst::Sequence(ast));
    diagram.set_default_symbol_repository_id(Some("rust-neo4j-ask-57bbe2dfb196"));
    session.diagrams_mut().insert(diagram_id.clone(), diagram);
    session.set_active_diagram_id(Some(diagram_id.clone()));

    folder.save_session(&session).unwrap();

    let mmd_path = session_dir.join("diagrams/read-search-hybrid-flow.mmd");
    assert_eq!(std::fs::read_to_string(&mmd_path).unwrap(), expected_mmd);

    let sidecar_path = session_dir.join("diagrams/read-search-hybrid-flow.meta.json");
    let sidecar: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&sidecar_path).unwrap()).unwrap();
    assert_eq!(
        sidecar["default_symbol_repository_id"].as_str(),
        Some("rust-neo4j-ask-57bbe2dfb196")
    );
    assert_eq!(
        sidecar["sequence_participant_symbols"]["p:CLI"]["stable_symbol_id"].as_str(),
        Some("sym-16c57df0026ced40")
    );
    assert!(sidecar["sequence_participant_symbols"]["p:CLI"].get("repository_id").is_none());

    let loaded = folder.load_session().unwrap();
    let loaded_diagram = loaded.diagrams().get(&diagram_id).expect("diagram");
    assert_eq!(loaded_diagram.default_symbol_repository_id(), Some("rust-neo4j-ask-57bbe2dfb196"));
    let DiagramAst::Sequence(loaded_ast) = loaded_diagram.ast() else {
        panic!("expected sequence");
    };
    let loaded_participant = loaded_ast.participants().get(&participant_id).expect("participant");
    assert_eq!(
        loaded_participant.symbol().map(|symbol| symbol.stable_symbol_id()),
        Some("sym-16c57df0026ced40")
    );
}

#[cfg(unix)]
#[rstest]
fn save_session_refuses_writing_through_symlinked_diagrams_dir(ctx: SessionFolderTestCtx) {
    use std::os::unix::fs::symlink;

    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let outside = ctx.tmp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();

    let diagrams_dir = session_dir.join("diagrams");
    symlink(&outside, &diagrams_dir).unwrap();

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let seq_id = DiagramId::new("d1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:alice").unwrap();
    let p_bob = ObjectId::new("p:bob").unwrap();
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").unwrap(),
        p_alice,
        p_bob,
        SequenceMessageKind::Sync,
        "Hello",
        1,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id, "Seq Example", DiagramAst::Sequence(seq_ast)));

    let err = folder.save_session(&session).unwrap_err();
    match err {
        StoreError::SymlinkRefused { path } => assert_eq!(path, diagrams_dir),
        other => panic!("expected SymlinkRefused, got: {other:?}"),
    }
}

#[rstest]
fn save_walkthrough_writes_json_and_text_export_and_load_round_trips(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let walkthrough_id = WalkthroughId::new("w1").unwrap();
    let mut walkthrough = Walkthrough::new(walkthrough_id.clone(), "Invite acceptance");
    walkthrough.set_source(Some("docs/protocol-01.md#2.5".to_owned()));
    walkthrough.bump_rev();
    walkthrough.bump_rev();

    let node_start_id = WalkthroughNodeId::new("n:start").unwrap();
    let mut node_start = WalkthroughNode::new(node_start_id.clone(), "Start");
    node_start.set_body_md(Some("Beginning of the flow.".to_owned()));
    node_start.tags_mut().push("entry".to_owned());
    node_start.set_status(Some("draft".to_owned()));

    let obj_ref = ObjectRef::new(
        DiagramId::new("d1").unwrap(),
        CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()]).unwrap(),
        ObjectId::new("m:0001").unwrap(),
    );
    node_start.refs_mut().push(obj_ref);
    walkthrough.nodes_mut().push(node_start);

    let node_end_id = WalkthroughNodeId::new("n:end").unwrap();
    let mut node_end = WalkthroughNode::new(node_end_id.clone(), "End");
    node_end.tags_mut().push("exit".to_owned());
    walkthrough.nodes_mut().push(node_end);

    let mut edge = WalkthroughEdge::new(node_start_id, node_end_id, "next");
    edge.set_label(Some("continue".to_owned()));
    walkthrough.edges_mut().push(edge);

    folder.save_walkthrough(&walkthrough).unwrap();
    folder.flush_ascii_exports();

    let wt_path = folder.walkthrough_json_path(&walkthrough_id);
    assert_eq!(wt_path, session_dir.join("walkthroughs/w1.wt.json"));

    let wt_json_str = std::fs::read_to_string(&wt_path).unwrap();
    let wt_json: serde_json::Value = serde_json::from_str(&wt_json_str).unwrap();
    assert_eq!(wt_json["walkthrough_id"].as_str().unwrap(), "w1");
    assert_eq!(wt_json["title"].as_str().unwrap(), "Invite acceptance");
    assert_eq!(wt_json["rev"].as_u64().unwrap(), 2);
    assert_eq!(wt_json["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(wt_json["edges"].as_array().unwrap().len(), 1);

    let text_path = folder.walkthrough_ascii_path(&walkthrough_id);
    assert_eq!(text_path, session_dir.join("walkthroughs/w1.ascii.txt"));
    let mut expected_text = render_walkthrough_unicode(&walkthrough).unwrap();
    if !expected_text.ends_with('\n') {
        expected_text.push('\n');
    }
    assert_eq!(std::fs::read_to_string(&text_path).unwrap(), expected_text);

    let loaded = folder.load_walkthrough(&walkthrough_id).unwrap();
    assert_eq!(loaded, walkthrough);
}

#[rstest]
fn default_diagram_mmd_path_encodes_unsafe_ids(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let diagram_id = DiagramId::new("d:1").unwrap();
    let mmd_path = folder.default_diagram_mmd_path(&diagram_id);
    assert_eq!(mmd_path, session_dir.join("diagrams/~643a31.mmd"));
}

#[rstest]
fn save_and_load_session_supports_encoded_ids_for_diagrams_and_walkthroughs(
    ctx: SessionFolderTestCtx,
) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let diagram_id = DiagramId::new("d:1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:alice").unwrap();
    let p_bob = ObjectId::new("p:bob").unwrap();
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").unwrap(),
        p_alice,
        p_bob,
        SequenceMessageKind::Sync,
        "Hello",
        1,
    ));
    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Seq Example", DiagramAst::Sequence(seq_ast)),
    );

    let walkthrough_id = WalkthroughId::new("w:1").unwrap();
    let mut walkthrough = Walkthrough::new(walkthrough_id.clone(), "Walkthrough");
    walkthrough
        .nodes_mut()
        .push(WalkthroughNode::new(WalkthroughNodeId::new("n:start").unwrap(), "Start"));
    session.walkthroughs_mut().insert(walkthrough_id.clone(), walkthrough);

    folder.save_session(&session).unwrap();

    assert!(session_dir.join("diagrams/~643a31.mmd").is_file());
    assert!(session_dir.join("walkthroughs/~773a31.wt.json").is_file());

    let loaded = folder.load_session().unwrap();
    assert!(loaded.diagrams().contains_key(&diagram_id));
    assert!(loaded.walkthroughs().contains_key(&walkthrough_id));
}

#[rstest]
fn load_walkthrough_falls_back_to_legacy_filename_for_unsafe_ids(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let walkthrough_id = WalkthroughId::new("w:1").unwrap();
    let walkthrough = Walkthrough::new(walkthrough_id.clone(), "Legacy");
    folder.save_walkthrough(&walkthrough).unwrap();

    let encoded_path = folder.walkthrough_json_path(&walkthrough_id);
    let legacy_path = folder.legacy_walkthrough_json_path(&walkthrough_id).unwrap();
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::rename(&encoded_path, &legacy_path).unwrap();

    let loaded = folder.load_walkthrough(&walkthrough_id).unwrap();
    assert_eq!(loaded, walkthrough);
}

#[rstest]
fn load_walkthrough_rejects_legacy_fallback_with_path_separators(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let walkthrough_id = WalkthroughId::new(r"..\..\outside\loot").unwrap();
    let err = folder.load_walkthrough(&walkthrough_id).unwrap_err();

    match err {
        StoreError::InvalidRelativePath { field, value } => {
            assert_eq!(field, "walkthrough_id");
            assert_eq!(value, std::path::PathBuf::from(walkthrough_id.as_str()));
        }
        other => panic!("expected InvalidRelativePath, got {other:?}"),
    }
}

#[rstest]
fn walkthrough_files_remain_loadable_after_moving_session_folder(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let walkthrough_id = WalkthroughId::new("w1").unwrap();
    let walkthrough = Walkthrough::new(walkthrough_id.clone(), "Movable");
    folder.save_walkthrough(&walkthrough).unwrap();

    let moved_dir = ctx.tmp.path().join("my-session-renamed");
    std::fs::rename(session_dir, &moved_dir).unwrap();

    let moved_folder = SessionFolder::new(&moved_dir);
    let loaded = moved_folder.load_walkthrough(&walkthrough_id).unwrap();
    assert_eq!(loaded, walkthrough);
}

#[rstest]
fn removed_walkthrough_does_not_resurrect_after_save_load(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let w1 = WalkthroughId::new("w1").unwrap();
    let w2 = WalkthroughId::new("w2").unwrap();
    session.walkthroughs_mut().insert(w1.clone(), Walkthrough::new(w1.clone(), "One"));
    session.walkthroughs_mut().insert(w2.clone(), Walkthrough::new(w2.clone(), "Two"));

    folder.save_session(&session).unwrap();

    session.walkthroughs_mut().remove(&w2);
    folder.save_session(&session).unwrap();

    let loaded = folder.load_session().unwrap();
    assert!(loaded.walkthroughs().contains_key(&w1));
    assert!(!loaded.walkthroughs().contains_key(&w2));
}

#[cfg(unix)]
#[rstest]
fn walkthrough_gc_refuses_symlinked_walkthroughs_dir(ctx: SessionFolderTestCtx) {
    use std::os::unix::fs::symlink;

    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;

    let outside = ctx.tmp.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let victim = outside.join("stale.wt.json");
    std::fs::write(&victim, "{}").unwrap();

    let walkthroughs_dir = session_dir.join("walkthroughs");
    symlink(&outside, &walkthroughs_dir).unwrap();

    let err = folder.garbage_collect_walkthrough_files(&[]).unwrap_err();
    match err {
        StoreError::SymlinkRefused { path } => assert_eq!(path, walkthroughs_dir),
        other => panic!("expected SymlinkRefused, got {other:?}"),
    }

    assert!(victim.is_file());
}

#[rstest]
fn legacy_meta_without_walkthrough_ids_scans_directory(ctx: SessionFolderTestCtx) {
    let session_dir = &ctx.session_dir;
    let folder = &ctx.folder;
    std::fs::create_dir_all(session_dir.join("walkthroughs")).unwrap();

    std::fs::write(
        folder.meta_path(),
        r#"{
  "session_id": "s1",
  "active_diagram_id": null,
  "active_walkthrough_id": null,
  "diagrams": [],
  "xrefs": []
}"#,
    )
    .unwrap();

    std::fs::write(
        session_dir.join("walkthroughs/w1.wt.json"),
        r#"{
  "walkthrough_id": "w1",
  "title": "Legacy",
  "rev": 0,
  "nodes": [],
  "edges": [],
  "source": null
}"#,
    )
    .unwrap();

    let loaded = folder.load_session().unwrap();
    let w1 = WalkthroughId::new("w1").unwrap();
    assert!(loaded.walkthroughs().contains_key(&w1));
}

#[rstest]
fn save_session_recovers_preexisting_write_lock_file(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;
    let lock_path = folder.root().join(".nereid-session.write.lock");
    std::fs::write(&lock_path, "pid=999999 acquired_unix_ms=0\n").unwrap();

    let session = Session::new(SessionId::new("s1").unwrap());
    folder.save_session(&session).unwrap();

    assert!(folder.meta_path().is_file());
}

#[cfg(unix)]
#[rstest]
fn save_session_rolls_back_diagram_when_sidecar_write_fails(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let d1 = DiagramId::new("d1").unwrap();
    let n_a = ObjectId::new("n:a").unwrap();
    let n_b = ObjectId::new("n:b").unwrap();

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let mut ast0 = FlowchartAst::default();
    ast0.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    session
        .diagrams_mut()
        .insert(d1.clone(), Diagram::new(d1.clone(), "D", DiagramAst::Flowchart(ast0)));
    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&d1);
    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();
    assert!(mmd_path.is_file() && sidecar_path.is_file());

    let sidecar_backing = sidecar_path.with_extension("json.backing");
    std::fs::rename(&sidecar_path, &sidecar_backing).unwrap();
    std::os::unix::fs::symlink(&sidecar_backing, &sidecar_path).unwrap();

    {
        let diagram = session.diagrams_mut().get_mut(&d1).unwrap();
        let DiagramAst::Flowchart(mut ast1) = diagram.ast().clone() else { panic!() };
        ast1.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
        diagram.set_ast(DiagramAst::Flowchart(ast1)).unwrap();
        diagram.bump_rev();
    }
    let err = folder.save_session(&session).unwrap_err();
    match err {
        StoreError::SymlinkRefused { .. } => {}
        other => panic!("expected SymlinkRefused from sidecar write, got: {other:?}"),
    }

    std::fs::remove_file(&sidecar_path).unwrap();
    std::fs::rename(&sidecar_backing, &sidecar_path).unwrap();

    let loaded = folder.load_session().unwrap();
    let diagram = loaded.diagrams().get(&d1).expect("diagram");
    let DiagramAst::Flowchart(ast) = diagram.ast() else { panic!() };
    assert!(ast.nodes().contains_key(&n_a), "n:a must survive");
    assert!(
        !ast.nodes().contains_key(&n_b),
        "sidecar-first ordering must not have written the new .mmd; n:b leaked from a failed save",
    );
}

#[cfg(unix)]
#[rstest]
fn save_session_restores_sidecar_when_mmd_write_fails(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let d1 = DiagramId::new("d1").unwrap();
    let n_a = ObjectId::new("n:a").unwrap();
    let n_b = ObjectId::new("n:b").unwrap();

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let mut ast0 = FlowchartAst::default();
    ast0.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    session
        .diagrams_mut()
        .insert(d1.clone(), Diagram::new(d1.clone(), "D", DiagramAst::Flowchart(ast0)));
    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&d1);
    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();
    let sidecar_before = std::fs::read(&sidecar_path).unwrap();

    let mmd_backing = mmd_path.with_extension("mmd.backing");
    std::fs::rename(&mmd_path, &mmd_backing).unwrap();
    std::os::unix::fs::symlink(&mmd_backing, &mmd_path).unwrap();

    {
        let diagram = session.diagrams_mut().get_mut(&d1).unwrap();
        let DiagramAst::Flowchart(mut ast1) = diagram.ast().clone() else { panic!() };
        ast1.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
        diagram.set_ast(DiagramAst::Flowchart(ast1)).unwrap();
        diagram.bump_rev();
    }

    let err = folder.save_session(&session).unwrap_err();
    match err {
        StoreError::SymlinkRefused { path } => assert_eq!(path, mmd_path),
        other => panic!("expected SymlinkRefused from mmd write, got: {other:?}"),
    }

    let sidecar_after = std::fs::read(&sidecar_path).unwrap();
    assert_eq!(sidecar_after, sidecar_before, "failed mmd write must restore old sidecar");

    std::fs::remove_file(&mmd_path).unwrap();
    std::fs::rename(&mmd_backing, &mmd_path).unwrap();

    let loaded = folder.load_session().unwrap();
    let diagram = loaded.diagrams().get(&d1).expect("diagram");
    let DiagramAst::Flowchart(ast) = diagram.ast() else { panic!() };
    assert!(ast.nodes().contains_key(&n_a), "n:a must survive");
    assert!(
        !ast.nodes().contains_key(&n_b),
        "failed .mmd write must leave the diagram at the prior revision",
    );
}

#[cfg(unix)]
#[rstest]
fn save_session_rolls_back_diagram_artifacts_when_meta_write_fails(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let d1 = DiagramId::new("d1").unwrap();
    let n_a = ObjectId::new("n:a").unwrap();
    let n_b = ObjectId::new("n:b").unwrap();

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let mut ast0 = FlowchartAst::default();
    ast0.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
    session
        .diagrams_mut()
        .insert(d1.clone(), Diagram::new(d1.clone(), "D", DiagramAst::Flowchart(ast0)));
    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&d1);
    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();
    let mmd_before = std::fs::read(&mmd_path).unwrap();
    let sidecar_before = std::fs::read(&sidecar_path).unwrap();

    let meta_path = folder.meta_path();
    let meta_backing = folder.root().join("nereid-session.meta.json.backing");
    std::fs::rename(&meta_path, &meta_backing).unwrap();
    std::os::unix::fs::symlink(&meta_backing, &meta_path).unwrap();

    {
        let diagram = session.diagrams_mut().get_mut(&d1).unwrap();
        let DiagramAst::Flowchart(mut ast1) = diagram.ast().clone() else { panic!() };
        ast1.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
        diagram.set_ast(DiagramAst::Flowchart(ast1)).unwrap();
        diagram.bump_rev();
    }

    let err = folder.save_session(&session).unwrap_err();
    match err {
        StoreError::SymlinkRefused { path } => assert_eq!(path, meta_path),
        other => panic!("expected SymlinkRefused from meta write, got: {other:?}"),
    }

    assert_eq!(std::fs::read(&mmd_path).unwrap(), mmd_before, "mmd must roll back");
    assert_eq!(std::fs::read(&sidecar_path).unwrap(), sidecar_before, "sidecar must roll back",);

    std::fs::remove_file(&meta_path).unwrap();
    std::fs::rename(&meta_backing, &meta_path).unwrap();

    let loaded = folder.load_session().unwrap();
    let diagram = loaded.diagrams().get(&d1).expect("diagram");
    let DiagramAst::Flowchart(ast) = diagram.ast() else { panic!() };
    assert!(ast.nodes().contains_key(&n_a), "n:a must survive");
    assert!(
        !ast.nodes().contains_key(&n_b),
        "failed meta commit must leave the diagram at the prior revision",
    );
}

#[cfg(unix)]
#[rstest]
fn save_session_rolls_back_walkthrough_artifacts_when_meta_write_fails(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let w1 = WalkthroughId::new("w:1").unwrap();
    session.walkthroughs_mut().insert(w1.clone(), Walkthrough::new(w1.clone(), "Before"));
    folder.save_session(&session).unwrap();
    folder.flush_ascii_exports();

    let wt_path = folder.walkthrough_json_path(&w1);
    let ascii_path = folder.walkthrough_ascii_path(&w1);
    let wt_before = std::fs::read(&wt_path).unwrap();
    let ascii_before = std::fs::read(&ascii_path).unwrap();

    let meta_path = folder.meta_path();
    let backing = folder.root().join("nereid-session.meta.json.backing");
    std::fs::rename(&meta_path, &backing).unwrap();
    std::os::unix::fs::symlink(&backing, &meta_path).unwrap();

    {
        let walkthrough = session.walkthroughs_mut().get_mut(&w1).unwrap();
        walkthrough.set_title("After");
        walkthrough.bump_rev();
    }

    let err = folder.save_session(&session).unwrap_err();
    match err {
        StoreError::SymlinkRefused { path } => assert_eq!(path, meta_path),
        other => panic!("expected SymlinkRefused from meta write, got: {other:?}"),
    }
    folder.flush_ascii_exports();

    assert_eq!(std::fs::read(&wt_path).unwrap(), wt_before, "walkthrough JSON must roll back");
    assert_eq!(
        std::fs::read(&ascii_path).unwrap(),
        ascii_before,
        "walkthrough ASCII must roll back",
    );
}

#[cfg(unix)]
#[rstest]
fn save_session_keeps_removed_walkthrough_loadable_when_meta_commit_fails(
    ctx: SessionFolderTestCtx,
) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let w1 = WalkthroughId::new("w:1").unwrap();
    let w2 = WalkthroughId::new("w:2").unwrap();
    session.walkthroughs_mut().insert(w1.clone(), Walkthrough::new(w1.clone(), "One"));
    session.walkthroughs_mut().insert(w2.clone(), Walkthrough::new(w2.clone(), "Two"));
    folder.save_session(&session).unwrap();

    let w2_path = folder.walkthrough_json_path(&w2);
    assert!(w2_path.is_file(), "precondition: w2 file written");

    let meta_path = folder.meta_path();
    let backing = folder.root().join("nereid-session.meta.json.backing");
    std::fs::rename(&meta_path, &backing).unwrap();
    std::os::unix::fs::symlink(&backing, &meta_path).unwrap();

    session.walkthroughs_mut().remove(&w2);
    let err = folder.save_session(&session).unwrap_err();
    match err {
        StoreError::SymlinkRefused { .. } => {}
        other => panic!("expected SymlinkRefused from meta write, got: {other:?}"),
    }

    assert!(
        w2_path.is_file(),
        "removed walkthrough file must not be deleted before the meta commit succeeds",
    );

    let loaded = folder.load_session().unwrap();
    assert!(loaded.walkthroughs().contains_key(&w2));
}

#[test]
fn walkthrough_rev_is_capped_on_load() {
    let walkthrough = super::walkthrough_from_json(super::WalkthroughJson {
        walkthrough_id: "w1".to_owned(),
        title: "Cap".to_owned(),
        rev: super::WALKTHROUGH_REV_CAP.saturating_add(123),
        nodes: Vec::new(),
        edges: Vec::new(),
        source: None,
    })
    .unwrap();

    assert_eq!(walkthrough.rev(), super::WALKTHROUGH_REV_CAP);
}

#[rstest]
fn save_session_and_load_session_round_trips_diagrams_and_walkthroughs(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    // Sequence diagram (participant ids must match mermaid names to round-trip via `.mmd`).
    let seq_id = DiagramId::new("d1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:Alice").unwrap();
    let p_bob = ObjectId::new("p:Bob").unwrap();
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").unwrap(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));
    session.diagrams_mut().insert(seq_id.clone(), {
        let mut diagram =
            Diagram::new(seq_id.clone(), "Seq Example", DiagramAst::Sequence(seq_ast));
        diagram.bump_rev();
        diagram.bump_rev();
        diagram
    });

    // Flowchart diagram.
    let flow_id = DiagramId::new("d2").unwrap();
    let mut flow_ast = FlowchartAst::default();
    let node_start_id = ObjectId::new("n:start").unwrap();
    let node_end_id = ObjectId::new("n:end").unwrap();
    let mut start = FlowNode::new("Start");
    start.set_mermaid_id(Some("start"));
    let mut end = FlowNode::new("End");
    end.set_mermaid_id(Some("end"));
    flow_ast.nodes_mut().insert(node_start_id.clone(), start);
    flow_ast.nodes_mut().insert(node_end_id.clone(), end);
    flow_ast
        .edges_mut()
        .insert(ObjectId::new("e:0001").unwrap(), FlowEdge::new(node_start_id, node_end_id));
    session.diagrams_mut().insert(flow_id.clone(), {
        let mut diagram =
            Diagram::new(flow_id.clone(), "Flow Example", DiagramAst::Flowchart(flow_ast));
        diagram.bump_rev();
        diagram
    });

    // Walkthrough.
    let walkthrough_id = WalkthroughId::new("w1").unwrap();
    let mut walkthrough = Walkthrough::new(walkthrough_id.clone(), "Invite acceptance");
    walkthrough.set_source(Some("docs/protocol-01.md#2.5".to_owned()));
    walkthrough.bump_rev();

    let node_start_id = WalkthroughNodeId::new("n:start").unwrap();
    let mut node_start = WalkthroughNode::new(node_start_id.clone(), "Start");
    node_start.set_body_md(Some("Beginning of the flow.".to_owned()));
    node_start.tags_mut().push("entry".to_owned());
    node_start.set_status(Some("draft".to_owned()));

    let obj_ref = ObjectRef::new(
        seq_id.clone(),
        CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()]).unwrap(),
        ObjectId::new("m:0001").unwrap(),
    );
    node_start.refs_mut().push(obj_ref);
    walkthrough.nodes_mut().push(node_start);

    let node_end_id = WalkthroughNodeId::new("n:end").unwrap();
    walkthrough.nodes_mut().push(WalkthroughNode::new(node_end_id.clone(), "End"));

    walkthrough.edges_mut().push(WalkthroughEdge::new(node_start_id, node_end_id, "next"));
    session.walkthroughs_mut().insert(walkthrough_id.clone(), walkthrough);

    session.set_active_diagram_id(Some(seq_id.clone()));
    session.set_active_walkthrough_id(Some(walkthrough_id.clone()));

    let xref_id = XRefId::new("x1").unwrap();
    let from_ref = ObjectRef::new(
        seq_id.clone(),
        CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()]).unwrap(),
        ObjectId::new("m:0001").unwrap(),
    );
    let to_ref = ObjectRef::new(
        flow_id.clone(),
        CategoryPath::new(vec!["flow".to_owned(), "node".to_owned()]).unwrap(),
        ObjectId::new("n:end").unwrap(),
    );
    let mut xref = XRef::new(from_ref, to_ref, "relates_to", ModelXRefStatus::Ok);
    xref.set_label(Some("demo link".to_owned()));
    session.xrefs_mut().insert(xref_id, xref);

    folder.save_session(&session).unwrap();
    let loaded = folder.load_session().unwrap();

    assert_eq!(loaded, session);
}

#[rstest]
fn save_and_load_preserves_stable_object_ids_across_mermaid_visible_renames(
    ctx: SessionFolderTestCtx,
) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let seq_id = DiagramId::new("d-seq").unwrap();
    let participant_alice_id = ObjectId::new("p:alice").unwrap();
    let participant_bob_id = ObjectId::new("p:bob").unwrap();
    let message_id = ObjectId::new("m:0001").unwrap();
    let mut seq_ast = SequenceAst::default();
    seq_ast
        .participants_mut()
        .insert(participant_alice_id.clone(), SequenceParticipant::new("Alicia"));
    seq_ast.participants_mut().insert(participant_bob_id.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        message_id,
        participant_alice_id.clone(),
        participant_bob_id.clone(),
        SequenceMessageKind::Sync,
        "Hi",
        1000,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)));

    let flow_id = DiagramId::new("d-flow").unwrap();
    let node_authorize_id = ObjectId::new("n:authorize").unwrap();
    let node_end_id = ObjectId::new("n:end").unwrap();
    let edge_id = ObjectId::new("e:0001").unwrap();
    let mut flow_ast = FlowchartAst::default();
    let mut authorize = FlowNode::new("Authorize");
    authorize.set_mermaid_id(Some("authz"));
    let mut end = FlowNode::new("End");
    end.set_mermaid_id(Some("end"));
    flow_ast.nodes_mut().insert(node_authorize_id.clone(), authorize);
    flow_ast.nodes_mut().insert(node_end_id.clone(), end);
    flow_ast
        .edges_mut()
        .insert(edge_id, FlowEdge::new(node_authorize_id.clone(), node_end_id.clone()));
    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
    );

    let xref_id = XRefId::new("x:rename").unwrap();
    let from_ref = ObjectRef::new(
        seq_id.clone(),
        CategoryPath::new(vec!["seq".to_owned(), "participant".to_owned()]).unwrap(),
        participant_alice_id,
    );
    let to_ref = ObjectRef::new(
        flow_id.clone(),
        CategoryPath::new(vec!["flow".to_owned(), "node".to_owned()]).unwrap(),
        node_authorize_id,
    );
    session
        .xrefs_mut()
        .insert(xref_id, XRef::new(from_ref, to_ref, "relates_to", ModelXRefStatus::Ok));

    folder.save_session(&session).unwrap();

    let seq_sidecar = folder.load_diagram_meta(&folder.default_diagram_mmd_path(&seq_id)).unwrap();
    assert_eq!(
        seq_sidecar.stable_id_map.by_name.get("Alicia").map(String::as_str),
        Some("p:alice")
    );

    let flow_sidecar =
        folder.load_diagram_meta(&folder.default_diagram_mmd_path(&flow_id)).unwrap();
    assert_eq!(
        flow_sidecar.stable_id_map.by_mermaid_id.get("authz").map(String::as_str),
        Some("n:authorize")
    );

    let loaded = folder.load_session().unwrap();
    assert_eq!(loaded, session);
}

#[rstest]
fn save_and_load_session_round_trips_inline_notes_via_sidecar(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    // Sequence diagram (participant ids must match mermaid names to round-trip via `.mmd`).
    let seq_id = DiagramId::new("d1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:Alice").unwrap();
    let p_bob = ObjectId::new("p:Bob").unwrap();
    let mut alice = SequenceParticipant::new("Alice");
    alice.set_note(Some("caller must be authenticated"));
    seq_ast.participants_mut().insert(p_alice.clone(), alice);
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").unwrap(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id, "Seq Notes", DiagramAst::Sequence(seq_ast)));

    // Flowchart diagram.
    let flow_id = DiagramId::new("d2").unwrap();
    let mut flow_ast = FlowchartAst::default();
    let node_start_id = ObjectId::new("n:start").unwrap();
    let node_end_id = ObjectId::new("n:end").unwrap();
    let mut node_start = FlowNode::new("Start");
    node_start.set_mermaid_id(Some("start"));
    flow_ast.nodes_mut().insert(node_start_id.clone(), node_start);
    let mut node_end = FlowNode::new("End");
    node_end.set_mermaid_id(Some("end"));
    node_end.set_note(Some("must be idempotent"));
    flow_ast.nodes_mut().insert(node_end_id.clone(), node_end);
    flow_ast
        .edges_mut()
        .insert(ObjectId::new("e:0001").unwrap(), FlowEdge::new(node_start_id, node_end_id));
    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id, "Flow Notes", DiagramAst::Flowchart(flow_ast)),
    );

    folder.save_session(&session).unwrap();
    let loaded = folder.load_session().unwrap();

    assert_eq!(loaded, session);
}

#[rstest]
fn save_and_load_flowchart_preserves_edge_ids_and_style_via_sidecar(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let flow_id = DiagramId::new("d1").unwrap();
    let mut flow_ast = FlowchartAst::default();
    let node_a = ObjectId::new("n:a").unwrap();
    let node_b = ObjectId::new("n:b").unwrap();
    flow_ast.nodes_mut().insert(node_a.clone(), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(node_b.clone(), FlowNode::new("B"));
    flow_ast.edges_mut().insert(
        ObjectId::new("e:custom").unwrap(),
        FlowEdge::new_with(node_a, node_b, Some("yes".to_owned()), Some("dashed".to_owned())),
    );
    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
    );

    folder.save_session(&session).unwrap();

    let loaded = folder.load_session().unwrap();
    let loaded_diagram = loaded.diagrams().get(&flow_id).expect("flow diagram");
    let DiagramAst::Flowchart(loaded_ast) = loaded_diagram.ast() else {
        panic!("expected flowchart ast");
    };

    let edge_id = ObjectId::new("e:custom").unwrap();
    let edge = loaded_ast.edges().get(&edge_id).expect("edge");
    assert_eq!(edge.style(), Some("dashed"));
}

#[rstest]
fn load_session_does_not_reuse_edge_ids_from_sidecar_for_new_edges(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let flow_id = DiagramId::new("d1").unwrap();
    let mut flow_ast = FlowchartAst::default();
    let node_a = ObjectId::new("n:a").unwrap();
    let node_b = ObjectId::new("n:b").unwrap();
    let node_c = ObjectId::new("n:c").unwrap();
    flow_ast.nodes_mut().insert(node_a.clone(), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(node_b.clone(), FlowNode::new("B"));
    flow_ast.nodes_mut().insert(node_c.clone(), FlowNode::new("C"));
    flow_ast
        .edges_mut()
        .insert(ObjectId::new("e:0001").unwrap(), FlowEdge::new(node_a.clone(), node_b));
    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
    );

    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&flow_id);
    std::fs::write(&mmd_path, "flowchart\n  a --> c\n").unwrap();

    let loaded = folder.load_session().unwrap();
    let loaded_diagram = loaded.diagrams().get(&flow_id).expect("flow diagram");
    let DiagramAst::Flowchart(loaded_ast) = loaded_diagram.ast() else {
        panic!("expected flowchart ast");
    };

    assert_eq!(loaded_ast.edges().len(), 1);
    let (edge_id, edge) = loaded_ast.edges().iter().next().expect("edge");
    assert_ne!(edge_id.as_str(), "e:0001");
    assert_eq!(edge.from_node_id().as_str(), "n:a");
    assert_eq!(edge.to_node_id().as_str(), "n:c");
}

#[rstest]
fn save_and_load_sequence_preserves_message_ids_via_sidecar_even_when_parse_order_changes(
    ctx: SessionFolderTestCtx,
) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let seq_id = DiagramId::new("d1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:Alice").unwrap();
    let p_bob = ObjectId::new("p:Bob").unwrap();
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:alpha").unwrap(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "First",
        1000,
    ));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:beta").unwrap(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "Second",
        2000,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)));

    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&seq_id);
    std::fs::write(
        &mmd_path,
        "sequenceDiagram\n  Alice ->> Bob: New\n  Alice ->> Bob: First\n  Alice ->> Bob: Second\n",
    )
    .unwrap();

    let loaded = folder.load_session().unwrap();
    let loaded_diagram = loaded.diagrams().get(&seq_id).expect("seq diagram");
    let DiagramAst::Sequence(loaded_ast) = loaded_diagram.ast() else {
        panic!("expected sequence ast");
    };

    let first = loaded_ast
        .messages()
        .iter()
        .find(|msg| msg.message_id().as_str() == "m:alpha")
        .expect("first message");
    assert_eq!(first.text(), "First");

    let second = loaded_ast
        .messages()
        .iter()
        .find(|msg| msg.message_id().as_str() == "m:beta")
        .expect("second message");
    assert_eq!(second.text(), "Second");
}

#[rstest]
fn save_and_load_sequence_preserves_custom_block_and_section_ids(ctx: SessionFolderTestCtx) {
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceSection, SequenceSectionKind,
    };

    let folder = &ctx.folder;
    let mut session = Session::new(SessionId::new("s1").unwrap());
    let seq_id = DiagramId::new("d1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:Alice").unwrap();
    let p_bob = ObjectId::new("p:Bob").unwrap();
    let m_hit = ObjectId::new("m:hit").unwrap();
    let m_miss = ObjectId::new("m:miss").unwrap();
    let block_id = ObjectId::new("b:cache").unwrap();
    let main_sec = ObjectId::new("sec:cache:main").unwrap();
    let else_sec = ObjectId::new("sec:cache:else").unwrap();

    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        m_hit.clone(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "hit",
        1000,
    ));
    seq_ast.messages_mut().push(SequenceMessage::new(
        m_miss.clone(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "miss",
        2000,
    ));
    seq_ast.blocks_mut().push(SequenceBlock::new(
        block_id.clone(),
        SequenceBlockKind::Alt,
        Some("cache".to_owned()),
        vec![
            SequenceSection::new(main_sec.clone(), SequenceSectionKind::Main, None, vec![m_hit]),
            SequenceSection::new(
                else_sec.clone(),
                SequenceSectionKind::Else,
                Some("miss".to_owned()),
                vec![m_miss],
            ),
        ],
        Vec::new(),
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)));

    folder.save_session(&session).unwrap();

    // Reparse from mermaid (positional block ids) and confirm sidecar remaps back.
    let loaded = folder.load_session().unwrap();
    let loaded_diagram = loaded.diagrams().get(&seq_id).expect("seq diagram");
    let DiagramAst::Sequence(loaded_ast) = loaded_diagram.ast() else {
        panic!("expected sequence ast");
    };

    assert!(loaded_ast.contains_block_id(&block_id));
    assert!(loaded_ast.contains_section_id(&main_sec));
    assert!(loaded_ast.contains_section_id(&else_sec));
    assert_eq!(
        loaded_ast.find_section(&main_sec).expect("main").message_ids()[0].as_str(),
        "m:hit"
    );
    assert_eq!(
        loaded_ast.find_section(&else_sec).expect("else").message_ids()[0].as_str(),
        "m:miss"
    );
}

#[rstest]
fn load_session_does_not_reuse_message_ids_from_sidecar_for_new_messages(
    ctx: SessionFolderTestCtx,
) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let seq_id = DiagramId::new("d1").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:Alice").unwrap();
    let p_bob = ObjectId::new("p:Bob").unwrap();
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        ObjectId::new("m:0001").unwrap(),
        p_alice.clone(),
        p_bob.clone(),
        SequenceMessageKind::Sync,
        "First",
        1000,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)));

    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&seq_id);
    std::fs::write(&mmd_path, "sequenceDiagram\n  Alice ->> Bob: Second\n").unwrap();

    let loaded = folder.load_session().unwrap();
    let loaded_diagram = loaded.diagrams().get(&seq_id).expect("seq diagram");
    let DiagramAst::Sequence(loaded_ast) = loaded_diagram.ast() else {
        panic!("expected sequence ast");
    };

    assert_eq!(loaded_ast.messages().len(), 1);
    let msg = &loaded_ast.messages()[0];
    assert_ne!(msg.message_id().as_str(), "m:0001");
    assert_eq!(msg.text(), "Second");
}

#[rstest]
fn xrefs_targeting_flow_edges_and_seq_messages_round_trip_without_becoming_dangling(
    ctx: SessionFolderTestCtx,
) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());

    let flow_id = DiagramId::new("flow").unwrap();
    let mut flow_ast = FlowchartAst::default();
    let node_a = ObjectId::new("n:a").unwrap();
    let node_b = ObjectId::new("n:b").unwrap();
    let edge_id = ObjectId::new("e:custom").unwrap();
    flow_ast.nodes_mut().insert(node_a.clone(), FlowNode::new("A"));
    flow_ast.nodes_mut().insert(node_b.clone(), FlowNode::new("B"));
    flow_ast.edges_mut().insert(edge_id.clone(), FlowEdge::new(node_a, node_b));
    session.diagrams_mut().insert(
        flow_id.clone(),
        Diagram::new(flow_id.clone(), "Flow", DiagramAst::Flowchart(flow_ast)),
    );

    let seq_id = DiagramId::new("seq").unwrap();
    let mut seq_ast = SequenceAst::default();
    let p_alice = ObjectId::new("p:Alice").unwrap();
    let p_bob = ObjectId::new("p:Bob").unwrap();
    let message_id = ObjectId::new("m:alpha").unwrap();
    seq_ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
    seq_ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
    seq_ast.messages_mut().push(SequenceMessage::new(
        message_id.clone(),
        p_alice,
        p_bob,
        SequenceMessageKind::Sync,
        "Hello",
        1000,
    ));
    session
        .diagrams_mut()
        .insert(seq_id.clone(), Diagram::new(seq_id.clone(), "Seq", DiagramAst::Sequence(seq_ast)));

    let from_ref = ObjectRef::new(
        flow_id.clone(),
        CategoryPath::new(vec!["flow".to_owned(), "edge".to_owned()]).unwrap(),
        edge_id.clone(),
    );
    let to_ref = ObjectRef::new(
        seq_id.clone(),
        CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()]).unwrap(),
        message_id.clone(),
    );
    let xref_id = XRefId::new("x:1").unwrap();
    session.xrefs_mut().insert(
        xref_id.clone(),
        XRef::new(from_ref.clone(), to_ref.clone(), "relates_to", ModelXRefStatus::Ok),
    );

    folder.save_session(&session).unwrap();
    let loaded = folder.load_session().unwrap();

    let loaded_xref = loaded.xrefs().get(&xref_id).expect("xref");
    assert_eq!(loaded_xref.status(), ModelXRefStatus::Ok);
    assert_eq!(loaded_xref.from(), &from_ref);
    assert_eq!(loaded_xref.to(), &to_ref);
}

#[rstest]
fn save_session_writes_diagram_sidecars(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let diagram_id = DiagramId::new("d1").unwrap();
    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Flow", DiagramAst::Flowchart(FlowchartAst::default())),
    );

    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&diagram_id);
    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();
    assert!(sidecar_path.is_file());

    let meta_str = std::fs::read_to_string(&sidecar_path).unwrap();
    let meta_json: serde_json::Value = serde_json::from_str(&meta_str).unwrap();
    assert_eq!(meta_json["diagram_id"].as_str().unwrap(), "d1");
    assert_eq!(meta_json["mmd_path"].as_str().unwrap(), "diagrams/d1.mmd");
}

#[rstest]
fn load_session_is_compatible_when_diagram_sidecars_are_missing(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let diagram_id = DiagramId::new("d1").unwrap();
    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Flow", DiagramAst::Flowchart(FlowchartAst::default())),
    );
    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&diagram_id);
    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();
    std::fs::remove_file(&sidecar_path).unwrap();

    let loaded = folder.load_session().unwrap();
    assert!(loaded.diagrams().contains_key(&diagram_id));
}

#[rstest]
fn load_session_errors_when_diagram_sidecar_is_invalid_json(ctx: SessionFolderTestCtx) {
    let folder = &ctx.folder;

    let mut session = Session::new(SessionId::new("s1").unwrap());
    let diagram_id = DiagramId::new("d1").unwrap();
    session.diagrams_mut().insert(
        diagram_id.clone(),
        Diagram::new(diagram_id.clone(), "Flow", DiagramAst::Flowchart(FlowchartAst::default())),
    );
    folder.save_session(&session).unwrap();

    let mmd_path = folder.default_diagram_mmd_path(&diagram_id);
    let sidecar_path = folder.diagram_meta_path(&mmd_path).unwrap();
    std::fs::write(&sidecar_path, b"{ not json").unwrap();

    let err = folder.load_session().unwrap_err();
    match err {
        StoreError::Json { path, .. } => assert_eq!(path, sidecar_path),
        other => panic!("expected Json error, got: {other:?}"),
    }
}
