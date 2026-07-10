// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Co-presence MCP tools: human/agent attention, selection, follow-AI, view state.
//!
//! Coordinates the live TUI camera and working set without mutating diagram ASTs.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};

use super::*;

#[tool_router(router = collaboration_tool_router, vis = "pub(super)")]
impl NereidMcp {
    /// Read human-owned attention from live TUI state; call early in a turn, then localize with
    /// `diagram_get_slice` and `object_read`.
    #[tool(name = "attention_human_read")]
    pub(super) async fn attention_human_read(
        &self,
    ) -> Result<Json<AttentionReadResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;
        Ok(Json(AttentionReadResponse {
            object_ref: context.human_active_object_ref.clone(),
            diagram_id: context.human_active_diagram_id.clone(),
            context,
        }))
    }

    /// Read agent-owned attention (single spotlight); call before `attention_agent_set`/`clear`
    /// to avoid unnecessary spotlight churn.
    #[tool(name = "attention_agent_read")]
    pub(super) async fn attention_agent_read(
        &self,
    ) -> Result<Json<AttentionReadResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let object_ref = self.agent_highlights.lock().await.iter().next().cloned();
        let diagram_id =
            object_ref.as_ref().map(|object_ref| object_ref.diagram_id().as_str().to_owned());
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(AttentionReadResponse {
            object_ref: object_ref.map(|object_ref| object_ref.to_string()),
            diagram_id,
            context,
        }))
    }

    /// Set agent-owned attention to one object; call before explanations/edits so the user can
    /// follow the agent in real time.
    #[tool(name = "attention_agent_set")]
    pub(super) async fn attention_agent_set(
        &self,
        params: Parameters<AttentionAgentSetParams>,
    ) -> Result<Json<AttentionSetResponse>, ErrorData> {
        let AttentionAgentSetParams { object_ref } = params.0;
        let parsed = parse_object_ref(&object_ref)?;

        let state = self.lock_state_synced().await?;
        if object_ref_is_missing(&state.session, &parsed) {
            return Err(ErrorData::resource_not_found(
                "object not found",
                Some(serde_json::json!({ "object_ref": object_ref })),
            ));
        }
        drop(state);

        let mut agent_highlights = self.agent_highlights.lock().await;
        agent_highlights.clear();
        agent_highlights.insert(parsed.clone());

        Ok(Json(AttentionSetResponse {
            object_ref: parsed.to_string(),
            diagram_id: parsed.diagram_id().as_str().to_owned(),
        }))
    }

    /// Clear agent-owned attention; use when done with a topic or before changing context.
    #[tool(name = "attention_agent_clear")]
    pub(super) async fn attention_agent_clear(
        &self,
    ) -> Result<Json<AttentionClearResponse>, ErrorData> {
        let mut agent_highlights = self.agent_highlights.lock().await;
        let cleared = agent_highlights.len() as u64;
        agent_highlights.clear();

        Ok(Json(AttentionClearResponse { cleared }))
    }

    /// Read follow-AI mode (`true` means TUI tracks agent spotlight); check this before
    /// spotlight-heavy guidance, and pair with `follow_ai_set` when handing off control.
    #[tool(name = "follow_ai_read")]
    pub(super) async fn follow_ai_read(&self) -> Result<Json<FollowAiReadResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;
        let enabled = context.follow_ai.unwrap_or(true);
        Ok(Json(FollowAiReadResponse { enabled, context }))
    }

    /// Set follow-AI mode (`true` to track agent spotlight in TUI); use with `attention_agent_set`
    /// for guided handoff.
    #[tool(name = "follow_ai_set")]
    pub(super) async fn follow_ai_set(
        &self,
        params: Parameters<FollowAiSetParams>,
    ) -> Result<Json<FollowAiSetResponse>, ErrorData> {
        let FollowAiSetParams { enabled } = params.0;
        if let Some(ui_state) = self.ui_state.as_ref() {
            ui_state.lock().await.set_follow_ai(enabled);
        }
        Ok(Json(FollowAiSetResponse { enabled }))
    }

    /// Read the shared multi-selection working set as canonical `object_ref`s; call after
    /// `attention_human_read` and before `object_read` or `selection_update`.
    #[tool(name = "selection_read")]
    pub(super) async fn selection_get(&self) -> Result<Json<SelectionGetResponse>, ErrorData> {
        let mut state = self.lock_state_synced().await?;
        if let Some(session_folder) = &self.session_folder {
            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(format!("failed to load session meta: {err}"), None)
            })?;
            state.session.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
            retain_existing_selected_object_refs(&mut state.session);
        }
        let object_refs = state
            .session
            .selected_object_refs()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(SelectionGetResponse { object_refs, context }))
    }

    /// Update shared multi-selection (`replace`/`add`/`remove`); use to mark a temporary working
    /// set for discussion or edits.
    #[tool(name = "selection_update")]
    pub(super) async fn selection_update(
        &self,
        params: Parameters<SelectionUpdateParams>,
    ) -> Result<Json<SelectionUpdateResponse>, ErrorData> {
        let SelectionUpdateParams { object_refs, mode } = params.0;

        let mut state = self.lock_state_synced().await?;
        let mut applied_refs = BTreeSet::new();
        let mut ignored_refs = BTreeSet::new();

        for object_ref in object_refs {
            let parsed = parse_object_ref(&object_ref)?;
            if object_ref_is_missing(&state.session, &parsed) {
                ignored_refs.insert(parsed.to_string());
            } else {
                applied_refs.insert(parsed);
            }
        }

        let applied = applied_refs.iter().map(ToString::to_string).collect::<Vec<_>>();

        fn apply_mode(session: &mut Session, mode: UpdateMode, object_refs: &BTreeSet<ObjectRef>) {
            match mode {
                UpdateMode::Replace => {
                    let selected = session.selected_object_refs_mut();
                    selected.clear();
                    selected.extend(object_refs.iter().cloned());
                }
                UpdateMode::Add => {
                    session.selected_object_refs_mut().extend(object_refs.iter().cloned());
                }
                UpdateMode::Remove => {
                    let selected = session.selected_object_refs_mut();
                    for object_ref in object_refs {
                        selected.remove(object_ref);
                    }
                }
            }
        }

        if let Some(session_folder) = &self.session_folder {
            let mut candidate = state.session.clone();
            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(format!("failed to load session meta: {err}"), None)
            })?;
            candidate.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
            retain_existing_selected_object_refs(&mut candidate);
            apply_mode(&mut candidate, mode, &applied_refs);
            session_folder.save_selected_object_refs(&candidate).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist selected object refs: {err}"),
                    Some(serde_json::json!({
                        "selected_count": candidate.selected_object_refs().len() as u64,
                    })),
                )
            })?;
            replace_committed_session(&mut state, candidate);
        } else {
            apply_mode(&mut state.session, mode, &applied_refs);
        }

        let response =
            Json(SelectionUpdateResponse { applied, ignored: ignored_refs.into_iter().collect() });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Read UI view state (active diagram, scroll, panes); use with
    /// `attention_human_read`/`attention_agent_read` for orientation without mutating focus.
    #[tool(name = "view_read_state")]
    pub(super) async fn view_get_state(&self) -> Result<Json<ViewGetStateResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(active_diagram_id.clone()).await;

        Ok(Json(ViewGetStateResponse {
            active_diagram_id,
            scroll: ViewScroll { x: 0.0, y: 0.0 },
            panes: BTreeMap::new(),
            context,
        }))
    }
    /// Read concrete object fields by ref; use this as evidence before answering.
    #[tool(name = "object_read")]
    pub(super) async fn object_read(
        &self,
        params: Parameters<ObjectGetParams>,
    ) -> Result<Json<ObjectGetResponse>, ErrorData> {
        let ObjectGetParams { object_ref, object_refs } = params.0;

        let object_refs = match (object_ref, object_refs) {
            (Some(_), Some(_)) => {
                return Err(ErrorData::invalid_params(
                    "provide either object_ref or object_refs, not both",
                    None,
                ));
            }
            (None, None) => {
                return Err(ErrorData::invalid_params(
                    "object_ref or object_refs is required",
                    None,
                ));
            }
            (Some(object_ref), None) => vec![object_ref],
            (None, Some(object_refs)) => object_refs,
        };

        if object_refs.is_empty() {
            return Err(ErrorData::invalid_params("object_refs must not be empty", None));
        }

        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let mut objects = Vec::with_capacity(object_refs.len());

        for object_ref in object_refs {
            let parsed = parse_object_ref(&object_ref)?;
            let diagram = state.session.diagrams().get(parsed.diagram_id()).ok_or_else(|| {
                ErrorData::resource_not_found(
                    "diagram not found",
                    Some(serde_json::json!({
                        "diagram_id": parsed.diagram_id().as_str(),
                        "object_ref": object_ref.as_str(),
                    })),
                )
            })?;

            let segments = parsed.category().segments();
            let object_id = parsed.object_id();

            let object = match (segments, diagram.ast()) {
                ([left, right], DiagramAst::Sequence(ast))
                    if left == "seq" && right == "participant" =>
                {
                    let participant = ast.participants().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "seq participant not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;

                    McpObject::SeqParticipant {
                        mermaid_name: participant.mermaid_name().to_owned(),
                        role: participant.role().map(|r| r.to_owned()),
                        note: participant.note().map(ToOwned::to_owned),
                        symbol: participant.symbol().map(mcp_symbol_anchor),
                    }
                }
                ([left, right], DiagramAst::Sequence(ast)) if left == "seq" && right == "block" => {
                    let block = ast.find_block(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "seq block not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;

                    McpObject::SeqBlock {
                        kind: map_seq_block_kind_to_mcp(block.kind()),
                        header: block.header().map(|h| h.to_owned()),
                        section_ids: block
                            .sections()
                            .iter()
                            .map(|section| section.section_id().to_string())
                            .collect(),
                        child_block_ids: block
                            .blocks()
                            .iter()
                            .map(|child| child.block_id().to_string())
                            .collect(),
                    }
                }
                ([left, right], DiagramAst::Sequence(ast))
                    if left == "seq" && right == "section" =>
                {
                    let section = ast.find_section(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "seq section not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;

                    McpObject::SeqSection {
                        kind: map_seq_section_kind_to_mcp(section.kind()),
                        header: section.header().map(|h| h.to_owned()),
                        message_ids: section
                            .message_ids()
                            .iter()
                            .map(|message_id| message_id.to_string())
                            .collect(),
                    }
                }
                ([left, right], DiagramAst::Sequence(ast))
                    if left == "seq" && right == "message" =>
                {
                    let message =
                        ast.messages().iter().find(|m| m.message_id() == object_id).ok_or_else(
                            || {
                                ErrorData::resource_not_found(
                                    "seq message not found",
                                    Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                                )
                            },
                        )?;

                    McpObject::SeqMessage {
                        from_participant_id: message.from_participant_id().to_string(),
                        to_participant_id: message.to_participant_id().to_string(),
                        kind: map_message_kind_to_mcp(message.kind()),
                        arrow: message.raw_arrow().map(ToOwned::to_owned),
                        text: message.text().to_owned(),
                        order_key: message.order_key(),
                    }
                }
                ([left, right], DiagramAst::Flowchart(ast))
                    if left == "flow" && right == "node" =>
                {
                    let node = ast.nodes().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "flow node not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;

                    McpObject::FlowNode {
                        label: node.label().to_owned(),
                        shape: node.shape().to_owned(),
                        mermaid_id: node.mermaid_id().map(|s| s.to_owned()),
                        note: node.note().map(ToOwned::to_owned),
                        symbol: node.symbol().map(mcp_symbol_anchor),
                    }
                }
                ([left, right], DiagramAst::Flowchart(ast))
                    if left == "flow" && right == "edge" =>
                {
                    let edge = ast.edges().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "flow edge not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;

                    McpObject::FlowEdge {
                        from_node_id: edge.from_node_id().to_string(),
                        to_node_id: edge.to_node_id().to_string(),
                        label: edge.label().map(|s| s.to_owned()),
                        connector: edge.connector().map(|s| s.to_owned()),
                        style: edge.style().map(|s| s.to_owned()),
                    }
                }
                ([left, right], DiagramAst::Class(ast)) if left == "class" && right == "class" => {
                    let class = ast.classes().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "class not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;
                    McpObject::ClassNode {
                        name: class.name().to_owned(),
                        attributes: class.attributes().to_vec(),
                        methods: class.methods().to_vec(),
                        note: class.note().map(ToOwned::to_owned),
                    }
                }
                ([left, right], DiagramAst::Class(ast))
                    if left == "class" && right == "relation" =>
                {
                    let rel = ast.relations().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "class relation not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;
                    McpObject::ClassRelation {
                        from_class_id: rel.from_class_id().to_string(),
                        to_class_id: rel.to_class_id().to_string(),
                        kind: map_class_relation_kind_to_mcp(rel.kind()),
                        label: rel.label().map(ToOwned::to_owned),
                        raw_connector: rel.raw_connector().map(ToOwned::to_owned),
                    }
                }
                ([left, right], DiagramAst::Er(ast)) if left == "er" && right == "entity" => {
                    let entity = ast.entities().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "entity not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;
                    McpObject::ErEntity {
                        name: entity.name().to_owned(),
                        note: entity.note().map(ToOwned::to_owned),
                    }
                }
                ([left, right], DiagramAst::Er(ast)) if left == "er" && right == "relationship" => {
                    let rel = ast.relationships().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "relationship not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;
                    McpObject::ErRelationship {
                        from_entity_id: rel.from_entity_id().to_string(),
                        to_entity_id: rel.to_entity_id().to_string(),
                        from_cardinality: map_er_cardinality_to_mcp(rel.from_card()),
                        to_cardinality: map_er_cardinality_to_mcp(rel.to_card()),
                        stroke: map_er_stroke_to_mcp(rel.stroke()),
                        label: rel.label().map(ToOwned::to_owned),
                        raw_connector: rel.raw_connector().map(ToOwned::to_owned),
                    }
                }
                ([left, right], DiagramAst::Gantt(ast))
                    if left == "gantt" && right == "section" =>
                {
                    let section = ast
                        .sections()
                        .iter()
                        .find(|section| section.section_id() == object_id)
                        .ok_or_else(|| {
                            ErrorData::resource_not_found(
                                "gantt section not found",
                                Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                            )
                        })?;
                    McpObject::GanttSection {
                        name: section.name().to_owned(),
                        task_ids: section.task_ids().iter().map(ToString::to_string).collect(),
                    }
                }
                ([left, right], DiagramAst::Gantt(ast)) if left == "gantt" && right == "task" => {
                    let task = ast.tasks().get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "gantt task not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;
                    McpObject::GanttTask {
                        mermaid_tag: task.mermaid_tag().map(ToOwned::to_owned),
                        name: task.name().to_owned(),
                        start: map_gantt_task_start_to_mcp(task.start()),
                        duration_days: task.duration_days(),
                        raw_duration: task.raw_duration().to_owned(),
                        note: task.note().map(ToOwned::to_owned),
                    }
                }
                ([left, right], DiagramAst::Gantt(ast)) if left == "gantt" && right == "lane" => {
                    let lanes = ast.lanes();
                    let label = lanes.get(object_id).ok_or_else(|| {
                        ErrorData::resource_not_found(
                            "gantt lane not found",
                            Some(serde_json::json!({ "object_ref": object_ref.as_str() })),
                        )
                    })?;
                    let note = ast.lane_note(object_id).map(ToOwned::to_owned);
                    McpObject::GanttLane { label: label.clone(), note }
                }
                _ => {
                    return Err(ErrorData::invalid_params(
                        "unsupported category for diagram kind",
                        Some(serde_json::json!({
                            "object_ref": object_ref.as_str(),
                            "diagram_kind": diagram_kind_label(diagram.kind()),
                            "category": segments.to_vec(),
                        })),
                    ));
                }
            };

            objects.push(ObjectGetItem { object_ref, object });
        }

        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(ObjectGetResponse { objects, context }))
    }
}
