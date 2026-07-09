// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

// Sequence and flowchart op application: validation, AST mutation, and delta recording.

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
            validate_seq_participant_mermaid_name(ast, None, mermaid_name)?;
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
            if !ast.participants().contains_key(participant_id) {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: participant_id.clone(),
                });
            }

            if let Some(mermaid_name) = &patch.mermaid_name {
                validate_seq_participant_mermaid_name(ast, Some(participant_id), mermaid_name)?;
                let existing = ast
                    .participants_mut()
                    .get_mut(participant_id)
                    .expect("participant existence checked above");
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
        SeqOp::SetParticipantSymbol {
            participant_id,
            symbol,
        } => {
            let Some(existing) = ast.participants_mut().get_mut(participant_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqParticipant,
                    object_id: participant_id.clone(),
                });
            };

            existing.set_symbol(symbol.clone());
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
            prune_messages_from_blocks(ast, &removed_message_ids.iter().cloned().collect());
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
            section_id,
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
            if let Some(section_id) = section_id {
                if !ast.contains_section_id(section_id) {
                    return Err(ApplyError::NotFound {
                        kind: ObjectKind::SeqSection,
                        object_id: section_id.clone(),
                    });
                }
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
            if let Some(section_id) = section_id {
                attach_message_to_section(diagram_id, ast, message_id, section_id, delta)?;
            }
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
            let structure_before = structure_snapshot(ast);
            prune_messages_from_blocks(ast, &HashSet::from([message_id.clone()]));
            record_structure_prune_delta(diagram_id, &structure_before, ast, delta);
            delta.record_removed(seq_message_ref(diagram_id, message_id));
            Ok(())
        }
        SeqOp::SetMessageSection {
            message_id,
            section_id,
        } => {
            if !ast.messages().iter().any(|m| m.message_id() == message_id) {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqMessage,
                    object_id: message_id.clone(),
                });
            }
            match section_id {
                Some(section_id) => {
                    if !ast.contains_section_id(section_id) {
                        return Err(ApplyError::NotFound {
                            kind: ObjectKind::SeqSection,
                            object_id: section_id.clone(),
                        });
                    }
                    attach_message_to_section(diagram_id, ast, message_id, section_id, delta)?;
                }
                None => {
                    let previous_sections = ast.sections_containing_message(message_id);
                    let detached = ast.detach_message_from_all_sections(message_id);
                    if detached > 0 {
                        delta.record_updated(seq_message_ref(diagram_id, message_id));
                        for previous_section_id in previous_sections {
                            delta.record_updated(seq_section_ref(diagram_id, &previous_section_id));
                            if let Some(block) = ast.find_block_for_section(&previous_section_id) {
                                delta.record_updated(seq_block_ref(diagram_id, block.block_id()));
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        SeqOp::AddBlock {
            block_id,
            kind,
            header,
            parent_block_id,
            main_section_id,
        } => {
            if ast.contains_block_id(block_id) {
                return Err(ApplyError::AlreadyExists {
                    kind: ObjectKind::SeqBlock,
                    object_id: block_id.clone(),
                });
            }
            if ast.contains_section_id(main_section_id) {
                return Err(ApplyError::AlreadyExists {
                    kind: ObjectKind::SeqSection,
                    object_id: main_section_id.clone(),
                });
            }

            if let Some(parent_id) = parent_block_id {
                let Some(depth) = ast.block_depth(parent_id) else {
                    return Err(ApplyError::NotFound {
                        kind: ObjectKind::SeqBlock,
                        object_id: parent_id.clone(),
                    });
                };
                if depth >= MAX_SEQ_BLOCK_NEST_DEPTH {
                    return Err(ApplyError::InvalidSeqBlockNesting {
                        block_id: block_id.clone(),
                        reason: format!(
                            "parent nest depth {depth} already at max {MAX_SEQ_BLOCK_NEST_DEPTH}"
                        ),
                    });
                }
                if ast.find_block(parent_id).is_some_and(|parent| parent.sections().is_empty()) {
                    return Err(ApplyError::InvalidSeqBlockNesting {
                        block_id: block_id.clone(),
                        reason: format!(
                            "parent block {} has no sections; nested messages must also join an ancestor section (attach via section_id on AddMessage or SetMessageSection)",
                            parent_id.as_str()
                        ),
                    });
                }
            }

            let block = SequenceBlock::new(
                block_id.clone(),
                *kind,
                header.clone(),
                vec![SequenceSection::new(
                    main_section_id.clone(),
                    SequenceSectionKind::Main,
                    None,
                    Vec::new(),
                )],
                Vec::new(),
            );

            if let Some(parent_id) = parent_block_id {
                let parent = ast.find_block_mut(parent_id).expect("parent existence checked");
                parent.blocks_mut().push(block);
            } else {
                ast.blocks_mut().push(block);
            }

            delta.record_added(seq_block_ref(diagram_id, block_id));
            delta.record_added(seq_section_ref(diagram_id, main_section_id));
            Ok(())
        }
        SeqOp::UpdateBlock { block_id, patch } => {
            let Some(block) = ast.find_block_mut(block_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqBlock,
                    object_id: block_id.clone(),
                });
            };
            if let Some(header) = &patch.header {
                block.set_header(header.clone());
            }
            delta.record_updated(seq_block_ref(diagram_id, block_id));
            Ok(())
        }
        SeqOp::RemoveBlock { block_id } => {
            let Some(removed) = ast.remove_block(block_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqBlock,
                    object_id: block_id.clone(),
                });
            };
            record_removed_block_tree(diagram_id, &removed, delta);
            Ok(())
        }
        SeqOp::AddSection {
            section_id,
            block_id,
            kind,
            header,
        } => {
            if ast.contains_section_id(section_id) {
                return Err(ApplyError::AlreadyExists {
                    kind: ObjectKind::SeqSection,
                    object_id: section_id.clone(),
                });
            }
            let Some(block) = ast.find_block(block_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqBlock,
                    object_id: block_id.clone(),
                });
            };
            validate_section_kind_for_block(block.kind(), *kind, block_id)?;
            if *kind == SequenceSectionKind::Main {
                return Err(ApplyError::InvalidSeqSectionKind {
                    block_id: block_id.clone(),
                    block_kind: block.kind(),
                    section_kind: *kind,
                });
            }

            let block = ast.find_block_mut(block_id).expect("block existence checked");
            block.sections_mut().push(SequenceSection::new(
                section_id.clone(),
                *kind,
                header.clone(),
                Vec::new(),
            ));
            delta.record_added(seq_section_ref(diagram_id, section_id));
            delta.record_updated(seq_block_ref(diagram_id, block_id));
            Ok(())
        }
        SeqOp::UpdateSection { section_id, patch } => {
            let Some(section) = ast.find_section_mut(section_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqSection,
                    object_id: section_id.clone(),
                });
            };
            if let Some(header) = &patch.header {
                section.set_header(header.clone());
            }
            delta.record_updated(seq_section_ref(diagram_id, section_id));
            Ok(())
        }
        SeqOp::RemoveSection { section_id } => {
            let Some(block) = ast.find_block_for_section(section_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::SeqSection,
                    object_id: section_id.clone(),
                });
            };
            let block_id = block.block_id().clone();
            let section = block
                .sections()
                .iter()
                .find(|section| section.section_id() == section_id)
                .expect("section ownership checked");
            if section.kind() == SequenceSectionKind::Main {
                return Err(ApplyError::InvalidSeqSectionIsMain {
                    section_id: section_id.clone(),
                });
            }
            if !section.message_ids().is_empty() {
                return Err(ApplyError::InvalidSeqSectionNotEmpty {
                    section_id: section_id.clone(),
                });
            }

            let block = ast.find_block_mut(&block_id).expect("block ownership checked");
            block.sections_mut().retain(|section| section.section_id() != section_id);
            delta.record_removed(seq_section_ref(diagram_id, section_id));
            delta.record_updated(seq_block_ref(diagram_id, &block_id));
            Ok(())
        }
    }
}

fn attach_message_to_section(
    diagram_id: &DiagramId,
    ast: &mut SequenceAst,
    message_id: &ObjectId,
    section_id: &ObjectId,
    delta: &mut DeltaBuilder,
) -> Result<(), ApplyError> {
    // Mirror Mermaid parse: a message inside nested blocks is a member of every open
    // ancestor section so export ranges remain nested and contiguous.
    let Some(owner_block) = ast.find_block_for_section(section_id) else {
        return Err(ApplyError::NotFound {
            kind: ObjectKind::SeqSection,
            object_id: section_id.clone(),
        });
    };
    let owner_block_id = owner_block.block_id().clone();
    let ancestor_chain = ast.block_ancestor_chain(&owner_block_id);
    if ancestor_chain.is_empty() {
        return Err(ApplyError::NotFound {
            kind: ObjectKind::SeqBlock,
            object_id: owner_block_id,
        });
    }

    // Target leaf section, then one section per ancestor parent (outer → inner).
    let mut membership_sections = vec![section_id.clone()];
    for window in ancestor_chain.windows(2) {
        let parent_id = &window[0];
        let child_id = &window[1];
        let parent_section_id = select_parent_section_for_nested_child(ast, parent_id, child_id)?;
        membership_sections.push(parent_section_id);
    }
    membership_sections.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    membership_sections.dedup();

    let previous_sections = ast.sections_containing_message(message_id);
    ast.detach_message_from_all_sections(message_id);
    for previous_section_id in &previous_sections {
        if !membership_sections.contains(previous_section_id) {
            delta.record_updated(seq_section_ref(diagram_id, previous_section_id));
            if let Some(block) = ast.find_block_for_section(previous_section_id) {
                delta.record_updated(seq_block_ref(diagram_id, block.block_id()));
            }
        }
    }

    for membership_section_id in &membership_sections {
        let Some(section) = ast.find_section_mut(membership_section_id) else {
            return Err(ApplyError::NotFound {
                kind: ObjectKind::SeqSection,
                object_id: membership_section_id.clone(),
            });
        };
        if !section.message_ids().contains(message_id) {
            section.message_ids_mut().push(message_id.clone());
        }
        delta.record_updated(seq_section_ref(diagram_id, membership_section_id));
        if let Some(block) = ast.find_block_for_section(membership_section_id) {
            delta.record_updated(seq_block_ref(diagram_id, block.block_id()));
        }
    }

    delta.record_updated(seq_message_ref(diagram_id, message_id));
    Ok(())
}

/// Choose which parent section should contain a nested child block's messages.
///
/// Prefer a parent section that already lists messages belonging to the child tree;
/// otherwise fall back to the main section, then the first section.
fn select_parent_section_for_nested_child(
    ast: &SequenceAst,
    parent_block_id: &ObjectId,
    child_block_id: &ObjectId,
) -> Result<ObjectId, ApplyError> {
    let Some(parent) = ast.find_block(parent_block_id) else {
        return Err(ApplyError::NotFound {
            kind: ObjectKind::SeqBlock,
            object_id: parent_block_id.clone(),
        });
    };
    if parent.sections().is_empty() {
        return Err(ApplyError::InvalidSeqBlockNesting {
            block_id: child_block_id.clone(),
            reason: format!(
                "parent block {} has no sections to host nested block {}",
                parent_block_id.as_str(),
                child_block_id.as_str()
            ),
        });
    }

    let child_message_ids = ast.collect_block_message_ids(child_block_id);
    if !child_message_ids.is_empty() {
        for section in parent.sections() {
            if section
                .message_ids()
                .iter()
                .any(|message_id| child_message_ids.contains(message_id))
            {
                return Ok(section.section_id().clone());
            }
        }
    }

    if let Some(main) = parent
        .sections()
        .iter()
        .find(|section| section.kind() == SequenceSectionKind::Main)
    {
        return Ok(main.section_id().clone());
    }

    Ok(parent.sections()[0].section_id().clone())
}

fn validate_section_kind_for_block(
    block_kind: SequenceBlockKind,
    section_kind: SequenceSectionKind,
    block_id: &ObjectId,
) -> Result<(), ApplyError> {
    let ok = match (block_kind, section_kind) {
        (_, SequenceSectionKind::Main) => true,
        (SequenceBlockKind::Alt, SequenceSectionKind::Else) => true,
        (SequenceBlockKind::Par, SequenceSectionKind::And) => true,
        (SequenceBlockKind::Opt | SequenceBlockKind::Loop, SequenceSectionKind::Else | SequenceSectionKind::And) => {
            false
        }
        (SequenceBlockKind::Alt, SequenceSectionKind::And) => false,
        (SequenceBlockKind::Par, SequenceSectionKind::Else) => false,
    };
    if ok {
        Ok(())
    } else {
        Err(ApplyError::InvalidSeqSectionKind {
            block_id: block_id.clone(),
            block_kind,
            section_kind,
        })
    }
}

fn record_removed_block_tree(
    diagram_id: &DiagramId,
    block: &SequenceBlock,
    delta: &mut DeltaBuilder,
) {
    for section in block.sections() {
        delta.record_removed(seq_section_ref(diagram_id, section.section_id()));
    }
    for nested in block.blocks() {
        record_removed_block_tree(diagram_id, nested, delta);
    }
    delta.record_removed(seq_block_ref(diagram_id, block.block_id()));
}

fn validate_sequence_block_structure(ast: &SequenceAst) -> Result<(), ApplyError> {
    if ast.blocks().is_empty() {
        return Ok(());
    }
    crate::format::mermaid::export_sequence_diagram(ast).map_err(|err| {
        ApplyError::InvalidSeqBlockStructure {
            reason: err.to_string(),
        }
    })?;
    Ok(())
}

fn sort_seq_messages(ast: &mut SequenceAst) {
    ast.messages_mut().sort_by(SequenceMessage::cmp_in_order);
}

fn prune_messages_from_blocks(ast: &mut SequenceAst, removed: &HashSet<ObjectId>) {
    if removed.is_empty() {
        return;
    }
    ast.blocks_mut().retain_mut(|block| prune_messages_from_block(block, removed));
}

#[derive(Default)]
struct StructureSnapshot {
    block_ids: HashSet<ObjectId>,
    section_ids: HashSet<ObjectId>,
}

fn structure_snapshot(ast: &SequenceAst) -> StructureSnapshot {
    StructureSnapshot {
        block_ids: ast.collect_block_ids().into_iter().collect(),
        section_ids: ast.collect_section_ids().into_iter().collect(),
    }
}

fn record_structure_prune_delta(
    diagram_id: &DiagramId,
    before: &StructureSnapshot,
    after: &SequenceAst,
    delta: &mut DeltaBuilder,
) {
    let after_blocks: HashSet<ObjectId> = after.collect_block_ids().into_iter().collect();
    let after_sections: HashSet<ObjectId> = after.collect_section_ids().into_iter().collect();

    for block_id in &before.block_ids {
        if !after_blocks.contains(block_id) {
            delta.record_removed(seq_block_ref(diagram_id, block_id));
        }
    }
    for section_id in &before.section_ids {
        if !after_sections.contains(section_id) {
            delta.record_removed(seq_section_ref(diagram_id, section_id));
        } else {
            // Surviving sections may have lost membership; treat as updated when prune ran.
            delta.record_updated(seq_section_ref(diagram_id, section_id));
        }
    }
    for block_id in &after_blocks {
        if before.block_ids.contains(block_id) {
            delta.record_updated(seq_block_ref(diagram_id, block_id));
        }
    }
}

fn prune_messages_from_block(block: &mut SequenceBlock, removed: &HashSet<ObjectId>) -> bool {
    block.blocks_mut().retain_mut(|nested| prune_messages_from_block(nested, removed));

    let block_id = block.block_id().clone();
    let kind = block.kind();
    let mut header = block.header().map(ToOwned::to_owned);
    let mut sections = std::mem::take(block.sections_mut());
    for section in sections.iter_mut() {
        section.message_ids_mut().retain(|message_id| !removed.contains(message_id));
    }
    sections.retain(|section| !section.message_ids().is_empty());

    if sections.first().is_some_and(|first| first.kind() != SequenceSectionKind::Main) {
        let first = &sections[0];
        header = first.header().map(ToOwned::to_owned);
        let section_id = first.section_id().clone();
        let message_ids = first.message_ids().to_vec();
        sections[0] = SequenceSection::new(
            section_id,
            SequenceSectionKind::Main,
            None::<String>,
            message_ids,
        );
    } else if !block.blocks().is_empty() {
        let mut nested_message_ids = Vec::new();
        collect_nested_block_message_ids(block.blocks(), &mut nested_message_ids);
        if !nested_message_ids.is_empty() {
            sections.push(SequenceSection::new(
                ObjectId::new(format!("{}:main", block_id.as_str())).expect("valid section id"),
                SequenceSectionKind::Main,
                None::<String>,
                nested_message_ids,
            ));
        }
    }

    let blocks = std::mem::take(block.blocks_mut());
    let keep = !sections.is_empty() || !blocks.is_empty();
    *block = SequenceBlock::new(block_id, kind, header, sections, blocks);
    keep
}

fn collect_nested_block_message_ids(blocks: &[SequenceBlock], message_ids: &mut Vec<ObjectId>) {
    for block in blocks {
        for section in block.sections() {
            for message_id in section.message_ids() {
                if !message_ids.contains(message_id) {
                    message_ids.push(message_id.clone());
                }
            }
        }
        collect_nested_block_message_ids(block.blocks(), message_ids);
    }
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

fn validate_mermaid_ident_for_ops(ident: &str) -> Result<(), MermaidIdentError> {
    if ident.is_empty() {
        return Err(MermaidIdentError::Empty);
    }
    if ident.chars().any(|ch| ch.is_whitespace()) {
        return Err(MermaidIdentError::ContainsWhitespace);
    }
    if ident.contains('/') {
        return Err(MermaidIdentError::ContainsSlash);
    }
    if let Some(ch) = ident
        .chars()
        .find(|ch| !ch.is_ascii_alphanumeric() && *ch != '_')
    {
        return Err(MermaidIdentError::InvalidChar { ch });
    }
    Ok(())
}

fn validate_seq_participant_mermaid_name(
    ast: &SequenceAst,
    participant_id: Option<&ObjectId>,
    mermaid_name: &str,
) -> Result<(), ApplyError> {
    validate_mermaid_ident_for_ops(mermaid_name).map_err(|reason| {
        ApplyError::InvalidSeqParticipantMermaidName {
            mermaid_name: mermaid_name.to_owned(),
            reason,
        }
    })?;

    if let Some(other_participant_id) =
        ast.participants().iter().find_map(|(candidate_id, candidate)| {
            if participant_id.is_some_and(|current| current == candidate_id) {
                return None;
            }

            (candidate.mermaid_name() == mermaid_name).then(|| candidate_id.clone())
        })
    {
        return Err(ApplyError::DuplicateSeqParticipantMermaidName {
            mermaid_name: mermaid_name.to_owned(),
            participant_id: other_participant_id,
        });
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

fn validate_flow_node_mermaid_identity(
    ast: &FlowchartAst,
    node_id: &ObjectId,
    node: &FlowNode,
    current_node_id: Option<&ObjectId>,
) -> Result<(), ApplyError> {
    let mermaid_id = flow_node_mermaid_id_for_uniqueness(node_id, node).ok_or_else(|| {
        ApplyError::InvalidFlowNodeMermaidId {
            mermaid_id: node_id.to_string(),
            reason: MermaidIdentError::Empty,
        }
    })?;

    validate_mermaid_ident_for_ops(mermaid_id).map_err(|reason| {
        ApplyError::InvalidFlowNodeMermaidId {
            mermaid_id: mermaid_id.to_owned(),
            reason,
        }
    })?;

    if let Some(other_node_id) = ast.nodes().iter().find_map(|(candidate_id, candidate)| {
        if current_node_id.is_some_and(|current| current == candidate_id) {
            return None;
        }

        let candidate_mermaid_id = flow_node_mermaid_id_for_uniqueness(candidate_id, candidate)?;
        (candidate_mermaid_id == mermaid_id).then(|| candidate_id.clone())
    }) {
        return Err(ApplyError::DuplicateFlowNodeMermaidId {
            mermaid_id: mermaid_id.to_owned(),
            node_id: other_node_id,
        });
    }

    Ok(())
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
            validate_flow_node_mermaid_identity(ast, node_id, &node, None)?;
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
            let Some(existing) = ast.nodes().get(node_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::FlowNode,
                    object_id: node_id.clone(),
                });
            };

            let mut candidate = existing.clone();
            candidate.set_mermaid_id(mermaid_id.clone());
            validate_flow_node_mermaid_identity(ast, node_id, &candidate, Some(node_id))?;

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
        FlowOp::SetNodeSymbol { node_id, symbol } => {
            let Some(existing) = ast.nodes_mut().get_mut(node_id) else {
                return Err(ApplyError::NotFound {
                    kind: ObjectKind::FlowNode,
                    object_id: node_id.clone(),
                });
            };

            existing.set_symbol(symbol.clone());
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

fn seq_block_ref(diagram_id: &DiagramId, block_id: &ObjectId) -> ObjectRef {
    object_ref(diagram_id, &["seq", "block"], block_id)
}

fn seq_section_ref(diagram_id: &DiagramId, section_id: &ObjectId) -> ObjectRef {
    object_ref(diagram_id, &["seq", "section"], section_id)
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
