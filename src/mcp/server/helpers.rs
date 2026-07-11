// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

// include!: MCP server private helpers (not a standalone module).
//
// Role: id/ref parsing, digest builders, delta history, apply/error mapping, and
// Mermaid projection for `diagram_read` / snapshots. Mermaid export is fail-closed:
// export errors become MCP errors rather than skeleton or wrong-identity text.
// Owned by `server.rs` via `include!` so helpers share NereidMcp/tool scope.

fn diagram_kind_label(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "Sequence",
        DiagramKind::Flowchart => "Flowchart",
        DiagramKind::Class => "Class",
        DiagramKind::Er => "ER",
        DiagramKind::Gantt => "Gantt",
    }
}

fn detect_mermaid_kind(input: &str) -> Option<DiagramKind> {
    for raw_line in input.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }
        if trimmed.starts_with("sequenceDiagram") {
            return Some(DiagramKind::Sequence);
        }
        if trimmed.starts_with("flowchart") || trimmed.starts_with("graph") {
            return Some(DiagramKind::Flowchart);
        }
        if trimmed.starts_with("classDiagram") {
            return Some(DiagramKind::Class);
        }
        if trimmed.starts_with("erDiagram") {
            return Some(DiagramKind::Er);
        }
        if trimmed.starts_with("gantt") {
            return Some(DiagramKind::Gantt);
        }
        return None;
    }
    None
}

fn allocate_diagram_id(session: &Session, kind: DiagramKind) -> DiagramId {
    let base = match kind {
        DiagramKind::Sequence => "seq",
        DiagramKind::Flowchart => "flow",
        DiagramKind::Class => "class",
        DiagramKind::Er => "er",
        DiagramKind::Gantt => "gantt",
    };

    if !session.diagrams().contains_key(base) {
        return DiagramId::new(base.to_owned()).expect("valid diagram id");
    }

    for idx in 2.. {
        let candidate = format!("{base}-{idx}");
        if !session.diagrams().contains_key(candidate.as_str()) {
            return DiagramId::new(candidate).expect("valid diagram id");
        }
    }

    unreachable!("exhausted diagram id space")
}

fn resolve_diagram_id(session: &Session, diagram_id: Option<&str>) -> Result<DiagramId, ErrorData> {
    if let Some(diagram_id) = diagram_id {
        return DiagramId::new(diagram_id.to_owned()).map_err(|err| {
            ErrorData::invalid_params(
                format!("invalid diagram_id: {err}"),
                Some(serde_json::json!({ "diagram_id": diagram_id })),
            )
        });
    }

    session.active_diagram_id().cloned().ok_or_else(|| {
        ErrorData::invalid_params("diagram_id is required (no active diagram)", None)
    })
}

fn digest_for_diagram(diagram: &Diagram) -> DiagramDigest {
    match diagram.ast() {
        DiagramAst::Sequence(ast) => DiagramDigest {
            rev: diagram.rev(),
            counts: DiagramCounts {
                participants: ast.participants().len() as u64,
                messages: ast.messages().len() as u64,
                nodes: 0,
                edges: 0,
                classes: 0,
                relations: 0,
                entities: 0,
                relationships: 0,
                sections: 0,
                tasks: 0,
                dependencies: 0,
                lanes: 0,
            },
            key_names: ast.participants().values().map(|p| p.mermaid_name().to_owned()).collect(),
            context: ReadContext::default(),
        },
        DiagramAst::Flowchart(ast) => DiagramDigest {
            rev: diagram.rev(),
            counts: DiagramCounts {
                participants: 0,
                messages: 0,
                nodes: ast.nodes().len() as u64,
                edges: ast.edges().len() as u64,
                classes: 0,
                relations: 0,
                entities: 0,
                relationships: 0,
                sections: 0,
                tasks: 0,
                dependencies: 0,
                lanes: 0,
            },
            key_names: ast.nodes().values().map(|n| n.label().to_owned()).collect(),
            context: ReadContext::default(),
        },
        DiagramAst::Class(ast) => DiagramDigest {
            rev: diagram.rev(),
            counts: DiagramCounts {
                participants: 0,
                messages: 0,
                nodes: ast.classes().len() as u64,
                edges: ast.relations().len() as u64,
                classes: ast.classes().len() as u64,
                relations: ast.relations().len() as u64,
                entities: 0,
                relationships: 0,
                sections: 0,
                tasks: 0,
                dependencies: 0,
                lanes: 0,
            },
            key_names: ast.classes().values().map(|c| c.name().to_owned()).collect(),
            context: ReadContext::default(),
        },
        DiagramAst::Er(ast) => DiagramDigest {
            rev: diagram.rev(),
            counts: DiagramCounts {
                participants: 0,
                messages: 0,
                nodes: ast.entities().len() as u64,
                edges: ast.relationships().len() as u64,
                classes: 0,
                relations: 0,
                entities: ast.entities().len() as u64,
                relationships: ast.relationships().len() as u64,
                sections: 0,
                tasks: 0,
                dependencies: 0,
                lanes: 0,
            },
            key_names: ast.entities().values().map(|e| e.name().to_owned()).collect(),
            context: ReadContext::default(),
        },
        DiagramAst::Gantt(ast) => DiagramDigest {
            rev: diagram.rev(),
            counts: DiagramCounts {
                participants: 0,
                messages: 0,
                nodes: ast.tasks().len() as u64,
                edges: ast
                    .tasks()
                    .values()
                    .filter(|task| matches!(task.start(), crate::model::GanttTaskStart::After(_)))
                    .count() as u64,
                classes: 0,
                relations: 0,
                entities: 0,
                relationships: 0,
                sections: ast.sections().len() as u64,
                tasks: ast.tasks().len() as u64,
                dependencies: ast
                    .tasks()
                    .values()
                    .filter(|task| matches!(task.start(), crate::model::GanttTaskStart::After(_)))
                    .count() as u64,
                lanes: ast.lanes().len() as u64,
            },
            key_names: ast.tasks().values().map(|t| t.name().to_owned()).collect(),
            context: ReadContext::default(),
        },
    }
}

fn digest_for_walkthrough(walkthrough: &crate::model::Walkthrough) -> WalkthroughDigest {
    WalkthroughDigest {
        rev: walkthrough.rev(),
        counts: WalkthroughDigestCounts {
            nodes: walkthrough.nodes().len() as u64,
            edges: walkthrough.edges().len() as u64,
        },
    }
}

fn mermaid_export_error(kind: &str, err: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(
        format!("failed to export {kind} diagram as Mermaid: {err}"),
        Some(serde_json::json!({ "kind": kind, "export_error": err.to_string() })),
    )
}

/// Canonical Mermaid for `diagram_read`. Fail closed on export errors so agents never
/// treat an empty skeleton as a successful snapshot of a non-exportable AST.
fn mermaid_for_diagram(diagram: &Diagram) -> Result<String, ErrorData> {
    match diagram.ast() {
        DiagramAst::Sequence(ast) => mermaid_for_sequence(ast),
        DiagramAst::Flowchart(ast) => mermaid_for_flowchart(ast),
        DiagramAst::Class(ast) => mermaid_for_class(ast),
        DiagramAst::Er(ast) => mermaid_for_er(ast),
        DiagramAst::Gantt(ast) => mermaid_for_gantt(ast),
    }
}

fn mermaid_for_er(ast: &crate::model::ErAst) -> Result<String, ErrorData> {
    crate::format::mermaid::export_er_diagram(ast).map_err(|err| mermaid_export_error("ER", err))
}

fn mermaid_for_gantt(ast: &crate::model::GanttAst) -> Result<String, ErrorData> {
    crate::format::mermaid::export_gantt_diagram(ast)
        .map_err(|err| mermaid_export_error("Gantt", err))
}

fn mermaid_for_class(ast: &crate::model::ClassAst) -> Result<String, ErrorData> {
    crate::format::mermaid::export_class_diagram(ast)
        .map_err(|err| mermaid_export_error("Class", err))
}

fn mcp_ast_for_diagram(diagram: &Diagram) -> McpDiagramAst {
    match diagram.ast() {
        DiagramAst::Sequence(ast) => {
            let mut participants = ast
                .participants()
                .iter()
                .map(|(participant_id, participant)| McpSeqParticipantAst {
                    participant_id: participant_id.to_string(),
                    mermaid_name: participant.mermaid_name().to_owned(),
                    role: participant.role().map(ToOwned::to_owned),
                    note: participant.note().map(ToOwned::to_owned),
                    symbol: participant.symbol().map(mcp_symbol_anchor),
                })
                .collect::<Vec<_>>();
            participants.sort_by(|a, b| a.participant_id.cmp(&b.participant_id));

            let mut messages = ast
                .messages()
                .iter()
                .map(|message| McpSeqMessageAst {
                    message_id: message.message_id().to_string(),
                    from_participant_id: message.from_participant_id().to_string(),
                    to_participant_id: message.to_participant_id().to_string(),
                    kind: map_message_kind_to_mcp(message.kind()),
                    arrow: message.raw_arrow().map(ToOwned::to_owned),
                    text: message.text().to_owned(),
                    order_key: message.order_key(),
                })
                .collect::<Vec<_>>();
            messages.sort_by(|a, b| {
                a.order_key.cmp(&b.order_key).then_with(|| a.message_id.cmp(&b.message_id))
            });

            let mut blocks = ast.blocks().iter().map(map_seq_block_to_mcp).collect::<Vec<_>>();
            blocks.sort_by(|a, b| a.block_id.cmp(&b.block_id));

            McpDiagramAst::Sequence { participants, messages, blocks }
        }
        DiagramAst::Flowchart(ast) => {
            let mut nodes = ast
                .nodes()
                .iter()
                .map(|(node_id, node)| McpFlowNodeAst {
                    node_id: node_id.to_string(),
                    label: node.label().to_owned(),
                    shape: node.shape().to_owned(),
                    mermaid_id: node.mermaid_id().map(ToOwned::to_owned),
                    note: node.note().map(ToOwned::to_owned),
                    symbol: node.symbol().map(mcp_symbol_anchor),
                })
                .collect::<Vec<_>>();
            nodes.sort_by(|a, b| a.node_id.cmp(&b.node_id));

            let mut edges = ast
                .edges()
                .iter()
                .map(|(edge_id, edge)| McpFlowEdgeAst {
                    edge_id: edge_id.to_string(),
                    from_node_id: edge.from_node_id().to_string(),
                    to_node_id: edge.to_node_id().to_string(),
                    label: edge.label().map(ToOwned::to_owned),
                    connector: edge.connector().map(ToOwned::to_owned),
                    style: edge.style().map(ToOwned::to_owned),
                })
                .collect::<Vec<_>>();
            edges.sort_by(|a, b| a.edge_id.cmp(&b.edge_id));

            McpDiagramAst::Flowchart { nodes, edges }
        }
        DiagramAst::Class(ast) => {
            let mut classes = ast
                .classes()
                .iter()
                .map(|(class_id, class)| McpClassNodeAst {
                    class_id: class_id.to_string(),
                    name: class.name().to_owned(),
                    attributes: class.attributes().to_vec(),
                    methods: class.methods().to_vec(),
                    note: class.note().map(str::to_owned),
                })
                .collect::<Vec<_>>();
            classes.sort_by(|a, b| a.class_id.cmp(&b.class_id));

            let mut relations = ast
                .relations()
                .iter()
                .map(|(rel_id, rel)| McpClassRelationAst {
                    relation_id: rel_id.to_string(),
                    from_class_id: rel.from_class_id().to_string(),
                    to_class_id: rel.to_class_id().to_string(),
                    kind: map_class_relation_kind_to_mcp(rel.kind()),
                    label: rel.label().map(ToOwned::to_owned),
                    raw_connector: rel.raw_connector().map(ToOwned::to_owned),
                })
                .collect::<Vec<_>>();
            relations.sort_by(|a, b| a.relation_id.cmp(&b.relation_id));

            McpDiagramAst::Class { classes, relations }
        }
        DiagramAst::Er(ast) => {
            let mut entities = ast
                .entities()
                .iter()
                .map(|(id, entity)| McpErEntityAst {
                    entity_id: id.to_string(),
                    name: entity.name().to_owned(),
                    note: entity.note().map(str::to_owned),
                })
                .collect::<Vec<_>>();
            entities.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
            let mut relationships = ast
                .relationships()
                .iter()
                .map(|(id, relationship)| McpErRelationshipAst {
                    relationship_id: id.to_string(),
                    from_entity_id: relationship.from_entity_id().to_string(),
                    to_entity_id: relationship.to_entity_id().to_string(),
                    from_cardinality: map_er_cardinality_to_mcp(relationship.from_card()),
                    to_cardinality: map_er_cardinality_to_mcp(relationship.to_card()),
                    stroke: map_er_stroke_to_mcp(relationship.stroke()),
                    label: relationship.label().map(ToOwned::to_owned),
                    raw_connector: relationship.raw_connector().map(ToOwned::to_owned),
                })
                .collect::<Vec<_>>();
            relationships.sort_by(|a, b| a.relationship_id.cmp(&b.relationship_id));
            McpDiagramAst::Er { entities, relationships }
        }
        DiagramAst::Gantt(ast) => {
            let sections = ast
                .sections()
                .iter()
                .map(|section| McpGanttSectionAst {
                    section_id: section.section_id().to_string(),
                    name: section.name().to_owned(),
                    task_ids: section.task_ids().iter().map(ToString::to_string).collect(),
                })
                .collect();
            let mut tasks = ast
                .tasks()
                .iter()
                .map(|(id, task)| McpGanttTaskAst {
                    task_id: id.to_string(),
                    mermaid_tag: task.mermaid_tag().map(ToOwned::to_owned),
                    name: task.name().to_owned(),
                    start: map_gantt_task_start_to_mcp(task.start()),
                    duration_days: task.duration_days(),
                    raw_duration: task.raw_duration().to_owned(),
                    note: task.note().map(str::to_owned),
                })
                .collect::<Vec<_>>();
            tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
            let lanes = ast
                .lanes()
                .into_iter()
                .map(|(lane_id, label)| McpGanttLaneAst {
                    note: ast.lane_note(&lane_id).map(ToOwned::to_owned),
                    lane_id: lane_id.to_string(),
                    label,
                })
                .collect();
            McpDiagramAst::Gantt {
                title: ast.title().map(ToOwned::to_owned),
                date_format: ast.date_format().map(ToOwned::to_owned),
                sections,
                tasks,
                lanes,
            }
        }
    }
}

fn map_class_relation_kind_to_mcp(kind: crate::model::ClassRelationKind) -> McpClassRelationKind {
    match kind {
        crate::model::ClassRelationKind::Inheritance => McpClassRelationKind::Inheritance,
        crate::model::ClassRelationKind::Composition => McpClassRelationKind::Composition,
        crate::model::ClassRelationKind::Aggregation => McpClassRelationKind::Aggregation,
        crate::model::ClassRelationKind::Association => McpClassRelationKind::Association,
        crate::model::ClassRelationKind::Dependency => McpClassRelationKind::Dependency,
        crate::model::ClassRelationKind::Realization => McpClassRelationKind::Realization,
        crate::model::ClassRelationKind::Link => McpClassRelationKind::Link,
    }
}

fn map_er_cardinality_to_mcp(cardinality: crate::model::ErCardinality) -> McpErCardinality {
    match cardinality {
        crate::model::ErCardinality::ExactlyOne => McpErCardinality::ExactlyOne,
        crate::model::ErCardinality::ZeroOrOne => McpErCardinality::ZeroOrOne,
        crate::model::ErCardinality::OneOrMore => McpErCardinality::OneOrMore,
        crate::model::ErCardinality::ZeroOrMore => McpErCardinality::ZeroOrMore,
    }
}

fn map_er_stroke_to_mcp(stroke: crate::model::ErStroke) -> McpErStroke {
    match stroke {
        crate::model::ErStroke::Identifying => McpErStroke::Identifying,
        crate::model::ErStroke::NonIdentifying => McpErStroke::NonIdentifying,
    }
}

fn map_gantt_task_start_to_mcp(start: &crate::model::GanttTaskStart) -> McpGanttTaskStart {
    match start {
        crate::model::GanttTaskStart::Date(date) => McpGanttTaskStart::Date { date: date.clone() },
        crate::model::GanttTaskStart::After(task_id) => {
            McpGanttTaskStart::After { task_id: task_id.to_string() }
        }
        crate::model::GanttTaskStart::Unspecified => McpGanttTaskStart::Unspecified,
    }
}

fn mcp_symbol_anchor(symbol: &SymbolAnchor) -> McpSymbolAnchor {
    McpSymbolAnchor {
        stable_symbol_id: symbol.stable_symbol_id().to_owned(),
        repository_id: symbol.repository_id().map(ToOwned::to_owned),
    }
}

fn internal_symbol_anchor(symbol: &McpSymbolAnchor) -> Result<SymbolAnchor, ErrorData> {
    SymbolAnchor::new(symbol.stable_symbol_id.clone(), symbol.repository_id.clone()).map_err(
        |err| {
            ErrorData::invalid_params(
                "invalid stable_symbol_id",
                Some(serde_json::json!({
                    "stable_symbol_id": symbol.stable_symbol_id,
                    "reason": err.to_string(),
                })),
            )
        },
    )
}

fn map_seq_block_kind_to_mcp(kind: crate::model::seq_ast::SequenceBlockKind) -> McpSeqBlockKind {
    match kind {
        crate::model::seq_ast::SequenceBlockKind::Alt => McpSeqBlockKind::Alt,
        crate::model::seq_ast::SequenceBlockKind::Opt => McpSeqBlockKind::Opt,
        crate::model::seq_ast::SequenceBlockKind::Loop => McpSeqBlockKind::Loop,
        crate::model::seq_ast::SequenceBlockKind::Par => McpSeqBlockKind::Par,
    }
}

fn map_mcp_seq_block_kind(kind: McpSeqBlockKind) -> crate::model::seq_ast::SequenceBlockKind {
    match kind {
        McpSeqBlockKind::Alt => crate::model::seq_ast::SequenceBlockKind::Alt,
        McpSeqBlockKind::Opt => crate::model::seq_ast::SequenceBlockKind::Opt,
        McpSeqBlockKind::Loop => crate::model::seq_ast::SequenceBlockKind::Loop,
        McpSeqBlockKind::Par => crate::model::seq_ast::SequenceBlockKind::Par,
    }
}

fn map_mcp_seq_section_add_kind(
    kind: McpSeqSectionAddKind,
) -> crate::model::seq_ast::SequenceSectionKind {
    match kind {
        McpSeqSectionAddKind::Else => crate::model::seq_ast::SequenceSectionKind::Else,
        McpSeqSectionAddKind::And => crate::model::seq_ast::SequenceSectionKind::And,
    }
}

fn map_seq_section_kind_to_mcp(
    kind: crate::model::seq_ast::SequenceSectionKind,
) -> McpSeqSectionKind {
    match kind {
        crate::model::seq_ast::SequenceSectionKind::Main => McpSeqSectionKind::Main,
        crate::model::seq_ast::SequenceSectionKind::Else => McpSeqSectionKind::Else,
        crate::model::seq_ast::SequenceSectionKind::And => McpSeqSectionKind::And,
    }
}

fn map_seq_block_to_mcp(block: &crate::model::seq_ast::SequenceBlock) -> McpSeqBlockAst {
    let mut sections = block
        .sections()
        .iter()
        .map(|section| McpSeqSectionAst {
            section_id: section.section_id().to_string(),
            kind: map_seq_section_kind_to_mcp(section.kind()),
            header: section.header().map(ToOwned::to_owned),
            message_ids: section.message_ids().iter().map(ToString::to_string).collect(),
        })
        .collect::<Vec<_>>();
    sections.sort_by(|a, b| a.section_id.cmp(&b.section_id));

    let mut blocks = block.blocks().iter().map(map_seq_block_to_mcp).collect::<Vec<_>>();
    blocks.sort_by(|a, b| a.block_id.cmp(&b.block_id));

    McpSeqBlockAst {
        block_id: block.block_id().to_string(),
        kind: map_seq_block_kind_to_mcp(block.kind()),
        header: block.header().map(ToOwned::to_owned),
        sections,
        blocks,
    }
}

fn mermaid_for_sequence(ast: &crate::model::SequenceAst) -> Result<String, ErrorData> {
    // Same exporter as store/TUI; no messages-only reconstruction that drops blocks.
    crate::format::mermaid::export_sequence_diagram(ast)
        .map_err(|err| mermaid_export_error("Sequence", err))
}

fn mermaid_for_flowchart(ast: &crate::model::FlowchartAst) -> Result<String, ErrorData> {
    // Prefer the same exporter as store/TUI so mermaid_id (not object id) is used.
    // Fail closed on export error rather than emitting a skeleton or wrong identities.
    crate::format::mermaid::export_flowchart(ast)
        .map_err(|err| mermaid_export_error("Flowchart", err))
}

fn delta_response_from_history(
    history: &VecDeque<LastDelta>,
    diagram_id: &DiagramId,
    since_rev: u64,
    current_rev: u64,
) -> Option<DiagramDeltaResponse> {
    let start_index = history.iter().position(|d| d.from_rev == since_rev)?;

    let mut expected_from = since_rev;
    let mut added = BTreeSet::<String>::new();
    let mut removed = BTreeSet::<String>::new();
    let mut updated = BTreeSet::<String>::new();

    for delta in history.iter().skip(start_index) {
        if delta.from_rev != expected_from {
            return None;
        }

        for r in &delta.delta.added {
            added.insert(r.to_string());
        }
        for r in &delta.delta.removed {
            removed.insert(r.to_string());
        }
        for r in &delta.delta.updated {
            updated.insert(r.to_string());
        }
        if delta.diagram_metadata_updated {
            updated.insert(diagram_meta_ref(diagram_id));
        }

        expected_from = delta.to_rev;
        if expected_from == current_rev {
            break;
        }
    }

    if expected_from != current_rev {
        return None;
    }

    let mut changes = Vec::new();
    if !added.is_empty() {
        changes
            .push(DeltaChange { kind: DeltaChangeKind::Added, refs: added.into_iter().collect() });
    }
    if !removed.is_empty() {
        changes.push(DeltaChange {
            kind: DeltaChangeKind::Removed,
            refs: removed.into_iter().collect(),
        });
    }
    if !updated.is_empty() {
        changes.push(DeltaChange {
            kind: DeltaChangeKind::Updated,
            refs: updated.into_iter().collect(),
        });
    }

    Some(DiagramDeltaResponse { from_rev: since_rev, to_rev: current_rev, changes })
}

fn diagram_meta_ref(diagram_id: &DiagramId) -> String {
    format!("d:{}/meta", diagram_id.as_str())
}

fn walkthrough_delta_response_from_history(
    history: &VecDeque<WalkthroughLastDelta>,
    since_rev: u64,
    current_rev: u64,
) -> Option<WalkthroughDeltaResponse> {
    let start_index = history.iter().position(|d| d.from_rev == since_rev)?;

    let mut expected_from = since_rev;
    let mut added = BTreeSet::<String>::new();
    let mut removed = BTreeSet::<String>::new();
    let mut updated = BTreeSet::<String>::new();

    for delta in history.iter().skip(start_index) {
        if delta.from_rev != expected_from {
            return None;
        }

        added.extend(delta.delta.added.iter().cloned());
        removed.extend(delta.delta.removed.iter().cloned());
        updated.extend(delta.delta.updated.iter().cloned());

        expected_from = delta.to_rev;
        if expected_from == current_rev {
            break;
        }
    }

    if expected_from != current_rev {
        return None;
    }

    let mut changes = Vec::new();
    if !added.is_empty() {
        changes
            .push(DeltaChange { kind: DeltaChangeKind::Added, refs: added.into_iter().collect() });
    }
    if !removed.is_empty() {
        changes.push(DeltaChange {
            kind: DeltaChangeKind::Removed,
            refs: removed.into_iter().collect(),
        });
    }
    if !updated.is_empty() {
        changes.push(DeltaChange {
            kind: DeltaChangeKind::Updated,
            refs: updated.into_iter().collect(),
        });
    }

    Some(WalkthroughDeltaResponse { from_rev: since_rev, to_rev: current_rev, changes })
}

fn delta_unavailable(since_rev: u64, current_rev: u64, supported_since_rev: u64) -> ErrorData {
    ErrorData::invalid_request(
        "delta unavailable; use diagram_read",
        Some(serde_json::json!({
            "since_rev": since_rev,
            "current_rev": current_rev,
            "supported_since_rev": supported_since_rev,
            "snapshot_tool": "diagram_read",
        })),
    )
}

fn walkthrough_delta_unavailable(
    since_rev: u64,
    current_rev: u64,
    supported_since_rev: u64,
) -> ErrorData {
    ErrorData::invalid_request(
        "delta unavailable; use walkthrough_read",
        Some(serde_json::json!({
            "since_rev": since_rev,
            "current_rev": current_rev,
            "supported_since_rev": supported_since_rev,
            "snapshot_tool": "walkthrough_read",
        })),
    )
}

fn walkthrough_meta_ref(walkthrough_id: &WalkthroughId) -> String {
    format!("w:{}/meta", walkthrough_id.as_str())
}

fn walkthrough_node_ref(walkthrough_id: &WalkthroughId, node_id: &WalkthroughNodeId) -> String {
    format!("w:{}/node/{}", walkthrough_id.as_str(), node_id.as_str())
}

fn walkthrough_edge_ref(
    walkthrough_id: &WalkthroughId,
    from_node_id: &WalkthroughNodeId,
    to_node_id: &WalkthroughNodeId,
    kind: &str,
) -> String {
    format!(
        "w:{}/edge/{}/{}/{}",
        walkthrough_id.as_str(),
        from_node_id.as_str(),
        to_node_id.as_str(),
        kind
    )
}

fn validate_walkthrough_edge_kind(kind: &str) -> Result<(), ErrorData> {
    if kind.is_empty() || kind.contains('/') {
        return Err(ErrorData::invalid_params(
            "invalid edge kind (must be non-empty and not contain '/')",
            Some(serde_json::json!({ "kind": kind })),
        ));
    }

    Ok(())
}

fn apply_walkthrough_ops(
    walkthrough: &mut Walkthrough,
    walkthrough_id: &WalkthroughId,
    ops: &[McpWalkthroughOp],
) -> Result<WalkthroughDelta, ErrorData> {
    let mut delta = WalkthroughDelta::default();

    for op in ops {
        match op {
            McpWalkthroughOp::SetTitle { title } => {
                walkthrough.set_title(title.clone());
                delta.updated.insert(walkthrough_meta_ref(walkthrough_id));
            }
            McpWalkthroughOp::AddNode { node_id, title, body_md, refs, tags, status } => {
                let parsed_node_id = parse_walkthrough_node_id(node_id)?;

                if walkthrough.nodes().iter().any(|node| node.node_id() == &parsed_node_id) {
                    return Err(ErrorData::invalid_params(
                        "node_id already exists",
                        Some(serde_json::json!({
                            "walkthrough_id": walkthrough_id.as_str(),
                            "node_id": node_id,
                        })),
                    ));
                }

                let mut node = WalkthroughNode::new(parsed_node_id.clone(), title.as_str());
                node.set_body_md(body_md.clone());
                node.set_status(status.clone());

                if let Some(refs) = refs {
                    for raw_ref in refs {
                        node.refs_mut().push(parse_object_ref(raw_ref)?);
                    }
                }

                if let Some(tags) = tags {
                    for tag in tags {
                        node.tags_mut().push(tag.clone());
                    }
                }

                walkthrough.nodes_mut().push(node);
                delta.added.insert(walkthrough_node_ref(walkthrough_id, &parsed_node_id));
            }
            McpWalkthroughOp::UpdateNode { node_id, title, body_md, refs, tags, status } => {
                let parsed_node_id = parse_walkthrough_node_id(node_id)?;
                let (
                    node_index,
                    existing_title,
                    existing_body_md,
                    existing_refs,
                    existing_tags,
                    existing_status,
                ) = walkthrough
                    .nodes()
                    .iter()
                    .enumerate()
                    .find(|(_, node)| node.node_id() == &parsed_node_id)
                    .map(|(idx, node)| {
                        (
                            idx,
                            node.title().to_owned(),
                            node.body_md().map(|body| body.to_owned()),
                            node.refs().to_vec(),
                            node.tags().to_vec(),
                            node.status().map(|status| status.to_owned()),
                        )
                    })
                    .ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "walkthrough node not found",
                            Some(serde_json::json!({
                                "walkthrough_id": walkthrough_id.as_str(),
                                "node_id": node_id,
                            })),
                        )
                    })?;

                let new_title = title.clone().unwrap_or(existing_title);
                let new_body_md = body_md.clone().unwrap_or(existing_body_md);
                let new_refs = if let Some(raw_refs) = refs {
                    raw_refs
                        .iter()
                        .map(|raw_ref| parse_object_ref(raw_ref))
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    existing_refs
                };
                let new_tags = tags.clone().unwrap_or(existing_tags);
                let new_status = status.clone().unwrap_or(existing_status);

                let mut replacement =
                    WalkthroughNode::new(parsed_node_id.clone(), new_title.as_str());
                replacement.set_body_md(new_body_md);
                replacement.set_status(new_status);
                for r in new_refs {
                    replacement.refs_mut().push(r);
                }
                for t in new_tags {
                    replacement.tags_mut().push(t);
                }

                walkthrough.nodes_mut()[node_index] = replacement;
                delta.updated.insert(walkthrough_node_ref(walkthrough_id, &parsed_node_id));
            }
            McpWalkthroughOp::RemoveNode { node_id } => {
                let parsed_node_id = parse_walkthrough_node_id(node_id)?;
                let node_index = walkthrough
                    .nodes()
                    .iter()
                    .position(|node| node.node_id() == &parsed_node_id)
                    .ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "walkthrough node not found",
                            Some(serde_json::json!({
                                "walkthrough_id": walkthrough_id.as_str(),
                                "node_id": node_id,
                            })),
                        )
                    })?;
                walkthrough.nodes_mut().remove(node_index);
                delta.removed.insert(walkthrough_node_ref(walkthrough_id, &parsed_node_id));

                let mut removed_edges = Vec::new();
                walkthrough.edges_mut().retain(|edge| {
                    let incident = edge.from_node_id() == &parsed_node_id
                        || edge.to_node_id() == &parsed_node_id;
                    if incident {
                        removed_edges.push((
                            edge.from_node_id().clone(),
                            edge.to_node_id().clone(),
                            edge.kind().to_owned(),
                        ));
                    }
                    !incident
                });

                for (from, to, kind) in removed_edges {
                    delta.removed.insert(walkthrough_edge_ref(walkthrough_id, &from, &to, &kind));
                }
            }
            McpWalkthroughOp::AddEdge { from_node_id, to_node_id, kind, label } => {
                validate_walkthrough_edge_kind(kind)?;
                let parsed_from = parse_walkthrough_node_id(from_node_id)?;
                let parsed_to = parse_walkthrough_node_id(to_node_id)?;

                if !walkthrough.nodes().iter().any(|node| node.node_id() == &parsed_from) {
                    return Err(ErrorData::resource_not_found(
                        "walkthrough node not found",
                        Some(serde_json::json!({
                            "walkthrough_id": walkthrough_id.as_str(),
                            "node_id": from_node_id,
                        })),
                    ));
                }

                if !walkthrough.nodes().iter().any(|node| node.node_id() == &parsed_to) {
                    return Err(ErrorData::resource_not_found(
                        "walkthrough node not found",
                        Some(serde_json::json!({
                            "walkthrough_id": walkthrough_id.as_str(),
                            "node_id": to_node_id,
                        })),
                    ));
                }

                if walkthrough.edges().iter().any(|edge| {
                    edge.from_node_id() == &parsed_from
                        && edge.to_node_id() == &parsed_to
                        && edge.kind() == kind
                }) {
                    return Err(ErrorData::invalid_params(
                        "edge already exists",
                        Some(serde_json::json!({
                            "walkthrough_id": walkthrough_id.as_str(),
                            "from_node_id": from_node_id,
                            "to_node_id": to_node_id,
                            "kind": kind,
                        })),
                    ));
                }

                let mut edge =
                    WalkthroughEdge::new(parsed_from.clone(), parsed_to.clone(), kind.as_str());
                edge.set_label(label.clone());
                walkthrough.edges_mut().push(edge);

                delta.added.insert(walkthrough_edge_ref(
                    walkthrough_id,
                    &parsed_from,
                    &parsed_to,
                    kind,
                ));
            }
            McpWalkthroughOp::UpdateEdge { from_node_id, to_node_id, kind, label } => {
                validate_walkthrough_edge_kind(kind)?;
                let parsed_from = parse_walkthrough_node_id(from_node_id)?;
                let parsed_to = parse_walkthrough_node_id(to_node_id)?;

                let edge_index = walkthrough
                    .edges()
                    .iter()
                    .position(|edge| {
                        edge.from_node_id() == &parsed_from
                            && edge.to_node_id() == &parsed_to
                            && edge.kind() == kind
                    })
                    .ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "walkthrough edge not found",
                            Some(serde_json::json!({
                                "walkthrough_id": walkthrough_id.as_str(),
                                "from_node_id": from_node_id,
                                "to_node_id": to_node_id,
                                "kind": kind,
                            })),
                        )
                    })?;

                if let Some(label) = label {
                    walkthrough.edges_mut()[edge_index].set_label(label.clone());
                }

                delta.updated.insert(walkthrough_edge_ref(
                    walkthrough_id,
                    &parsed_from,
                    &parsed_to,
                    kind,
                ));
            }
            McpWalkthroughOp::RemoveEdge { from_node_id, to_node_id, kind } => {
                validate_walkthrough_edge_kind(kind)?;
                let parsed_from = parse_walkthrough_node_id(from_node_id)?;
                let parsed_to = parse_walkthrough_node_id(to_node_id)?;

                let edge_index = walkthrough
                    .edges()
                    .iter()
                    .position(|edge| {
                        edge.from_node_id() == &parsed_from
                            && edge.to_node_id() == &parsed_to
                            && edge.kind() == kind
                    })
                    .ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "walkthrough edge not found",
                            Some(serde_json::json!({
                                "walkthrough_id": walkthrough_id.as_str(),
                                "from_node_id": from_node_id,
                                "to_node_id": to_node_id,
                                "kind": kind,
                            })),
                        )
                    })?;

                walkthrough.edges_mut().remove(edge_index);
                delta.removed.insert(walkthrough_edge_ref(
                    walkthrough_id,
                    &parsed_from,
                    &parsed_to,
                    kind,
                ));
            }
        }
    }

    Ok(delta)
}

fn identity_report_from_sets(
    previous: &std::collections::BTreeSet<String>,
    next: &std::collections::BTreeSet<String>,
) -> DiagramIdentityReport {
    let newly_allocated = next.difference(previous).cloned().collect::<Vec<_>>();
    let dropped = previous.difference(next).cloned().collect::<Vec<_>>();
    let preserved = previous.intersection(next).cloned().collect::<Vec<_>>();
    DiagramIdentityReport { newly_allocated, dropped, preserved }
}

fn dangling_xref_ids_for_diagram(session: &Session, diagram_id: &DiagramId) -> Vec<String> {
    session
        .xrefs()
        .iter()
        .filter(|(_, xref)| {
            if xref.status() == crate::model::XRefStatus::Ok {
                return false;
            }
            xref.from().diagram_id() == diagram_id || xref.to().diagram_id() == diagram_id
        })
        .map(|(xref_id, _)| xref_id.to_string())
        .collect()
}

fn map_replace_error(err: crate::store::DiagramMermaidReplaceError) -> ErrorData {
    use crate::store::DiagramMermaidReplaceError;
    match err {
        DiagramMermaidReplaceError::KindMismatch { expected, found } => ErrorData::invalid_params(
            "mermaid kind does not match existing diagram",
            Some(serde_json::json!({
                "expected": format!("{expected:?}"),
                "found": format!("{found:?}"),
            })),
        ),
        DiagramMermaidReplaceError::MissingOrUnknownKind { first_line } => ErrorData::invalid_params(
            "expected 'flowchart'/'graph', 'sequenceDiagram', 'classDiagram', 'erDiagram', or 'gantt' as the first non-empty line",
            Some(serde_json::json!({ "first_line": first_line })),
        ),
        DiagramMermaidReplaceError::ParseSequence(err) => ErrorData::invalid_params(
            format!("cannot parse Mermaid sequence diagram: {err}"),
            None,
        ),
        DiagramMermaidReplaceError::ParseClass(err) => ErrorData::invalid_params(
            format!("cannot parse Mermaid class diagram: {err}"),
            None,
        ),
        DiagramMermaidReplaceError::ParseEr(err) => ErrorData::invalid_params(
            format!("cannot parse Mermaid er diagram: {err}"),
            None,
        ),
        DiagramMermaidReplaceError::ParseGantt(err) => ErrorData::invalid_params(
            format!("cannot parse Mermaid gantt diagram: {err}"),
            None,
        ),
        DiagramMermaidReplaceError::ParseFlowchart(err) => ErrorData::invalid_params(
            format!("cannot parse Mermaid flowchart diagram: {err}"),
            None,
        ),
        DiagramMermaidReplaceError::AstKindMismatch(err) => {
            ErrorData::invalid_params(err.to_string(), None)
        }
    }
}

fn map_apply_error(err: ApplyError) -> ErrorData {
    match err {
        ApplyError::Conflict { base_rev, current_rev } => ErrorData::invalid_request(
            "conflict: stale base_rev",
            Some(serde_json::json!({ "base_rev": base_rev, "current_rev": current_rev })),
        ),
        ApplyError::KindMismatch { diagram_kind, op_kind } => ErrorData::invalid_params(
            "op kind mismatch for diagram kind",
            Some(
                serde_json::json!({ "diagram_kind": format!("{diagram_kind:?}"), "op_kind": format!("{op_kind:?}") }),
            ),
        ),
        ApplyError::UnsupportedOp { op_kind } => ErrorData::invalid_params(
            "unsupported op kind",
            Some(serde_json::json!({ "op_kind": format!("{op_kind:?}") })),
        ),
        ApplyError::AlreadyExists { kind, object_id } => ErrorData::invalid_params(
            "object already exists",
            Some(
                serde_json::json!({ "kind": format!("{kind:?}"), "object_id": object_id.to_string() }),
            ),
        ),
        ApplyError::NotFound { kind, object_id } => ErrorData::resource_not_found(
            "object not found",
            Some(
                serde_json::json!({ "kind": format!("{kind:?}"), "object_id": object_id.to_string() }),
            ),
        ),
        ApplyError::MissingFlowNode { node_id } => ErrorData::resource_not_found(
            "flow node not found",
            Some(serde_json::json!({ "node_id": node_id.to_string() })),
        ),
        ApplyError::InvalidSeqParticipantMermaidName { mermaid_name, reason } => {
            ErrorData::invalid_params(
                "invalid sequence participant Mermaid name",
                Some(serde_json::json!({
                    "mermaid_name": mermaid_name,
                    "reason": reason.to_string(),
                })),
            )
        }
        ApplyError::DuplicateSeqParticipantMermaidName { mermaid_name, participant_id } => {
            ErrorData::invalid_params(
                "sequence participant Mermaid name already in use",
                Some(serde_json::json!({
                    "mermaid_name": mermaid_name,
                    "participant_id": participant_id.to_string(),
                })),
            )
        }
        ApplyError::InvalidFlowNodeMermaidId { mermaid_id, reason } => ErrorData::invalid_params(
            "invalid flow node Mermaid id",
            Some(serde_json::json!({ "mermaid_id": mermaid_id, "reason": reason.to_string() })),
        ),
        ApplyError::DuplicateFlowNodeMermaidId { mermaid_id, node_id } => {
            ErrorData::invalid_params(
                "flow node Mermaid id already in use",
                Some(
                    serde_json::json!({ "mermaid_id": mermaid_id, "node_id": node_id.to_string() }),
                ),
            )
        }
        ApplyError::InvalidSymbolAnchor { source } => ErrorData::invalid_params(
            "invalid symbol anchor",
            Some(serde_json::json!({ "reason": source.to_string() })),
        ),
        ApplyError::InvalidSeqSectionKind { block_id, block_kind, section_kind } => {
            ErrorData::invalid_params(
                "invalid section kind for block",
                Some(serde_json::json!({
                    "block_id": block_id.to_string(),
                    "block_kind": format!("{block_kind:?}"),
                    "section_kind": format!("{section_kind:?}"),
                })),
            )
        }
        ApplyError::InvalidSeqSectionNotEmpty { section_id } => ErrorData::invalid_params(
            "section still has messages",
            Some(serde_json::json!({ "section_id": section_id.to_string() })),
        ),
        ApplyError::InvalidSeqSectionIsMain { section_id } => ErrorData::invalid_params(
            "cannot remove main section",
            Some(serde_json::json!({ "section_id": section_id.to_string() })),
        ),
        ApplyError::InvalidSeqBlockNesting { block_id, reason } => ErrorData::invalid_params(
            "invalid sequence block nesting",
            Some(serde_json::json!({
                "block_id": block_id.to_string(),
                "reason": reason,
            })),
        ),
        ApplyError::InvalidSeqBlockStructure { reason } => ErrorData::invalid_params(
            "invalid sequence block structure",
            Some(serde_json::json!({
                "reason": reason,
                "hint": "section messages must be contiguous in order_key order",
            })),
        ),
        ApplyError::InvalidSeqMessageText { text } => ErrorData::invalid_params(
            "sequence message text must be non-empty",
            Some(serde_json::json!({ "text": text })),
        ),
    }
}

fn parse_object_id(value: &str) -> Result<ObjectId, ErrorData> {
    ObjectId::new(value.to_owned()).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid object id: {err}"),
            Some(serde_json::json!({ "object_id": value })),
        )
    })
}

fn parse_xref_id(value: &str) -> Result<XRefId, ErrorData> {
    XRefId::new(value.to_owned()).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid xref_id: {err}"),
            Some(serde_json::json!({ "xref_id": value })),
        )
    })
}

fn parse_walkthrough_id(value: &str) -> Result<WalkthroughId, ErrorData> {
    WalkthroughId::new(value.to_owned()).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid walkthrough_id: {err}"),
            Some(serde_json::json!({ "walkthrough_id": value })),
        )
    })
}

fn parse_walkthrough_node_id(value: &str) -> Result<WalkthroughNodeId, ErrorData> {
    WalkthroughNodeId::new(value.to_owned()).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid node_id: {err}"),
            Some(serde_json::json!({ "node_id": value })),
        )
    })
}

fn parse_object_ref(value: &str) -> Result<ObjectRef, ErrorData> {
    ObjectRef::parse(value).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid object_ref: {err}"),
            Some(serde_json::json!({ "object_ref": value })),
        )
    })
}

fn parse_object_ref_from(value: &str) -> Result<ObjectRef, ErrorData> {
    ObjectRef::parse(value).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid from: {err}"),
            Some(serde_json::json!({ "from": value })),
        )
    })
}

fn parse_object_ref_to(value: &str) -> Result<ObjectRef, ErrorData> {
    ObjectRef::parse(value).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid to: {err}"),
            Some(serde_json::json!({ "to": value })),
        )
    })
}

fn parse_object_ref_from_ref(value: &str) -> Result<ObjectRef, ErrorData> {
    ObjectRef::parse(value).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid from_ref: {err}"),
            Some(serde_json::json!({ "from_ref": value })),
        )
    })
}

fn parse_object_ref_to_ref(value: &str) -> Result<ObjectRef, ErrorData> {
    ObjectRef::parse(value).map_err(|err| {
        ErrorData::invalid_params(
            format!("invalid to_ref: {err}"),
            Some(serde_json::json!({ "to_ref": value })),
        )
    })
}

fn object_ref_is_missing(session: &Session, object_ref: &ObjectRef) -> bool {
    session.object_ref_is_missing(object_ref)
}

fn retain_existing_selected_object_refs(session: &mut Session) {
    let retained = session
        .selected_object_refs()
        .iter()
        .filter(|object_ref| session.object_ref_exists(object_ref))
        .cloned()
        .collect();
    session.set_selected_object_refs(retained);
}

fn refresh_xref_statuses(session: &mut Session) {
    let next_statuses = session
        .xrefs()
        .iter()
        .map(|(xref_id, xref)| {
            let from_missing = session.object_ref_is_missing(xref.from());
            let to_missing = session.object_ref_is_missing(xref.to());
            (xref_id.clone(), XRefStatus::from_flags(from_missing, to_missing))
        })
        .collect::<Vec<_>>();

    for (xref_id, status) in next_statuses {
        if let Some(xref) = session.xrefs_mut().get_mut(&xref_id) {
            xref.set_status(status);
        }
    }
}

fn mcp_op_to_internal(op: &McpOp) -> Result<Op, ErrorData> {
    Ok(match op {
        McpOp::SeqAddParticipant { participant_id, mermaid_name } => {
            Op::Seq(SeqOp::AddParticipant {
                participant_id: parse_object_id(participant_id)?,
                mermaid_name: mermaid_name.clone(),
            })
        }
        McpOp::SeqUpdateParticipant { participant_id, mermaid_name } => {
            Op::Seq(SeqOp::UpdateParticipant {
                participant_id: parse_object_id(participant_id)?,
                patch: SeqParticipantPatch { mermaid_name: mermaid_name.clone() },
            })
        }
        McpOp::SeqSetParticipantNote { participant_id, note } => {
            Op::Seq(SeqOp::SetParticipantNote {
                participant_id: parse_object_id(participant_id)?,
                note: note.clone(),
            })
        }
        McpOp::SeqSetParticipantSymbol { participant_id, symbol } => {
            Op::Seq(SeqOp::SetParticipantSymbol {
                participant_id: parse_object_id(participant_id)?,
                symbol: symbol.as_ref().map(internal_symbol_anchor).transpose()?,
            })
        }
        McpOp::SeqRemoveParticipant { participant_id } => {
            Op::Seq(SeqOp::RemoveParticipant { participant_id: parse_object_id(participant_id)? })
        }
        McpOp::SeqAddMessage {
            message_id,
            from_participant_id,
            to_participant_id,
            kind,
            arrow,
            text,
            order_key,
            section_id,
        } => Op::Seq(SeqOp::AddMessage {
            message_id: parse_object_id(message_id)?,
            from_participant_id: parse_object_id(from_participant_id)?,
            to_participant_id: parse_object_id(to_participant_id)?,
            kind: map_message_kind(*kind),
            arrow: arrow.clone(),
            text: text.clone(),
            order_key: *order_key,
            section_id: section_id.as_deref().map(parse_object_id).transpose()?,
        }),
        McpOp::SeqUpdateMessage {
            message_id,
            from_participant_id,
            to_participant_id,
            kind,
            arrow,
            text,
            order_key,
        } => Op::Seq(SeqOp::UpdateMessage {
            message_id: parse_object_id(message_id)?,
            patch: SeqMessagePatch {
                from_participant_id: from_participant_id
                    .as_deref()
                    .map(parse_object_id)
                    .transpose()?,
                to_participant_id: to_participant_id.as_deref().map(parse_object_id).transpose()?,
                kind: kind.map(map_message_kind),
                arrow: arrow.clone(),
                text: text.clone(),
                order_key: *order_key,
            },
        }),
        McpOp::SeqRemoveMessage { message_id } => {
            Op::Seq(SeqOp::RemoveMessage { message_id: parse_object_id(message_id)? })
        }
        McpOp::SeqSetMessageSection { message_id, section_id } => {
            Op::Seq(SeqOp::SetMessageSection {
                message_id: parse_object_id(message_id)?,
                section_id: section_id.as_deref().map(parse_object_id).transpose()?,
            })
        }
        McpOp::SeqAddBlock { block_id, kind, header, parent_block_id, main_section_id } => {
            Op::Seq(SeqOp::AddBlock {
                block_id: parse_object_id(block_id)?,
                kind: map_mcp_seq_block_kind(*kind),
                header: header.clone(),
                parent_block_id: parent_block_id.as_deref().map(parse_object_id).transpose()?,
                main_section_id: parse_object_id(main_section_id)?,
            })
        }
        McpOp::SeqUpdateBlock { block_id, header } => Op::Seq(SeqOp::UpdateBlock {
            block_id: parse_object_id(block_id)?,
            patch: SeqBlockPatch { header: header.clone() },
        }),
        McpOp::SeqRemoveBlock { block_id } => {
            Op::Seq(SeqOp::RemoveBlock { block_id: parse_object_id(block_id)? })
        }
        McpOp::SeqAddSection { section_id, block_id, kind, header } => Op::Seq(SeqOp::AddSection {
            section_id: parse_object_id(section_id)?,
            block_id: parse_object_id(block_id)?,
            kind: map_mcp_seq_section_add_kind(*kind),
            header: header.clone(),
        }),
        McpOp::SeqUpdateSection { section_id, header } => Op::Seq(SeqOp::UpdateSection {
            section_id: parse_object_id(section_id)?,
            patch: SeqSectionPatch { header: header.clone() },
        }),
        McpOp::SeqRemoveSection { section_id } => {
            Op::Seq(SeqOp::RemoveSection { section_id: parse_object_id(section_id)? })
        }
        McpOp::FlowAddNode { node_id, label, shape } => Op::Flow(FlowOp::AddNode {
            node_id: parse_object_id(node_id)?,
            label: label.clone(),
            shape: shape.clone(),
        }),
        McpOp::FlowUpdateNode { node_id, label, shape } => Op::Flow(FlowOp::UpdateNode {
            node_id: parse_object_id(node_id)?,
            patch: FlowNodePatch { label: label.clone(), shape: shape.clone() },
        }),
        McpOp::FlowSetNodeMermaidId { node_id, mermaid_id } => Op::Flow(FlowOp::SetNodeMermaidId {
            node_id: parse_object_id(node_id)?,
            mermaid_id: mermaid_id.clone(),
        }),
        McpOp::FlowSetNodeNote { node_id, note } => {
            Op::Flow(FlowOp::SetNodeNote { node_id: parse_object_id(node_id)?, note: note.clone() })
        }
        McpOp::FlowSetNodeSymbol { node_id, symbol } => Op::Flow(FlowOp::SetNodeSymbol {
            node_id: parse_object_id(node_id)?,
            symbol: symbol.as_ref().map(internal_symbol_anchor).transpose()?,
        }),
        McpOp::FlowRemoveNode { node_id } => {
            Op::Flow(FlowOp::RemoveNode { node_id: parse_object_id(node_id)? })
        }
        McpOp::FlowAddEdge { edge_id, from_node_id, to_node_id, label, connector, style } => {
            Op::Flow(FlowOp::AddEdge {
                edge_id: parse_object_id(edge_id)?,
                from_node_id: parse_object_id(from_node_id)?,
                to_node_id: parse_object_id(to_node_id)?,
                label: label.clone(),
                connector: connector.clone(),
                style: style.clone(),
            })
        }
        McpOp::FlowUpdateEdge { edge_id, from_node_id, to_node_id, label, connector, style } => {
            Op::Flow(FlowOp::UpdateEdge {
                edge_id: parse_object_id(edge_id)?,
                patch: FlowEdgePatch {
                    from_node_id: from_node_id.as_deref().map(parse_object_id).transpose()?,
                    to_node_id: to_node_id.as_deref().map(parse_object_id).transpose()?,
                    label: label.clone(),
                    connector: connector.clone(),
                    style: style.clone(),
                },
            })
        }
        McpOp::FlowRemoveEdge { edge_id } => {
            Op::Flow(FlowOp::RemoveEdge { edge_id: parse_object_id(edge_id)? })
        }
    })
}

fn map_message_kind(kind: MessageKind) -> crate::model::SequenceMessageKind {
    match kind {
        MessageKind::Sync => crate::model::SequenceMessageKind::Sync,
        MessageKind::Async => crate::model::SequenceMessageKind::Async,
        MessageKind::Return => crate::model::SequenceMessageKind::Return,
    }
}

fn map_message_kind_to_mcp(kind: crate::model::SequenceMessageKind) -> MessageKind {
    match kind {
        crate::model::SequenceMessageKind::Sync => MessageKind::Sync,
        crate::model::SequenceMessageKind::Async => MessageKind::Async,
        crate::model::SequenceMessageKind::Return => MessageKind::Return,
    }
}
