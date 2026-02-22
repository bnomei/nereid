// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::ids::ObjectId;

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceParticipant {
    mermaid_name: String,
    role: Option<String>,
    note: Option<String>,
}

impl SequenceParticipant {
    pub fn new(mermaid_name: impl Into<String>) -> Self {
        Self { mermaid_name: mermaid_name.into(), role: None, note: None }
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

    pub fn mermaid_name(&self) -> &str {
        &self.mermaid_name
    }

    pub fn role(&self) -> Option<&str> {
        self.role.as_deref()
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
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
    use super::{SequenceBlock, SequenceParticipant, SequenceSection};

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
}
