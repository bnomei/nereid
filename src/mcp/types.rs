// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! MCP tool JSON types and schemars schemas.
//!
//! Agent-facing contract (underscore tool names, snake_case op tags). Lifecycle groups:
//! diagram tools, walkthrough, collab (attention/selection/follow-AI), and queries.
//! Keep in sync with `tool_schema.snapshot.json` after field changes.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Lightweight diagram listing row for MCP list/current responses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramSummary {
    pub diagram_id: String,
    pub name: String,
    pub kind: String,
    pub rev: u64,
}

/// `diagram_list` payload with co-presence context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDiagramsResponse {
    pub diagrams: Vec<DiagramSummary>,
    pub context: ReadContext,
}

/// Lightweight walkthrough row for list responses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughSummary {
    pub walkthrough_id: String,
    pub title: String,
    pub rev: u64,
    pub nodes: u64,
    pub edges: u64,
}

/// `walkthrough_list` payload with co-presence context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListWalkthroughsResponse {
    pub walkthroughs: Vec<WalkthroughSummary>,
    pub context: ReadContext,
}

/// Params for `walkthrough_get`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughGetParams {
    pub walkthrough_id: String,
}

/// Params for `walkthrough_get_node`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughGetNodeParams {
    pub walkthrough_id: String,
    pub node_id: String,
}

/// Wire form of a walkthrough node (body, object refs, tags, status).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpWalkthroughNode {
    pub node_id: String,
    pub title: String,
    pub body_md: Option<String>,
    pub refs: Vec<String>,
    pub tags: Vec<String>,
    pub status: Option<String>,
}

/// Single-node walkthrough read response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughGetNodeResponse {
    pub node: McpWalkthroughNode,
    pub context: ReadContext,
}

/// Wire form of a directed walkthrough edge.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpWalkthroughEdge {
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: String,
    pub label: Option<String>,
}

/// Full walkthrough payload for get/apply responses.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpWalkthrough {
    pub walkthrough_id: String,
    pub title: String,
    pub rev: u64,
    pub nodes: Vec<McpWalkthroughNode>,
    pub edges: Vec<McpWalkthroughEdge>,
}

/// `walkthrough_get` payload with co-presence context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughGetResponse {
    pub walkthrough: McpWalkthrough,
    pub context: ReadContext,
}

/// Node/edge counts inside a walkthrough digest.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughDigestCounts {
    pub nodes: u64,
    pub edges: u64,
}

/// Cheap walkthrough fingerprint (rev + counts) without full body.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughDigest {
    pub rev: u64,
    pub counts: WalkthroughDigestCounts,
}

/// `walkthrough_get_digest` response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughGetDigestResponse {
    pub digest: WalkthroughDigest,
    pub context: ReadContext,
}

/// Unicode walkthrough preview from `walkthrough_render_text`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughRenderTextResponse {
    pub text: String,
    pub context: ReadContext,
}

/// Unicode diagram preview from `diagram_render_text`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramRenderTextResponse {
    pub text: String,
    pub context: ReadContext,
}

/// Params for cross-diagram `route_find`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RouteFindParams {
    pub from_ref: String,
    pub to_ref: String,
    pub limit: Option<u64>,
    pub max_hops: Option<u64>,
    pub ordering: Option<String>,
}

/// Ordered paths of object refs between two endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RouteFindResponse {
    pub routes: Vec<Vec<String>>,
}

/// Params for `diagram_open` (set session active diagram).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramOpenParams {
    pub diagram_id: String,
}

/// Confirms the new session-active diagram id.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramOpenResponse {
    pub active_diagram_id: String,
}

/// Params for `diagram_delete`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramDeleteParams {
    pub diagram_id: String,
}

/// Delete result including any new active diagram after removal.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramDeleteResponse {
    pub deleted_diagram_id: String,
    pub active_diagram_id: Option<String>,
}

/// Session-active diagram id plus co-presence context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramCurrentResponse {
    pub active_diagram_id: Option<String>,
    pub context: ReadContext,
}

/// Params for `walkthrough_open`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughOpenParams {
    pub walkthrough_id: String,
}

/// Confirms the new session-active walkthrough id.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughOpenResponse {
    pub active_walkthrough_id: String,
}

/// Session-active walkthrough id plus co-presence context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughCurrentResponse {
    pub active_walkthrough_id: Option<String>,
    pub context: ReadContext,
}

/// Human or agent attention spotlight (object + diagram) for collab tools.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttentionReadResponse {
    pub object_ref: Option<String>,
    pub diagram_id: Option<String>,
    pub context: ReadContext,
}

/// Params for `attention_agent_set` (agent highlight spotlight).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AttentionAgentSetParams {
    pub object_ref: String,
}

/// Confirms agent attention after set.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttentionSetResponse {
    pub object_ref: String,
    pub diagram_id: String,
}

/// Count of cleared agent attention entries.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttentionClearResponse {
    pub cleared: u64,
}

/// Whether TUI follow-AI camera is enabled.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FollowAiReadResponse {
    pub enabled: bool,
    pub context: ReadContext,
}

/// Params for `follow_ai_set`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FollowAiSetParams {
    pub enabled: bool,
}

/// Confirms follow-AI after set.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FollowAiSetResponse {
    pub enabled: bool,
}

/// How `selection_update` merges `object_refs` into the multi-select set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMode {
    #[default]
    Replace,
    Add,
    Remove,
}

/// Current multi-select object refs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectionGetResponse {
    pub object_refs: Vec<String>,
    pub context: ReadContext,
}

/// Params for `selection_update` (replace/add/remove modes).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SelectionUpdateParams {
    pub object_refs: Vec<String>,
    #[serde(default)]
    pub mode: UpdateMode,
}

/// Applied vs ignored refs from a selection update.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectionUpdateResponse {
    pub applied: Vec<String>,
    pub ignored: Vec<String>,
}

/// Diagram canvas scroll offsets for `view_get_state`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ViewScroll {
    pub x: f64,
    pub y: f64,
}

/// Coarse TUI view snapshot (active diagram, scroll, pane visibility).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ViewGetStateResponse {
    pub active_diagram_id: Option<String>,
    pub scroll: ViewScroll,
    pub panes: BTreeMap<String, bool>,
    pub context: ReadContext,
}

/// Per-kind object counts for digests (unused kinds stay 0).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramCounts {
    pub participants: u64,
    pub messages: u64,
    pub nodes: u64,
    pub edges: u64,
    pub classes: u64,
    pub relations: u64,
    pub entities: u64,
    pub relationships: u64,
    pub sections: u64,
    pub tasks: u64,
    pub dependencies: u64,
    pub lanes: u64,
}

/// Cheap diagram fingerprint (`diagram_get_digest`) before full AST/Mermaid reads.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramDigest {
    pub rev: u64,
    pub counts: DiagramCounts,
    pub key_names: Vec<String>,
    pub context: ReadContext,
}

/// Full Mermaid projection + rev for `diagram_read` (fail-closed export).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramSnapshot {
    pub rev: u64,
    pub kind: String,
    pub mermaid: String,
    pub context: ReadContext,
}

/// Optional co-presence fields attached to many read responses (TUI/MCP shared state).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ReadContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_active_diagram_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub human_active_diagram_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub human_active_object_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_ai: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_rev: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_session_rev: Option<u64>,
}

/// Create a diagram from raw Mermaid; optional id/name and whether it becomes session-active.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramCreateFromMermaidParams {
    /// Raw Mermaid diagram source (`flowchart`/`graph` or `sequenceDiagram`).
    pub mermaid: String,
    /// Optional explicit diagram id to use; when omitted a unique id is allocated.
    pub diagram_id: Option<String>,
    /// Optional display name; defaults to the chosen diagram id.
    pub name: Option<String>,
    /// When true (default), sets the created diagram as active.
    pub make_active: Option<bool>,
}

/// Create-from-Mermaid result with optional new session-active diagram.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramCreateFromMermaidResponse {
    pub diagram: DiagramSummary,
    pub active_diagram_id: Option<String>,
}

/// Revision-gated Mermaid whole-diagram replace (`base_rev` must match).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramReplaceFromMermaidParams {
    pub diagram_id: Option<String>,
    pub base_rev: u64,
    /// Replacement Mermaid source; kind must match the existing diagram.
    pub mermaid: String,
}

/// Stable-id reconciliation report after Mermaid replace.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramIdentityReport {
    /// Object ids present after replace but not before (new allocations).
    pub newly_allocated: Vec<String>,
    /// Object ids present before replace but not after (dropped/unmatched).
    pub dropped: Vec<String>,
    /// Object ids present both before and after replace.
    pub preserved: Vec<String>,
}

/// Replace result: new rev, identity report, and dangling xrefs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramReplaceFromMermaidResponse {
    pub new_rev: u64,
    pub diagram_id: String,
    pub kind: String,
    pub identity: DiagramIdentityReport,
    /// XRef ids that are dangling after the replace (from, to, or both).
    pub dangling_xref_ids: Vec<String>,
}

/// Typed AST snapshot from `diagram_get_ast`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramGetAstResponse {
    pub diagram_id: String,
    pub kind: String,
    pub rev: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_symbol_repository_id: Option<String>,
    pub ast: McpDiagramAst,
}

/// Frigg symbol anchor on sequence participants / flow nodes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSymbolAnchor {
    pub stable_symbol_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_id: Option<String>,
}

/// Kind-tagged diagram AST for MCP (structure ops and get_ast).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpDiagramAst {
    Sequence {
        participants: Vec<McpSeqParticipantAst>,
        messages: Vec<McpSeqMessageAst>,
        blocks: Vec<McpSeqBlockAst>,
    },
    Flowchart {
        nodes: Vec<McpFlowNodeAst>,
        edges: Vec<McpFlowEdgeAst>,
    },
    Class {
        classes: Vec<McpClassNodeAst>,
        relations: Vec<McpClassRelationAst>,
    },
    Er {
        entities: Vec<McpErEntityAst>,
        relationships: Vec<McpErRelationshipAst>,
    },
    Gantt {
        title: Option<String>,
        date_format: Option<String>,
        sections: Vec<McpGanttSectionAst>,
        tasks: Vec<McpGanttTaskAst>,
        lanes: Vec<McpGanttLaneAst>,
    },
}

/// Class node in MCP AST / object reads.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpClassNodeAst {
    pub class_id: String,
    pub name: String,
    pub attributes: Vec<String>,
    pub methods: Vec<String>,
    pub note: Option<String>,
}

/// UML-ish class relation kind on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpClassRelationKind {
    Inheritance,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Realization,
    Link,
}

/// Class relation edge in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpClassRelationAst {
    pub relation_id: String,
    pub from_class_id: String,
    pub to_class_id: String,
    pub kind: McpClassRelationKind,
    pub label: Option<String>,
    pub raw_connector: Option<String>,
}

/// ER entity node in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpErEntityAst {
    pub entity_id: String,
    pub name: String,
    pub note: Option<String>,
}

/// ER endpoint cardinality on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpErCardinality {
    ExactlyOne,
    ZeroOrOne,
    OneOrMore,
    ZeroOrMore,
}

/// Identifying vs non-identifying ER relationship stroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpErStroke {
    Identifying,
    NonIdentifying,
}

/// ER relationship edge in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpErRelationshipAst {
    pub relationship_id: String,
    pub from_entity_id: String,
    pub to_entity_id: String,
    pub from_cardinality: McpErCardinality,
    pub to_cardinality: McpErCardinality,
    pub stroke: McpErStroke,
    pub label: Option<String>,
    pub raw_connector: Option<String>,
}

/// Gantt section grouping task ids.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpGanttSectionAst {
    pub section_id: String,
    pub name: String,
    pub task_ids: Vec<String>,
}

/// Gantt task start: absolute date, after another task, or unspecified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpGanttTaskStart {
    Date { date: String },
    After { task_id: String },
    Unspecified,
}

/// Gantt task row in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpGanttTaskAst {
    pub task_id: String,
    pub mermaid_tag: Option<String>,
    pub name: String,
    pub start: McpGanttTaskStart,
    pub duration_days: u32,
    pub raw_duration: String,
    pub note: Option<String>,
}

/// Gantt time-lane header in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpGanttLaneAst {
    pub lane_id: String,
    pub label: String,
    pub note: Option<String>,
}

/// Sequence participant in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSeqParticipantAst {
    pub participant_id: String,
    pub mermaid_name: String,
    pub role: Option<String>,
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<McpSymbolAnchor>,
}

/// Sequence message in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSeqMessageAst {
    pub message_id: String,
    pub from_participant_id: String,
    pub to_participant_id: String,
    pub kind: MessageKind,
    pub arrow: Option<String>,
    pub text: String,
    pub order_key: i64,
}

/// Nested alt/opt/loop/par block with sections and child blocks.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSeqBlockAst {
    pub block_id: String,
    pub kind: McpSeqBlockKind,
    pub header: Option<String>,
    pub sections: Vec<McpSeqSectionAst>,
    pub blocks: Vec<McpSeqBlockAst>,
}

/// Sequence fragment block kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpSeqBlockKind {
    Alt,
    Opt,
    Loop,
    Par,
}

/// One section inside a sequence block (main/else/and) owning message ids.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSeqSectionAst {
    pub section_id: String,
    pub kind: McpSeqSectionKind,
    pub header: Option<String>,
    pub message_ids: Vec<String>,
}

/// Section role within a sequence block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpSeqSectionKind {
    Main,
    Else,
    And,
}

/// Write-only section kinds for `seq_add_section` (main is created by `seq_add_block`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpSeqSectionAddKind {
    Else,
    And,
}

/// Flowchart node in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpFlowNodeAst {
    pub node_id: String,
    pub label: String,
    pub shape: String,
    pub mermaid_id: Option<String>,
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<McpSymbolAnchor>,
}

/// Flowchart edge in MCP AST.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpFlowEdgeAst {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub label: Option<String>,
    pub connector: Option<String>,
    pub style: Option<String>,
}

/// Kind of object-ref change inside a diagram delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeltaChangeKind {
    Added,
    Removed,
    Updated,
}

/// One grouped change of object refs between revs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeltaChange {
    pub kind: DeltaChangeKind,
    pub refs: Vec<String>,
}

/// `diagram_get_delta` response spanning `from_rev`→`to_rev`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramDeltaResponse {
    pub from_rev: u64,
    pub to_rev: u64,
    pub changes: Vec<DeltaChange>,
}

/// Compact added/removed/updated object refs after apply.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeltaSummary {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<String>,
}

/// Result of `diagram_apply_ops` / walkthrough apply (new rev + delta).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyOpsResponse {
    pub new_rev: u64,
    pub applied: u64,
    pub delta: DeltaSummary,
}

/// Optional diagram id for tools defaulting to session-active diagram.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramTargetParams {
    pub diagram_id: Option<String>,
}

/// Neighborhood slice around a center object ref.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramGetSliceParams {
    pub diagram_id: Option<String>,
    pub center_ref: String,
    pub radius: Option<u64>,
    pub depth: Option<u64>,
    pub filters: Option<DiagramSliceFilters>,
}

/// Category include/exclude filters for diagram slices.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramSliceFilters {
    pub include_categories: Option<Vec<String>>,
    pub exclude_categories: Option<Vec<String>>,
}

/// Object and edge refs in a `diagram_get_slice` neighborhood.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramGetSliceResponse {
    pub objects: Vec<String>,
    pub edges: Vec<String>,
}

/// Params for `diagram_get_delta` since a known rev.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetDeltaParams {
    pub diagram_id: Option<String>,
    pub since_rev: u64,
}

/// Params for `walkthrough_get_delta`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughGetDeltaParams {
    pub walkthrough_id: String,
    pub since_rev: u64,
}

/// Revision-gated diagram structure ops (`base_rev` OCC).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ApplyOpsParams {
    pub diagram_id: Option<String>,
    pub base_rev: u64,
    pub ops: Vec<McpOp>,
}

/// Revision-gated walkthrough structure ops.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughApplyOpsParams {
    pub walkthrough_id: String,
    pub base_rev: u64,
    pub ops: Vec<McpWalkthroughOp>,
}

/// Dry-run apply params for `diagram_propose_ops` (same shape as apply).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramProposeOpsParams {
    pub diagram_id: Option<String>,
    pub base_rev: u64,
    pub ops: Vec<McpOp>,
}

/// Proposed-apply preview without persisting (same fields as apply response).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramProposeOpsResponse {
    pub new_rev: u64,
    pub applied: u64,
    pub delta: DeltaSummary,
}

/// Walkthrough delta since a known rev.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughDeltaResponse {
    pub from_rev: u64,
    pub to_rev: u64,
    pub changes: Vec<DeltaChange>,
}

/// Filters for `xref_list` (status, endpoints, dangling).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct XRefListParams {
    pub dangling_only: Option<bool>,
    pub status: Option<String>,
    pub kind: Option<String>,
    pub from_ref: Option<String>,
    pub to_ref: Option<String>,
    pub involves_ref: Option<String>,
    pub label_contains: Option<String>,
    pub limit: Option<u64>,
}

/// One cross-reference row for list tools.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefSummary {
    pub xref_id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
    pub status: String,
}

/// `xref_list` payload.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefListResponse {
    pub xrefs: Vec<XRefSummary>,
}

/// Params for one-hop xref neighborhood traversal.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct XRefNeighborsParams {
    pub object_ref: String,
    pub direction: Option<String>,
}

/// Neighbor object refs via xrefs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefNeighborsResponse {
    pub neighbors: Vec<String>,
}

/// Params for `xref_add`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct XRefAddParams {
    pub xref_id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
}

/// Created xref id and resolved status.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefAddResponse {
    pub xref_id: String,
    pub status: String,
}

/// Params for `xref_remove`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct XRefRemoveParams {
    pub xref_id: String,
}

/// Whether the xref existed and was removed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefRemoveResponse {
    pub removed: bool,
}

/// Params for sequence message trace (forward/back along order).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SeqTraceParams {
    pub diagram_id: Option<String>,
    pub from_message_id: Option<String>,
    pub direction: Option<String>,
    pub limit: Option<u64>,
}

/// Ordered message ids from a sequence trace.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SeqTraceResponse {
    pub messages: Vec<String>,
}

/// Params for sequence message text search.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SeqSearchParams {
    pub diagram_id: Option<String>,
    pub needle: String,
    pub mode: Option<String>,
    pub case_insensitive: Option<bool>,
}

/// Message ids matching a sequence search.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SeqSearchResponse {
    pub messages: Vec<String>,
}

/// Filter messages by from/to participant.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SeqMessagesParams {
    pub diagram_id: Option<String>,
    pub from_participant_id: Option<String>,
    pub to_participant_id: Option<String>,
}

/// Message ids between optional participant endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SeqMessagesResponse {
    pub messages: Vec<String>,
}

/// Params for flowchart reachability from a start node.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowReachableParams {
    pub diagram_id: Option<String>,
    pub from_node_id: String,
    pub direction: Option<String>,
}

/// Reachable flow node ids.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowReachableResponse {
    pub nodes: Vec<String>,
}

/// Params for nodes not reachable from a start (or graph roots).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowUnreachableParams {
    pub diagram_id: Option<String>,
    pub start_node_id: Option<String>,
}

/// Unreachable flow node ids.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowUnreachableResponse {
    pub nodes: Vec<String>,
}

/// Params for simple paths between two flow nodes.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowPathsParams {
    pub diagram_id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub limit: Option<u64>,
    pub max_extra_hops: Option<u64>,
}

/// Node-id paths between flow endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowPathsResponse {
    pub paths: Vec<Vec<String>>,
}

/// Detected directed cycles as node-id rings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowCyclesResponse {
    pub cycles: Vec<Vec<String>>,
}

/// Flow nodes with no outgoing edges.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowDeadEndsResponse {
    pub nodes: Vec<String>,
}

/// Params for flow degree ranking.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowDegreesParams {
    pub diagram_id: Option<String>,
    pub top: Option<u64>,
    pub sort_by: Option<String>,
}

/// One flow node's in/out degree ranking row.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowDegreeNode {
    pub node_ref: String,
    pub label: String,
    pub in_degree: u64,
    pub out_degree: u64,
}

/// Ranked flow nodes by degree.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowDegreesResponse {
    pub nodes: Vec<FlowDegreeNode>,
}

/// Sequence message arrow kind (sync/async/return).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Sync,
    Async,
    Return,
}

/// Params for `object_read` (single ref and/or batch).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ObjectGetParams {
    pub object_ref: Option<String>,
    pub object_refs: Option<Vec<String>>,
}

/// One resolved object for `object_read`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObjectGetItem {
    pub object_ref: String,
    pub object: McpObject,
}

/// Batch object payload with co-presence context.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObjectGetResponse {
    pub objects: Vec<ObjectGetItem>,
    pub context: ReadContext,
}

/// Kind-tagged object body for `object_read` (mirrors domain categories).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpObject {
    SeqParticipant {
        mermaid_name: String,
        role: Option<String>,
        note: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        symbol: Option<McpSymbolAnchor>,
    },
    SeqBlock {
        kind: McpSeqBlockKind,
        header: Option<String>,
        section_ids: Vec<String>,
        child_block_ids: Vec<String>,
    },
    SeqSection {
        kind: McpSeqSectionKind,
        header: Option<String>,
        message_ids: Vec<String>,
    },
    SeqMessage {
        from_participant_id: String,
        to_participant_id: String,
        kind: MessageKind,
        arrow: Option<String>,
        text: String,
        order_key: i64,
    },
    FlowNode {
        label: String,
        shape: String,
        mermaid_id: Option<String>,
        note: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        symbol: Option<McpSymbolAnchor>,
    },
    FlowEdge {
        from_node_id: String,
        to_node_id: String,
        label: Option<String>,
        connector: Option<String>,
        style: Option<String>,
    },
    ClassNode {
        name: String,
        attributes: Vec<String>,
        methods: Vec<String>,
        note: Option<String>,
    },
    ClassRelation {
        from_class_id: String,
        to_class_id: String,
        kind: McpClassRelationKind,
        label: Option<String>,
        raw_connector: Option<String>,
    },
    ErEntity {
        name: String,
        note: Option<String>,
    },
    ErRelationship {
        from_entity_id: String,
        to_entity_id: String,
        from_cardinality: McpErCardinality,
        to_cardinality: McpErCardinality,
        stroke: McpErStroke,
        label: Option<String>,
        raw_connector: Option<String>,
    },
    GanttSection {
        name: String,
        task_ids: Vec<String>,
    },
    GanttTask {
        mermaid_tag: Option<String>,
        name: String,
        start: McpGanttTaskStart,
        duration_days: u32,
        raw_duration: String,
        note: Option<String>,
    },
    GanttLane {
        label: String,
        note: Option<String>,
    },
}

/// Tagged diagram mutation op for `diagram_apply_ops` / `diagram_propose_ops`.
/// Covers sequence (participants/messages/blocks) and flowchart (nodes/edges) structure.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpOp {
    SeqAddParticipant {
        participant_id: String,
        mermaid_name: String,
    },
    SeqUpdateParticipant {
        participant_id: String,
        mermaid_name: Option<String>,
    },
    SeqSetParticipantNote {
        participant_id: String,
        note: Option<String>,
    },
    SeqSetParticipantSymbol {
        participant_id: String,
        symbol: Option<McpSymbolAnchor>,
    },
    SeqRemoveParticipant {
        participant_id: String,
    },
    SeqAddMessage {
        message_id: String,
        from_participant_id: String,
        to_participant_id: String,
        kind: MessageKind,
        arrow: Option<String>,
        text: String,
        order_key: i64,
        #[serde(default)]
        section_id: Option<String>,
    },
    SeqUpdateMessage {
        message_id: String,
        from_participant_id: Option<String>,
        to_participant_id: Option<String>,
        kind: Option<MessageKind>,
        arrow: Option<String>,
        text: Option<String>,
        order_key: Option<i64>,
    },
    SeqRemoveMessage {
        message_id: String,
    },
    SeqSetMessageSection {
        message_id: String,
        /// Omit or null to detach from all sections.
        #[serde(default)]
        section_id: Option<String>,
    },
    SeqAddBlock {
        block_id: String,
        kind: McpSeqBlockKind,
        #[serde(default)]
        header: Option<String>,
        /// Omit for root blocks.
        #[serde(default)]
        parent_block_id: Option<String>,
        main_section_id: String,
    },
    SeqUpdateBlock {
        block_id: String,
        /// When present, replaces the block header (`null` clears it).
        #[serde(default)]
        header: Option<Option<String>>,
    },
    SeqRemoveBlock {
        block_id: String,
    },
    SeqAddSection {
        section_id: String,
        block_id: String,
        kind: McpSeqSectionAddKind,
        #[serde(default)]
        header: Option<String>,
    },
    SeqUpdateSection {
        section_id: String,
        /// When present, replaces the section header (`null` clears it).
        #[serde(default)]
        header: Option<Option<String>>,
    },
    SeqRemoveSection {
        section_id: String,
    },
    FlowAddNode {
        node_id: String,
        label: String,
        shape: Option<String>,
    },
    FlowUpdateNode {
        node_id: String,
        label: Option<String>,
        shape: Option<String>,
    },
    FlowSetNodeMermaidId {
        node_id: String,
        mermaid_id: Option<String>,
    },
    FlowSetNodeNote {
        node_id: String,
        note: Option<String>,
    },
    FlowSetNodeSymbol {
        node_id: String,
        symbol: Option<McpSymbolAnchor>,
    },
    FlowRemoveNode {
        node_id: String,
    },
    FlowAddEdge {
        edge_id: String,
        from_node_id: String,
        to_node_id: String,
        label: Option<String>,
        connector: Option<String>,
        style: Option<String>,
    },
    FlowUpdateEdge {
        edge_id: String,
        from_node_id: Option<String>,
        to_node_id: Option<String>,
        label: Option<String>,
        connector: Option<String>,
        style: Option<String>,
    },
    FlowRemoveEdge {
        edge_id: String,
    },
}

/// Tagged walkthrough mutation for `walkthrough_apply_ops` (nodes/edges/title).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpWalkthroughOp {
    SetTitle {
        title: String,
    },
    AddNode {
        node_id: String,
        title: String,
        body_md: Option<String>,
        refs: Option<Vec<String>>,
        tags: Option<Vec<String>>,
        status: Option<String>,
    },
    UpdateNode {
        node_id: String,
        title: Option<String>,
        body_md: Option<Option<String>>,
        refs: Option<Vec<String>>,
        tags: Option<Vec<String>>,
        status: Option<Option<String>>,
    },
    RemoveNode {
        node_id: String,
    },
    AddEdge {
        from_node_id: String,
        to_node_id: String,
        kind: String,
        label: Option<String>,
    },
    UpdateEdge {
        from_node_id: String,
        to_node_id: String,
        kind: String,
        label: Option<Option<String>>,
    },
    RemoveEdge {
        from_node_id: String,
        to_node_id: String,
        kind: String,
    },
}
