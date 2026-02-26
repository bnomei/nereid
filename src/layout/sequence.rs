// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::model::ids::ObjectId;
use crate::model::seq_ast::{SequenceAst, SequenceBlock, SequenceMessage, SequenceSection};

const BASE_MESSAGE_LABEL_CAPACITY_PER_SPAN: usize = 16;
const BASE_PARTICIPANT_LABEL_CAPACITY: usize = 10;
const BASE_BLOCK_HEADER_LABEL_CAPACITY: usize = 14;
const BASE_SECTION_HEADER_LABEL_CAPACITY: usize = 12;
const BASE_SELF_LOOP_STUB_LEN: usize = 8;
const MAX_SELF_LOOP_STUB_LEN: usize = 32;
const SELF_LOOP_CORNER_RESERVE: usize = 2;
const PRESSURE_PER_EXTRA_ROW: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceLayout {
    participant_cols: BTreeMap<ObjectId, usize>,
    messages: Vec<SequenceMessageLayout>,
    spacing_budget: SequenceSpacingBudget,
}

impl SequenceLayout {
    pub fn participant_cols(&self) -> &BTreeMap<ObjectId, usize> {
        &self.participant_cols
    }

    pub fn messages(&self) -> &[SequenceMessageLayout] {
        &self.messages
    }

    pub fn spacing_budget(&self) -> &SequenceSpacingBudget {
        &self.spacing_budget
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SequenceSpacingBudget {
    participant_label_pressure_by_id: BTreeMap<ObjectId, usize>,
    message_span_pressure_by_id: BTreeMap<ObjectId, usize>,
    block_header_pressure_by_id: BTreeMap<ObjectId, usize>,
    section_header_pressure_by_id: BTreeMap<ObjectId, usize>,
    row_extra_spacing_by_row: BTreeMap<usize, usize>,
    col_gap_extra_spacing_by_col: BTreeMap<usize, usize>,
    self_loop_stub_len_by_message_id: BTreeMap<ObjectId, usize>,
}

impl SequenceSpacingBudget {
    pub fn participant_label_pressure_by_id(&self) -> &BTreeMap<ObjectId, usize> {
        &self.participant_label_pressure_by_id
    }

    pub fn message_span_pressure_by_id(&self) -> &BTreeMap<ObjectId, usize> {
        &self.message_span_pressure_by_id
    }

    pub fn block_header_pressure_by_id(&self) -> &BTreeMap<ObjectId, usize> {
        &self.block_header_pressure_by_id
    }

    pub fn section_header_pressure_by_id(&self) -> &BTreeMap<ObjectId, usize> {
        &self.section_header_pressure_by_id
    }

    pub fn row_extra_spacing_by_row(&self) -> &BTreeMap<usize, usize> {
        &self.row_extra_spacing_by_row
    }

    pub fn col_gap_extra_spacing_by_col(&self) -> &BTreeMap<usize, usize> {
        &self.col_gap_extra_spacing_by_col
    }

    pub fn self_loop_stub_len_by_message_id(&self) -> &BTreeMap<ObjectId, usize> {
        &self.self_loop_stub_len_by_message_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SequenceSpacingMeasurement {
    participant_label_pressure_by_id: BTreeMap<ObjectId, usize>,
    message_span_pressure_by_id: BTreeMap<ObjectId, usize>,
    block_header_pressure_by_id: BTreeMap<ObjectId, usize>,
    section_header_pressure_by_id: BTreeMap<ObjectId, usize>,
    self_loop_stub_pressure_by_message_id: BTreeMap<ObjectId, usize>,
    self_loop_stub_len_by_message_id: BTreeMap<ObjectId, usize>,
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

fn cmp_sequence_message_layout_order(a: &SequenceMessage, b: &SequenceMessage) -> Ordering {
    SequenceMessage::cmp_in_order(a, b)
}

fn messages_in_layout_order(ast: &SequenceAst) -> Vec<&SequenceMessage> {
    let mut messages = ast.messages().iter().collect::<Vec<_>>();
    messages.sort_unstable_by(|a, b| cmp_sequence_message_layout_order(a, b));
    messages
}

fn text_cell_width(text: &str) -> usize {
    text.chars().count()
}

fn pressure_for_label_width(label_width: usize, base_capacity: usize) -> usize {
    label_width.saturating_sub(base_capacity)
}

fn ceil_div(value: usize, divisor: usize) -> usize {
    if value == 0 {
        return 0;
    }
    ((value - 1) / divisor) + 1
}

fn set_max_spacing_budget(spacing: &mut BTreeMap<usize, usize>, key: usize, value: usize) {
    if value == 0 {
        return;
    }
    spacing.entry(key).and_modify(|existing| *existing = (*existing).max(value)).or_insert(value);
}

fn collect_block_header_measurement(
    block: &SequenceBlock,
    measurement: &mut SequenceSpacingMeasurement,
) {
    if let Some(header) = block.header() {
        let pressure =
            pressure_for_label_width(text_cell_width(header), BASE_BLOCK_HEADER_LABEL_CAPACITY);
        measurement.block_header_pressure_by_id.insert(block.block_id().clone(), pressure);
    }

    for section in block.sections() {
        if let Some(header) = section.header() {
            let pressure = pressure_for_label_width(
                text_cell_width(header),
                BASE_SECTION_HEADER_LABEL_CAPACITY,
            );
            measurement
                .section_header_pressure_by_id
                .insert(section.section_id().clone(), pressure);
        }
    }

    for nested in block.blocks() {
        collect_block_header_measurement(nested, measurement);
    }
}

fn measure_sequence_spacing(
    ast: &SequenceAst,
    messages: &[SequenceMessageLayout],
    ordered_messages: &[&SequenceMessage],
) -> SequenceSpacingMeasurement {
    debug_assert_eq!(messages.len(), ordered_messages.len());

    let mut measurement = SequenceSpacingMeasurement::default();

    for (participant_id, participant) in ast.participants() {
        let pressure = pressure_for_label_width(
            text_cell_width(participant.mermaid_name()),
            BASE_PARTICIPANT_LABEL_CAPACITY,
        );
        measurement.participant_label_pressure_by_id.insert(participant_id.clone(), pressure);
    }

    for (msg_layout, msg) in messages.iter().zip(ordered_messages.iter().copied()) {
        let label_width = text_cell_width(msg.text());
        let span_cols = msg_layout.from_col().abs_diff(msg_layout.to_col()).max(1);
        let span_capacity = span_cols.saturating_mul(BASE_MESSAGE_LABEL_CAPACITY_PER_SPAN);
        let span_pressure = pressure_for_label_width(label_width, span_capacity);

        measurement.message_span_pressure_by_id.insert(msg.message_id().clone(), span_pressure);

        if msg_layout.from_col() == msg_layout.to_col() {
            let stub_len_target = label_width
                .saturating_add(SELF_LOOP_CORNER_RESERVE)
                .clamp(BASE_SELF_LOOP_STUB_LEN, MAX_SELF_LOOP_STUB_LEN);
            let stub_pressure = stub_len_target.saturating_sub(BASE_SELF_LOOP_STUB_LEN);
            measurement
                .self_loop_stub_len_by_message_id
                .insert(msg.message_id().clone(), stub_len_target);
            measurement
                .self_loop_stub_pressure_by_message_id
                .insert(msg.message_id().clone(), stub_pressure);
        }
    }

    for block in ast.blocks() {
        collect_block_header_measurement(block, &mut measurement);
    }

    measurement
}

fn section_anchor_row(
    section: &SequenceSection,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
) -> Option<usize> {
    section
        .message_ids()
        .iter()
        .filter_map(|message_id| message_row_by_id.get(message_id).copied())
        .min()
}

fn assign_block_and_section_row_budget(
    block: &SequenceBlock,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    measurement: &SequenceSpacingMeasurement,
    row_extra_spacing_by_row: &mut BTreeMap<usize, usize>,
) -> Option<usize> {
    let mut block_anchor_row = None::<usize>;

    for section in block.sections() {
        let section_row = section_anchor_row(section, message_row_by_id);
        if let Some(row) = section_row {
            block_anchor_row = Some(block_anchor_row.map_or(row, |current| current.min(row)));
            if let Some(pressure) =
                measurement.section_header_pressure_by_id.get(section.section_id())
            {
                let extra_rows = ceil_div(*pressure, PRESSURE_PER_EXTRA_ROW);
                set_max_spacing_budget(row_extra_spacing_by_row, row, extra_rows);
            }
        }
    }

    for nested in block.blocks() {
        if let Some(nested_anchor) = assign_block_and_section_row_budget(
            nested,
            message_row_by_id,
            measurement,
            row_extra_spacing_by_row,
        ) {
            block_anchor_row =
                Some(block_anchor_row.map_or(nested_anchor, |row| row.min(nested_anchor)));
        }
    }

    if let Some(anchor_row) = block_anchor_row {
        if let Some(pressure) = measurement.block_header_pressure_by_id.get(block.block_id()) {
            let extra_rows = ceil_div(*pressure, PRESSURE_PER_EXTRA_ROW);
            set_max_spacing_budget(row_extra_spacing_by_row, anchor_row, extra_rows);
        }
    }

    block_anchor_row
}

fn build_sequence_spacing_budget(
    ast: &SequenceAst,
    participant_cols: &BTreeMap<ObjectId, usize>,
    messages: &[SequenceMessageLayout],
    measurement: &SequenceSpacingMeasurement,
) -> SequenceSpacingBudget {
    let mut row_extra_spacing_by_row = BTreeMap::<usize, usize>::new();
    let mut col_gap_extra_spacing_by_col = BTreeMap::<usize, usize>::new();

    let participant_count = participant_cols.len();
    for (participant_id, col) in participant_cols {
        let pressure =
            measurement.participant_label_pressure_by_id.get(participant_id).copied().unwrap_or(0);
        if pressure == 0 {
            continue;
        }

        if *col > 0 && *col + 1 < participant_count {
            let left_extra = pressure / 2;
            let right_extra = pressure.saturating_sub(left_extra);
            set_max_spacing_budget(&mut col_gap_extra_spacing_by_col, col - 1, left_extra);
            set_max_spacing_budget(&mut col_gap_extra_spacing_by_col, *col, right_extra);
        } else if *col > 0 {
            set_max_spacing_budget(&mut col_gap_extra_spacing_by_col, col - 1, pressure);
        } else if *col + 1 < participant_count {
            set_max_spacing_budget(&mut col_gap_extra_spacing_by_col, *col, pressure);
        }
    }

    let mut message_row_by_id = BTreeMap::<ObjectId, usize>::new();
    for msg in messages {
        message_row_by_id.insert(msg.message_id().clone(), msg.row());
        let span_pressure =
            measurement.message_span_pressure_by_id.get(msg.message_id()).copied().unwrap_or(0);
        if span_pressure == 0 {
            continue;
        }

        let min_col = msg.from_col().min(msg.to_col());
        let max_col = msg.from_col().max(msg.to_col());
        if min_col < max_col {
            let gap_count = max_col - min_col;
            let per_gap_extra = ceil_div(span_pressure, gap_count);
            for gap_col in min_col..max_col {
                set_max_spacing_budget(&mut col_gap_extra_spacing_by_col, gap_col, per_gap_extra);
            }
        } else if min_col + 1 < participant_count {
            set_max_spacing_budget(&mut col_gap_extra_spacing_by_col, min_col, span_pressure);
        } else if min_col > 0 {
            set_max_spacing_budget(&mut col_gap_extra_spacing_by_col, min_col - 1, span_pressure);
        }
    }

    for (message_id, stub_pressure) in &measurement.self_loop_stub_pressure_by_message_id {
        if *stub_pressure == 0 {
            continue;
        }
        if let Some(row) = message_row_by_id.get(message_id).copied() {
            let extra_rows = ceil_div(*stub_pressure, PRESSURE_PER_EXTRA_ROW);
            set_max_spacing_budget(&mut row_extra_spacing_by_row, row, extra_rows);
        }
    }

    for block in ast.blocks() {
        assign_block_and_section_row_budget(
            block,
            &message_row_by_id,
            measurement,
            &mut row_extra_spacing_by_row,
        );
    }

    SequenceSpacingBudget {
        participant_label_pressure_by_id: measurement.participant_label_pressure_by_id.clone(),
        message_span_pressure_by_id: measurement.message_span_pressure_by_id.clone(),
        block_header_pressure_by_id: measurement.block_header_pressure_by_id.clone(),
        section_header_pressure_by_id: measurement.section_header_pressure_by_id.clone(),
        row_extra_spacing_by_row,
        col_gap_extra_spacing_by_col,
        self_loop_stub_len_by_message_id: measurement.self_loop_stub_len_by_message_id.clone(),
    }
}

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

    let ordered_messages = messages_in_layout_order(ast);
    let messages = ordered_messages
        .iter()
        .copied()
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

    let spacing_measurement = measure_sequence_spacing(ast, &messages, &ordered_messages);
    let spacing_budget =
        build_sequence_spacing_budget(ast, &participant_cols, &messages, &spacing_measurement);

    Ok(SequenceLayout { participant_cols, messages, spacing_budget })
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{
        cmp_sequence_message_layout_order, layout_sequence, SequenceLayoutError,
        BASE_SELF_LOOP_STUB_LEN,
    };
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind,
        SequenceParticipant, SequenceSection, SequenceSectionKind,
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
    fn message_order_tie_breaker_uses_message_id() {
        let p_alice = ObjectId::new("p:alice").expect("participant id");
        let p_bob = ObjectId::new("p:bob").expect("participant id");

        let m_0001 = SequenceMessage::new(
            ObjectId::new("m:0001").expect("message id"),
            p_alice.clone(),
            p_bob.clone(),
            SequenceMessageKind::Sync,
            "Hello 1",
            1000,
        );
        let m_0002 = SequenceMessage::new(
            ObjectId::new("m:0002").expect("message id"),
            p_alice,
            p_bob,
            SequenceMessageKind::Sync,
            "Hello 2",
            1000,
        );

        assert_eq!(cmp_sequence_message_layout_order(&m_0001, &m_0002), Ordering::Less);
        assert_eq!(cmp_sequence_message_layout_order(&m_0002, &m_0001), Ordering::Greater);
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
    fn spacing_budget_collects_deterministic_pressure_inputs() {
        let mut ast = SequenceAst::default();
        let p_alice = ObjectId::new("p:alice").expect("participant id");
        let p_bob = ObjectId::new("p:bob").expect("participant id");
        let p_carol = ObjectId::new("p:carol").expect("participant id");
        ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
        ast.participants_mut()
            .insert(p_bob.clone(), SequenceParticipant::new("ParticipantWithExtendedDisplayName"));
        ast.participants_mut().insert(p_carol.clone(), SequenceParticipant::new("Carol"));

        let m_0001 = ObjectId::new("m:0001").expect("message id");
        let m_0002 = ObjectId::new("m:0002").expect("message id");
        ast.messages_mut().push(SequenceMessage::new(
            m_0001.clone(),
            p_alice,
            p_carol,
            SequenceMessageKind::Sync,
            "This message label is intentionally long to exceed baseline span capacity",
            1000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            m_0002.clone(),
            p_bob.clone(),
            p_bob,
            SequenceMessageKind::Sync,
            "self loop label requires more horizontal room",
            2000,
        ));

        ast.blocks_mut().push(SequenceBlock::new(
            ObjectId::new("b:0001").expect("block id"),
            SequenceBlockKind::Alt,
            Some("Alternative branch with long deterministic header".to_owned()),
            vec![SequenceSection::new(
                ObjectId::new("sec:0001:00").expect("section id"),
                SequenceSectionKind::Main,
                Some("Main section with long deterministic header".to_owned()),
                vec![m_0002.clone()],
            )],
            Vec::new(),
        ));

        let layout = layout_sequence(&ast).expect("layout");
        let budget = layout.spacing_budget();

        assert!(
            budget
                .participant_label_pressure_by_id()
                .get(&ObjectId::new("p:bob").expect("participant id"))
                .copied()
                .unwrap_or(0)
                > 0
        );
        assert!(budget.message_span_pressure_by_id().get(&m_0001).copied().unwrap_or(0) > 0);
        assert!(budget.block_header_pressure_by_id().values().any(|pressure| *pressure > 0));
        assert!(budget.section_header_pressure_by_id().values().any(|pressure| *pressure > 0));
        assert!(
            budget
                .self_loop_stub_len_by_message_id()
                .get(&m_0002)
                .copied()
                .unwrap_or(BASE_SELF_LOOP_STUB_LEN)
                > BASE_SELF_LOOP_STUB_LEN
        );
        assert!(!budget.col_gap_extra_spacing_by_col().is_empty());

        let self_loop_row = layout
            .messages()
            .iter()
            .find(|msg| msg.message_id() == &m_0002)
            .map(|msg| msg.row())
            .expect("self-loop message row");
        assert!(budget.row_extra_spacing_by_row().get(&self_loop_row).copied().unwrap_or(0) > 0);
    }

    #[test]
    fn spacing_budget_tie_case_stably_targets_self_loop_row() {
        let mut ast = SequenceAst::default();
        let p_alice = ObjectId::new("p:alice").expect("participant id");
        let p_bob = ObjectId::new("p:bob").expect("participant id");
        ast.participants_mut().insert(p_alice.clone(), SequenceParticipant::new("Alice"));
        ast.participants_mut().insert(p_bob.clone(), SequenceParticipant::new("Bob"));

        let m_0002 = ObjectId::new("m:0002").expect("message id");
        ast.messages_mut().push(SequenceMessage::new(
            m_0002.clone(),
            p_alice.clone(),
            p_alice.clone(),
            SequenceMessageKind::Sync,
            "self loop tie case needs extra stub",
            1000,
        ));

        let m_0001 = ObjectId::new("m:0001").expect("message id");
        ast.messages_mut().push(SequenceMessage::new(
            m_0001.clone(),
            p_alice,
            p_bob,
            SequenceMessageKind::Sync,
            "ok",
            1000,
        ));

        let layout = layout_sequence(&ast).expect("layout");
        let ordered_ids =
            layout.messages().iter().map(|msg| msg.message_id().as_str()).collect::<Vec<_>>();
        assert_eq!(ordered_ids, vec!["m:0001", "m:0002"]);

        let self_loop_row = layout
            .messages()
            .iter()
            .find(|msg| msg.message_id() == &m_0002)
            .map(|msg| msg.row())
            .expect("self-loop message row");
        assert_eq!(self_loop_row, 1);

        let budget = layout.spacing_budget();
        assert!(budget.row_extra_spacing_by_row().get(&self_loop_row).copied().unwrap_or(0) > 0);
        assert_eq!(budget.row_extra_spacing_by_row().get(&0).copied().unwrap_or(0), 0);
        assert!(
            budget
                .self_loop_stub_len_by_message_id()
                .get(&m_0002)
                .copied()
                .unwrap_or(BASE_SELF_LOOP_STUB_LEN)
                > BASE_SELF_LOOP_STUB_LEN
        );
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
