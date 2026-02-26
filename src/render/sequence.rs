// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::layout::SequenceLayout;
use crate::model::ids::{DiagramId, ObjectId};
use crate::model::seq_ast::{
    SequenceAst, SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind,
    SequenceSection, SequenceSectionKind,
};
use crate::model::{CategoryPath, ObjectRef};

use super::text::{text_len, truncate_with_ellipsis};
use super::{
    clamp_highlight_index_to_text, AnnotatedRender, Canvas, CanvasError, HighlightIndex, LineSpan,
    RenderOptions,
};

const BOX_HEIGHT_NO_NOTES: usize = 3;
const BOX_HEIGHT_WITH_NOTES: usize = 4;
const HEADER_GAP: usize = 2;
const ROW_SPACING: usize = 2;
const COL_GAP: usize = 8;
const BLOCK_TOP_ROW_EXTRA: usize = 2;
const BLOCK_TOP_LABEL_OFFSET: usize = 2;
const SECTION_TOP_ROW_EXTRA: usize = 2;
const SECTION_LABEL_OFFSET: usize = 2;
const TOP_LEVEL_BLOCK_TRANSITION_EXTRA: usize = 1;
const PARTICIPANT_LEFT_MARGIN: usize = 1;
const RIGHT_MARGIN: usize = 6;
const MIN_BOX_INNER_WIDTH: usize = 3;
const SELF_MESSAGE_STUB_LEN: usize = 8;
const SELF_MESSAGE_STUB_MAX_LEN: usize = 32;
const SELF_MESSAGE_STUB_FRACTION_NUM: usize = 5;
const SELF_MESSAGE_STUB_FRACTION_DEN: usize = 6;
const SELF_MESSAGE_LOOP_DROP: usize = 1;
const SELF_MESSAGE_LABEL_LEFT_PADDING: usize = 1;
const SELF_MESSAGE_LABEL_PRE_CORNER_RESERVE: usize = 1;
const SELF_MESSAGE_LABEL_RIGHT_RESERVE: usize = 1 + SELF_MESSAGE_LABEL_PRE_CORNER_RESERVE;
const OBJECT_LABEL_PREFIX: &str = "▴ ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParticipantRender<'a> {
    col: usize,
    participant_id: &'a ObjectId,
    name: &'a str,
    note: Option<&'a str>,
    box_x0: usize,
    box_x1: usize,
    box_inner_width: usize,
    lifeline_x: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceRenderError {
    Canvas(CanvasError),
    MissingParticipant { participant_id: ObjectId },
    MissingMessage { message_id: ObjectId },
    InvalidParticipantColumn { col: usize },
    InvalidBlockMembership { block_id: ObjectId, reason: String },
}

impl fmt::Display for SequenceRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canvas(err) => write!(f, "canvas error: {err}"),
            Self::MissingParticipant { participant_id } => {
                write!(f, "missing participant {participant_id} in AST")
            }
            Self::MissingMessage { message_id } => write!(f, "missing message {message_id} in AST"),
            Self::InvalidParticipantColumn { col } => {
                write!(f, "invalid participant column: {col}")
            }
            Self::InvalidBlockMembership { block_id, reason } => {
                write!(f, "invalid block membership (block_id={block_id}): {reason}")
            }
        }
    }
}

impl std::error::Error for SequenceRenderError {}

impl From<CanvasError> for SequenceRenderError {
    fn from(value: CanvasError) -> Self {
        Self::Canvas(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrowDir {
    Left,
    Right,
}

/// Deterministic baseline Unicode renderer for a sequence diagram.
///
/// This consumes coordinates-only `SequenceLayout` and uses the AST only for participant names
/// and message labels.
pub fn render_sequence_unicode(
    ast: &SequenceAst,
    layout: &SequenceLayout,
) -> Result<String, SequenceRenderError> {
    render_sequence_unicode_with_options(ast, layout, RenderOptions::default())
}

pub fn render_sequence_unicode_with_options(
    ast: &SequenceAst,
    layout: &SequenceLayout,
    options: RenderOptions,
) -> Result<String, SequenceRenderError> {
    let box_height = seq_box_height(options);
    let participants = participants_in_col_order(layout);
    let messages_by_id = messages_by_id(ast);

    let mut participant_renders = Vec::<ParticipantRender>::with_capacity(participants.len());
    let mut cursor_x = PARTICIPANT_LEFT_MARGIN;

    for (col, participant_id) in participants {
        let participant = ast.participants().get(participant_id).ok_or_else(|| {
            SequenceRenderError::MissingParticipant { participant_id: participant_id.clone() }
        })?;
        let name = participant.mermaid_name();
        let note = if options.show_notes { participant.note() } else { None };

        let (box_inner_width, box_total_width) = box_widths_prefixed(name, options);
        let box_x0 = cursor_x;
        let box_x1 = box_x0 + box_total_width - 1;
        let lifeline_x = box_x0 + (box_total_width / 2);

        participant_renders.push(ParticipantRender {
            col,
            participant_id,
            name,
            note,
            box_x0,
            box_x1,
            box_inner_width,
            lifeline_x,
        });

        cursor_x = box_x1 + 1 + COL_GAP;
    }

    let width = participant_renders.last().map(|p| p.box_x1 + 1 + RIGHT_MARGIN).unwrap_or(1);

    let message_top_y = box_height + HEADER_GAP;
    let self_loop_rows = collect_self_loop_rows(layout);
    let row_y_by_row = build_message_row_positions_with_spacing_budget(
        ast,
        layout,
        message_top_y,
        &self_loop_rows,
    )?;
    let bottom_y =
        compute_sequence_bottom_y(ast, layout, &row_y_by_row, &self_loop_rows, box_height)?;
    let height = bottom_y + 1;

    // Layer 1: participants, lifelines, and message connectors/text.
    let mut connector_layer = Canvas::new(width, height)?;

    let mut lifeline_x_by_col = BTreeMap::<usize, usize>::new();
    let next_lifeline_x_by_col = next_lifeline_x_by_col(&participant_renders);

    for p in &participant_renders {
        connector_layer.draw_box(p.box_x0, 0, p.box_x1, box_height - 1)?;

        let display_name = prefixed_object_label(p.name, options);
        let name_len = text_len(&display_name);
        let left_pad = (p.box_inner_width.saturating_sub(name_len)) / 2;
        let name_x = p.box_x0 + 1 + left_pad;
        connector_layer.write_str(name_x, 1, &display_name)?;

        if options.show_notes {
            if let Some(note) = p.note {
                let clipped = truncate_with_ellipsis(note, p.box_inner_width);
                let clipped_len = text_len(&clipped);
                let left_pad = (p.box_inner_width.saturating_sub(clipped_len)) / 2;
                let note_x = p.box_x0 + 1 + left_pad;
                connector_layer.write_str(note_x, 2, &clipped)?;
            }
        }

        connector_layer.draw_vline(p.lifeline_x, box_height, bottom_y)?;
        lifeline_x_by_col.insert(p.col, p.lifeline_x);
    }

    for msg_layout in layout.messages() {
        let msg = messages_by_id.get(msg_layout.message_id()).ok_or_else(|| {
            SequenceRenderError::MissingMessage { message_id: msg_layout.message_id().clone() }
        })?;

        let from_x = *lifeline_x_by_col
            .get(&msg_layout.from_col())
            .ok_or(SequenceRenderError::InvalidParticipantColumn { col: msg_layout.from_col() })?;
        let to_x = *lifeline_x_by_col
            .get(&msg_layout.to_col())
            .ok_or(SequenceRenderError::InvalidParticipantColumn { col: msg_layout.to_col() })?;

        let y = row_y_for(msg_layout.row(), &row_y_by_row, message_top_y);
        let message_text = prefixed_object_label(msg.text(), options);
        let self_right_limit =
            self_message_right_limit(msg_layout.from_col(), &next_lifeline_x_by_col, width);
        let self_stub_len_target =
            self_message_stub_len_target(layout, msg_layout.message_id(), &message_text);
        draw_message(
            &mut connector_layer,
            from_x,
            to_x,
            y,
            msg.kind(),
            &message_text,
            self_right_limit,
            self_stub_len_target,
        )?;
    }

    // Layer 2: block/section frame geometry.
    let mut frame_layer = Canvas::new(width, height)?;
    let overlays = draw_sequence_block_decorations(
        &mut frame_layer,
        ast,
        layout,
        &row_y_by_row,
        &self_loop_rows,
        width,
    )?;
    let mut composited = connector_layer;
    blend_low_priority_layer(&mut composited, &frame_layer)?;

    // Layer 3: labels are projected as final overlays by `canvas_to_string_trimmed_with_overlays`.
    Ok(canvas_to_string_trimmed_with_overlays(&composited, &overlays))
}

pub fn render_sequence_unicode_annotated(
    diagram_id: &DiagramId,
    ast: &SequenceAst,
    layout: &SequenceLayout,
) -> Result<AnnotatedRender, SequenceRenderError> {
    render_sequence_unicode_annotated_with_options(
        diagram_id,
        ast,
        layout,
        RenderOptions::default(),
    )
}

pub fn render_sequence_unicode_annotated_with_options(
    diagram_id: &DiagramId,
    ast: &SequenceAst,
    layout: &SequenceLayout,
    options: RenderOptions,
) -> Result<AnnotatedRender, SequenceRenderError> {
    let box_height = seq_box_height(options);
    let text = render_sequence_unicode_with_options(ast, layout, options)?;

    let participants = participants_in_col_order(layout);
    let mut participant_renders = Vec::<ParticipantRender>::with_capacity(participants.len());
    let mut cursor_x = PARTICIPANT_LEFT_MARGIN;

    for (col, participant_id) in participants {
        let participant = ast.participants().get(participant_id).ok_or_else(|| {
            SequenceRenderError::MissingParticipant { participant_id: participant_id.clone() }
        })?;
        let name = participant.mermaid_name();
        let note = if options.show_notes { participant.note() } else { None };

        let (box_inner_width, box_total_width) = box_widths_prefixed(name, options);
        let box_x0 = cursor_x;
        let box_x1 = box_x0 + box_total_width - 1;
        let lifeline_x = box_x0 + (box_total_width / 2);

        participant_renders.push(ParticipantRender {
            col,
            participant_id,
            name,
            note,
            box_x0,
            box_x1,
            box_inner_width,
            lifeline_x,
        });

        cursor_x = box_x1 + 1 + COL_GAP;
    }

    let width = participant_renders.last().map(|p| p.box_x1 + 1 + RIGHT_MARGIN).unwrap_or(1);

    let message_top_y = box_height + HEADER_GAP;
    let self_loop_rows = collect_self_loop_rows(layout);
    let row_y_by_row = build_message_row_positions_with_spacing_budget(
        ast,
        layout,
        message_top_y,
        &self_loop_rows,
    )?;
    let bottom_y =
        compute_sequence_bottom_y(ast, layout, &row_y_by_row, &self_loop_rows, box_height)?;

    let mut lifeline_x_by_col = BTreeMap::<usize, usize>::new();
    for p in &participant_renders {
        lifeline_x_by_col.insert(p.col, p.lifeline_x);
    }
    let next_lifeline_x_by_col = next_lifeline_x_by_col(&participant_renders);

    let seq_participant_category =
        CategoryPath::new(vec!["seq".to_owned(), "participant".to_owned()]).expect("valid");
    let seq_message_category =
        CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()]).expect("valid");
    let seq_note_category =
        CategoryPath::new(vec!["seq".to_owned(), "note".to_owned()]).expect("valid");
    let seq_block_category =
        CategoryPath::new(vec!["seq".to_owned(), "block".to_owned()]).expect("valid");
    let seq_section_category =
        CategoryPath::new(vec!["seq".to_owned(), "section".to_owned()]).expect("valid");

    let mut highlight_index = HighlightIndex::new();
    let mut connector_object_refs = Vec::<ObjectRef>::new();
    let frame_object_refs =
        collect_block_and_section_refs(ast, diagram_id, &seq_block_category, &seq_section_category);

    for p in &participant_renders {
        let object_ref = ObjectRef::new(
            diagram_id.clone(),
            seq_participant_category.clone(),
            p.participant_id.clone(),
        );

        let mut spans = Vec::<LineSpan>::new();
        for y in 0..box_height {
            spans.push((y, p.box_x0, p.box_x1));
        }
        for y in box_height..=bottom_y {
            spans.push((y, p.lifeline_x, p.lifeline_x));
        }

        spans.sort();
        spans.dedup();
        connector_object_refs.push(object_ref.clone());
        highlight_index.insert(object_ref, spans);

        if options.show_notes {
            if let Some(note) = p.note {
                let clipped = truncate_with_ellipsis(note, p.box_inner_width);
                let clipped_len = text_len(&clipped);
                if clipped_len > 0 {
                    let left_pad = (p.box_inner_width.saturating_sub(clipped_len)) / 2;
                    let note_x = p.box_x0 + 1 + left_pad;
                    let note_y = 2usize;
                    let note_ref = ObjectRef::new(
                        diagram_id.clone(),
                        seq_note_category.clone(),
                        p.participant_id.clone(),
                    );
                    connector_object_refs.push(note_ref.clone());
                    highlight_index.insert(
                        note_ref,
                        vec![(note_y, note_x, note_x + clipped_len.saturating_sub(1))],
                    );
                }
            }
        }
    }

    for msg_layout in layout.messages() {
        let from_x = *lifeline_x_by_col
            .get(&msg_layout.from_col())
            .ok_or(SequenceRenderError::InvalidParticipantColumn { col: msg_layout.from_col() })?;
        let to_x = *lifeline_x_by_col
            .get(&msg_layout.to_col())
            .ok_or(SequenceRenderError::InvalidParticipantColumn { col: msg_layout.to_col() })?;

        let y = row_y_for(msg_layout.row(), &row_y_by_row, message_top_y);
        let mut spans = Vec::<LineSpan>::new();
        if from_x == to_x {
            let self_right_limit =
                self_message_right_limit(msg_layout.from_col(), &next_lifeline_x_by_col, width);
            let self_stub_len_target = layout
                .spacing_budget()
                .self_loop_stub_len_by_message_id()
                .get(msg_layout.message_id())
                .copied();
            if let Some(stub_end) =
                self_message_stub_end(from_x, self_right_limit, self_stub_len_target)
            {
                spans.push((y, from_x, stub_end));
                let y1 = y.saturating_add(SELF_MESSAGE_LOOP_DROP);
                spans.push((y1, stub_end, stub_end));
                spans.push((y1, from_x, stub_end));
            }
        } else {
            // Include both endpoint lifelines so focused/selected message highlight touches
            // sender and receiver participant lines at the message row.
            spans.push((y, from_x.min(to_x), from_x.max(to_x)));
        }

        spans.sort();
        spans.dedup();

        let object_ref = ObjectRef::new(
            diagram_id.clone(),
            seq_message_category.clone(),
            msg_layout.message_id().clone(),
        );
        connector_object_refs.push(object_ref.clone());
        highlight_index.insert(object_ref, spans);
    }

    if !ast.blocks().is_empty() {
        let mut message_row_by_id = BTreeMap::<ObjectId, usize>::new();
        for msg in layout.messages() {
            message_row_by_id.insert(msg.message_id().clone(), msg.row());
        }

        for block in ast.blocks() {
            insert_sequence_block_highlights(
                &mut highlight_index,
                diagram_id,
                &seq_block_category,
                &seq_section_category,
                block,
                0,
                &message_row_by_id,
                &row_y_by_row,
                &self_loop_rows,
                width,
            )?;
        }
    }

    clamp_highlight_index_to_text(&mut highlight_index, &text);
    project_highlight_index_to_final_visible_cells(
        &mut highlight_index,
        &text,
        &connector_object_refs,
        &frame_object_refs,
        ast,
        layout,
        &row_y_by_row,
        &self_loop_rows,
        width,
        bottom_y,
    )?;
    Ok(AnnotatedRender { text, highlight_index })
}

fn collect_block_and_section_refs(
    ast: &SequenceAst,
    diagram_id: &DiagramId,
    seq_block_category: &CategoryPath,
    seq_section_category: &CategoryPath,
) -> Vec<ObjectRef> {
    fn collect_from_block(
        refs: &mut Vec<ObjectRef>,
        block: &SequenceBlock,
        diagram_id: &DiagramId,
        seq_block_category: &CategoryPath,
        seq_section_category: &CategoryPath,
    ) {
        refs.push(ObjectRef::new(
            diagram_id.clone(),
            seq_block_category.clone(),
            block.block_id().clone(),
        ));
        for section in block.sections() {
            refs.push(ObjectRef::new(
                diagram_id.clone(),
                seq_section_category.clone(),
                section.section_id().clone(),
            ));
        }
        for nested in block.blocks() {
            collect_from_block(refs, nested, diagram_id, seq_block_category, seq_section_category);
        }
    }

    let mut out = Vec::<ObjectRef>::new();
    for block in ast.blocks() {
        collect_from_block(&mut out, block, diagram_id, seq_block_category, seq_section_category);
    }
    out
}

fn spans_to_cells(spans: &[LineSpan]) -> BTreeSet<(usize, usize)> {
    let mut cells = BTreeSet::<(usize, usize)>::new();
    for (y, x0, x1) in spans {
        for x in *x0..=*x1 {
            cells.insert((*y, x));
        }
    }
    cells
}

fn cells_to_spans(cells: &BTreeSet<(usize, usize)>) -> Vec<LineSpan> {
    let mut by_row = BTreeMap::<usize, Vec<usize>>::new();
    for (y, x) in cells {
        by_row.entry(*y).or_default().push(*x);
    }

    let mut spans = Vec::<LineSpan>::new();
    for (y, mut xs) in by_row {
        if xs.is_empty() {
            continue;
        }

        xs.sort_unstable();
        xs.dedup();

        let mut start = xs[0];
        let mut prev = xs[0];
        for &x in xs.iter().skip(1) {
            if x == prev.saturating_add(1) {
                prev = x;
                continue;
            }
            spans.push((y, start, prev));
            start = x;
            prev = x;
        }
        spans.push((y, start, prev));
    }
    spans
}

fn text_lines_as_cells(text: &str) -> Vec<Vec<char>> {
    text.split('\n').map(|line| line.chars().collect::<Vec<_>>()).collect::<Vec<_>>()
}

fn cell_is_visible(lines: &[Vec<char>], y: usize, x: usize) -> bool {
    lines.get(y).and_then(|line| line.get(x)).copied().map(|ch| ch != ' ').unwrap_or(false)
}

#[allow(clippy::too_many_arguments)]
fn project_highlight_index_to_final_visible_cells(
    highlight_index: &mut HighlightIndex,
    text: &str,
    connector_object_refs: &[ObjectRef],
    frame_object_refs: &[ObjectRef],
    ast: &SequenceAst,
    layout: &SequenceLayout,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
    width: usize,
    bottom_y: usize,
) -> Result<(), SequenceRenderError> {
    let mut object_cells = BTreeMap::<ObjectRef, BTreeSet<(usize, usize)>>::new();
    for (object_ref, spans) in highlight_index.iter() {
        object_cells.insert(object_ref.clone(), spans_to_cells(spans));
    }

    let mut connector_cells = BTreeSet::<(usize, usize)>::new();
    for object_ref in connector_object_refs {
        if let Some(cells) = object_cells.get(object_ref) {
            connector_cells.extend(cells.iter().copied());
        }
    }

    let mut frame_candidate_cells = BTreeMap::<ObjectRef, BTreeSet<(usize, usize)>>::new();
    for object_ref in frame_object_refs {
        let Some(cells) = object_cells.get_mut(object_ref) else {
            continue;
        };
        frame_candidate_cells.insert(object_ref.clone(), cells.clone());
        cells.retain(|cell| !connector_cells.contains(cell));
    }

    // Replay frame-label overlays as final layer owners.
    let mut frame_layer_probe = Canvas::new(width, bottom_y.saturating_add(1))?;
    let overlays = draw_sequence_block_decorations(
        &mut frame_layer_probe,
        ast,
        layout,
        row_y_by_row,
        self_loop_rows,
        width,
    )?;
    for overlay in overlays {
        let mut x = overlay.x;
        for ch in overlay.text.chars() {
            if ch == ' ' {
                x = x.saturating_add(1);
                continue;
            }
            let cell = (overlay.y, x);
            let owners = frame_object_refs
                .iter()
                .filter(|object_ref| {
                    frame_candidate_cells
                        .get(*object_ref)
                        .is_some_and(|cells| cells.contains(&cell))
                })
                .cloned()
                .collect::<Vec<_>>();
            if owners.is_empty() {
                x = x.saturating_add(1);
                continue;
            }

            for cells in object_cells.values_mut() {
                cells.remove(&cell);
            }
            for owner in owners {
                object_cells.entry(owner).or_default().insert(cell);
            }

            x = x.saturating_add(1);
        }
    }

    let lines = text_lines_as_cells(text);
    for (object_ref, cells) in object_cells {
        let mut visible_cells = cells;
        visible_cells.retain(|(y, x)| cell_is_visible(&lines, *y, *x));
        let projected = cells_to_spans(&visible_cells);
        highlight_index.insert(object_ref, projected);
    }

    Ok(())
}

fn build_message_row_positions_with_spacing_budget(
    ast: &SequenceAst,
    layout: &SequenceLayout,
    message_top_y: usize,
    self_loop_rows: &BTreeSet<usize>,
) -> Result<BTreeMap<usize, usize>, SequenceRenderError> {
    let mut row_y_by_row = build_message_row_positions(ast, layout, message_top_y, self_loop_rows)?;
    inflate_row_positions_with_spacing_budget(layout, &mut row_y_by_row);
    Ok(row_y_by_row)
}

fn inflate_row_positions_with_spacing_budget(
    layout: &SequenceLayout,
    row_y_by_row: &mut BTreeMap<usize, usize>,
) {
    if layout.messages().is_empty() || row_y_by_row.is_empty() {
        return;
    }

    let row_budget = layout.spacing_budget().row_extra_spacing_by_row();
    if row_budget.is_empty() {
        return;
    }

    let last_row = layout.messages().iter().map(|m| m.row()).max().unwrap_or(0);
    let mut cumulative_extra = 0usize;

    for row in 0..=last_row {
        if row > 0 {
            cumulative_extra =
                cumulative_extra.saturating_add(row_budget.get(&row).copied().unwrap_or(0));
        }

        if cumulative_extra == 0 {
            continue;
        }

        if let Some(y) = row_y_by_row.get_mut(&row) {
            *y = y.saturating_add(cumulative_extra);
        }
    }
}

// Extracted sequence rendering internals and block drawing helpers.
include!("sequence/helpers.rs");

#[cfg(test)]
mod row_planner_tests {
    use super::{
        build_message_row_positions, build_message_row_positions_with_spacing_budget,
        collect_self_loop_rows, seq_box_height, HEADER_GAP,
    };
    use crate::layout::layout_sequence;
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    };
    use crate::render::RenderOptions;

    fn fixture_row_spacing_ast() -> SequenceAst {
        let mut ast = SequenceAst::default();
        let p_a = ObjectId::new("p:a").expect("participant id");
        let p_b = ObjectId::new("p:b").expect("participant id");

        ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
        ast.participants_mut().insert(p_b.clone(), SequenceParticipant::new("B"));

        ast.messages_mut().push(SequenceMessage::new(
            ObjectId::new("m:0001").expect("message id"),
            p_a.clone(),
            p_b.clone(),
            SequenceMessageKind::Sync,
            "start",
            1000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            ObjectId::new("m:0002").expect("message id"),
            p_b.clone(),
            p_b.clone(),
            SequenceMessageKind::Sync,
            "self loop label that should trigger deterministic vertical inflation",
            2000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            ObjectId::new("m:0003").expect("message id"),
            p_b,
            p_a,
            SequenceMessageKind::Sync,
            "end",
            3000,
        ));

        ast
    }

    #[test]
    fn row_planner_applies_spacing_budget_for_self_loop_rows() {
        let ast = fixture_row_spacing_ast();
        let layout = layout_sequence(&ast).expect("layout");
        let self_loop_rows = collect_self_loop_rows(&layout);
        let message_top_y = seq_box_height(RenderOptions::default()) + HEADER_GAP;

        let base = build_message_row_positions(&ast, &layout, message_top_y, &self_loop_rows)
            .expect("base row plan");
        let inflated = build_message_row_positions_with_spacing_budget(
            &ast,
            &layout,
            message_top_y,
            &self_loop_rows,
        )
        .expect("inflated row plan");

        assert!(
            layout.spacing_budget().row_extra_spacing_by_row().get(&1).copied().unwrap_or(0) > 0
        );
        assert!(inflated[&1] > base[&1]);
        assert!(inflated[&2] > base[&2]);
    }

    #[test]
    fn row_planner_budget_inflation_is_deterministic_for_message_insertion_order() {
        let ast_a = fixture_row_spacing_ast();

        let mut ast_b = SequenceAst::default();
        for (id, participant) in ast_a.participants() {
            ast_b.participants_mut().insert(id.clone(), participant.clone());
        }
        for msg in ast_a.messages().iter().rev() {
            ast_b.messages_mut().push(msg.clone());
        }

        let message_top_y = seq_box_height(RenderOptions::default()) + HEADER_GAP;

        let layout_a = layout_sequence(&ast_a).expect("layout a");
        let loops_a = collect_self_loop_rows(&layout_a);
        let rows_a = build_message_row_positions_with_spacing_budget(
            &ast_a,
            &layout_a,
            message_top_y,
            &loops_a,
        )
        .expect("rows a");

        let layout_b = layout_sequence(&ast_b).expect("layout b");
        let loops_b = collect_self_loop_rows(&layout_b);
        let rows_b = build_message_row_positions_with_spacing_budget(
            &ast_b,
            &layout_b,
            message_top_y,
            &loops_b,
        )
        .expect("rows b");

        assert_eq!(rows_a, rows_b);
    }
}

#[cfg(test)]
mod self_loop_geometry_tests {
    use super::{
        build_message_row_positions_with_spacing_budget, collect_self_loop_rows,
        render_sequence_unicode, self_message_label_bounds, self_message_stub_end,
        self_message_stub_len_target, seq_box_height, HEADER_GAP, SELF_MESSAGE_STUB_MAX_LEN,
    };
    use crate::layout::layout_sequence;
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    };
    use crate::render::RenderOptions;

    #[test]
    fn self_loop_stub_end_clamps_preferred_len_deterministically() {
        assert_eq!(self_message_stub_end(10, 20, Some(64)), Some(20));
        assert_eq!(self_message_stub_end(10, 20, Some(1)), Some(18));
        assert_eq!(self_message_stub_end(10, 11, Some(64)), Some(11));
        assert_eq!(self_message_stub_end(10, 10, Some(64)), None);
    }

    #[test]
    fn self_loop_label_bounds_keep_corner_reserve() {
        assert_eq!(self_message_label_bounds(4, 12), Some((5, 10)));
        assert_eq!(self_message_label_bounds(4, 5), None);
    }

    #[test]
    fn dense_self_loop_render_preserves_corner_readability_with_budget_clamp() {
        let mut ast = SequenceAst::default();
        let p_a = ObjectId::new("p:a").expect("participant id");
        let p_b = ObjectId::new("p:b").expect("participant id");
        let m_0001 = ObjectId::new("m:0001").expect("message id");
        let label = "self loop label that is intentionally too long for narrow clamp window";

        ast.participants_mut().insert(p_a.clone(), SequenceParticipant::new("A"));
        ast.participants_mut().insert(p_b, SequenceParticipant::new("B"));
        ast.messages_mut().push(SequenceMessage::new(
            m_0001.clone(),
            p_a.clone(),
            p_a,
            SequenceMessageKind::Sync,
            label,
            1000,
        ));

        let layout = layout_sequence(&ast).expect("layout");
        let budget_stub = layout
            .spacing_budget()
            .self_loop_stub_len_by_message_id()
            .get(&m_0001)
            .copied()
            .expect("budget stub len");
        assert_eq!(
            self_message_stub_len_target(&layout, &m_0001, label),
            Some(budget_stub.clamp(0, SELF_MESSAGE_STUB_MAX_LEN))
        );

        let rendered = render_sequence_unicode(&ast, &layout).expect("render");
        let lines = rendered.lines().collect::<Vec<_>>();
        let self_loop_rows = collect_self_loop_rows(&layout);
        let message_top_y = seq_box_height(RenderOptions::default()) + HEADER_GAP;
        let row_y_by_row = build_message_row_positions_with_spacing_budget(
            &ast,
            &layout,
            message_top_y,
            &self_loop_rows,
        )
        .expect("row positions");
        let row = layout
            .messages()
            .iter()
            .find(|msg| msg.message_id() == &m_0001)
            .map(|msg| msg.row())
            .expect("message row");
        let y = row_y_by_row.get(&row).copied().expect("row y");
        let top_line = lines.get(y).expect("top row");
        let chars = top_line.chars().collect::<Vec<_>>();

        let corner_idx = chars.iter().position(|ch| *ch == '┐').expect("top right corner");
        assert!(corner_idx > 0);
        assert_eq!(chars[corner_idx - 1], '─');
        assert!(top_line.contains('…'));
    }
}

#[cfg(test)]
mod block_layering_tests {
    use super::render_sequence_unicode;
    use crate::format::mermaid::sequence::parse_sequence_diagram;
    use crate::layout::layout_sequence;

    fn nested_block_fixture() -> &'static str {
        "\
sequenceDiagram\n\
participant A\n\
participant B\n\
A->>B: Pre\n\
alt Outer\n\
B->>A: In0\n\
opt Inner\n\
B->>A: In1\n\
end\n\
B->>A: In2\n\
else Other\n\
B->>A: In3\n\
end\n\
A->>B: Post\n"
    }

    fn has_arrow_head(line: &str) -> bool {
        line.contains('◀') || line.contains('▶') || line.contains('◁') || line.contains('▷')
    }

    #[test]
    fn nested_block_layering_preserves_message_arrowheads() {
        let ast = parse_sequence_diagram(nested_block_fixture()).expect("parse");
        let layout = layout_sequence(&ast).expect("layout");
        let rendered = render_sequence_unicode(&ast, &layout).expect("render");
        let lines = rendered.lines().collect::<Vec<_>>();

        for label in ["In0", "In1", "In2", "In3", "Post"] {
            let line = lines
                .iter()
                .find(|line| line.contains(label))
                .unwrap_or_else(|| panic!("missing message label `{label}` in:\n{rendered}"));
            assert!(
                has_arrow_head(line),
                "message label `{label}` lost arrowhead after layering: `{line}`"
            );
        }
    }

    #[test]
    fn nested_block_layering_is_deterministic_across_repeated_renders() {
        let ast = parse_sequence_diagram(nested_block_fixture()).expect("parse");
        let layout = layout_sequence(&ast).expect("layout");
        let baseline = render_sequence_unicode(&ast, &layout).expect("render baseline");

        for _ in 0..50 {
            let rerendered = render_sequence_unicode(&ast, &layout).expect("render rerun");
            assert_eq!(rerendered, baseline);
        }
    }
}

#[cfg(test)]
mod tests;
