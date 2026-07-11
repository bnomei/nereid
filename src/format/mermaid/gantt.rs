// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Limited `gantt` parse/export for Nereid's gantt AST.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::model::ids::ObjectId;
use crate::model::{GanttAst, GanttSection, GanttTask, GanttTaskStart};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidGanttParseError {
    MissingHeader,
    EmptyInput,
    UnsupportedLine { line_no: usize, line: String },
    InvalidTask { line_no: usize, line: String },
    UnknownAfterTag { line_no: usize, tag: String },
    DuplicateMermaidTag { line_no: usize, tag: String },
}

impl fmt::Display for MermaidGanttParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => f.write_str("expected 'gantt' as the first non-empty line"),
            Self::EmptyInput => f.write_str("empty gantt diagram input"),
            Self::UnsupportedLine { line_no, line } => {
                write!(f, "unsupported gantt line {line_no}: {line}")
            }
            Self::InvalidTask { line_no, line } => {
                write!(f, "invalid gantt task on line {line_no}: {line}")
            }
            Self::UnknownAfterTag { line_no, tag } => {
                write!(f, "unknown after tag '{tag}' on line {line_no}")
            }
            Self::DuplicateMermaidTag { line_no, tag } => {
                write!(f, "gantt Mermaid task tag is not unique: {tag} (line {line_no})")
            }
        }
    }
}

impl std::error::Error for MermaidGanttParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidGanttExportError {
    EmptyTaskName {
        task_id: ObjectId,
    },
    MissingSectionTask {
        section_id: ObjectId,
        task_id: ObjectId,
    },
    DuplicateTaskMembership {
        task_id: ObjectId,
        first_section_id: ObjectId,
        second_section_id: ObjectId,
    },
    UnsectionedTask {
        task_id: ObjectId,
    },
    TaskIdMismatch {
        map_id: ObjectId,
        task_id: ObjectId,
    },
    MissingDependency {
        task_id: ObjectId,
        dependency_id: ObjectId,
    },
    DuplicateMermaidTag {
        tag: String,
    },
}

impl fmt::Display for MermaidGanttExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTaskName { task_id } => write!(f, "task {task_id} has an empty name"),
            Self::MissingSectionTask { section_id, task_id } => {
                write!(f, "gantt section {section_id} references missing task {task_id}")
            }
            Self::DuplicateTaskMembership {
                task_id,
                first_section_id,
                second_section_id,
            } => write!(
                f,
                "gantt task {task_id} belongs to both sections {first_section_id} and {second_section_id}"
            ),
            Self::UnsectionedTask { task_id } => {
                write!(f, "gantt task {task_id} does not belong to a section")
            }
            Self::TaskIdMismatch { map_id, task_id } => {
                write!(f, "gantt task map id {map_id} does not match embedded id {task_id}")
            }
            Self::MissingDependency { task_id, dependency_id } => {
                write!(f, "gantt task {task_id} depends on missing task {dependency_id}")
            }
            Self::DuplicateMermaidTag { tag } => {
                write!(f, "gantt Mermaid task tag is not unique: {tag}")
            }
        }
    }
}

impl std::error::Error for MermaidGanttExportError {}

fn parse_duration_days(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    // Require an explicit `d`/`D` suffix so bare integers remain available as mermaid tags
    // (e.g. `Task :1, 2014-01-01, 30d` keeps tag `1` and duration `30d`).
    let num = raw.strip_suffix('d').or_else(|| raw.strip_suffix('D'))?;
    num.parse().ok()
}

fn looks_like_date(s: &str) -> bool {
    // Require YYYY-MM-DD so tags like `task-1` / `a-1` are not misclassified as dates.
    let mut parts = s.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    y.len() == 4
        && y.chars().all(|c| c.is_ascii_digit())
        && m.len() == 2
        && m.chars().all(|c| c.is_ascii_digit())
        && d.len() == 2
        && d.chars().all(|c| c.is_ascii_digit())
}

/// Parse limited gantt subset (title, dateFormat, section, tasks with date/after/duration).
pub fn parse_gantt_diagram(input: &str) -> Result<GanttAst, MermaidGanttParseError> {
    let mut lines = input.lines().enumerate().filter(|(_, l)| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with("%%")
    });

    let Some((_, first)) = lines.next() else {
        return Err(MermaidGanttParseError::EmptyInput);
    };
    if first.trim() != "gantt" {
        return Err(MermaidGanttParseError::MissingHeader);
    }

    let mut ast = GanttAst::default();
    let mut current_section: Option<usize> = None;
    let mut tag_to_id: BTreeMap<String, ObjectId> = BTreeMap::new();
    let mut task_seq = 0u32;
    let mut section_seq = 0u32;
    // (task_id, after_tag, line_no)
    let mut pending_after: Vec<(ObjectId, String, usize)> = Vec::new();

    for (idx, line) in lines {
        let line_no = idx + 1;
        let trimmed = line.trim();

        if let Some(rest) = trimmed.strip_prefix("title ") {
            ast.set_title(Some(rest.trim()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("dateFormat ") {
            ast.set_date_format(Some(rest.trim()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("section ") {
            section_seq = section_seq.saturating_add(1);
            let sid = ObjectId::new(format!("sec:{section_seq:04}")).expect("section id");
            ast.sections_mut().push(GanttSection::new(sid, rest.trim()));
            current_section = Some(ast.sections().len() - 1);
            continue;
        }

        let Some((name_part, meta_part)) = trimmed.split_once(':') else {
            return Err(MermaidGanttParseError::UnsupportedLine {
                line_no,
                line: trimmed.to_owned(),
            });
        };
        let name = name_part.trim();
        if name.is_empty() {
            return Err(MermaidGanttParseError::InvalidTask { line_no, line: trimmed.to_owned() });
        }

        let parts: Vec<&str> =
            meta_part.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err(MermaidGanttParseError::InvalidTask { line_no, line: trimmed.to_owned() });
        }

        let mut mermaid_tag: Option<String> = None;
        let mut start = GanttTaskStart::Unspecified;
        let mut after_tag: Option<String> = None;
        let mut duration_raw = "1d".to_owned();
        let mut duration_days = 1u32;

        for (i, p) in parts.iter().enumerate() {
            if let Some(days) = parse_duration_days(p) {
                duration_raw = (*p).to_owned();
                duration_days = days;
                continue;
            }
            if let Some(tag) = p.strip_prefix("after ") {
                after_tag = Some(tag.trim().to_owned());
                continue;
            }
            if let Some(tag) = p.strip_prefix("after") {
                let tag = tag.trim();
                if !tag.is_empty() {
                    after_tag = Some(tag.to_owned());
                    continue;
                }
            }
            if looks_like_date(p) {
                start = GanttTaskStart::Date((*p).to_owned());
                continue;
            }
            // First non-date, non-duration token is the task id tag.
            if i == 0 && mermaid_tag.is_none() {
                mermaid_tag = Some((*p).to_owned());
            } else if matches!(start, GanttTaskStart::Unspecified) {
                start = GanttTaskStart::Date((*p).to_owned());
            }
        }

        task_seq = task_seq.saturating_add(1);
        let task_id = ObjectId::new(format!("t:{task_seq:04}")).expect("task id");

        if let Some(tag) = after_tag {
            pending_after.push((task_id.clone(), tag, line_no));
            start = GanttTaskStart::Unspecified; // filled after resolve
        }

        if let Some(tag) = mermaid_tag.as_ref() {
            if tag_to_id.contains_key(tag) {
                return Err(MermaidGanttParseError::DuplicateMermaidTag {
                    line_no,
                    tag: tag.clone(),
                });
            }
            tag_to_id.insert(tag.clone(), task_id.clone());
        }

        let task = GanttTask::new(task_id.clone(), name, start, duration_days, duration_raw)
            .with_mermaid_tag(mermaid_tag);
        ast.tasks_mut().insert(task_id.clone(), task);

        if current_section.is_none() {
            section_seq = section_seq.max(1);
            if ast.sections().is_empty() {
                let sid = ObjectId::new("sec:0001").expect("section id");
                ast.sections_mut().push(GanttSection::new(sid, "Tasks"));
            }
            current_section = Some(0);
        }
        if let Some(sec_idx) = current_section {
            ast.sections_mut()[sec_idx].task_ids_mut().push(task_id);
        }
    }

    for (task_id, tag, line_no) in pending_after {
        let Some(dep_id) = tag_to_id.get(&tag).cloned() else {
            return Err(MermaidGanttParseError::UnknownAfterTag { line_no, tag });
        };
        if let Some(task) = ast.tasks_mut().get_mut(&task_id) {
            let note = task.note().map(str::to_owned);
            let updated = GanttTask::new(
                task.task_id().clone(),
                task.name(),
                GanttTaskStart::After(dep_id),
                task.duration_days(),
                task.raw_duration(),
            )
            .with_mermaid_tag(task.mermaid_tag().map(str::to_owned));
            *task = updated;
            if let Some(note) = note {
                ast.tasks_mut().get_mut(&task_id).expect("task just updated").set_note(Some(note));
            }
        }
    }

    Ok(ast)
}

/// Export gantt diagram to Mermaid.
pub fn export_gantt_diagram(ast: &GanttAst) -> Result<String, MermaidGanttExportError> {
    let mut membership = BTreeMap::<ObjectId, ObjectId>::new();
    for section in ast.sections() {
        for task_id in section.task_ids() {
            if !ast.tasks().contains_key(task_id) {
                return Err(MermaidGanttExportError::MissingSectionTask {
                    section_id: section.section_id().clone(),
                    task_id: task_id.clone(),
                });
            }
            if let Some(first_section_id) =
                membership.insert(task_id.clone(), section.section_id().clone())
            {
                return Err(MermaidGanttExportError::DuplicateTaskMembership {
                    task_id: task_id.clone(),
                    first_section_id,
                    second_section_id: section.section_id().clone(),
                });
            }
        }
    }

    let mut used_tags = BTreeSet::<String>::new();
    let mut effective_tags = BTreeMap::<ObjectId, String>::new();
    let mut referenced_dependencies = BTreeSet::<ObjectId>::new();
    for (task_id, task) in ast.tasks() {
        if task.task_id() != task_id {
            return Err(MermaidGanttExportError::TaskIdMismatch {
                map_id: task_id.clone(),
                task_id: task.task_id().clone(),
            });
        }
        if task.name().is_empty() {
            return Err(MermaidGanttExportError::EmptyTaskName { task_id: task_id.clone() });
        }
        if !membership.contains_key(task_id) {
            return Err(MermaidGanttExportError::UnsectionedTask { task_id: task_id.clone() });
        }
        if let Some(tag) = task.mermaid_tag() {
            if !used_tags.insert(tag.to_owned()) {
                return Err(MermaidGanttExportError::DuplicateMermaidTag { tag: tag.to_owned() });
            }
            effective_tags.insert(task_id.clone(), tag.to_owned());
        }
        if let GanttTaskStart::After(dependency_id) = task.start() {
            if !ast.tasks().contains_key(dependency_id) {
                return Err(MermaidGanttExportError::MissingDependency {
                    task_id: task_id.clone(),
                    dependency_id: dependency_id.clone(),
                });
            }
            referenced_dependencies.insert(dependency_id.clone());
        }
    }

    for dependency_id in referenced_dependencies {
        if effective_tags.contains_key(&dependency_id) {
            continue;
        }
        let base = dependency_id
            .as_str()
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' { ch } else { '_' })
            .collect::<String>();
        let base = format!("nereid_{base}");
        let mut tag = base.clone();
        let mut suffix = 2_u64;
        while used_tags.contains(&tag) {
            tag = format!("{base}_{suffix}");
            suffix = suffix.saturating_add(1);
        }
        used_tags.insert(tag.clone());
        effective_tags.insert(dependency_id, tag);
    }

    let mut out = String::from("gantt\n");
    if let Some(title) = ast.title() {
        out.push_str(&format!("    title {title}\n"));
    }
    if let Some(fmt) = ast.date_format() {
        out.push_str(&format!("    dateFormat {fmt}\n"));
    }

    for section in ast.sections() {
        out.push_str(&format!("    section {}\n", section.name()));
        for tid in section.task_ids() {
            let Some(task) = ast.tasks().get(tid) else {
                continue;
            };
            let mut meta = Vec::new();
            if let Some(tag) = effective_tags.get(tid) {
                meta.push(tag.clone());
            }
            match task.start() {
                GanttTaskStart::Date(d) => meta.push(d.clone()),
                GanttTaskStart::After(dep) => {
                    let dep_tag = effective_tags
                        .get(dep)
                        .expect("validated dependency receives an effective tag");
                    meta.push(format!("after {dep_tag}"));
                }
                GanttTaskStart::Unspecified => {}
            }
            meta.push(task.raw_duration().to_owned());
            out.push_str(&format!("    {} :{}\n", task.name(), meta.join(", ")));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_screenshot_like_gantt() {
        let input = r#"
gantt
title A Gantt Diagram
dateFormat YYYY-MM-DD
section Section
A task           :a1, 2014-01-01, 30d
Another task     :after a1, 20d
section Another
Task in Another  :2014-01-12, 12d
another task     :24d
"#;
        let ast = parse_gantt_diagram(input).expect("parse");
        assert_eq!(ast.title(), Some("A Gantt Diagram"));
        assert_eq!(ast.sections().len(), 2);
        assert_eq!(ast.tasks().len(), 4);
        let a1 = ast.tasks().values().find(|t| t.mermaid_tag() == Some("a1")).unwrap();
        assert_eq!(a1.duration_days(), 30);
        let after = ast.tasks().values().find(|t| t.name() == "Another task").unwrap();
        assert!(matches!(after.start(), GanttTaskStart::After(_)));
    }

    #[test]
    fn export_synthesizes_tag_for_untagged_after_dependency() {
        let first_id = ObjectId::new("task:first").unwrap();
        let second_id = ObjectId::new("task:second").unwrap();
        let mut ast = GanttAst::default();
        let mut section = GanttSection::new(ObjectId::new("sec:stable").unwrap(), "Build");
        section.task_ids_mut().extend([first_id.clone(), second_id.clone()]);
        ast.sections_mut().push(section);
        ast.tasks_mut().insert(
            first_id.clone(),
            GanttTask::new(
                first_id.clone(),
                "First",
                GanttTaskStart::Date("2026-01-01".to_owned()),
                2,
                "2d",
            ),
        );
        ast.tasks_mut().insert(
            second_id.clone(),
            GanttTask::new(second_id, "Second", GanttTaskStart::After(first_id), 1, "1d"),
        );

        let exported = export_gantt_diagram(&ast).expect("export");
        assert!(exported.contains("nereid_task_first"), "{exported}");
        let reparsed = parse_gantt_diagram(&exported).expect("exported gantt reparses");
        assert!(reparsed.tasks().values().any(|task| {
            matches!(task.start(), GanttTaskStart::After(_)) && task.name() == "Second"
        }));
    }

    #[test]
    fn export_rejects_unsectioned_and_duplicate_task_membership() {
        let task_id = ObjectId::new("t:one").unwrap();
        let mut ast = GanttAst::default();
        ast.tasks_mut().insert(
            task_id.clone(),
            GanttTask::new(task_id.clone(), "One", GanttTaskStart::Unspecified, 1, "1d"),
        );
        assert_eq!(
            export_gantt_diagram(&ast),
            Err(MermaidGanttExportError::UnsectionedTask { task_id: task_id.clone() })
        );

        for (section_id, name) in [("sec:a", "A"), ("sec:b", "B")] {
            let mut section = GanttSection::new(ObjectId::new(section_id).unwrap(), name);
            section.task_ids_mut().push(task_id.clone());
            ast.sections_mut().push(section);
        }
        assert!(matches!(
            export_gantt_diagram(&ast),
            Err(MermaidGanttExportError::DuplicateTaskMembership { .. })
        ));
    }

    #[test]
    fn parse_rejects_duplicate_mermaid_tags() {
        let input = r#"
gantt
section S
T1 :a, 2026-01-01, 1d
T2 :a, 2026-01-02, 1d
T3 :after a, 1d
"#;
        let err = parse_gantt_diagram(input).expect_err("duplicate tags must fail parse");
        assert_eq!(
            err,
            MermaidGanttParseError::DuplicateMermaidTag {
                line_no: 5,
                tag: "a".to_owned(),
            }
        );
        assert_eq!(
            err.to_string(),
            "gantt Mermaid task tag is not unique: a (line 5)"
        );
    }

    #[test]
    fn export_rejects_duplicate_mermaid_tags() {
        let first_id = ObjectId::new("t:1").unwrap();
        let second_id = ObjectId::new("t:2").unwrap();
        let mut ast = GanttAst::default();
        let mut section = GanttSection::new(ObjectId::new("sec:1").unwrap(), "S");
        section.task_ids_mut().extend([first_id.clone(), second_id.clone()]);
        ast.sections_mut().push(section);
        ast.tasks_mut().insert(
            first_id.clone(),
            GanttTask::new(
                first_id.clone(),
                "T1",
                GanttTaskStart::Date("2026-01-01".to_owned()),
                1,
                "1d",
            )
            .with_mermaid_tag(Some("a")),
        );
        ast.tasks_mut().insert(
            second_id.clone(),
            GanttTask::new(
                second_id,
                "T2",
                GanttTaskStart::Date("2026-01-02".to_owned()),
                1,
                "1d",
            )
            .with_mermaid_tag(Some("a")),
        );
        assert_eq!(
            export_gantt_diagram(&ast),
            Err(MermaidGanttExportError::DuplicateMermaidTag {
                tag: "a".to_owned()
            })
        );
    }
}
