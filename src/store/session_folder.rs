// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fmt;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::format::mermaid::{
    export_flowchart, export_sequence_diagram, parse_flowchart, parse_sequence_diagram,
    MermaidFlowchartExportError, MermaidFlowchartParseError, MermaidSequenceExportError,
    MermaidSequenceParseError,
};
use crate::layout::{layout_flowchart, layout_sequence, FlowchartLayoutError, SequenceLayoutError};
use crate::model::{
    Diagram, DiagramAst, DiagramId, DiagramKind, FlowEdge, FlowNode, FlowchartAst, IdError,
    ObjectId, ObjectRef, ParseObjectRefError, SequenceAst, SequenceMessage, SequenceMessageKind,
    Session, SessionId, Walkthrough, WalkthroughEdge, WalkthroughId, WalkthroughNode,
    WalkthroughNodeId, XRef, XRefId, XRefStatus as ModelXRefStatus,
};
use crate::render::{
    render_flowchart_unicode, render_sequence_unicode, render_walkthrough_unicode,
    FlowchartRenderError, SequenceRenderError, WalkthroughRenderError,
};

const SESSION_META_FILENAME: &str = "nereid-session.meta.json";
const LEGACY_SESSION_META_FILENAME: &str = "session.meta.json";

#[derive(Debug)]
enum AsciiExportTask {
    Diagram {
        session_dir: PathBuf,
        mmd_path: PathBuf,
        text_path: PathBuf,
        durability: WriteDurability,
        ast: DiagramAst,
    },
    Walkthrough {
        session_dir: PathBuf,
        json_path: PathBuf,
        text_path: PathBuf,
        durability: WriteDurability,
        walkthrough: Walkthrough,
    },
}

impl AsciiExportTask {
    fn output_path(&self) -> &Path {
        match self {
            Self::Diagram { text_path, .. } | Self::Walkthrough { text_path, .. } => text_path,
        }
    }

    fn session_dir(&self) -> &Path {
        match self {
            Self::Diagram { session_dir, .. } | Self::Walkthrough { session_dir, .. } => {
                session_dir
            }
        }
    }
}

#[derive(Debug, Default)]
struct AsciiExportState {
    pending: HashMap<PathBuf, AsciiExportTask>,
    queue: VecDeque<PathBuf>,
    in_flight_session_dir: Option<PathBuf>,
}

#[derive(Debug)]
struct AsciiExportInner {
    state: Mutex<AsciiExportState>,
    cv: Condvar,
}

#[derive(Debug)]
struct AsciiExportManager {
    inner: Arc<AsciiExportInner>,
}

impl AsciiExportManager {
    fn new() -> Self {
        let inner = Arc::new(AsciiExportInner {
            state: Mutex::new(AsciiExportState::default()),
            cv: Condvar::new(),
        });

        std::thread::Builder::new()
            .name("nereid-ascii-export".to_owned())
            .spawn({
                let inner = inner.clone();
                move || Self::run_worker(inner)
            })
            .expect("spawn ascii export worker thread");

        Self { inner }
    }

    fn schedule(&self, task: AsciiExportTask) {
        let output_path = task.output_path().to_path_buf();

        let mut state = self.inner.state.lock().expect("ascii export lock poisoned");
        if state.pending.contains_key(&output_path) {
            state.pending.insert(output_path, task);
            return;
        }

        state.pending.insert(output_path.clone(), task);
        state.queue.push_back(output_path);
        self.inner.cv.notify_one();
    }

    fn cancel(&self, output_path: &Path) {
        let mut state = self.inner.state.lock().expect("ascii export lock poisoned");
        state.pending.remove(output_path);
    }

    fn flush_session_dir(&self, session_dir: &Path) {
        let mut state = self.inner.state.lock().expect("ascii export lock poisoned");
        while state
            .in_flight_session_dir
            .as_deref()
            .is_some_and(|active| active == session_dir)
            || state
                .pending
                .values()
                .any(|task| task.session_dir() == session_dir)
        {
            state = self.inner.cv.wait(state).expect("ascii export cv poisoned");
        }
    }

    fn run_worker(inner: Arc<AsciiExportInner>) {
        loop {
            let task = {
                let mut state = inner.state.lock().expect("ascii export lock poisoned");

                loop {
                    if let Some(output_path) = state.queue.pop_front() {
                        if let Some(task) = state.pending.remove(&output_path) {
                            state.in_flight_session_dir = Some(task.session_dir().to_path_buf());
                            break task;
                        }
                    }

                    state = inner.cv.wait(state).expect("ascii export cv poisoned");
                }
            };

            match task {
                AsciiExportTask::Diagram {
                    session_dir,
                    mmd_path,
                    text_path,
                    durability,
                    ast,
                } => {
                    if !mmd_path.is_file() {
                        // Likely removed or cleaned up; avoid resurrecting temp session dirs.
                    } else if let Some(mut text) = match ast {
                        DiagramAst::Sequence(ast) => match layout_sequence(&ast) {
                            Ok(layout) => render_sequence_unicode(&ast, &layout).ok(),
                            Err(_) => None,
                        },
                        DiagramAst::Flowchart(ast) => match layout_flowchart(&ast) {
                            Ok(layout) => render_flowchart_unicode(&ast, &layout).ok(),
                            Err(_) => None,
                        },
                    } {
                        if !text.ends_with('\n') {
                            text.push('\n');
                        }
                        let _ = write_atomic_in_session_if_session_dir_exists(
                            &session_dir,
                            &text_path,
                            text.as_bytes(),
                            durability,
                        );
                    }
                }
                AsciiExportTask::Walkthrough {
                    session_dir,
                    json_path,
                    text_path,
                    durability,
                    walkthrough,
                } => {
                    if !json_path.is_file() {
                        // Walkthrough was likely deleted/garbage-collected.
                    } else if let Ok(mut text) = render_walkthrough_unicode(&walkthrough) {
                        if !text.ends_with('\n') {
                            text.push('\n');
                        }
                        let _ = write_atomic_in_session_if_session_dir_exists(
                            &session_dir,
                            &text_path,
                            text.as_bytes(),
                            durability,
                        );
                    }
                }
            }

            let mut state = inner.state.lock().expect("ascii export lock poisoned");
            state.in_flight_session_dir = None;
            inner.cv.notify_all();
        }
    }
}

static ASCII_EXPORTS: OnceLock<AsciiExportManager> = OnceLock::new();

fn ascii_exports() -> &'static AsciiExportManager {
    ASCII_EXPORTS.get_or_init(AsciiExportManager::new)
}

#[derive(Debug)]
pub enum StoreError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    MermaidSequenceParse {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<MermaidSequenceParseError>,
    },
    MermaidFlowchartParse {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<MermaidFlowchartParseError>,
    },
    MermaidSequenceExport {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<MermaidSequenceExportError>,
    },
    MermaidFlowchartExport {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<MermaidFlowchartExportError>,
    },
    SequenceLayout {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<SequenceLayoutError>,
    },
    SequenceRender {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<SequenceRenderError>,
    },
    FlowchartLayout {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<FlowchartLayoutError>,
    },
    FlowchartRender {
        diagram_id: DiagramId,
        path: PathBuf,
        source: Box<FlowchartRenderError>,
    },
    WalkthroughRender {
        walkthrough_id: WalkthroughId,
        path: PathBuf,
        source: Box<WalkthroughRenderError>,
    },
    InvalidId {
        field: &'static str,
        value: String,
        source: Box<IdError>,
    },
    InvalidObjectRef {
        field: &'static str,
        value: String,
        source: Box<ParseObjectRefError>,
    },
    InvalidRelativePath {
        field: &'static str,
        value: PathBuf,
    },
    PathOutsideSession {
        session_dir: PathBuf,
        path: PathBuf,
    },
    SymlinkRefused {
        path: PathBuf,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "io error at {path:?}: {source}"),
            Self::Json { path, source } => write!(f, "json error at {path:?}: {source}"),
            Self::MermaidSequenceParse {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot parse Mermaid sequence diagram {diagram_id} from {path:?}: {source}"
            ),
            Self::MermaidFlowchartParse {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot parse Mermaid flowchart diagram {diagram_id} from {path:?}: {source}"
            ),
            Self::MermaidSequenceExport {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot export Mermaid sequence diagram {diagram_id} to {path:?}: {source}"
            ),
            Self::MermaidFlowchartExport {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot export Mermaid flowchart diagram {diagram_id} to {path:?}: {source}"
            ),
            Self::SequenceLayout {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot layout sequence diagram {diagram_id} for unicode export to {path:?}: {source}"
            ),
            Self::SequenceRender {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot render sequence diagram {diagram_id} for unicode export to {path:?}: {source}"
            ),
            Self::FlowchartLayout {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot layout flowchart diagram {diagram_id} for unicode export to {path:?}: {source}"
            ),
            Self::FlowchartRender {
                diagram_id,
                path,
                source,
            } => write!(
                f,
                "cannot render flowchart diagram {diagram_id} for unicode export to {path:?}: {source}"
            ),
            Self::WalkthroughRender {
                walkthrough_id,
                path,
                source,
            } => write!(
                f,
                "cannot render walkthrough {walkthrough_id} for unicode export to {path:?}: {source}"
            ),
            Self::InvalidId {
                field,
                value,
                source,
            } => write!(f, "invalid id for {field}: {value:?}: {source}"),
            Self::InvalidObjectRef {
                field,
                value,
                source,
            } => write!(f, "invalid object ref for {field}: {value:?}: {source}"),
            Self::InvalidRelativePath { field, value } => {
                write!(f, "invalid relative path for {field}: {value:?}")
            }
            Self::PathOutsideSession { session_dir, path } => write!(
                f,
                "path is outside session dir: session_dir={session_dir:?} path={path:?}"
            ),
            Self::SymlinkRefused { path } => {
                write!(f, "refusing to write through symlink at {path:?}")
            }
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::MermaidSequenceParse { source, .. } => Some(source),
            Self::MermaidFlowchartParse { source, .. } => Some(source),
            Self::MermaidSequenceExport { source, .. } => Some(source),
            Self::MermaidFlowchartExport { source, .. } => Some(source),
            Self::SequenceLayout { source, .. } => Some(source),
            Self::SequenceRender { source, .. } => Some(source),
            Self::FlowchartLayout { source, .. } => Some(source),
            Self::FlowchartRender { source, .. } => Some(source),
            Self::WalkthroughRender { source, .. } => Some(source),
            Self::InvalidId { source, .. } => Some(source),
            Self::InvalidObjectRef { source, .. } => Some(source),
            Self::InvalidRelativePath { .. } => None,
            Self::PathOutsideSession { .. } => None,
            Self::SymlinkRefused { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMeta {
    pub session_id: SessionId,
    pub active_diagram_id: Option<DiagramId>,
    pub active_walkthrough_id: Option<WalkthroughId>,
    pub walkthrough_ids: Option<Vec<WalkthroughId>>,
    pub diagrams: Vec<SessionMetaDiagram>,
    pub xrefs: Vec<SessionXRef>,
    pub selected_object_refs: Vec<ObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMetaDiagram {
    pub diagram_id: DiagramId,
    pub name: String,
    pub kind: DiagramKind,
    pub mmd_path: PathBuf,
    pub rev: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionXRef {
    pub xref_id: XRefId,
    pub from: ObjectRef,
    pub to: ObjectRef,
    pub kind: String,
    pub label: Option<String>,
    pub status: ModelXRefStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramMeta {
    pub diagram_id: DiagramId,
    pub mmd_path: PathBuf,
    pub stable_id_map: DiagramStableIdMap,
    pub xrefs: Vec<DiagramXRef>,
    pub flow_edges: Vec<DiagramFlowEdgeMeta>,
    pub sequence_messages: Vec<DiagramSequenceMessageMeta>,
    pub flow_node_notes: BTreeMap<ObjectId, String>,
    pub sequence_participant_notes: BTreeMap<ObjectId, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagramStableIdMap {
    pub by_mermaid_id: BTreeMap<String, String>,
    pub by_name: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramXRef {
    pub xref_id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
    pub status: XRefStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramFlowEdgeMeta {
    pub edge_id: ObjectId,
    pub from_node_id: ObjectId,
    pub to_node_id: ObjectId,
    pub label: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagramSequenceMessageMeta {
    pub message_id: ObjectId,
    pub from_participant_id: ObjectId,
    pub to_participant_id: ObjectId,
    pub kind: SequenceMessageKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefStatus {
    Ok,
    DanglingFrom,
    DanglingTo,
    DanglingBoth,
}

#[derive(Debug, Clone)]
pub struct SessionFolder {
    root: PathBuf,
    durability: WriteDurability,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WriteDurability {
    /// Fast, best-effort persistence.
    ///
    /// - Writes a temp file and renames atomically into place.
    /// - Does not perform per-file fsync/sync.
    #[default]
    BestEffort,

    /// Slower, best-effort durability.
    ///
    /// Attempts to flush written file contents and rename operations to stable storage where
    /// possible. Exact guarantees are platform/filesystem-dependent.
    Durable,
}

fn encode_persisted_id_segment(segment: &str) -> String {
    if !needs_windows_safe_filename_segment_encoding(segment) {
        return segment.to_owned();
    }

    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(1 + segment.len().saturating_mul(2));
    out.push('~');
    for &b in segment.as_bytes() {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn needs_windows_safe_filename_segment_encoding(segment: &str) -> bool {
    if segment.starts_with('~') {
        return true;
    }
    if segment == "." || segment == ".." {
        return true;
    }
    if segment.ends_with(' ') || segment.ends_with('.') {
        return true;
    }

    let trimmed = segment.trim_end_matches([' ', '.']);
    let base = trimmed.split('.').next().unwrap_or(trimmed);
    if is_windows_device_name(base) {
        return true;
    }

    for ch in segment.chars() {
        if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            return true;
        }
        if ch <= '\u{1f}' || ch == '\u{7f}' {
            return true;
        }
    }

    false
}

fn is_windows_device_name(base: &str) -> bool {
    let base = base.to_ascii_uppercase();
    match base.as_str() {
        "CON" | "PRN" | "AUX" | "NUL" => true,
        _ => {
            if let Some(num) = base.strip_prefix("COM") {
                matches!(num, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            } else if let Some(num) = base.strip_prefix("LPT") {
                matches!(num, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            } else {
                false
            }
        }
    }
}

impl SessionFolder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            durability: WriteDurability::default(),
        }
    }

    pub fn with_durability(mut self, durability: WriteDurability) -> Self {
        self.durability = durability;
        self
    }

    pub fn durability(&self) -> WriteDurability {
        self.durability
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn meta_path(&self) -> PathBuf {
        self.root.join(SESSION_META_FILENAME)
    }

    fn legacy_meta_path(&self) -> PathBuf {
        self.root.join(LEGACY_SESSION_META_FILENAME)
    }

    pub fn default_diagram_mmd_path(&self, diagram_id: &DiagramId) -> PathBuf {
        let file_stem = encode_persisted_id_segment(diagram_id.as_str());
        self.root.join("diagrams").join(format!("{file_stem}.mmd"))
    }

    /// Returns the path for the deterministic text render export.
    ///
    /// Note: The `.ascii.txt` extension is a legacy filename; the contents may include Unicode.
    ///
    /// This file is generated asynchronously as a best-effort export. It may lag behind `.mmd`
    /// updates during rapid edits.
    pub fn diagram_ascii_path(&self, mmd_path: &Path) -> Result<PathBuf, StoreError> {
        let relative_mmd_path = to_relative_path(self.root(), mmd_path, "mmd_path")?;
        let relative_ascii_path = relative_mmd_path.with_extension("ascii.txt");
        Ok(self.root.join(relative_ascii_path))
    }

    pub fn diagram_meta_path(&self, mmd_path: &Path) -> Result<PathBuf, StoreError> {
        let relative_mmd_path = to_relative_path(self.root(), mmd_path, "mmd_path")?;
        let relative_meta_path = relative_mmd_path.with_extension("meta.json");
        Ok(self.root.join(relative_meta_path))
    }

    pub fn walkthrough_json_path(&self, walkthrough_id: &WalkthroughId) -> PathBuf {
        let file_stem = encode_persisted_id_segment(walkthrough_id.as_str());
        self.root
            .join("walkthroughs")
            .join(format!("{file_stem}.wt.json"))
    }

    fn legacy_walkthrough_json_path(&self, walkthrough_id: &WalkthroughId) -> PathBuf {
        self.root
            .join("walkthroughs")
            .join(format!("{}.wt.json", walkthrough_id.as_str()))
    }

    /// Returns the path for the deterministic text render export.
    ///
    /// Note: The `.ascii.txt` extension is a legacy filename; the contents may include Unicode.
    ///
    /// This file is generated asynchronously as a best-effort export. It may lag behind `.wt.json`
    /// updates during rapid edits.
    pub fn walkthrough_ascii_path(&self, walkthrough_id: &WalkthroughId) -> PathBuf {
        let file_stem = encode_persisted_id_segment(walkthrough_id.as_str());
        self.root
            .join("walkthroughs")
            .join(format!("{file_stem}.ascii.txt"))
    }

    pub fn flush_ascii_exports(&self) {
        ascii_exports().flush_session_dir(self.root());
    }

    fn initial_session_id(&self) -> SessionId {
        let candidate = self
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .map(|name| format!("s:{name}"))
            .unwrap_or_else(|| "s:session".to_owned());

        SessionId::new(candidate).unwrap_or_else(|_| {
            SessionId::new("s:session").expect("hard-coded fallback session id is valid")
        })
    }

    fn initial_session(&self) -> Session {
        let mut session = Session::new(self.initial_session_id());
        let diagram_id = DiagramId::new("flow").expect("hard-coded initial diagram id is valid");
        let node_id = ObjectId::new("n:hello").expect("hard-coded initial node id is valid");

        let mut ast = FlowchartAst::default();
        ast.nodes_mut().insert(node_id, FlowNode::new("Hello"));

        let diagram = Diagram::new(diagram_id.clone(), "Flow", DiagramAst::Flowchart(ast));
        session.diagrams_mut().insert(diagram_id.clone(), diagram);
        session.set_active_diagram_id(Some(diagram_id));
        session
    }

    pub fn load_or_init_session(&self) -> Result<Session, StoreError> {
        match self.load_session() {
            Ok(session) => Ok(session),
            Err(StoreError::Io { path, source })
                if source.kind() == io::ErrorKind::NotFound && path == self.meta_path() =>
            {
                let session = self.initial_session();
                self.save_session(&session)?;
                Ok(session)
            }
            Err(err) => Err(err),
        }
    }

    pub fn save_session(&self, session: &Session) -> Result<(), StoreError> {
        #[derive(Debug, Deserialize)]
        struct WalkthroughRevJson {
            #[serde(default)]
            rev: u64,
        }

        let existing_meta = match self.load_meta() {
            Ok(meta) => Some(meta),
            Err(StoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => None,
            Err(err) => return Err(err),
        };

        let mut existing_diagram_revs = BTreeMap::<DiagramId, u64>::new();
        let mut existing_walkthrough_id_set = Option::<BTreeSet<WalkthroughId>>::None;

        if let Some(meta) = existing_meta.as_ref() {
            for diagram in &meta.diagrams {
                existing_diagram_revs.insert(diagram.diagram_id.clone(), diagram.rev);
            }
            if let Some(ids) = meta.walkthrough_ids.as_ref() {
                existing_walkthrough_id_set = Some(ids.iter().cloned().collect());
            }
        }

        let read_walkthrough_rev = |path: &Path| -> Option<u64> {
            let wt_str = fs::read_to_string(path).ok()?;
            let wt_rev: WalkthroughRevJson = serde_json::from_str(&wt_str).ok()?;
            Some(wt_rev.rev)
        };

        let mut meta = SessionMeta {
            session_id: session.session_id().clone(),
            active_diagram_id: session.active_diagram_id().cloned(),
            active_walkthrough_id: session.active_walkthrough_id().cloned(),
            walkthrough_ids: Some(Vec::new()),
            diagrams: Vec::new(),
            xrefs: Vec::new(),
            selected_object_refs: session.selected_object_refs().iter().cloned().collect(),
        };

        for (diagram_id, diagram) in session.diagrams() {
            let mmd_path = self.default_diagram_mmd_path(diagram_id);
            let ascii_path = self.diagram_ascii_path(&mmd_path)?;
            let meta_path = self.diagram_meta_path(&mmd_path)?;

            let diagram_rev_unchanged = existing_diagram_revs
                .get(diagram_id)
                .copied()
                .is_some_and(|rev| rev == diagram.rev())
                && mmd_path.is_file()
                && meta_path.is_file();

            if !diagram_rev_unchanged {
                export_diagram_mmd(self, diagram, &mmd_path)?;

                let flow_edges = match diagram.ast() {
                    DiagramAst::Flowchart(ast) => ast
                        .edges()
                        .iter()
                        .map(|(edge_id, edge)| DiagramFlowEdgeMeta {
                            edge_id: edge_id.clone(),
                            from_node_id: edge.from_node_id().clone(),
                            to_node_id: edge.to_node_id().clone(),
                            label: edge.label().map(ToOwned::to_owned),
                            style: edge.style().map(ToOwned::to_owned),
                        })
                        .collect(),
                    DiagramAst::Sequence(_) => Vec::new(),
                };

                let sequence_messages = match diagram.ast() {
                    DiagramAst::Sequence(ast) => ast
                        .messages_in_order()
                        .into_iter()
                        .map(|msg| DiagramSequenceMessageMeta {
                            message_id: msg.message_id().clone(),
                            from_participant_id: msg.from_participant_id().clone(),
                            to_participant_id: msg.to_participant_id().clone(),
                            kind: msg.kind(),
                            text: msg.text().to_owned(),
                        })
                        .collect(),
                    DiagramAst::Flowchart(_) => Vec::new(),
                };

                let flow_node_notes = match diagram.ast() {
                    DiagramAst::Flowchart(ast) => ast
                        .nodes()
                        .iter()
                        .filter_map(|(node_id, node)| {
                            node.note().map(|note| (node_id.clone(), note.to_owned()))
                        })
                        .collect(),
                    DiagramAst::Sequence(_) => BTreeMap::new(),
                };

                let sequence_participant_notes = match diagram.ast() {
                    DiagramAst::Sequence(ast) => ast
                        .participants()
                        .iter()
                        .filter_map(|(participant_id, participant)| {
                            participant
                                .note()
                                .map(|note| (participant_id.clone(), note.to_owned()))
                        })
                        .collect(),
                    DiagramAst::Flowchart(_) => BTreeMap::new(),
                };

                self.save_diagram_meta(&DiagramMeta {
                    diagram_id: diagram_id.clone(),
                    mmd_path: mmd_path.clone(),
                    stable_id_map: stable_id_map_from_ast(diagram.ast()),
                    xrefs: Vec::new(),
                    flow_edges,
                    sequence_messages,
                    flow_node_notes,
                    sequence_participant_notes,
                })?;
            }

            if !diagram_rev_unchanged || !ascii_path.is_file() {
                self.schedule_diagram_ascii_export(&mmd_path, diagram)?;
            }

            meta.diagrams.push(SessionMetaDiagram {
                diagram_id: diagram_id.clone(),
                name: diagram.name().to_owned(),
                kind: diagram.kind(),
                mmd_path,
                rev: diagram.rev(),
            });
        }

        for (xref_id, xref) in session.xrefs() {
            meta.xrefs.push(SessionXRef {
                xref_id: xref_id.clone(),
                from: xref.from().clone(),
                to: xref.to().clone(),
                kind: xref.kind().to_owned(),
                label: xref.label().map(ToOwned::to_owned),
                status: xref.status(),
            });
        }

        let mut walkthrough_ids = session.walkthroughs().keys().cloned().collect::<Vec<_>>();
        walkthrough_ids.sort();
        let current_walkthrough_id_set: BTreeSet<_> = walkthrough_ids.iter().cloned().collect();
        let skip_walkthrough_gc = existing_walkthrough_id_set
            .as_ref()
            .is_some_and(|prev| prev == &current_walkthrough_id_set);
        meta.walkthrough_ids = Some(walkthrough_ids.clone());

        for walkthrough_id in walkthrough_ids {
            let walkthrough = session
                .walkthroughs()
                .get(&walkthrough_id)
                .expect("walkthrough id listed in walkthrough_ids");
            let json_path = self.walkthrough_json_path(&walkthrough_id);
            let ascii_path = self.walkthrough_ascii_path(&walkthrough_id);
            let rev_matches = json_path.is_file()
                && read_walkthrough_rev(&json_path).is_some_and(|rev| rev == walkthrough.rev());

            if !rev_matches {
                self.save_walkthrough(walkthrough)?;
            } else if !ascii_path.is_file() {
                self.schedule_walkthrough_ascii_export(walkthrough)?;
            }
        }

        if !skip_walkthrough_gc {
            if let Some(walkthrough_ids) = meta.walkthrough_ids.as_deref() {
                self.garbage_collect_walkthrough_files(walkthrough_ids)?;
            }
        }

        self.save_meta(&meta)?;
        Ok(())
    }

    fn garbage_collect_walkthrough_files(
        &self,
        walkthrough_ids: &[WalkthroughId],
    ) -> Result<(), StoreError> {
        let mut keep_stems = std::collections::BTreeSet::<String>::new();
        for id in walkthrough_ids {
            keep_stems.insert(id.to_string());
            keep_stems.insert(encode_persisted_id_segment(id.as_str()));
        }

        let walkthroughs_dir = self.root.join("walkthroughs");
        let entries = match fs::read_dir(&walkthroughs_dir) {
            Ok(entries) => entries,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(source) => {
                return Err(StoreError::Io {
                    path: walkthroughs_dir,
                    source,
                });
            }
        };

        for entry in entries.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            let Some(walkthrough_id) = file_name
                .strip_suffix(".wt.json")
                .or_else(|| file_name.strip_suffix(".ascii.txt"))
            else {
                continue;
            };

            if keep_stems.contains(walkthrough_id) {
                continue;
            }

            ascii_exports().cancel(&path);

            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(StoreError::Io { path, source });
                }
            }
        }

        Ok(())
    }

    pub fn load_session(&self) -> Result<Session, StoreError> {
        let meta = self.load_meta()?;

        let mut session = Session::new(meta.session_id);
        session.set_active_diagram_id(meta.active_diagram_id);
        session.set_active_walkthrough_id(meta.active_walkthrough_id);
        session.set_selected_object_refs(meta.selected_object_refs.into_iter().collect());
        let walkthrough_ids = meta.walkthrough_ids.clone();

        for diagram_meta in meta.diagrams {
            let diagram_id = diagram_meta.diagram_id;
            let mmd_path = diagram_meta.mmd_path;
            let mmd = fs::read_to_string(&mmd_path).map_err(|source| StoreError::Io {
                path: mmd_path.clone(),
                source,
            })?;

            let sidecar = match self.load_diagram_meta(&mmd_path) {
                Ok(sidecar) => Some(sidecar),
                Err(StoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                    None
                }
                Err(err) => return Err(err),
            };

            let mut ast = match diagram_meta.kind {
                DiagramKind::Sequence => {
                    DiagramAst::Sequence(parse_sequence_diagram(&mmd).map_err(|source| {
                        StoreError::MermaidSequenceParse {
                            diagram_id: diagram_id.clone(),
                            path: mmd_path.clone(),
                            source: Box::new(source),
                        }
                    })?)
                }
                DiagramKind::Flowchart => {
                    DiagramAst::Flowchart(parse_flowchart(&mmd).map_err(|source| {
                        StoreError::MermaidFlowchartParse {
                            diagram_id: diagram_id.clone(),
                            path: mmd_path.clone(),
                            source: Box::new(source),
                        }
                    })?)
                }
            };

            if let Some(sidecar) = sidecar.as_ref() {
                match &mut ast {
                    DiagramAst::Flowchart(flow_ast) => {
                        reconcile_flowchart_nodes(flow_ast, sidecar);
                        reconcile_flowchart_edges(flow_ast, sidecar);
                        reconcile_flowchart_notes(flow_ast, sidecar);
                    }
                    DiagramAst::Sequence(seq_ast) => {
                        reconcile_sequence_participants(seq_ast, sidecar);
                        reconcile_sequence_messages(seq_ast, sidecar);
                        reconcile_sequence_participant_notes(seq_ast, sidecar);
                    }
                }
            }

            let mut diagram = Diagram::new(diagram_id.clone(), diagram_meta.name, ast);
            diagram.set_rev(diagram_meta.rev);
            session.diagrams_mut().insert(diagram_id, diagram);
        }

        for xref_meta in meta.xrefs {
            let mut xref = XRef::new(
                xref_meta.from,
                xref_meta.to,
                xref_meta.kind,
                xref_meta.status,
            );
            xref.set_label(xref_meta.label);
            session.xrefs_mut().insert(xref_meta.xref_id, xref);
        }
        refresh_xref_statuses(&mut session);

        match walkthrough_ids {
            Some(walkthrough_ids) => {
                for walkthrough_id in walkthrough_ids {
                    let walkthrough = self.load_walkthrough(&walkthrough_id)?;
                    session
                        .walkthroughs_mut()
                        .insert(walkthrough_id, walkthrough);
                }
            }
            None => {
                let walkthroughs_dir = self.root.join("walkthroughs");
                match fs::read_dir(&walkthroughs_dir) {
                    Ok(entries) => {
                        let mut wt_paths = entries
                            .filter_map(|entry| entry.ok())
                            .map(|entry| entry.path())
                            .filter(|path| path.is_file())
                            .filter(|path| path.to_string_lossy().ends_with(".wt.json"))
                            .collect::<Vec<_>>();
                        wt_paths.sort();

                        for wt_path in wt_paths {
                            let wt_str =
                                fs::read_to_string(&wt_path).map_err(|source| StoreError::Io {
                                    path: wt_path.clone(),
                                    source,
                                })?;

                            let wt_json: WalkthroughJson =
                                serde_json::from_str(&wt_str).map_err(|source| {
                                    StoreError::Json {
                                        path: wt_path.clone(),
                                        source,
                                    }
                                })?;

                            let walkthrough = walkthrough_from_json(wt_json)?;
                            session
                                .walkthroughs_mut()
                                .insert(walkthrough.walkthrough_id().clone(), walkthrough);
                        }
                    }
                    Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => {
                        return Err(StoreError::Io {
                            path: walkthroughs_dir,
                            source,
                        });
                    }
                }
            }
        }

        Ok(session)
    }

    pub fn load_meta(&self) -> Result<SessionMeta, StoreError> {
        let meta_path = self.meta_path();
        let (meta_path, meta_str) = match fs::read_to_string(&meta_path) {
            Ok(meta_str) => (meta_path, meta_str),
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                let legacy_path = self.legacy_meta_path();
                match fs::read_to_string(&legacy_path) {
                    Ok(meta_str) => (legacy_path, meta_str),
                    Err(legacy_source) if legacy_source.kind() == io::ErrorKind::NotFound => {
                        return Err(StoreError::Io {
                            path: meta_path,
                            source,
                        });
                    }
                    Err(legacy_source) => {
                        return Err(StoreError::Io {
                            path: legacy_path,
                            source: legacy_source,
                        });
                    }
                }
            }
            Err(source) => {
                return Err(StoreError::Io {
                    path: meta_path.clone(),
                    source,
                });
            }
        };

        let meta_json: SessionMetaJson =
            serde_json::from_str(&meta_str).map_err(|source| StoreError::Json {
                path: meta_path.clone(),
                source,
            })?;

        session_meta_from_json(self.root(), meta_json)
    }

    pub fn save_meta(&self, meta: &SessionMeta) -> Result<(), StoreError> {
        fs::create_dir_all(self.root()).map_err(|source| StoreError::Io {
            path: self.root.clone(),
            source,
        })?;

        let meta_path = self.meta_path();
        let meta_json = session_meta_to_json(self.root(), meta)?;
        let meta_str =
            serde_json::to_string_pretty(&meta_json).map_err(|source| StoreError::Json {
                path: meta_path.clone(),
                source,
            })?;

        write_atomic_in_session(
            self.root(),
            &meta_path,
            format!("{meta_str}\n").as_bytes(),
            self.durability,
        )?;

        let legacy_path = self.legacy_meta_path();
        if legacy_path != meta_path {
            match fs::remove_file(&legacy_path) {
                Ok(()) => {}
                Err(source) if source.kind() == io::ErrorKind::NotFound => {}
                Err(_source) => {}
            }
        }

        Ok(())
    }

    pub fn save_selected_object_refs(&self, session: &Session) -> Result<(), StoreError> {
        match self.load_meta() {
            Ok(mut meta) => {
                meta.selected_object_refs =
                    session.selected_object_refs().iter().cloned().collect();
                self.save_meta(&meta)?;
                Ok(())
            }
            Err(StoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                self.save_session(session)
            }
            Err(err) => Err(err),
        }
    }

    pub fn save_active_diagram_id(&self, session: &Session) -> Result<(), StoreError> {
        match self.load_meta() {
            Ok(mut meta) => {
                meta.active_diagram_id = session.active_diagram_id().cloned();
                self.save_meta(&meta)?;
                Ok(())
            }
            Err(StoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                self.save_session(session)
            }
            Err(err) => Err(err),
        }
    }

    pub fn load_walkthrough(
        &self,
        walkthrough_id: &WalkthroughId,
    ) -> Result<Walkthrough, StoreError> {
        let wt_path = self.walkthrough_json_path(walkthrough_id);
        let (wt_path, wt_str) = match fs::read_to_string(&wt_path) {
            Ok(wt_str) => (wt_path, wt_str),
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                let legacy_path = self.legacy_walkthrough_json_path(walkthrough_id);
                let wt_str = fs::read_to_string(&legacy_path).map_err(|source| StoreError::Io {
                    path: legacy_path.clone(),
                    source,
                })?;
                (legacy_path, wt_str)
            }
            Err(source) => {
                return Err(StoreError::Io {
                    path: wt_path.clone(),
                    source,
                });
            }
        };

        let wt_json: WalkthroughJson =
            serde_json::from_str(&wt_str).map_err(|source| StoreError::Json {
                path: wt_path.clone(),
                source,
            })?;

        walkthrough_from_json(wt_json)
    }

    pub fn save_walkthrough(&self, walkthrough: &Walkthrough) -> Result<(), StoreError> {
        let wt_path = self.walkthrough_json_path(walkthrough.walkthrough_id());

        let wt_json = walkthrough_to_json(walkthrough);
        let wt_str = serde_json::to_string_pretty(&wt_json).map_err(|source| StoreError::Json {
            path: wt_path.clone(),
            source,
        })?;

        write_atomic_in_session(
            self.root(),
            &wt_path,
            format!("{wt_str}\n").as_bytes(),
            self.durability,
        )?;

        self.schedule_walkthrough_ascii_export(walkthrough)?;

        Ok(())
    }

    fn schedule_diagram_ascii_export(
        &self,
        mmd_path: &Path,
        diagram: &Diagram,
    ) -> Result<(), StoreError> {
        let text_path = self.diagram_ascii_path(mmd_path)?;

        ascii_exports().schedule(AsciiExportTask::Diagram {
            session_dir: self.root.clone(),
            mmd_path: mmd_path.to_path_buf(),
            text_path,
            durability: self.durability,
            ast: diagram.ast().clone(),
        });

        Ok(())
    }

    fn schedule_walkthrough_ascii_export(
        &self,
        walkthrough: &Walkthrough,
    ) -> Result<(), StoreError> {
        let json_path = self.walkthrough_json_path(walkthrough.walkthrough_id());
        let text_path = self.walkthrough_ascii_path(walkthrough.walkthrough_id());

        ascii_exports().schedule(AsciiExportTask::Walkthrough {
            session_dir: self.root.clone(),
            json_path,
            text_path,
            durability: self.durability,
            walkthrough: walkthrough.clone(),
        });

        Ok(())
    }

    pub fn load_diagram_meta(&self, mmd_path: &Path) -> Result<DiagramMeta, StoreError> {
        let meta_path = self.diagram_meta_path(mmd_path)?;
        let meta_str = fs::read_to_string(&meta_path).map_err(|source| StoreError::Io {
            path: meta_path.clone(),
            source,
        })?;

        let meta_json: DiagramMetaJson =
            serde_json::from_str(&meta_str).map_err(|source| StoreError::Json {
                path: meta_path.clone(),
                source,
            })?;

        diagram_meta_from_json(self.root(), meta_json)
    }

    pub fn save_diagram_meta(&self, meta: &DiagramMeta) -> Result<(), StoreError> {
        let meta_path = self.diagram_meta_path(&meta.mmd_path)?;

        let meta_json = diagram_meta_to_json(self.root(), meta)?;
        let meta_str =
            serde_json::to_string_pretty(&meta_json).map_err(|source| StoreError::Json {
                path: meta_path.clone(),
                source,
            })?;

        write_atomic_in_session(
            self.root(),
            &meta_path,
            format!("{meta_str}\n").as_bytes(),
            self.durability,
        )?;

        Ok(())
    }
}

// Extracted persistence and reconciliation helpers for `SessionFolder`.
include!("session_folder/helpers.rs");

#[cfg(test)]
mod tests;
