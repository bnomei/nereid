// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

/// Sequence/flow mutation implementation helpers used by `apply_ops`.
/// Keeps `ops::mod` focused on public op types and orchestration.
fn apply_seq_op(
    diagram_id: &DiagramId,
    ast: &mut SequenceAst,
    op: &SeqOp,
    delta: &mut DeltaBuilder,
) -> Result<(), ApplyError> {
    match op {
        SeqOp::AddParticipant {
            participant_id,
            mermaid_name,
        } => {
            if ast.participants().contains_key(participant_id) {
                return Err(ApplyError::AlreadyExists {
                    kind: ObjectKind::SeqParticipant,
                    object_id: participant_id.clone(),
                });
            }
            ast.participants_mut().insert(
                participant_id.clone(),
                SequenceParticipant::new(mermaid_name.clone()),
            );
            delta.record_added(seq_participant_ref(diagram_id, participant_id));
            Ok(())
        }
        SeqOp::UpdateParticipant {
            participant_id,
            patch,
        } => {
            let Some(existing) = ast.participants_mut().get_mut(participant_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: participant_id.clone(),
                });
            };

            if let Some(mermaid_name) = &patch.mermaid_name {
                existing.set_mermaid_name(mermaid_name.clone());
            }
            delta.record_updated(seq_participant_ref(diagram_id, participant_id));
            Ok(())
        }
        SeqOp::SetParticipantNote {
            participant_id,
            note,
        } => {
            let Some(existing) = ast.participants_mut().get_mut(participant_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: participant_id.clone(),
                });
            };

            existing.set_note(note.as_deref());
            delta.record_updated(seq_participant_ref(diagram_id, participant_id));
            Ok(())
        }
        SeqOp::RemoveParticipant { participant_id } => {
            if ast.participants_mut().remove(participant_id).is_none() {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: participant_id.clone(),
                });
            }
            let removed_message_ids = ast
                .messages()
                .iter()
                .filter(|m| {
                    m.from_participant_id() == participant_id
                        || m.to_participant_id() == participant_id
                })
                .map(|m| m.message_id().clone())
                .collect::<Vec<_>>();
            ast.messages_mut().retain(|m| {
                m.from_participant_id() != participant_id && m.to_participant_id() != participant_id
            });
            for message_id in removed_message_ids {
                delta.record_removed(seq_message_ref(diagram_id, &message_id));
            }
            delta.record_removed(seq_participant_ref(diagram_id, participant_id));
            Ok(())
        }
        SeqOp::AddMessage {
            message_id,
            from_participant_id,
            to_participant_id,
            kind,
            arrow,
            text,
            order_key,
        } => {
            if ast.messages().iter().any(|m| m.message_id() == message_id) {
                return Err(ApplyError::AlreadyExists {
                    kind: ObjectKind::SeqMessage,
                    object_id: message_id.clone(),
                });
            }
            if !ast.participants().contains_key(from_participant_id) {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: from_participant_id.clone(),
                });
            }
            if !ast.participants().contains_key(to_participant_id) {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: to_participant_id.clone(),
                });
            }
            let mut message = SequenceMessage::new(
                message_id.clone(),
                from_participant_id.clone(),
                to_participant_id.clone(),
                *kind,
                text.clone(),
                *order_key,
            );
            message.set_raw_arrow(normalize_seq_raw_arrow(*kind, arrow.clone()));
            ast.messages_mut().push(message);
            sort_seq_messages(ast);
            delta.record_added(seq_message_ref(diagram_id, message_id));
            Ok(())
        }
        SeqOp::UpdateMessage { message_id, patch } => {
            let Some(index) = ast
                .messages()
                .iter()
                .position(|m| m.message_id() == message_id)
            else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqMessage,
                    object_id: message_id.clone(),
                });
            };

            let existing = &ast.messages()[index];
            let updated_from = patch
                .from_participant_id
                .clone()
                .unwrap_or_else(|| existing.from_participant_id().clone());
            let updated_to = patch
                .to_participant_id
                .clone()
                .unwrap_or_else(|| existing.to_participant_id().clone());
            let updated_kind = patch.kind.unwrap_or(existing.kind());
            let updated_arrow = patch
                .arrow
                .clone()
                .or_else(|| existing.raw_arrow().map(ToOwned::to_owned));
            let updated_text = patch
                .text
                .clone()
                .unwrap_or_else(|| existing.text().to_owned());
            let updated_order_key = patch.order_key.unwrap_or(existing.order_key());

            if !ast.participants().contains_key(&updated_from) {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: updated_from.clone(),
                });
            }
            if !ast.participants().contains_key(&updated_to) {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: updated_to.clone(),
                });
            }

            let mut updated = SequenceMessage::new(
                message_id.clone(),
                updated_from,
                updated_to,
                updated_kind,
                updated_text,
                updated_order_key,
            );
            updated.set_raw_arrow(normalize_seq_raw_arrow(updated_kind, updated_arrow));
            ast.messages_mut()[index] = updated;
            sort_seq_messages(ast);
            delta.record_updated(seq_message_ref(diagram_id, message_id));
            Ok(())
        }
        SeqOp::RemoveMessage { message_id } => {
            let before_len = ast.messages().len();
            ast.messages_mut().retain(|m| m.message_id() != message_id);
            if ast.messages().len() == before_len {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqMessage,
                    object_id: message_id.clone(),
                });
            }
            delta.record_removed(seq_message_ref(diagram_id, message_id));
            Ok(())
        }
    }
}

fn sort_seq_messages(ast: &mut SequenceAst) {
    ast.messages_mut().sort_by(SequenceMessage::cmp_in_order);
}

fn normalize_seq_raw_arrow(kind: SequenceMessageKind, raw_arrow: Option<String>) -> Option<String> {
    let raw_arrow = raw_arrow?;
    let trimmed = raw_arrow.trim();
    if trimmed.is_empty() {
        return None;
    }

    let canonical = match kind {
        SequenceMessageKind::Sync => "->>",
        SequenceMessageKind::Async => "-)",
        SequenceMessageKind::Return => "-->>",
    };

    (trimmed != canonical).then_some(trimmed.to_owned())
}

fn normalize_flow_connector(raw_connector: Option<String>) -> Option<String> {
    let raw_connector = raw_connector?;
    let trimmed = raw_connector.trim();
    if trimmed.is_empty() {
        return None;
    }

    let trimmed = if trimmed.contains('<') && !trimmed.contains('>') {
        let mut normalized = String::with_capacity(trimmed.len().saturating_add(1));
        for ch in trimmed.chars() {
            if ch != '<' {
                normalized.push(ch);
            }
        }

        match normalized.chars().last() {
            Some('o' | 'x') => {
                let decoration = normalized.pop().expect("non-empty after last()");
                normalized.push('>');
                normalized.push(decoration);
            }
            _ => normalized.push('>'),
        }

        normalized
    } else {
        trimmed.to_owned()
    };

    (trimmed != "-->").then_some(trimmed)
}

fn validate_flow_node_mermaid_id(mermaid_id: &str) -> Result<(), MermaidIdentError> {
    if mermaid_id.is_empty() {
        return Err(MermaidIdentError::Empty);
    }
    if mermaid_id.chars().any(|ch| ch.is_whitespace()) {
        return Err(MermaidIdentError::ContainsWhitespace);
    }
    if mermaid_id.contains('/') {
        return Err(MermaidIdentError::ContainsSlash);
    }
    if let Some(ch) = mermaid_id
        .chars()
        .find(|ch| !ch.is_ascii_alphanumeric() && *ch != '_')
    {
        return Err(MermaidIdentError::InvalidChar { ch });
    }
    Ok(())
}

fn flow_node_mermaid_id_for_uniqueness<'a>(
    node_id: &'a ObjectId,
    node: &'a FlowNode,
) -> Option<&'a str> {
    node.mermaid_id()
        .or_else(|| node_id.as_str().strip_prefix("n:"))
}

fn apply_flow_op(
    diagram_id: &DiagramId,
    ast: &mut FlowchartAst,
    op: &FlowOp,
    delta: &mut DeltaBuilder,
) -> Result<(), ApplyError> {
    match op {
        FlowOp::AddNode {
            node_id,
            label,
            shape,
        } => {
            if ast.nodes().contains_key(node_id) {
                return Err(ApplyError::AlreadyExists {
                    kind: ObjectKind::FlowNode,
                    object_id: node_id.clone(),
                });
            }
            let mut node = FlowNode::new(label.clone());
            if let Some(shape) = shape {
                node.set_shape(shape.clone());
            }
            ast.nodes_mut().insert(node_id.clone(), node);
            delta.record_added(flow_node_ref(diagram_id, node_id));
            Ok(())
        }
        FlowOp::UpdateNode { node_id, patch } => {
            let Some(existing) = ast.nodes_mut().get_mut(node_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::FlowNode,
                    object_id: node_id.clone(),
                });
            };
            if let Some(label) = &patch.label {
                existing.set_label(label.clone());
            }
            if let Some(shape) = &patch.shape {
                existing.set_shape(shape.clone());
            }
            delta.record_updated(flow_node_ref(diagram_id, node_id));
            Ok(())
        }
        FlowOp::SetNodeMermaidId {
            node_id,
            mermaid_id,
        } => {
            if !ast.nodes().contains_key(node_id) {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::FlowNode,
                    object_id: node_id.clone(),
                });
            }

            if let Some(mermaid_id) = mermaid_id.as_deref() {
                validate_flow_node_mermaid_id(mermaid_id).map_err(|reason| {
                    ApplyError::InvalidFlowNodeMermaidId {
                        mermaid_id: mermaid_id.to_owned(),
                        reason,
                    }
                })?;

                if let Some(other_node_id) =
                    ast.nodes().iter().find_map(|(candidate_id, candidate)| {
                        if candidate_id == node_id {
                            return None;
                        }

                        let candidate_mermaid_id =
                            flow_node_mermaid_id_for_uniqueness(candidate_id, candidate)?;
                        (candidate_mermaid_id == mermaid_id).then(|| candidate_id.clone())
                    })
                {
                    return Err(ApplyError::DuplicateFlowNodeMermaidId {
                        mermaid_id: mermaid_id.to_owned(),
                        node_id: other_node_id,
                    });
                }
            }

            let existing = ast
                .nodes_mut()
                .get_mut(node_id)
                .expect("node existence checked above");
            existing.set_mermaid_id(mermaid_id.clone());
            delta.record_updated(flow_node_ref(diagram_id, node_id));
            Ok(())
        }
        FlowOp::SetNodeNote { node_id, note } => {
            let Some(existing) = ast.nodes_mut().get_mut(node_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::FlowNode,
                    object_id: node_id.clone(),
                });
            };

            existing.set_note(note.as_deref());
            delta.record_updated(flow_node_ref(diagram_id, node_id));
            Ok(())
        }
        FlowOp::RemoveNode { node_id } => {
            if ast.nodes_mut().remove(node_id).is_none() {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::FlowNode,
                    object_id: node_id.clone(),
                });
            }
            let to_remove = ast
                .edges()
                .iter()
                .filter(|(_, e)| e.from_node_id() == node_id || e.to_node_id() == node_id)
                .map(|(edge_id, _)| edge_id.clone())
                .collect::<Vec<_>>();
            for edge_id in to_remove {
                ast.edges_mut().remove(&edge_id);
                delta.record_removed(flow_edge_ref(diagram_id, &edge_id));
            }
            delta.record_removed(flow_node_ref(diagram_id, node_id));
            Ok(())
        }
        FlowOp::AddEdge {
            edge_id,
            from_node_id,
            to_node_id,
            label,
            connector,
            style,
        } => {
            if ast.edges().contains_key(edge_id) {
                return Err(ApplyError::AlreadyExists {
                    kind: ObjectKind::FlowEdge,
                    object_id: edge_id.clone(),
                });
            }
            if !ast.nodes().contains_key(from_node_id) {
                return Err(ApplyError::MissingFlowNode {
                    node_id: from_node_id.clone(),
                });
            }
            if !ast.nodes().contains_key(to_node_id) {
                return Err(ApplyError::MissingFlowNode {
                    node_id: to_node_id.clone(),
                });
            }
            let mut edge = FlowEdge::new(from_node_id.clone(), to_node_id.clone());
            edge.set_label(label.clone());
            edge.set_connector(normalize_flow_connector(connector.clone()));
            edge.set_style(style.clone());
            ast.edges_mut().insert(edge_id.clone(), edge);
            delta.record_added(flow_edge_ref(diagram_id, edge_id));
            Ok(())
        }
        FlowOp::UpdateEdge { edge_id, patch } => {
            let (updated_from, updated_to, updated_label, updated_connector, updated_style) = {
                let Some(existing) = ast.edges().get(edge_id) else {
                    return Err(ApplyError::NotFound {
                        kind: ObjectKind::FlowEdge,
                        object_id: edge_id.clone(),
                    });
                };

                let updated_from = patch
                    .from_node_id
                    .clone()
                    .unwrap_or_else(|| existing.from_node_id().clone());
                let updated_to = patch
                    .to_node_id
                    .clone()
                    .unwrap_or_else(|| existing.to_node_id().clone());
                let updated_label = patch
                    .label
                    .clone()
                    .or_else(|| existing.label().map(ToOwned::to_owned));
                let updated_connector = patch
                    .connector
                    .clone()
                    .or_else(|| existing.connector().map(ToOwned::to_owned));
                let updated_style = patch
                    .style
                    .clone()
                    .or_else(|| existing.style().map(ToOwned::to_owned));

                (
                    updated_from,
                    updated_to,
                    updated_label,
                    updated_connector,
                    updated_style,
                )
            };

            if !ast.nodes().contains_key(&updated_from) {
                return Err(ApplyError::MissingFlowNode {
                    node_id: updated_from,
                });
            }
            if !ast.nodes().contains_key(&updated_to) {
                return Err(ApplyError::MissingFlowNode {
                    node_id: updated_to,
                });
            }

            let mut edge =
                FlowEdge::new_with(updated_from, updated_to, updated_label, updated_style);
            edge.set_connector(normalize_flow_connector(updated_connector));
            ast.edges_mut().insert(edge_id.clone(), edge);
            delta.record_updated(flow_edge_ref(diagram_id, edge_id));
            Ok(())
        }
        FlowOp::RemoveEdge { edge_id } => {
            if ast.edges_mut().remove(edge_id).is_none() {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::FlowEdge,
                    object_id: edge_id.clone(),
                });
            };
            delta.record_removed(flow_edge_ref(diagram_id, edge_id));
            Ok(())
        }
    }
}

fn seq_participant_ref(diagram_id: &DiagramId, participant_id: &ObjectId) -> ObjectRef {
    object_ref(diagram_id, &["seq", "participant"], participant_id)
}

fn seq_message_ref(diagram_id: &DiagramId, message_id: &ObjectId) -> ObjectRef {
    object_ref(diagram_id, &["seq", "message"], message_id)
}

fn flow_node_ref(diagram_id: &DiagramId, node_id: &ObjectId) -> ObjectRef {
    object_ref(diagram_id, &["flow", "node"], node_id)
}

fn flow_edge_ref(diagram_id: &DiagramId, edge_id: &ObjectId) -> ObjectRef {
    object_ref(diagram_id, &["flow", "edge"], edge_id)
}

fn object_ref(
    diagram_id: &DiagramId,
    category_segments: &[&str],
    object_id: &ObjectId,
) -> ObjectRef {
    let category = CategoryPath::new(
        category_segments
            .iter()
            .copied()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
    )
    .expect("static category path");

    ObjectRef::new(diagram_id.clone(), category, object_id.clone())
}
