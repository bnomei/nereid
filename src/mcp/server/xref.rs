// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Xref MCP tools: list/filter, add/remove, and neighbor traversal across diagrams.

use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::{tool, tool_router};

use crate::model::XRef;

use super::*;

#[tool_router(router = xref_tool_router, vis = "pub(super)")]
impl NereidMcp {
    /// List session xrefs (including dangling filters); use to audit mappings before route/search
    /// exploration or cleanup.
    #[tool(name = "xref_list")]
    pub(super) async fn xref_list(
        &self,
        params: Parameters<XRefListParams>,
    ) -> Result<Json<XRefListResponse>, ErrorData> {
        #[derive(Clone, Copy)]
        enum StatusFilter<'a> {
            Exact(&'a str),
            AnyDangling,
        }

        let XRefListParams {
            dangling_only,
            status,
            kind,
            from_ref,
            to_ref,
            involves_ref,
            label_contains,
            limit,
        } = params.0;

        let dangling_only = dangling_only.unwrap_or(false);
        let status = status.as_deref().filter(|status| !status.is_empty());
        let status_filter = match status {
            None => None,
            Some("dangling_*") => Some(StatusFilter::AnyDangling),
            Some("ok") | Some("dangling_from") | Some("dangling_to") | Some("dangling_both") => {
                Some(StatusFilter::Exact(status.expect("status is Some")))
            }
            Some(other) => {
                return Err(ErrorData::invalid_params(
                    "invalid status (expected ok|dangling_from|dangling_to|dangling_both|dangling_*)",
                    Some(serde_json::json!({ "status": other })),
                ));
            }
        };

        let kind = kind.filter(|kind| !kind.is_empty());
        let label_contains = label_contains.filter(|label_contains| !label_contains.is_empty());
        let from_ref = from_ref.as_deref().map(parse_object_ref_from_ref).transpose()?;
        let to_ref = to_ref.as_deref().map(parse_object_ref_to_ref).transpose()?;
        let involves_ref = involves_ref
            .as_deref()
            .map(|value| {
                ObjectRef::parse(value).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid involves_ref: {err}"),
                        Some(serde_json::json!({ "involves_ref": value })),
                    )
                })
            })
            .transpose()?;
        let limit = limit.map(|limit| limit.min(usize::MAX as u64) as usize);

        let state = self.lock_state_synced().await?;

        let mut xrefs = state
            .session
            .xrefs()
            .iter()
            .filter_map(|(xref_id, xref)| {
                if dangling_only && !xref.status().is_dangling() {
                    return None;
                }

                if let Some(filter) = status_filter {
                    match filter {
                        StatusFilter::Exact(status) if xref.status().as_str() != status => {
                            return None;
                        }
                        StatusFilter::AnyDangling if !xref.status().is_dangling() => {
                            return None;
                        }
                        _ => {}
                    }
                }

                if kind.as_deref().is_some_and(|kind| xref.kind() != kind) {
                    return None;
                }

                if from_ref.as_ref().is_some_and(|from_ref| xref.from() != from_ref) {
                    return None;
                }
                if to_ref.as_ref().is_some_and(|to_ref| xref.to() != to_ref) {
                    return None;
                }
                if involves_ref.as_ref().is_some_and(|involves_ref| {
                    xref.from() != involves_ref && xref.to() != involves_ref
                }) {
                    return None;
                }

                if label_contains.as_deref().is_some_and(|needle| match xref.label() {
                    Some(label) => !label.contains(needle),
                    None => true,
                }) {
                    return None;
                }

                Some(XRefSummary {
                    xref_id: xref_id.as_str().to_owned(),
                    from: xref.from().to_string(),
                    to: xref.to().to_string(),
                    kind: xref.kind().to_owned(),
                    label: xref.label().map(|label| label.to_owned()),
                    status: xref.status().as_str().to_owned(),
                })
            })
            .collect::<Vec<_>>();
        xrefs.sort_by(|a, b| a.xref_id.cmp(&b.xref_id));

        if let Some(limit) = limit {
            if limit == 0 {
                xrefs.clear();
            } else if xrefs.len() > limit {
                xrefs.truncate(limit);
            }
        }

        Ok(Json(XRefListResponse { xrefs }))
    }

    /// List xref-neighbor objects connected to an `object_ref`; useful probe step after
    /// `attention_human_read` or `route_find`.
    #[tool(name = "xref_neighbors")]
    pub(super) async fn xref_neighbors(
        &self,
        params: Parameters<XRefNeighborsParams>,
    ) -> Result<Json<XRefNeighborsResponse>, ErrorData> {
        let XRefNeighborsParams { object_ref, direction } = params.0;

        let object_ref_parsed = parse_object_ref(&object_ref)?;
        let direction = direction.as_deref().unwrap_or("both");
        let (want_out, want_in) = match direction {
            "out" => (true, false),
            "in" => (false, true),
            "both" => (true, true),
            other => {
                return Err(ErrorData::invalid_params(
                    "invalid direction (expected out|in|both)",
                    Some(serde_json::json!({ "direction": other })),
                ));
            }
        };

        let state = self.lock_state_synced().await?;
        let mut neighbors = BTreeSet::new();
        for xref in state.session.xrefs().values() {
            if want_out && xref.from() == &object_ref_parsed {
                neighbors.insert(xref.to().to_string());
            }
            if want_in && xref.to() == &object_ref_parsed {
                neighbors.insert(xref.from().to_string());
            }
        }

        Ok(Json(XRefNeighborsResponse { neighbors: neighbors.into_iter().collect() }))
    }

    /// Add a cross-diagram xref; use to persist discovered relationships from route/trace analysis
    /// and walkthrough work.
    #[tool(name = "xref_add")]
    pub(super) async fn xref_add(
        &self,
        params: Parameters<XRefAddParams>,
    ) -> Result<Json<XRefAddResponse>, ErrorData> {
        let XRefAddParams { xref_id, from, to, kind, label } = params.0;

        let xref_id_parsed = parse_xref_id(&xref_id)?;
        let from = parse_object_ref_from(&from)?;
        let to = parse_object_ref_to(&to)?;

        let mut state = self.lock_state_synced().await?;
        if let Some(session_folder) = &self.session_folder {
            let (candidate, status) = {
                let mut update = session_folder.begin_session_update().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to reload session before save: {err}"),
                        Some(serde_json::json!({ "xref_id": xref_id })),
                    )
                })?;
                let candidate = update.session_mut();
                if candidate.xrefs().contains_key(&xref_id_parsed) {
                    return Err(ErrorData::invalid_params(
                        "xref_id already exists",
                        Some(serde_json::json!({ "xref_id": xref_id })),
                    ));
                }

                let from_missing = object_ref_is_missing(candidate, &from);
                let to_missing = object_ref_is_missing(candidate, &to);
                let status = XRefStatus::from_flags(from_missing, to_missing);

                let mut xref = XRef::new(from, to, kind, status);
                xref.set_label(label);
                candidate.xrefs_mut().insert(xref_id_parsed.clone(), xref);

                let candidate = update.commit().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to persist session: {err}"),
                        Some(serde_json::json!({ "xref_id": xref_id })),
                    )
                })?;

                (candidate, status)
            };

            replace_committed_session(&mut state, candidate);
            let response = Json(XRefAddResponse {
                xref_id: xref_id_parsed.as_str().to_owned(),
                status: status.as_str().to_owned(),
            });
            drop(state);
            self.notify_ui_session_changed().await;
            return Ok(response);
        }

        if state.session.xrefs().contains_key(&xref_id_parsed) {
            return Err(ErrorData::invalid_params(
                "xref_id already exists",
                Some(serde_json::json!({ "xref_id": xref_id })),
            ));
        }

        let from_missing = object_ref_is_missing(&state.session, &from);
        let to_missing = object_ref_is_missing(&state.session, &to);
        let status = XRefStatus::from_flags(from_missing, to_missing);

        let mut xref = XRef::new(from, to, kind, status);
        xref.set_label(label);
        state.session.xrefs_mut().insert(xref_id_parsed.clone(), xref);

        let response = Json(XRefAddResponse {
            xref_id: xref_id_parsed.as_str().to_owned(),
            status: status.as_str().to_owned(),
        });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Remove an xref by id; typically follows `xref_list` review or dangling cleanup.
    #[tool(name = "xref_remove")]
    pub(super) async fn xref_remove(
        &self,
        params: Parameters<XRefRemoveParams>,
    ) -> Result<Json<XRefRemoveResponse>, ErrorData> {
        let XRefRemoveParams { xref_id } = params.0;
        let xref_id_parsed = parse_xref_id(&xref_id)?;

        let mut state = self.lock_state_synced().await?;
        if let Some(session_folder) = &self.session_folder {
            let candidate = {
                let mut update = session_folder.begin_session_update().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to reload session before save: {err}"),
                        Some(serde_json::json!({ "xref_id": xref_id })),
                    )
                })?;
                let candidate = update.session_mut();
                let removed = candidate.xrefs_mut().remove(&xref_id_parsed).is_some();
                if !removed {
                    return Err(ErrorData::resource_not_found(
                        "xref not found",
                        Some(serde_json::json!({ "xref_id": xref_id })),
                    ));
                }

                update.commit().map_err(|err| {
                    ErrorData::internal_error(
                        format!("failed to persist session: {err}"),
                        Some(serde_json::json!({ "xref_id": xref_id })),
                    )
                })?
            };
            replace_committed_session(&mut state, candidate);
            let response = Json(XRefRemoveResponse { removed: true });
            drop(state);
            self.notify_ui_session_changed().await;
            return Ok(response);
        }

        let removed = state.session.xrefs_mut().remove(&xref_id_parsed).is_some();
        if !removed {
            return Err(ErrorData::resource_not_found(
                "xref not found",
                Some(serde_json::json!({ "xref_id": xref_id })),
            ));
        }

        let response = Json(XRefRemoveResponse { removed: true });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }
}
