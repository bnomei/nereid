// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler, ServiceExt};
use tokio::sync::Mutex;

use crate::format::mermaid::{parse_flowchart, parse_sequence_diagram};
use crate::model::{
    CategoryPath, Diagram, DiagramAst, DiagramId, DiagramKind, ObjectId, ObjectRef, Session,
    Walkthrough, WalkthroughEdge, WalkthroughId, WalkthroughNode, WalkthroughNodeId, XRef, XRefId,
    XRefStatus,
};
use crate::ops::{
    apply_ops, ApplyError, FlowEdgePatch, FlowNodePatch, FlowOp, Op, SeqMessagePatch, SeqOp,
    SeqParticipantPatch,
};
use crate::render::{render_diagram_unicode, render_walkthrough_unicode};
use crate::store::SessionFolder;
use crate::ui::UiState;

use super::types::*;

const DELTA_HISTORY_LIMIT: usize = 64;

#[derive(Debug, Clone)]
struct LastDelta {
    from_rev: u64,
    to_rev: u64,
    delta: crate::ops::Delta,
}

#[derive(Debug, Clone, Default)]
struct WalkthroughDelta {
    added: BTreeSet<String>,
    removed: BTreeSet<String>,
    updated: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct WalkthroughLastDelta {
    from_rev: u64,
    to_rev: u64,
    delta: WalkthroughDelta,
}

#[derive(Debug)]
struct McpState {
    session: Session,
    delta_history: BTreeMap<DiagramId, VecDeque<LastDelta>>,
    walkthrough_delta_history: BTreeMap<WalkthroughId, VecDeque<WalkthroughLastDelta>>,
}

#[derive(Clone)]
pub struct NereidMcp {
    state: Arc<Mutex<McpState>>,
    session_folder: Option<Arc<SessionFolder>>,
    agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ui_state: Option<Arc<Mutex<UiState>>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl NereidMcp {
    pub fn new(session: Session) -> Self {
        Self::new_with_agent_highlights(session, Arc::new(Mutex::new(BTreeSet::new())))
    }

    pub fn new_with_agent_highlights(
        session: Session,
        agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ) -> Self {
        Self::new_with_agent_highlights_and_ui_state(session, agent_highlights, None)
    }

    pub fn new_with_agent_highlights_and_ui_state(
        session: Session,
        agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
        ui_state: Option<Arc<Mutex<UiState>>>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(McpState {
                session,
                delta_history: BTreeMap::new(),
                walkthrough_delta_history: BTreeMap::new(),
            })),
            session_folder: None,
            agent_highlights,
            ui_state,
            tool_router: Self::tool_router(),
        }
    }

    pub fn new_persistent(session: Session, session_folder: SessionFolder) -> Self {
        Self::new_persistent_with_agent_highlights(
            session,
            session_folder,
            Arc::new(Mutex::new(BTreeSet::new())),
        )
    }

    pub fn new_persistent_with_agent_highlights(
        session: Session,
        session_folder: SessionFolder,
        agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ) -> Self {
        Self::new_persistent_with_agent_highlights_and_ui_state(
            session,
            session_folder,
            agent_highlights,
            None,
        )
    }

    pub fn new_persistent_with_agent_highlights_and_ui_state(
        session: Session,
        session_folder: SessionFolder,
        agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
        ui_state: Option<Arc<Mutex<UiState>>>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(McpState {
                session,
                delta_history: BTreeMap::new(),
                walkthrough_delta_history: BTreeMap::new(),
            })),
            session_folder: Some(Arc::new(session_folder)),
            agent_highlights,
            ui_state,
            tool_router: Self::tool_router(),
        }
    }

    pub async fn serve_stdio(self) -> Result<(), rmcp::RmcpError> {
        let service = self.serve((tokio::io::stdin(), tokio::io::stdout())).await?;
        service.waiting().await?;
        Ok(())
    }

    async fn notify_ui_session_changed(&self) {
        if let Some(ui_state) = self.ui_state.as_ref() {
            ui_state.lock().await.bump_session_rev();
        }
    }

    async fn read_context(&self, session_active_diagram_id: Option<String>) -> ReadContext {
        let mut context = ReadContext {
            session_active_diagram_id,
            human_active_diagram_id: None,
            human_active_object_ref: None,
            follow_ai: None,
            ui_rev: None,
            ui_session_rev: None,
        };

        if let Some(ui_state) = self.ui_state.as_ref() {
            let snapshot = ui_state.lock().await.clone();
            context.human_active_diagram_id =
                snapshot.human_active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
            context.human_active_object_ref =
                snapshot.human_active_object_ref().map(ToString::to_string);
            context.follow_ai = Some(snapshot.follow_ai());
            context.ui_rev = Some(snapshot.rev());
            context.ui_session_rev = Some(snapshot.session_rev());
        }

        context
    }

    async fn lock_state_synced(&self) -> Result<tokio::sync::MutexGuard<'_, McpState>, ErrorData> {
        let mut state = self.state.lock().await;
        if let Some(session_folder) = &self.session_folder {
            self.sync_state_with_session_folder(&mut state, session_folder)?;
        }
        Ok(state)
    }

    fn sync_state_with_session_folder(
        &self,
        state: &mut McpState,
        session_folder: &SessionFolder,
    ) -> Result<(), ErrorData> {
        let mut disk_session = session_folder.load_session().map_err(|err| {
            ErrorData::internal_error(format!("failed to load session from disk: {err}"), None)
        })?;
        retain_existing_selected_object_refs(&mut disk_session);
        refresh_xref_statuses(&mut disk_session);

        if disk_session == state.session {
            return Ok(());
        }

        let previous = state.session.clone();
        state.session = disk_session;

        state.delta_history.retain(|diagram_id, _| {
            previous.diagrams().get(diagram_id).map(|diagram| diagram.rev())
                == state.session.diagrams().get(diagram_id).map(|diagram| diagram.rev())
        });
        state.walkthrough_delta_history.retain(|walkthrough_id, _| {
            previous.walkthroughs().get(walkthrough_id).map(|walkthrough| walkthrough.rev())
                == state
                    .session
                    .walkthroughs()
                    .get(walkthrough_id)
                    .map(|walkthrough| walkthrough.rev())
        });

        Ok(())
    }

    /// List diagrams in the current session; start here, then call `diagram.current` or
    /// `diagram.open` (bootstrap with `diagram.create_from_mermaid` if empty).
    #[tool(name = "diagram.list")]
    async fn diagram_list(&self) -> Result<Json<ListDiagramsResponse>, ErrorData> {
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

    /// Create a diagram from raw Mermaid; use to bootstrap a session, then continue with
    /// `diagram.open`/`diagram.stat`.
    #[tool(name = "diagram.create_from_mermaid")]
    async fn diagram_create_from_mermaid(
        &self,
        params: Parameters<DiagramCreateFromMermaidParams>,
    ) -> Result<Json<DiagramCreateFromMermaidResponse>, ErrorData> {
        let DiagramCreateFromMermaidParams { mermaid, diagram_id, name, make_active } = params.0;

        let Some(kind) = detect_mermaid_kind(&mermaid) else {
            return Err(ErrorData::invalid_params(
                "expected 'flowchart'/'graph' or 'sequenceDiagram' as the first non-empty line",
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
        };

        let kind_label = diagram_kind_label(kind).to_owned();
        let make_active = make_active.unwrap_or(true);

        let mut state = self.lock_state_synced().await?;
        let diagram_id = match diagram_id {
            Some(diagram_id) => {
                let parsed = DiagramId::new(diagram_id.clone()).map_err(|err| {
                    ErrorData::invalid_params(
                        format!("invalid diagram_id: {err}"),
                        Some(serde_json::json!({ "diagram_id": diagram_id })),
                    )
                })?;
                if state.session.diagrams().contains_key(&parsed) {
                    return Err(ErrorData::invalid_params(
                        "diagram_id already exists",
                        Some(serde_json::json!({ "diagram_id": parsed.as_str() })),
                    ));
                }
                parsed
            }
            None => allocate_diagram_id(&state.session, kind),
        };

        let name = name.unwrap_or_else(|| diagram_id.as_str().to_owned());
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

        if let Some(session_folder) = &self.session_folder {
            let mut candidate = state.session.clone();
            candidate.diagrams_mut().insert(diagram_id.clone(), diagram);
            if make_active {
                candidate.set_active_diagram_id(Some(diagram_id.clone()));
            }

            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
                )
            })?;
            candidate.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
            session_folder.save_session(&candidate).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id.as_str() })),
                )
            })?;

            state.session = candidate;
        } else {
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

    /// Set the active diagram default for diagram-scoped tools; typically follows `diagram.list`
    /// or `diagram.create_from_mermaid`.
    #[tool(name = "diagram.open")]
    async fn diagram_open(
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
            let mut candidate = state.session.clone();
            candidate.set_active_diagram_id(Some(parsed.clone()));
            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id })),
                )
            })?;
            candidate.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
            session_folder.save_session(&candidate).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id })),
                )
            })?;
            state.session = candidate;
        } else {
            state.session.set_active_diagram_id(Some(parsed.clone()));
        }

        let response = Json(DiagramOpenResponse { active_diagram_id: parsed.as_str().to_owned() });
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Remove a diagram by id and retarget active diagram when needed.
    #[tool(name = "diagram.delete")]
    async fn diagram_delete(
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
            let mut candidate = state.session.clone();
            candidate.diagrams_mut().remove(&parsed);

            if candidate.active_diagram_id().is_some_and(|active| active == &parsed) {
                let next_active = candidate.diagrams().keys().next().cloned();
                candidate.set_active_diagram_id(next_active);
            }

            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id })),
                )
            })?;
            candidate.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
            retain_existing_selected_object_refs(&mut candidate);
            refresh_xref_statuses(&mut candidate);

            session_folder.save_session(&candidate).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id })),
                )
            })?;
            state.session = candidate;
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
    /// `diagram.open`, then continue with `diagram.stat`/`diagram.get_slice`.
    #[tool(name = "diagram.current")]
    async fn diagram_current(&self) -> Result<Json<DiagramCurrentResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(active_diagram_id.clone()).await;

        Ok(Json(DiagramCurrentResponse { active_diagram_id, context }))
    }

    /// Set the active walkthrough default for walkthrough-scoped tools; usually after
    /// `walkthrough.list`.
    #[tool(name = "walkthrough.open")]
    async fn walkthrough_open(
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
            let mut candidate = state.session.clone();
            candidate.set_active_walkthrough_id(Some(parsed.clone()));
            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id })),
                )
            })?;
            candidate.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
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
    async fn walkthrough_current(&self) -> Result<Json<WalkthroughCurrentResponse>, ErrorData> {
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

    /// Read human-owned attention from live TUI state; call early in a turn, then localize with
    /// `diagram.get_slice` and `object.read`.
    #[tool(name = "attention.human.read")]
    async fn attention_human_read(&self) -> Result<Json<AttentionReadResponse>, ErrorData> {
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

    /// Read agent-owned attention (single spotlight); call before `attention.agent.set`/`clear`
    /// to avoid unnecessary spotlight churn.
    #[tool(name = "attention.agent.read")]
    async fn attention_agent_read(&self) -> Result<Json<AttentionReadResponse>, ErrorData> {
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
    #[tool(name = "attention.agent.set")]
    async fn attention_agent_set(
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
    #[tool(name = "attention.agent.clear")]
    async fn attention_agent_clear(&self) -> Result<Json<AttentionClearResponse>, ErrorData> {
        let mut agent_highlights = self.agent_highlights.lock().await;
        let cleared = agent_highlights.len() as u64;
        agent_highlights.clear();

        Ok(Json(AttentionClearResponse { cleared }))
    }

    /// Read follow-AI mode (`true` means TUI tracks agent spotlight); check this before
    /// spotlight-heavy guidance, and pair with `follow_ai.set` when handing off control.
    #[tool(name = "follow_ai.read")]
    async fn follow_ai_read(&self) -> Result<Json<FollowAiReadResponse>, ErrorData> {
        let state = self.lock_state_synced().await?;
        let session_active_diagram_id =
            state.session.active_diagram_id().map(|diagram_id| diagram_id.as_str().to_owned());
        drop(state);
        let context = self.read_context(session_active_diagram_id).await;
        let enabled = context.follow_ai.unwrap_or(true);
        Ok(Json(FollowAiReadResponse { enabled, context }))
    }

    /// Set follow-AI mode (`true` to track agent spotlight in TUI); use with `attention.agent.set`
    /// for guided handoff.
    #[tool(name = "follow_ai.set")]
    async fn follow_ai_set(
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
    /// `attention.human.read` and before `object.read` or `selection.update`.
    #[tool(name = "selection.read")]
    async fn selection_get(&self) -> Result<Json<SelectionGetResponse>, ErrorData> {
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
    #[tool(name = "selection.update")]
    async fn selection_update(
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
            state.session = candidate;
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
    /// `attention.human.read`/`attention.agent.read` for orientation without mutating focus.
    #[tool(name = "view.read_state")]
    async fn view_get_state(&self) -> Result<Json<ViewGetStateResponse>, ErrorData> {
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

    /// List walkthroughs in the current session; start here, then `walkthrough.open`,
    /// `walkthrough.stat`, or `walkthrough.read`.
    #[tool(name = "walkthrough.list")]
    async fn walkthrough_list(&self) -> Result<Json<ListWalkthroughsResponse>, ErrorData> {
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
    async fn walkthrough_read(
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
    async fn walkthrough_get_node(
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
    async fn walkthrough_stat(
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
    async fn walkthrough_render_text(
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
    async fn walkthrough_diff(
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
    async fn walkthrough_apply_ops(
        &self,
        params: Parameters<WalkthroughApplyOpsParams>,
    ) -> Result<Json<ApplyOpsResponse>, ErrorData> {
        let WalkthroughApplyOpsParams { walkthrough_id, base_rev, ops } = params.0;
        let parsed = parse_walkthrough_id(&walkthrough_id)?;

        let mut state = self.lock_state_synced().await?;

        if let Some(session_folder) = &self.session_folder {
            let mut candidate_session = state.session.clone();
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

            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "walkthrough_id": walkthrough_id, "base_rev": base_rev })),
                )
            })?;
            candidate_session
                .set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
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

        let delta = apply_walkthrough_ops(walkthrough, &parsed, &ops)?;
        walkthrough.bump_rev();
        let new_rev = walkthrough.rev();

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

    /// Find cross-diagram routes between two object refs; combine with `xref.neighbors` and
    /// `diagram.get_slice` for explain-and-refine flows.
    #[tool(name = "route.find")]
    async fn route_find(
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

    /// List session xrefs (including dangling filters); use to audit mappings before route/search
    /// exploration or cleanup.
    #[tool(name = "xref.list")]
    async fn xref_list(
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
    /// `attention.human.read` or `route.find`.
    #[tool(name = "xref.neighbors")]
    async fn xref_neighbors(
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
    #[tool(name = "xref.add")]
    async fn xref_add(
        &self,
        params: Parameters<XRefAddParams>,
    ) -> Result<Json<XRefAddResponse>, ErrorData> {
        let XRefAddParams { xref_id, from, to, kind, label } = params.0;

        let xref_id_parsed = parse_xref_id(&xref_id)?;
        let from = parse_object_ref_from(&from)?;
        let to = parse_object_ref_to(&to)?;

        let mut state = self.lock_state_synced().await?;
        if let Some(session_folder) = &self.session_folder {
            let mut candidate = state.session.clone();
            if candidate.xrefs().contains_key(&xref_id_parsed) {
                return Err(ErrorData::invalid_params(
                    "xref_id already exists",
                    Some(serde_json::json!({ "xref_id": xref_id })),
                ));
            }

            let from_missing = object_ref_is_missing(&candidate, &from);
            let to_missing = object_ref_is_missing(&candidate, &to);
            let status = XRefStatus::from_flags(from_missing, to_missing);

            let mut xref = XRef::new(from, to, kind, status);
            xref.set_label(label);
            candidate.xrefs_mut().insert(xref_id_parsed.clone(), xref);
            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "xref_id": xref_id })),
                )
            })?;
            candidate.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());

            session_folder.save_session(&candidate).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "xref_id": xref_id })),
                )
            })?;

            state.session = candidate;
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

    /// Remove an xref by id; typically follows `xref.list` review or dangling cleanup.
    #[tool(name = "xref.remove")]
    async fn xref_remove(
        &self,
        params: Parameters<XRefRemoveParams>,
    ) -> Result<Json<XRefRemoveResponse>, ErrorData> {
        let XRefRemoveParams { xref_id } = params.0;
        let xref_id_parsed = parse_xref_id(&xref_id)?;

        let mut state = self.lock_state_synced().await?;
        if let Some(session_folder) = &self.session_folder {
            let mut candidate = state.session.clone();
            let removed = candidate.xrefs_mut().remove(&xref_id_parsed).is_some();
            if !removed {
                return Err(ErrorData::resource_not_found(
                    "xref not found",
                    Some(serde_json::json!({ "xref_id": xref_id })),
                ));
            }

            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "xref_id": xref_id })),
                )
            })?;
            candidate.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
            session_folder.save_session(&candidate).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "xref_id": xref_id })),
                )
            })?;
            state.session = candidate;
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

    /// Read concrete object fields by ref; use this as evidence before answering.
    #[tool(name = "object.read")]
    async fn object_read(
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

    /// Trace sequence message order before/after a message id (returns refs); use for timeline
    /// explanations and local impact checks.
    #[tool(name = "seq.trace")]
    async fn seq_trace(
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
    async fn seq_search(
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
    async fn seq_messages(
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
                from_participant_id.as_ref().map_or(true, |from| msg.from_participant_id() == from)
            })
            .filter(|msg| {
                to_participant_id.as_ref().map_or(true, |to| msg.to_participant_id() == to)
            })
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
    async fn flow_reachable(
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

        if !ast.nodes().contains_key(&from_node_id_parsed) {
            return Ok(Json(FlowReachableResponse { nodes: Vec::new() }));
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
    async fn flow_paths(
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
    async fn flow_cycles(
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
    async fn flow_dead_ends(
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
    async fn flow_degrees(
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
    async fn flow_unreachable(
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

    /// Get a compact diagram digest (rev + counts + key names); use as the default first read
    /// before `diagram.get_slice`, typed queries, or mutation planning.
    #[tool(name = "diagram.stat")]
    async fn diagram_stat(
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
    #[tool(name = "diagram.read")]
    async fn diagram_read(
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
    #[tool(name = "diagram.get_ast")]
    async fn diagram_get_ast(
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
            ast: mcp_ast_for_diagram(diagram),
        }))
    }

    /// Get deterministic local neighborhood around an `object_ref`; primary probe tool after
    /// attention/selection or search hits.
    #[tool(name = "diagram.get_slice")]
    async fn diagram_get_slice(
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
    /// and review, then return to `diagram.stat`/`diagram.get_slice` for targeted reasoning.
    #[tool(name = "diagram.render_text")]
    async fn diagram_render_text(
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

    /// Get diagram delta since a revision; default refresh step after `diagram.apply_ops` or
    /// external changes.
    #[tool(name = "diagram.diff")]
    async fn diagram_diff(
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

        let Some(delta) = delta_response_from_history(history, since_rev, current_rev) else {
            return Err(delta_unavailable(since_rev, current_rev, supported_since_rev));
        };

        Ok(Json(delta))
    }

    /// Apply structured diagram ops gated by `base_rev`; prefer `diagram.propose_ops` first, then
    /// refresh with `diagram.diff`.
    #[tool(name = "diagram.apply_ops")]
    async fn diagram_apply_ops(
        &self,
        params: Parameters<ApplyOpsParams>,
    ) -> Result<Json<ApplyOpsResponse>, ErrorData> {
        let mut state = self.lock_state_synced().await?;
        let diagram_id = resolve_diagram_id(&state.session, params.0.diagram_id.as_deref())?;
        let diagram = state
            .session
            .diagrams()
            .get(&diagram_id)
            .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

        let ops = params.0.ops.iter().map(mcp_op_to_internal).collect::<Result<Vec<_>, _>>()?;

        let base_rev = params.0.base_rev;
        let current_rev = diagram.rev();
        if base_rev != current_rev {
            let digest = digest_for_diagram(diagram);
            return Err(ErrorData::invalid_request(
                "conflict: stale base_rev",
                Some(serde_json::json!({
                    "base_rev": base_rev,
                    "current_rev": current_rev,
                    "snapshot_tool": "diagram.stat",
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

        if let Some(session_folder) = &self.session_folder {
            let mut candidate_session = state.session.clone();
            let mut candidate_diagram = candidate_session
                .diagrams()
                .get(&diagram_id)
                .cloned()
                .ok_or_else(|| ErrorData::resource_not_found("diagram not found", None))?;

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

            let mut history =
                state.delta_history.get(&diagram_id).cloned().unwrap_or_else(VecDeque::new);
            history.push_back(LastDelta {
                from_rev: base_rev,
                to_rev: result.new_rev,
                delta: result.delta.clone(),
            });
            while history.len() > DELTA_HISTORY_LIMIT {
                history.pop_front();
            }

            let meta = session_folder.load_meta().map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to load session meta: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id.as_str(), "base_rev": base_rev })),
                )
            })?;
            candidate_session
                .set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
            session_folder.save_session(&candidate_session).map_err(|err| {
                ErrorData::internal_error(
                    format!("failed to persist session: {err}"),
                    Some(serde_json::json!({ "diagram_id": diagram_id.as_str(), "base_rev": base_rev })),
                )
            })?;

            state.session = candidate_session;
            state.delta_history.insert(diagram_id, history);

            let response = Json(ApplyOpsResponse {
                new_rev: result.new_rev,
                applied: result.applied as u64,
                delta: DeltaSummary {
                    added: result.delta.added.iter().map(ToString::to_string).collect(),
                    removed: result.delta.removed.iter().map(ToString::to_string).collect(),
                    updated: result.delta.updated.iter().map(ToString::to_string).collect(),
                },
            });
            drop(state);
            self.notify_ui_session_changed().await;
            return Ok(response);
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
        let history = state.delta_history.entry(diagram_id).or_insert_with(VecDeque::new);
        history.push_back(LastDelta {
            from_rev: base_rev,
            to_rev: result.new_rev,
            delta: result.delta.clone(),
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
        drop(state);
        self.notify_ui_session_changed().await;
        Ok(response)
    }

    /// Validate ops against `base_rev` and return predicted delta without mutation; use immediately
    /// before `diagram.apply_ops` for safe human-agent collaboration.
    #[tool(name = "diagram.propose_ops")]
    async fn diagram_propose_ops(
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

#[tool_handler]
impl ServerHandler for NereidMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Nereid diagram collaboration server (tools: diagram.list, diagram.open, diagram.delete, diagram.current, diagram.read, diagram.stat, diagram.diff, diagram.render_text, diagram.get_ast, diagram.get_slice, diagram.create_from_mermaid, diagram.apply_ops, diagram.propose_ops, walkthrough.list, walkthrough.open, walkthrough.current, walkthrough.read, walkthrough.stat, walkthrough.diff, walkthrough.render_text, walkthrough.get_node, walkthrough.apply_ops, route.find, attention.human.read, attention.agent.read, attention.agent.set, attention.agent.clear, follow_ai.read, follow_ai.set, selection.read, selection.update, view.read_state, object.read, xref.list, xref.neighbors, xref.add, xref.remove, seq.messages, seq.trace, seq.search, flow.reachable, flow.unreachable, flow.paths, flow.cycles, flow.dead_ends, flow.degrees)"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

// Extracted mapping/parsing/delta helpers for MCP tool handlers.
include!("server/helpers.rs");

#[cfg(test)]
mod e2e;

#[cfg(test)]
mod tests;
