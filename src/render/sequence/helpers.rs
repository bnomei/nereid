// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

/// Sequence rendering internals:
/// row/column placement, block decoration, overlays, and text composition.
fn seq_box_height(options: RenderOptions) -> usize {
    if options.show_notes {
        BOX_HEIGHT_WITH_NOTES
    } else {
        BOX_HEIGHT_NO_NOTES
    }
}

fn participants_in_col_order(layout: &SequenceLayout) -> Vec<(usize, &ObjectId)> {
    let mut participants = layout
        .participant_cols()
        .iter()
        .map(|(id, col)| (*col, id))
        .collect::<Vec<_>>();
    participants
        .sort_by(|(a_col, a_id), (b_col, b_id)| a_col.cmp(b_col).then_with(|| a_id.cmp(b_id)));
    participants
}

fn next_lifeline_x_by_col(participant_renders: &[ParticipantRender<'_>]) -> BTreeMap<usize, usize> {
    let mut out = BTreeMap::<usize, usize>::new();
    for window in participant_renders.windows(2) {
        let current = window[0];
        let next = window[1];
        out.insert(current.col, next.lifeline_x);
    }
    out
}

fn self_message_right_limit(
    from_col: usize,
    next_lifeline_x_by_col: &BTreeMap<usize, usize>,
    canvas_width: usize,
) -> usize {
    let max_canvas_x = canvas_width.saturating_sub(1);
    match next_lifeline_x_by_col.get(&from_col).copied() {
        Some(next_lifeline_x) => next_lifeline_x.saturating_sub(1).min(max_canvas_x),
        None => max_canvas_x,
    }
}

fn self_message_stub_end(from_x: usize, right_limit: usize) -> Option<usize> {
    if right_limit <= from_x {
        return None;
    }

    let available = right_limit.saturating_sub(from_x);
    let preferred =
        available.saturating_mul(SELF_MESSAGE_STUB_FRACTION_NUM) / SELF_MESSAGE_STUB_FRACTION_DEN;
    let stub_len = preferred.max(SELF_MESSAGE_STUB_LEN).min(available);
    Some(from_x.saturating_add(stub_len))
}

fn messages_by_id(ast: &SequenceAst) -> BTreeMap<&ObjectId, &SequenceMessage> {
    ast.messages()
        .iter()
        .map(|m| (m.message_id(), m))
        .collect::<BTreeMap<_, _>>()
}

fn row_y_for(row: usize, row_y_by_row: &BTreeMap<usize, usize>, message_top_y: usize) -> usize {
    row_y_by_row
        .get(&row)
        .copied()
        .unwrap_or_else(|| message_top_y.saturating_add(row.saturating_mul(ROW_SPACING)))
}

fn collect_self_loop_rows(layout: &SequenceLayout) -> BTreeSet<usize> {
    layout
        .messages()
        .iter()
        .filter(|msg| msg.from_col() == msg.to_col())
        .map(|msg| msg.row())
        .collect::<BTreeSet<_>>()
}

fn message_row_map(layout: &SequenceLayout) -> BTreeMap<ObjectId, usize> {
    let mut message_row_by_id = BTreeMap::<ObjectId, usize>::new();
    for msg in layout.messages() {
        message_row_by_id.insert(msg.message_id().clone(), msg.row());
    }
    message_row_by_id
}

fn compute_sequence_bottom_y(
    ast: &SequenceAst,
    layout: &SequenceLayout,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
    box_height: usize,
) -> Result<usize, SequenceRenderError> {
    let Some(last_row) = layout.messages().iter().map(|m| m.row()).max() else {
        return Ok(box_height);
    };

    let mut bottom_y = row_y_for(last_row, row_y_by_row, 0).saturating_add(1);
    if !ast.blocks().is_empty() {
        let message_row_by_id = message_row_map(layout);
        if let Some(block_bottom) =
            max_block_bottom_y(ast, &message_row_by_id, row_y_by_row, self_loop_rows)?
        {
            bottom_y = bottom_y.max(block_bottom);
        }
    }

    Ok(bottom_y)
}

fn build_message_row_positions(
    ast: &SequenceAst,
    layout: &SequenceLayout,
    message_top_y: usize,
    self_loop_rows: &BTreeSet<usize>,
) -> Result<BTreeMap<usize, usize>, SequenceRenderError> {
    let mut row_y_by_row = BTreeMap::<usize, usize>::new();
    if layout.messages().is_empty() {
        return Ok(row_y_by_row);
    }

    let mut message_row_by_id = BTreeMap::<ObjectId, usize>::new();
    for msg in layout.messages() {
        message_row_by_id.insert(msg.message_id().clone(), msg.row());
    }

    let block_start_rows = collect_block_start_rows(ast, &message_row_by_id)?;
    let section_start_rows = collect_section_split_start_rows(ast, &message_row_by_id)?;
    let top_level_transition_rows =
        collect_top_level_block_transition_rows(ast, &message_row_by_id)?;
    let block_end_rows = collect_block_end_rows(ast, &message_row_by_id)?;
    let last_row = layout.messages().iter().map(|m| m.row()).max().unwrap_or(0);
    let mut post_block_row_extra = BTreeMap::<usize, usize>::new();
    for end_row in block_end_rows {
        let next_row = end_row.saturating_add(1);
        if next_row > last_row {
            continue;
        }
        if block_start_rows.contains(&next_row) || section_start_rows.contains(&next_row) {
            continue;
        }

        let extra = if self_loop_rows.contains(&end_row) {
            2
        } else {
            1
        };
        post_block_row_extra
            .entry(next_row)
            .and_modify(|current| *current = (*current).max(extra))
            .or_insert(extra);
    }
    let mut starts_after_self_loop = BTreeSet::<usize>::new();
    for row in block_start_rows.iter().chain(section_start_rows.iter()) {
        if *row > 0 && self_loop_rows.contains(&row.saturating_sub(1)) {
            starts_after_self_loop.insert(*row);
        }
    }

    let mut extra_rows = 0usize;
    for row in 0..=last_row {
        if row > 0 && block_start_rows.contains(&row) {
            extra_rows = extra_rows.saturating_add(BLOCK_TOP_ROW_EXTRA);
        }
        if row > 0 && section_start_rows.contains(&row) {
            extra_rows = extra_rows.saturating_add(SECTION_TOP_ROW_EXTRA);
        }
        if row > 0 && top_level_transition_rows.contains(&row) {
            extra_rows = extra_rows.saturating_add(TOP_LEVEL_BLOCK_TRANSITION_EXTRA);
        }
        if row > 0 && starts_after_self_loop.contains(&row) {
            extra_rows = extra_rows.saturating_add(1);
        }
        if let Some(extra) = post_block_row_extra.get(&row) {
            extra_rows = extra_rows.saturating_add(*extra);
        }
        let y = message_top_y
            .saturating_add(row.saturating_mul(ROW_SPACING))
            .saturating_add(extra_rows);
        row_y_by_row.insert(row, y);
    }

    Ok(row_y_by_row)
}

fn collect_block_start_rows(
    ast: &SequenceAst,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
) -> Result<BTreeSet<usize>, SequenceRenderError> {
    let mut starts = BTreeSet::<usize>::new();
    for block in ast.blocks() {
        collect_block_start_rows_from_block(block, message_row_by_id, &mut starts)?;
    }
    Ok(starts)
}

fn collect_section_split_start_rows(
    ast: &SequenceAst,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
) -> Result<BTreeSet<usize>, SequenceRenderError> {
    let mut starts = BTreeSet::<usize>::new();
    for block in ast.blocks() {
        collect_section_split_start_rows_from_block(block, message_row_by_id, &mut starts)?;
    }
    Ok(starts)
}

fn collect_block_start_rows_from_block(
    block: &SequenceBlock,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    starts: &mut BTreeSet<usize>,
) -> Result<(), SequenceRenderError> {
    let section_ranges = section_row_ranges(block, message_row_by_id)?;
    if let Some(start_row) = section_ranges.iter().map(|range| range.start_row).min() {
        starts.insert(start_row);
    }

    for nested in block.blocks() {
        collect_block_start_rows_from_block(nested, message_row_by_id, starts)?;
    }

    Ok(())
}

fn collect_section_split_start_rows_from_block(
    block: &SequenceBlock,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    starts: &mut BTreeSet<usize>,
) -> Result<(), SequenceRenderError> {
    let section_ranges = section_row_ranges(block, message_row_by_id)?;
    for section in section_ranges.into_iter().skip(1) {
        starts.insert(section.start_row);
    }

    for nested in block.blocks() {
        collect_section_split_start_rows_from_block(nested, message_row_by_id, starts)?;
    }

    Ok(())
}

fn collect_top_level_block_transition_rows(
    ast: &SequenceAst,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
) -> Result<BTreeSet<usize>, SequenceRenderError> {
    let mut ranges = Vec::<(usize, usize)>::new();
    for block in ast.blocks() {
        let section_ranges = section_row_ranges(block, message_row_by_id)?;
        let start = section_ranges
            .iter()
            .map(|range| range.start_row)
            .min()
            .unwrap_or(0);
        let end = section_ranges
            .iter()
            .map(|range| range.end_row)
            .max()
            .unwrap_or(0);
        ranges.push((start, end));
    }

    ranges.sort_by(|(a_start, a_end), (b_start, b_end)| {
        a_start.cmp(b_start).then_with(|| a_end.cmp(b_end))
    });

    let mut out = BTreeSet::<usize>::new();
    let mut prev_end = None::<usize>;
    for (start, end) in ranges {
        if let Some(last_end) = prev_end {
            if start == last_end.saturating_add(1) {
                out.insert(start);
            }
            prev_end = Some(last_end.max(end));
        } else {
            prev_end = Some(end);
        }
    }

    Ok(out)
}

fn max_block_bottom_y(
    ast: &SequenceAst,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
) -> Result<Option<usize>, SequenceRenderError> {
    let mut max_bottom = None::<usize>;
    for block in ast.blocks() {
        let bottom =
            max_block_bottom_y_for_block(block, message_row_by_id, row_y_by_row, self_loop_rows)?;
        max_bottom = Some(max_bottom.map_or(bottom, |prev| prev.max(bottom)));
    }
    Ok(max_bottom)
}

fn max_block_bottom_y_for_block(
    block: &SequenceBlock,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
) -> Result<usize, SequenceRenderError> {
    let section_ranges = section_row_ranges(block, message_row_by_id)?;
    let end_row = section_ranges
        .iter()
        .map(|range| range.end_row)
        .max()
        .unwrap_or(0);
    let mut max_bottom = block_bottom_y(end_row, row_y_by_row, self_loop_rows);

    for nested in block.blocks() {
        let nested_bottom =
            max_block_bottom_y_for_block(nested, message_row_by_id, row_y_by_row, self_loop_rows)?;
        max_bottom = max_bottom.max(nested_bottom);
    }

    Ok(max_bottom)
}

fn collect_block_end_rows(
    ast: &SequenceAst,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
) -> Result<BTreeSet<usize>, SequenceRenderError> {
    let mut ends = BTreeSet::<usize>::new();
    for block in ast.blocks() {
        collect_block_end_rows_from_block(block, message_row_by_id, &mut ends)?;
    }
    Ok(ends)
}

fn collect_block_end_rows_from_block(
    block: &SequenceBlock,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    ends: &mut BTreeSet<usize>,
) -> Result<(), SequenceRenderError> {
    let section_ranges = section_row_ranges(block, message_row_by_id)?;
    if let Some(end_row) = section_ranges.iter().map(|range| range.end_row).max() {
        ends.insert(end_row);
    }

    for nested in block.blocks() {
        collect_block_end_rows_from_block(nested, message_row_by_id, ends)?;
    }

    Ok(())
}

fn box_widths(name: &str) -> (usize, usize) {
    let name_len = text_len(name);
    let mut inner_width = (name_len + 2).max(MIN_BOX_INNER_WIDTH);
    let mut total_width = inner_width + 2;

    // Keep widths odd so lifelines fall on a true center cell.
    if total_width % 2 == 0 {
        total_width += 1;
        inner_width += 1;
    }

    (inner_width, total_width)
}

fn box_widths_prefixed(name: &str, options: RenderOptions) -> (usize, usize) {
    box_widths(&prefixed_object_label(name, options))
}

fn prefixed_object_label(label: &str, options: RenderOptions) -> String {
    if options.prefix_object_labels {
        format!("{OBJECT_LABEL_PREFIX}{label}")
    } else {
        label.to_owned()
    }
}

fn draw_message(
    canvas: &mut Canvas,
    from_x: usize,
    to_x: usize,
    y: usize,
    kind: SequenceMessageKind,
    text: &str,
    self_right_limit: usize,
) -> Result<(), SequenceRenderError> {
    if from_x == to_x {
        return draw_self_message(canvas, from_x, y, kind, text, self_right_limit);
    }

    let dir = if from_x < to_x {
        ArrowDir::Right
    } else {
        ArrowDir::Left
    };
    let head = arrow_head(kind, dir);

    match dir {
        ArrowDir::Right => {
            let arrow_head_x = to_x.saturating_sub(1);
            canvas.draw_hline(from_x, arrow_head_x, y)?;
            canvas.set(arrow_head_x, y, head)?;
            write_message_text(canvas, from_x + 1, arrow_head_x.saturating_sub(1), y, text)?;
        }
        ArrowDir::Left => {
            let arrow_head_x = to_x + 1;
            canvas.draw_hline(arrow_head_x, from_x, y)?;
            canvas.set(arrow_head_x, y, head)?;
            write_message_text(canvas, arrow_head_x + 1, from_x.saturating_sub(1), y, text)?;
        }
    }

    Ok(())
}

fn draw_self_message(
    canvas: &mut Canvas,
    from_x: usize,
    y: usize,
    kind: SequenceMessageKind,
    text: &str,
    right_limit: usize,
) -> Result<(), SequenceRenderError> {
    let max_canvas_x = canvas.width().saturating_sub(1);
    let right_limit = right_limit.min(max_canvas_x);
    let Some(stub_end) = self_message_stub_end(from_x, right_limit) else {
        return Ok(());
    };

    let y1 = y.saturating_add(SELF_MESSAGE_LOOP_DROP);
    if y1 >= canvas.height() {
        return Ok(());
    }

    // Top run with a real right corner.
    if stub_end > from_x {
        canvas.draw_hline(from_x, stub_end.saturating_sub(1), y)?;
    }
    canvas.set(stub_end, y, super::UNICODE_BOX_TOP_RIGHT)?;
    // Bottom run with a real right corner.
    if stub_end > from_x.saturating_add(1) {
        canvas.draw_hline(from_x.saturating_add(1), stub_end.saturating_sub(1), y1)?;
    }
    canvas.set(stub_end, y1, super::UNICODE_BOX_BOTTOM_RIGHT)?;
    canvas.set(from_x, y1, arrow_head(kind, ArrowDir::Left))?;

    // Keep one connector cell right before the top corner so long labels don't collapse `┐` to `│`.
    write_message_text(canvas, from_x + 1, stub_end.saturating_sub(2), y, text)?;
    Ok(())
}

fn arrow_head(kind: SequenceMessageKind, dir: ArrowDir) -> char {
    match kind {
        SequenceMessageKind::Sync => match dir {
            ArrowDir::Left => '◀',
            ArrowDir::Right => '▶',
        },
        SequenceMessageKind::Async | SequenceMessageKind::Return => match dir {
            ArrowDir::Left => '◁',
            ArrowDir::Right => '▷',
        },
    }
}

fn write_message_text(
    canvas: &mut Canvas,
    x0: usize,
    x1: usize,
    y: usize,
    text: &str,
) -> Result<(), CanvasError> {
    if x0 > x1 {
        return Ok(());
    }

    let available = (x1 - x0) + 1;
    if available == 0 {
        return Ok(());
    }

    let clipped = truncate_with_ellipsis(text, available);
    let clipped_len = text_len(&clipped);
    let offset = (available.saturating_sub(clipped_len)) / 2;
    canvas.write_str(x0 + offset, y, &clipped)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SectionRowRange<'a> {
    section: &'a SequenceSection,
    start_row: usize,
    end_row: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LabelOverlay {
    x: usize,
    y: usize,
    text: String,
}

fn draw_sequence_block_decorations(
    canvas: &mut Canvas,
    ast: &SequenceAst,
    layout: &SequenceLayout,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
    width: usize,
) -> Result<Vec<LabelOverlay>, SequenceRenderError> {
    if ast.blocks().is_empty() {
        return Ok(Vec::new());
    }

    let mut message_row_by_id = BTreeMap::<ObjectId, usize>::new();
    for msg in layout.messages() {
        message_row_by_id.insert(msg.message_id().clone(), msg.row());
    }

    let mut label_overlays = Vec::<LabelOverlay>::new();
    let mut block_layer = Canvas::new(canvas.width(), canvas.height())?;

    for block in ast.blocks() {
        draw_sequence_block(
            &mut block_layer,
            block,
            0,
            &message_row_by_id,
            row_y_by_row,
            self_loop_rows,
            width,
            &mut label_overlays,
        )?;
    }

    blend_low_priority_layer(canvas, &block_layer)?;
    Ok(label_overlays)
}

fn blend_low_priority_layer(base: &mut Canvas, overlay: &Canvas) -> Result<(), CanvasError> {
    for y in 0..base.height() {
        for x in 0..base.width() {
            let overlay_ch = overlay.get(x, y)?;
            if overlay_ch == ' ' {
                continue;
            }

            if base.get(x, y)? == ' ' {
                base.set(x, y, overlay_ch)?;
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn draw_sequence_block(
    canvas: &mut Canvas,
    block: &SequenceBlock,
    depth: usize,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
    width: usize,
    label_overlays: &mut Vec<LabelOverlay>,
) -> Result<(), SequenceRenderError> {
    let right_margin_inset = depth.saturating_mul(2);
    let left = right_margin_inset;
    let right = width.saturating_sub(1 + right_margin_inset);
    if right <= left {
        return Ok(());
    }

    let section_ranges = section_row_ranges(block, message_row_by_id)?;
    let mut block_start_row = section_ranges
        .iter()
        .map(|r| r.start_row)
        .min()
        .unwrap_or(0);
    let mut block_end_row = section_ranges.iter().map(|r| r.end_row).max().unwrap_or(0);

    // Defensive: clamp to the layout’s message rows.
    if let Some(max_row) = layout_max_row(message_row_by_id) {
        block_start_row = block_start_row.min(max_row);
        block_end_row = block_end_row.min(max_row);
    }

    let top_y = nested_block_top_y(
        block_start_row,
        row_y_by_row,
        depth,
        block_end_row,
        self_loop_rows,
    );
    let bottom_y = block_bottom_y(block_end_row, row_y_by_row, self_loop_rows);

    draw_frame_top(canvas, left, right, top_y)?;
    draw_frame_bottom(canvas, left, right, bottom_y)?;
    draw_frame_sides(canvas, left, right, top_y, bottom_y)?;

    let block_label = format_block_label(block);
    push_frame_label(label_overlays, left, right, top_y, &block_label);

    for section_range in section_ranges.iter().skip(1) {
        let y = section_separator_y(section_range.start_row, row_y_by_row);
        draw_frame_separator(canvas, left, right, y)?;
        if let Some(label) = format_section_split_label(section_range.section) {
            push_frame_label(label_overlays, left, right, y, &label);
        }
    }

    for nested in block.blocks() {
        draw_sequence_block(
            canvas,
            nested,
            depth.saturating_add(1),
            message_row_by_id,
            row_y_by_row,
            self_loop_rows,
            width,
            label_overlays,
        )?;
    }

    Ok(())
}

fn layout_max_row(message_row_by_id: &BTreeMap<ObjectId, usize>) -> Option<usize> {
    message_row_by_id.values().copied().max()
}

#[allow(clippy::too_many_arguments)]
fn insert_sequence_block_highlights(
    highlight_index: &mut HighlightIndex,
    diagram_id: &DiagramId,
    seq_block_category: &CategoryPath,
    seq_section_category: &CategoryPath,
    block: &SequenceBlock,
    depth: usize,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
    width: usize,
) -> Result<(), SequenceRenderError> {
    let right_margin_inset = depth.saturating_mul(2);
    let left = right_margin_inset;
    let right = width.saturating_sub(1 + right_margin_inset);
    if right <= left {
        return Ok(());
    }

    let section_ranges = section_row_ranges(block, message_row_by_id)?;
    let mut block_start_row = section_ranges
        .iter()
        .map(|r| r.start_row)
        .min()
        .unwrap_or(0);
    let mut block_end_row = section_ranges.iter().map(|r| r.end_row).max().unwrap_or(0);

    if let Some(max_row) = layout_max_row(message_row_by_id) {
        block_start_row = block_start_row.min(max_row);
        block_end_row = block_end_row.min(max_row);
    }

    let top_y = nested_block_top_y(
        block_start_row,
        row_y_by_row,
        depth,
        block_end_row,
        self_loop_rows,
    );
    let bottom_y = block_bottom_y(block_end_row, row_y_by_row, self_loop_rows);

    let mut block_spans = Vec::<LineSpan>::new();
    block_spans.push((top_y, left, right));
    block_spans.push((bottom_y, left, right));
    for y in top_y..=bottom_y {
        block_spans.push((y, left, left));
        block_spans.push((y, right, right));
    }
    for section_range in section_ranges.iter().skip(1) {
        let y = section_separator_y(section_range.start_row, row_y_by_row);
        block_spans.push((y, left, right));
    }
    block_spans.sort();
    block_spans.dedup();

    highlight_index.insert(
        ObjectRef::new(
            diagram_id.clone(),
            seq_block_category.clone(),
            block.block_id().clone(),
        ),
        block_spans,
    );

    for (idx, section_range) in section_ranges.iter().enumerate() {
        let section_start_y = if idx == 0 {
            top_y
        } else {
            section_separator_y(section_range.start_row, row_y_by_row)
        };
        let section_end_y = if idx + 1 < section_ranges.len() {
            section_separator_y(section_ranges[idx + 1].start_row, row_y_by_row)
        } else {
            bottom_y
        };

        let mut spans = Vec::<LineSpan>::new();
        spans.push((section_start_y, left, right));
        spans.push((section_end_y, left, right));
        for y in section_start_y..=section_end_y {
            spans.push((y, left, left));
            spans.push((y, right, right));
        }
        spans.sort();
        spans.dedup();

        highlight_index.insert(
            ObjectRef::new(
                diagram_id.clone(),
                seq_section_category.clone(),
                section_range.section.section_id().clone(),
            ),
            spans,
        );
    }

    for nested in block.blocks() {
        insert_sequence_block_highlights(
            highlight_index,
            diagram_id,
            seq_block_category,
            seq_section_category,
            nested,
            depth.saturating_add(1),
            message_row_by_id,
            row_y_by_row,
            self_loop_rows,
            width,
        )?;
    }

    Ok(())
}

fn nested_block_top_y(
    start_row: usize,
    row_y_by_row: &BTreeMap<usize, usize>,
    depth: usize,
    end_row: usize,
    self_loop_rows: &BTreeSet<usize>,
) -> usize {
    let top_y = block_top_y(start_row, row_y_by_row);
    if depth == 0 {
        top_y
    } else {
        let bottom_y = block_bottom_y(end_row, row_y_by_row, self_loop_rows);
        top_y.saturating_add(1).min(bottom_y)
    }
}

fn section_row_ranges<'a>(
    block: &'a SequenceBlock,
    message_row_by_id: &BTreeMap<ObjectId, usize>,
) -> Result<Vec<SectionRowRange<'a>>, SequenceRenderError> {
    if block.sections().is_empty() {
        return Err(SequenceRenderError::InvalidBlockMembership {
            block_id: block.block_id().clone(),
            reason: "has no sections".to_owned(),
        });
    }

    let mut ranges = Vec::<SectionRowRange<'a>>::with_capacity(block.sections().len());

    for section in block.sections() {
        if section.message_ids().is_empty() {
            return Err(SequenceRenderError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!("section {} is empty", section.section_id()),
            });
        }

        let mut min_row = None::<usize>;
        let mut max_row = None::<usize>;
        for message_id in section.message_ids() {
            let row = message_row_by_id.get(message_id).copied().ok_or_else(|| {
                SequenceRenderError::InvalidBlockMembership {
                    block_id: block.block_id().clone(),
                    reason: format!(
                        "section {} references missing message id {}",
                        section.section_id(),
                        message_id
                    ),
                }
            })?;

            min_row = Some(min_row.map_or(row, |prev| prev.min(row)));
            max_row = Some(max_row.map_or(row, |prev| prev.max(row)));
        }

        ranges.push(SectionRowRange {
            section,
            start_row: min_row.unwrap_or(0),
            end_row: max_row.unwrap_or(0),
        });
    }

    Ok(ranges)
}

fn block_top_y(start_row: usize, row_y_by_row: &BTreeMap<usize, usize>) -> usize {
    row_y_for(start_row, row_y_by_row, 0).saturating_sub(BLOCK_TOP_LABEL_OFFSET)
}

fn block_bottom_y(
    end_row: usize,
    row_y_by_row: &BTreeMap<usize, usize>,
    self_loop_rows: &BTreeSet<usize>,
) -> usize {
    let base = row_y_for(end_row, row_y_by_row, 0).saturating_add(1);
    if self_loop_rows.contains(&end_row) {
        base.saturating_add(1)
    } else {
        base
    }
}

fn section_separator_y(start_row: usize, row_y_by_row: &BTreeMap<usize, usize>) -> usize {
    row_y_for(start_row, row_y_by_row, 0).saturating_sub(SECTION_LABEL_OFFSET)
}

fn is_box_drawing_char(ch: char) -> bool {
    matches!(
        ch,
        super::UNICODE_BOX_HORIZONTAL
            | super::UNICODE_BOX_VERTICAL
            | super::UNICODE_BOX_TOP_LEFT
            | super::UNICODE_BOX_TOP_RIGHT
            | super::UNICODE_BOX_BOTTOM_LEFT
            | super::UNICODE_BOX_BOTTOM_RIGHT
            | super::UNICODE_BOX_TEE_RIGHT
            | super::UNICODE_BOX_TEE_LEFT
            | super::UNICODE_BOX_TEE_DOWN
            | super::UNICODE_BOX_TEE_UP
            | super::UNICODE_BOX_CROSS
    )
}

fn try_set_box_char(canvas: &mut Canvas, x: usize, y: usize, ch: char) -> Result<(), CanvasError> {
    let existing = canvas.get(x, y)?;
    if existing == ' ' || is_box_drawing_char(existing) {
        canvas.set(x, y, ch)?;
    }
    Ok(())
}

fn draw_frame_top(
    canvas: &mut Canvas,
    left: usize,
    right: usize,
    y: usize,
) -> Result<(), CanvasError> {
    for x in (left + 1)..right {
        try_set_box_char(canvas, x, y, super::UNICODE_BOX_HORIZONTAL)?;
    }
    try_set_box_char(canvas, left, y, super::UNICODE_BOX_TOP_LEFT)?;
    try_set_box_char(canvas, right, y, super::UNICODE_BOX_TOP_RIGHT)?;
    Ok(())
}

fn draw_frame_bottom(
    canvas: &mut Canvas,
    left: usize,
    right: usize,
    y: usize,
) -> Result<(), CanvasError> {
    for x in (left + 1)..right {
        try_set_box_char(canvas, x, y, super::UNICODE_BOX_HORIZONTAL)?;
    }
    try_set_box_char(canvas, left, y, super::UNICODE_BOX_BOTTOM_LEFT)?;
    try_set_box_char(canvas, right, y, super::UNICODE_BOX_BOTTOM_RIGHT)?;
    Ok(())
}

fn draw_frame_sides(
    canvas: &mut Canvas,
    left: usize,
    right: usize,
    top_y: usize,
    bottom_y: usize,
) -> Result<(), CanvasError> {
    if top_y.saturating_add(1) >= bottom_y {
        return Ok(());
    }

    for y in (top_y + 1)..bottom_y {
        try_set_box_char(canvas, left, y, super::UNICODE_BOX_VERTICAL)?;
        try_set_box_char(canvas, right, y, super::UNICODE_BOX_VERTICAL)?;
    }

    Ok(())
}

fn draw_frame_separator(
    canvas: &mut Canvas,
    left: usize,
    right: usize,
    y: usize,
) -> Result<(), CanvasError> {
    for x in (left + 1)..right {
        try_set_box_char(canvas, x, y, super::UNICODE_BOX_HORIZONTAL)?;
    }
    try_set_box_char(canvas, left, y, super::UNICODE_BOX_TEE_RIGHT)?;
    try_set_box_char(canvas, right, y, super::UNICODE_BOX_TEE_LEFT)?;
    Ok(())
}

fn push_frame_label(
    overlays: &mut Vec<LabelOverlay>,
    left: usize,
    right: usize,
    y: usize,
    label: &str,
) {
    if label.is_empty() {
        return;
    }

    let x0 = left.saturating_add(2);
    if x0 >= right {
        return;
    }

    let max_len = right.saturating_sub(x0);
    if max_len == 0 {
        return;
    }

    let clipped = truncate_with_ellipsis(label, max_len);
    overlays.push(LabelOverlay {
        x: x0,
        y,
        text: clipped,
    });
}

fn format_block_label(block: &SequenceBlock) -> String {
    let mut out = block_kind_keyword(block.kind()).to_owned();
    if let Some(header) = block.header() {
        let header = header.trim();
        if !header.is_empty() {
            out.push(' ');
            out.push_str(header);
        }
    }
    out
}

fn format_section_split_label(section: &SequenceSection) -> Option<String> {
    let keyword = match section.kind() {
        SequenceSectionKind::Main => return None,
        SequenceSectionKind::Else => "ELSE",
        SequenceSectionKind::And => "AND",
    };

    let mut out = keyword.to_owned();
    if let Some(header) = section.header() {
        let header = header.trim();
        if !header.is_empty() {
            out.push(' ');
            out.push_str(header);
        }
    }
    Some(out)
}

fn block_kind_keyword(kind: SequenceBlockKind) -> &'static str {
    match kind {
        SequenceBlockKind::Alt => "ALT",
        SequenceBlockKind::Opt => "OPT",
        SequenceBlockKind::Loop => "LOOP",
        SequenceBlockKind::Par => "PAR",
    }
}

fn canvas_to_string_trimmed_with_overlays(canvas: &Canvas, overlays: &[LabelOverlay]) -> String {
    let mut lines = Vec::<Vec<char>>::with_capacity(canvas.height());
    for y in 0..canvas.height() {
        let mut line = Vec::<char>::with_capacity(canvas.width());
        for x in 0..canvas.width() {
            let ch = canvas.get(x, y).expect("in bounds");
            line.push(ch);
        }
        lines.push(line);
    }

    for overlay in overlays {
        if overlay.y >= lines.len() {
            continue;
        }
        let line = &mut lines[overlay.y];
        let mut x = overlay.x;
        for ch in overlay.text.chars() {
            if x >= line.len() {
                break;
            }
            line[x] = ch;
            x += 1;
        }
    }

    let mut out = Vec::<String>::with_capacity(lines.len());
    for line in lines {
        let s: String = line.into_iter().collect();
        out.push(s.trim_end_matches(' ').to_owned());
    }

    while matches!(out.last(), Some(line) if line.is_empty()) {
        out.pop();
    }

    out.join("\n")
}
