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

const GANTT_TICK_EVERY_DAYS: u32 = 7;

/// Resolved half-open day window used by Gantt layout and lane/reference derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GanttTaskWindow {
    pub(crate) start: u32,
    pub(crate) end: u32,
}

/// Gantt diagram content.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GanttAst {
    title: Option<String>,
    date_format: Option<String>,
    sections: Vec<GanttSection>,
    tasks: BTreeMap<ObjectId, GanttTask>,
    /// Notes on time-lane headers (`lane:2026-01-01`, or relative `lane:0000` when the
    /// chart has no valid absolute dates); sidecar-only, not Mermaid.
    lane_notes: BTreeMap<ObjectId, String>,
    /// Ephemeral TUI-only prefixes for rendered lane labels; never exported or persisted.
    lane_label_prefixes: BTreeMap<ObjectId, String>,
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

    pub(crate) fn set_lane_label_prefix(
        &mut self,
        lane_id: ObjectId,
        prefix: Option<impl Into<String>>,
    ) {
        match prefix {
            Some(prefix) => {
                self.lane_label_prefixes.insert(lane_id, prefix.into());
            }
            None => {
                self.lane_label_prefixes.remove(&lane_id);
            }
        }
    }

    /// Render-visible time lanes keyed by stable object ids.
    ///
    /// Lanes are derived from the same resolved task windows as the text renderer. Keeping the
    /// derivation on the AST gives persistence, xrefs, TUI lists, and MCP reads one definition of
    /// which lane references actually exist.
    pub fn lanes(&self) -> BTreeMap<ObjectId, String> {
        self.lanes_with_days().into_iter().map(|(lane_id, _, label)| (lane_id, label)).collect()
    }

    /// Render-visible lanes paired with their relative layout day.
    ///
    /// Calendar-backed ids encode the date instead of the chart-relative offset, so inserting an
    /// earlier task cannot silently retarget notes, selections, or cross-references.
    pub(crate) fn lanes_with_days(&self) -> Vec<(ObjectId, u32, String)> {
        let (windows, axis_labels) = self.resolved_task_windows();
        let max_end = windows.values().map(|window| window.end).max().unwrap_or(1).max(1);
        let origin = self.absolute_axis_origin();
        let mut lanes = axis_labels
            .into_iter()
            .filter(|(day, _)| *day < max_end)
            .map(|(day, label)| {
                let suffix = origin
                    .and_then(|ordinal| ordinal.checked_add(i64::from(day)))
                    .map(format_ymd_ordinal)
                    .unwrap_or_else(|| format!("{day:04}"));
                let lane_id = ObjectId::new(format!("lane:{suffix}")).expect("derived lane id");
                let label = self
                    .lane_label_prefixes
                    .get(&lane_id)
                    .map(|prefix| format!("{prefix}{label}"))
                    .unwrap_or(label);
                (lane_id, day, label)
            })
            .collect::<Vec<_>>();
        lanes.sort_by_key(|(_, day, _)| *day);
        if lanes.is_empty() {
            let lane_id = ObjectId::new("lane:0000").expect("static lane id");
            let label = self
                .lane_label_prefixes
                .get(&lane_id)
                .map(|prefix| format!("{prefix}d0"))
                .unwrap_or_else(|| "d0".to_owned());
            lanes.push((lane_id, 0, label));
        }
        lanes
    }

    pub(crate) fn lane_id_for_relative_day(&self, day: u32) -> Option<ObjectId> {
        self.lanes_with_days()
            .into_iter()
            .find_map(|(lane_id, lane_day, _)| (lane_day == day).then_some(lane_id))
    }

    pub(crate) fn ordered_task_ids(&self) -> Vec<ObjectId> {
        let mut ordered = Vec::new();
        for section in self.sections() {
            for task_id in section.task_ids() {
                if !ordered.contains(task_id) {
                    ordered.push(task_id.clone());
                }
            }
        }
        for task_id in self.tasks().keys() {
            if !ordered.contains(task_id) {
                ordered.push(task_id.clone());
            }
        }
        ordered
    }

    pub(crate) fn resolved_task_windows(
        &self,
    ) -> (BTreeMap<ObjectId, GanttTaskWindow>, BTreeMap<u32, String>) {
        let mut windows = BTreeMap::<ObjectId, GanttTaskWindow>::new();
        let mut task_end = BTreeMap::<ObjectId, u32>::new();
        let mut axis_labels = BTreeMap::<u32, String>::new();

        let mut date_to_day = BTreeMap::<String, u32>::new();
        let mut parsed_dates = Vec::<(String, i64)>::new();
        for task in self.tasks().values() {
            if let GanttTaskStart::Date(date) = task.start() {
                if let Some(ordinal) = parse_ymd_ordinal(date) {
                    parsed_dates.push((date.clone(), ordinal));
                }
            }
        }
        if let Some(min_ordinal) = parsed_dates.iter().map(|(_, ordinal)| *ordinal).min() {
            for (date, ordinal) in parsed_dates {
                let day = u32::try_from(ordinal - min_ordinal).unwrap_or(u32::MAX);
                date_to_day.entry(date.clone()).or_insert(day);
                axis_labels.entry(day).or_insert(date);
            }
        } else {
            let mut next_day = 0_u32;
            for task in self.tasks().values() {
                if let GanttTaskStart::Date(date) = task.start() {
                    date_to_day.entry(date.clone()).or_insert_with(|| {
                        let day = next_day;
                        next_day = next_day.saturating_add(GANTT_TICK_EVERY_DAYS);
                        day
                    });
                }
            }
            for (date, day) in &date_to_day {
                axis_labels.entry(*day).or_insert_with(|| date.clone());
            }
        }

        let ordered = self.ordered_task_ids();
        let max_passes = ordered.len().saturating_add(1).max(1);
        for _ in 0..max_passes {
            let mut changed = false;
            let mut cursor = 0_u32;
            for task_id in &ordered {
                let Some(task) = self.tasks().get(task_id) else {
                    continue;
                };
                let start = match task.start() {
                    GanttTaskStart::Date(date) => date_to_day.get(date).copied().unwrap_or(cursor),
                    GanttTaskStart::After(dependency_id) => {
                        task_end.get(dependency_id).copied().unwrap_or(cursor)
                    }
                    GanttTaskStart::Unspecified => cursor,
                };
                let end = start.saturating_add(task.duration_days().max(1));
                let window = GanttTaskWindow { start, end };
                if windows.get(task_id).copied() != Some(window) {
                    windows.insert(task_id.clone(), window);
                    changed = true;
                }
                task_end.insert(task_id.clone(), end);
                cursor = end;
            }
            if !changed {
                break;
            }
        }

        let max_end = windows.values().map(|window| window.end).max().unwrap_or(1).max(1);
        let origin = self.absolute_axis_origin();
        let mut day = 0_u32;
        while day < max_end {
            axis_labels.entry(day).or_insert_with(|| {
                origin
                    .and_then(|ordinal| ordinal.checked_add(i64::from(day)))
                    .map(format_ymd_ordinal)
                    .unwrap_or_else(|| format!("d{day}"))
            });
            day = day.saturating_add(GANTT_TICK_EVERY_DAYS);
        }

        (windows, axis_labels)
    }

    fn absolute_axis_origin(&self) -> Option<i64> {
        self.tasks()
            .values()
            .filter_map(|task| match task.start() {
                GanttTaskStart::Date(date) => parse_ymd_ordinal(date),
                GanttTaskStart::After(_) | GanttTaskStart::Unspecified => None,
            })
            .min()
    }
}

fn parse_ymd_ordinal(value: &str) -> Option<i64> {
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) {
        return None;
    }
    let max_day = days_in_month(year, month);
    if !(1..=max_day).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn is_leap_year(year: i32) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// Howard Hinnant's proleptic-Gregorian civil calendar conversion, shifted to Unix epoch.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn format_ymd_ordinal(ordinal: i64) -> String {
    let z = ordinal + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
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

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
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

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
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

#[cfg(test)]
mod tests {
    use super::{GanttAst, GanttTask, GanttTaskStart};
    use crate::model::ObjectId;

    fn add_dated_task(ast: &mut GanttAst, id: &str, date: &str, duration: u32) -> ObjectId {
        let id = ObjectId::new(id).unwrap();
        ast.tasks_mut().insert(
            id.clone(),
            GanttTask::new(
                id.clone(),
                id.as_str(),
                GanttTaskStart::Date(date.to_owned()),
                duration,
                format!("{duration}d"),
            ),
        );
        id
    }

    #[test]
    fn gregorian_windows_include_leap_day() {
        let mut ast = GanttAst::default();
        let before = add_dated_task(&mut ast, "t:before", "2024-02-28", 1);
        let after = add_dated_task(&mut ast, "t:after", "2024-03-01", 1);

        let (windows, _) = ast.resolved_task_windows();
        assert_eq!(windows[&before].start, 0);
        assert_eq!(windows[&after].start, 2);
        assert!(ast.lanes().contains_key(&ObjectId::new("lane:2024-03-01").unwrap()));
    }

    #[test]
    fn calendar_lane_ids_do_not_rebase_when_earlier_task_is_added() {
        let mut ast = GanttAst::default();
        add_dated_task(&mut ast, "t:existing", "2026-01-08", 8);
        let existing_lane = ObjectId::new("lane:2026-01-08").unwrap();
        ast.set_lane_note(existing_lane.clone(), Some("existing checkpoint"));
        assert!(ast.lanes().contains_key(&existing_lane));

        add_dated_task(&mut ast, "t:earlier", "2026-01-01", 1);

        assert!(ast.lanes().contains_key(&existing_lane));
        assert_eq!(ast.lane_note(&existing_lane), Some("existing checkpoint"));
        assert!(ast.lanes().contains_key(&ObjectId::new("lane:2026-01-01").unwrap()));
    }
}
