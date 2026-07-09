// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Sequence-diagram AST: participants, ordered messages, notes, and alt/par blocks.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::ids::ObjectId;
use super::symbol::SymbolAnchor;

/// Mutable sequence-diagram content: participants, ordered messages, notes, and blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SequenceAst {
    participants: BTreeMap<ObjectId, SequenceParticipant>,
    messages: Vec<SequenceMessage>,
    notes: Vec<SequenceNote>,
    blocks: Vec<SequenceBlock>,
}

impl SequenceAst {
    pub fn participants(&self) -> &BTreeMap<ObjectId, SequenceParticipant> {
        &self.participants
    }

    pub fn participants_mut(&mut self) -> &mut BTreeMap<ObjectId, SequenceParticipant> {
        &mut self.participants
    }

    pub fn messages(&self) -> &[SequenceMessage] {
        &self.messages
    }

    pub fn messages_mut(&mut self) -> &mut Vec<SequenceMessage> {
        &mut self.messages
    }

    /// Returns message references in deterministic `(order_key, message_id)` order.
    pub fn messages_in_order(&self) -> Vec<&SequenceMessage> {
        let mut messages = self.messages.iter().collect::<Vec<_>>();
        messages.sort_by(|a, b| SequenceMessage::cmp_in_order(a, b));
        messages
    }

    pub fn notes(&self) -> &[SequenceNote] {
        &self.notes
    }

    pub fn notes_mut(&mut self) -> &mut Vec<SequenceNote> {
        &mut self.notes
    }

    pub fn blocks(&self) -> &[SequenceBlock] {
        &self.blocks
    }

    pub fn blocks_mut(&mut self) -> &mut Vec<SequenceBlock> {
        &mut self.blocks
    }

    pub fn find_block(&self, block_id: &ObjectId) -> Option<&SequenceBlock> {
        fn find<'a>(blocks: &'a [SequenceBlock], block_id: &ObjectId) -> Option<&'a SequenceBlock> {
            for block in blocks {
                if block.block_id() == block_id {
                    return Some(block);
                }
                if let Some(found) = find(block.blocks(), block_id) {
                    return Some(found);
                }
            }
            None
        }

        find(&self.blocks, block_id)
    }

    pub fn find_block_mut(&mut self, block_id: &ObjectId) -> Option<&mut SequenceBlock> {
        fn find<'a>(
            blocks: &'a mut [SequenceBlock],
            block_id: &ObjectId,
        ) -> Option<&'a mut SequenceBlock> {
            for index in 0..blocks.len() {
                if blocks[index].block_id() == block_id {
                    return Some(&mut blocks[index]);
                }
            }
            for block in blocks.iter_mut() {
                if let Some(found) = find(block.blocks_mut(), block_id) {
                    return Some(found);
                }
            }
            None
        }

        find(&mut self.blocks, block_id)
    }

    pub fn find_section(&self, section_id: &ObjectId) -> Option<&SequenceSection> {
        fn find<'a>(
            blocks: &'a [SequenceBlock],
            section_id: &ObjectId,
        ) -> Option<&'a SequenceSection> {
            for block in blocks {
                for section in block.sections() {
                    if section.section_id() == section_id {
                        return Some(section);
                    }
                }
                if let Some(found) = find(block.blocks(), section_id) {
                    return Some(found);
                }
            }
            None
        }

        find(&self.blocks, section_id)
    }

    pub fn find_section_mut(&mut self, section_id: &ObjectId) -> Option<&mut SequenceSection> {
        fn find<'a>(
            blocks: &'a mut [SequenceBlock],
            section_id: &ObjectId,
        ) -> Option<&'a mut SequenceSection> {
            for block in blocks.iter_mut() {
                if let Some(index) =
                    block.sections().iter().position(|section| section.section_id() == section_id)
                {
                    return Some(&mut block.sections_mut()[index]);
                }
                if let Some(found) = find(block.blocks_mut(), section_id) {
                    return Some(found);
                }
            }
            None
        }

        find(&mut self.blocks, section_id)
    }

    /// Returns the block that owns `section_id`, if any.
    pub fn find_block_for_section(&self, section_id: &ObjectId) -> Option<&SequenceBlock> {
        fn find<'a>(
            blocks: &'a [SequenceBlock],
            section_id: &ObjectId,
        ) -> Option<&'a SequenceBlock> {
            for block in blocks {
                if block.sections().iter().any(|section| section.section_id() == section_id) {
                    return Some(block);
                }
                if let Some(found) = find(block.blocks(), section_id) {
                    return Some(found);
                }
            }
            None
        }

        find(&self.blocks, section_id)
    }

    pub fn find_block_for_section_mut(
        &mut self,
        section_id: &ObjectId,
    ) -> Option<&mut SequenceBlock> {
        fn find<'a>(
            blocks: &'a mut [SequenceBlock],
            section_id: &ObjectId,
        ) -> Option<&'a mut SequenceBlock> {
            for index in 0..blocks.len() {
                if blocks[index].sections().iter().any(|section| section.section_id() == section_id)
                {
                    return Some(&mut blocks[index]);
                }
            }
            for block in blocks.iter_mut() {
                if let Some(found) = find(block.blocks_mut(), section_id) {
                    return Some(found);
                }
            }
            None
        }

        find(&mut self.blocks, section_id)
    }

    /// True if any block in the tree uses `block_id`.
    pub fn contains_block_id(&self, block_id: &ObjectId) -> bool {
        self.find_block(block_id).is_some()
    }

    /// True if any section in the tree uses `section_id`.
    pub fn contains_section_id(&self, section_id: &ObjectId) -> bool {
        self.find_section(section_id).is_some()
    }

    /// Nesting depth of `block_id` (root blocks are depth 1). Returns `None` if missing.
    pub fn block_depth(&self, block_id: &ObjectId) -> Option<usize> {
        fn depth_of(blocks: &[SequenceBlock], block_id: &ObjectId, depth: usize) -> Option<usize> {
            for block in blocks {
                if block.block_id() == block_id {
                    return Some(depth);
                }
                if let Some(found) = depth_of(block.blocks(), block_id, depth + 1) {
                    return Some(found);
                }
            }
            None
        }

        depth_of(&self.blocks, block_id, 1)
    }

    /// Parent block id for a nested block, if any.
    pub fn parent_block_id(&self, block_id: &ObjectId) -> Option<ObjectId> {
        fn find(blocks: &[SequenceBlock], block_id: &ObjectId) -> Option<ObjectId> {
            for block in blocks {
                if block.blocks().iter().any(|child| child.block_id() == block_id) {
                    return Some(block.block_id().clone());
                }
                if let Some(found) = find(block.blocks(), block_id) {
                    return Some(found);
                }
            }
            None
        }

        find(&self.blocks, block_id)
    }

    /// Chain of block ids from root to `block_id` inclusive. Empty if missing.
    pub fn block_ancestor_chain(&self, block_id: &ObjectId) -> Vec<ObjectId> {
        fn find(blocks: &[SequenceBlock], block_id: &ObjectId, path: &mut Vec<ObjectId>) -> bool {
            for block in blocks {
                path.push(block.block_id().clone());
                if block.block_id() == block_id {
                    return true;
                }
                if find(block.blocks(), block_id, path) {
                    return true;
                }
                path.pop();
            }
            false
        }

        let mut path = Vec::new();
        if find(&self.blocks, block_id, &mut path) {
            path
        } else {
            Vec::new()
        }
    }

    /// All section ids that currently list `message_id`.
    pub fn sections_containing_message(&self, message_id: &ObjectId) -> Vec<ObjectId> {
        fn walk(blocks: &[SequenceBlock], message_id: &ObjectId, out: &mut Vec<ObjectId>) {
            for block in blocks {
                for section in block.sections() {
                    if section.message_ids().contains(message_id) {
                        out.push(section.section_id().clone());
                    }
                }
                walk(block.blocks(), message_id, out);
            }
        }

        let mut out = Vec::new();
        walk(&self.blocks, message_id, &mut out);
        out
    }

    /// Collect every message id referenced by `block` and its nested descendants.
    pub fn collect_block_message_ids(&self, block_id: &ObjectId) -> Vec<ObjectId> {
        let Some(block) = self.find_block(block_id) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(block: &SequenceBlock, out: &mut Vec<ObjectId>) {
            for section in block.sections() {
                for message_id in section.message_ids() {
                    if !out.contains(message_id) {
                        out.push(message_id.clone());
                    }
                }
            }
            for child in block.blocks() {
                walk(child, out);
            }
        }
        walk(block, &mut out);
        out
    }

    /// Removes the block (and nested children) from the tree. Messages are not removed.
    ///
    /// Returns the removed block when found.
    pub fn remove_block(&mut self, block_id: &ObjectId) -> Option<SequenceBlock> {
        fn remove_from(
            blocks: &mut Vec<SequenceBlock>,
            block_id: &ObjectId,
        ) -> Option<SequenceBlock> {
            if let Some(index) = blocks.iter().position(|block| block.block_id() == block_id) {
                return Some(blocks.remove(index));
            }
            for block in blocks.iter_mut() {
                if let Some(removed) = remove_from(block.blocks_mut(), block_id) {
                    return Some(removed);
                }
            }
            None
        }

        remove_from(&mut self.blocks, block_id)
    }

    /// Removes `message_id` from every section in the tree. Returns how many memberships were cleared.
    pub fn detach_message_from_all_sections(&mut self, message_id: &ObjectId) -> usize {
        fn detach(blocks: &mut [SequenceBlock], message_id: &ObjectId) -> usize {
            let mut removed = 0usize;
            for block in blocks {
                for section in block.sections_mut() {
                    let before = section.message_ids().len();
                    section.message_ids_mut().retain(|id| id != message_id);
                    removed =
                        removed.saturating_add(before.saturating_sub(section.message_ids().len()));
                }
                removed = removed.saturating_add(detach(block.blocks_mut(), message_id));
            }
            removed
        }

        detach(&mut self.blocks, message_id)
    }

    /// Collects every block id in pre-order (parents before nested children).
    pub fn collect_block_ids(&self) -> Vec<ObjectId> {
        fn walk(blocks: &[SequenceBlock], out: &mut Vec<ObjectId>) {
            for block in blocks {
                out.push(block.block_id().clone());
                walk(block.blocks(), out);
            }
        }

        let mut out = Vec::new();
        walk(&self.blocks, &mut out);
        out
    }

    /// Collects every section id in pre-order (block sections, then nested blocks).
    pub fn collect_section_ids(&self) -> Vec<ObjectId> {
        fn walk(blocks: &[SequenceBlock], out: &mut Vec<ObjectId>) {
            for block in blocks {
                for section in block.sections() {
                    out.push(section.section_id().clone());
                }
                walk(block.blocks(), out);
            }
        }

        let mut out = Vec::new();
        walk(&self.blocks, &mut out);
        out
    }
}

/// Sequence lifeline with Mermaid name, optional role alias, and note text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceParticipant {
    mermaid_name: String,
    role: Option<String>,
    note: Option<String>,
    symbol: Option<SymbolAnchor>,
}

impl SequenceParticipant {
    pub fn new(mermaid_name: impl Into<String>) -> Self {
        Self { mermaid_name: mermaid_name.into(), role: None, note: None, symbol: None }
    }

    pub fn set_mermaid_name(&mut self, mermaid_name: impl Into<String>) {
        self.mermaid_name = mermaid_name.into();
    }

    pub fn set_role<T: Into<String>>(&mut self, role: Option<T>) {
        self.role = role.map(Into::into);
    }

    pub fn set_note<T: Into<String>>(&mut self, note: Option<T>) {
        self.note = note.map(Into::into);
    }

    pub fn set_symbol(&mut self, symbol: Option<SymbolAnchor>) {
        self.symbol = symbol;
    }

    pub fn mermaid_name(&self) -> &str {
        &self.mermaid_name
    }

    pub fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn symbol(&self) -> Option<&SymbolAnchor> {
        self.symbol.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceBlockKind {
    Alt,
    Opt,
    Loop,
    Par,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceBlock {
    block_id: ObjectId,
    kind: SequenceBlockKind,
    header: Option<String>,
    sections: Vec<SequenceSection>,
    blocks: Vec<SequenceBlock>,
}

impl SequenceBlock {
    pub fn make_block_id(block_index: usize) -> ObjectId {
        ObjectId::new(format!("b:{block_index:04}")).expect("valid block id")
    }

    pub fn new(
        block_id: ObjectId,
        kind: SequenceBlockKind,
        header: Option<String>,
        sections: Vec<SequenceSection>,
        blocks: Vec<SequenceBlock>,
    ) -> Self {
        Self { block_id, kind, header, sections, blocks }
    }

    pub fn block_id(&self) -> &ObjectId {
        &self.block_id
    }

    pub fn kind(&self) -> SequenceBlockKind {
        self.kind
    }

    pub fn header(&self) -> Option<&str> {
        self.header.as_deref()
    }

    pub fn set_header<T: Into<String>>(&mut self, header: Option<T>) {
        self.header = header.map(Into::into);
    }

    pub fn sections(&self) -> &[SequenceSection] {
        &self.sections
    }

    pub fn sections_mut(&mut self) -> &mut Vec<SequenceSection> {
        &mut self.sections
    }

    pub fn blocks(&self) -> &[SequenceBlock] {
        &self.blocks
    }

    pub fn blocks_mut(&mut self) -> &mut Vec<SequenceBlock> {
        &mut self.blocks
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceSectionKind {
    Main,
    Else,
    And,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceSection {
    section_id: ObjectId,
    kind: SequenceSectionKind,
    header: Option<String>,
    message_ids: Vec<ObjectId>,
}

impl SequenceSection {
    pub fn make_section_id(block_index: usize, section_index: usize) -> ObjectId {
        ObjectId::new(format!("sec:{block_index:04}:{section_index:02}")).expect("valid section id")
    }

    pub fn new(
        section_id: ObjectId,
        kind: SequenceSectionKind,
        header: Option<String>,
        message_ids: Vec<ObjectId>,
    ) -> Self {
        Self { section_id, kind, header, message_ids }
    }

    pub fn section_id(&self) -> &ObjectId {
        &self.section_id
    }

    pub fn kind(&self) -> SequenceSectionKind {
        self.kind
    }

    pub fn header(&self) -> Option<&str> {
        self.header.as_deref()
    }

    pub fn set_header<T: Into<String>>(&mut self, header: Option<T>) {
        self.header = header.map(Into::into);
    }

    pub fn message_ids(&self) -> &[ObjectId] {
        &self.message_ids
    }

    pub fn message_ids_mut(&mut self) -> &mut Vec<ObjectId> {
        &mut self.message_ids
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SequenceMessageKind {
    Sync,
    Async,
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceMessage {
    message_id: ObjectId,
    from_participant_id: ObjectId,
    to_participant_id: ObjectId,
    kind: SequenceMessageKind,
    raw_arrow: Option<String>,
    text: String,
    order_key: i64,
}

impl SequenceMessage {
    pub fn new(
        message_id: ObjectId,
        from_participant_id: ObjectId,
        to_participant_id: ObjectId,
        kind: SequenceMessageKind,
        text: impl Into<String>,
        order_key: i64,
    ) -> Self {
        Self {
            message_id,
            from_participant_id,
            to_participant_id,
            kind,
            raw_arrow: None,
            text: text.into(),
            order_key,
        }
    }

    pub fn set_raw_arrow<T: Into<String>>(&mut self, raw_arrow: Option<T>) {
        self.raw_arrow = raw_arrow.map(Into::into);
    }

    pub fn message_id(&self) -> &ObjectId {
        &self.message_id
    }

    pub fn from_participant_id(&self) -> &ObjectId {
        &self.from_participant_id
    }

    pub fn to_participant_id(&self) -> &ObjectId {
        &self.to_participant_id
    }

    pub fn kind(&self) -> SequenceMessageKind {
        self.kind
    }

    pub fn raw_arrow(&self) -> Option<&str> {
        self.raw_arrow.as_deref()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn order_key(&self) -> i64 {
        self.order_key
    }

    pub fn cmp_in_order(a: &Self, b: &Self) -> Ordering {
        a.order_key.cmp(&b.order_key).then_with(|| a.message_id.cmp(&b.message_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceNote {
    note_id: ObjectId,
    text: String,
}

impl SequenceNote {
    pub fn new(note_id: ObjectId, text: impl Into<String>) -> Self {
        Self { note_id, text: text.into() }
    }

    pub fn note_id(&self) -> &ObjectId {
        &self.note_id
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SequenceAst, SequenceBlock, SequenceBlockKind, SequenceParticipant, SequenceSection,
        SequenceSectionKind,
    };
    use crate::model::ids::ObjectId;

    #[test]
    fn sequence_participant_can_be_updated_in_place() {
        let mut participant = SequenceParticipant::new("Alice");
        assert_eq!(participant.mermaid_name(), "Alice");
        assert_eq!(participant.role(), None);
        assert_eq!(participant.note(), None);

        participant.set_role(Some("actor"));
        assert_eq!(participant.role(), Some("actor"));

        participant.set_mermaid_name("Alice2");
        assert_eq!(participant.mermaid_name(), "Alice2");
        assert_eq!(participant.role(), Some("actor"));

        participant.set_note(Some("invariant"));
        assert_eq!(participant.note(), Some("invariant"));

        participant.set_role::<&str>(None);
        assert_eq!(participant.role(), None);

        participant.set_note::<&str>(None);
        assert_eq!(participant.note(), None);
    }

    #[test]
    fn sequence_block_and_section_ids_are_allocated_deterministically() {
        assert_eq!(SequenceBlock::make_block_id(1).as_str(), "b:0001");
        assert_eq!(SequenceSection::make_section_id(1, 0).as_str(), "sec:0001:00");
    }

    fn oid(raw: &str) -> ObjectId {
        ObjectId::new(raw.to_owned()).expect("valid object id")
    }

    fn sample_nested_ast() -> SequenceAst {
        let mut ast = SequenceAst::default();
        let nested = SequenceBlock::new(
            oid("b:nested"),
            SequenceBlockKind::Opt,
            Some("warm".to_owned()),
            vec![SequenceSection::new(
                oid("sec:nested:main"),
                SequenceSectionKind::Main,
                None,
                vec![oid("m:2")],
            )],
            Vec::new(),
        );
        let root = SequenceBlock::new(
            oid("b:root"),
            SequenceBlockKind::Alt,
            Some("hit".to_owned()),
            vec![
                SequenceSection::new(
                    oid("sec:root:main"),
                    SequenceSectionKind::Main,
                    None,
                    vec![oid("m:1"), oid("m:2")],
                ),
                SequenceSection::new(
                    oid("sec:root:else"),
                    SequenceSectionKind::Else,
                    Some("miss".to_owned()),
                    vec![oid("m:3")],
                ),
            ],
            vec![nested],
        );
        ast.blocks_mut().push(root);
        ast
    }

    #[test]
    fn find_block_mut_and_find_section_mut_update_nested_tree() {
        let mut ast = sample_nested_ast();

        ast.find_block_mut(&oid("b:nested")).expect("nested block").set_header(Some("warm-cache"));
        assert_eq!(ast.find_block(&oid("b:nested")).expect("nested").header(), Some("warm-cache"));

        ast.find_section_mut(&oid("sec:root:else"))
            .expect("else section")
            .set_header(Some("cache-miss"));
        assert_eq!(
            ast.find_section(&oid("sec:root:else")).expect("else").header(),
            Some("cache-miss")
        );

        assert_eq!(
            ast.find_block_for_section(&oid("sec:root:else")).expect("owner").block_id().as_str(),
            "b:root"
        );
        assert_eq!(ast.block_depth(&oid("b:root")), Some(1));
        assert_eq!(ast.block_depth(&oid("b:nested")), Some(2));
        assert!(ast.contains_block_id(&oid("b:nested")));
        assert!(ast.contains_section_id(&oid("sec:nested:main")));
    }

    #[test]
    fn detach_message_from_all_sections_clears_memberships() {
        let mut ast = sample_nested_ast();
        let removed = ast.detach_message_from_all_sections(&oid("m:2"));
        assert_eq!(removed, 2);
        assert!(!ast
            .find_section(&oid("sec:root:main"))
            .expect("main")
            .message_ids()
            .contains(&oid("m:2")));
        assert!(!ast
            .find_section(&oid("sec:nested:main"))
            .expect("nested main")
            .message_ids()
            .contains(&oid("m:2")));
        assert!(ast
            .find_section(&oid("sec:root:else"))
            .expect("else")
            .message_ids()
            .contains(&oid("m:3")));
    }

    #[test]
    fn remove_block_keeps_sibling_structure_and_messages_lists() {
        let mut ast = sample_nested_ast();
        let removed = ast.remove_block(&oid("b:nested")).expect("removed nested");
        assert_eq!(removed.block_id().as_str(), "b:nested");
        assert!(!ast.contains_block_id(&oid("b:nested")));
        assert!(ast.contains_block_id(&oid("b:root")));
        assert_eq!(
            ast.find_section(&oid("sec:root:main")).expect("main").message_ids(),
            &[oid("m:1"), oid("m:2")]
        );
        assert_eq!(
            ast.collect_block_ids().iter().map(ObjectId::as_str).collect::<Vec<_>>(),
            vec!["b:root"]
        );
        assert_eq!(
            ast.collect_section_ids().iter().map(ObjectId::as_str).collect::<Vec<_>>(),
            vec!["sec:root:main", "sec:root:else"]
        );
    }
}
