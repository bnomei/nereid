// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! MCP server core: shared session lock, tool routers, and stdio/HTTP entrypoints.
//!
//! Holds `McpState` (session + optional folder + delta history + agent highlights) and
//! composes tool modules. Tool handlers live in sibling files; helpers are include!'d.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool_handler, ErrorData, ServerHandler, ServiceExt};
use tokio::sync::Mutex;

use crate::model::{
    Diagram, DiagramAst, DiagramId, DiagramKind, ObjectId, ObjectRef, Session, SymbolAnchor,
    Walkthrough, WalkthroughEdge, WalkthroughId, WalkthroughNode, WalkthroughNodeId, XRefId,
    XRefStatus,
};
use crate::ops::{
    ApplyError, FlowEdgePatch, FlowNodePatch, FlowOp, Op, SeqBlockPatch, SeqMessagePatch, SeqOp,
    SeqParticipantPatch, SeqSectionPatch,
};
use crate::store::SessionFolder;
use crate::ui::UiState;

use super::types::*;

mod collaboration;
mod diagram;
mod queries;
mod walkthrough;
mod xref;

const DELTA_HISTORY_LIMIT: usize = 64;

/// Ring-buffer entry for diagram apply deltas (`diagram_get_delta`).
#[derive(Debug, Clone)]
struct LastDelta {
    from_rev: u64,
    to_rev: u64,
    delta: crate::ops::Delta,
    diagram_metadata_updated: bool,
}

/// Compact walkthrough change sets for delta history.
#[derive(Debug, Clone, Default)]
struct WalkthroughDelta {
    added: BTreeSet<String>,
    removed: BTreeSet<String>,
    updated: BTreeSet<String>,
}

/// Ring-buffer entry for walkthrough apply deltas.
#[derive(Debug, Clone)]
struct WalkthroughLastDelta {
    from_rev: u64,
    to_rev: u64,
    delta: WalkthroughDelta,
}

/// In-memory session plus bounded delta histories under the MCP mutex.
#[derive(Debug)]
struct McpState {
    session: Session,
    delta_history: BTreeMap<DiagramId, VecDeque<LastDelta>>,
    walkthrough_delta_history: BTreeMap<WalkthroughId, VecDeque<WalkthroughLastDelta>>,
}

/// MCP server owning session state, optional folder persistence, and tool routing.
///
/// Shares agent highlights and UI state with the TUI when constructed for combined mode.
#[derive(Clone)]
pub struct NereidMcp {
    state: Arc<Mutex<McpState>>,
    session_folder: Option<Arc<SessionFolder>>,
    agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ui_state: Option<Arc<Mutex<UiState>>>,
    tool_router: ToolRouter<Self>,
}

impl NereidMcp {
    fn tool_router() -> ToolRouter<Self> {
        Self::diagram_tool_router()
            + Self::walkthrough_tool_router()
            + Self::collaboration_tool_router()
            + Self::xref_tool_router()
            + Self::queries_tool_router()
    }

    /// In-memory MCP server without filesystem persistence.
    pub fn new(session: Session) -> Self {
        Self::new_with_agent_highlights(session, Arc::new(Mutex::new(BTreeSet::new())))
    }

    /// In-memory MCP with a shared agent highlight set (TUI co-host without folder).
    pub fn new_with_agent_highlights(
        session: Session,
        agent_highlights: Arc<Mutex<BTreeSet<ObjectRef>>>,
    ) -> Self {
        Self::new_with_agent_highlights_and_ui_state(session, agent_highlights, None)
    }

    /// In-memory MCP with agent highlight and optional shared UiState.
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

    /// MCP server that persists session mutations through a session folder.
    pub fn new_persistent(session: Session, session_folder: SessionFolder) -> Self {
        Self::new_persistent_with_agent_highlights(
            session,
            session_folder,
            Arc::new(Mutex::new(BTreeSet::new())),
        )
    }

    /// Persistent MCP with shared agent highlight (stdio or HTTP without TUI UiState).
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

    /// Full co-host: session folder, agent highlight, and live TUI UiState.
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

    /// Stable JSON snapshot of registered MCP tool schemas for CLI introspection.
    pub fn tool_schema_snapshot() -> Result<String, serde_json::Error> {
        let tools = Self::tool_router().list_all();
        let mut snapshot = serde_json::to_string_pretty(&tools)?;
        snapshot.push('\n');
        Ok(snapshot)
    }

    /// Serve MCP over stdio until the transport closes.
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

    async fn prune_missing_agent_highlights(&self, session: &Session) {
        let mut agent_highlights = self.agent_highlights.lock().await;
        agent_highlights.retain(|object_ref| !object_ref_is_missing(session, object_ref));
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

        replace_committed_session(state, disk_session);

        Ok(())
    }
}

fn replace_committed_session(state: &mut McpState, session: Session) {
    let previous = std::mem::replace(&mut state.session, session);
    reconcile_delta_history_after_session_swap(state, &previous);
}

fn reconcile_delta_history_after_session_swap(state: &mut McpState, previous: &Session) {
    state.delta_history.retain(|diagram_id, _| {
        previous.diagrams().get(diagram_id).map(|diagram| diagram.rev())
            == state.session.diagrams().get(diagram_id).map(|diagram| diagram.rev())
    });
    state.walkthrough_delta_history.retain(|walkthrough_id, _| {
        previous.walkthroughs().get(walkthrough_id).map(|walkthrough| walkthrough.rev())
            == state.session.walkthroughs().get(walkthrough_id).map(|walkthrough| walkthrough.rev())
    });
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for NereidMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Nereid diagram collaboration server (tools: diagram_list, diagram_open, diagram_delete, diagram_current, diagram_read, diagram_stat, diagram_diff, diagram_render_text, diagram_get_ast, diagram_get_slice, diagram_create_from_mermaid, diagram_replace_from_mermaid, diagram_apply_ops, diagram_propose_ops, walkthrough_list, walkthrough_open, walkthrough_current, walkthrough_read, walkthrough_stat, walkthrough_diff, walkthrough_render_text, walkthrough_get_node, walkthrough_apply_ops, route_find, attention_human_read, attention_agent_read, attention_agent_set, attention_agent_clear, follow_ai_read, follow_ai_set, selection_read, selection_update, view_read_state, object_read, xref_list, xref_neighbors, xref_add, xref_remove, seq_messages, seq_trace, seq_search, flow_reachable, flow_unreachable, flow_paths, flow_cycles, flow_dead_ends, flow_degrees)",
        )
    }
}

// include!: helpers.rs
include!("server/helpers.rs");

#[cfg(test)]
mod e2e;

#[cfg(test)]
mod tests;
