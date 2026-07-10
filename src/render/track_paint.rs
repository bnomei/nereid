// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Track-family paint: shared content boxes and gantt multi-column spans.
//!
//! Sequence and gantt both paint sequence-style content boxes (border + title + optional note).
//! Sequence nodes occupy a single lane column; gantt task nodes span multiple time columns.

use crate::model::gantt_ast::{GanttAst, GanttTaskWindow};
use crate::model::ids::ObjectId;
use crate::render::text::{canvas_to_string_trimmed, text_len, truncate_with_ellipsis};
use crate::render::{Canvas, CanvasError, RenderOptions};

/// Minimum inner width for a content box (matches sequence).
pub const TRACK_MIN_BOX_INNER_WIDTH: usize = 3;
/// Content box height without a note row.
pub const TRACK_BOX_HEIGHT_NO_NOTES: usize = 3;
/// Content box height with a note row (when notes are shown).
pub const TRACK_BOX_HEIGHT_WITH_NOTES: usize = 4;

/// Chars per gantt time-lane unit (one day). Sequence uses one lane per participant; gantt
/// stretches a task box across `duration` of these units.
const GANTT_UNIT_WIDTH: usize = 3;
const GANTT_LEFT_MARGIN: usize = 1;
const GANTT_RIGHT_MARGIN: usize = 2;
const GANTT_ROW_GAP: usize = 1;

/// Height of a track content box. When notes are enabled, reserves a note row for every box
/// (sequence layout depends on uniform header height across lanes).
pub fn track_content_box_height(options: RenderOptions) -> usize {
    if options.show_notes {
        TRACK_BOX_HEIGHT_WITH_NOTES
    } else {
        TRACK_BOX_HEIGHT_NO_NOTES
    }
}

/// Paint a sequence-style content box: borders, centered title, optional note row.
///
/// Used for sequence participant headers (1-lane) and gantt task bars (multi-lane span).
/// `x0..=x1` and `y0` define the box; height is derived from [`track_content_box_height`].
pub fn paint_track_content_box(
    canvas: &mut Canvas,
    x0: usize,
    y0: usize,
    x1: usize,
    title: &str,
    note: Option<&str>,
    options: RenderOptions,
) -> Result<usize, CanvasError> {
    let height = track_content_box_height(options);
    let y1 = y0.saturating_add(height.saturating_sub(1));
    if x1 < x0 {
        return Ok(height);
    }
    canvas.draw_box(x0, y0, x1, y1)?;

    let inner_width = x1.saturating_sub(x0).saturating_sub(1);
    if inner_width == 0 {
        return Ok(height);
    }

    let title_clipped = truncate_with_ellipsis(title, inner_width);
    let title_len = text_len(&title_clipped);
    let title_pad = (inner_width.saturating_sub(title_len)) / 2;
    canvas.write_str(x0 + 1 + title_pad, y0 + 1, &title_clipped)?;

    if options.show_notes {
        if let Some(note) = note {
            let clipped = truncate_with_ellipsis(note, inner_width);
            let clipped_len = text_len(&clipped);
            let note_pad = (inner_width.saturating_sub(clipped_len)) / 2;
            canvas.write_str(x0 + 1 + note_pad, y0 + 2, &clipped)?;
        }
    }

    Ok(height)
}

/// Placed content-box geometry shared by time-lane headers and multi-col task spans.
#[derive(Debug, Clone)]
struct GanttBoxPlacement {
    /// Stable object id (`t:…` task or `lane:…` time header).
    object_id: ObjectId,
    kind: GanttBoxKind,
    x0: usize,
    y0: usize,
    x1: usize,
    box_h: usize,
    title: String,
    note: Option<String>,
    /// Lifeline / tick x (center of lane header; for tasks unused).
    mid_x: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GanttBoxKind {
    /// Time column header — sequence participant analogue (1-lane label box).
    Lane,
    /// Named task group from the Gantt AST.
    Section,
    /// Task duration bar — multi-column content box.
    Task,
}

fn gantt_box_x_range(window: GanttTaskWindow, width: usize) -> (usize, usize) {
    let x0 =
        GANTT_LEFT_MARGIN.saturating_add((window.start as usize).saturating_mul(GANTT_UNIT_WIDTH));
    let mut x1 = GANTT_LEFT_MARGIN
        .saturating_add((window.end as usize).saturating_mul(GANTT_UNIT_WIDTH))
        .saturating_sub(1);
    let min_total = TRACK_MIN_BOX_INNER_WIDTH.saturating_add(2);
    if x1.saturating_sub(x0).saturating_add(1) < min_total {
        x1 = x0.saturating_add(min_total.saturating_sub(1)).min(width.saturating_sub(1));
    }
    x1 = x1.min(width.saturating_sub(1));
    (x0, x1)
}

/// Render a gantt AST as sequence-style track content boxes.
///
/// Layout mirrors sequence diagrams:
/// - **Lane headers** (top): content boxes for each time tick (like participant headers)
/// - **Lifelines**: verticals from each lane mid down through open body space
/// - **Task nodes**: multi-column content boxes (sequence is always 1-lane wide)
/// - No chart title / no left section chrome
pub fn render_gantt_unicode(ast: &GanttAst, options: RenderOptions) -> Result<String, CanvasError> {
    Ok(render_gantt_inner(ast, options)?.0)
}

/// Annotated gantt render: lane/task/note highlight spans for TUI F/C and note dimming.
pub fn render_gantt_unicode_annotated(
    diagram_id: &crate::model::ids::DiagramId,
    ast: &GanttAst,
    options: RenderOptions,
) -> Result<crate::render::AnnotatedRender, CanvasError> {
    use crate::model::{CategoryPath, ObjectRef};
    use crate::render::{clamp_highlight_index_to_text, AnnotatedRender, HighlightIndex, LineSpan};

    let (text, placements) = render_gantt_inner(ast, options)?;

    let lane_cat =
        CategoryPath::new(vec!["gantt".to_owned(), "lane".to_owned()]).expect("valid lane cat");
    let section_cat = CategoryPath::new(vec!["gantt".to_owned(), "section".to_owned()])
        .expect("valid section cat");
    let task_cat =
        CategoryPath::new(vec!["gantt".to_owned(), "task".to_owned()]).expect("valid task cat");
    let note_cat =
        CategoryPath::new(vec!["gantt".to_owned(), "note".to_owned()]).expect("valid note cat");

    let mut highlight_index = HighlightIndex::new();
    for p in &placements {
        let cat = match p.kind {
            GanttBoxKind::Lane => lane_cat.clone(),
            GanttBoxKind::Section => section_cat.clone(),
            GanttBoxKind::Task => task_cat.clone(),
        };
        let object_ref = ObjectRef::new(diagram_id.clone(), cat, p.object_id.clone());
        let mut spans = Vec::<LineSpan>::new();
        let y1 = p.y0.saturating_add(p.box_h.saturating_sub(1));
        for y in p.y0..=y1 {
            spans.push((y, p.x0, p.x1));
        }
        highlight_index.insert(object_ref, spans);

        if options.show_notes {
            if let Some(note) = p.note.as_deref() {
                let inner_width = p.x1.saturating_sub(p.x0).saturating_sub(1);
                let clipped = truncate_with_ellipsis(note, inner_width);
                let clipped_len = text_len(&clipped);
                if clipped_len > 0 {
                    let pad = (inner_width.saturating_sub(clipped_len)) / 2;
                    let note_x = p.x0.saturating_add(1).saturating_add(pad);
                    let note_y = p.y0.saturating_add(2);
                    let note_ref =
                        ObjectRef::new(diagram_id.clone(), note_cat.clone(), p.object_id.clone());
                    highlight_index.insert(
                        note_ref,
                        vec![(note_y, note_x, note_x + clipped_len.saturating_sub(1))],
                    );
                }
            }
        }
    }

    clamp_highlight_index_to_text(&mut highlight_index, &text);
    Ok(AnnotatedRender { text, highlight_index })
}

fn render_gantt_inner(
    ast: &GanttAst,
    options: RenderOptions,
) -> Result<(String, Vec<GanttBoxPlacement>), CanvasError> {
    let (windows, _) = ast.resolved_task_windows();
    let max_end = windows.values().map(|w| w.end).max().unwrap_or(1).max(1);

    // Chart geometry: each day is one lane unit of GANTT_UNIT_WIDTH chars.
    let chart_w = (max_end as usize).saturating_mul(GANTT_UNIT_WIDTH).max(GANTT_UNIT_WIDTH);
    let chart_width =
        GANTT_LEFT_MARGIN.saturating_add(chart_w).saturating_add(GANTT_RIGHT_MARGIN).max(1);
    let title_width = ast.title().map(text_len).unwrap_or(0);
    let section_width = ast
        .sections()
        .iter()
        .map(|section| GANTT_LEFT_MARGIN.saturating_add(2).saturating_add(text_len(section.name())))
        .max()
        .unwrap_or(0);
    let width = chart_width.max(title_width).max(section_width).max(1);

    // Sequence-style uniform content-box height for headers + task nodes.
    let box_h = track_content_box_height(options);
    // Match sequence HEADER_GAP (2) under participant boxes before body content.
    const HEADER_GAP: usize = 2;
    let title_rows = usize::from(ast.title().is_some_and(|title| !title.trim().is_empty())) * 2;
    let header_y0 = title_rows;
    let body_top = header_y0.saturating_add(box_h).saturating_add(HEADER_GAP);

    // Time-lane headers: one content box per axis tick spanning until the next tick.
    let lanes = ast.lanes_with_days();

    let mut placements = Vec::new();
    for (i, (lane_id, day, lane_label)) in lanes.iter().enumerate() {
        let next = lanes.get(i + 1).map(|(_, day, _)| *day).unwrap_or(max_end);
        let (x0, x1) =
            gantt_box_x_range(GanttTaskWindow { start: *day, end: next.max(day + 1) }, width);
        let label = shorten_axis_label(lane_label);
        let mid_x = x0.saturating_add(x1.saturating_sub(x0) / 2);
        let lane_note =
            if options.show_notes { ast.lane_note(lane_id).map(str::to_owned) } else { None };
        placements.push(GanttBoxPlacement {
            object_id: lane_id.clone(),
            kind: GanttBoxKind::Lane,
            x0,
            y0: header_y0,
            x1,
            box_h,
            title: label,
            // Sequence-style: note row inside the lane header box when present.
            note: lane_note,
            mid_x,
        });
    }

    // Section labels and task multi-col boxes under the header gap.
    let mut y = body_top;
    let mut placed_task_ids = std::collections::BTreeSet::new();
    for section in ast.sections() {
        let section_title = format!("§ {}", section.name());
        let section_x1 = GANTT_LEFT_MARGIN
            .saturating_add(text_len(&section_title).saturating_sub(1))
            .min(width.saturating_sub(1));
        placements.push(GanttBoxPlacement {
            object_id: section.section_id().clone(),
            kind: GanttBoxKind::Section,
            x0: GANTT_LEFT_MARGIN,
            y0: y,
            x1: section_x1,
            box_h: 1,
            title: section_title,
            note: None,
            mid_x: GANTT_LEFT_MARGIN,
        });
        y = y.saturating_add(2);
        for tid in section.task_ids() {
            if !placed_task_ids.insert(tid.clone()) {
                continue;
            }
            let Some(task) = ast.tasks().get(tid) else {
                continue;
            };
            let window = windows.get(tid).copied().unwrap_or(GanttTaskWindow { start: 0, end: 1 });
            let (x0, x1) = gantt_box_x_range(window, width);
            placements.push(GanttBoxPlacement {
                object_id: tid.clone(),
                kind: GanttBoxKind::Task,
                x0,
                y0: y,
                x1,
                box_h,
                title: task.name().to_owned(),
                note: if options.show_notes { task.note().map(str::to_owned) } else { None },
                mid_x: x0.saturating_add(x1.saturating_sub(x0) / 2),
            });
            y = y.saturating_add(box_h).saturating_add(GANTT_ROW_GAP);
        }
    }
    // Keep malformed/programmatic ASTs inspectable even though export rejects unsectioned tasks.
    for tid in ast.ordered_task_ids() {
        if !placed_task_ids.insert(tid.clone()) {
            continue;
        }
        let Some(task) = ast.tasks().get(&tid) else {
            continue;
        };
        let window = windows.get(&tid).copied().unwrap_or(GanttTaskWindow { start: 0, end: 1 });
        let (x0, x1) = gantt_box_x_range(window, width);
        placements.push(GanttBoxPlacement {
            object_id: tid.clone(),
            kind: GanttBoxKind::Task,
            x0,
            y0: y,
            x1,
            box_h,
            title: task.name().to_owned(),
            note: if options.show_notes { task.note().map(str::to_owned) } else { None },
            mid_x: x0.saturating_add(x1.saturating_sub(x0) / 2),
        });
        y = y.saturating_add(box_h).saturating_add(GANTT_ROW_GAP);
    }
    let height = y.saturating_sub(GANTT_ROW_GAP).max(header_y0.saturating_add(box_h)).max(1);

    let mut canvas = Canvas::new(width, height)?;

    if let Some(title) = ast.title().filter(|title| !title.trim().is_empty()) {
        let clipped = truncate_with_ellipsis(title, width);
        let title_x = width.saturating_sub(text_len(&clipped)) / 2;
        canvas.write_str(title_x, 0, &clipped)?;
    }

    // Paint lane headers then task boxes (sequence paints headers first).
    for p in placements.iter().filter(|p| p.kind == GanttBoxKind::Lane) {
        paint_track_content_box(
            &mut canvas,
            p.x0,
            p.y0,
            p.x1,
            &p.title,
            p.note.as_deref(),
            options,
        )?;
    }
    for p in placements.iter().filter(|p| p.kind == GanttBoxKind::Task) {
        paint_track_content_box(
            &mut canvas,
            p.x0,
            p.y0,
            p.x1,
            &p.title,
            p.note.as_deref(),
            options,
        )?;
    }
    for p in placements.iter().filter(|p| p.kind == GanttBoxKind::Section) {
        canvas.write_str(p.x0, p.y0, &p.title)?;
    }

    // Lifelines from under each lane header through open body cells (sequence verticals).
    // Never paint inside any box (interiors are spaces, so a plain emptiness check is wrong).
    let life_top = header_y0.saturating_add(box_h);
    for p in placements.iter().filter(|p| p.kind == GanttBoxKind::Lane) {
        let x = p.mid_x;
        if x >= width {
            continue;
        }
        for yy in life_top..height {
            if cell_in_any_box(&placements, x, yy) {
                continue;
            }
            if canvas.get(x, yy)? == ' ' {
                canvas.set(x, yy, '│')?;
            }
        }
    }

    Ok((canvas_to_string_trimmed(&canvas), placements))
}

fn cell_in_any_box(placements: &[GanttBoxPlacement], x: usize, y: usize) -> bool {
    placements.iter().filter(|p| p.kind != GanttBoxKind::Section).any(|p| {
        let y1 = p.y0.saturating_add(p.box_h.saturating_sub(1));
        x >= p.x0 && x <= p.x1 && y >= p.y0 && y <= y1
    })
}

fn shorten_axis_label(label: &str) -> String {
    // "2014-01-05" → "01-05"; leave other labels as-is.
    let parts: Vec<&str> = label.split('-').collect();
    if parts.len() == 3 && parts[0].len() == 4 {
        return format!("{}-{}", parts[1], parts[2]);
    }
    label.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::mermaid::parse_gantt_diagram;
    use crate::model::{GanttAst, GanttTask, GanttTaskStart, ObjectId};

    #[test]
    fn gantt_render_uses_boxes_not_fill_blocks() {
        let input = r#"
gantt
title A Gantt Diagram
dateFormat YYYY-MM-DD
section Section
A task :a1, 2014-01-01, 30d
Another task :after a1, 20d
section Another
Task in Another :2014-01-12, 12d
another task :24d
"#;
        let ast = parse_gantt_diagram(input).expect("parse");
        let text = render_gantt_unicode(&ast, RenderOptions::default()).expect("render");

        assert!(text.contains("A Gantt Diagram"), "{text}");
        assert!(text.contains("§ Section"), "{text}");
        assert!(text.contains("§ Another"), "{text}");
        assert!(!text.contains('█'), "solid bar fill should be gone:\n{text}");

        // Sequence-style boxes for lane headers and tasks.
        assert!(text.contains('┌'), "expected box corners:\n{text}");
        assert!(text.contains('└'), "expected box corners:\n{text}");
        assert!(text.contains("A task") || text.contains("A tas"), "{text}");
        assert!(text.contains("Another task") || text.contains("Another"), "{text}");

        // Lane headers are boxed at the top (sequence participant analogue).
        let first = text.lines().nth(2).unwrap_or("");
        assert!(
            first.contains('┌'),
            "expected sequence-style lane header boxes on top row:\n{text}"
        );
        assert!(
            text.contains("01-") || text.contains("d0") || text.chars().any(|c| c.is_ascii_digit()),
            "expected time labels inside lane boxes:\n{text}"
        );

        // Lifelines under lane headers (through open chart space).
        assert!(
            text.chars().any(|c| c == '│'),
            "expected vertical lifelines under lane headers:\n{text}"
        );
        // Box corners must stay pure (verticals must not merge into walls).
        assert!(
            !text.contains('┼') && !text.contains('├') && !text.contains('┤'),
            "vertical ticks must not corrupt box walls:\n{text}"
        );
        // Lifelines must not run through the interior of a task title cell.
        for line in text.lines() {
            if let Some(idx) = line.find("A task") {
                let after = &line[idx..];
                let end = after.find('│').unwrap_or(after.len());
                let interior = &after[..end];
                assert!(
                    !interior[1.min(interior.len())..].contains('│'),
                    "lifeline inside task title interior:\n{line}\nfull:\n{text}"
                );
            }
        }
    }

    #[test]
    fn gantt_short_chart_keeps_full_title_and_section_labels() {
        let ast = parse_gantt_diagram(
            "gantt\ntitle Release Readiness\nsection Final Verification\nShip :ship, 2026-01-01, 1d\n",
        )
        .expect("parse");
        let text = render_gantt_unicode(&ast, RenderOptions::default()).expect("render");
        assert!(text.contains("Release Readiness"), "{text}");
        assert!(text.contains("§ Final Verification"), "{text}");
    }

    #[test]
    fn gantt_annotated_indexes_lane_and_task_refs() {
        use crate::model::ids::DiagramId;

        let input = r#"
gantt
dateFormat YYYY-MM-DD
section S
A task :a1, 2014-01-01, 14d
"#;
        let ast = parse_gantt_diagram(input).expect("parse");
        let diagram_id = DiagramId::new("demo-gantt").unwrap();
        let annotated = render_gantt_unicode_annotated(&diagram_id, &ast, RenderOptions::default())
            .expect("render");
        let keys: Vec<_> = annotated.highlight_index.keys().map(|r| r.to_string()).collect();
        assert!(keys.iter().any(|k| k.contains("/gantt/lane/")), "expected lane refs: {keys:?}");
        assert!(
            keys.iter().any(|k| k.contains("/gantt/section/")),
            "expected section refs: {keys:?}"
        );
        assert!(keys.iter().any(|k| k.contains("/gantt/task/")), "expected task refs: {keys:?}");
    }

    #[test]
    fn gantt_annotated_indexes_task_and_note_spans() {
        use crate::model::ids::DiagramId;

        let mut ast = GanttAst::default();
        let id = ObjectId::new("t:0001").unwrap();
        let mut task = GanttTask::new(
            id.clone(),
            "Ship",
            GanttTaskStart::Date("2014-01-01".into()),
            14,
            "14d",
        );
        task.set_note(Some("critical path"));
        ast.tasks_mut().insert(id.clone(), task);

        let diagram_id = DiagramId::new("demo-gantt").unwrap();
        let annotated = render_gantt_unicode_annotated(
            &diagram_id,
            &ast,
            RenderOptions { show_notes: true, ..Default::default() },
        )
        .expect("render");

        let task_key = format!("d:demo-gantt/gantt/task/{id}");
        let note_key = format!("d:demo-gantt/gantt/note/{id}");
        assert!(
            annotated.highlight_index.keys().any(|r| r.to_string() == task_key),
            "missing task ref in {:?}",
            annotated.highlight_index.keys().map(|r| r.to_string()).collect::<Vec<_>>()
        );
        assert!(
            annotated.highlight_index.keys().any(|r| r.to_string() == note_key),
            "missing note ref in {:?}",
            annotated.highlight_index.keys().map(|r| r.to_string()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn gantt_task_note_paints_inside_box_when_enabled() {
        let mut ast = GanttAst::default();
        let id = ObjectId::new("t:0001").unwrap();
        let mut task = GanttTask::new(
            id.clone(),
            "Ship",
            GanttTaskStart::Date("2014-01-01".into()),
            14,
            "14d",
        );
        task.set_note(Some("critical path"));
        ast.tasks_mut().insert(id, task);

        let off =
            render_gantt_unicode(&ast, RenderOptions { show_notes: false, ..Default::default() })
                .expect("render");
        assert!(!off.contains("critical path"), "{off}");

        let on =
            render_gantt_unicode(&ast, RenderOptions { show_notes: true, ..Default::default() })
                .expect("render");
        assert!(on.contains("critical path"), "{on}");
        assert!(on.contains("Ship"), "{on}");
    }

    #[test]
    fn gantt_lane_note_paints_inside_header_box_when_enabled() {
        let mut ast = GanttAst::default();
        let id = ObjectId::new("t:0001").unwrap();
        ast.tasks_mut().insert(
            id.clone(),
            GanttTask::new(id, "Ship", GanttTaskStart::Date("2014-01-01".into()), 21, "21d"),
        );
        let lane0 = ObjectId::new("lane:2014-01-01").unwrap();
        ast.set_lane_note(lane0, Some("kickoff week"));

        let off =
            render_gantt_unicode(&ast, RenderOptions { show_notes: false, ..Default::default() })
                .expect("render");
        assert!(!off.contains("kickoff week"), "{off}");

        let on =
            render_gantt_unicode(&ast, RenderOptions { show_notes: true, ..Default::default() })
                .expect("render");
        assert!(on.contains("kickoff week"), "lane note missing from header:\n{on}");
        // Header is a box (not bare text).
        assert!(on.lines().next().unwrap_or("").contains('┌'), "{on}");
    }

    #[test]
    fn paint_track_content_box_matches_sequence_geometry() {
        let mut canvas = Canvas::new(12, 4).unwrap();
        let h = paint_track_content_box(
            &mut canvas,
            1,
            0,
            10,
            "Alice",
            Some("note"),
            RenderOptions { show_notes: true, ..Default::default() },
        )
        .unwrap();
        assert_eq!(h, TRACK_BOX_HEIGHT_WITH_NOTES);
        let text = canvas.to_string();
        assert!(text.contains("Alice"), "{text}");
        assert!(text.contains("note"), "{text}");
        assert!(text.contains('┌'), "{text}");
    }

    #[test]
    fn multi_day_task_spans_multiple_lane_units() {
        let mut ast = GanttAst::default();
        let id = ObjectId::new("t:0001").unwrap();
        ast.tasks_mut().insert(
            id.clone(),
            GanttTask::new(id, "Long", GanttTaskStart::Date("2014-01-01".into()), 10, "10d"),
        );
        let text = render_gantt_unicode(&ast, RenderOptions::default()).expect("render");
        // Box width should exceed single unit: look for a reasonably wide top border.
        let top = text.lines().find(|l| l.contains('┌')).expect("box top");
        let border_len = top.chars().filter(|&c| c == '─').count();
        assert!(
            border_len >= GANTT_UNIT_WIDTH.saturating_mul(2),
            "expected multi-col span, got border_len={border_len}:\n{text}"
        );
    }
}
