// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};

use super::*;

#[tool_router(router = queries_tool_router, vis = "pub(super)")]
impl NereidMcp {
    /// Find cross-diagram routes between two object refs; combine with `xref.neighbors` and
    /// `diagram.get_slice` for explain-and-refine flows.
    #[tool(name = "route.find")]
    pub(super) async fn route_find(
        &self,
        params: Parameters<RouteFindParams>,
    ) -> Result<Json<RouteFindResponse>, ErrorData> {
        let RouteFindParams { from_ref, to_ref, limit, max_hops, ordering } = params.0;

        let from_ref = parse_object_ref_from_ref(&from_ref)?;
        let to_ref = parse_object_ref_to_ref(&to_ref)?;

        let limit = limit.unwrap_or(1);
        if limit == 0 {
            return Ok(Json(RouteFindResponse { routes: Vec::new() }));
        }

        let ordering = match ordering.as_deref().filter(|value| !value.is_empty()) {
            None | Some("fewest_hops") => crate::query::session_routes::RoutesOrdering::FewestHops,
            Some("lexicographic") => crate::query::session_routes::RoutesOrdering::Lexicographic,
            Some(other) => {
                return Err(ErrorData::invalid_params(
                    "invalid ordering (expected fewest_hops|lexicographic)",
                    Some(serde_json::json!({ "ordering": other })),
                ));
            }
        };

        let state = self.lock_state_synced().await?;
        let routes = crate::query::session_routes::find_routes(
            &state.session,
            &from_ref,
            &to_ref,
            limit,
            max_hops,
            ordering,
        );

        Ok(Json(RouteFindResponse {
            routes: routes
                .into_iter()
                .map(|route| route.into_iter().map(|item| item.to_string()).collect())
                .collect(),
        }))
    }
    /// Trace sequence message order before/after a message id (returns refs); use for timeline
    /// explanations and local impact checks.
    #[tool(name = "seq.trace")]
    pub(super) async fn seq_trace(
        &self,
        params: Parameters<SeqTraceParams>,
    ) -> Result<Json<SeqTraceResponse>, ErrorData> {
        let SeqTraceParams { diagram_id, from_message_id, direction, limit } = params.0;

        #[derive(Debug, Clone, Copy)]
        enum TraceDirection {
            Before,
            After,
        }

        let direction = match direction.as_deref().unwrap_or("after") {
            "before" => TraceDirection::Before,
            "after" => TraceDirection::After,
            other => {
                return Err(ErrorData::invalid_params(
                    "invalid direction (expected before|after)",
                    Some(serde_json::json!({ "direction": other })),
                ));
            }
        };

        let limit_raw = limit.unwrap_or(25);
        let limit = usize::try_from(limit_raw).map_err(|_| {
            ErrorData::invalid_params(
                "limit too large",
                Some(serde_json::json!({ "limit": limit_raw })),
            )
        })?;

        let from_message_id = from_message_id.as_deref().map(parse_object_id).transpose()?;

        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Sequence(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a sequence diagram",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        let messages = if let Some(from_message_id) = from_message_id.as_ref() {
            let traced = match direction {
                TraceDirection::After => {
                    crate::query::sequence::trace_after(ast, from_message_id, limit)
                }
                TraceDirection::Before => {
                    crate::query::sequence::trace_before(ast, from_message_id, limit)
                }
            }
            .ok_or_else(|| {
                ErrorData::resource_not_found(
                    "seq message not found",
                    Some(serde_json::json!({
                        "diagram_id": diagram_id.as_str(),
                        "from_message_id": from_message_id.as_str(),
                    })),
                )
            })?;
            traced
        } else {
            let messages = ast.messages_in_order();

            match direction {
                TraceDirection::After => messages.into_iter().take(limit).collect(),
                TraceDirection::Before => {
                    let start_index = messages.len().saturating_sub(limit);
                    messages[start_index..].to_vec()
                }
            }
        };

        let messages = messages
            .into_iter()
            .map(|msg| format!("d:{}/seq/message/{}", diagram_id.as_str(), msg.message_id()))
            .collect::<Vec<_>>();

        Ok(Json(SeqTraceResponse { messages }))
    }

    /// Search sequence messages by substring/regex (returns refs); typically feed results into
    /// `object.read`, `seq.trace`, or attention/selection updates.
    #[tool(name = "seq.search")]
    pub(super) async fn seq_search(
        &self,
        params: Parameters<SeqSearchParams>,
    ) -> Result<Json<SeqSearchResponse>, ErrorData> {
        let SeqSearchParams { diagram_id, needle, mode, case_insensitive } = params.0;

        if needle.is_empty() {
            return Err(ErrorData::invalid_params(
                "needle must not be empty",
                Some(serde_json::json!({ "needle": needle })),
            ));
        }

        let mode_label = mode.as_deref().unwrap_or("substring");
        let mode = match mode_label {
            "substring" => crate::query::sequence::MessageSearchMode::Substring,
            "regex" => crate::query::sequence::MessageSearchMode::Regex,
            other => {
                return Err(ErrorData::invalid_params(
                    "invalid mode (expected substring|regex)",
                    Some(serde_json::json!({ "mode": other })),
                ));
            }
        };
        let case_insensitive = case_insensitive.unwrap_or(true);

        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Sequence(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a sequence diagram",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        let messages = crate::query::sequence::message_search(ast, &needle, mode, case_insensitive)
            .map_err(|err| {
                ErrorData::invalid_params(
                    format!("invalid regex: {err}"),
                    Some(serde_json::json!({
                        "needle": needle,
                        "mode": mode_label,
                        "case_insensitive": case_insensitive,
                    })),
                )
            })?
            .into_iter()
            .map(|msg| format!("d:{}/seq/message/{}", diagram_id.as_str(), msg.message_id()))
            .collect::<Vec<_>>();

        Ok(Json(SeqSearchResponse { messages }))
    }

    /// List sequence messages (returns refs) with optional filters; good starting point before
    /// `seq.trace` or targeted mutation planning.
    #[tool(name = "seq.messages")]
    pub(super) async fn seq_messages(
        &self,
        params: Parameters<SeqMessagesParams>,
    ) -> Result<Json<SeqMessagesResponse>, ErrorData> {
        let SeqMessagesParams { diagram_id, from_participant_id, to_participant_id } = params.0;

        let from_participant_id = from_participant_id
            .as_deref()
            .map(|from_participant_id| {
                ObjectId::new(from_participant_id.to_owned()).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid from_participant_id: {err}"),
                        Some(serde_json::json!({ "from_participant_id": from_participant_id })),
                    )
                })
            })
            .transpose()?;
        let to_participant_id = to_participant_id
            .as_deref()
            .map(|to_participant_id| {
                ObjectId::new(to_participant_id.to_owned()).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid to_participant_id: {err}"),
                        Some(serde_json::json!({ "to_participant_id": to_participant_id })),
                    )
                })
            })
            .transpose()?;

        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Sequence(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a sequence diagram",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        let mut messages = ast
            .messages()
            .iter()
            .filter(|msg| {
                from_participant_id.as_ref().is_none_or(|from| msg.from_participant_id() == from)
            })
            .filter(|msg| to_participant_id.as_ref().is_none_or(|to| msg.to_participant_id() == to))
            .collect::<Vec<_>>();
        messages.sort_by(|a, b| crate::model::SequenceMessage::cmp_in_order(a, b));

        let messages = messages
            .into_iter()
            .map(|msg| format!("d:{}/seq/message/{}", diagram_id.as_str(), msg.message_id()))
            .collect::<Vec<_>>();

        Ok(Json(SeqMessagesResponse { messages }))
    }

    /// List flow nodes reachable from a node id (returns refs); pair with `flow.paths` and
    /// `diagram.get_slice` for local traversal.
    #[tool(name = "flow.reachable")]
    pub(super) async fn flow_reachable(
        &self,
        params: Parameters<FlowReachableParams>,
    ) -> Result<Json<FlowReachableResponse>, ErrorData> {
        let FlowReachableParams { diagram_id, from_node_id, direction } = params.0;

        let direction_label = direction.as_deref().unwrap_or("out");
        let direction = match direction_label {
            "out" => crate::query::flow::ReachDirection::Out,
            "in" => crate::query::flow::ReachDirection::In,
            "both" => crate::query::flow::ReachDirection::Both,
            other => {
                return Err(ErrorData::invalid_params(
                    "invalid direction (expected out|in|both)",
                    Some(serde_json::json!({ "direction": other })),
                ));
            }
        };

        let from_node_id_parsed = ObjectId::new(from_node_id.clone()).map_err(|err| {
            ErrorData::invalid_params(
                format!("invalid from_node_id: {err}"),
                Some(serde_json::json!({ "from_node_id": from_node_id })),
            )
        })?;

        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Flowchart(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a flowchart",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        // Surface a missing node consistently with sibling flow tools (`flow.paths`,
        // `flow.unreachable`) instead of returning an empty success, which is indistinguishable
        // from a valid node that simply reaches nothing.
        if !ast.nodes().contains_key(&from_node_id_parsed) {
            return Err(ErrorData::resource_not_found(
                "from node not found",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "from_node_id": from_node_id,
                })),
            ));
        }

        let reachable =
            crate::query::flow::reachable_with_direction(ast, &from_node_id_parsed, direction);

        let mut nodes = reachable
            .into_iter()
            .map(|node_id| format!("d:{}/flow/node/{}", diagram_id.as_str(), node_id))
            .collect::<Vec<_>>();
        nodes.sort();

        Ok(Json(FlowReachableResponse { nodes }))
    }

    /// Find bounded paths between two flow nodes (returns ref paths); use after
    /// `flow.reachable`/`object.read` to explain alternatives.
    #[tool(name = "flow.paths")]
    pub(super) async fn flow_paths(
        &self,
        params: Parameters<FlowPathsParams>,
    ) -> Result<Json<FlowPathsResponse>, ErrorData> {
        let FlowPathsParams { diagram_id, from_node_id, to_node_id, limit, max_extra_hops } =
            params.0;

        let limit_u64 = limit.unwrap_or(10);
        let max_extra_hops_u64 = max_extra_hops.unwrap_or(0);

        let limit = usize::try_from(limit_u64).map_err(|_| {
            ErrorData::invalid_params(
                "limit is too large",
                Some(serde_json::json!({ "limit": limit_u64 })),
            )
        })?;
        let max_extra_hops = usize::try_from(max_extra_hops_u64).map_err(|_| {
            ErrorData::invalid_params(
                "max_extra_hops is too large",
                Some(serde_json::json!({ "max_extra_hops": max_extra_hops_u64 })),
            )
        })?;

        let from_node_id_parsed = ObjectId::new(from_node_id.clone()).map_err(|err| {
            ErrorData::invalid_params(
                format!("invalid from_node_id: {err}"),
                Some(serde_json::json!({ "from_node_id": from_node_id })),
            )
        })?;
        let to_node_id_parsed = ObjectId::new(to_node_id.clone()).map_err(|err| {
            ErrorData::invalid_params(
                format!("invalid to_node_id: {err}"),
                Some(serde_json::json!({ "to_node_id": to_node_id })),
            )
        })?;

        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Flowchart(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a flowchart",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        if !ast.nodes().contains_key(&from_node_id_parsed) {
            return Err(ErrorData::resource_not_found(
                "from node not found",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "from_node_id": from_node_id,
                })),
            ));
        }
        if !ast.nodes().contains_key(&to_node_id_parsed) {
            return Err(ErrorData::resource_not_found(
                "to node not found",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "to_node_id": to_node_id,
                })),
            ));
        }

        let paths = crate::query::flow::paths(
            ast,
            &from_node_id_parsed,
            &to_node_id_parsed,
            limit,
            max_extra_hops,
        )
        .into_iter()
        .map(|path| {
            path.into_iter()
                .map(|node_id| format!("d:{}/flow/node/{}", diagram_id.as_str(), node_id))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

        Ok(Json(FlowPathsResponse { paths }))
    }

    /// Detect flowchart cycles (returns node ref cycles); use for risk checks before edits.
    #[tool(name = "flow.cycles")]
    pub(super) async fn flow_cycles(
        &self,
        params: Parameters<DiagramTargetParams>,
    ) -> Result<Json<FlowCyclesResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Flowchart(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a flowchart",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        let cycles = crate::query::flow::cycles(ast)
            .into_iter()
            .map(|cycle| {
                cycle
                    .into_iter()
                    .map(|node_id| format!("d:{}/flow/node/{}", diagram_id.as_str(), node_id))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(Json(FlowCyclesResponse { cycles }))
    }

    /// List terminal flowchart nodes (returns refs); combine with `flow.unreachable` to identify
    /// dead routes.
    #[tool(name = "flow.dead_ends")]
    pub(super) async fn flow_dead_ends(
        &self,
        params: Parameters<DiagramTargetParams>,
    ) -> Result<Json<FlowDeadEndsResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Flowchart(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a flowchart",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        let mut nodes = crate::query::flow::dead_ends(ast)
            .into_iter()
            .map(|node_id| format!("d:{}/flow/node/{}", diagram_id.as_str(), node_id))
            .collect::<Vec<_>>();
        nodes.sort();

        Ok(Json(FlowDeadEndsResponse { nodes }))
    }

    /// Compute flow fan-in/fan-out degrees (returns refs + counts); use to identify hubs and
    /// bottlenecks before refactoring.
    #[tool(name = "flow.degrees")]
    pub(super) async fn flow_degrees(
        &self,
        params: Parameters<FlowDegreesParams>,
    ) -> Result<Json<FlowDegreesResponse>, ErrorData> {
        #[derive(Clone, Copy)]
        enum SortBy {
            In,
            Out,
            Total,
        }

        let FlowDegreesParams { diagram_id, top, sort_by } = params.0;

        let top_u64 = top.unwrap_or(10);
        let top = usize::try_from(top_u64).map_err(|_| {
            ErrorData::invalid_params(
                "top is too large",
                Some(serde_json::json!({ "top": top_u64 })),
            )
        })?;
        if top == 0 {
            return Ok(Json(FlowDegreesResponse { nodes: Vec::new() }));
        }

        let sort_by = sort_by.as_deref().unwrap_or("out");
        let sort_by = match sort_by {
            "" | "out" => SortBy::Out,
            "in" => SortBy::In,
            "total" => SortBy::Total,
            other => {
                return Err(ErrorData::invalid_params(
                    "invalid sort_by (expected out|in|total)",
                    Some(serde_json::json!({ "sort_by": other })),
                ));
            }
        };

        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Flowchart(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a flowchart",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        let degrees = crate::query::flow::degrees(ast);
        let mut nodes = degrees
            .iter()
            .map(|(node_id, degree)| FlowDegreeNode {
                node_ref: format!("d:{}/flow/node/{}", diagram_id.as_str(), node_id),
                label: ast
                    .nodes()
                    .get(node_id)
                    .map(|node| node.label().to_owned())
                    .unwrap_or_default(),
                in_degree: degree.in_degree,
                out_degree: degree.out_degree,
            })
            .collect::<Vec<_>>();

        nodes.sort_by(|a, b| {
            let score_a = match sort_by {
                SortBy::In => a.in_degree,
                SortBy::Out => a.out_degree,
                SortBy::Total => a.in_degree.saturating_add(a.out_degree),
            };
            let score_b = match sort_by {
                SortBy::In => b.in_degree,
                SortBy::Out => b.out_degree,
                SortBy::Total => b.in_degree.saturating_add(b.out_degree),
            };
            score_b.cmp(&score_a).then_with(|| a.node_ref.cmp(&b.node_ref))
        });

        nodes.truncate(top);

        Ok(Json(FlowDegreesResponse { nodes }))
    }

    /// List nodes unreachable from start nodes (returns refs); use for cleanup/TODO mapping and
    /// follow with `diagram.get_slice`.
    #[tool(name = "flow.unreachable")]
    pub(super) async fn flow_unreachable(
        &self,
        params: Parameters<FlowUnreachableParams>,
    ) -> Result<Json<FlowUnreachableResponse>, ErrorData> {
        let FlowUnreachableParams { diagram_id, start_node_id } = params.0;

        let start_node_id = start_node_id
            .as_deref()
            .map(|start_node_id| {
                ObjectId::new(start_node_id.to_owned()).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid start_node_id: {err}"),
                        Some(serde_json::json!({ "start_node_id": start_node_id })),
                    )
                })
            })
            .transpose()?;

        let state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, diagram_id.as_deref())?;
        let diagram = state.session.diagrams().get(&diagram_id).ok_or_else(|| {
            ErrorData::resource_not_found(
                "diagram not found",
                Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
            )
        })?;

        let DiagramAst::Flowchart(ast) = diagram.ast() else {
            return Err(ErrorData::invalid_params(
                "diagram is not a flowchart",
                Some(serde_json::json!({
                    "diagram_id": diagram_id.as_str(),
                    "diagram_kind": diagram_kind_label(diagram.kind()),
                })),
            ));
        };

        let mut outgoing: BTreeMap<ObjectId, BTreeSet<ObjectId>> = BTreeMap::new();
        let mut indegree: BTreeMap<ObjectId, usize> = BTreeMap::new();

        for node_id in ast.nodes().keys() {
            outgoing.insert(node_id.clone(), BTreeSet::new());
            indegree.insert(node_id.clone(), 0);
        }

        for edge in ast.edges().values() {
            let from = edge.from_node_id();
            let to = edge.to_node_id();
            if outgoing.contains_key(from) && outgoing.contains_key(to) {
                outgoing.get_mut(from).expect("node exists").insert(to.clone());
                *indegree.get_mut(to).expect("node exists") += 1;
            }
        }

        let starts: Vec<ObjectId> = if let Some(start_node_id) = start_node_id.as_ref() {
            if !outgoing.contains_key(start_node_id) {
                return Err(ErrorData::invalid_params(
                    "start node not found",
                    Some(serde_json::json!({
                        "diagram_id": diagram_id.as_str(),
                        "start_node_id": start_node_id.as_str(),
                    })),
                ));
            }
            vec![start_node_id.clone()]
        } else {
            let mut starts = indegree
                .iter()
                .filter(|(_node_id, degree)| **degree == 0)
                .map(|(node_id, _degree)| node_id.clone())
                .collect::<Vec<_>>();

            if starts.is_empty() {
                starts = outgoing.keys().cloned().collect();
            }

            starts
        };

        fn bfs(
            adjacency: &BTreeMap<ObjectId, BTreeSet<ObjectId>>,
            starts: impl IntoIterator<Item = ObjectId>,
        ) -> BTreeSet<ObjectId> {
            let mut visited: BTreeSet<ObjectId> = BTreeSet::new();
            let mut queue: VecDeque<ObjectId> = VecDeque::new();

            for start in starts {
                if !adjacency.contains_key(&start) {
                    continue;
                }
                if visited.insert(start.clone()) {
                    queue.push_back(start);
                }
            }

            while let Some(node_id) = queue.pop_front() {
                for next_id in adjacency.get(&node_id).into_iter().flatten() {
                    if visited.insert(next_id.clone()) {
                        queue.push_back(next_id.clone());
                    }
                }
            }

            visited
        }

        let reachable = bfs(&outgoing, starts);

        let mut nodes = outgoing
            .keys()
            .filter(|node_id| !reachable.contains(*node_id))
            .map(|node_id| format!("d:{}/flow/node/{}", diagram_id.as_str(), node_id))
            .collect::<Vec<_>>();
        nodes.sort();

        Ok(Json(FlowUnreachableResponse { nodes }))
    }
}
