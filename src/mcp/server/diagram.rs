// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Diagram MCP tools: lifecycle, AST/text reads, structure ops, and Mermaid replace.
//!
//! Mutations require `base_rev`; replace reconciles stable ids and reports identity deltas.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};

use crate::format::mermaid::{
    parse_class_diagram, parse_er_diagram, parse_flowchart, parse_gantt_diagram,
    parse_sequence_diagram,
};
use crate::model::{CategoryPath, DiagramAst, DiagramId, ObjectId, ObjectRef};
use crate::ops::apply_ops;
use crate::render::render_diagram_unicode;

use super::*;

fn category(segments: &[&str]) -> CategoryPath {
    CategoryPath::new(segments.iter().map(|segment| (*segment).to_owned()).collect())
        .expect("static object category")
}

fn object_ref(diagram_id: &DiagramId, category: CategoryPath, object_id: &ObjectId) -> ObjectRef {
    ObjectRef::new(diagram_id.clone(), category, object_id.clone())
}

fn stable_object_snapshots(
    diagram_id: &DiagramId,
    ast: &DiagramAst,
) -> std::collections::BTreeMap<ObjectRef, String> {
    let mut snapshots = std::collections::BTreeMap::new();

    match ast {
        DiagramAst::Class(class_ast) => {
            let class_category = category(&["class", "class"]);
            let relation_category = category(&["class", "relation"]);
            for (class_id, node) in class_ast.classes() {
                snapshots.insert(
                    object_ref(diagram_id, class_category.clone(), class_id),
                    format!("name={:?};note={:?}", node.name(), node.note()),
                );
            }
            for (rel_id, rel) in class_ast.relations() {
                snapshots.insert(
                    object_ref(diagram_id, relation_category.clone(), rel_id),
                    format!(
                        "from={:?};to={:?};kind={:?};label={:?}",
                        rel.from_class_id(),
                        rel.to_class_id(),
                        rel.kind(),
                        rel.label()
                    ),
                );
            }
        }
        DiagramAst::Er(er_ast) => {
            let ent_cat = category(&["er", "entity"]);
            let rel_cat = category(&["er", "relationship"]);
            for (id, e) in er_ast.entities() {
                snapshots.insert(
                    object_ref(diagram_id, ent_cat.clone(), id),
                    format!("name={:?};note={:?}", e.name(), e.note()),
                );
            }
            for (id, r) in er_ast.relationships() {
                snapshots.insert(
                    object_ref(diagram_id, rel_cat.clone(), id),
                    format!("label={:?}", r.label()),
                );
            }
        }
        DiagramAst::Gantt(gantt_ast) => {
            let task_cat = category(&["gantt", "task"]);
            for (id, task) in gantt_ast.tasks() {
                snapshots.insert(
                    object_ref(diagram_id, task_cat.clone(), id),
                    format!("name={:?}", task.name()),
                );
            }
        }
        DiagramAst::Sequence(seq) => {
            let participant_category = category(&["seq", "participant"]);
            let message_category = category(&["seq", "message"]);
            let block_category = category(&["seq", "block"]);
            let section_category = category(&["seq", "section"]);

            for (participant_id, participant) in seq.participants() {
                snapshots.insert(
                    object_ref(diagram_id, participant_category.clone(), participant_id),
                    format!(
                        "name={:?};role={:?};note={:?};symbol={:?}",
                        participant.mermaid_name(),
                        participant.role(),
                        participant.note(),
                        participant.symbol()
                    ),
                );
            }

            for message in seq.messages() {
                snapshots.insert(
                    object_ref(diagram_id, message_category.clone(), message.message_id()),
                    format!(
                        "from={};to={};kind={:?};arrow={:?};text={:?};order={}",
                        message.from_participant_id(),
                        message.to_participant_id(),
                        message.kind(),
                        message.raw_arrow(),
                        message.text(),
                        message.order_key()
                    ),
                );
            }

            fn collect_blocks(
                diagram_id: &DiagramId,
                block_category: &CategoryPath,
                section_category: &CategoryPath,
                snapshots: &mut std::collections::BTreeMap<ObjectRef, String>,
                blocks: &[crate::model::seq_ast::SequenceBlock],
            ) {
                for block in blocks {
                    let section_ids = block
                        .sections()
                        .iter()
                        .map(|section| section.section_id().to_string())
                        .collect::<Vec<_>>();
                    let child_block_ids = block
                        .blocks()
                        .iter()
                        .map(|child| child.block_id().to_string())
                        .collect::<Vec<_>>();
                    snapshots.insert(
                        object_ref(diagram_id, block_category.clone(), block.block_id()),
                        format!(
                            "kind={:?};header={:?};sections={:?};children={:?}",
                            block.kind(),
                            block.header(),
                            section_ids,
                            child_block_ids
                        ),
                    );

                    for section in block.sections() {
                        let message_ids = section
                            .message_ids()
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        snapshots.insert(
                            object_ref(diagram_id, section_category.clone(), section.section_id()),
                            format!(
                                "kind={:?};header={:?};messages={:?}",
                                section.kind(),
                                section.header(),
                                message_ids
                            ),
                        );
                    }

                    collect_blocks(
                        diagram_id,
                        block_category,
                        section_category,
                        snapshots,
                        block.blocks(),
                    );
                }
            }

            collect_blocks(
                diagram_id,
                &block_category,
                &section_category,
                &mut snapshots,
                seq.blocks(),
            );
        }
        DiagramAst::Flowchart(flow) => {
            let node_category = category(&["flow", "node"]);
            let edge_category = category(&["flow", "edge"]);

            for (node_id, node) in flow.nodes() {
                snapshots.insert(
                    object_ref(diagram_id, node_category.clone(), node_id),
                    format!(
                        "mermaid_id={:?};label={:?};shape={:?};note={:?};symbol={:?}",
                        node.mermaid_id(),
                        node.label(),
                        node.shape(),
                        node.note(),
                        node.symbol()
                    ),
                );
            }

            for (edge_id, edge) in flow.edges() {
                snapshots.insert(
                    object_ref(diagram_id, edge_category.clone(), edge_id),
                    format!(
                        "from={};to={};label={:?};connector={:?};style={:?}",
                        edge.from_node_id(),
                        edge.to_node_id(),
                        edge.label(),
                        edge.connector(),
                        edge.style()
                    ),
                );
            }
        }
    }

    snapshots
}

fn replace_delta_from_asts(
    diagram_id: &DiagramId,
    previous_ast: &DiagramAst,
    next_ast: &DiagramAst,
) -> (crate::ops::Delta, bool) {
    let previous = stable_object_snapshots(diagram_id, previous_ast);
    let next = stable_object_snapshots(diagram_id, next_ast);

    let mut delta = crate::ops::Delta::default();

    for (object_ref, next_payload) in &next {
        match previous.get(object_ref) {
            Some(previous_payload) if previous_payload != next_payload => {
                delta.updated.push(object_ref.clone());
            }
            Some(_) => {}
            None => delta.added.push(object_ref.clone()),
        }
    }

    for object_ref in previous.keys() {
        if !next.contains_key(object_ref) {
            delta.removed.push(object_ref.clone());
        }
    }

    let diagram_metadata_updated = matches!(
        (previous_ast, next_ast),
        (DiagramAst::Flowchart(previous), DiagramAst::Flowchart(next))
            if previous.default_edge_style() != next.default_edge_style()
    );

    (delta, diagram_metadata_updated)
}

#[tool_router(router = diagram_tool_router, vis = "pub(super)")]
impl NereidMcp {
    /// List diagrams in the current session; start here, then call `diagram_current` or
    /// `diagram_open` (bootstrap with `diagram_create_from_mermaid` if empty).
    #[tool(name = "diagram_list")]
    pub(super) async fn diagram_list(&self) -> Result<Json<ListDiagramsResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let diagrams = state
            .session
            .diagrams()
            .iter()
            .map(|(diagram_id, diagram)| DiagramSummary {
                diagram_id: diagram_id.as_str().to_owned(),
                name: diagram.name().to_owned(),
                kind: diagram_kind_label(diagram.kind()).to_owned(),
                rev: diagram.rev(),
            })
            .collect::<Vec<_>>();
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(ListDiagramsResponse { diagrams, context }))
    }

    /// Replace an existing diagram's content from Mermaid while reconciling stable object ids
    /// from the current AST (messages, blocks, nodes, edges). Prefer structure ops for local
    /// edits; use this for bulk rewrites that should keep identity when fingerprints match.
    #[tool(name = "diagram_replace_from_mermaid")]
    pub(super) async fn diagram_replace_from_mermaid(
        &self,
        params: Parameters<DiagramReplaceFromMermaidParams>,
    ) -> Result<Json<DiagramReplaceFromMermaidResponse>, ErrorData> {
        let DiagramReplaceFromMermaidParams { diagram_id: requested_diagram_id, base_rev, mermaid } =
            params.0;

        let mut state = self.lock_state_synced().await?;

        let (response, history_update) = if let Some(session_folder) = &self.session_folder {
            let mut update = session_folder.begin_session_update().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to reload session before save: {err}"),
                    Some(serde_json::json!({
                        "diagram_id": requested_diagram_id.as_deref(),
                        "base_rev": base_rev
                    })),
                )
            })?;
            let candidate_session = update.session_mut();
            let diagram_id =
                resolve_diagram_id(candidate_session, requested_diagram_id.as_deref())?;
            let mut candidate_diagram = candidate_session
                .diagrams()
                .get(&diagram_id)
                .cloned()
                .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

            let current_rev = candidate_diagram.rev();
            if base_rev != current_rev {
                return Err(ErrorData::invalid_request(
                    "conflict: stale base_rev",
                    Some(serde_json::json!({
                        "base_rev": base_rev,
                        "current_rev": current_rev,
                        "snapshot_tool": "diagram_stat",
                    })),
                ));
            }

            let previous_ast = candidate_diagram.ast().clone();
            let replace =
                crate::store::replace_diagram_from_mermaid(&mut candidate_diagram, &mermaid)
                    .map_err(map_replace_error)?;
            let (delta, diagram_metadata_updated) =
                replace_delta_from_asts(&diagram_id, &previous_ast, candidate_diagram.ast());
            render_diagram_unicode(&candidate_diagram).map_err(|err| {
                ErrorData::invalid_request(
                    format!("cannot render diagram after replace_from_mermaid: {err}"),
                    Some(serde_json::json!({
                        "diagram_id": diagram_id.as_str(),
                        "base_rev": base_rev,
                        "render_error": err.to_string(),
                    })),
                )
            })?;
            candidate_session.diagrams_mut().insert(diagram_id.clone(), candidate_diagram);
            retain_existing_selected_object_refs(candidate_session);
            refresh_xref_statuses(candidate_session);

            let dangling_xref_ids = dangling_xref_ids_for_diagram(candidate_session, &diagram_id);

            let identity =
                identity_report_from_sets(&replace.previous_object_ids, &replace.next_object_ids);

            let mut history =
                state.delta_history.get(&diagram_id).cloned().unwrap_or_else(VecDeque::new);
            history.push_back(LastDelta {
                from_rev: base_rev,
                to_rev: replace.new_rev,
                delta,
                diagram_metadata_updated,
            });
            while history.len() > DELTA_HISTORY_LIMIT {
                history.pop_front();
            }

            let candidate_session = update.commit().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id.as_str(), "base_rev": base_rev })),
                )
            })?;
            replace_committed_session(&mut state, candidate_session);

            (
                Json(DiagramReplaceFromMermaidResponse {
                    new_rev: replace.new_rev,
                    diagram_id: diagram_id.as_str().to_owned(),
                    kind: diagram_kind_label(
                        state
                            .session
                            .diagrams()
                            .get(&diagram_id)
                            .map(|d| d.kind())
                            .unwrap_or(DiagramKind::Sequence),
                    )
                    .to_owned(),
                    identity,
                    dangling_xref_ids,
                }),
                Some((diagram_id, history)),
            )
        } else {
            let diagram_id = resolve_diagram_id(&state.session, requested_diagram_id.as_deref())?;
            let mut candidate_diagram = state
                .session
                .diagrams()
                .get(&diagram_id)
                .cloned()
                .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;
            let current_rev = candidate_diagram.rev();
            if base_rev != current_rev {
                return Err(ErrorData::invalid_request(
                    "conflict: stale base_rev",
                    Some(serde_json::json!({
                        "base_rev": base_rev,
                        "current_rev": current_rev,
                        "snapshot_tool": "diagram_stat",
                    })),
                ));
            }

            let previous_ast = candidate_diagram.ast().clone();
            let replace =
                crate::store::replace_diagram_from_mermaid(&mut candidate_diagram, &mermaid)
                    .map_err(map_replace_error)?;
            let (delta, diagram_metadata_updated) =
                replace_delta_from_asts(&diagram_id, &previous_ast, candidate_diagram.ast());
            render_diagram_unicode(&candidate_diagram).map_err(|err| {
                ErrorData::invalid_request(
                    format!("cannot render diagram after replace_from_mermaid: {err}"),
                    Some(serde_json::json!({
                        "diagram_id": diagram_id.as_str(),
                        "base_rev": base_rev,
                        "render_error": err.to_string(),
                    })),
                )
            })?;
            let kind = diagram_kind_label(candidate_diagram.kind()).to_owned();
            state.session.diagrams_mut().insert(diagram_id.clone(), candidate_diagram);
            retain_existing_selected_object_refs(&mut state.session);
            refresh_xref_statuses(&mut state.session);
            let dangling_xref_ids = dangling_xref_ids_for_diagram(&state.session, &diagram_id);
            let identity =
                identity_report_from_sets(&replace.previous_object_ids, &replace.next_object_ids);

            let mut history =
                state.delta_history.get(&diagram_id).cloned().unwrap_or_else(VecDeque::new);
            history.push_back(LastDelta {
                from_rev: base_rev,
                to_rev: replace.new_rev,
                delta,
                diagram_metadata_updated,
            });
            while history.len() > DELTA_HISTORY_LIMIT {
                history.pop_front();
            }

            (
                Json(DiagramReplaceFromMermaidResponse {
                    new_rev: replace.new_rev,
                    diagram_id: diagram_id.as_str().to_owned(),
                    kind,
                    identity,
                    dangling_xref_ids,
                }),
                Some((diagram_id, history)),
            )
        };

        if let Some((diagram_id, history)) = history_update {
            state.delta_history.insert(diagram_id, history);
        }
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Create a diagram from raw Mermaid; use to bootstrap a session, then continue with
    /// `diagram_open`/`diagram_stat`.
    #[tool(name = "diagram_create_from_mermaid")]
    pub(super) async fn diagram_create_from_mermaid(
        &self,
        params: Parameters<DiagramCreateFromMermaidParams>,
    ) -> Result<Json<DiagramCreateFromMermaidResponse>, ErrorData> {
        let DiagramCreateFromMermaidParams { mermaid, diagram_id, name, make_active } = params.0;

        let Some(kind) = detect_mermaid_kind(&mermaid) else {
            return Err(ErrorData::invalid_params(
                "expected flowchart/graph, sequenceDiagram, classDiagram, erDiagram, or gantt as the first non-empty line",
                None,
            ));
        };

        let ast = match kind {
            DiagramKind::Sequence => {
                DiagramAst::Sequence(parse_sequence_diagram(&mermaid).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("cannot parse Mermaid sequence diagram: {err}"),
                        None,
                    )
                })?)
            }
            DiagramKind::Flowchart => {
                DiagramAst::Flowchart(parse_flowchart(&mermaid).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("cannot parse Mermaid flowchart diagram: {err}"),
                        None,
                    )
                })?)
            }
            DiagramKind::Class => {
                DiagramAst::Class(parse_class_diagram(&mermaid).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("cannot parse Mermaid class diagram: {err}"),
                        None,
                    )
                })?)
            }
            DiagramKind::Er => DiagramAst::Er(parse_er_diagram(&mermaid).map_err(|err| {
                ErrorData::invalid_params(format!("cannot parse Mermaid er diagram: {err}"), None)
            })?),
            DiagramKind::Gantt => {
                DiagramAst::Gantt(parse_gantt_diagram(&mermaid).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("cannot parse Mermaid gantt diagram: {err}"),
                        None,
                    )
                })?)
            }
        };

        let kind_label = diagram_kind_label(kind).to_owned();
        let make_active = make_active.unwrap_or(true);

        let requested_diagram_id = diagram_id
            .map(|diagram_id| {
                DiagramId::new(diagram_id.clone()).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid diagram_id: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    )
                })
            })
            .transpose()?;
        let requested_name = name;

        let mut state = self.lock_state_synced().await?;
        let diagram_id;
        let name;
        if let Some(session_folder) = &self.session_folder {
            let (candidate, committed_diagram_id, committed_name) = {
                let mut update = session_folder.begin_session_update().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to reload session before save: {err}"),
                        requested_diagram_id.as_ref().map(
                            |diagram_id| serde_json::json!({ "diagram_id": diagram_id.as_str() }),
                        ),
                    )
                })?;
                let candidate = update.session_mut();
                let diagram_id = requested_diagram_id
                    .clone()
                    .unwrap_or_else(|| allocate_diagram_id(candidate, kind));
                if candidate.diagrams().contains_key(&diagram_id) {
                    return Err(ErrorData::invalid_params(
                        "diagram_id already exists",
                        Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
                    ));
                }
                let name = requested_name.clone().unwrap_or_else(|| diagram_id.as_str().to_owned());
                let diagram = Diagram::new(diagram_id.clone(), name.clone(), ast.clone());
                render_diagram_unicode(&diagram).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("cannot render Mermaid diagram: {err}"),
                        Some(serde_json::json!({
                            "diagram_id": diagram_id.as_str(),
                            "kind": kind_label.clone(),
                            "render_error": err.to_string(),
                        })),
                    )
                })?;
                candidate.diagrams_mut().insert(diagram_id.clone(), diagram);
                if make_active {
                    candidate.set_active_diagram_id(Some(diagram_id.clone()));
                }

                let candidate = update.commit().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to persist session: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
                    )
                })?;
                (candidate, diagram_id, name)
            };
            replace_committed_session(&mut state, candidate);
            diagram_id = committed_diagram_id;
            name = committed_name;
        } else {
            diagram_id =
                requested_diagram_id.unwrap_or_else(|| allocate_diagram_id(&state.session, kind));
            if state.session.diagrams().contains_key(&diagram_id) {
                return Err(ErrorData::invalid_params(
                    "diagram_id already exists",
                    Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
                ));
            }
            name = requested_name.unwrap_or_else(|| diagram_id.as_str().to_owned());
            let diagram = Diagram::new(diagram_id.clone(), name.clone(), ast);
            render_diagram_unicode(&diagram).map_err(|err| {
                ErrorData::invalid_params(
                    format!("cannot render Mermaid diagram: {err}"),
                    Some(serde_json::json!({
                        "diagram_id": diagram_id.as_str(),
                        "kind": kind_label.clone(),
                        "render_error": err.to_string(),
                    })),
                )
            })?;
            state.session.diagrams_mut().insert(diagram_id.clone(), diagram);
            if make_active {
                state.session.set_active_diagram_id(Some(diagram_id.clone()));
            }
        }

        let response = Json(DiagramCreateFromMermaidResponse {
            diagram: DiagramSummary {
                diagram_id: diagram_id.as_str().to_owned(),
                name,
                kind: kind_label,
                rev: 0,
            },
            active_diagram_id: state
                .session
                .active_diagram_id()
                .map(|diagram_id| diagram_id.as_str().to_owned()),
        });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Set the active diagram default for diagram-scoped tools; typically follows `diagram_list`
    /// or `diagram_create_from_mermaid`.
    #[tool(name = "diagram_open")]
    pub(super) async fn diagram_open(
        &self,
        params: Parameters<DiagramOpenParams>,
    ) -> Result<Json<DiagramOpenResponse>, ErrorData> {
        let diagram_id = params.0.diagram_id;
        let parsed = DiagramId::new(diagram_id.clone()).map_err(|err| {
            ErrorData::invalid_params(
                format!("invalid diagram_id: {err}"),
                Some(serde_json::json!({ "diagram_id": diagram_id })),
            )
        })?;

        let mut state = self.lock_state_synced().await?;
        if !state.session.diagrams().contains_key(&parsed) {
            return Err(ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id })),
            ));
        }

        if let Some(session_folder) = &self.session_folder {
            let candidate = {
                let mut update = session_folder.begin_session_update().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to reload session before save: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    )
                })?;
                let candidate = update.session_mut();
                if !candidate.diagrams().contains_key(&parsed) {
                    return Err(ErrorData::resource_not_found(
                        "diagram not found",
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    ));
                }
                candidate.set_active_diagram_id(Some(parsed.clone()));
                update.commit().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to persist session: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    )
                })?
            };
            replace_committed_session(&mut state, candidate);
        } else {
            state.session.set_active_diagram_id(Some(parsed.clone()));
        }

        let response = Json(DiagramOpenResponse { active_diagram_id: parsed.as_str().to_owned() });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Remove a diagram by id and retarget active diagram when needed.
    #[tool(name = "diagram_delete")]
    pub(super) async fn diagram_delete(
        &self,
        params: Parameters<DiagramDeleteParams>,
    ) -> Result<Json<DiagramDeleteResponse>, ErrorData> {
        let diagram_id = params.0.diagram_id;
        let parsed = DiagramId::new(diagram_id.clone()).map_err(|err| {
            ErrorData::invalid_params(
                format!("invalid diagram_id: {err}"),
                Some(serde_json::json!({ "diagram_id": diagram_id })),
            )
        })?;

        let mut state = self.lock_state_synced().await?;
        if !state.session.diagrams().contains_key(&parsed) {
            return Err(ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id })),
            ));
        }

        if let Some(session_folder) = &self.session_folder {
            let candidate = {
                let mut update = session_folder.begin_session_update().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to reload session before save: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    )
                })?;
                let candidate = update.session_mut();
                if !candidate.diagrams().contains_key(&parsed) {
                    return Err(ErrorData::resource_not_found(
                        "diagram not found",
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    ));
                }
                candidate.diagrams_mut().remove(&parsed);

                if candidate.active_diagram_id().is_some_and(|active| active == &parsed) {
                    let next_active = candidate.diagrams().keys().next().cloned();
                    candidate.set_active_diagram_id(next_active);
                }

                retain_existing_selected_object_refs(candidate);
                refresh_xref_statuses(candidate);

                update.commit().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to persist session: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    )
                })?
            };
            replace_committed_session(&mut state, candidate);
        } else {
            state.session.diagrams_mut().remove(&parsed);
            if state.session.active_diagram_id().is_some_and(|active| active == &parsed) {
                let next_active = state.session.diagrams().keys().next().cloned();
                state.session.set_active_diagram_id(next_active);
            }

            retain_existing_selected_object_refs(&mut state.session);
            refresh_xref_statuses(&mut state.session);
        }

        state.delta_history.remove(&parsed);
        let active_diagram_id =
            state.session.active_diagram_id().map(|active| active.as_str().to_owned());
        drop(state);

        let mut agent_highlights = self.agent_highlights.lock().await;
        agent_highlights.retain(|object_ref| object_ref.diagram_id() != &parsed);

        let response = Json(DiagramDeleteResponse {
            deleted_diagram_id: parsed.as_str().to_owned(),
            active_diagram_id,
        });
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Get the active diagram id (`null` when unset); check this before deciding whether to call
    /// `diagram_open`, then continue with `diagram_stat`/`diagram_get_slice`.
    #[tool(name = "diagram_current")]
    pub(super) async fn diagram_current(&self) -> Result<Json<DiagramCurrentResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(active_diagram_id.clone()).await;

        Ok(Json(DiagramCurrentResponse { active_diagram_id, context }))
    }
    /// Get a compact diagram digest (rev + counts + key names); use as the default first read
    /// before `diagram_get_slice`, typed queries, or mutation planning.
    #[tool(name = "diagram_stat")]
    pub(super) async fn diagram_stat(
        &self,
        params: Parameters<DiagramTargetParams>,
    ) -> Result<Json<DiagramDigest>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;
        let mut digest = digest_for_diagram(diagram);
        drop(state);
        digest.context = self.read_context(session_active_diagram_id).await;

        Ok(Json(digest))
    }

    /// Read canonical Mermaid snapshot of current diagram AST; use for export/review and
    /// debugging, not as the default probe.
    #[tool(name = "diagram_read")]
    pub(super) async fn diagram_read(
        &self,
        params: Parameters<DiagramTargetParams>,
    ) -> Result<Json<DiagramSnapshot>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;
        let response = DiagramSnapshot {
            rev: diagram.rev(),
            kind: diagram_kind_label(diagram.kind()).to_owned(),
            mermaid: mermaid_for_diagram(diagram),
            context: ReadContext::default(),
        };
        drop(state);
        let mut response = response;
        response.context = self.read_context(session_active_diagram_id).await;

        Ok(Json(response))
    }

    /// Read full diagram AST for id/label resolution; prefer this over session-file reads.
    #[tool(name = "diagram_get_ast")]
    pub(super) async fn diagram_get_ast(
        &self,
        params: Parameters<DiagramTargetParams>,
    ) -> Result<Json<DiagramGetAstResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        Ok(Json(DiagramGetAstResponse {
            diagram_id: diagram_id.as_str().to_owned(),
            kind: diagram_kind_label(diagram.kind()).to_owned(),
            rev: diagram.rev(),
            default_symbol_repository_id: diagram
                .default_symbol_repository_id()
                .map(ToOwned::to_owned),
            ast: mcp_ast_for_diagram(diagram),
        }))
    }

    /// Get deterministic local neighborhood around an `object_ref`; primary probe tool after
    /// attention/selection or search hits.
    #[tool(name = "diagram_get_slice")]
    pub(super) async fn diagram_get_slice(
        &self,
        params: Parameters<DiagramGetSliceParams>,
    ) -> Result<Json<DiagramGetSliceResponse>, ErrorData> {
        let DiagramGetSliceParams { diagram_id, center_ref, radius, depth, filters } = params.0;

        let requested_diagram_id = diagram_id
            .as_deref()
            .map(|diagram_id| {
                DiagramId::new(diagram_id.to_owned()).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid diagram_id: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    )
                })
            })
            .transpose()?;

        let center_ref_parsed = ObjectRef::parse(&center_ref).map_err(|err| {
            ErrorData::invalid_params(
                format!("invalid center_ref: {err}"),
                Some(serde_json::json!({ "center_ref": center_ref })),
            )
        })?;

        if let Some(requested_diagram_id) = requested_diagram_id.as_ref() {
            if requested_diagram_id != center_ref_parsed.diagram_id() {
                return Err(ErrorData::invalid_params(
                    "center_ref diagram_id does not match diagram_id",
                    Some(serde_json::json!({
                        "diagram_id": requested_diagram_id.as_str(),
                        "center_ref_diagram_id": center_ref_parsed.diagram_id().as_str(),
                    })),
                ));
            }
        }

        let diagram_id =
            requested_diagram_id.unwrap_or_else(|| center_ref_parsed.diagram_id().clone());

        let depth_u64 = depth.or(radius).unwrap_or(1);
        let max_hops = usize::try_from(depth_u64).map_err(|_| {
            ErrorData::invalid_params(
                "depth is too large",
                Some(serde_json::json!({ "depth": depth_u64 })),
            )
        })?;

        let (include_categories, exclude_categories) = if let Some(filters) = filters.as_ref() {
            let mut include = BTreeSet::new();
            if let Some(values) = filters.include_categories.as_ref() {
                for value in values {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        include.insert(trimmed.to_owned());
                    }
                }
            }
            let mut exclude = BTreeSet::new();
            if let Some(values) = filters.exclude_categories.as_ref() {
                for value in values {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        exclude.insert(trimmed.to_owned());
                    }
                }
            }
            (Some(include), Some(exclude))
        } else {
            (None, None)
        };

        fn bfs_within_radius(
            adjacency: &BTreeMap<ObjectId, BTreeSet<ObjectId>>,
            starts: impl IntoIterator<Item = ObjectId>,
            max_hops: usize,
        ) -> BTreeSet<ObjectId> {
            let mut visited: BTreeSet<ObjectId> = BTreeSet::new();
            let mut queue: VecDeque<(ObjectId, usize)> = VecDeque::new();

            for start in starts {
                if !adjacency.contains_key(&start) {
                    continue;
                }
                if visited.insert(start.clone()) {
                    queue.push_back((start, 0));
                }
            }

            while let Some((node_id, hops)) = queue.pop_front() {
                if hops >= max_hops {
                    continue;
                }
                let next_hops = hops.saturating_add(1);
                for next_id in adjacency.get(&node_id).into_iter().flatten() {
                    if visited.insert(next_id.clone()) {
                        queue.push_back((next_id.clone(), next_hops));
                    }
                }
            }

            visited
        }

        let state = self.lock_state_synced().await?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;
        let (mut objects, mut edges) = match diagram.ast() {
            DiagramAst::Class(ast) => {
                // Object-id adjacency (class nodes + relation endpoints).
                let mut adjacency: BTreeMap<ObjectId, BTreeSet<ObjectId>> = BTreeMap::new();
                for id in ast.classes().keys() {
                    adjacency.insert(id.clone(), BTreeSet::new());
                }
                for rel in ast.relations().values() {
                    let from = rel.from_class_id();
                    let to = rel.to_class_id();
                    if adjacency.contains_key(from) && adjacency.contains_key(to) {
                        adjacency.get_mut(from).expect("from").insert(to.clone());
                        adjacency.get_mut(to).expect("to").insert(from.clone());
                    }
                }
                let segments = center_ref_parsed.category().segments();
                let starts: Vec<ObjectId> = match segments {
                    [a, b] if a.as_str() == "class" && b.as_str() == "class" => {
                        let id = center_ref_parsed.object_id().clone();
                        if !ast.classes().contains_key(&id) {
                            return Err(ErrorData::resource_not_found(
                                "class not found",
                                Some(serde_json::json!({ "class_id": id.as_str() })),
                            ));
                        }
                        vec![id]
                    }
                    [a, b] if a.as_str() == "class" && b.as_str() == "relation" => {
                        let rel = ast.relations().get(center_ref_parsed.object_id()).ok_or_else(
                            || {
                                ErrorData::resource_not_found(
                                    "class relation not found",
                                    Some(serde_json::json!({
                                        "relation_id": center_ref_parsed.object_id().as_str()
                                    })),
                                )
                            },
                        )?;
                        vec![rel.from_class_id().clone(), rel.to_class_id().clone()]
                    }
                    _ => {
                        return Err(ErrorData::invalid_params(
                            "center_ref is not a class diagram object",
                            Some(serde_json::json!({ "center_ref": center_ref })),
                        ));
                    }
                };
                let nodes = bfs_within_radius(&adjacency, starts, max_hops);
                let mut rel_ids = BTreeSet::new();
                for (rid, rel) in ast.relations() {
                    if nodes.contains(rel.from_class_id()) && nodes.contains(rel.to_class_id()) {
                        rel_ids.insert(rid.clone());
                    }
                }
                let objects = nodes
                    .into_iter()
                    .map(|id| format!("d:{}/class/class/{}", diagram_id.as_str(), id))
                    .collect();
                let edges = rel_ids
                    .into_iter()
                    .map(|id| format!("d:{}/class/relation/{}", diagram_id.as_str(), id))
                    .collect();
                (objects, edges)
            }
            DiagramAst::Er(ast) => {
                let mut adjacency: BTreeMap<ObjectId, BTreeSet<ObjectId>> = BTreeMap::new();
                for id in ast.entities().keys() {
                    adjacency.insert(id.clone(), BTreeSet::new());
                }
                for rel in ast.relationships().values() {
                    let from = rel.from_entity_id();
                    let to = rel.to_entity_id();
                    if adjacency.contains_key(from) && adjacency.contains_key(to) {
                        adjacency.get_mut(from).expect("from").insert(to.clone());
                        adjacency.get_mut(to).expect("to").insert(from.clone());
                    }
                }
                let segments = center_ref_parsed.category().segments();
                let starts: Vec<ObjectId> = match segments {
                    [a, b] if a.as_str() == "er" && b.as_str() == "entity" => {
                        let id = center_ref_parsed.object_id().clone();
                        if !ast.entities().contains_key(&id) {
                            return Err(ErrorData::resource_not_found(
                                "entity not found",
                                Some(serde_json::json!({ "entity_id": id.as_str() })),
                            ));
                        }
                        vec![id]
                    }
                    [a, b] if a.as_str() == "er" && b.as_str() == "relationship" => {
                        let rel = ast
                            .relationships()
                            .get(center_ref_parsed.object_id())
                            .ok_or_else(|| {
                                ErrorData::resource_not_found(
                                    "relationship not found",
                                    Some(serde_json::json!({
                                        "relationship_id": center_ref_parsed.object_id().as_str()
                                    })),
                                )
                            })?;
                        vec![rel.from_entity_id().clone(), rel.to_entity_id().clone()]
                    }
                    _ => {
                        return Err(ErrorData::invalid_params(
                            "center_ref is not an er diagram object",
                            Some(serde_json::json!({ "center_ref": center_ref })),
                        ));
                    }
                };
                let nodes = bfs_within_radius(&adjacency, starts, max_hops);
                let mut rel_ids = BTreeSet::new();
                for (rid, rel) in ast.relationships() {
                    if nodes.contains(rel.from_entity_id()) && nodes.contains(rel.to_entity_id()) {
                        rel_ids.insert(rid.clone());
                    }
                }
                let objects = nodes
                    .into_iter()
                    .map(|id| format!("d:{}/er/entity/{}", diagram_id.as_str(), id))
                    .collect();
                let edges = rel_ids
                    .into_iter()
                    .map(|id| format!("d:{}/er/relationship/{}", diagram_id.as_str(), id))
                    .collect();
                (objects, edges)
            }
            DiagramAst::Gantt(ast) => {
                // Gantt has no structural edges; return the center task (if valid) only.
                let segments = center_ref_parsed.category().segments();
                match segments {
                    [a, b] if a.as_str() == "gantt" && b.as_str() == "task" => {
                        let id = center_ref_parsed.object_id().clone();
                        if !ast.tasks().contains_key(&id) {
                            return Err(ErrorData::resource_not_found(
                                "gantt task not found",
                                Some(serde_json::json!({ "task_id": id.as_str() })),
                            ));
                        }
                        (vec![format!("d:{}/gantt/task/{}", diagram_id.as_str(), id)], Vec::new())
                    }
                    _ => {
                        return Err(ErrorData::invalid_params(
                            "center_ref is not a gantt task object",
                            Some(serde_json::json!({ "center_ref": center_ref })),
                        ));
                    }
                }
            }
            DiagramAst::Flowchart(ast) => {
                let segments = center_ref_parsed.category().segments();
                let mut adjacency: BTreeMap<ObjectId, BTreeSet<ObjectId>> = BTreeMap::new();
                for node_id in ast.nodes().keys() {
                    adjacency.insert(node_id.clone(), BTreeSet::new());
                }
                for edge in ast.edges().values() {
                    let from = edge.from_node_id();
                    let to = edge.to_node_id();
                    if adjacency.contains_key(from) && adjacency.contains_key(to) {
                        adjacency.get_mut(from).expect("from node exists").insert(to.clone());
                        adjacency.get_mut(to).expect("to node exists").insert(from.clone());
                    }
                }

                let starts: Vec<ObjectId> = match segments {
                    [a, b] if a.as_str() == "flow" && b.as_str() == "node" => {
                        let node_id = center_ref_parsed.object_id().clone();
                        if !ast.nodes().contains_key(&node_id) {
                            return Err(ErrorData::resource_not_found(
                                "flow node not found",
                                Some(serde_json::json!({
                                    "diagram_id": diagram_id.as_str(),
                                    "node_id": node_id.as_str(),
                                })),
                            ));
                        }
                        vec![node_id]
                    }
                    [a, b] if a.as_str() == "flow" && b.as_str() == "edge" => {
                        let edge_id = center_ref_parsed.object_id().clone();
                        let edge = ast.edges().get(&edge_id).ok_or_else(|| {
                            ErrorData::resource_not_found(
                                "flow edge not found",
                                Some(serde_json::json!({
                                    "diagram_id": diagram_id.as_str(),
                                    "edge_id": edge_id.as_str(),
                                })),
                            )
                        })?;
                        vec![edge.from_node_id().clone(), edge.to_node_id().clone()]
                    }
                    _ => {
                        return Err(ErrorData::invalid_params(
                            "center_ref is not a flowchart object",
                            Some(serde_json::json!({ "center_ref": center_ref })),
                        ));
                    }
                };

                let nodes = bfs_within_radius(&adjacency, starts, max_hops);
                let mut edge_ids: BTreeSet<ObjectId> = BTreeSet::new();
                for (edge_id, edge) in ast.edges() {
                    if nodes.contains(edge.from_node_id()) && nodes.contains(edge.to_node_id()) {
                        edge_ids.insert(edge_id.clone());
                    }
                }

                let objects = nodes
                    .into_iter()
                    .map(|node_id| format!("d:{}/flow/node/{}", diagram_id.as_str(), node_id))
                    .collect::<Vec<_>>();
                let edges = edge_ids
                    .into_iter()
                    .map(|edge_id| format!("d:{}/flow/edge/{}", diagram_id.as_str(), edge_id))
                    .collect::<Vec<_>>();
                (objects, edges)
            }
            DiagramAst::Sequence(ast) => {
                fn insert_node(
                    adjacency: &mut BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
                    node: ObjectRef,
                ) {
                    adjacency.entry(node).or_default();
                }

                fn insert_edge(
                    adjacency: &mut BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
                    from: ObjectRef,
                    to: ObjectRef,
                ) {
                    adjacency.entry(from).or_default().insert(to);
                }

                fn bfs_refs(
                    adjacency: &BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
                    starts: impl IntoIterator<Item = ObjectRef>,
                    max_hops: usize,
                ) -> BTreeSet<ObjectRef> {
                    let mut visited: BTreeSet<ObjectRef> = BTreeSet::new();
                    let mut queue: VecDeque<(ObjectRef, usize)> = VecDeque::new();

                    for start in starts {
                        if !adjacency.contains_key(&start) {
                            continue;
                        }
                        if visited.insert(start.clone()) {
                            queue.push_back((start, 0));
                        }
                    }

                    while let Some((node, hops)) = queue.pop_front() {
                        if hops >= max_hops {
                            continue;
                        }
                        let next_hops = hops.saturating_add(1);
                        for next in adjacency.get(&node).into_iter().flatten() {
                            if visited.insert(next.clone()) {
                                queue.push_back((next.clone(), next_hops));
                            }
                        }
                    }

                    visited
                }

                let seq_participant_category =
                    CategoryPath::new(vec!["seq".to_owned(), "participant".to_owned()])
                        .expect("seq participant category");
                let seq_message_category =
                    CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()])
                        .expect("seq message category");
                let seq_block_category =
                    CategoryPath::new(vec!["seq".to_owned(), "block".to_owned()])
                        .expect("seq block category");
                let seq_section_category =
                    CategoryPath::new(vec!["seq".to_owned(), "section".to_owned()])
                        .expect("seq section category");

                let seq_participant_ref = |participant_id: &ObjectId| {
                    ObjectRef::new(
                        diagram_id.clone(),
                        seq_participant_category.clone(),
                        participant_id.clone(),
                    )
                };
                let seq_message_ref = |message_id: &ObjectId| {
                    ObjectRef::new(
                        diagram_id.clone(),
                        seq_message_category.clone(),
                        message_id.clone(),
                    )
                };
                let seq_block_ref = |block_id: &ObjectId| {
                    ObjectRef::new(diagram_id.clone(), seq_block_category.clone(), block_id.clone())
                };
                let seq_section_ref = |section_id: &ObjectId| {
                    ObjectRef::new(
                        diagram_id.clone(),
                        seq_section_category.clone(),
                        section_id.clone(),
                    )
                };

                let mut adjacency: BTreeMap<ObjectRef, BTreeSet<ObjectRef>> = BTreeMap::new();

                for participant_id in ast.participants().keys() {
                    insert_node(&mut adjacency, seq_participant_ref(participant_id));
                }

                for msg in ast.messages() {
                    let msg_ref = seq_message_ref(msg.message_id());
                    let from_ref = seq_participant_ref(msg.from_participant_id());
                    let to_ref = seq_participant_ref(msg.to_participant_id());
                    insert_node(&mut adjacency, msg_ref.clone());
                    insert_node(&mut adjacency, from_ref.clone());
                    insert_node(&mut adjacency, to_ref.clone());
                    insert_edge(&mut adjacency, from_ref.clone(), msg_ref.clone());
                    insert_edge(&mut adjacency, msg_ref.clone(), from_ref);
                    insert_edge(&mut adjacency, to_ref.clone(), msg_ref.clone());
                    insert_edge(&mut adjacency, msg_ref, to_ref);
                }

                fn add_block(
                    diagram_id: &DiagramId,
                    block: &crate::model::seq_ast::SequenceBlock,
                    adjacency: &mut BTreeMap<ObjectRef, BTreeSet<ObjectRef>>,
                    parent: Option<ObjectRef>,
                ) {
                    let seq_block_category =
                        CategoryPath::new(vec!["seq".to_owned(), "block".to_owned()])
                            .expect("seq block category");
                    let seq_section_category =
                        CategoryPath::new(vec!["seq".to_owned(), "section".to_owned()])
                            .expect("seq section category");
                    let seq_message_category =
                        CategoryPath::new(vec!["seq".to_owned(), "message".to_owned()])
                            .expect("seq message category");

                    let block_ref = ObjectRef::new(
                        diagram_id.clone(),
                        seq_block_category,
                        block.block_id().clone(),
                    );
                    adjacency.entry(block_ref.clone()).or_default();

                    if let Some(parent_ref) = parent.as_ref() {
                        adjacency.entry(parent_ref.clone()).or_default().insert(block_ref.clone());
                        adjacency.entry(block_ref.clone()).or_default().insert(parent_ref.clone());
                    }

                    for section in block.sections() {
                        let section_ref = ObjectRef::new(
                            diagram_id.clone(),
                            seq_section_category.clone(),
                            section.section_id().clone(),
                        );
                        adjacency.entry(section_ref.clone()).or_default();
                        adjacency.entry(block_ref.clone()).or_default().insert(section_ref.clone());
                        adjacency.entry(section_ref.clone()).or_default().insert(block_ref.clone());

                        for message_id in section.message_ids() {
                            let message_ref = ObjectRef::new(
                                diagram_id.clone(),
                                seq_message_category.clone(),
                                message_id.clone(),
                            );
                            adjacency.entry(message_ref.clone()).or_default();
                            adjacency
                                .entry(section_ref.clone())
                                .or_default()
                                .insert(message_ref.clone());
                            adjacency.entry(message_ref).or_default().insert(section_ref.clone());
                        }
                    }

                    for child in block.blocks() {
                        add_block(diagram_id, child, adjacency, Some(block_ref.clone()));
                    }
                }

                for block in ast.blocks() {
                    add_block(&diagram_id, block, &mut adjacency, None);
                }

                let segments = center_ref_parsed.category().segments();
                let starts: Vec<ObjectRef> = match segments {
                    [a, b] if a.as_str() == "seq" && b.as_str() == "participant" => {
                        let participant_id = center_ref_parsed.object_id().clone();
                        if !ast.participants().contains_key(&participant_id) {
                            return Err(ErrorData::resource_not_found(
                                "seq participant not found",
                                Some(serde_json::json!({
                                    "diagram_id": diagram_id.as_str(),
                                    "participant_id": participant_id.as_str(),
                                })),
                            ));
                        }
                        vec![seq_participant_ref(&participant_id)]
                    }
                    [a, b] if a.as_str() == "seq" && b.as_str() == "message" => {
                        let message_id = center_ref_parsed.object_id().clone();
                        let msg = ast
                            .messages()
                            .iter()
                            .find(|msg| msg.message_id() == &message_id)
                            .ok_or_else(|| {
                                ErrorData::resource_not_found(
                                    "seq message not found",
                                    Some(serde_json::json!({
                                        "diagram_id": diagram_id.as_str(),
                                        "message_id": message_id.as_str(),
                                    })),
                                )
                            })?;
                        vec![
                            seq_message_ref(&message_id),
                            seq_participant_ref(msg.from_participant_id()),
                            seq_participant_ref(msg.to_participant_id()),
                        ]
                    }
                    [a, b] if a.as_str() == "seq" && b.as_str() == "block" => {
                        let block_id = center_ref_parsed.object_id().clone();
                        if ast.find_block(&block_id).is_none() {
                            return Err(ErrorData::resource_not_found(
                                "seq block not found",
                                Some(serde_json::json!({
                                    "diagram_id": diagram_id.as_str(),
                                    "block_id": block_id.as_str(),
                                })),
                            ));
                        }
                        vec![seq_block_ref(&block_id)]
                    }
                    [a, b] if a.as_str() == "seq" && b.as_str() == "section" => {
                        let section_id = center_ref_parsed.object_id().clone();
                        if ast.find_section(&section_id).is_none() {
                            return Err(ErrorData::resource_not_found(
                                "seq section not found",
                                Some(serde_json::json!({
                                    "diagram_id": diagram_id.as_str(),
                                    "section_id": section_id.as_str(),
                                })),
                            ));
                        }
                        vec![seq_section_ref(&section_id)]
                    }
                    _ => {
                        return Err(ErrorData::invalid_params(
                            "center_ref is not a sequence diagram object",
                            Some(serde_json::json!({ "center_ref": center_ref })),
                        ));
                    }
                };

                let visited = bfs_refs(&adjacency, starts, max_hops);
                let mut objects = Vec::new();
                let mut edges = Vec::new();
                for item in visited {
                    let segments = item.category().segments();
                    if segments.len() == 2 && segments[0] == "seq" && segments[1] == "message" {
                        edges.push(item.to_string());
                    } else {
                        objects.push(item.to_string());
                    }
                }

                (objects, edges)
            }
        };

        objects.sort();
        edges.sort();

        if include_categories.is_some() || exclude_categories.is_some() {
            fn category_of(ref_str: &str) -> Result<String, ErrorData> {
                let parsed = ObjectRef::parse(ref_str).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid object ref: {err}"),
                        Some(serde_json::json!({ "object_ref": ref_str })),
                    )
                })?;
                Ok(parsed.category().segments().join("/"))
            }

            fn filter_refs(
                refs: Vec<String>,
                include: &Option<BTreeSet<String>>,
                exclude: &Option<BTreeSet<String>>,
            ) -> Result<Vec<String>, ErrorData> {
                let mut filtered = Vec::with_capacity(refs.len());
                for value in refs {
                    let category = category_of(&value)?;
                    if include
                        .as_ref()
                        .is_some_and(|set| !set.is_empty() && !set.contains(&category))
                    {
                        continue;
                    }
                    if exclude
                        .as_ref()
                        .is_some_and(|set| !set.is_empty() && set.contains(&category))
                    {
                        continue;
                    }
                    filtered.push(value);
                }
                Ok(filtered)
            }

            objects = filter_refs(objects, &include_categories, &exclude_categories)?;
            edges = filter_refs(edges, &include_categories, &exclude_categories)?;
        }

        Ok(Json(DiagramGetSliceResponse { objects, edges }))
    }

    /// Render diagram as deterministic text (Unicode allowed); use for human-readable snapshots
    /// and review, then return to `diagram_stat`/`diagram_get_slice` for targeted reasoning.
    #[tool(name = "diagram_render_text")]
    pub(super) async fn diagram_render_text(
        &self,
        params: Parameters<DiagramTargetParams>,
    ) -> Result<Json<DiagramRenderTextResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|active| active.as_str().to_owned());
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

        let text = render_diagram_unicode(diagram).map_err(|err| {
            ErrorData::internal_error(
                format!("render error: {err}"),
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(DiagramRenderTextResponse { text, context }))
    }

    /// Get diagram delta since a revision; default refresh step after `diagram_apply_ops` or
    /// external changes.
    #[tool(name = "diagram_diff")]
    pub(super) async fn diagram_diff(
        &self,
        params: Parameters<GetDeltaParams>,
    ) -> Result<Json<DiagramDeltaResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

        let current_rev = diagram.rev();
        let since_rev = params.0.since_rev;
        if since_rev > current_rev {
            return Err(ErrorData::invalid_params(
                "since_rev must be <= current rev",
                Some(serde_json::json!({ "since_rev": since_rev, "current_rev": current_rev })),
            ));
        }

        if since_rev == current_rev {
            return Ok(Json(DiagramDeltaResponse {
                from_rev: current_rev,
                to_rev: current_rev,
                changes: Vec::new(),
            }));
        }

        let Some(history) = state.delta_history.get(&diagram_id) else {
            return Err(delta_unavailable(since_rev, current_rev, current_rev));
        };

        let supported_since_rev = history.front().map(|d| d.from_rev).unwrap_or(current_rev);
        if since_rev < supported_since_rev {
            return Err(delta_unavailable(since_rev, current_rev, supported_since_rev));
        }

        let Some(delta) = delta_response_from_history(history, &diagram_id, since_rev, current_rev)
        else {
            return Err(delta_unavailable(since_rev, current_rev, supported_since_rev));
        };

        Ok(Json(delta))
    }

    /// Apply structured diagram ops gated by `base_rev`; prefer `diagram_propose_ops` first, then
    /// refresh with `diagram_diff`.
    #[tool(name = "diagram_apply_ops")]
    pub(super) async fn diagram_apply_ops(
        &self,
        params: Parameters<ApplyOpsParams>,
    ) -> Result<Json<ApplyOpsResponse>, ErrorData> {
        let ApplyOpsParams { diagram_id: requested_diagram_id, base_rev, ops } = params.0;
        let ops = ops.iter().map(mcp_op_to_internal).collect::<Result<Vec<_>, _>>()?;

        let mut state = self.lock_state_synced().await?;

        if let Some(session_folder) = &self.session_folder {
            let (candidate_session, diagram_id, history, response) = {
                let mut update = session_folder.begin_session_update().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to reload session before save: {err}"),
                        Some(serde_json::json!({
                            "diagram_id": requested_diagram_id.as_deref(),
                            "base_rev": base_rev
                        })),
                    )
                })?;
                let candidate_session = update.session_mut();
                let diagram_id =
                    resolve_diagram_id(candidate_session, requested_diagram_id.as_deref())?;
                let mut candidate_diagram = candidate_session
                    .diagrams()
                    .get(&diagram_id)
                    .cloned()
                    .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

                let current_rev = candidate_diagram.rev();
                if base_rev != current_rev {
                    let digest = digest_for_diagram(&candidate_diagram);
                    return Err(ErrorData::invalid_request(
                        "conflict: stale base_rev",
                        Some(serde_json::json!({
                            "base_rev": base_rev,
                            "current_rev": current_rev,
                            "snapshot_tool": "diagram_stat",
                            "digest": {
                                "rev": digest.rev,
                                "counts": {
                                    "participants": digest.counts.participants,
                                    "messages": digest.counts.messages,
                                    "nodes": digest.counts.nodes,
                                    "edges": digest.counts.edges,
                                },
                                "key_names": digest.key_names,
                            },
                        })),
                    ));
                }

                let result =
                    apply_ops(&mut candidate_diagram, base_rev, &ops).map_err(map_apply_error)?;
                render_diagram_unicode(&candidate_diagram).map_err(|err| {
                    ErrorData::invalid_request(
                        format!("cannot render diagram after apply_ops: {err}"),
                        Some(serde_json::json!({
                            "diagram_id": diagram_id.as_str(),
                            "base_rev": base_rev,
                            "op_count": ops.len() as u64,
                            "render_error": err.to_string(),
                        })),
                    )
                })?;
                candidate_session.diagrams_mut().insert(diagram_id.clone(), candidate_diagram);
                retain_existing_selected_object_refs(candidate_session);
                refresh_xref_statuses(candidate_session);

                let mut history =
                    state.delta_history.get(&diagram_id).cloned().unwrap_or_else(VecDeque::new);
                history.push_back(LastDelta {
                    from_rev: base_rev,
                    to_rev: result.new_rev,
                    delta: result.delta.clone(),
                    diagram_metadata_updated: false,
                });
                while history.len() > DELTA_HISTORY_LIMIT {
                    history.pop_front();
                }

                let candidate_session = update.commit().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to persist session: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id.as_str(), "base_rev": base_rev })),
                    )
                })?;

                let response = Json(ApplyOpsResponse {
                    new_rev: result.new_rev,
                    applied: result.applied as u64,
                    delta: DeltaSummary {
                        added: result.delta.added.iter().map(ToString::to_string).collect(),
                        removed: result.delta.removed.iter().map(ToString::to_string).collect(),
                        updated: result.delta.updated.iter().map(ToString::to_string).collect(),
                    },
                });

                (candidate_session, diagram_id, history, response)
            };
            replace_committed_session(&mut state, candidate_session);
            state.delta_history.insert(diagram_id, history);
            self.prune_missing_agent_highlights(&state.session).await;
            drop(state);
            self.notify_ui_session_changed().await;
            return Ok(response);
        }

        let diagram_id = resolve_diagram_id(&state.session, requested_diagram_id.as_deref())?;
        let diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

        let current_rev = diagram.rev();
        if base_rev != current_rev {
            let digest = digest_for_diagram(diagram);
            return Err(ErrorData::invalid_request(
                "conflict: stale base_rev",
                Some(serde_json::json!({
                    "base_rev": base_rev,
                    "current_rev": current_rev,
                    "snapshot_tool": "diagram_stat",
                    "digest": {
                        "rev": digest.rev,
                        "counts": {
                            "participants": digest.counts.participants,
                            "messages": digest.counts.messages,
                            "nodes": digest.counts.nodes,
                            "edges": digest.counts.edges,
                        },
                        "key_names": digest.key_names,
                    },
                })),
            ));
        }

        let mut candidate_diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .cloned()
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

        let result = apply_ops(&mut candidate_diagram, base_rev, &ops).map_err(map_apply_error)?;
        render_diagram_unicode(&candidate_diagram).map_err(|err| {
            ErrorData::invalid_request(
                format!("cannot render diagram after apply_ops: {err}"),
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "base_rev": base_rev,
                    "op_count": ops.len() as u64,
                    "render_error": err.to_string(),
                })),
            )
        })?;
        state.session.diagrams_mut().insert(diagram_id.clone(), candidate_diagram);
        retain_existing_selected_object_refs(&mut state.session);
        refresh_xref_statuses(&mut state.session);
        let history = state.delta_history.entry(diagram_id).or_insert_with(VecDeque::new);
        history.push_back(LastDelta {
            from_rev: base_rev,
            to_rev: result.new_rev,
            delta: result.delta.clone(),
            diagram_metadata_updated: false,
        });
        while history.len() > DELTA_HISTORY_LIMIT {
            history.pop_front();
        }

        let response = Json(ApplyOpsResponse {
            new_rev: result.new_rev,
            applied: result.applied as u64,
            delta: DeltaSummary {
                added: result.delta.added.iter().map(ToString::to_string).collect(),
                removed: result.delta.removed.iter().map(ToString::to_string).collect(),
                updated: result.delta.updated.iter().map(ToString::to_string).collect(),
            },
        });
        self.prune_missing_agent_highlights(&state.session).await;
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Validate ops against `base_rev` and return predicted delta without mutation; use immediately
    /// before `diagram_apply_ops` for safe human-agent collaboration.
    #[tool(name = "diagram_propose_ops")]
    pub(super) async fn diagram_propose_ops(
        &self,
        params: Parameters<DiagramProposeOpsParams>,
    ) -> Result<Json<DiagramProposeOpsResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;

        let diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

        let ops = params.0.ops.iter().map(mcp_op_to_internal).collect::<Result<Vec<_>, _>>()?;

        let base_rev = params.0.base_rev;

        let mut candidate = diagram.clone();
        let result = apply_ops(&mut candidate, base_rev, &ops).map_err(map_apply_error)?;
        render_diagram_unicode(&candidate).map_err(|err| {
            ErrorData::invalid_request(
                format!("cannot render diagram after propose_ops: {err}"),
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "base_rev": base_rev,
                    "op_count": ops.len() as u64,
                    "render_error": err.to_string(),
                })),
            )
        })?;

        Ok(Json(DiagramProposeOpsResponse {
            new_rev: result.new_rev,
            applied: result.applied as u64,
            delta: DeltaSummary {
                added: result.delta.added.iter().map(ToString::to_string).collect(),
                removed: result.delta.removed.iter().map(ToString::to_string).collect(),
                updated: result.delta.updated.iter().map(ToString::to_string).collect(),
            },
        }))
    }
}
