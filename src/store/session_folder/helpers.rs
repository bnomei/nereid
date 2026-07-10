// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

// Session-folder helpers (include!'d into parent): meta/walkthrough JSON codecs,
// atomic writes, path safety, and durability-aware filesystem commits.

fn export_diagram_mmd(
    folder: &SessionFolder,
    diagram: &Diagram,
    mmd_path: &Path,
) -> Result<(), StoreError> {
    let mmd = match diagram.ast() {
        DiagramAst::Sequence(ast) => {
            export_sequence_diagram(ast).map_err(|source| StoreError::MermaidSequenceExport {
                diagram_id: diagram.diagram_id().clone(),
                path: mmd_path.to_path_buf(),
                source: Box::new(source),
            })?
        }
        DiagramAst::Flowchart(ast) => {
            export_flowchart(ast).map_err(|source| StoreError::MermaidFlowchartExport {
                diagram_id: diagram.diagram_id().clone(),
                path: mmd_path.to_path_buf(),
                source: Box::new(source),
            })?
        }
        DiagramAst::Class(ast) => {
            export_class_diagram(ast).map_err(|source| StoreError::MermaidClassExport {
                diagram_id: diagram.diagram_id().clone(),
                path: mmd_path.to_path_buf(),
                source: Box::new(source),
            })?
        }
        DiagramAst::Er(ast) => {
            export_er_diagram(ast).map_err(|source| StoreError::MermaidErExport {
                diagram_id: diagram.diagram_id().clone(),
                path: mmd_path.to_path_buf(),
                source: Box::new(source),
            })?
        }
        DiagramAst::Gantt(ast) => {
            export_gantt_diagram(ast).map_err(|source| StoreError::MermaidGanttExport {
                diagram_id: diagram.diagram_id().clone(),
                path: mmd_path.to_path_buf(),
                source: Box::new(source),
            })?
        }
    };

    write_atomic_in_session(folder.root(), mmd_path, mmd.as_bytes(), folder.durability)?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalkthroughJson {
    walkthrough_id: String,
    title: String,
    #[serde(default)]
    rev: u64,
    #[serde(default)]
    nodes: Vec<WalkthroughNodeJson>,
    #[serde(default)]
    edges: Vec<WalkthroughEdgeJson>,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalkthroughNodeJson {
    node_id: String,
    title: String,
    #[serde(default)]
    body_md: Option<String>,
    #[serde(default)]
    refs: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalkthroughEdgeJson {
    from_node_id: String,
    to_node_id: String,
    kind: String,
    #[serde(default)]
    label: Option<String>,
}


fn walkthrough_to_json(walkthrough: &Walkthrough) -> WalkthroughJson {
    WalkthroughJson {
        walkthrough_id: walkthrough.walkthrough_id().to_string(),
        title: walkthrough.title().to_owned(),
        rev: walkthrough.rev(),
        nodes: walkthrough
            .nodes()
            .iter()
            .map(|node| WalkthroughNodeJson {
                node_id: node.node_id().to_string(),
                title: node.title().to_owned(),
                body_md: node.body_md().map(ToOwned::to_owned),
                refs: node.refs().iter().map(ToString::to_string).collect(),
                tags: node.tags().to_vec(),
                status: node.status().map(ToOwned::to_owned),
            })
            .collect(),
        edges: walkthrough
            .edges()
            .iter()
            .map(|edge| WalkthroughEdgeJson {
                from_node_id: edge.from_node_id().to_string(),
                to_node_id: edge.to_node_id().to_string(),
                kind: edge.kind().to_owned(),
                label: edge.label().map(ToOwned::to_owned),
            })
            .collect(),
        source: walkthrough.source().map(ToOwned::to_owned),
    }
}

const WALKTHROUGH_REV_CAP: u64 = 1_000_000;

fn walkthrough_from_json(walkthrough_json: WalkthroughJson) -> Result<Walkthrough, StoreError> {
    let walkthrough_id =
        WalkthroughId::new(walkthrough_json.walkthrough_id.clone()).map_err(|source| {
            StoreError::InvalidId {
                field: "walkthrough_id",
                value: walkthrough_json.walkthrough_id,
                source: Box::new(source),
            }
        })?;

    let mut walkthrough = Walkthrough::new(walkthrough_id, walkthrough_json.title);
    walkthrough.set_source(walkthrough_json.source);

    walkthrough.set_rev(walkthrough_json.rev.min(WALKTHROUGH_REV_CAP));

    for node_json in walkthrough_json.nodes {
        let node_id = WalkthroughNodeId::new(node_json.node_id.clone()).map_err(|source| {
            StoreError::InvalidId {
                field: "nodes[].node_id",
                value: node_json.node_id,
                source: Box::new(source),
            }
        })?;

        let mut node = WalkthroughNode::new(node_id, node_json.title);
        node.set_body_md(node_json.body_md);
        node.set_status(node_json.status);

        for obj_ref_str in node_json.refs {
            let obj_ref =
                ObjectRef::parse(&obj_ref_str).map_err(|source| StoreError::InvalidObjectRef {
                    field: "nodes[].refs[]",
                    value: obj_ref_str.clone(),
                    source: Box::new(source),
                })?;
            node.refs_mut().push(obj_ref);
        }

        node.tags_mut().extend(node_json.tags);
        walkthrough.nodes_mut().push(node);
    }

    for edge_json in walkthrough_json.edges {
        let from_node_id =
            WalkthroughNodeId::new(edge_json.from_node_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "edges[].from_node_id",
                    value: edge_json.from_node_id,
                    source: Box::new(source),
                }
            })?;
        let to_node_id =
            WalkthroughNodeId::new(edge_json.to_node_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "edges[].to_node_id",
                    value: edge_json.to_node_id,
                    source: Box::new(source),
                }
            })?;

        let mut edge = WalkthroughEdge::new(from_node_id, to_node_id, edge_json.kind);
        edge.set_label(edge_json.label);
        walkthrough.edges_mut().push(edge);
    }

    Ok(walkthrough)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetaJson {
    session_id: String,
    #[serde(default)]
    active_diagram_id: Option<String>,
    #[serde(default)]
    active_walkthrough_id: Option<String>,
    #[serde(default)]
    walkthrough_ids: Option<Vec<String>>,
    #[serde(default)]
    diagrams: Vec<SessionMetaDiagramJson>,
    #[serde(default)]
    xrefs: Vec<SessionXRefJson>,
    #[serde(default)]
    selected_object_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetaDiagramJson {
    diagram_id: String,
    name: String,
    kind: DiagramKindJson,
    mmd_path: String,
    #[serde(default)]
    rev: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionXRefJson {
    xref_id: String,
    from: String,
    to: String,
    kind: String,
    #[serde(default)]
    label: Option<String>,
    status: XRefStatusJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum DiagramKindJson {
    Sequence,
    Flowchart,
    Class,
    Er,
    Gantt,
}

impl From<DiagramKind> for DiagramKindJson {
    fn from(kind: DiagramKind) -> Self {
        match kind {
            DiagramKind::Sequence => Self::Sequence,
            DiagramKind::Flowchart => Self::Flowchart,
            DiagramKind::Class => Self::Class,
            DiagramKind::Er => Self::Er,
            DiagramKind::Gantt => Self::Gantt,
        }
    }
}

impl From<DiagramKindJson> for DiagramKind {
    fn from(kind: DiagramKindJson) -> Self {
        match kind {
            DiagramKindJson::Sequence => Self::Sequence,
            DiagramKindJson::Flowchart => Self::Flowchart,
            DiagramKindJson::Class => Self::Class,
            DiagramKindJson::Er => Self::Er,
            DiagramKindJson::Gantt => Self::Gantt,
        }
    }
}

fn session_meta_to_json(
    session_dir: &Path,
    meta: &SessionMeta,
) -> Result<SessionMetaJson, StoreError> {
    let diagrams = meta
        .diagrams
        .iter()
        .map(|diagram| {
            let relative_mmd_path =
                to_relative_path(session_dir, &diagram.mmd_path, "diagrams[].mmd_path")?;
            Ok(SessionMetaDiagramJson {
                diagram_id: diagram.diagram_id.to_string(),
                name: diagram.name.clone(),
                kind: diagram.kind.into(),
                mmd_path: relative_mmd_path.to_string_lossy().into_owned(),
                rev: diagram.rev,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let xrefs = meta
        .xrefs
        .iter()
        .map(|xref| SessionXRefJson {
            xref_id: xref.xref_id.to_string(),
            from: xref.from.to_string(),
            to: xref.to.to_string(),
            kind: xref.kind.clone(),
            label: xref.label.clone(),
            status: xref.status.into(),
        })
        .collect();

    Ok(SessionMetaJson {
        session_id: meta.session_id.to_string(),
        active_diagram_id: meta.active_diagram_id.as_ref().map(ToString::to_string),
        active_walkthrough_id: meta.active_walkthrough_id.as_ref().map(ToString::to_string),
        walkthrough_ids: meta
            .walkthrough_ids
            .as_ref()
            .map(|ids| ids.iter().map(ToString::to_string).collect()),
        diagrams,
        xrefs,
        selected_object_refs: meta
            .selected_object_refs
            .iter()
            .map(ToString::to_string)
            .collect(),
    })
}

fn session_meta_from_json(
    session_dir: &Path,
    meta_json: SessionMetaJson,
) -> Result<SessionMeta, StoreError> {
    let session_id =
        SessionId::new(meta_json.session_id.clone()).map_err(|source| StoreError::InvalidId {
            field: "session_id",
            value: meta_json.session_id,
            source: Box::new(source),
        })?;

    let active_diagram_id = meta_json
        .active_diagram_id
        .map(|value| {
            DiagramId::new(value.clone()).map_err(|source| StoreError::InvalidId {
                field: "active_diagram_id",
                value,
                source: Box::new(source),
            })
        })
        .transpose()?;

    let active_walkthrough_id = meta_json
        .active_walkthrough_id
        .map(|value| {
            WalkthroughId::new(value.clone()).map_err(|source| StoreError::InvalidId {
                field: "active_walkthrough_id",
                value,
                source: Box::new(source),
            })
        })
        .transpose()?;

    let walkthrough_ids = meta_json
        .walkthrough_ids
        .map(|values| {
            values
                .into_iter()
                .map(|value| {
                    WalkthroughId::new(value.clone()).map_err(|source| StoreError::InvalidId {
                        field: "walkthrough_ids[]",
                        value,
                        source: Box::new(source),
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()
        })
        .transpose()?;

    let diagrams = meta_json
        .diagrams
        .into_iter()
        .map(|diagram_json| {
            let diagram_id = DiagramId::new(diagram_json.diagram_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "diagrams[].diagram_id",
                    value: diagram_json.diagram_id,
                    source: Box::new(source),
                }
            })?;

            let relative_mmd_path = PathBuf::from(&diagram_json.mmd_path);
            validate_relative_path("diagrams[].mmd_path", &relative_mmd_path)?;

            Ok(SessionMetaDiagram {
                diagram_id,
                name: diagram_json.name,
                kind: diagram_json.kind.into(),
                mmd_path: session_dir.join(relative_mmd_path),
                rev: diagram_json.rev,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let xrefs = meta_json
        .xrefs
        .into_iter()
        .map(|xref_json| {
            let xref_id =
                XRefId::new(xref_json.xref_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "xrefs[].xref_id",
                    value: xref_json.xref_id,
                    source: Box::new(source),
                })?;

            let from = ObjectRef::parse(&xref_json.from).map_err(|source| {
                StoreError::InvalidObjectRef {
                    field: "xrefs[].from",
                    value: xref_json.from,
                    source: Box::new(source),
                }
            })?;

            let to =
                ObjectRef::parse(&xref_json.to).map_err(|source| StoreError::InvalidObjectRef {
                    field: "xrefs[].to",
                    value: xref_json.to,
                    source: Box::new(source),
                })?;

            Ok(SessionXRef {
                xref_id,
                from,
                to,
                kind: xref_json.kind,
                label: xref_json.label,
                status: xref_json.status.into(),
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let selected_object_refs = meta_json
        .selected_object_refs
        .into_iter()
        .map(|value| {
            ObjectRef::parse(&value).map_err(|source| StoreError::InvalidObjectRef {
                field: "selected_object_refs[]",
                value,
                source: Box::new(source),
            })
        })
        .collect::<Result<BTreeSet<_>, StoreError>>()?
        .into_iter()
        .collect::<Vec<_>>();

    Ok(SessionMeta {
        session_id,
        active_diagram_id,
        active_walkthrough_id,
        walkthrough_ids,
        diagrams,
        xrefs,
        selected_object_refs,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramMetaJson {
    diagram_id: String,
    mmd_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_symbol_repository_id: Option<String>,
    #[serde(default)]
    stable_id_map: DiagramStableIdMapJson,
    #[serde(default)]
    xrefs: Vec<DiagramXRefJson>,
    #[serde(default)]
    flow_edges: Vec<DiagramFlowEdgeMetaJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    class_relations: Vec<DiagramClassRelationMetaJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    er_relationships: Vec<DiagramErRelationshipMetaJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    gantt_sections: Vec<DiagramGanttSectionMetaJson>,
    #[serde(default)]
    sequence_messages: Vec<DiagramSequenceMessageMetaJson>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sequence_blocks: Vec<DiagramSequenceBlockMetaJson>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    flow_node_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    sequence_participant_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    class_node_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    er_entity_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    gantt_task_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    gantt_lane_notes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    flow_node_symbols: BTreeMap<String, DiagramSymbolAnchorJson>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    sequence_participant_symbols: BTreeMap<String, DiagramSymbolAnchorJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramSymbolAnchorJson {
    stable_symbol_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    repository_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DiagramStableIdMapJson {
    #[serde(default)]
    by_mermaid_id: BTreeMap<String, String>,
    #[serde(default)]
    by_name: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    by_fingerprint: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramXRefJson {
    xref_id: String,
    from: String,
    to: String,
    kind: String,
    #[serde(default)]
    label: Option<String>,
    status: XRefStatusJson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramFlowEdgeMetaJson {
    edge_id: String,
    from_node_id: String,
    to_node_id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramClassRelationMetaJson {
    relation_id: String,
    from_class_id: String,
    to_class_id: String,
    kind: ClassRelationKindJson,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    raw_connector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramErRelationshipMetaJson {
    relationship_id: String,
    from_entity_id: String,
    to_entity_id: String,
    from_card: ErCardinalityJson,
    to_card: ErCardinalityJson,
    stroke: ErStrokeJson,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramGanttSectionMetaJson {
    section_id: String,
    name: String,
    #[serde(default)]
    task_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ClassRelationKindJson {
    Inheritance,
    Composition,
    Aggregation,
    Association,
    Dependency,
    Realization,
    Link,
}

impl From<ClassRelationKind> for ClassRelationKindJson {
    fn from(kind: ClassRelationKind) -> Self {
        match kind {
            ClassRelationKind::Inheritance => Self::Inheritance,
            ClassRelationKind::Composition => Self::Composition,
            ClassRelationKind::Aggregation => Self::Aggregation,
            ClassRelationKind::Association => Self::Association,
            ClassRelationKind::Dependency => Self::Dependency,
            ClassRelationKind::Realization => Self::Realization,
            ClassRelationKind::Link => Self::Link,
        }
    }
}

impl From<ClassRelationKindJson> for ClassRelationKind {
    fn from(kind: ClassRelationKindJson) -> Self {
        match kind {
            ClassRelationKindJson::Inheritance => Self::Inheritance,
            ClassRelationKindJson::Composition => Self::Composition,
            ClassRelationKindJson::Aggregation => Self::Aggregation,
            ClassRelationKindJson::Association => Self::Association,
            ClassRelationKindJson::Dependency => Self::Dependency,
            ClassRelationKindJson::Realization => Self::Realization,
            ClassRelationKindJson::Link => Self::Link,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ErCardinalityJson {
    ExactlyOne,
    ZeroOrOne,
    OneOrMore,
    ZeroOrMore,
}

impl From<ErCardinality> for ErCardinalityJson {
    fn from(cardinality: ErCardinality) -> Self {
        match cardinality {
            ErCardinality::ExactlyOne => Self::ExactlyOne,
            ErCardinality::ZeroOrOne => Self::ZeroOrOne,
            ErCardinality::OneOrMore => Self::OneOrMore,
            ErCardinality::ZeroOrMore => Self::ZeroOrMore,
        }
    }
}

impl From<ErCardinalityJson> for ErCardinality {
    fn from(cardinality: ErCardinalityJson) -> Self {
        match cardinality {
            ErCardinalityJson::ExactlyOne => Self::ExactlyOne,
            ErCardinalityJson::ZeroOrOne => Self::ZeroOrOne,
            ErCardinalityJson::OneOrMore => Self::OneOrMore,
            ErCardinalityJson::ZeroOrMore => Self::ZeroOrMore,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ErStrokeJson {
    Identifying,
    NonIdentifying,
}

impl From<ErStroke> for ErStrokeJson {
    fn from(stroke: ErStroke) -> Self {
        match stroke {
            ErStroke::Identifying => Self::Identifying,
            ErStroke::NonIdentifying => Self::NonIdentifying,
        }
    }
}

impl From<ErStrokeJson> for ErStroke {
    fn from(stroke: ErStrokeJson) -> Self {
        match stroke {
            ErStrokeJson::Identifying => Self::Identifying,
            ErStrokeJson::NonIdentifying => Self::NonIdentifying,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramSequenceMessageMetaJson {
    message_id: String,
    from_participant_id: String,
    to_participant_id: String,
    kind: SequenceMessageKindJson,
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramSequenceBlockMetaJson {
    block_id: String,
    kind: SequenceBlockKindJson,
    #[serde(default)]
    header: Option<String>,
    #[serde(default)]
    parent_block_id: Option<String>,
    #[serde(default)]
    sections: Vec<DiagramSequenceSectionMetaJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagramSequenceSectionMetaJson {
    section_id: String,
    kind: SequenceSectionKindJson,
    #[serde(default)]
    header: Option<String>,
    #[serde(default)]
    message_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SequenceBlockKindJson {
    Alt,
    Opt,
    Loop,
    Par,
}

impl From<crate::model::seq_ast::SequenceBlockKind> for SequenceBlockKindJson {
    fn from(kind: crate::model::seq_ast::SequenceBlockKind) -> Self {
        match kind {
            crate::model::seq_ast::SequenceBlockKind::Alt => Self::Alt,
            crate::model::seq_ast::SequenceBlockKind::Opt => Self::Opt,
            crate::model::seq_ast::SequenceBlockKind::Loop => Self::Loop,
            crate::model::seq_ast::SequenceBlockKind::Par => Self::Par,
        }
    }
}

impl From<SequenceBlockKindJson> for crate::model::seq_ast::SequenceBlockKind {
    fn from(kind: SequenceBlockKindJson) -> Self {
        match kind {
            SequenceBlockKindJson::Alt => Self::Alt,
            SequenceBlockKindJson::Opt => Self::Opt,
            SequenceBlockKindJson::Loop => Self::Loop,
            SequenceBlockKindJson::Par => Self::Par,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SequenceSectionKindJson {
    Main,
    Else,
    And,
}

impl From<crate::model::seq_ast::SequenceSectionKind> for SequenceSectionKindJson {
    fn from(kind: crate::model::seq_ast::SequenceSectionKind) -> Self {
        match kind {
            crate::model::seq_ast::SequenceSectionKind::Main => Self::Main,
            crate::model::seq_ast::SequenceSectionKind::Else => Self::Else,
            crate::model::seq_ast::SequenceSectionKind::And => Self::And,
        }
    }
}

impl From<SequenceSectionKindJson> for crate::model::seq_ast::SequenceSectionKind {
    fn from(kind: SequenceSectionKindJson) -> Self {
        match kind {
            SequenceSectionKindJson::Main => Self::Main,
            SequenceSectionKindJson::Else => Self::Else,
            SequenceSectionKindJson::And => Self::And,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SequenceMessageKindJson {
    Sync,
    Async,
    Return,
}

impl From<SequenceMessageKind> for SequenceMessageKindJson {
    fn from(kind: SequenceMessageKind) -> Self {
        match kind {
            SequenceMessageKind::Sync => Self::Sync,
            SequenceMessageKind::Async => Self::Async,
            SequenceMessageKind::Return => Self::Return,
        }
    }
}

impl From<SequenceMessageKindJson> for SequenceMessageKind {
    fn from(kind: SequenceMessageKindJson) -> Self {
        match kind {
            SequenceMessageKindJson::Sync => Self::Sync,
            SequenceMessageKindJson::Async => Self::Async,
            SequenceMessageKindJson::Return => Self::Return,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum XRefStatusJson {
    Ok,
    DanglingFrom,
    DanglingTo,
    DanglingBoth,
}

impl From<XRefStatus> for XRefStatusJson {
    fn from(status: XRefStatus) -> Self {
        match status {
            XRefStatus::Ok => Self::Ok,
            XRefStatus::DanglingFrom => Self::DanglingFrom,
            XRefStatus::DanglingTo => Self::DanglingTo,
            XRefStatus::DanglingBoth => Self::DanglingBoth,
        }
    }
}

impl From<XRefStatusJson> for XRefStatus {
    fn from(status: XRefStatusJson) -> Self {
        match status {
            XRefStatusJson::Ok => Self::Ok,
            XRefStatusJson::DanglingFrom => Self::DanglingFrom,
            XRefStatusJson::DanglingTo => Self::DanglingTo,
            XRefStatusJson::DanglingBoth => Self::DanglingBoth,
        }
    }
}

impl From<ModelXRefStatus> for XRefStatusJson {
    fn from(status: ModelXRefStatus) -> Self {
        match status {
            ModelXRefStatus::Ok => Self::Ok,
            ModelXRefStatus::DanglingFrom => Self::DanglingFrom,
            ModelXRefStatus::DanglingTo => Self::DanglingTo,
            ModelXRefStatus::DanglingBoth => Self::DanglingBoth,
        }
    }
}

impl From<XRefStatusJson> for ModelXRefStatus {
    fn from(status: XRefStatusJson) -> Self {
        match status {
            XRefStatusJson::Ok => Self::Ok,
            XRefStatusJson::DanglingFrom => Self::DanglingFrom,
            XRefStatusJson::DanglingTo => Self::DanglingTo,
            XRefStatusJson::DanglingBoth => Self::DanglingBoth,
        }
    }
}

fn diagram_meta_to_json(
    session_dir: &Path,
    meta: &DiagramMeta,
) -> Result<DiagramMetaJson, StoreError> {
    let relative_mmd_path = to_relative_path(session_dir, &meta.mmd_path, "mmd_path")?;

    let stable_id_map = DiagramStableIdMapJson {
        by_mermaid_id: meta.stable_id_map.by_mermaid_id.clone(),
        by_name: meta.stable_id_map.by_name.clone(),
        by_fingerprint: meta.stable_id_map.by_fingerprint.clone(),
    };

    let xrefs = meta
        .xrefs
        .iter()
        .map(|xref| DiagramXRefJson {
            xref_id: xref.xref_id.clone(),
            from: xref.from.clone(),
            to: xref.to.clone(),
            kind: xref.kind.clone(),
            label: xref.label.clone(),
            status: xref.status.into(),
        })
        .collect();

    let flow_edges = meta
        .flow_edges
        .iter()
        .map(|edge| DiagramFlowEdgeMetaJson {
            edge_id: edge.edge_id.to_string(),
            from_node_id: edge.from_node_id.to_string(),
            to_node_id: edge.to_node_id.to_string(),
            label: edge.label.clone(),
            style: edge.style.clone(),
        })
        .collect();

    let class_relations = meta
        .class_relations
        .iter()
        .map(|relation| DiagramClassRelationMetaJson {
            relation_id: relation.relation_id.to_string(),
            from_class_id: relation.from_class_id.to_string(),
            to_class_id: relation.to_class_id.to_string(),
            kind: relation.kind.into(),
            label: relation.label.clone(),
            raw_connector: relation.raw_connector.clone(),
        })
        .collect();

    let er_relationships = meta
        .er_relationships
        .iter()
        .map(|relationship| DiagramErRelationshipMetaJson {
            relationship_id: relationship.relationship_id.to_string(),
            from_entity_id: relationship.from_entity_id.to_string(),
            to_entity_id: relationship.to_entity_id.to_string(),
            from_card: relationship.from_card.into(),
            to_card: relationship.to_card.into(),
            stroke: relationship.stroke.into(),
            label: relationship.label.clone(),
        })
        .collect();

    let gantt_sections = meta
        .gantt_sections
        .iter()
        .map(|section| DiagramGanttSectionMetaJson {
            section_id: section.section_id.to_string(),
            name: section.name.clone(),
            task_ids: section.task_ids.iter().map(ToString::to_string).collect(),
        })
        .collect();

    let sequence_messages = meta
        .sequence_messages
        .iter()
        .map(|msg| DiagramSequenceMessageMetaJson {
            message_id: msg.message_id.to_string(),
            from_participant_id: msg.from_participant_id.to_string(),
            to_participant_id: msg.to_participant_id.to_string(),
            kind: msg.kind.into(),
            text: msg.text.clone(),
        })
        .collect();

    let sequence_blocks = meta
        .sequence_blocks
        .iter()
        .map(|block| DiagramSequenceBlockMetaJson {
            block_id: block.block_id.to_string(),
            kind: block.kind.into(),
            header: block.header.clone(),
            parent_block_id: block.parent_block_id.as_ref().map(ToString::to_string),
            sections: block
                .sections
                .iter()
                .map(|section| DiagramSequenceSectionMetaJson {
                    section_id: section.section_id.to_string(),
                    kind: section.kind.into(),
                    header: section.header.clone(),
                    message_ids: section
                        .message_ids
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                })
                .collect(),
        })
        .collect();

    let flow_node_notes: BTreeMap<String, String> = meta
        .flow_node_notes
        .iter()
        .map(|(node_id, note)| (node_id.to_string(), note.clone()))
        .collect();

    let sequence_participant_notes: BTreeMap<String, String> = meta
        .sequence_participant_notes
        .iter()
        .map(|(participant_id, note)| (participant_id.to_string(), note.clone()))
        .collect();

    let class_node_notes: BTreeMap<String, String> = meta
        .class_node_notes
        .iter()
        .map(|(class_id, note)| (class_id.to_string(), note.clone()))
        .collect();

    let er_entity_notes: BTreeMap<String, String> = meta
        .er_entity_notes
        .iter()
        .map(|(entity_id, note)| (entity_id.to_string(), note.clone()))
        .collect();

    let gantt_task_notes: BTreeMap<String, String> = meta
        .gantt_task_notes
        .iter()
        .map(|(task_id, note)| (task_id.to_string(), note.clone()))
        .collect();

    let gantt_lane_notes: BTreeMap<String, String> = meta
        .gantt_lane_notes
        .iter()
        .map(|(lane_id, note)| (lane_id.to_string(), note.clone()))
        .collect();

    let flow_node_symbols: BTreeMap<String, DiagramSymbolAnchorJson> = meta
        .flow_node_symbols
        .iter()
        .map(|(node_id, symbol)| {
            (
                node_id.to_string(),
                DiagramSymbolAnchorJson {
                    stable_symbol_id: symbol.stable_symbol_id().to_owned(),
                    repository_id: symbol.repository_id().map(ToOwned::to_owned),
                },
            )
        })
        .collect();

    let sequence_participant_symbols: BTreeMap<String, DiagramSymbolAnchorJson> = meta
        .sequence_participant_symbols
        .iter()
        .map(|(participant_id, symbol)| {
            (
                participant_id.to_string(),
                DiagramSymbolAnchorJson {
                    stable_symbol_id: symbol.stable_symbol_id().to_owned(),
                    repository_id: symbol.repository_id().map(ToOwned::to_owned),
                },
            )
        })
        .collect();

    Ok(DiagramMetaJson {
        diagram_id: meta.diagram_id.to_string(),
        mmd_path: relative_mmd_path.to_string_lossy().into_owned(),
        default_symbol_repository_id: meta.default_symbol_repository_id.clone(),
        stable_id_map,
        xrefs,
        flow_edges,
        class_relations,
        er_relationships,
        gantt_sections,
        sequence_messages,
        sequence_blocks,
        flow_node_notes,
        sequence_participant_notes,
        class_node_notes,
        er_entity_notes,
        gantt_task_notes,
        gantt_lane_notes,
        flow_node_symbols,
        sequence_participant_symbols,
    })
}

fn diagram_meta_from_json(
    session_dir: &Path,
    meta_json: DiagramMetaJson,
) -> Result<DiagramMeta, StoreError> {
    let diagram_id =
        DiagramId::new(meta_json.diagram_id.clone()).map_err(|source| StoreError::InvalidId {
            field: "diagram_id",
            value: meta_json.diagram_id,
            source: Box::new(source),
        })?;

    let relative_mmd_path = PathBuf::from(&meta_json.mmd_path);
    validate_relative_path("mmd_path", &relative_mmd_path)?;
    let mmd_path = session_dir.join(relative_mmd_path);

    let stable_id_map = DiagramStableIdMap {
        by_mermaid_id: meta_json.stable_id_map.by_mermaid_id,
        by_name: meta_json.stable_id_map.by_name,
        by_fingerprint: meta_json.stable_id_map.by_fingerprint,
    };

    let xrefs = meta_json
        .xrefs
        .into_iter()
        .map(|xref_json| DiagramXRef {
            xref_id: xref_json.xref_id,
            from: xref_json.from,
            to: xref_json.to,
            kind: xref_json.kind,
            label: xref_json.label,
            status: xref_json.status.into(),
        })
        .collect();

    let flow_edges = meta_json
        .flow_edges
        .into_iter()
        .map(|edge_json| {
            let edge_id = ObjectId::new(edge_json.edge_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "flow_edges[].edge_id",
                    value: edge_json.edge_id,
                    source: Box::new(source),
                }
            })?;
            let from_node_id = ObjectId::new(edge_json.from_node_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "flow_edges[].from_node_id",
                    value: edge_json.from_node_id,
                    source: Box::new(source),
                }
            })?;
            let to_node_id = ObjectId::new(edge_json.to_node_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "flow_edges[].to_node_id",
                    value: edge_json.to_node_id,
                    source: Box::new(source),
                }
            })?;

            Ok(DiagramFlowEdgeMeta {
                edge_id,
                from_node_id,
                to_node_id,
                label: edge_json.label,
                style: edge_json.style,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let class_relations = meta_json
        .class_relations
        .into_iter()
        .map(|relation| {
            let relation_id = ObjectId::new(relation.relation_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "class_relations[].relation_id",
                    value: relation.relation_id,
                    source: Box::new(source),
                }
            })?;
            let from_class_id =
                ObjectId::new(relation.from_class_id.clone()).map_err(|source| {
                    StoreError::InvalidId {
                        field: "class_relations[].from_class_id",
                        value: relation.from_class_id,
                        source: Box::new(source),
                    }
                })?;
            let to_class_id = ObjectId::new(relation.to_class_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "class_relations[].to_class_id",
                    value: relation.to_class_id,
                    source: Box::new(source),
                }
            })?;
            Ok(DiagramClassRelationMeta {
                relation_id,
                from_class_id,
                to_class_id,
                kind: relation.kind.into(),
                label: relation.label,
                raw_connector: relation.raw_connector,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let er_relationships = meta_json
        .er_relationships
        .into_iter()
        .map(|relationship| {
            let relationship_id =
                ObjectId::new(relationship.relationship_id.clone()).map_err(|source| {
                    StoreError::InvalidId {
                        field: "er_relationships[].relationship_id",
                        value: relationship.relationship_id,
                        source: Box::new(source),
                    }
                })?;
            let from_entity_id =
                ObjectId::new(relationship.from_entity_id.clone()).map_err(|source| {
                    StoreError::InvalidId {
                        field: "er_relationships[].from_entity_id",
                        value: relationship.from_entity_id,
                        source: Box::new(source),
                    }
                })?;
            let to_entity_id =
                ObjectId::new(relationship.to_entity_id.clone()).map_err(|source| {
                    StoreError::InvalidId {
                        field: "er_relationships[].to_entity_id",
                        value: relationship.to_entity_id,
                        source: Box::new(source),
                    }
                })?;
            Ok(DiagramErRelationshipMeta {
                relationship_id,
                from_entity_id,
                to_entity_id,
                from_card: relationship.from_card.into(),
                to_card: relationship.to_card.into(),
                stroke: relationship.stroke.into(),
                label: relationship.label,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let gantt_sections = meta_json
        .gantt_sections
        .into_iter()
        .map(|section| {
            let section_id = ObjectId::new(section.section_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "gantt_sections[].section_id",
                    value: section.section_id,
                    source: Box::new(source),
                }
            })?;
            let task_ids = section
                .task_ids
                .into_iter()
                .map(|raw| {
                    ObjectId::new(raw.clone()).map_err(|source| StoreError::InvalidId {
                        field: "gantt_sections[].task_ids[]",
                        value: raw,
                        source: Box::new(source),
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            Ok(DiagramGanttSectionMeta { section_id, name: section.name, task_ids })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let sequence_messages =
        meta_json
            .sequence_messages
            .into_iter()
            .map(|msg_json| {
                let message_id = ObjectId::new(msg_json.message_id.clone()).map_err(|source| {
                    StoreError::InvalidId {
                        field: "sequence_messages[].message_id",
                        value: msg_json.message_id,
                        source: Box::new(source),
                    }
                })?;
                let from_participant_id = ObjectId::new(msg_json.from_participant_id.clone())
                    .map_err(|source| StoreError::InvalidId {
                        field: "sequence_messages[].from_participant_id",
                        value: msg_json.from_participant_id,
                        source: Box::new(source),
                    })?;
                let to_participant_id =
                    ObjectId::new(msg_json.to_participant_id.clone()).map_err(|source| {
                        StoreError::InvalidId {
                            field: "sequence_messages[].to_participant_id",
                            value: msg_json.to_participant_id,
                            source: Box::new(source),
                        }
                    })?;

                Ok(DiagramSequenceMessageMeta {
                    message_id,
                    from_participant_id,
                    to_participant_id,
                    kind: msg_json.kind.into(),
                    text: msg_json.text,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

    let sequence_blocks = meta_json
        .sequence_blocks
        .into_iter()
        .map(|block_json| {
            let block_id = ObjectId::new(block_json.block_id.clone()).map_err(|source| {
                StoreError::InvalidId {
                    field: "sequence_blocks[].block_id",
                    value: block_json.block_id,
                    source: Box::new(source),
                }
            })?;
            let parent_block_id = block_json
                .parent_block_id
                .map(|raw| {
                    ObjectId::new(raw.clone()).map_err(|source| StoreError::InvalidId {
                        field: "sequence_blocks[].parent_block_id",
                        value: raw,
                        source: Box::new(source),
                    })
                })
                .transpose()?;
            let sections = block_json
                .sections
                .into_iter()
                .map(|section_json| {
                    let section_id =
                        ObjectId::new(section_json.section_id.clone()).map_err(|source| {
                            StoreError::InvalidId {
                                field: "sequence_blocks[].sections[].section_id",
                                value: section_json.section_id,
                                source: Box::new(source),
                            }
                        })?;
                    let message_ids = section_json
                        .message_ids
                        .into_iter()
                        .map(|raw| {
                            ObjectId::new(raw.clone()).map_err(|source| StoreError::InvalidId {
                                field: "sequence_blocks[].sections[].message_ids[]",
                                value: raw,
                                source: Box::new(source),
                            })
                        })
                        .collect::<Result<Vec<_>, StoreError>>()?;
                    Ok(DiagramSequenceSectionMeta {
                        section_id,
                        kind: section_json.kind.into(),
                        header: section_json.header,
                        message_ids,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            Ok(DiagramSequenceBlockMeta {
                block_id,
                kind: block_json.kind.into(),
                header: block_json.header,
                parent_block_id,
                sections,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;

    let flow_node_notes = meta_json
        .flow_node_notes
        .into_iter()
        .map(|(node_id, note)| {
            let node_id =
                ObjectId::new(node_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "flow_node_notes keys",
                    value: node_id,
                    source: Box::new(source),
                })?;
            Ok((node_id, note))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    let sequence_participant_notes = meta_json
        .sequence_participant_notes
        .into_iter()
        .map(|(participant_id, note)| {
            let participant_id =
                ObjectId::new(participant_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "sequence_participant_notes keys",
                    value: participant_id,
                    source: Box::new(source),
                })?;
            Ok((participant_id, note))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    let class_node_notes = meta_json
        .class_node_notes
        .into_iter()
        .map(|(class_id, note)| {
            let class_id =
                ObjectId::new(class_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "class_node_notes keys",
                    value: class_id,
                    source: Box::new(source),
                })?;
            Ok((class_id, note))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    let er_entity_notes = meta_json
        .er_entity_notes
        .into_iter()
        .map(|(entity_id, note)| {
            let entity_id =
                ObjectId::new(entity_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "er_entity_notes keys",
                    value: entity_id,
                    source: Box::new(source),
                })?;
            Ok((entity_id, note))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    let gantt_task_notes = meta_json
        .gantt_task_notes
        .into_iter()
        .map(|(task_id, note)| {
            let task_id =
                ObjectId::new(task_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "gantt_task_notes keys",
                    value: task_id,
                    source: Box::new(source),
                })?;
            Ok((task_id, note))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    let gantt_lane_notes = meta_json
        .gantt_lane_notes
        .into_iter()
        .map(|(lane_id, note)| {
            let lane_id =
                ObjectId::new(lane_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "gantt_lane_notes keys",
                    value: lane_id,
                    source: Box::new(source),
                })?;
            Ok((lane_id, note))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    let flow_node_symbols = meta_json
        .flow_node_symbols
        .into_iter()
        .map(|(node_id, symbol)| {
            let parsed_node_id =
                ObjectId::new(node_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "flow_node_symbols keys",
                    value: node_id,
                    source: Box::new(source),
                })?;
            let stable_symbol_id = symbol.stable_symbol_id;
            let symbol = SymbolAnchor::new(stable_symbol_id.clone(), symbol.repository_id)
                .map_err(|source| StoreError::InvalidSymbolAnchor {
                    field: "flow_node_symbols values",
                    value: stable_symbol_id,
                    source: Box::new(source),
                })?;
            Ok((parsed_node_id, symbol))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    let sequence_participant_symbols = meta_json
        .sequence_participant_symbols
        .into_iter()
        .map(|(participant_id, symbol)| {
            let parsed_participant_id =
                ObjectId::new(participant_id.clone()).map_err(|source| StoreError::InvalidId {
                    field: "sequence_participant_symbols keys",
                    value: participant_id,
                    source: Box::new(source),
                })?;
            let stable_symbol_id = symbol.stable_symbol_id;
            let symbol = SymbolAnchor::new(stable_symbol_id.clone(), symbol.repository_id)
                .map_err(|source| StoreError::InvalidSymbolAnchor {
                    field: "sequence_participant_symbols values",
                    value: stable_symbol_id,
                    source: Box::new(source),
                })?;
            Ok((parsed_participant_id, symbol))
        })
        .collect::<Result<BTreeMap<_, _>, StoreError>>()?;

    Ok(DiagramMeta {
        diagram_id,
        mmd_path,
        stable_id_map,
        xrefs,
        flow_edges,
        class_relations,
        er_relationships,
        gantt_sections,
        sequence_messages,
        sequence_blocks,
        default_symbol_repository_id: meta_json.default_symbol_repository_id,
        flow_node_notes,
        sequence_participant_notes,
        class_node_notes,
        er_entity_notes,
        gantt_task_notes,
        gantt_lane_notes,
        flow_node_symbols,
        sequence_participant_symbols,
    })
}

fn validate_relative_path(field: &'static str, path: &Path) -> Result<(), StoreError> {
    if path.as_os_str().is_empty() {
        return Err(StoreError::InvalidRelativePath {
            field,
            value: path.to_path_buf(),
        });
    }

    if path.is_absolute() {
        return Err(StoreError::InvalidRelativePath {
            field,
            value: path.to_path_buf(),
        });
    }

    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err(StoreError::InvalidRelativePath {
                    field,
                    value: path.to_path_buf(),
                });
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

fn to_relative_path(
    session_dir: &Path,
    path: &Path,
    field: &'static str,
) -> Result<PathBuf, StoreError> {
    let relative = if path.is_absolute() {
        path.strip_prefix(session_dir)
            .map(PathBuf::from)
            .map_err(|_| StoreError::PathOutsideSession {
                session_dir: session_dir.to_path_buf(),
                path: path.to_path_buf(),
            })?
    } else {
        path.to_path_buf()
    };

    validate_relative_path(field, &relative)?;
    Ok(relative)
}

fn create_dir_all_safe(session_dir: &Path, relative: &Path) -> Result<(), StoreError> {
    if relative.as_os_str().is_empty() {
        return Ok(());
    }

    validate_relative_path("dir", relative)?;

    let mut current = session_dir.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            continue;
        };

        current.push(part);

        match fs::symlink_metadata(&current) {
            Ok(md) => {
                if md.file_type().is_symlink() {
                    return Err(StoreError::SymlinkRefused { path: current });
                }
                if !md.is_dir() {
                    return Err(StoreError::Io {
                        path: current,
                        source: io::Error::new(io::ErrorKind::AlreadyExists, "expected directory"),
                    });
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|source| StoreError::Io {
                    path: current.clone(),
                    source,
                })?;
            }
            Err(source) => {
                return Err(StoreError::Io {
                    path: current,
                    source,
                })
            }
        }
    }

    Ok(())
}

fn rename_overwrite(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        match fs::rename(from, to) {
            Ok(()) => Ok(()),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
                ) =>
            {
                let _ = fs::remove_file(to);
                fs::rename(from, to)
            }
            Err(err) => Err(err),
        }
    }

    #[cfg(not(windows))]
    {
        fs::rename(from, to)
    }
}

/// Refuse if `path` exists and is a symlink. Missing paths are allowed.
fn refuse_if_symlink(path: &Path) -> Result<(), StoreError> {
    match fs::symlink_metadata(path) {
        Ok(md) if md.file_type().is_symlink() => Err(StoreError::SymlinkRefused {
            path: path.to_path_buf(),
        }),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(StoreError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Ensure every component of `relative` under `session_dir` is a real directory (not a symlink).
///
/// Call immediately before open/rename so a concurrent swap of a parent to a symlink after
/// [`create_dir_all_safe`] cannot redirect session writes outside the session root.
fn ensure_real_dirs_under_session(session_dir: &Path, relative: &Path) -> Result<(), StoreError> {
    if relative.as_os_str().is_empty() {
        return Ok(());
    }

    let mut current = session_dir.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            continue;
        };

        current.push(part);

        match fs::symlink_metadata(&current) {
            Ok(md) => {
                if md.file_type().is_symlink() {
                    return Err(StoreError::SymlinkRefused { path: current });
                }
                if !md.is_dir() {
                    return Err(StoreError::Io {
                        path: current,
                        source: io::Error::new(io::ErrorKind::AlreadyExists, "expected directory"),
                    });
                }
            }
            Err(source) => {
                return Err(StoreError::Io {
                    path: current,
                    source,
                });
            }
        }
    }

    Ok(())
}

/// Re-check parents and leaf immediately before a filesystem mutation that could escape.
fn revalidate_session_write_path(
    session_dir: &Path,
    parent_rel: &Path,
    path: &Path,
) -> Result<(), StoreError> {
    ensure_real_dirs_under_session(session_dir, parent_rel)?;
    refuse_if_symlink(path)
}

fn write_atomic_in_session(
    session_dir: &Path,
    path: &Path,
    contents: &[u8],
    durability: WriteDurability,
) -> Result<(), StoreError> {
    write_atomic_in_session_inner(session_dir, path, contents, durability, true)
}

fn write_atomic_in_session_if_session_dir_exists(
    session_dir: &Path,
    path: &Path,
    contents: &[u8],
    durability: WriteDurability,
) -> Result<(), StoreError> {
    write_atomic_in_session_inner(session_dir, path, contents, durability, false)
}

fn write_atomic_in_session_inner(
    session_dir: &Path,
    path: &Path,
    contents: &[u8],
    durability: WriteDurability,
    create_root: bool,
) -> Result<(), StoreError> {
    if create_root {
        fs::create_dir_all(session_dir).map_err(|source| StoreError::Io {
            path: session_dir.to_path_buf(),
            source,
        })?;
    } else {
        match fs::metadata(session_dir) {
            Ok(md) if md.is_dir() => {}
            Ok(_) => return Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(StoreError::Io {
                    path: session_dir.to_path_buf(),
                    source,
                })
            }
        }
    }

    let relative = to_relative_path(session_dir, path, "path")?;
    let parent_rel = relative.parent().unwrap_or_else(|| Path::new(""));
    create_dir_all_safe(session_dir, parent_rel)?;

    let Some(parent) = path.parent() else {
        return Err(StoreError::Io {
            path: path.to_path_buf(),
            source: io::Error::other("path has no parent"),
        });
    };

    let Some(file_name) = path.file_name() else {
        return Err(StoreError::Io {
            path: path.to_path_buf(),
            source: io::Error::other("path has no file name"),
        });
    };

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = parent.join(format!(
        ".nereid.tmp.{}.{}",
        file_name.to_string_lossy(),
        nanos
    ));

    // Re-validate every parent component + leaf immediately before open so a concurrent
    // writer cannot replace a parent directory with a symlink after create_dir_all_safe.
    revalidate_session_write_path(session_dir, parent_rel, path)?;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)
        .map_err(|source| StoreError::Io {
            path: tmp_path.clone(),
            source,
        })?;

    file.write_all(contents).map_err(|source| StoreError::Io {
        path: tmp_path.clone(),
        source,
    })?;

    if durability == WriteDurability::Durable {
        file.sync_all().map_err(|source| StoreError::Io {
            path: tmp_path.clone(),
            source,
        })?;
    }
    drop(file);

    // Re-validate again immediately before rename (same TOCTOU class as open).
    if let Err(err) = revalidate_session_write_path(session_dir, parent_rel, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err);
    }

    if let Err(source) = rename_overwrite(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(StoreError::Io {
            path: path.to_path_buf(),
            source,
        });
    }

    if durability == WriteDurability::Durable {
        #[cfg(unix)]
        {
            let dir = fs::File::open(parent).map_err(|source| StoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
            dir.sync_all().map_err(|source| StoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    Ok(())
}

fn refresh_xref_statuses(session: &mut Session) {
    let updates = session
        .xrefs()
        .iter()
        .map(|(xref_id, xref)| {
            let from_exists = session.object_ref_exists(xref.from());
            let to_exists = session.object_ref_exists(xref.to());
            let status = ModelXRefStatus::from_flags(!from_exists, !to_exists);
            (xref_id.clone(), status)
        })
        .collect::<Vec<_>>();

    for (xref_id, status) in updates {
        if let Some(xref) = session.xrefs_mut().get_mut(&xref_id) {
            xref.set_status(status);
        }
    }
}
