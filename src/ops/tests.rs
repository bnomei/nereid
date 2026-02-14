// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use crate::model::{
    DiagramAst, DiagramId, FlowchartAst, ObjectId, SequenceAst, SequenceParticipant,
};

use super::{
    apply_ops, FlowEdgePatch, FlowNodePatch, FlowOp, Op, SeqMessagePatch, SeqOp,
    SeqParticipantPatch,
};

#[test]
fn apply_seq_op_bumps_rev_and_mutates_ast() {
    let diagram_id = DiagramId::new("d:1").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice_id = ObjectId::new("p:alice").expect("participant id");
    let ops = [Op::Seq(SeqOp::AddParticipant {
        participant_id: alice_id.clone(),
        mermaid_name: "Alice".to_owned(),
    })];

    let result = apply_ops(&mut diagram, 0, &ops).expect("apply");
    assert_eq!(result.new_rev, 1);
    assert_eq!(diagram.rev(), 1);
    assert_eq!(result.delta.added.len(), 1);
    assert!(result.delta.removed.is_empty());
    assert!(result.delta.updated.is_empty());

    let expected = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["seq".to_owned(), "participant".to_owned()])
            .expect("category"),
        alice_id.clone(),
    );
    assert_eq!(result.delta.added, vec![expected]);

    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    assert!(ast.participants().contains_key(&alice_id));
}

#[test]
fn apply_seq_update_participant_preserves_role() {
    let diagram_id = DiagramId::new("d:update-participant").expect("diagram id");
    let participant_id = ObjectId::new("p:alice").expect("participant id");

    let mut ast = SequenceAst::default();
    let mut participant = SequenceParticipant::new("Alice");
    participant.set_role(Some("actor"));
    ast.participants_mut()
        .insert(participant_id.clone(), participant);

    let mut diagram = crate::model::Diagram::new(diagram_id, "seq", DiagramAst::Sequence(ast));

    let ops = [Op::Seq(SeqOp::UpdateParticipant {
        participant_id: participant_id.clone(),
        patch: SeqParticipantPatch {
            mermaid_name: Some("Alice2".to_owned()),
        },
    })];

    apply_ops(&mut diagram, 0, &ops).expect("apply");

    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    let participant = ast
        .participants()
        .get(&participant_id)
        .expect("participant");
    assert_eq!(participant.mermaid_name(), "Alice2");
    assert_eq!(participant.role(), Some("actor"));
}

#[test]
fn apply_seq_set_participant_note_sets_note_and_records_delta_updated() {
    let diagram_id = DiagramId::new("d:seq-note").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let participant_id = ObjectId::new("p:alice").expect("participant id");
    apply_ops(
        &mut diagram,
        0,
        &[Op::Seq(SeqOp::AddParticipant {
            participant_id: participant_id.clone(),
            mermaid_name: "Alice".to_owned(),
        })],
    )
    .expect("setup apply");

    let result = apply_ops(
        &mut diagram,
        1,
        &[Op::Seq(SeqOp::SetParticipantNote {
            participant_id: participant_id.clone(),
            note: Some("invariant".to_owned()),
        })],
    )
    .expect("apply");

    let expected = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["seq".to_owned(), "participant".to_owned()])
            .expect("category"),
        participant_id.clone(),
    );
    assert_eq!(result.delta.updated, vec![expected]);

    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    let participant = ast
        .participants()
        .get(&participant_id)
        .expect("participant exists");
    assert_eq!(participant.note(), Some("invariant"));
}

#[test]
fn apply_conflicts_on_stale_base_rev() {
    let diagram_id = DiagramId::new("d:1").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice_id = ObjectId::new("p:alice").expect("participant id");
    let ops = [Op::Seq(SeqOp::AddParticipant {
        participant_id: alice_id.clone(),
        mermaid_name: "Alice".to_owned(),
    })];

    apply_ops(&mut diagram, 0, &ops).expect("first apply");
    let err = apply_ops(&mut diagram, 0, &ops).unwrap_err();
    assert!(matches!(err, super::ApplyError::Conflict { .. }));
}

#[test]
fn apply_seq_add_message_rejects_missing_from_participant() {
    let diagram_id = DiagramId::new("d:missing-from").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let missing_from = ObjectId::new("p:missing-from").expect("participant id");
    let bob = ObjectId::new("p:bob").expect("participant id");
    let message_id = ObjectId::new("m:1").expect("message id");

    let ops = [
        Op::Seq(SeqOp::AddParticipant {
            participant_id: bob.clone(),
            mermaid_name: "Bob".to_owned(),
        }),
        Op::Seq(SeqOp::AddMessage {
            message_id,
            from_participant_id: missing_from.clone(),
            to_participant_id: bob,
            kind: crate::model::SequenceMessageKind::Sync,
            arrow: None,
            text: "hi".to_owned(),
            order_key: 0,
        }),
    ];

    let err = apply_ops(&mut diagram, 0, &ops).unwrap_err();
    assert_eq!(
        err,
        super::ApplyError::NotFound {
            kind: super::ObjectKind::SeqParticipant,
            object_id: missing_from,
        }
    );
}

#[test]
fn apply_seq_add_message_rejects_missing_to_participant() {
    let diagram_id = DiagramId::new("d:missing-to").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice = ObjectId::new("p:alice").expect("participant id");
    let missing_to = ObjectId::new("p:missing-to").expect("participant id");
    let message_id = ObjectId::new("m:1").expect("message id");

    let ops = [
        Op::Seq(SeqOp::AddParticipant {
            participant_id: alice.clone(),
            mermaid_name: "Alice".to_owned(),
        }),
        Op::Seq(SeqOp::AddMessage {
            message_id,
            from_participant_id: alice,
            to_participant_id: missing_to.clone(),
            kind: crate::model::SequenceMessageKind::Sync,
            arrow: None,
            text: "hi".to_owned(),
            order_key: 0,
        }),
    ];

    let err = apply_ops(&mut diagram, 0, &ops).unwrap_err();
    assert_eq!(
        err,
        super::ApplyError::NotFound {
            kind: super::ObjectKind::SeqParticipant,
            object_id: missing_to,
        }
    );
}

#[test]
fn apply_seq_update_message_rejects_missing_from_participant() {
    let diagram_id = DiagramId::new("d:update-missing-from").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice = ObjectId::new("p:alice").expect("participant id");
    let bob = ObjectId::new("p:bob").expect("participant id");
    let message_id = ObjectId::new("m:1").expect("message id");
    let missing_from = ObjectId::new("p:missing-from").expect("participant id");

    let ops = [
        Op::Seq(SeqOp::AddParticipant {
            participant_id: alice.clone(),
            mermaid_name: "Alice".to_owned(),
        }),
        Op::Seq(SeqOp::AddParticipant {
            participant_id: bob.clone(),
            mermaid_name: "Bob".to_owned(),
        }),
        Op::Seq(SeqOp::AddMessage {
            message_id: message_id.clone(),
            from_participant_id: alice,
            to_participant_id: bob,
            kind: crate::model::SequenceMessageKind::Sync,
            arrow: None,
            text: "hi".to_owned(),
            order_key: 0,
        }),
        Op::Seq(SeqOp::UpdateMessage {
            message_id,
            patch: SeqMessagePatch {
                from_participant_id: Some(missing_from.clone()),
                ..Default::default()
            },
        }),
    ];

    let err = apply_ops(&mut diagram, 0, &ops).unwrap_err();
    assert_eq!(
        err,
        super::ApplyError::NotFound {
            kind: super::ObjectKind::SeqParticipant,
            object_id: missing_from,
        }
    );
}

#[test]
fn apply_seq_update_message_rejects_missing_to_participant() {
    let diagram_id = DiagramId::new("d:update-missing-to").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice = ObjectId::new("p:alice").expect("participant id");
    let bob = ObjectId::new("p:bob").expect("participant id");
    let message_id = ObjectId::new("m:1").expect("message id");
    let missing_to = ObjectId::new("p:missing-to").expect("participant id");

    let ops = [
        Op::Seq(SeqOp::AddParticipant {
            participant_id: alice.clone(),
            mermaid_name: "Alice".to_owned(),
        }),
        Op::Seq(SeqOp::AddParticipant {
            participant_id: bob.clone(),
            mermaid_name: "Bob".to_owned(),
        }),
        Op::Seq(SeqOp::AddMessage {
            message_id: message_id.clone(),
            from_participant_id: alice,
            to_participant_id: bob,
            kind: crate::model::SequenceMessageKind::Sync,
            arrow: None,
            text: "hi".to_owned(),
            order_key: 0,
        }),
        Op::Seq(SeqOp::UpdateMessage {
            message_id,
            patch: SeqMessagePatch {
                to_participant_id: Some(missing_to.clone()),
                ..Default::default()
            },
        }),
    ];

    let err = apply_ops(&mut diagram, 0, &ops).unwrap_err();
    assert_eq!(
        err,
        super::ApplyError::NotFound {
            kind: super::ObjectKind::SeqParticipant,
            object_id: missing_to,
        }
    );
}

#[test]
fn apply_flow_op_adds_node_edge_and_bumps_rev() {
    let diagram_id = DiagramId::new("d:2").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(crate::model::FlowchartAst::default()),
    );

    let n1 = ObjectId::new("n:1").expect("node id");
    let n2 = ObjectId::new("n:2").expect("node id");
    let e1 = ObjectId::new("e:1").expect("edge id");

    let ops = [
        Op::Flow(FlowOp::AddNode {
            node_id: n1.clone(),
            label: "Start".to_owned(),
            shape: None,
        }),
        Op::Flow(FlowOp::AddNode {
            node_id: n2.clone(),
            label: "End".to_owned(),
            shape: None,
        }),
        Op::Flow(FlowOp::AddEdge {
            edge_id: e1.clone(),
            from_node_id: n1.clone(),
            to_node_id: n2.clone(),
            label: None,
            connector: None,
            style: None,
        }),
    ];

    let result = apply_ops(&mut diagram, 0, &ops).expect("apply");
    assert_eq!(result.new_rev, 1);
    assert_eq!(diagram.rev(), 1);

    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("expected flowchart ast");
    };
    assert!(ast.nodes().contains_key(&n1));
    assert!(ast.nodes().contains_key(&n2));
    assert!(ast.edges().contains_key(&e1));
}

#[test]
fn apply_flow_remove_node_records_cascading_edge_removal_in_delta() {
    let diagram_id = DiagramId::new("d:3").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(crate::model::FlowchartAst::default()),
    );

    let n1 = ObjectId::new("n:1").expect("node id");
    let n2 = ObjectId::new("n:2").expect("node id");
    let e1 = ObjectId::new("e:1").expect("edge id");

    let setup_ops = [
        Op::Flow(FlowOp::AddNode {
            node_id: n1.clone(),
            label: "Start".to_owned(),
            shape: None,
        }),
        Op::Flow(FlowOp::AddNode {
            node_id: n2.clone(),
            label: "End".to_owned(),
            shape: None,
        }),
        Op::Flow(FlowOp::AddEdge {
            edge_id: e1.clone(),
            from_node_id: n1.clone(),
            to_node_id: n2.clone(),
            label: None,
            connector: None,
            style: None,
        }),
    ];
    apply_ops(&mut diagram, 0, &setup_ops).expect("setup apply");

    let result = apply_ops(
        &mut diagram,
        1,
        &[Op::Flow(FlowOp::RemoveNode {
            node_id: n1.clone(),
        })],
    )
    .expect("apply");

    assert_eq!(result.new_rev, 2);
    assert_eq!(result.delta.removed.len(), 2);
    assert!(result.delta.added.is_empty());
    assert!(result.delta.updated.is_empty());

    let expected_node = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["flow".to_owned(), "node".to_owned()])
            .expect("category"),
        n1.clone(),
    );
    let expected_edge = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["flow".to_owned(), "edge".to_owned()])
            .expect("category"),
        e1.clone(),
    );
    assert!(result.delta.removed.contains(&expected_node));
    assert!(result.delta.removed.contains(&expected_edge));

    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("expected flowchart ast");
    };
    assert!(!ast.nodes().contains_key(&n1));
    assert!(ast.nodes().contains_key(&n2));
    assert!(!ast.edges().contains_key(&e1));
}

#[test]
fn apply_flow_update_node_patch_preserves_unrelated_fields() {
    let diagram_id = DiagramId::new("d:node-patch").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(crate::model::FlowchartAst::default()),
    );

    let node_id = ObjectId::new("n:1").expect("node id");
    let ops = [
        Op::Flow(FlowOp::AddNode {
            node_id: node_id.clone(),
            label: "Start".to_owned(),
            shape: Some("stadium".to_owned()),
        }),
        Op::Flow(FlowOp::UpdateNode {
            node_id: node_id.clone(),
            patch: FlowNodePatch {
                label: Some("Begin".to_owned()),
                ..Default::default()
            },
        }),
        Op::Flow(FlowOp::UpdateNode {
            node_id: node_id.clone(),
            patch: FlowNodePatch {
                shape: Some("circle".to_owned()),
                ..Default::default()
            },
        }),
    ];

    apply_ops(&mut diagram, 0, &ops).expect("apply");

    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("expected flowchart ast");
    };
    let node = ast.nodes().get(&node_id).expect("node exists");
    assert_eq!(node.label(), "Begin");
    assert_eq!(node.shape(), "circle");
}

#[test]
fn apply_flow_set_node_mermaid_id_updates_node_without_changing_stable_id() {
    let diagram_id = DiagramId::new("d:set-node-mermaid-id").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(FlowchartAst::default()),
    );

    let node_id = ObjectId::new("n:authorize").expect("node id");
    apply_ops(
        &mut diagram,
        0,
        &[Op::Flow(FlowOp::AddNode {
            node_id: node_id.clone(),
            label: "Authorize".to_owned(),
            shape: None,
        })],
    )
    .expect("setup");

    let result = apply_ops(
        &mut diagram,
        1,
        &[Op::Flow(FlowOp::SetNodeMermaidId {
            node_id: node_id.clone(),
            mermaid_id: Some("authz".to_owned()),
        })],
    )
    .expect("apply");

    let expected = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["flow".to_owned(), "node".to_owned()])
            .expect("category"),
        node_id.clone(),
    );
    assert_eq!(result.delta.updated, vec![expected]);

    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("expected flowchart ast");
    };
    let node = ast.nodes().get(&node_id).expect("node exists");
    assert_eq!(node.mermaid_id(), Some("authz"));
    assert_eq!(node.label(), "Authorize");
}

#[test]
fn apply_flow_set_node_mermaid_id_rejects_invalid_identifiers() {
    let diagram_id = DiagramId::new("d:set-node-mermaid-id-invalid").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(FlowchartAst::default()),
    );

    let node_id = ObjectId::new("n:a").expect("node id");
    apply_ops(
        &mut diagram,
        0,
        &[Op::Flow(FlowOp::AddNode {
            node_id: node_id.clone(),
            label: "A".to_owned(),
            shape: None,
        })],
    )
    .expect("setup");

    let err = apply_ops(
        &mut diagram,
        1,
        &[Op::Flow(FlowOp::SetNodeMermaidId {
            node_id,
            mermaid_id: Some("bad-id".to_owned()),
        })],
    )
    .unwrap_err();

    assert!(matches!(
        err,
        super::ApplyError::InvalidFlowNodeMermaidId { .. }
    ));
}

#[test]
fn apply_flow_set_node_mermaid_id_rejects_duplicates() {
    let diagram_id = DiagramId::new("d:set-node-mermaid-id-duplicate").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(FlowchartAst::default()),
    );

    let node_a = ObjectId::new("n:a").expect("node id");
    let node_b = ObjectId::new("n:b").expect("node id");
    apply_ops(
        &mut diagram,
        0,
        &[
            Op::Flow(FlowOp::AddNode {
                node_id: node_a.clone(),
                label: "A".to_owned(),
                shape: None,
            }),
            Op::Flow(FlowOp::AddNode {
                node_id: node_b.clone(),
                label: "B".to_owned(),
                shape: None,
            }),
        ],
    )
    .expect("setup");

    let err = apply_ops(
        &mut diagram,
        1,
        &[Op::Flow(FlowOp::SetNodeMermaidId {
            node_id: node_a,
            mermaid_id: Some("b".to_owned()),
        })],
    )
    .unwrap_err();

    assert!(matches!(
        err,
        super::ApplyError::DuplicateFlowNodeMermaidId { .. }
    ));
}

#[test]
fn apply_flow_set_node_note_clears_note_and_records_delta_updated() {
    let diagram_id = DiagramId::new("d:flow-note").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(FlowchartAst::default()),
    );

    let node_id = ObjectId::new("n:1").expect("node id");
    apply_ops(
        &mut diagram,
        0,
        &[Op::Flow(FlowOp::AddNode {
            node_id: node_id.clone(),
            label: "Start".to_owned(),
            shape: None,
        })],
    )
    .expect("setup apply");

    apply_ops(
        &mut diagram,
        1,
        &[Op::Flow(FlowOp::SetNodeNote {
            node_id: node_id.clone(),
            note: Some("invariant".to_owned()),
        })],
    )
    .expect("apply set");

    let result = apply_ops(
        &mut diagram,
        2,
        &[Op::Flow(FlowOp::SetNodeNote {
            node_id: node_id.clone(),
            note: None,
        })],
    )
    .expect("apply clear");

    let expected = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["flow".to_owned(), "node".to_owned()])
            .expect("category"),
        node_id.clone(),
    );
    assert_eq!(result.delta.updated, vec![expected]);

    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("expected flowchart ast");
    };
    let node = ast.nodes().get(&node_id).expect("node exists");
    assert_eq!(node.note(), None);
}

#[test]
fn apply_flow_edge_label_style_updates_and_are_preserved_on_endpoint_changes() {
    let diagram_id = DiagramId::new("d:edge-patch").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(crate::model::FlowchartAst::default()),
    );

    let n1 = ObjectId::new("n:1").expect("node id");
    let n2 = ObjectId::new("n:2").expect("node id");
    let n3 = ObjectId::new("n:3").expect("node id");
    let e1 = ObjectId::new("e:1").expect("edge id");

    let ops = [
        Op::Flow(FlowOp::AddNode {
            node_id: n1.clone(),
            label: "One".to_owned(),
            shape: None,
        }),
        Op::Flow(FlowOp::AddNode {
            node_id: n2.clone(),
            label: "Two".to_owned(),
            shape: None,
        }),
        Op::Flow(FlowOp::AddNode {
            node_id: n3.clone(),
            label: "Three".to_owned(),
            shape: None,
        }),
        Op::Flow(FlowOp::AddEdge {
            edge_id: e1.clone(),
            from_node_id: n1.clone(),
            to_node_id: n2.clone(),
            label: Some("yes".to_owned()),
            connector: None,
            style: Some("dashed".to_owned()),
        }),
        Op::Flow(FlowOp::UpdateEdge {
            edge_id: e1.clone(),
            patch: FlowEdgePatch {
                from_node_id: Some(n2.clone()),
                to_node_id: Some(n3.clone()),
                ..Default::default()
            },
        }),
        Op::Flow(FlowOp::UpdateEdge {
            edge_id: e1.clone(),
            patch: FlowEdgePatch {
                label: Some("maybe".to_owned()),
                style: Some("thick".to_owned()),
                ..Default::default()
            },
        }),
    ];

    apply_ops(&mut diagram, 0, &ops).expect("apply");

    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("expected flowchart ast");
    };
    let edge = ast.edges().get(&e1).expect("edge exists");

    assert_eq!(edge.from_node_id(), &n2);
    assert_eq!(edge.to_node_id(), &n3);
    assert_eq!(edge.label(), Some("maybe"));
    assert_eq!(edge.style(), Some("thick"));
}

#[test]
fn apply_seq_remove_participant_records_cascading_message_removal_in_delta() {
    let diagram_id = DiagramId::new("d:4").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice = ObjectId::new("p:alice").expect("participant id");
    let bob = ObjectId::new("p:bob").expect("participant id");
    let m1 = ObjectId::new("m:1").expect("message id");

    let setup_ops = [
        Op::Seq(SeqOp::AddParticipant {
            participant_id: alice.clone(),
            mermaid_name: "Alice".to_owned(),
        }),
        Op::Seq(SeqOp::AddParticipant {
            participant_id: bob.clone(),
            mermaid_name: "Bob".to_owned(),
        }),
        Op::Seq(SeqOp::AddMessage {
            message_id: m1.clone(),
            from_participant_id: alice.clone(),
            to_participant_id: bob.clone(),
            kind: crate::model::SequenceMessageKind::Sync,
            arrow: None,
            text: "hi".to_owned(),
            order_key: 0,
        }),
    ];
    apply_ops(&mut diagram, 0, &setup_ops).expect("setup apply");

    let result = apply_ops(
        &mut diagram,
        1,
        &[Op::Seq(SeqOp::RemoveParticipant {
            participant_id: alice.clone(),
        })],
    )
    .expect("apply");

    assert_eq!(result.new_rev, 2);
    assert_eq!(result.delta.removed.len(), 2);
    assert!(result.delta.added.is_empty());
    assert!(result.delta.updated.is_empty());

    let expected_participant = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["seq".to_owned(), "participant".to_owned()])
            .expect("category"),
        alice.clone(),
    );
    let expected_message = crate::model::ObjectRef::new(
        diagram.diagram_id().clone(),
        crate::model::CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()])
            .expect("category"),
        m1.clone(),
    );
    assert!(result.delta.removed.contains(&expected_participant));
    assert!(result.delta.removed.contains(&expected_message));

    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    assert!(!ast.participants().contains_key(&alice));
    assert!(ast.participants().contains_key(&bob));
    assert!(ast.messages().is_empty());
}
