// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::ident::validate_mermaid_ident;
pub use super::ident::MermaidIdentError;

use crate::model::ids::ObjectId;
use crate::model::seq_ast::{
    SequenceAst, SequenceBlock, SequenceBlockKind, SequenceMessage, SequenceMessageKind,
    SequenceParticipant, SequenceSection, SequenceSectionKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidSequenceParseError {
    MissingHeader,
    UnsupportedSyntax {
        line_no: usize,
        line: String,
    },
    InvalidParticipantDecl {
        line_no: usize,
        line: String,
    },
    InvalidParticipantName {
        line_no: usize,
        name: String,
        reason: MermaidIdentError,
    },
    InvalidMessageLine {
        line_no: usize,
        line: String,
    },
    InvalidMessageParticipant {
        line_no: usize,
        name: String,
        reason: MermaidIdentError,
    },
    MissingMessageText {
        line_no: usize,
        line: String,
    },
    UnmatchedEnd {
        line_no: usize,
    },
    ElseOutsideAlt {
        line_no: usize,
        line: String,
    },
    AndOutsidePar {
        line_no: usize,
        line: String,
    },
    BlockNestingTooDeep {
        line_no: usize,
        max_depth: usize,
    },
    EmptyBlockSection {
        line_no: usize,
        section_id: ObjectId,
    },
    UnclosedBlock {
        opened_on_line_no: usize,
        block_id: ObjectId,
        kind: SequenceBlockKind,
    },
}

impl fmt::Display for MermaidSequenceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingHeader => {
                f.write_str("expected 'sequenceDiagram' as the first non-empty line")
            }
            Self::UnsupportedSyntax { line_no, line } => {
                write!(f, "unsupported Mermaid syntax on line {line_no}: {line}")
            }
            Self::InvalidParticipantDecl { line_no, line } => {
                write!(
                    f,
                    "invalid participant declaration on line {line_no}: {line} (expected 'participant <name>')"
                )
            }
            Self::InvalidParticipantName {
                line_no,
                name,
                reason,
            } => write!(
                f,
                "invalid participant name on line {line_no}: {name} ({reason})"
            ),
            Self::InvalidMessageLine { line_no, line } => write!(
                f,
                "invalid message syntax on line {line_no}: {line} (expected '<from><arrow><to>: <text>')"
            ),
            Self::InvalidMessageParticipant {
                line_no,
                name,
                reason,
            } => write!(
                f,
                "invalid message participant on line {line_no}: {name} ({reason})"
            ),
            Self::MissingMessageText { line_no, line } => {
                write!(f, "missing message text on line {line_no}: {line}")
            }
            Self::UnmatchedEnd { line_no } => {
                write!(
                    f,
                    "unmatched 'end' on line {line_no}: no block is currently open"
                )
            }
            Self::ElseOutsideAlt { line_no, line } => write!(
                f,
                "invalid 'else' on line {line_no}: only valid inside an open 'alt' block: {line}"
            ),
            Self::AndOutsidePar { line_no, line } => write!(
                f,
                "invalid 'and' on line {line_no}: only valid inside an open 'par' block: {line}"
            ),
            Self::BlockNestingTooDeep { line_no, max_depth } => write!(
                f,
                "block nesting too deep on line {line_no}: max supported depth is {max_depth}"
            ),
            Self::EmptyBlockSection {
                line_no,
                section_id,
            } => write!(
                f,
                "empty block section on line {line_no}: {section_id} contains no messages"
            ),
            Self::UnclosedBlock {
                opened_on_line_no,
                block_id,
                kind,
            } => write!(
                f,
                "unclosed '{}' block {block_id}: missing 'end' for block opened on line {opened_on_line_no}",
                block_kind_keyword(*kind)
            ),
        }
    }
}

impl std::error::Error for MermaidSequenceParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MermaidSequenceExportError {
    MissingParticipant { participant_id: ObjectId },
    InvalidMessageText { message_id: ObjectId, text: String },
    InvalidBlockMembership { block_id: ObjectId, reason: String },
}

impl fmt::Display for MermaidSequenceExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingParticipant { participant_id } => {
                write!(f, "message references missing participant id: {participant_id}")
            }
            Self::InvalidMessageText { message_id, text } => write!(
                f,
                "cannot export message text for {message_id}: contains unsupported characters: {text:?}"
            ),
            Self::InvalidBlockMembership { block_id, reason } => {
                write!(f, "cannot export block {block_id}: {reason}")
            }
        }
    }
}

impl std::error::Error for MermaidSequenceExportError {}

fn participant_id_from_mermaid_name(name: &str) -> Result<ObjectId, MermaidIdentError> {
    validate_mermaid_ident(name)?;
    // Stable and human-friendly by default; long-term stability is carried in `.meta.json` sidecars.
    ObjectId::new(format!("p:{name}")).map_err(|_| MermaidIdentError::ContainsSlash)
}

fn message_id_from_index(index: usize) -> ObjectId {
    ObjectId::new(format!("m:{index:04}")).expect("valid message id")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arrow {
    Sync,
    Async,
    Return,
}

impl Arrow {
    fn from_token(token: &str) -> Option<Self> {
        match token {
            // Mermaid currently documents 10 message arrow tokens. Nereid intentionally normalizes
            // them into a smaller rendering/export set.
            "-)" | "--)" => Some(Self::Async),
            "-->>" | "<<-->>" => Some(Self::Return),
            // Everything else is treated as a sync message for now.
            "->>" | "->" | "-->" | "<<->>" | "-x" | "--x" => Some(Self::Sync),
            _ => None,
        }
    }

    fn as_token(self) -> &'static str {
        match self {
            Self::Sync => "->>",
            Self::Async => "-)",
            Self::Return => "-->>",
        }
    }

    fn kind(self) -> SequenceMessageKind {
        match self {
            Self::Sync => SequenceMessageKind::Sync,
            Self::Async => SequenceMessageKind::Async,
            Self::Return => SequenceMessageKind::Return,
        }
    }

    fn from_kind(kind: SequenceMessageKind) -> Self {
        match kind {
            SequenceMessageKind::Sync => Self::Sync,
            SequenceMessageKind::Async => Self::Async,
            SequenceMessageKind::Return => Self::Return,
        }
    }
}

fn split_once_any<'a>(
    haystack: &'a str,
    needles: &[&'static str],
) -> Option<(&'a str, &'static str, &'a str)> {
    let mut best: Option<(usize, &'static str)> = None;
    for &needle in needles {
        if let Some(idx) = haystack.find(needle) {
            let take = match best {
                None => true,
                Some((best_idx, best_needle)) => {
                    idx < best_idx || (idx == best_idx && needle.len() > best_needle.len())
                }
            };
            if take {
                best = Some((idx, needle));
            }
        }
    }
    let (idx, needle) = best?;
    let left = &haystack[..idx];
    let right = &haystack[idx + needle.len()..];
    Some((left, needle, right))
}

fn is_comment_line(trimmed: &str) -> bool {
    trimmed.starts_with("%%")
}

fn ensure_participant(
    participants: &mut BTreeMap<ObjectId, SequenceParticipant>,
    name: &str,
    line_no: usize,
) -> Result<ObjectId, MermaidSequenceParseError> {
    let participant_id = participant_id_from_mermaid_name(name).map_err(|reason| {
        MermaidSequenceParseError::InvalidMessageParticipant {
            line_no,
            name: name.to_owned(),
            reason,
        }
    })?;

    participants
        .entry(participant_id.clone())
        .or_insert_with(|| SequenceParticipant::new(name.to_owned()));

    Ok(participant_id)
}

const MAX_BLOCK_NEST_DEPTH: usize = 8;

#[derive(Debug, Clone)]
struct OpenBlock {
    block_index: usize,
    block_id: ObjectId,
    kind: SequenceBlockKind,
    header: Option<String>,
    sections: Vec<SequenceSection>,
    blocks: Vec<SequenceBlock>,
    current_section_index: usize,
    opened_on_line_no: usize,
}

impl OpenBlock {
    fn new(
        block_index: usize,
        kind: SequenceBlockKind,
        header: Option<String>,
        opened_on_line_no: usize,
    ) -> Self {
        let block_id = SequenceBlock::make_block_id(block_index);
        let section_id = SequenceSection::make_section_id(block_index, 0);
        Self {
            block_index,
            block_id,
            kind,
            header,
            sections: vec![SequenceSection::new(
                section_id,
                SequenceSectionKind::Main,
                None,
                Vec::new(),
            )],
            blocks: Vec::new(),
            current_section_index: 0,
            opened_on_line_no,
        }
    }

    fn current_section(&self) -> &SequenceSection {
        self.sections
            .get(self.current_section_index)
            .expect("current section in range")
    }

    fn current_section_mut(&mut self) -> &mut SequenceSection {
        self.sections
            .get_mut(self.current_section_index)
            .expect("current section in range")
    }

    fn push_message_id(&mut self, message_id: ObjectId) {
        self.current_section_mut()
            .message_ids_mut()
            .push(message_id);
    }

    fn start_section(&mut self, kind: SequenceSectionKind, header: Option<String>) {
        let section_index = self.sections.len();
        let section_id = SequenceSection::make_section_id(self.block_index, section_index);
        self.sections
            .push(SequenceSection::new(section_id, kind, header, Vec::new()));
        self.current_section_index = section_index;
    }

    fn into_block(self) -> SequenceBlock {
        SequenceBlock::new(
            self.block_id,
            self.kind,
            self.header,
            self.sections,
            self.blocks,
        )
    }
}

fn block_kind_keyword(kind: SequenceBlockKind) -> &'static str {
    match kind {
        SequenceBlockKind::Alt => "alt",
        SequenceBlockKind::Opt => "opt",
        SequenceBlockKind::Loop => "loop",
        SequenceBlockKind::Par => "par",
    }
}

fn section_kind_keyword(kind: SequenceSectionKind) -> &'static str {
    match kind {
        SequenceSectionKind::Main => "",
        SequenceSectionKind::Else => "else",
        SequenceSectionKind::And => "and",
    }
}

fn keyword_header(trimmed: &str, keyword: &str) -> Option<String> {
    let rest = trimmed.get(keyword.len()..).unwrap_or_default().trim();
    (!rest.is_empty()).then(|| rest.to_owned())
}

/// Parse a deliberately limited `sequenceDiagram` Mermaid subset.
///
/// Supported lines (after `sequenceDiagram`):
/// - `participant <name>` (identifier must not contain whitespace or `/`)
/// - `<from><arrow><to>: <text>` where `<arrow>` is one of Mermaid's documented message arrows
///   (normalized internally; export uses `->>`, `-)`, `-->>`)
/// - `alt [header...]` / `opt [header...]` / `loop [header...]` / `par [header...]`
/// - `else [header...]` (only inside `alt`)
/// - `and [header...]` (only inside `par`)
/// - `end` (closes the most recently opened block)
///
/// All other Mermaid syntax is rejected with an actionable error.
pub fn parse_sequence_diagram(input: &str) -> Result<SequenceAst, MermaidSequenceParseError> {
    let mut ast = SequenceAst::default();

    let mut saw_header = false;
    let mut used_message_ids = BTreeSet::<ObjectId>::new();
    let mut open_blocks = Vec::<OpenBlock>::new();
    let mut next_block_index = 0usize;
    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || is_comment_line(trimmed) {
            continue;
        }

        if !saw_header {
            if trimmed == "sequenceDiagram" {
                saw_header = true;
                continue;
            }
            return Err(MermaidSequenceParseError::MissingHeader);
        }

        if let Some(keyword) = trimmed.split_whitespace().next() {
            if keyword == "participant" {
                let mut parts = trimmed.split_whitespace();
                parts.next(); // keyword
                let Some(name) = parts.next() else {
                    return Err(MermaidSequenceParseError::InvalidParticipantDecl {
                        line_no,
                        line: trimmed.to_owned(),
                    });
                };
                if parts.next().is_some() {
                    return Err(MermaidSequenceParseError::InvalidParticipantDecl {
                        line_no,
                        line: trimmed.to_owned(),
                    });
                }

                validate_mermaid_ident(name).map_err(|reason| {
                    MermaidSequenceParseError::InvalidParticipantName {
                        line_no,
                        name: name.to_owned(),
                        reason,
                    }
                })?;

                let participant_id = participant_id_from_mermaid_name(name).map_err(|reason| {
                    MermaidSequenceParseError::InvalidParticipantName {
                        line_no,
                        name: name.to_owned(),
                        reason,
                    }
                })?;
                ast.participants_mut()
                    .entry(participant_id)
                    .or_insert_with(|| SequenceParticipant::new(name.to_owned()));
                continue;
            }

            match keyword {
                "alt" | "opt" | "loop" | "par" => {
                    if open_blocks.len() >= MAX_BLOCK_NEST_DEPTH {
                        return Err(MermaidSequenceParseError::BlockNestingTooDeep {
                            line_no,
                            max_depth: MAX_BLOCK_NEST_DEPTH,
                        });
                    }

                    next_block_index += 1;
                    let kind = match keyword {
                        "alt" => SequenceBlockKind::Alt,
                        "opt" => SequenceBlockKind::Opt,
                        "loop" => SequenceBlockKind::Loop,
                        "par" => SequenceBlockKind::Par,
                        _ => unreachable!("matched keyword"),
                    };
                    let header = keyword_header(trimmed, keyword);
                    open_blocks.push(OpenBlock::new(next_block_index, kind, header, line_no));
                    continue;
                }
                "else" => {
                    let Some(top) = open_blocks.last_mut() else {
                        return Err(MermaidSequenceParseError::ElseOutsideAlt {
                            line_no,
                            line: trimmed.to_owned(),
                        });
                    };
                    if top.kind != SequenceBlockKind::Alt {
                        return Err(MermaidSequenceParseError::ElseOutsideAlt {
                            line_no,
                            line: trimmed.to_owned(),
                        });
                    }
                    if top.current_section().message_ids().is_empty() {
                        return Err(MermaidSequenceParseError::EmptyBlockSection {
                            line_no,
                            section_id: top.current_section().section_id().clone(),
                        });
                    }
                    let header = keyword_header(trimmed, keyword);
                    top.start_section(SequenceSectionKind::Else, header);
                    continue;
                }
                "and" => {
                    let Some(top) = open_blocks.last_mut() else {
                        return Err(MermaidSequenceParseError::AndOutsidePar {
                            line_no,
                            line: trimmed.to_owned(),
                        });
                    };
                    if top.kind != SequenceBlockKind::Par {
                        return Err(MermaidSequenceParseError::AndOutsidePar {
                            line_no,
                            line: trimmed.to_owned(),
                        });
                    }
                    if top.current_section().message_ids().is_empty() {
                        return Err(MermaidSequenceParseError::EmptyBlockSection {
                            line_no,
                            section_id: top.current_section().section_id().clone(),
                        });
                    }
                    let header = keyword_header(trimmed, keyword);
                    top.start_section(SequenceSectionKind::And, header);
                    continue;
                }
                "end" => {
                    if trimmed != "end" {
                        return Err(MermaidSequenceParseError::UnsupportedSyntax {
                            line_no,
                            line: trimmed.to_owned(),
                        });
                    }
                    let Some(top) = open_blocks.last() else {
                        return Err(MermaidSequenceParseError::UnmatchedEnd { line_no });
                    };
                    if top.current_section().message_ids().is_empty() {
                        return Err(MermaidSequenceParseError::EmptyBlockSection {
                            line_no,
                            section_id: top.current_section().section_id().clone(),
                        });
                    }

                    let finished = open_blocks.pop().expect("present");
                    let block = finished.into_block();
                    if let Some(parent) = open_blocks.last_mut() {
                        parent.blocks.push(block);
                    } else {
                        ast.blocks_mut().push(block);
                    }
                    continue;
                }
                _ => {}
            }
        }

        let (from_raw, arrow_token, rest) = split_once_any(
            trimmed,
            &[
                "<<-->>", "<<->>", "-->>", "->>", "--)", "-)", "--x", "-x", "-->", "->",
            ],
        )
        .ok_or_else(|| MermaidSequenceParseError::UnsupportedSyntax {
            line_no,
            line: trimmed.to_owned(),
        })?;
        let arrow = Arrow::from_token(arrow_token).ok_or_else(|| {
            MermaidSequenceParseError::InvalidMessageLine {
                line_no,
                line: trimmed.to_owned(),
            }
        })?;

        let rest = rest.trim_start();
        let (raw_arrow, rest) = match rest.chars().next() {
            Some('+' | '-') => {
                let suffix = rest.chars().next().expect("present");
                let mut raw_arrow = arrow_token.to_owned();
                raw_arrow.push(suffix);
                (raw_arrow, &rest[suffix.len_utf8()..])
            }
            _ => (arrow_token.to_owned(), rest),
        };

        let (to_raw, text_raw) =
            rest.split_once(':')
                .ok_or_else(|| MermaidSequenceParseError::InvalidMessageLine {
                    line_no,
                    line: trimmed.to_owned(),
                })?;

        let from_name = from_raw.trim();
        let to_name = to_raw.trim();
        validate_mermaid_ident(from_name).map_err(|reason| {
            MermaidSequenceParseError::InvalidMessageParticipant {
                line_no,
                name: from_name.to_owned(),
                reason,
            }
        })?;
        validate_mermaid_ident(to_name).map_err(|reason| {
            MermaidSequenceParseError::InvalidMessageParticipant {
                line_no,
                name: to_name.to_owned(),
                reason,
            }
        })?;

        let text = text_raw.trim();
        if text.is_empty() {
            return Err(MermaidSequenceParseError::MissingMessageText {
                line_no,
                line: trimmed.to_owned(),
            });
        }

        let from_participant_id = ensure_participant(ast.participants_mut(), from_name, line_no)?;
        let to_participant_id = ensure_participant(ast.participants_mut(), to_name, line_no)?;

        let message_index = ast.messages().len() + 1;
        let mut message_id = message_id_from_index(message_index);
        let mut bump = 0usize;
        while used_message_ids.contains(&message_id) {
            bump += 1;
            message_id = message_id_from_index(message_index + bump);
        }
        used_message_ids.insert(message_id.clone());
        let message_id_for_membership = message_id.clone();
        let order_key = (message_index as i64) * 1000;
        let mut message = SequenceMessage::new(
            message_id,
            from_participant_id,
            to_participant_id,
            arrow.kind(),
            text.to_owned(),
            order_key,
        );
        let canonical = Arrow::from_kind(arrow.kind()).as_token();
        message.set_raw_arrow((raw_arrow != canonical).then_some(raw_arrow));

        for open_block in &mut open_blocks {
            open_block.push_message_id(message_id_for_membership.clone());
        }

        ast.messages_mut().push(message);
    }

    if !saw_header {
        return Err(MermaidSequenceParseError::MissingHeader);
    }

    if let Some(unclosed) = open_blocks.last() {
        return Err(MermaidSequenceParseError::UnclosedBlock {
            opened_on_line_no: unclosed.opened_on_line_no,
            block_id: unclosed.block_id.clone(),
            kind: unclosed.kind,
        });
    }

    Ok(ast)
}

fn validate_export_message_text(text: &str) -> bool {
    !text.contains('\n') && !text.contains('\r')
}

fn validate_export_arrow_token(raw: &str, expected_kind: SequenceMessageKind) -> Option<&str> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.contains('\n')
        || raw.contains('\r')
        || raw.chars().any(|ch| ch.is_whitespace())
    {
        return None;
    }

    let (base, _suffix) = match raw.strip_suffix('+') {
        Some(base) => (base, Some('+')),
        None => match raw.strip_suffix('-') {
            Some(base) => (base, Some('-')),
            None => (raw, None),
        },
    };

    let arrow = Arrow::from_token(base)?;
    if arrow.kind() != expected_kind {
        return None;
    }

    Some(raw)
}

#[derive(Debug, Clone, Copy)]
struct ExportBlockRange {
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy)]
struct ExportSectionRange<'a> {
    section: &'a SequenceSection,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone, Copy)]
enum ExportEvent<'a> {
    BlockOpen {
        block: &'a SequenceBlock,
        depth: usize,
    },
    SectionSplit {
        block: &'a SequenceBlock,
        section: &'a SequenceSection,
        depth: usize,
    },
    BlockClose {
        block: &'a SequenceBlock,
        depth: usize,
    },
}

fn export_event_sort_key_before<'a>(
    event: &ExportEvent<'a>,
) -> (u8, usize, &'a ObjectId, Option<&'a ObjectId>) {
    match *event {
        ExportEvent::SectionSplit {
            block,
            section,
            depth,
        } => (0, depth, block.block_id(), Some(section.section_id())),
        ExportEvent::BlockOpen { block, depth } => (1, depth, block.block_id(), None),
        ExportEvent::BlockClose { block, depth } => (2, depth, block.block_id(), None),
    }
}

fn export_event_sort_key_after<'a>(event: &ExportEvent<'a>) -> (usize, &'a ObjectId) {
    match *event {
        ExportEvent::BlockClose { block, depth } => (usize::MAX - depth, block.block_id()),
        ExportEvent::BlockOpen { block, depth } => (usize::MAX - depth, block.block_id()),
        ExportEvent::SectionSplit {
            block,
            depth,
            section: _,
        } => (usize::MAX - depth, block.block_id()),
    }
}

fn export_section_ranges<'a>(
    block: &'a SequenceBlock,
    message_index_by_id: &BTreeMap<ObjectId, usize>,
) -> Result<Vec<ExportSectionRange<'a>>, MermaidSequenceExportError> {
    let mut ranges = Vec::<ExportSectionRange<'a>>::new();

    if block.sections().is_empty() {
        return Err(MermaidSequenceExportError::InvalidBlockMembership {
            block_id: block.block_id().clone(),
            reason: "has no sections".to_owned(),
        });
    }

    for section in block.sections() {
        if section.message_ids().is_empty() {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!("section {} is empty", section.section_id()),
            });
        }

        let mut indices = Vec::<usize>::with_capacity(section.message_ids().len());
        for message_id in section.message_ids() {
            let Some(&idx) = message_index_by_id.get(message_id) else {
                return Err(MermaidSequenceExportError::InvalidBlockMembership {
                    block_id: block.block_id().clone(),
                    reason: format!(
                        "section {} references missing message id {}",
                        section.section_id(),
                        message_id
                    ),
                });
            };
            indices.push(idx);
        }
        indices.sort_unstable();
        indices.dedup();
        if indices.is_empty() {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!("section {} is empty after dedup", section.section_id()),
            });
        }
        for window in indices.windows(2) {
            if window[1] != window[0] + 1 {
                return Err(MermaidSequenceExportError::InvalidBlockMembership {
                    block_id: block.block_id().clone(),
                    reason: format!(
                        "section {} message membership is not contiguous",
                        section.section_id()
                    ),
                });
            }
        }

        ranges.push(ExportSectionRange {
            section,
            start: indices[0],
            end: indices[indices.len() - 1],
        });
    }

    // Sections are stored in declaration order; enforce they cover a contiguous range without gaps.
    for (idx, range) in ranges.iter().enumerate() {
        if idx == 0 {
            continue;
        }
        let prev = &ranges[idx - 1];
        if range.start != prev.end + 1 {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!(
                    "section {} does not start immediately after previous section {}",
                    range.section.section_id(),
                    prev.section.section_id()
                ),
            });
        }
    }

    Ok(ranges)
}

fn export_schedule_block<'a>(
    block: &'a SequenceBlock,
    depth: usize,
    message_index_by_id: &BTreeMap<ObjectId, usize>,
    before: &mut [Vec<ExportEvent<'a>>],
    after: &mut [Vec<ExportEvent<'a>>],
) -> Result<ExportBlockRange, MermaidSequenceExportError> {
    let section_ranges = export_section_ranges(block, message_index_by_id)?;
    let start = section_ranges.first().expect("non-empty").start;
    let end = section_ranges.last().expect("non-empty").end;

    if start >= before.len() || end >= after.len() {
        return Err(MermaidSequenceExportError::InvalidBlockMembership {
            block_id: block.block_id().clone(),
            reason: "block message range is out of bounds".to_owned(),
        });
    }

    before[start].push(ExportEvent::BlockOpen { block, depth });
    after[end].push(ExportEvent::BlockClose { block, depth });

    for range in section_ranges.iter().skip(1) {
        before[range.start].push(ExportEvent::SectionSplit {
            block,
            section: range.section,
            depth,
        });
    }

    let mut child_ranges = Vec::<(usize, usize, &ObjectId)>::new();
    for child in block.blocks() {
        let child_range =
            export_schedule_block(child, depth + 1, message_index_by_id, before, after)?;
        if child_range.start < start || child_range.end > end {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!(
                    "nested block {} is outside parent message range",
                    child.block_id()
                ),
            });
        }

        let mut containing_sections = section_ranges
            .iter()
            .filter(|section| child_range.start >= section.start && child_range.end <= section.end);
        let Some(_section) = containing_sections.next() else {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!(
                    "nested block {} is not contained within a single parent section",
                    child.block_id()
                ),
            });
        };
        if containing_sections.next().is_some() {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!(
                    "nested block {} is ambiguously contained in multiple parent sections",
                    child.block_id()
                ),
            });
        }

        child_ranges.push((child_range.start, child_range.end, child.block_id()));
    }

    child_ranges.sort_by_key(|(start, _end, block_id)| (*start, (*block_id).clone()));
    for window in child_ranges.windows(2) {
        let (a_start, a_end, a_id) = window[0];
        let (b_start, b_end, b_id) = window[1];
        if a_end >= b_start {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: block.block_id().clone(),
                reason: format!(
                    "nested blocks overlap: {a_id} [{a_start}..{a_end}] and {b_id} [{b_start}..{b_end}]"
                ),
            });
        }
    }

    Ok(ExportBlockRange { start, end })
}

/// Export a `sequenceDiagram` to canonical Mermaid `.mmd`.
///
/// Export is stable/deterministic:
/// - Participants are emitted in `ObjectId` order (typically lexical by `p:<name>`).
/// - Messages are emitted in `(order_key, message_id)` order.
pub fn export_sequence_diagram(ast: &SequenceAst) -> Result<String, MermaidSequenceExportError> {
    let mut out = String::new();
    out.push_str("sequenceDiagram\n");

    for participant in ast.participants().values() {
        if let Some(role) = participant.role() {
            out.push_str(role);
            out.push(' ');
            out.push_str(participant.mermaid_name());
        } else {
            out.push_str("participant ");
            out.push_str(participant.mermaid_name());
        }
        out.push('\n');
    }

    let mut messages = ast.messages().iter().collect::<Vec<_>>();
    messages.sort_by(|a, b| SequenceMessage::cmp_in_order(a, b));

    let mut before = vec![Vec::<ExportEvent<'_>>::new(); messages.len()];
    let mut after = vec![Vec::<ExportEvent<'_>>::new(); messages.len()];

    if !ast.blocks().is_empty() {
        if messages.is_empty() {
            return Err(MermaidSequenceExportError::InvalidBlockMembership {
                block_id: ast
                    .blocks()
                    .first()
                    .map(|block| block.block_id().clone())
                    .unwrap_or_else(|| ObjectId::new("b:0000").expect("valid id")),
                reason: "diagram contains blocks but no messages".to_owned(),
            });
        }

        let message_index_by_id = messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| (msg.message_id().clone(), idx))
            .collect::<BTreeMap<_, _>>();

        let mut root_ranges = Vec::<(usize, usize, &ObjectId)>::new();
        for block in ast.blocks() {
            let range =
                export_schedule_block(block, 0, &message_index_by_id, &mut before, &mut after)?;
            root_ranges.push((range.start, range.end, block.block_id()));
        }

        root_ranges.sort_by_key(|(start, _end, block_id)| (*start, (*block_id).clone()));
        for window in root_ranges.windows(2) {
            let (a_start, a_end, a_id) = window[0];
            let (b_start, b_end, b_id) = window[1];
            if a_end >= b_start {
                return Err(MermaidSequenceExportError::InvalidBlockMembership {
                    block_id: a_id.clone(),
                    reason: format!(
                        "root blocks overlap: {a_id} [{a_start}..{a_end}] and {b_id} [{b_start}..{b_end}]"
                    ),
                });
            }
        }
    }

    for (idx, msg) in messages.into_iter().enumerate() {
        let mut before_events = std::mem::take(&mut before[idx]);
        before_events
            .sort_by(|a, b| export_event_sort_key_before(a).cmp(&export_event_sort_key_before(b)));
        for event in before_events {
            match event {
                ExportEvent::BlockOpen { block, depth: _ } => {
                    out.push_str(block_kind_keyword(block.kind()));
                    if let Some(header) = block.header() {
                        if !header.is_empty() {
                            out.push(' ');
                            out.push_str(header);
                        }
                    }
                    out.push('\n');
                }
                ExportEvent::SectionSplit {
                    block: _,
                    section,
                    depth: _,
                } => {
                    let keyword = section_kind_keyword(section.kind());
                    if keyword.is_empty() {
                        continue;
                    }
                    out.push_str(keyword);
                    if let Some(header) = section.header() {
                        if !header.is_empty() {
                            out.push(' ');
                            out.push_str(header);
                        }
                    }
                    out.push('\n');
                }
                ExportEvent::BlockClose { .. } => {
                    // Block closes are emitted from `after`.
                }
            }
        }

        let from_name = ast
            .participants()
            .get(msg.from_participant_id())
            .map(|p| p.mermaid_name())
            .ok_or_else(|| MermaidSequenceExportError::MissingParticipant {
                participant_id: msg.from_participant_id().clone(),
            })?;
        let to_name = ast
            .participants()
            .get(msg.to_participant_id())
            .map(|p| p.mermaid_name())
            .ok_or_else(|| MermaidSequenceExportError::MissingParticipant {
                participant_id: msg.to_participant_id().clone(),
            })?;

        out.push_str(from_name);
        let arrow = msg
            .raw_arrow()
            .and_then(|raw| validate_export_arrow_token(raw, msg.kind()))
            .unwrap_or_else(|| Arrow::from_kind(msg.kind()).as_token());
        out.push_str(arrow);
        out.push_str(to_name);
        out.push_str(": ");
        let text = msg.text();
        if !validate_export_message_text(text) {
            return Err(MermaidSequenceExportError::InvalidMessageText {
                message_id: msg.message_id().clone(),
                text: text.to_owned(),
            });
        }
        out.push_str(text);
        out.push('\n');

        let mut after_events = std::mem::take(&mut after[idx]);
        after_events
            .sort_by(|a, b| export_event_sort_key_after(a).cmp(&export_event_sort_key_after(b)));
        for event in after_events {
            if let ExportEvent::BlockClose { .. } = event {
                out.push_str("end\n");
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        export_sequence_diagram, parse_sequence_diagram, MermaidSequenceExportError,
        MermaidSequenceParseError,
    };
    use crate::model::seq_ast::{SequenceAst, SequenceMessageKind};
    use crate::model::seq_ast::{SequenceBlockKind, SequenceMessage, SequenceSectionKind};
    use std::collections::BTreeSet;

    type SequenceParticipantSemanticView = BTreeSet<String>;
    type SequenceMessageSemanticView = Vec<(String, String, SequenceMessageKind, String)>;

    fn assert_canonical_roundtrip(input: &str, expected: &str) {
        let ast1 = parse_sequence_diagram(input).expect("parse 1");
        let out1 = export_sequence_diagram(&ast1).expect("export 1");
        assert_eq!(out1, expected);

        let ast2 = parse_sequence_diagram(&out1).expect("parse 2");
        let out2 = export_sequence_diagram(&ast2).expect("export 2");
        assert_eq!(out2, expected);
    }

    fn semantic_view(
        ast: &SequenceAst,
    ) -> (SequenceParticipantSemanticView, SequenceMessageSemanticView) {
        let participants = ast
            .participants()
            .values()
            .map(|p| p.mermaid_name().to_owned())
            .collect::<BTreeSet<_>>();

        let mut messages = Vec::new();
        for msg in ast.messages() {
            let from = ast
                .participants()
                .get(msg.from_participant_id())
                .expect("from participant")
                .mermaid_name()
                .to_owned();
            let to = ast
                .participants()
                .get(msg.to_participant_id())
                .expect("to participant")
                .mermaid_name()
                .to_owned();
            messages.push((from, to, msg.kind(), msg.text().to_owned()));
        }
        (participants, messages)
    }

    #[test]
    fn parses_participants_and_messages() {
        let input = r#"
            %% comment
            sequenceDiagram
            participant Alice
            participant Bob
            Alice->>Bob: Hello
            Bob-->>Alice: Great!
            Alice-)Bob: See you later
        "#;

        let ast = parse_sequence_diagram(input).expect("parse");
        let (participants, messages) = semantic_view(&ast);

        assert_eq!(
            participants,
            ["Alice".to_owned(), "Bob".to_owned()].into_iter().collect()
        );
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].0, "Alice");
        assert_eq!(messages[0].1, "Bob");
        assert_eq!(messages[0].2, SequenceMessageKind::Sync);
        assert_eq!(messages[0].3, "Hello");
        assert_eq!(messages[1].2, SequenceMessageKind::Return);
        assert_eq!(messages[2].2, SequenceMessageKind::Async);
    }

    #[test]
    fn accepts_additional_mermaid_arrow_variants_and_activation_suffixes() {
        let input = r#"
            sequenceDiagram
            participant Alice
            participant Bob
            participant Carol
            Alice->Bob: no-head
            Alice-->Bob: dotted no-head
            Alice->>Bob: head
            Alice-->>Bob: dotted head
            Alice<<->>Bob: bidi head
            Alice<<-->>Bob: dotted bidi
            Alice-xBob: cross
            Alice--xBob: dotted cross
            Alice-)Bob: async
            Alice--)Bob: dotted async
            Alice->>+Carol: activated
            Bob-->>-Alice: deactivated
        "#;

        let ast = parse_sequence_diagram(input).expect("parse");
        let (_, messages) = semantic_view(&ast);

        assert_eq!(messages.len(), 12);
        assert_eq!(messages[0].2, SequenceMessageKind::Sync);
        assert_eq!(messages[1].2, SequenceMessageKind::Sync);
        assert_eq!(messages[2].2, SequenceMessageKind::Sync);
        assert_eq!(messages[3].2, SequenceMessageKind::Return);
        assert_eq!(messages[4].2, SequenceMessageKind::Sync);
        assert_eq!(messages[5].2, SequenceMessageKind::Return);
        assert_eq!(messages[6].2, SequenceMessageKind::Sync);
        assert_eq!(messages[7].2, SequenceMessageKind::Sync);
        assert_eq!(messages[8].2, SequenceMessageKind::Async);
        assert_eq!(messages[9].2, SequenceMessageKind::Async);
        assert_eq!(messages[10].2, SequenceMessageKind::Sync);
        assert_eq!(messages[11].2, SequenceMessageKind::Return);
    }

    #[test]
    fn creates_implicit_participants_from_messages() {
        let input = "sequenceDiagram\nAlice->>Bob: Hi\n";
        let ast = parse_sequence_diagram(input).expect("parse");
        let (participants, messages) = semantic_view(&ast);
        assert_eq!(
            participants,
            ["Alice".to_owned(), "Bob".to_owned()].into_iter().collect()
        );
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn semantic_roundtrip_parse_export_parse() {
        let input = r#"
            sequenceDiagram
            %% order of declarations doesn't matter for semantics
            Bob-->>Alice: Pong
            Alice->>Bob: Ping
        "#;

        let ast1 = parse_sequence_diagram(input).expect("parse 1");
        let out = export_sequence_diagram(&ast1).expect("export");
        let ast2 = parse_sequence_diagram(&out).expect("parse 2");

        assert_eq!(semantic_view(&ast1), semantic_view(&ast2));
    }

    #[test]
    fn preserves_non_canonical_arrow_tokens_and_activation_suffixes_on_export() {
        let input = r#"
            sequenceDiagram
            Alice->>Bob: Canonical
            Alice-->Bob: Dotted no arrowhead
            Bob<<-->>Alice: Two-way return
            Alice->>+Bob: Activate
            Bob--)Alice: Async dotted open
        "#;

        let ast1 = parse_sequence_diagram(input).expect("parse 1");
        let arrows1 = ast1
            .messages_in_order()
            .into_iter()
            .map(|msg| {
                (
                    msg.kind(),
                    msg.raw_arrow().map(ToOwned::to_owned),
                    msg.text().to_owned(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            arrows1,
            vec![
                (SequenceMessageKind::Sync, None, "Canonical".to_owned()),
                (
                    SequenceMessageKind::Sync,
                    Some("-->".to_owned()),
                    "Dotted no arrowhead".to_owned()
                ),
                (
                    SequenceMessageKind::Return,
                    Some("<<-->>".to_owned()),
                    "Two-way return".to_owned()
                ),
                (
                    SequenceMessageKind::Sync,
                    Some("->>+".to_owned()),
                    "Activate".to_owned()
                ),
                (
                    SequenceMessageKind::Async,
                    Some("--)".to_owned()),
                    "Async dotted open".to_owned()
                ),
            ]
        );

        let out = export_sequence_diagram(&ast1).expect("export");
        let ast2 = parse_sequence_diagram(&out).expect("parse 2");
        let arrows2 = ast2
            .messages_in_order()
            .into_iter()
            .map(|msg| {
                (
                    msg.kind(),
                    msg.raw_arrow().map(ToOwned::to_owned),
                    msg.text().to_owned(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(arrows2, arrows1);
    }

    #[test]
    fn rejects_missing_header() {
        let err = parse_sequence_diagram("participant Alice\n").unwrap_err();
        assert_eq!(err, MermaidSequenceParseError::MissingHeader);
    }

    #[test]
    fn export_rejects_newlines_and_cr_in_message_text() {
        let mut ast =
            parse_sequence_diagram("sequenceDiagram\nAlice->>Bob: Hello\n").expect("parse");
        let original = ast.messages()[0].clone();

        for text in ["Hello\nWorld", "Hello\rWorld"] {
            *ast.messages_mut() = vec![SequenceMessage::new(
                original.message_id().clone(),
                original.from_participant_id().clone(),
                original.to_participant_id().clone(),
                original.kind(),
                text,
                original.order_key(),
            )];

            let err = export_sequence_diagram(&ast).unwrap_err();
            assert_eq!(
                err,
                MermaidSequenceExportError::InvalidMessageText {
                    message_id: original.message_id().clone(),
                    text: text.to_owned(),
                }
            );
        }
    }

    #[test]
    fn exports_alt_else_block_canonically() {
        let input = r#"
            sequenceDiagram
            Alice->>Bob: Start
            alt success
            Bob->>Alice: OK
            else failure
            Bob->>Alice: Nope
            end
            Alice->>Bob: Done
        "#;

        let expected = "\
sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: Start
alt success
Bob->>Alice: OK
else failure
Bob->>Alice: Nope
end
Alice->>Bob: Done
";

        assert_canonical_roundtrip(input, expected);
    }

    #[test]
    fn exports_opt_block_canonically() {
        let input = r#"
            sequenceDiagram
            Alice->>Bob: Pre
            opt Maybe
            Bob->>Alice: Inside
            end
            Alice->>Bob: Post
        "#;

        let expected = "\
sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: Pre
opt Maybe
Bob->>Alice: Inside
end
Alice->>Bob: Post
";

        assert_canonical_roundtrip(input, expected);
    }

    #[test]
    fn exports_loop_block_canonically() {
        let input = r#"
            sequenceDiagram
            Alice->>Bob: Pre
            loop Retry
            Bob->>Alice: Attempt
            end
            Alice->>Bob: Post
        "#;

        let expected = "\
sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: Pre
loop Retry
Bob->>Alice: Attempt
end
Alice->>Bob: Post
";

        assert_canonical_roundtrip(input, expected);
    }

    #[test]
    fn exports_par_and_block_canonically() {
        let input = r#"
            sequenceDiagram
            Alice->>Bob: Pre
            par First
            Alice->>Bob: Left
            and Second
            Bob->>Alice: Right
            end
            Alice->>Bob: Post
        "#;

        let expected = "\
sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: Pre
par First
Alice->>Bob: Left
and Second
Bob->>Alice: Right
end
Alice->>Bob: Post
";

        assert_canonical_roundtrip(input, expected);
    }

    #[test]
    fn exports_nested_blocks_canonically() {
        let input = r#"
            sequenceDiagram
            Alice->>Bob: Start
            alt Outer
            opt Inner
            Bob->>Alice: Inside
            end
            Bob->>Alice: After
            else Other
            Bob->>Alice: ElseMsg
            end
            Alice->>Bob: Done
        "#;

        let expected = "\
sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: Start
alt Outer
opt Inner
Bob->>Alice: Inside
end
Bob->>Alice: After
else Other
Bob->>Alice: ElseMsg
end
Alice->>Bob: Done
";

        assert_canonical_roundtrip(input, expected);

        let ast = parse_sequence_diagram(input).expect("parse");
        assert_eq!(ast.blocks().len(), 1);
        let outer = &ast.blocks()[0];
        assert_eq!(outer.kind(), SequenceBlockKind::Alt);
        assert_eq!(outer.sections().len(), 2);
        assert_eq!(outer.sections()[0].kind(), SequenceSectionKind::Main);
        assert_eq!(outer.sections()[1].kind(), SequenceSectionKind::Else);
        assert_eq!(outer.blocks().len(), 1);
        assert_eq!(outer.blocks()[0].kind(), SequenceBlockKind::Opt);
    }

    #[test]
    fn rejects_unmatched_end() {
        let err = parse_sequence_diagram("sequenceDiagram\nAlice->>Bob: Hi\nend\n").unwrap_err();
        assert_eq!(err, MermaidSequenceParseError::UnmatchedEnd { line_no: 3 });
    }

    #[test]
    fn rejects_else_outside_alt() {
        let err =
            parse_sequence_diagram("sequenceDiagram\nAlice->>Bob: Hi\nelse oops\n").unwrap_err();
        assert_eq!(
            err,
            MermaidSequenceParseError::ElseOutsideAlt {
                line_no: 3,
                line: "else oops".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_and_outside_par() {
        let err =
            parse_sequence_diagram("sequenceDiagram\nAlice->>Bob: Hi\nand oops\n").unwrap_err();
        assert_eq!(
            err,
            MermaidSequenceParseError::AndOutsidePar {
                line_no: 3,
                line: "and oops".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_empty_section_before_else() {
        let err = parse_sequence_diagram("sequenceDiagram\nalt A\nelse B\nAlice->>Bob: Hi\nend\n")
            .unwrap_err();
        assert_eq!(
            err,
            MermaidSequenceParseError::EmptyBlockSection {
                line_no: 3,
                section_id: crate::model::seq_ast::SequenceSection::make_section_id(1, 0),
            }
        );
    }

    #[test]
    fn rejects_block_nesting_too_deep() {
        let input = "\
sequenceDiagram
opt a1
opt a2
opt a3
opt a4
opt a5
opt a6
opt a7
opt a8
opt a9
Alice->>Bob: Hi
";
        let err = parse_sequence_diagram(input).unwrap_err();
        assert_eq!(
            err,
            MermaidSequenceParseError::BlockNestingTooDeep {
                line_no: 10,
                max_depth: super::MAX_BLOCK_NEST_DEPTH,
            }
        );
    }
}
