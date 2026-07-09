// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Identity reconciliation for diagrams loaded from Mermaid + sidecar meta.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{DiagramMeta, DiagramStableIdMap};
use crate::model::{
    DiagramAst, FlowEdge, FlowNode, FlowchartAst, ObjectId, SequenceAst, SequenceMessage,
    SequenceMessageKind, SymbolAnchor,
};

pub(crate) fn stable_id_map_from_ast(ast: &DiagramAst) -> DiagramStableIdMap {
    match ast {
        DiagramAst::Sequence(seq_ast) => {
            let mut by_name = BTreeMap::new();
            for (participant_id, participant) in seq_ast.participants() {
                let mermaid_name = participant.mermaid_name();
                if mermaid_name.is_empty() {
                    continue;
                }
                by_name
                    .entry(mermaid_name.to_owned())
                    .or_insert_with(|| participant_id.to_string());
            }

            DiagramStableIdMap { by_mermaid_id: BTreeMap::new(), by_name }
        }
        DiagramAst::Flowchart(flow_ast) => {
            let mut by_mermaid_id = BTreeMap::new();
            for (node_id, node) in flow_ast.nodes() {
                let Some(mermaid_id) = flow_node_mermaid_id(node_id, node.mermaid_id()) else {
                    continue;
                };
                by_mermaid_id.entry(mermaid_id).or_insert_with(|| node_id.to_string());
            }

            DiagramStableIdMap { by_mermaid_id, by_name: BTreeMap::new() }
        }
    }
}

fn flow_node_mermaid_id(node_id: &ObjectId, mermaid_id: Option<&str>) -> Option<String> {
    mermaid_id.filter(|id| !id.is_empty()).map(ToOwned::to_owned).or_else(|| {
        node_id.as_str().strip_prefix("n:").filter(|id| !id.is_empty()).map(ToOwned::to_owned)
    })
}

fn parse_stable_object_id(raw: &str) -> Option<ObjectId> {
    ObjectId::new(raw.to_owned()).ok()
}

fn remap_id(remap: &BTreeMap<ObjectId, ObjectId>, id: &ObjectId) -> ObjectId {
    remap.get(id).cloned().unwrap_or_else(|| id.clone())
}

fn allocate_reconciled_object_id(
    base_id: &ObjectId,
    assigned_ids: &BTreeSet<ObjectId>,
) -> ObjectId {
    let mut suffix = 1_u64;
    loop {
        let candidate = ObjectId::new(format!("{}:reconcile:{suffix:04}", base_id.as_str()))
            .expect("reconciled object id should be valid");
        if !assigned_ids.contains(&candidate) {
            return candidate;
        }
        suffix = suffix.saturating_add(1);
    }
}

pub(crate) fn reconcile_sequence_participants(ast: &mut SequenceAst, sidecar: &DiagramMeta) {
    if sidecar.stable_id_map.by_name.is_empty() {
        return;
    }

    let mut remap = BTreeMap::<ObjectId, ObjectId>::new();
    let mut next_participants = BTreeMap::<ObjectId, crate::model::SequenceParticipant>::new();
    let mut assigned_ids = BTreeSet::<ObjectId>::new();

    for (parsed_participant_id, participant) in ast.participants() {
        let mut mapped_id = sidecar
            .stable_id_map
            .by_name
            .get(participant.mermaid_name())
            .and_then(|raw| parse_stable_object_id(raw))
            .filter(|stable_id| !assigned_ids.contains(stable_id))
            .unwrap_or_else(|| parsed_participant_id.clone());

        if assigned_ids.contains(&mapped_id) {
            mapped_id = if !assigned_ids.contains(parsed_participant_id) {
                parsed_participant_id.clone()
            } else {
                allocate_reconciled_object_id(parsed_participant_id, &assigned_ids)
            };
        }

        assigned_ids.insert(mapped_id.clone());
        if &mapped_id != parsed_participant_id {
            remap.insert(parsed_participant_id.clone(), mapped_id.clone());
        }
        next_participants.insert(mapped_id, participant.clone());
    }

    if remap.is_empty() {
        return;
    }

    *ast.participants_mut() = next_participants;

    let next_messages = ast
        .messages()
        .iter()
        .map(|msg| {
            let mut updated = SequenceMessage::new(
                msg.message_id().clone(),
                remap_id(&remap, msg.from_participant_id()),
                remap_id(&remap, msg.to_participant_id()),
                msg.kind(),
                msg.text().to_owned(),
                msg.order_key(),
            );
            updated.set_raw_arrow(msg.raw_arrow().map(ToOwned::to_owned));
            updated
        })
        .collect();
    *ast.messages_mut() = next_messages;
}

pub(crate) fn reconcile_flowchart_nodes(ast: &mut FlowchartAst, sidecar: &DiagramMeta) {
    if sidecar.stable_id_map.by_mermaid_id.is_empty() {
        return;
    }

    let mut remap = BTreeMap::<ObjectId, ObjectId>::new();
    let mut next_nodes = BTreeMap::<ObjectId, crate::model::FlowNode>::new();
    let mut assigned_ids = BTreeSet::<ObjectId>::new();

    for (parsed_node_id, node) in ast.nodes() {
        let mut mapped_id = flow_node_mermaid_id(parsed_node_id, node.mermaid_id())
            .and_then(|mermaid_id| sidecar.stable_id_map.by_mermaid_id.get(&mermaid_id).cloned())
            .and_then(|raw| parse_stable_object_id(&raw))
            .filter(|stable_id| !assigned_ids.contains(stable_id))
            .unwrap_or_else(|| parsed_node_id.clone());

        if assigned_ids.contains(&mapped_id) {
            mapped_id = if !assigned_ids.contains(parsed_node_id) {
                parsed_node_id.clone()
            } else {
                allocate_reconciled_object_id(parsed_node_id, &assigned_ids)
            };
        }

        assigned_ids.insert(mapped_id.clone());
        if &mapped_id != parsed_node_id {
            remap.insert(parsed_node_id.clone(), mapped_id.clone());
        }
        next_nodes.insert(mapped_id, node.clone());
    }

    if remap.is_empty() {
        return;
    }

    *ast.nodes_mut() = next_nodes;

    let next_edges = ast
        .edges()
        .iter()
        .map(|(edge_id, edge)| {
            let mut updated = FlowEdge::new_with(
                remap_id(&remap, edge.from_node_id()),
                remap_id(&remap, edge.to_node_id()),
                edge.label().map(ToOwned::to_owned),
                edge.style().map(ToOwned::to_owned),
            );
            updated.set_connector(edge.connector().map(ToOwned::to_owned));
            (edge_id.clone(), updated)
        })
        .collect();
    *ast.edges_mut() = next_edges;

    if ast.node_groups().is_empty() {
        return;
    }

    let mut next_node_groups = BTreeMap::<ObjectId, ObjectId>::new();
    for (node_id, group_id) in ast.node_groups() {
        let mapped_node_id = remap_id(&remap, node_id);
        next_node_groups.entry(mapped_node_id).or_insert(group_id.clone());
    }
    *ast.node_groups_mut() = next_node_groups;
}

pub(crate) fn reconcile_flowchart_edges(ast: &mut FlowchartAst, sidecar: &DiagramMeta) {
    if sidecar.flow_edges.is_empty() {
        return;
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct FlowEdgeFingerprint {
        from_node_id: ObjectId,
        to_node_id: ObjectId,
        label: Option<String>,
    }

    fn numeric_edge_id(edge_id: &ObjectId) -> Option<u64> {
        let raw = edge_id.as_str().strip_prefix("e:")?;
        if raw.is_empty() || !raw.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        raw.parse().ok()
    }

    fn allocate_edge_id(next_numeric: &mut u64, taken_ids: &mut BTreeSet<ObjectId>) -> ObjectId {
        loop {
            let candidate = ObjectId::new(format!("e:{next_numeric:04}")).expect("valid edge id");
            *next_numeric = next_numeric.saturating_add(1);
            if taken_ids.insert(candidate.clone()) {
                return candidate;
            }
        }
    }

    let mut by_fingerprint: BTreeMap<FlowEdgeFingerprint, VecDeque<(ObjectId, Option<String>)>> =
        BTreeMap::new();
    let mut taken_ids: BTreeSet<ObjectId> = BTreeSet::new();

    for entry in &sidecar.flow_edges {
        taken_ids.insert(entry.edge_id.clone());
        let fingerprint = FlowEdgeFingerprint {
            from_node_id: entry.from_node_id.clone(),
            to_node_id: entry.to_node_id.clone(),
            label: entry.label.clone(),
        };
        by_fingerprint
            .entry(fingerprint)
            .or_default()
            .push_back((entry.edge_id.clone(), entry.style.clone()));
    }

    let mut max_numeric = 0u64;
    for edge_id in taken_ids.iter().chain(ast.edges().keys()) {
        if let Some(value) = numeric_edge_id(edge_id) {
            max_numeric = max_numeric.max(value);
        }
    }
    let mut next_numeric = max_numeric.saturating_add(1);

    let mut next_edges: BTreeMap<ObjectId, FlowEdge> = BTreeMap::new();
    for (parsed_edge_id, edge) in ast.edges() {
        let fingerprint = FlowEdgeFingerprint {
            from_node_id: edge.from_node_id().clone(),
            to_node_id: edge.to_node_id().clone(),
            label: edge.label().map(ToOwned::to_owned),
        };

        if let Some(queue) = by_fingerprint.get_mut(&fingerprint) {
            if let Some((stable_id, style)) = queue.pop_front() {
                let mut updated = edge.clone();
                updated.set_style(style.clone());

                let target_id = if next_edges.contains_key(&stable_id) {
                    allocate_edge_id(&mut next_numeric, &mut taken_ids)
                } else {
                    stable_id
                };

                taken_ids.insert(target_id.clone());
                next_edges.insert(target_id, updated);
                continue;
            }
        }

        let updated = edge.clone();
        let target_id =
            if taken_ids.contains(parsed_edge_id) || next_edges.contains_key(parsed_edge_id) {
                allocate_edge_id(&mut next_numeric, &mut taken_ids)
            } else {
                taken_ids.insert(parsed_edge_id.clone());
                parsed_edge_id.clone()
            };

        next_edges.insert(target_id, updated);
    }

    *ast.edges_mut() = next_edges;
}

pub(crate) fn reconcile_sequence_messages(ast: &mut SequenceAst, sidecar: &DiagramMeta) {
    if sidecar.sequence_messages.is_empty() {
        return;
    }

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct MessageFingerprint {
        from_participant_id: ObjectId,
        to_participant_id: ObjectId,
        kind: u8,
        text: String,
    }

    fn kind_key(kind: SequenceMessageKind) -> u8 {
        match kind {
            SequenceMessageKind::Sync => 0,
            SequenceMessageKind::Async => 1,
            SequenceMessageKind::Return => 2,
        }
    }

    fn numeric_message_id(message_id: &ObjectId) -> Option<u64> {
        let raw = message_id.as_str().strip_prefix("m:")?;
        if raw.is_empty() || !raw.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }
        raw.parse().ok()
    }

    fn allocate_message_id(next_numeric: &mut u64, taken_ids: &mut BTreeSet<ObjectId>) -> ObjectId {
        loop {
            let candidate =
                ObjectId::new(format!("m:{next_numeric:04}")).expect("valid message id");
            *next_numeric = next_numeric.saturating_add(1);
            if taken_ids.insert(candidate.clone()) {
                return candidate;
            }
        }
    }

    let mut by_fingerprint: BTreeMap<MessageFingerprint, VecDeque<ObjectId>> = BTreeMap::new();
    let mut taken_ids: BTreeSet<ObjectId> = BTreeSet::new();

    for entry in &sidecar.sequence_messages {
        taken_ids.insert(entry.message_id.clone());
        let fingerprint = MessageFingerprint {
            from_participant_id: entry.from_participant_id.clone(),
            to_participant_id: entry.to_participant_id.clone(),
            kind: kind_key(entry.kind),
            text: entry.text.clone(),
        };
        by_fingerprint.entry(fingerprint).or_default().push_back(entry.message_id.clone());
    }

    let mut max_numeric = 0u64;
    for message_id in taken_ids.iter().chain(ast.messages().iter().map(|msg| msg.message_id())) {
        if let Some(value) = numeric_message_id(message_id) {
            max_numeric = max_numeric.max(value);
        }
    }
    let mut next_numeric = max_numeric.saturating_add(1);

    let mut next_messages = Vec::with_capacity(ast.messages().len());
    let mut assigned_ids: BTreeSet<ObjectId> = BTreeSet::new();
    let mut remap: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();

    for msg in ast.messages() {
        let original_message_id = msg.message_id().clone();
        let fingerprint = MessageFingerprint {
            from_participant_id: msg.from_participant_id().clone(),
            to_participant_id: msg.to_participant_id().clone(),
            kind: kind_key(msg.kind()),
            text: msg.text().to_owned(),
        };

        let message_id = match by_fingerprint.get_mut(&fingerprint) {
            Some(queue) => match queue.pop_front() {
                Some(stable_id) => {
                    if assigned_ids.contains(&stable_id) {
                        allocate_message_id(&mut next_numeric, &mut taken_ids)
                    } else {
                        stable_id
                    }
                }
                None => {
                    if taken_ids.contains(msg.message_id())
                        || assigned_ids.contains(msg.message_id())
                    {
                        allocate_message_id(&mut next_numeric, &mut taken_ids)
                    } else {
                        taken_ids.insert(msg.message_id().clone());
                        msg.message_id().clone()
                    }
                }
            },
            None => {
                if taken_ids.contains(msg.message_id()) || assigned_ids.contains(msg.message_id()) {
                    allocate_message_id(&mut next_numeric, &mut taken_ids)
                } else {
                    taken_ids.insert(msg.message_id().clone());
                    msg.message_id().clone()
                }
            }
        };

        assigned_ids.insert(message_id.clone());
        remap.insert(original_message_id, message_id.clone());

        let mut updated = SequenceMessage::new(
            message_id,
            msg.from_participant_id().clone(),
            msg.to_participant_id().clone(),
            msg.kind(),
            msg.text().to_owned(),
            msg.order_key(),
        );
        updated.set_raw_arrow(msg.raw_arrow().map(ToOwned::to_owned));
        next_messages.push(updated);
    }

    *ast.messages_mut() = next_messages;
    remap_sequence_block_message_ids(ast.blocks_mut(), &remap);
}

fn remap_sequence_block_message_ids(
    blocks: &mut [crate::model::seq_ast::SequenceBlock],
    remap: &BTreeMap<ObjectId, ObjectId>,
) {
    if remap.is_empty() {
        return;
    }

    for block in blocks {
        for section in block.sections_mut() {
            for message_id in section.message_ids_mut() {
                if let Some(mapped) = remap.get(message_id) {
                    *message_id = mapped.clone();
                }
            }
        }

        remap_sequence_block_message_ids(block.blocks_mut(), remap);
    }
}

pub(crate) fn reconcile_flowchart_notes(ast: &mut FlowchartAst, sidecar: &DiagramMeta) {
    if sidecar.flow_node_notes.is_empty() {
        return;
    }

    for (node_id, note) in &sidecar.flow_node_notes {
        if let Some(node) = ast.nodes_mut().get_mut(node_id) {
            node.set_note(Some(note.clone()));
        }
    }
}

pub(crate) fn reconcile_flowchart_symbols(ast: &mut FlowchartAst, sidecar: &DiagramMeta) {
    if sidecar.flow_node_symbols.is_empty() {
        return;
    }

    for (node_id, symbol) in &sidecar.flow_node_symbols {
        if let Some(node) = ast.nodes_mut().get_mut(node_id) {
            node.set_symbol(Some(symbol.clone()));
        }
    }
}

pub(crate) fn reconcile_sequence_participant_notes(ast: &mut SequenceAst, sidecar: &DiagramMeta) {
    if sidecar.sequence_participant_notes.is_empty() {
        return;
    }

    for (participant_id, note) in &sidecar.sequence_participant_notes {
        if let Some(participant) = ast.participants_mut().get_mut(participant_id) {
            participant.set_note(Some(note.clone()));
        }
    }
}

pub(crate) fn reconcile_sequence_participant_symbols(ast: &mut SequenceAst, sidecar: &DiagramMeta) {
    if sidecar.sequence_participant_symbols.is_empty() {
        return;
    }

    for (participant_id, symbol) in &sidecar.sequence_participant_symbols {
        if let Some(participant) = ast.participants_mut().get_mut(participant_id) {
            participant.set_symbol(Some(symbol.clone()));
        }
    }
}

/// Apply all identity/sidecar reconciliations appropriate for `ast`.
pub(crate) fn reconcile_diagram_ast(ast: &mut DiagramAst, sidecar: &DiagramMeta) {
    match ast {
        DiagramAst::Flowchart(flow_ast) => {
            reconcile_flowchart_nodes(flow_ast, sidecar);
            reconcile_flowchart_edges(flow_ast, sidecar);
            reconcile_flowchart_notes(flow_ast, sidecar);
            reconcile_flowchart_symbols(flow_ast, sidecar);
        }
        DiagramAst::Sequence(seq_ast) => {
            reconcile_sequence_participants(seq_ast, sidecar);
            reconcile_sequence_messages(seq_ast, sidecar);
            reconcile_sequence_participant_notes(seq_ast, sidecar);
            reconcile_sequence_participant_symbols(seq_ast, sidecar);
        }
    }
}
