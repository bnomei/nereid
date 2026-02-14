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
const SELF_MESSAGE_STUB_FRACTION_NUM: usize = 5;
const SELF_MESSAGE_STUB_FRACTION_DEN: usize = 6;
const SELF_MESSAGE_LOOP_DROP: usize = 1;
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
                write!(
                    f,
                    "invalid block membership (block_id={block_id}): {reason}"
                )
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
            SequenceRenderError::MissingParticipant {
                participant_id: participant_id.clone(),
            }
        })?;
        let name = participant.mermaid_name();
        let note = if options.show_notes {
            participant.note()
        } else {
            None
        };

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

    let width = participant_renders
        .last()
        .map(|p| p.box_x1 + 1 + RIGHT_MARGIN)
        .unwrap_or(1);

    let message_top_y = box_height + HEADER_GAP;
    let self_loop_rows = collect_self_loop_rows(layout);
    let row_y_by_row = build_message_row_positions(ast, layout, message_top_y, &self_loop_rows)?;
    let bottom_y =
        compute_sequence_bottom_y(ast, layout, &row_y_by_row, &self_loop_rows, box_height)?;
    let height = bottom_y + 1;

    let mut canvas = Canvas::new(width, height)?;

    let mut lifeline_x_by_col = BTreeMap::<usize, usize>::new();
    let next_lifeline_x_by_col = next_lifeline_x_by_col(&participant_renders);

    for p in &participant_renders {
        canvas.draw_box(p.box_x0, 0, p.box_x1, box_height - 1)?;

        let display_name = prefixed_object_label(p.name, options);
        let name_len = text_len(&display_name);
        let left_pad = (p.box_inner_width.saturating_sub(name_len)) / 2;
        let name_x = p.box_x0 + 1 + left_pad;
        canvas.write_str(name_x, 1, &display_name)?;

        if options.show_notes {
            if let Some(note) = p.note {
                let clipped = truncate_with_ellipsis(note, p.box_inner_width);
                let clipped_len = text_len(&clipped);
                let left_pad = (p.box_inner_width.saturating_sub(clipped_len)) / 2;
                let note_x = p.box_x0 + 1 + left_pad;
                canvas.write_str(note_x, 2, &clipped)?;
            }
        }

        canvas.draw_vline(p.lifeline_x, box_height, bottom_y)?;
        lifeline_x_by_col.insert(p.col, p.lifeline_x);
    }

    for msg_layout in layout.messages() {
        let msg = messages_by_id.get(msg_layout.message_id()).ok_or_else(|| {
            SequenceRenderError::MissingMessage {
                message_id: msg_layout.message_id().clone(),
            }
        })?;

        let from_x = *lifeline_x_by_col.get(&msg_layout.from_col()).ok_or(
            SequenceRenderError::InvalidParticipantColumn {
                col: msg_layout.from_col(),
            },
        )?;
        let to_x = *lifeline_x_by_col.get(&msg_layout.to_col()).ok_or(
            SequenceRenderError::InvalidParticipantColumn {
                col: msg_layout.to_col(),
            },
        )?;

        let y = row_y_for(msg_layout.row(), &row_y_by_row, message_top_y);
        let message_text = prefixed_object_label(msg.text(), options);
        let self_right_limit =
            self_message_right_limit(msg_layout.from_col(), &next_lifeline_x_by_col, width);
        draw_message(
            &mut canvas,
            from_x,
            to_x,
            y,
            msg.kind(),
            &message_text,
            self_right_limit,
        )?;
    }

    let overlays = draw_sequence_block_decorations(
        &mut canvas,
        ast,
        layout,
        &row_y_by_row,
        &self_loop_rows,
        width,
    )?;
    Ok(canvas_to_string_trimmed_with_overlays(&canvas, &overlays))
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
            SequenceRenderError::MissingParticipant {
                participant_id: participant_id.clone(),
            }
        })?;
        let name = participant.mermaid_name();
        let note = if options.show_notes {
            participant.note()
        } else {
            None
        };

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

    let width = participant_renders
        .last()
        .map(|p| p.box_x1 + 1 + RIGHT_MARGIN)
        .unwrap_or(1);

    let message_top_y = box_height + HEADER_GAP;
    let self_loop_rows = collect_self_loop_rows(layout);
    let row_y_by_row = build_message_row_positions(ast, layout, message_top_y, &self_loop_rows)?;
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
                    highlight_index.insert(
                        note_ref,
                        vec![(note_y, note_x, note_x + clipped_len.saturating_sub(1))],
                    );
                }
            }
        }
    }

    for msg_layout in layout.messages() {
        let from_x = *lifeline_x_by_col.get(&msg_layout.from_col()).ok_or(
            SequenceRenderError::InvalidParticipantColumn {
                col: msg_layout.from_col(),
            },
        )?;
        let to_x = *lifeline_x_by_col.get(&msg_layout.to_col()).ok_or(
            SequenceRenderError::InvalidParticipantColumn {
                col: msg_layout.to_col(),
            },
        )?;

        let y = row_y_for(msg_layout.row(), &row_y_by_row, message_top_y);
        let mut spans = Vec::<LineSpan>::new();
        if from_x == to_x {
            let self_right_limit =
                self_message_right_limit(msg_layout.from_col(), &next_lifeline_x_by_col, width);
            if let Some(stub_end) = self_message_stub_end(from_x, self_right_limit) {
                spans.push((y, from_x, stub_end));
                let y1 = y.saturating_add(SELF_MESSAGE_LOOP_DROP);
                spans.push((y1, stub_end, stub_end));
                spans.push((y1, from_x, stub_end));
            }
        } else if from_x < to_x {
            let arrow_head_x = to_x.saturating_sub(1);
            spans.push((y, from_x.min(arrow_head_x), from_x.max(arrow_head_x)));
        } else {
            let arrow_head_x = to_x + 1;
            spans.push((y, arrow_head_x.min(from_x), arrow_head_x.max(from_x)));
        }

        spans.sort();
        spans.dedup();

        let object_ref = ObjectRef::new(
            diagram_id.clone(),
            seq_message_category.clone(),
            msg_layout.message_id().clone(),
        );
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
    Ok(AnnotatedRender {
        text,
        highlight_index,
    })
}

// Extracted sequence rendering internals and block drawing helpers.
include!("sequence/helpers.rs");

#[cfg(test)]
mod tests;
