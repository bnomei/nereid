// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramSummary {
    pub diagram_id: String,
    pub name: String,
    pub kind: String,
    pub rev: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListDiagramsResponse {
    pub diagrams: Vec<DiagramSummary>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughSummary {
    pub walkthrough_id: String,
    pub title: String,
    pub rev: u64,
    pub nodes: u64,
    pub edges: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListWalkthroughsResponse {
    pub walkthroughs: Vec<WalkthroughSummary>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughGetParams {
    pub walkthrough_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughGetNodeParams {
    pub walkthrough_id: String,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpWalkthroughNode {
    pub node_id: String,
    pub title: String,
    pub body_md: Option<String>,
    pub refs: Vec<String>,
    pub tags: Vec<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughGetNodeResponse {
    pub node: McpWalkthroughNode,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpWalkthroughEdge {
    pub from_node_id: String,
    pub to_node_id: String,
    pub kind: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpWalkthrough {
    pub walkthrough_id: String,
    pub title: String,
    pub rev: u64,
    pub nodes: Vec<McpWalkthroughNode>,
    pub edges: Vec<McpWalkthroughEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughGetResponse {
    pub walkthrough: McpWalkthrough,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughDigestCounts {
    pub nodes: u64,
    pub edges: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughDigest {
    pub rev: u64,
    pub counts: WalkthroughDigestCounts,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughGetDigestResponse {
    pub digest: WalkthroughDigest,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughRenderTextResponse {
    pub text: String,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramRenderTextResponse {
    pub text: String,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RouteFindParams {
    pub from_ref: String,
    pub to_ref: String,
    pub limit: Option<u64>,
    pub max_hops: Option<u64>,
    pub ordering: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RouteFindResponse {
    pub routes: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramOpenParams {
    pub diagram_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramOpenResponse {
    pub active_diagram_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramDeleteParams {
    pub diagram_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramDeleteResponse {
    pub deleted_diagram_id: String,
    pub active_diagram_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramCurrentResponse {
    pub active_diagram_id: Option<String>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughOpenParams {
    pub walkthrough_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughOpenResponse {
    pub active_walkthrough_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughCurrentResponse {
    pub active_walkthrough_id: Option<String>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttentionReadResponse {
    pub object_ref: Option<String>,
    pub diagram_id: Option<String>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AttentionAgentSetParams {
    pub object_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttentionSetResponse {
    pub object_ref: String,
    pub diagram_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AttentionClearResponse {
    pub cleared: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FollowAiReadResponse {
    pub enabled: bool,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FollowAiSetParams {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FollowAiSetResponse {
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum UpdateMode {
    #[default]
    Replace,
    Add,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectionGetResponse {
    pub object_refs: Vec<String>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SelectionUpdateParams {
    pub object_refs: Vec<String>,
    #[serde(default)]
    pub mode: UpdateMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SelectionUpdateResponse {
    pub applied: Vec<String>,
    pub ignored: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ViewScroll {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ViewGetStateResponse {
    pub active_diagram_id: Option<String>,
    pub scroll: ViewScroll,
    pub panes: BTreeMap<String, bool>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramCounts {
    pub participants: u64,
    pub messages: u64,
    pub nodes: u64,
    pub edges: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramDigest {
    pub rev: u64,
    pub counts: DiagramCounts,
    pub key_names: Vec<String>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramSnapshot {
    pub rev: u64,
    pub kind: String,
    pub mermaid: String,
    pub context: ReadContext,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramCreateFromMermaidResponse {
    pub diagram: DiagramSummary,
    pub active_diagram_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramGetAstResponse {
    pub diagram_id: String,
    pub kind: String,
    pub rev: u64,
    pub ast: McpDiagramAst,
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSeqParticipantAst {
    pub participant_id: String,
    pub mermaid_name: String,
    pub role: Option<String>,
    pub note: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSeqBlockAst {
    pub block_id: String,
    pub kind: McpSeqBlockKind,
    pub header: Option<String>,
    pub sections: Vec<McpSeqSectionAst>,
    pub blocks: Vec<McpSeqBlockAst>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpSeqBlockKind {
    Alt,
    Opt,
    Loop,
    Par,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpSeqSectionAst {
    pub section_id: String,
    pub kind: McpSeqSectionKind,
    pub header: Option<String>,
    pub message_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpSeqSectionKind {
    Main,
    Else,
    And,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpFlowNodeAst {
    pub node_id: String,
    pub label: String,
    pub shape: String,
    pub mermaid_id: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McpFlowEdgeAst {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub label: Option<String>,
    pub connector: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeltaChangeKind {
    Added,
    Removed,
    Updated,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeltaChange {
    pub kind: DeltaChangeKind,
    pub refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramDeltaResponse {
    pub from_rev: u64,
    pub to_rev: u64,
    pub changes: Vec<DeltaChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeltaSummary {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyOpsResponse {
    pub new_rev: u64,
    pub applied: u64,
    pub delta: DeltaSummary,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramTargetParams {
    pub diagram_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramGetSliceParams {
    pub diagram_id: Option<String>,
    pub center_ref: String,
    pub radius: Option<u64>,
    pub depth: Option<u64>,
    pub filters: Option<DiagramSliceFilters>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramSliceFilters {
    pub include_categories: Option<Vec<String>>,
    pub exclude_categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramGetSliceResponse {
    pub objects: Vec<String>,
    pub edges: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetDeltaParams {
    pub diagram_id: Option<String>,
    pub since_rev: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughGetDeltaParams {
    pub walkthrough_id: String,
    pub since_rev: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ApplyOpsParams {
    pub diagram_id: Option<String>,
    pub base_rev: u64,
    pub ops: Vec<McpOp>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WalkthroughApplyOpsParams {
    pub walkthrough_id: String,
    pub base_rev: u64,
    pub ops: Vec<McpWalkthroughOp>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DiagramProposeOpsParams {
    pub diagram_id: Option<String>,
    pub base_rev: u64,
    pub ops: Vec<McpOp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagramProposeOpsResponse {
    pub new_rev: u64,
    pub applied: u64,
    pub delta: DeltaSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WalkthroughDeltaResponse {
    pub from_rev: u64,
    pub to_rev: u64,
    pub changes: Vec<DeltaChange>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefSummary {
    pub xref_id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefListResponse {
    pub xrefs: Vec<XRefSummary>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct XRefNeighborsParams {
    pub object_ref: String,
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefNeighborsResponse {
    pub neighbors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct XRefAddParams {
    pub xref_id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefAddResponse {
    pub xref_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct XRefRemoveParams {
    pub xref_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XRefRemoveResponse {
    pub removed: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SeqTraceParams {
    pub diagram_id: Option<String>,
    pub from_message_id: Option<String>,
    pub direction: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SeqTraceResponse {
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SeqSearchParams {
    pub diagram_id: Option<String>,
    pub needle: String,
    pub mode: Option<String>,
    pub case_insensitive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SeqSearchResponse {
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SeqMessagesParams {
    pub diagram_id: Option<String>,
    pub from_participant_id: Option<String>,
    pub to_participant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SeqMessagesResponse {
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowReachableParams {
    pub diagram_id: Option<String>,
    pub from_node_id: String,
    pub direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowReachableResponse {
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowUnreachableParams {
    pub diagram_id: Option<String>,
    pub start_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowUnreachableResponse {
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowPathsParams {
    pub diagram_id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub limit: Option<u64>,
    pub max_extra_hops: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowPathsResponse {
    pub paths: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowCyclesResponse {
    pub cycles: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowDeadEndsResponse {
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct FlowDegreesParams {
    pub diagram_id: Option<String>,
    pub top: Option<u64>,
    pub sort_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowDegreeNode {
    pub node_ref: String,
    pub label: String,
    pub in_degree: u64,
    pub out_degree: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowDegreesResponse {
    pub nodes: Vec<FlowDegreeNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Sync,
    Async,
    Return,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ObjectGetParams {
    pub object_ref: Option<String>,
    pub object_refs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObjectGetItem {
    pub object_ref: String,
    pub object: McpObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObjectGetResponse {
    pub objects: Vec<ObjectGetItem>,
    pub context: ReadContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpObject {
    SeqParticipant {
        mermaid_name: String,
        role: Option<String>,
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
    },
    FlowEdge {
        from_node_id: String,
        to_node_id: String,
        label: Option<String>,
        connector: Option<String>,
        style: Option<String>,
    },
}

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
