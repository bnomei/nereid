// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Gantt chart AST: sections, tasks, dates/durations, and after-dependencies.

use std::collections::BTreeMap;

use super::ids::ObjectId;

/// Gantt diagram content.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GanttAst {
    title: Option<String>,
    date_format: Option<String>,
    sections: Vec<GanttSection>,
    tasks: BTreeMap<ObjectId, GanttTask>,
    /// Notes on time-lane headers (`lane:0000`, …); sidecar-only, not Mermaid.
    lane_notes: BTreeMap<ObjectId, String>,
}

impl GanttAst {
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn set_title(&mut self, title: Option<impl Into<String>>) {
        self.title = title.map(Into::into);
    }

    pub fn date_format(&self) -> Option<&str> {
        self.date_format.as_deref()
    }

    pub fn set_date_format(&mut self, date_format: Option<impl Into<String>>) {
        self.date_format = date_format.map(Into::into);
    }

    pub fn sections(&self) -> &[GanttSection] {
        &self.sections
    }

    pub fn sections_mut(&mut self) -> &mut Vec<GanttSection> {
        &mut self.sections
    }

    pub fn tasks(&self) -> &BTreeMap<ObjectId, GanttTask> {
        &self.tasks
    }

    pub fn tasks_mut(&mut self) -> &mut BTreeMap<ObjectId, GanttTask> {
        &mut self.tasks
    }

    pub fn lane_notes(&self) -> &BTreeMap<ObjectId, String> {
        &self.lane_notes
    }

    pub fn lane_notes_mut(&mut self) -> &mut BTreeMap<ObjectId, String> {
        &mut self.lane_notes
    }

    pub fn lane_note(&self, lane_id: &ObjectId) -> Option<&str> {
        self.lane_notes.get(lane_id).map(String::as_str)
    }

    pub fn set_lane_note(&mut self, lane_id: ObjectId, note: Option<impl Into<String>>) {
        match note {
            Some(n) => {
                self.lane_notes.insert(lane_id, n.into());
            }
            None => {
                self.lane_notes.remove(&lane_id);
            }
        }
    }
}

/// Named section grouping task rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GanttSection {
    section_id: ObjectId,
    name: String,
    task_ids: Vec<ObjectId>,
}

impl GanttSection {
    pub fn new(section_id: ObjectId, name: impl Into<String>) -> Self {
        Self { section_id, name: name.into(), task_ids: Vec::new() }
    }

    pub fn section_id(&self) -> &ObjectId {
        &self.section_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn task_ids(&self) -> &[ObjectId] {
        &self.task_ids
    }

    pub fn task_ids_mut(&mut self) -> &mut Vec<ObjectId> {
        &mut self.task_ids
    }
}

/// How a task start is specified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GanttTaskStart {
    /// Absolute date string in the diagram's dateFormat.
    Date(String),
    /// Start after another task ends.
    After(ObjectId),
    /// Unspecified (layout places after previous task in section).
    Unspecified,
}

/// A gantt task with duration and optional id for after-references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GanttTask {
    task_id: ObjectId,
    /// Mermaid task tag used in `after id` (e.g. `a1`).
    mermaid_tag: Option<String>,
    name: String,
    start: GanttTaskStart,
    /// Duration in days for layout (parsed from `30d`).
    duration_days: u32,
    /// Raw duration token for export (e.g. `30d`).
    raw_duration: String,
    /// Top-level note (sidecar / UI); not part of Mermaid export.
    note: Option<String>,
}

impl GanttTask {
    pub fn new(
        task_id: ObjectId,
        name: impl Into<String>,
        start: GanttTaskStart,
        duration_days: u32,
        raw_duration: impl Into<String>,
    ) -> Self {
        Self {
            task_id,
            mermaid_tag: None,
            name: name.into(),
            start,
            duration_days,
            raw_duration: raw_duration.into(),
            note: None,
        }
    }

    pub fn with_mermaid_tag(mut self, tag: Option<impl Into<String>>) -> Self {
        self.mermaid_tag = tag.map(Into::into);
        self
    }

    pub fn set_note<T: Into<String>>(&mut self, note: Option<T>) {
        self.note = note.map(Into::into);
    }

    pub fn task_id(&self) -> &ObjectId {
        &self.task_id
    }

    pub fn mermaid_tag(&self) -> Option<&str> {
        self.mermaid_tag.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn start(&self) -> &GanttTaskStart {
        &self.start
    }

    pub fn duration_days(&self) -> u32 {
        self.duration_days
    }

    pub fn raw_duration(&self) -> &str {
        &self.raw_duration
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}
