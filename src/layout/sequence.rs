// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::BTreeMap;

use crate::model::ids::ObjectId;
use crate::model::seq_ast::SequenceAst;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceLayout {
    participant_cols: BTreeMap<ObjectId, usize>,
    messages: Vec<SequenceMessageLayout>,
}

impl SequenceLayout {
    pub fn participant_cols(&self) -> &BTreeMap<ObjectId, usize> {
        &self.participant_cols
    }

    pub fn messages(&self) -> &[SequenceMessageLayout] {
        &self.messages
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceMessageLayout {
    message_id: ObjectId,
    from_participant_id: ObjectId,
    to_participant_id: ObjectId,
    from_col: usize,
    to_col: usize,
    row: usize,
}

impl SequenceMessageLayout {
    pub fn message_id(&self) -> &ObjectId {
        &self.message_id
    }

    pub fn from_participant_id(&self) -> &ObjectId {
        &self.from_participant_id
    }

    pub fn to_participant_id(&self) -> &ObjectId {
        &self.to_participant_id
    }

    pub fn from_col(&self) -> usize {
        self.from_col
    }

    pub fn to_col(&self) -> usize {
        self.to_col
    }

    pub fn row(&self) -> usize {
        self.row
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceLayoutError {
    UnknownParticipant { message_id: ObjectId, participant_id: ObjectId },
}

impl std::fmt::Display for SequenceLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownParticipant { message_id, participant_id } => {
                write!(f, "message {message_id} references unknown participant {participant_id}")
            }
        }
    }
}

impl std::error::Error for SequenceLayoutError {}

/// Deterministic “coordinates-only” layout for a sequence diagram.
///
/// Baseline grid:
/// - `col`: assigned by participant `ObjectId` order (lexical by id)
/// - `row`: assigned by message `(order_key, message_id)` order
pub fn layout_sequence(ast: &SequenceAst) -> Result<SequenceLayout, SequenceLayoutError> {
    let mut participant_cols = BTreeMap::<ObjectId, usize>::new();
    for (idx, participant_id) in ast.participants().keys().enumerate() {
        participant_cols.insert(participant_id.clone(), idx);
    }

    let messages = ast
        .messages_in_order()
        .into_iter()
        .enumerate()
        .map(|(row, msg)| {
            let from_participant_id = msg.from_participant_id().clone();
            let to_participant_id = msg.to_participant_id().clone();

            let from_col = *participant_cols.get(&from_participant_id).ok_or_else(|| {
                SequenceLayoutError::UnknownParticipant {
                    message_id: msg.message_id().clone(),
                    participant_id: from_participant_id.clone(),
                }
            })?;
            let to_col = *participant_cols.get(&to_participant_id).ok_or_else(|| {
                SequenceLayoutError::UnknownParticipant {
                    message_id: msg.message_id().clone(),
                    participant_id: to_participant_id.clone(),
                }
            })?;

            Ok(SequenceMessageLayout {
                message_id: msg.message_id().clone(),
                from_participant_id,
                to_participant_id,
                from_col,
                to_col,
                row,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SequenceLayout { participant_cols, messages })
}

#[cfg(test)]
mod tests {
    use super::{layout_sequence, SequenceLayoutError};
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    };

    fn fixture_ast_messages_out_of_order() -> SequenceAst {
        let mut ast = SequenceAst::default();

        let p_bob = ObjectId::new("p:bob").expect("participant id");
        let p_alice = ObjectId::new("p:alice").expect("participant id");
        let p_carol = ObjectId::new("p:carol").expect("participant id");

        // Insert participants intentionally out of order; BTreeMap should keep deterministic ordering.
        ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));
        ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
        ast.participants_mut().insert(p_carol.clone(), SequenceParticipant::new("Carol"));

        let m_0002 = ObjectId::new("m:0002").expect("message id");
        let m_0001 = ObjectId::new("m:0001").expect("message id");
        let m_0003 = ObjectId::new("m:0003").expect("message id");

        // Intentionally insert messages out of order and with a tie on order_key.
        ast.messages_mut().push(SequenceMessage::new(
            m_0003.clone(),
            p_bob.clone(),
            p_carol.clone(),
            SequenceMessageKind::Async,
            "After",
            2000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            m_0002.clone(),
            p_alice.clone(),
            p_bob.clone(),
            SequenceMessageKind::Sync,
            "Hello 2",
            1000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            m_0001.clone(),
            p_alice.clone(),
            p_bob.clone(),
            SequenceMessageKind::Sync,
            "Hello 1",
            1000,
        ));

        ast
    }

    #[test]
    fn layout_orders_participants_deterministically_by_object_id() {
        let ast = fixture_ast_messages_out_of_order();
        let layout = layout_sequence(&ast).expect("layout");

        let participants =
            layout.participant_cols().keys().map(|id| id.as_str().to_owned()).collect::<Vec<_>>();
        assert_eq!(participants, vec!["p:alice", "p:bob", "p:carol"]);

        assert_eq!(
            layout
                .participant_cols()
                .iter()
                .map(|(id, col)| (id.as_str().to_owned(), *col))
                .collect::<Vec<_>>(),
            vec![("p:alice".to_owned(), 0), ("p:bob".to_owned(), 1), ("p:carol".to_owned(), 2)]
        );
    }

    #[test]
    fn layout_orders_messages_deterministically_and_assigns_rows() {
        let ast = fixture_ast_messages_out_of_order();
        let layout = layout_sequence(&ast).expect("layout");

        let messages = layout
            .messages()
            .iter()
            .map(|msg| (msg.message_id().as_str().to_owned(), msg.row()))
            .collect::<Vec<_>>();
        // order_key tie breaks by message_id
        assert_eq!(
            messages,
            vec![("m:0001".to_owned(), 0), ("m:0002".to_owned(), 1), ("m:0003".to_owned(), 2)]
        );

        let m_0001 = &layout.messages()[0];
        assert_eq!(m_0001.from_col(), 0); // p:alice
        assert_eq!(m_0001.to_col(), 1); // p:bob
    }

    #[test]
    fn layout_is_stable_across_message_insertion_order() {
        let ast1 = fixture_ast_messages_out_of_order();

        let mut ast2 = SequenceAst::default();
        for (id, participant) in ast1.participants() {
            ast2.participants_mut().insert(id.clone(), participant.clone());
        }

        // Insert messages in reverse order.
        for msg in ast1.messages().iter().rev() {
            ast2.messages_mut().push(msg.clone());
        }

        let layout1 = layout_sequence(&ast1).expect("layout1");
        let layout2 = layout_sequence(&ast2).expect("layout2");
        assert_eq!(layout1, layout2);
    }

    #[test]
    fn layout_errors_on_unknown_participants() {
        let mut ast = SequenceAst::default();
        let p_alice = ObjectId::new("p:alice").expect("participant id");
        ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));

        let missing = ObjectId::new("p:missing").expect("participant id");
        let m_0001 = ObjectId::new("m:0001").expect("message id");
        ast.messages_mut().push(SequenceMessage::new(
            m_0001.clone(),
            p_alice.clone(),
            missing.clone(),
            SequenceMessageKind::Sync,
            "Hello",
            1000,
        ));

        assert_eq!(
            layout_sequence(&ast),
            Err(SequenceLayoutError::UnknownParticipant {
                message_id: m_0001,
                participant_id: missing,
            })
        );
    }
}
