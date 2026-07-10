// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Track-family gantt bar painting (time columns + filled duration spans).

use std::collections::BTreeMap;

use crate::model::gantt_ast::{GanttAst, GanttTaskStart};
use crate::model::ids::ObjectId;
use crate::render::text::truncate_with_ellipsis;
use crate::render::{Canvas, CanvasError, RenderOptions};

const LABEL_COL_WIDTH: usize = 18;
const DAY_CHAR_WIDTH: usize = 1;
const BAR_FILL: char = '█';
const AXIS_TICK: char = '│';

/// Resolve task start day offsets (day 0 = earliest absolute date or 0).
fn resolve_task_windows(ast: &GanttAst) -> BTreeMap<ObjectId, (u32, u32)> {
    // start_day inclusive, end_day exclusive
    let mut windows = BTreeMap::<ObjectId, (u32, u32)>::new();
    let mut tag_end = BTreeMap::<ObjectId, u32>::new();

    // First pass: absolute dates as day indices by order of appearance among dated tasks.
    let mut date_to_day = BTreeMap::<String, u32>::new();
    let mut next_day = 0u32;
    for task in ast.tasks().values() {
        if let GanttTaskStart::Date(d) = task.start() {
            date_to_day.entry(d.clone()).or_insert_with(|| {
                let day = next_day;
                next_day = next_day.saturating_add(7); // space dated starts
                day
            });
        }
    }

    // Iterate in section order for stable after-resolution.
    let mut ordered = Vec::new();
    for section in ast.sections() {
        for tid in section.task_ids() {
            ordered.push(tid.clone());
        }
    }
    for tid in ast.tasks().keys() {
        if !ordered.contains(tid) {
            ordered.push(tid.clone());
        }
    }

    let mut cursor = 0u32;
    for tid in &ordered {
        let Some(task) = ast.tasks().get(tid) else {
            continue;
        };
        let start = match task.start() {
            GanttTaskStart::Date(d) => date_to_day.get(d).copied().unwrap_or(cursor),
            GanttTaskStart::After(dep) => tag_end.get(dep).copied().unwrap_or(cursor),
            GanttTaskStart::Unspecified => cursor,
        };
        let end = start.saturating_add(task.duration_days().max(1));
        windows.insert(tid.clone(), (start, end));
        tag_end.insert(tid.clone(), end);
        cursor = end;
    }
    windows
}

/// Render a gantt AST as Unicode bars.
pub fn render_gantt_unicode(
    ast: &GanttAst,
    _options: RenderOptions,
) -> Result<String, CanvasError> {
    let windows = resolve_task_windows(ast);
    let max_end = windows.values().map(|(_, e)| *e).max().unwrap_or(1).max(1);
    let chart_width = (max_end as usize).saturating_mul(DAY_CHAR_WIDTH).max(8);
    let width = LABEL_COL_WIDTH.saturating_add(1).saturating_add(chart_width).saturating_add(1);

    let mut rows: Vec<String> = Vec::new();
    if let Some(title) = ast.title() {
        rows.push(title.to_owned());
        rows.push(String::new());
    }

    for section in ast.sections() {
        rows.push(format!("▶ {}", section.name()));
        for tid in section.task_ids() {
            let Some(task) = ast.tasks().get(tid) else {
                continue;
            };
            let (start, end) = windows.get(tid).copied().unwrap_or((0, 1));
            let label = truncate_with_ellipsis(task.name(), LABEL_COL_WIDTH);
            let mut line = format!("{label:<LABEL_COL_WIDTH$}|");
            for day in 0..max_end {
                if day >= start && day < end {
                    line.push(BAR_FILL);
                } else {
                    line.push(' ');
                }
            }
            line.push(AXIS_TICK);
            rows.push(line);
        }
        rows.push(String::new());
    }

    // Trim trailing empty
    while matches!(rows.last(), Some(r) if r.is_empty()) {
        rows.pop();
    }

    // Build canvas for consistent API (optional) — join lines.
    let height = rows.len().max(1);
    let mut canvas = Canvas::new(width.max(1), height)?;
    for (y, row) in rows.iter().enumerate() {
        canvas.write_str(0, y, row)?;
    }
    use crate::render::text::canvas_to_string_trimmed;
    Ok(canvas_to_string_trimmed(&canvas))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::mermaid::parse_gantt_diagram;

    #[test]
    fn gantt_render_has_bars_and_sections() {
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
        assert!(text.contains("Section"), "{text}");
        assert!(text.contains(BAR_FILL), "{text}");
        assert!(text.contains("A task") || text.contains("A tas"), "{text}");
    }
}
