// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Op application regression tests: revision conflicts, kind mismatches, and deltas.

use crate::model::{
    DiagramAst, DiagramId, FlowchartAst, ObjectId, SequenceAst, SequenceParticipant, SymbolAnchor,
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
    ast.participants_mut().insert(participant_id.clone(), participant);

    let mut diagram = crate::model::Diagram::new(diagram_id, "seq", DiagramAst::Sequence(ast));

    let ops = [Op::Seq(SeqOp::UpdateParticipant {
        participant_id: participant_id.clone(),
        patch: SeqParticipantPatch { mermaid_name: Some("Alice2".to_owned()) },
    })];

    apply_ops(&mut diagram, 0, &ops).expect("apply");

    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    let participant = ast.participants().get(&participant_id).expect("participant");
    assert_eq!(participant.mermaid_name(), "Alice2");
    assert_eq!(participant.role(), Some("actor"));
}

#[test]
fn apply_seq_add_participant_rejects_invalid_mermaid_name() {
    let diagram_id = DiagramId::new("d:add-participant-invalid-name").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let participant_id = ObjectId::new("p:bad").expect("participant id");
    let err = apply_ops(
        &mut diagram,
        0,
        &[Op::Seq(SeqOp::AddParticipant { participant_id, mermaid_name: "Bad Name".to_owned() })],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::InvalidSeqParticipantMermaidName { .. }));
    assert_eq!(diagram.rev(), 0);
    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    assert!(ast.participants().is_empty());
}

#[test]
fn apply_seq_add_participant_rejects_duplicate_mermaid_name() {
    let diagram_id = DiagramId::new("d:add-participant-duplicate-name").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice_id = ObjectId::new("p:alice").expect("participant id");
    let alice_duplicate_id = ObjectId::new("p:alice-duplicate").expect("participant id");
    let err = apply_ops(
        &mut diagram,
        0,
        &[
            Op::Seq(SeqOp::AddParticipant {
                participant_id: alice_id,
                mermaid_name: "Alice".to_owned(),
            }),
            Op::Seq(SeqOp::AddParticipant {
                participant_id: alice_duplicate_id,
                mermaid_name: "Alice".to_owned(),
            }),
        ],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::DuplicateSeqParticipantMermaidName { .. }));
    assert_eq!(diagram.rev(), 0);
    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    assert!(ast.participants().is_empty());
}

#[test]
fn apply_seq_update_participant_rejects_duplicate_mermaid_name() {
    let diagram_id = DiagramId::new("d:update-participant-duplicate-name").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice_id = ObjectId::new("p:alice").expect("participant id");
    let bob_id = ObjectId::new("p:bob").expect("participant id");
    apply_ops(
        &mut diagram,
        0,
        &[
            Op::Seq(SeqOp::AddParticipant {
                participant_id: alice_id,
                mermaid_name: "Alice".to_owned(),
            }),
            Op::Seq(SeqOp::AddParticipant {
                participant_id: bob_id.clone(),
                mermaid_name: "Bob".to_owned(),
            }),
        ],
    )
    .expect("setup");

    let err = apply_ops(
        &mut diagram,
        1,
        &[Op::Seq(SeqOp::UpdateParticipant {
            participant_id: bob_id.clone(),
            patch: SeqParticipantPatch { mermaid_name: Some("Alice".to_owned()) },
        })],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::DuplicateSeqParticipantMermaidName { .. }));
    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    let bob = ast.participants().get(&bob_id).expect("bob participant");
    assert_eq!(bob.mermaid_name(), "Bob");
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
    let participant = ast.participants().get(&participant_id).expect("participant exists");
    assert_eq!(participant.note(), Some("invariant"));
}

#[test]
fn apply_seq_set_participant_symbol_sets_symbol_and_records_delta_updated() {
    let diagram_id = DiagramId::new("d:seq-symbol").expect("diagram id");
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

    let symbol = SymbolAnchor::new("sym-16c57df0026ced40", None).expect("symbol");
    let result = apply_ops(
        &mut diagram,
        1,
        &[Op::Seq(SeqOp::SetParticipantSymbol {
            participant_id: participant_id.clone(),
            symbol: Some(symbol),
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
    let participant = ast.participants().get(&participant_id).expect("participant exists");
    assert_eq!(
        participant.symbol().map(|symbol| symbol.stable_symbol_id()),
        Some("sym-16c57df0026ced40")
    );
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
            section_id: None,
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
            section_id: None,
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
            section_id: None,
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
            section_id: None,
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
fn apply_seq_add_message_rejects_empty_text() {
    let diagram_id = DiagramId::new("d:add-empty-text").expect("diagram id");
    let diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice = ObjectId::new("p:alice").expect("participant id");
    let bob = ObjectId::new("p:bob").expect("participant id");
    let message_id = ObjectId::new("m:1").expect("message id");

    for text in ["", "   ", "\t\n"] {
        let mut diagram = diagram.clone();
        let err = apply_ops(
            &mut diagram,
            0,
            &[
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
                    from_participant_id: alice.clone(),
                    to_participant_id: bob.clone(),
                    kind: crate::model::SequenceMessageKind::Sync,
                    arrow: None,
                    text: text.to_owned(),
                    order_key: 0,
                    section_id: None,
                }),
            ],
        )
        .unwrap_err();
        assert_eq!(
            err,
            super::ApplyError::InvalidSeqMessageText { text: text.to_owned() },
            "expected rejection for {text:?}"
        );
        assert_eq!(diagram.rev(), 0, "failed apply must not bump rev");
    }
}

#[test]
fn apply_seq_update_message_rejects_empty_text() {
    let diagram_id = DiagramId::new("d:update-empty-text").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "seq",
        DiagramAst::Sequence(crate::model::SequenceAst::default()),
    );

    let alice = ObjectId::new("p:alice").expect("participant id");
    let bob = ObjectId::new("p:bob").expect("participant id");
    let message_id = ObjectId::new("m:1").expect("message id");

    let setup = [
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
            section_id: None,
        }),
    ];
    apply_ops(&mut diagram, 0, &setup).expect("setup");
    let rev = diagram.rev();

    for text in ["", "   ", "\t\n"] {
        let err = apply_ops(
            &mut diagram,
            rev,
            &[Op::Seq(SeqOp::UpdateMessage {
                message_id: message_id.clone(),
                patch: SeqMessagePatch { text: Some(text.to_owned()), ..Default::default() },
            })],
        )
        .unwrap_err();
        assert_eq!(
            err,
            super::ApplyError::InvalidSeqMessageText { text: text.to_owned() },
            "expected rejection for {text:?}"
        );
        assert_eq!(diagram.rev(), rev, "failed apply must not bump rev");
    }

    let DiagramAst::Sequence(ast) = diagram.ast() else {
        panic!("expected sequence ast");
    };
    assert_eq!(
        ast.messages().iter().find(|m| m.message_id() == &message_id).expect("message").text(),
        "hi"
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
        Op::Flow(FlowOp::AddNode { node_id: n1.clone(), label: "Start".to_owned(), shape: None }),
        Op::Flow(FlowOp::AddNode { node_id: n2.clone(), label: "End".to_owned(), shape: None }),
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
        Op::Flow(FlowOp::AddNode { node_id: n1.clone(), label: "Start".to_owned(), shape: None }),
        Op::Flow(FlowOp::AddNode { node_id: n2.clone(), label: "End".to_owned(), shape: None }),
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

    let result =
        apply_ops(&mut diagram, 1, &[Op::Flow(FlowOp::RemoveNode { node_id: n1.clone() })])
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
            patch: FlowNodePatch { label: Some("Begin".to_owned()), ..Default::default() },
        }),
        Op::Flow(FlowOp::UpdateNode {
            node_id: node_id.clone(),
            patch: FlowNodePatch { shape: Some("circle".to_owned()), ..Default::default() },
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
        &[Op::Flow(FlowOp::SetNodeMermaidId { node_id, mermaid_id: Some("bad-id".to_owned()) })],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::InvalidFlowNodeMermaidId { .. }));
}

#[test]
fn apply_flow_add_node_rejects_invalid_derived_mermaid_id() {
    let diagram_id = DiagramId::new("d:add-node-invalid-derived-id").expect("diagram id");
    let mut diagram = crate::model::Diagram::new(
        diagram_id,
        "flow",
        DiagramAst::Flowchart(FlowchartAst::default()),
    );

    let node_id = ObjectId::new("n:bad-id").expect("node id");
    let err = apply_ops(
        &mut diagram,
        0,
        &[Op::Flow(FlowOp::AddNode { node_id, label: "Bad".to_owned(), shape: None })],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::InvalidFlowNodeMermaidId { .. }));
    assert_eq!(diagram.rev(), 0);
    let DiagramAst::Flowchart(ast) = diagram.ast() else {
        panic!("expected flowchart ast");
    };
    assert!(ast.nodes().is_empty());
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
        &[Op::Flow(FlowOp::SetNodeMermaidId { node_id: node_a, mermaid_id: Some("b".to_owned()) })],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::DuplicateFlowNodeMermaidId { .. }));
}

#[test]
fn apply_flow_add_node_rejects_duplicate_derived_mermaid_id() {
    let diagram_id = DiagramId::new("d:add-node-duplicate-derived-id").expect("diagram id");
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
        &[Op::Flow(FlowOp::AddNode {
            node_id: node_a.clone(),
            label: "A".to_owned(),
            shape: None,
        })],
    )
    .expect("setup add");
    apply_ops(
        &mut diagram,
        1,
        &[Op::Flow(FlowOp::SetNodeMermaidId { node_id: node_a, mermaid_id: Some("b".to_owned()) })],
    )
    .expect("setup mermaid id");

    let err = apply_ops(
        &mut diagram,
        2,
        &[Op::Flow(FlowOp::AddNode { node_id: node_b, label: "B".to_owned(), shape: None })],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::DuplicateFlowNodeMermaidId { .. }));
}

#[test]
fn apply_flow_clear_node_mermaid_id_rejects_derived_duplicate() {
    let diagram_id = DiagramId::new("d:clear-node-mermaid-id-duplicate").expect("diagram id");
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
    .expect("setup add");
    apply_ops(
        &mut diagram,
        1,
        &[Op::Flow(FlowOp::SetNodeMermaidId {
            node_id: node_a.clone(),
            mermaid_id: Some("c".to_owned()),
        })],
    )
    .expect("setup node a mermaid id");
    apply_ops(
        &mut diagram,
        2,
        &[Op::Flow(FlowOp::SetNodeMermaidId { node_id: node_b, mermaid_id: Some("a".to_owned()) })],
    )
    .expect("setup node b mermaid id");

    let err = apply_ops(
        &mut diagram,
        3,
        &[Op::Flow(FlowOp::SetNodeMermaidId { node_id: node_a, mermaid_id: None })],
    )
    .unwrap_err();

    assert!(matches!(err, super::ApplyError::DuplicateFlowNodeMermaidId { .. }));
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
        &[Op::Flow(FlowOp::SetNodeNote { node_id: node_id.clone(), note: None })],
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
fn apply_flow_set_node_symbol_clears_symbol_and_records_delta_updated() {
    let diagram_id = DiagramId::new("d:flow-symbol").expect("diagram id");
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
        &[Op::Flow(FlowOp::SetNodeSymbol {
            node_id: node_id.clone(),
            symbol: Some(
                SymbolAnchor::new("sym-468b7c6", Some("repo".to_owned())).expect("symbol"),
            ),
        })],
    )
    .expect("apply set");

    let result = apply_ops(
        &mut diagram,
        2,
        &[Op::Flow(FlowOp::SetNodeSymbol { node_id: node_id.clone(), symbol: None })],
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
    assert!(node.symbol().is_none());
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
        Op::Flow(FlowOp::AddNode { node_id: n1.clone(), label: "One".to_owned(), shape: None }),
        Op::Flow(FlowOp::AddNode { node_id: n2.clone(), label: "Two".to_owned(), shape: None }),
        Op::Flow(FlowOp::AddNode { node_id: n3.clone(), label: "Three".to_owned(), shape: None }),
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
            section_id: None,
        }),
    ];
    apply_ops(&mut diagram, 0, &setup_ops).expect("setup apply");

    let result = apply_ops(
        &mut diagram,
        1,
        &[Op::Seq(SeqOp::RemoveParticipant { participant_id: alice.clone() })],
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

#[test]
fn remove_message_prunes_section_membership_and_keeps_export_valid() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:0001").expect("id");
    let m2 = ObjectId::new("m:0002").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        m1.clone(),
        a.clone(),
        b.clone(),
        SequenceMessageKind::Sync,
        "One",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        m2.clone(),
        b.clone(),
        a.clone(),
        SequenceMessageKind::Sync,
        "Two",
        2000,
    ));
    ast.blocks_mut().push(SequenceBlock::new(
        ObjectId::new("b:0001").expect("id"),
        SequenceBlockKind::Alt,
        Some("X".to_owned()),
        vec![
            SequenceSection::new(
                ObjectId::new("sec:0001:00").expect("id"),
                SequenceSectionKind::Main,
                Some("X".to_owned()),
                vec![m1.clone()],
            ),
            SequenceSection::new(
                ObjectId::new("sec:0001:01").expect("id"),
                SequenceSectionKind::Else,
                Some("Y".to_owned()),
                vec![m2.clone()],
            ),
        ],
        Vec::new(),
    ));

    let mut diagram =
        Diagram::new(DiagramId::new("d:1").expect("id"), "seq", DiagramAst::Sequence(ast));

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    export_sequence_diagram(ast).expect("export before edit");

    apply_ops(&mut diagram, 0, &[Op::Seq(SeqOp::RemoveMessage { message_id: m2.clone() })])
        .expect("remove message");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    fn assert_message_pruned(blocks: &[SequenceBlock], message_id: &ObjectId) {
        for block in blocks {
            for section in block.sections() {
                assert!(
                    !section.message_ids().contains(message_id),
                    "section still references removed message {message_id}",
                );
            }
            assert_message_pruned(block.blocks(), message_id);
        }
    }

    assert_message_pruned(ast.blocks(), &m2);
    export_sequence_diagram(ast).expect("export after removing a section's message must succeed");
}

#[test]
fn remove_message_relabels_first_surviving_alt_branch() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:0001").expect("id");
    let m2 = ObjectId::new("m:0002").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        m1.clone(),
        a.clone(),
        b.clone(),
        SequenceMessageKind::Sync,
        "ok",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        m2.clone(),
        b.clone(),
        a.clone(),
        SequenceMessageKind::Sync,
        "fail",
        2000,
    ));
    ast.blocks_mut().push(SequenceBlock::new(
        ObjectId::new("b:0001").expect("id"),
        SequenceBlockKind::Alt,
        Some("ok".to_owned()),
        vec![
            SequenceSection::new(
                ObjectId::new("sec:0001:00").expect("id"),
                SequenceSectionKind::Main,
                None,
                vec![m1.clone()],
            ),
            SequenceSection::new(
                ObjectId::new("sec:0001:01").expect("id"),
                SequenceSectionKind::Else,
                Some("fail".to_owned()),
                vec![m2.clone()],
            ),
        ],
        Vec::new(),
    ));

    let mut diagram =
        Diagram::new(DiagramId::new("d:1").expect("id"), "seq", DiagramAst::Sequence(ast));

    apply_ops(&mut diagram, 0, &[Op::Seq(SeqOp::RemoveMessage { message_id: m1 })])
        .expect("remove message");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    let out = export_sequence_diagram(ast).expect("export after pruning first branch");
    assert!(out.contains("alt fail"), "remaining branch header should become opener:\n{out}");
    assert!(!out.contains("alt ok"), "removed branch header must not be reused:\n{out}");
    assert!(!out.contains("else fail"), "first surviving branch should not export as else:\n{out}");
}

#[test]
fn remove_message_keeps_parent_block_when_nested_content_survives() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:0001").expect("id");
    let m2 = ObjectId::new("m:0002").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        m1.clone(),
        a.clone(),
        b.clone(),
        SequenceMessageKind::Sync,
        "outer",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        m2.clone(),
        b.clone(),
        a.clone(),
        SequenceMessageKind::Sync,
        "inner",
        2000,
    ));
    ast.blocks_mut().push(SequenceBlock::new(
        ObjectId::new("b:0001").expect("id"),
        SequenceBlockKind::Alt,
        Some("outer".to_owned()),
        vec![SequenceSection::new(
            ObjectId::new("sec:0001:00").expect("id"),
            SequenceSectionKind::Main,
            None,
            vec![m1.clone()],
        )],
        vec![SequenceBlock::new(
            ObjectId::new("b:0002").expect("id"),
            SequenceBlockKind::Loop,
            Some("inner".to_owned()),
            vec![SequenceSection::new(
                ObjectId::new("sec:0002:00").expect("id"),
                SequenceSectionKind::Main,
                None,
                vec![m2.clone()],
            )],
            Vec::new(),
        )],
    ));

    let mut diagram =
        Diagram::new(DiagramId::new("d:1").expect("id"), "seq", DiagramAst::Sequence(ast));

    apply_ops(&mut diagram, 0, &[Op::Seq(SeqOp::RemoveMessage { message_id: m1 })])
        .expect("remove message");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    assert_eq!(ast.blocks().len(), 1, "parent block should keep surviving nested block");
    assert_eq!(ast.blocks()[0].blocks().len(), 1, "nested block should be preserved");
    export_sequence_diagram(ast).expect("export after pruning parent section");
}

#[test]
fn remove_message_records_synthetic_main_section_in_delta_added() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::{CategoryPath, Diagram, ObjectRef};

    // Parent Main holds only M1; nested block holds M2. Removing M1 empties parent sections and
    // synthesizes Main `b:0001:main` with nested membership — that new section id must appear in
    // delta.added so diagram_diff / MCP clients learn the live ref.
    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:0001").expect("id");
    let m2 = ObjectId::new("m:0002").expect("id");
    let parent_main = ObjectId::new("sec:0001:00").expect("id");
    let block_id = ObjectId::new("b:0001").expect("id");
    let synthetic_main = ObjectId::new("b:0001:main").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        m1.clone(),
        a.clone(),
        b.clone(),
        SequenceMessageKind::Sync,
        "outer",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        m2.clone(),
        b.clone(),
        a.clone(),
        SequenceMessageKind::Sync,
        "inner",
        2000,
    ));
    ast.blocks_mut().push(SequenceBlock::new(
        block_id.clone(),
        SequenceBlockKind::Alt,
        Some("outer".to_owned()),
        vec![SequenceSection::new(
            parent_main.clone(),
            SequenceSectionKind::Main,
            None,
            vec![m1.clone()],
        )],
        vec![SequenceBlock::new(
            ObjectId::new("b:0002").expect("id"),
            SequenceBlockKind::Loop,
            Some("inner".to_owned()),
            vec![SequenceSection::new(
                ObjectId::new("sec:0002:00").expect("id"),
                SequenceSectionKind::Main,
                None,
                vec![m2.clone()],
            )],
            Vec::new(),
        )],
    ));

    let diagram_id = DiagramId::new("d:1").expect("id");
    let mut diagram = Diagram::new(diagram_id.clone(), "seq", DiagramAst::Sequence(ast));

    let result =
        apply_ops(&mut diagram, 0, &[Op::Seq(SeqOp::RemoveMessage { message_id: m1.clone() })])
            .expect("remove message that triggers synthetic Main");

    let expected_synthetic = ObjectRef::new(
        diagram_id.clone(),
        CategoryPath::new(vec!["seq".to_owned(), "section".to_owned()]).expect("category"),
        synthetic_main.clone(),
    );
    let expected_removed_section = ObjectRef::new(
        diagram_id.clone(),
        CategoryPath::new(vec!["seq".to_owned(), "section".to_owned()]).expect("category"),
        parent_main.clone(),
    );
    let expected_removed_message = ObjectRef::new(
        diagram_id,
        CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()]).expect("category"),
        m1.clone(),
    );

    assert!(
        result.delta.added.contains(&expected_synthetic),
        "synthetic Main section must appear in delta.added; got added={:?}",
        result.delta.added,
    );
    assert!(
        result.delta.removed.contains(&expected_removed_section),
        "old emptied Main should be removed; got removed={:?}",
        result.delta.removed,
    );
    assert!(
        result.delta.removed.contains(&expected_removed_message),
        "removed message should be listed; got removed={:?}",
        result.delta.removed,
    );
    assert!(
        !result.delta.updated.iter().any(|r| r.object_id() == &synthetic_main),
        "synthetic Main must not also be listed as updated",
    );

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    assert_eq!(ast.blocks()[0].sections().len(), 1);
    assert_eq!(ast.blocks()[0].sections()[0].section_id(), &synthetic_main);
    assert_eq!(ast.blocks()[0].sections()[0].message_ids(), &[m2.clone()]);
    export_sequence_diagram(ast).expect("export after synthetic Main recovery");
}

#[test]
fn remove_message_keeps_single_main_when_nested_and_main_remain() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::Diagram;

    // Outer alt Main holds M1 + nested M2 (ancestor membership). Nested opt holds M2.
    // Removing M1 must leave a single non-empty Main — not synthesize a second Main.
    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:0001").expect("id");
    let m2 = ObjectId::new("m:0002").expect("id");
    let alt_main = ObjectId::new("sec:0001:00").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        m1.clone(),
        a.clone(),
        b.clone(),
        SequenceMessageKind::Sync,
        "outer",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        m2.clone(),
        b.clone(),
        a.clone(),
        SequenceMessageKind::Sync,
        "inner",
        2000,
    ));
    ast.blocks_mut().push(SequenceBlock::new(
        ObjectId::new("b:0001").expect("id"),
        SequenceBlockKind::Alt,
        Some("outer".to_owned()),
        vec![SequenceSection::new(
            alt_main.clone(),
            SequenceSectionKind::Main,
            None,
            vec![m1.clone(), m2.clone()],
        )],
        vec![SequenceBlock::new(
            ObjectId::new("b:0002").expect("id"),
            SequenceBlockKind::Opt,
            Some("inner".to_owned()),
            vec![SequenceSection::new(
                ObjectId::new("sec:0002:00").expect("id"),
                SequenceSectionKind::Main,
                None,
                vec![m2.clone()],
            )],
            Vec::new(),
        )],
    ));

    let mut diagram =
        Diagram::new(DiagramId::new("d:1").expect("id"), "seq", DiagramAst::Sequence(ast));

    export_sequence_diagram(match diagram.ast() {
        DiagramAst::Sequence(ast) => ast,
        _ => panic!("sequence"),
    })
    .expect("export before edit");

    apply_ops(&mut diagram, 0, &[Op::Seq(SeqOp::RemoveMessage { message_id: m1 })])
        .expect("remove message leaving non-empty Main with nested blocks");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    let parent = &ast.blocks()[0];
    let main_count =
        parent.sections().iter().filter(|s| s.kind() == SequenceSectionKind::Main).count();
    assert_eq!(main_count, 1, "must not synthesize a second Main when Main still has messages");
    assert_eq!(parent.sections().len(), 1);
    assert_eq!(parent.sections()[0].section_id(), &alt_main);
    assert_eq!(parent.sections()[0].message_ids(), &[m2.clone()]);
    assert_eq!(parent.blocks().len(), 1, "nested block should be preserved");
    export_sequence_diagram(ast).expect("export after remove with nested + non-empty Main");
}

#[test]
fn apply_seq_block_ops_update_remove_and_add_section() {
    use super::{SeqBlockPatch, SeqSectionPatch};
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:0001").expect("id");
    let m2 = ObjectId::new("m:0002").expect("id");
    let block_id = ObjectId::new("b:alt").expect("id");
    let main_sec = ObjectId::new("sec:alt:main").expect("id");
    let else_sec = ObjectId::new("sec:alt:else").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        m1.clone(),
        a.clone(),
        b.clone(),
        SequenceMessageKind::Sync,
        "hit",
        1000,
    ));
    ast.messages_mut().push(SequenceMessage::new(
        m2.clone(),
        a.clone(),
        b.clone(),
        SequenceMessageKind::Sync,
        "miss",
        2000,
    ));
    ast.blocks_mut().push(SequenceBlock::new(
        block_id.clone(),
        SequenceBlockKind::Alt,
        Some("cache".to_owned()),
        vec![
            SequenceSection::new(
                main_sec.clone(),
                SequenceSectionKind::Main,
                None,
                vec![m1.clone()],
            ),
            SequenceSection::new(
                else_sec.clone(),
                SequenceSectionKind::Else,
                Some("miss".to_owned()),
                vec![m2.clone()],
            ),
        ],
        Vec::new(),
    ));

    let mut diagram =
        Diagram::new(DiagramId::new("d:blocks").expect("id"), "seq", DiagramAst::Sequence(ast));

    apply_ops(
        &mut diagram,
        0,
        &[Op::Seq(SeqOp::UpdateBlock {
            block_id: block_id.clone(),
            patch: SeqBlockPatch { header: Some(Some("cache-lookup".to_owned())) },
        })],
    )
    .expect("update block");

    apply_ops(
        &mut diagram,
        1,
        &[Op::Seq(SeqOp::UpdateSection {
            section_id: else_sec.clone(),
            patch: SeqSectionPatch { header: Some(Some("cache-miss".to_owned())) },
        })],
    )
    .expect("update section");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    assert_eq!(ast.find_block(&block_id).expect("block").header(), Some("cache-lookup"));
    assert_eq!(ast.find_section(&else_sec).expect("else").header(), Some("cache-miss"));
    export_sequence_diagram(ast).expect("export after header updates");

    apply_ops(&mut diagram, 2, &[Op::Seq(SeqOp::RemoveMessage { message_id: m2.clone() })])
        .expect("remove else message");

    apply_ops(
        &mut diagram,
        3,
        &[Op::Seq(SeqOp::AddSection {
            section_id: ObjectId::new("sec:alt:else2").expect("id"),
            block_id: block_id.clone(),
            kind: SequenceSectionKind::Else,
            header: Some("empty".to_owned()),
        })],
    )
    .expect_err("empty else section must fail export validation");

    apply_ops(&mut diagram, 3, &[Op::Seq(SeqOp::RemoveBlock { block_id: block_id.clone() })])
        .expect("remove block");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("sequence") };
    assert!(!ast.contains_block_id(&block_id));
    assert!(ast.messages().iter().any(|msg| msg.message_id() == &m1));
    assert!(export_sequence_diagram(ast).is_ok());
}

#[test]
fn apply_seq_add_block_alone_fails_export_validation_until_messages_attached() {
    use crate::model::seq_ast::SequenceBlockKind;
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a, SequenceParticipant::new("A"));
    ast.participants_mut().insert(b, SequenceParticipant::new("B"));
    let mut diagram = Diagram::new(
        DiagramId::new("d:empty-block").expect("id"),
        "seq",
        DiagramAst::Sequence(ast),
    );

    let err = apply_ops(
        &mut diagram,
        0,
        &[Op::Seq(SeqOp::AddBlock {
            block_id: ObjectId::new("b:1").expect("id"),
            kind: SequenceBlockKind::Loop,
            header: Some("retry".to_owned()),
            parent_block_id: None,
            main_section_id: ObjectId::new("sec:1:main").expect("id"),
        })],
    )
    .expect_err("empty block is not export-valid");

    assert!(matches!(err, super::ApplyError::InvalidSeqBlockStructure { .. }));
    assert_eq!(diagram.rev(), 0);
}

#[test]
fn apply_seq_builds_alt_else_via_structure_and_membership_ops() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{SequenceBlockKind, SequenceMessageKind, SequenceSectionKind};
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:hit").expect("id");
    let m2 = ObjectId::new("m:miss").expect("id");
    let block_id = ObjectId::new("b:cache").expect("id");
    let main_sec = ObjectId::new("sec:cache:main").expect("id");
    let else_sec = ObjectId::new("sec:cache:else").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    let mut diagram =
        Diagram::new(DiagramId::new("d:alt").expect("id"), "seq", DiagramAst::Sequence(ast));

    let result = apply_ops(
        &mut diagram,
        0,
        &[
            Op::Seq(SeqOp::AddBlock {
                block_id: block_id.clone(),
                kind: SequenceBlockKind::Alt,
                header: Some("cache".to_owned()),
                parent_block_id: None,
                main_section_id: main_sec.clone(),
            }),
            Op::Seq(SeqOp::AddSection {
                section_id: else_sec.clone(),
                block_id: block_id.clone(),
                kind: SequenceSectionKind::Else,
                header: Some("miss".to_owned()),
            }),
            Op::Seq(SeqOp::AddMessage {
                message_id: m1.clone(),
                from_participant_id: a.clone(),
                to_participant_id: b.clone(),
                kind: SequenceMessageKind::Sync,
                arrow: None,
                text: "hit".to_owned(),
                order_key: 1000,
                section_id: Some(main_sec.clone()),
            }),
            Op::Seq(SeqOp::AddMessage {
                message_id: m2.clone(),
                from_participant_id: a.clone(),
                to_participant_id: b.clone(),
                kind: SequenceMessageKind::Sync,
                arrow: None,
                text: "miss".to_owned(),
                order_key: 2000,
                section_id: Some(else_sec.clone()),
            }),
        ],
    )
    .expect("build alt/else");

    assert_eq!(result.new_rev, 1);
    assert!(result.delta.added.iter().any(|r| r.object_id() == &block_id));
    assert!(result.delta.added.iter().any(|r| r.object_id() == &else_sec));

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("seq") };
    assert_eq!(ast.find_section(&main_sec).expect("main").message_ids(), std::slice::from_ref(&m1));
    assert_eq!(ast.find_section(&else_sec).expect("else").message_ids(), std::slice::from_ref(&m2));
    let exported = export_sequence_diagram(ast).expect("export alt");
    assert!(exported.contains("alt cache"));
    assert!(exported.contains("else miss"));
    assert!(exported.contains("A->>B: hit"));
    assert!(exported.contains("A->>B: miss"));

    // Empty main section after move is export-invalid.
    let err = apply_ops(
        &mut diagram,
        1,
        &[Op::Seq(SeqOp::SetMessageSection {
            message_id: m1.clone(),
            section_id: Some(else_sec.clone()),
        })],
    )
    .expect_err("empty main section is invalid");
    assert!(matches!(err, super::ApplyError::InvalidSeqBlockStructure { .. }));

    apply_ops(
        &mut diagram,
        1,
        &[
            Op::Seq(SeqOp::SetMessageSection { message_id: m1.clone(), section_id: None }),
            Op::Seq(SeqOp::SetMessageSection { message_id: m2.clone(), section_id: None }),
            Op::Seq(SeqOp::RemoveBlock { block_id: block_id.clone() }),
        ],
    )
    .expect("detach and remove block");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("seq") };
    assert!(!ast.contains_block_id(&block_id));
    assert_eq!(ast.messages().len(), 2);
}

#[test]
fn apply_seq_builds_nested_opt_inside_alt_via_ops() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{SequenceBlockKind, SequenceMessageKind, SequenceSectionKind};
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m_outer = ObjectId::new("m:outer").expect("id");
    let m_inner = ObjectId::new("m:inner").expect("id");
    let m_else = ObjectId::new("m:else").expect("id");
    let alt_id = ObjectId::new("b:alt").expect("id");
    let opt_id = ObjectId::new("b:opt").expect("id");
    let alt_main = ObjectId::new("sec:alt:main").expect("id");
    let alt_else = ObjectId::new("sec:alt:else").expect("id");
    let opt_main = ObjectId::new("sec:opt:main").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    let mut diagram =
        Diagram::new(DiagramId::new("d:nested").expect("id"), "seq", DiagramAst::Sequence(ast));

    apply_ops(
        &mut diagram,
        0,
        &[
            Op::Seq(SeqOp::AddBlock {
                block_id: alt_id.clone(),
                kind: SequenceBlockKind::Alt,
                header: Some("outer".to_owned()),
                parent_block_id: None,
                main_section_id: alt_main.clone(),
            }),
            Op::Seq(SeqOp::AddSection {
                section_id: alt_else.clone(),
                block_id: alt_id.clone(),
                kind: SequenceSectionKind::Else,
                header: Some("else".to_owned()),
            }),
            Op::Seq(SeqOp::AddBlock {
                block_id: opt_id.clone(),
                kind: SequenceBlockKind::Opt,
                header: Some("warm".to_owned()),
                parent_block_id: Some(alt_id.clone()),
                main_section_id: opt_main.clone(),
            }),
            Op::Seq(SeqOp::AddMessage {
                message_id: m_outer.clone(),
                from_participant_id: a.clone(),
                to_participant_id: b.clone(),
                kind: SequenceMessageKind::Sync,
                arrow: None,
                text: "outer".to_owned(),
                order_key: 1000,
                section_id: Some(alt_main.clone()),
            }),
            Op::Seq(SeqOp::AddMessage {
                message_id: m_inner.clone(),
                from_participant_id: a.clone(),
                to_participant_id: b.clone(),
                kind: SequenceMessageKind::Sync,
                arrow: None,
                text: "inner".to_owned(),
                order_key: 2000,
                section_id: Some(opt_main.clone()),
            }),
            Op::Seq(SeqOp::AddMessage {
                message_id: m_else.clone(),
                from_participant_id: a.clone(),
                to_participant_id: b.clone(),
                kind: SequenceMessageKind::Sync,
                arrow: None,
                text: "else".to_owned(),
                order_key: 3000,
                section_id: Some(alt_else.clone()),
            }),
        ],
    )
    .expect("build nested structure");

    let DiagramAst::Sequence(ast) = diagram.ast() else { panic!("seq") };
    assert!(ast.find_section(&opt_main).expect("opt").message_ids().contains(&m_inner));
    assert!(ast.find_section(&alt_main).expect("alt main").message_ids().contains(&m_inner));
    assert!(ast.find_section(&alt_main).expect("alt main").message_ids().contains(&m_outer));

    let exported = export_sequence_diagram(ast).expect("export nested");
    assert!(exported.contains("alt outer"), "{exported}");
    assert!(exported.contains("opt warm"), "{exported}");
    assert!(exported.contains("else else"), "{exported}");
}

#[test]
fn apply_seq_add_section_rejects_else_under_loop() {
    use crate::format::mermaid::sequence::export_sequence_diagram;
    use crate::model::seq_ast::{
        SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::Diagram;

    let a = ObjectId::new("p:a").expect("id");
    let b = ObjectId::new("p:b").expect("id");
    let m1 = ObjectId::new("m:1").expect("id");
    let block_id = ObjectId::new("b:loop").expect("id");
    let main_sec = ObjectId::new("sec:loop:main").expect("id");

    let mut ast = SequenceAst::default();
    ast.participants_mut().insert(a.clone(), SequenceParticipant::new("A"));
    ast.participants_mut().insert(b.clone(), SequenceParticipant::new("B"));
    ast.messages_mut().push(SequenceMessage::new(
        m1.clone(),
        a,
        b,
        SequenceMessageKind::Sync,
        "again",
        1000,
    ));
    ast.blocks_mut().push(SequenceBlock::new(
        block_id.clone(),
        SequenceBlockKind::Loop,
        Some("retry".to_owned()),
        vec![SequenceSection::new(main_sec, SequenceSectionKind::Main, None, vec![m1])],
        Vec::new(),
    ));
    let mut diagram =
        Diagram::new(DiagramId::new("d:loop").expect("id"), "seq", DiagramAst::Sequence(ast));
    let DiagramAst::Sequence(seq) = diagram.ast() else { panic!("seq") };
    export_sequence_diagram(seq).expect("baseline export");

    let err = apply_ops(
        &mut diagram,
        0,
        &[Op::Seq(SeqOp::AddSection {
            section_id: ObjectId::new("sec:loop:else").expect("id"),
            block_id,
            kind: SequenceSectionKind::Else,
            header: None,
        })],
    )
    .expect_err("else under loop");

    assert!(matches!(err, super::ApplyError::InvalidSeqSectionKind { .. }));
}
