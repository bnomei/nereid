// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};

use crate::render::render_walkthrough_unicode;

use super::*;

#[tool_router(router = walkthrough_tool_router, vis = "pub(super)")]
impl NereidMcp {
    /// Set the active walkthrough default for walkthrough-scoped tools; usually after
    /// `walkthrough.list`.
    #[tool(name = "walkthrough.open")]
    pub(super) async fn walkthrough_open(
        &self,
        params: Parameters<WalkthroughOpenParams>,
    ) -> Result<Json<WalkthroughOpenResponse>, ErrorData> {
        let walkthrough_id = params.0.walkthrough_id;
        let parsed = parse_walkthrough_id(&walkthrough_id)?;

        let mut state = self.lock_state_synced().await?;
        if !state.session.walkthroughs().contains_key(&parsed) {
            return Err(ErrorData::resource_not_found(
                "walkthrough not found",
                Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
            ));
        }

        if let Some(session_folder) = &self.session_folder {
            // Reload the on-disk session immediately before persisting so concurrent edits
            // to diagrams/walkthroughs (e.g. from a TUI sharing this SessionFolder) are
            // preserved instead of being overwritten by a stale full-session snapshot.
            let mut candidate = session_folder.load_session().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to reload session before save: {err}"),
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
                )
            })?;
            if !candidate.walkthroughs().contains_key(&parsed) {
                return Err(ErrorData::resource_not_found(
                    "walkthrough not found",
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
                ));
            }
            candidate.set_active_walkthrough_id(Some(parsed.clone()));
            session_folder.save_session(&candidate).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
                )
            })?;
            state.session = candidate;
        } else {
            state.session.set_active_walkthrough_id(Some(parsed.clone()));
        }

        let response =
            Json(WalkthroughOpenResponse { active_walkthrough_id: parsed.as_str().to_owned() });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Get the active walkthrough id (`null` when unset); call after `walkthrough.list` and
    /// before `walkthrough.open`/`walkthrough.read`.
    #[tool(name = "walkthrough.current")]
    pub(super) async fn walkthrough_current(
        &self,
    ) -> Result<Json<WalkthroughCurrentResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let active_walkthrough_id = state
            .session
            .active_walkthrough_id()
            .map(|walkthrough_id| walkthrough_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(WalkthroughCurrentResponse { active_walkthrough_id, context }))
    }
    /// List walkthroughs in the current session; start here, then `walkthrough.open`,
    /// `walkthrough.stat`, or `walkthrough.read`.
    #[tool(name = "walkthrough.list")]
    pub(super) async fn walkthrough_list(
        &self,
    ) -> Result<Json<ListWalkthroughsResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let mut walkthroughs = state
            .session
            .walkthroughs()
            .iter()
            .map(|(walkthrough_id, walkthrough)| WalkthroughSummary {
                walkthrough_id: walkthrough_id.as_str().to_owned(),
                title: walkthrough.title().to_owned(),
                rev: walkthrough.rev(),
                nodes: walkthrough.nodes().len() as u64,
                edges: walkthrough.edges().len() as u64,
            })
            .collect::<Vec<_>>();
        walkthroughs.sort_by(|a, b| a.walkthrough_id.cmp(&b.walkthrough_id));
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(ListWalkthroughsResponse { walkthroughs, context }))
    }

    /// Read a full walkthrough (nodes/edges/refs); call after `walkthrough.stat` when you need
    /// complete node/edge detail, and before targeted `walkthrough.get_node`.
    #[tool(name = "walkthrough.read")]
    pub(super) async fn walkthrough_read(
        &self,
        params: Parameters<WalkthroughGetParams>,
    ) -> Result<Json<WalkthroughGetResponse>, ErrorData> {
        let walkthrough_id = params.0.walkthrough_id;
        let parsed = parse_walkthrough_id(&walkthrough_id)?;

        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let walkthrough = state.session.walkthroughs().get(&parsed).ok_or_else(|| {
            ErrorData::resource_not_found(
                "walkthrough not found",
                Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
            )
        })?;

        let nodes = walkthrough
            .nodes()
            .iter()
            .map(|node| McpWalkthroughNode {
                node_id: node.node_id().as_str().to_owned(),
                title: node.title().to_owned(),
                body_md: node.body_md().map(|body| body.to_owned()),
                refs: node.refs().iter().map(ToString::to_string).collect(),
                tags: node.tags().to_vec(),
                status: node.status().map(|status| status.to_owned()),
            })
            .collect::<Vec<_>>();

        let edges = walkthrough
            .edges()
            .iter()
            .map(|edge| McpWalkthroughEdge {
                from_node_id: edge.from_node_id().as_str().to_owned(),
                to_node_id: edge.to_node_id().as_str().to_owned(),
                kind: edge.kind().to_owned(),
                label: edge.label().map(|label| label.to_owned()),
            })
            .collect::<Vec<_>>();
        let walkthrough = McpWalkthrough {
            walkthrough_id: walkthrough.walkthrough_id().as_str().to_owned(),
            title: walkthrough.title().to_owned(),
            rev: walkthrough.rev(),
            nodes,
            edges,
        };

        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(WalkthroughGetResponse { walkthrough, context }))
    }

    /// Get one walkthrough node by id; use for drill-down after `walkthrough.list` or
    /// `walkthrough.read`.
    #[tool(name = "walkthrough.get_node")]
    pub(super) async fn walkthrough_get_node(
        &self,
        params: Parameters<WalkthroughGetNodeParams>,
    ) -> Result<Json<WalkthroughGetNodeResponse>, ErrorData> {
        let WalkthroughGetNodeParams { walkthrough_id, node_id } = params.0;
        let parsed_walkthrough_id = parse_walkthrough_id(&walkthrough_id)?;
        let parsed_node_id = parse_walkthrough_node_id(&node_id)?;

        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let walkthrough =
            state.session.walkthroughs().get(&parsed_walkthrough_id).ok_or_else(|| {
                ErrorData::resource_not_found(
                    "walkthrough not found",
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id.as_str() })),
                )
            })?;

        let node =
            walkthrough.nodes().iter().find(|node| node.node_id() == &parsed_node_id).ok_or_else(
                || {
                    ErrorData::resource_not_found(
                        "walkthrough node not found",
                        Some(serde_json::json!({
                            "walkthrough_id": walkthrough_id.as_str(),
                            "node_id": node_id.as_str(),
                        })),
                    )
                },
            )?;
        let node = McpWalkthroughNode {
            node_id: node.node_id().as_str().to_owned(),
            title: node.title().to_owned(),
            body_md: node.body_md().map(|body| body.to_owned()),
            refs: node.refs().iter().map(ToString::to_string).collect(),
            tags: node.tags().to_vec(),
            status: node.status().map(|status| status.to_owned()),
        };

        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(WalkthroughGetNodeResponse { node, context }))
    }

    /// Read current walkthrough revision and counts; call before walkthrough mutations.
    #[tool(name = "walkthrough.stat")]
    pub(super) async fn walkthrough_stat(
        &self,
        params: Parameters<WalkthroughGetParams>,
    ) -> Result<Json<WalkthroughGetDigestResponse>, ErrorData> {
        let walkthrough_id = params.0.walkthrough_id;
        let parsed = parse_walkthrough_id(&walkthrough_id)?;

        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let walkthrough = state.session.walkthroughs().get(&parsed).ok_or_else(|| {
            ErrorData::resource_not_found(
                "walkthrough not found",
                Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
            )
        })?;
        let digest = digest_for_walkthrough(walkthrough);
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(WalkthroughGetDigestResponse { digest, context }))
    }

    /// Render walkthrough text for human-readable sharing/export; prefer
    /// `walkthrough.stat`/`walkthrough.read` for machine reasoning and follow-up edits.
    #[tool(name = "walkthrough.render_text")]
    pub(super) async fn walkthrough_render_text(
        &self,
        params: Parameters<WalkthroughGetParams>,
    ) -> Result<Json<WalkthroughRenderTextResponse>, ErrorData> {
        let walkthrough_id = params.0.walkthrough_id;
        let parsed = parse_walkthrough_id(&walkthrough_id)?;

        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        let walkthrough = state.session.walkthroughs().get(&parsed).ok_or_else(|| {
            ErrorData::resource_not_found(
                "walkthrough not found",
                Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
            )
        })?;

        let text = render_walkthrough_unicode(walkthrough).map_err(|err| {
            ErrorData::invalid_request(
                format!("render error: {err}"),
                Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
            )
        })?;
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;

        Ok(Json(WalkthroughRenderTextResponse { text, context }))
    }

    /// Read walkthrough delta since a revision; call after mutations to verify applied changes.
    #[tool(name = "walkthrough.diff")]
    pub(super) async fn walkthrough_diff(
        &self,
        params: Parameters<WalkthroughGetDeltaParams>,
    ) -> Result<Json<WalkthroughDeltaResponse>, ErrorData> {
        let walkthrough_id = params.0.walkthrough_id;
        let parsed = parse_walkthrough_id(&walkthrough_id)?;

        let state = self.lock_state_synced().await?;
        let walkthrough = state.session.walkthroughs().get(&parsed).ok_or_else(|| {
            ErrorData::resource_not_found(
                "walkthrough not found",
                Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
            )
        })?;

        let current_rev = walkthrough.rev();
        let since_rev = params.0.since_rev;
        if since_rev > current_rev {
            return Err(ErrorData::invalid_params(
                "since_rev must be <= current rev",
                Some(serde_json::json!({ "since_rev": since_rev, "current_rev": current_rev })),
            ));
        }

        if since_rev == current_rev {
            return Ok(Json(WalkthroughDeltaResponse {
                from_rev: current_rev,
                to_rev: current_rev,
                changes: Vec::new(),
            }));
        }

        let Some(history) = state.walkthrough_delta_history.get(&parsed) else {
            return Err(walkthrough_delta_unavailable(since_rev, current_rev, current_rev));
        };

        let supported_since_rev = history.front().map(|d| d.from_rev).unwrap_or(current_rev);
        if since_rev < supported_since_rev {
            return Err(walkthrough_delta_unavailable(since_rev, current_rev, supported_since_rev));
        }

        let Some(delta) = walkthrough_delta_response_from_history(history, since_rev, current_rev)
        else {
            return Err(walkthrough_delta_unavailable(since_rev, current_rev, supported_since_rev));
        };

        Ok(Json(delta))
    }

    /// Apply walkthrough ops using `base_rev` from `walkthrough.stat`; on conflict, refresh and retry.
    #[tool(name = "walkthrough.apply_ops")]
    pub(super) async fn walkthrough_apply_ops(
        &self,
        params: Parameters<WalkthroughApplyOpsParams>,
    ) -> Result<Json<ApplyOpsResponse>, ErrorData> {
        let WalkthroughApplyOpsParams { walkthrough_id, base_rev, ops } = params.0;
        let parsed = parse_walkthrough_id(&walkthrough_id)?;

        let mut state = self.lock_state_synced().await?;

        if let Some(session_folder) = &self.session_folder {
            // Reload the on-disk session immediately before persisting so concurrent edits
            // to other diagrams/walkthroughs (e.g. from a TUI sharing this SessionFolder)
            // are preserved instead of being overwritten by a stale full-session snapshot.
            // The `base_rev` check below runs against this freshly loaded walkthrough, so a
            // concurrent edit to this same walkthrough is reported as a conflict.
            let mut candidate_session = session_folder.load_session().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to reload session before save: {err}"),
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id, "base_rev": base_rev })),
                )
            })?;
            let walkthrough =
                candidate_session.walkthroughs_mut().get_mut(&parsed).ok_or_else(|| {
                    ErrorData::resource_not_found(
                        "walkthrough not found",
                        Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
                    )
                })?;

            let current_rev = walkthrough.rev();
            if base_rev != current_rev {
                let digest = digest_for_walkthrough(walkthrough);
                return Err(ErrorData::invalid_request(
                    "conflict: stale base_rev",
                    Some(serde_json::json!({
                        "base_rev": base_rev,
                        "current_rev": current_rev,
                        "snapshot_tool": "walkthrough.stat",
                        "digest": {
                            "rev": digest.rev,
                            "counts": {
                                "nodes": digest.counts.nodes,
                                "edges": digest.counts.edges,
                            },
                        },
                    })),
                ));
            }

            if ops.is_empty() {
                return Ok(Json(ApplyOpsResponse {
                    new_rev: current_rev,
                    applied: 0,
                    delta: DeltaSummary {
                        added: Vec::new(),
                        removed: Vec::new(),
                        updated: Vec::new(),
                    },
                }));
            }

            let delta = apply_walkthrough_ops(walkthrough, &parsed, &ops)?;
            walkthrough.bump_rev();
            let new_rev = walkthrough.rev();

            let mut history =
                state.walkthrough_delta_history.get(&parsed).cloned().unwrap_or_else(VecDeque::new);
            history.push_back(WalkthroughLastDelta {
                from_rev: base_rev,
                to_rev: new_rev,
                delta: delta.clone(),
            });
            while history.len() > DELTA_HISTORY_LIMIT {
                history.pop_front();
            }

            session_folder.save_session(&candidate_session).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id, "base_rev": base_rev })),
                )
            })?;

            state.session = candidate_session;
            state.walkthrough_delta_history.insert(parsed, history);

            let response = Json(ApplyOpsResponse {
                new_rev,
                applied: ops.len() as u64,
                delta: DeltaSummary {
                    added: delta.added.iter().cloned().collect(),
                    removed: delta.removed.iter().cloned().collect(),
                    updated: delta.updated.iter().cloned().collect(),
                },
            });
            drop(state);
            self.notify_ui_session_changed().await;
            return Ok(response);
        }

        let walkthrough = state.session.walkthroughs_mut().get_mut(&parsed).ok_or_else(|| {
            ErrorData::resource_not_found(
                "walkthrough not found",
                Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
            )
        })?;

        let current_rev = walkthrough.rev();
        if base_rev != current_rev {
            let digest = digest_for_walkthrough(walkthrough);
            return Err(ErrorData::invalid_request(
                "conflict: stale base_rev",
                Some(serde_json::json!({
                    "base_rev": base_rev,
                    "current_rev": current_rev,
                    "snapshot_tool": "walkthrough.stat",
                    "digest": {
                        "rev": digest.rev,
                        "counts": {
                            "nodes": digest.counts.nodes,
                            "edges": digest.counts.edges,
                        },
                    },
                })),
            ));
        }

        if ops.is_empty() {
            return Ok(Json(ApplyOpsResponse {
                new_rev: current_rev,
                applied: 0,
                delta: DeltaSummary { added: Vec::new(), removed: Vec::new(), updated: Vec::new() },
            }));
        }

        // Apply on a clone and commit only after every op succeeds, so a mid-batch failure
        // leaves the live walkthrough unchanged. This mirrors diagram.apply_ops and the
        // persistent walkthrough path; applying directly would leave partial mutations behind
        // when a later op in the batch errors.
        let mut candidate = walkthrough.clone();
        let delta = apply_walkthrough_ops(&mut candidate, &parsed, &ops)?;
        candidate.bump_rev();
        let new_rev = candidate.rev();
        *walkthrough = candidate;

        let history = state.walkthrough_delta_history.entry(parsed).or_insert_with(VecDeque::new);
        history.push_back(WalkthroughLastDelta {
            from_rev: base_rev,
            to_rev: new_rev,
            delta: delta.clone(),
        });
        while history.len() > DELTA_HISTORY_LIMIT {
            history.pop_front();
        }

        let response = Json(ApplyOpsResponse {
            new_rev,
            applied: ops.len() as u64,
            delta: DeltaSummary {
                added: delta.added.iter().cloned().collect(),
                removed: delta.removed.iter().cloned().collect(),
                updated: delta.updated.iter().cloned().collect(),
            },
        });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }
}
