// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use crate::model::flow_ast::{FlowEdge, FlowchartAst};
use crate::model::ids::ObjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowchartLayout {
    layers: Vec<Vec<ObjectId>>,
    node_placements: BTreeMap<ObjectId, FlowNodePlacement>,
}

impl FlowchartLayout {
    pub fn layers(&self) -> &[Vec<ObjectId>] {
        &self.layers
    }

    pub fn node_placements(&self) -> &BTreeMap<ObjectId, FlowNodePlacement> {
        &self.node_placements
    }

    pub fn placement(&self, node_id: &ObjectId) -> Option<&FlowNodePlacement> {
        self.node_placements.get(node_id)
    }

    /// Returns the node's anchor point in the routing grid coordinate system.
    ///
    /// Grid coordinates are integer points. Nodes are placed on even coordinates:
    /// - `x = layer * 2`
    /// - `y = index_in_layer * 2`
    ///
    /// The extra spacing leaves odd coordinates available for edge routing.
    pub fn node_grid_point(&self, node_id: &ObjectId) -> Option<GridPoint> {
        let placement = self.placement(node_id)?;
        Some(GridPoint::new(
            (placement.layer() * 2) as i32,
            (placement.index_in_layer() * 2) as i32,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowNodePlacement {
    layer: usize,
    index_in_layer: usize,
}

impl FlowNodePlacement {
    pub fn layer(&self) -> usize {
        self.layer
    }

    pub fn index_in_layer(&self) -> usize {
        self.index_in_layer
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GridPoint {
    x: i32,
    y: i32,
}

impl GridPoint {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    fn offset(self, dx: i32, dy: i32) -> Self {
        Self { x: self.x + dx, y: self.y + dy }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowchartLayoutError {
    UnknownNode { edge_id: ObjectId, endpoint: FlowEdgeEndpoint, node_id: ObjectId },
    CycleDetected { nodes: Vec<ObjectId> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowEdgeEndpoint {
    From,
    To,
}

impl std::fmt::Display for FlowchartLayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownNode { edge_id, endpoint, node_id } => {
                let endpoint = match endpoint {
                    FlowEdgeEndpoint::From => "from",
                    FlowEdgeEndpoint::To => "to",
                };
                write!(f, "edge {edge_id} references unknown {endpoint} node {node_id}")
            }
            Self::CycleDetected { nodes } => {
                if nodes.is_empty() {
                    return write!(f, "flowchart contains a cycle");
                }
                write!(f, "flowchart contains a cycle involving nodes: ")?;
                for (idx, node_id) in nodes.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{node_id}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for FlowchartLayoutError {}

#[derive(Debug, Clone, Copy)]
struct GridBounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl GridBounds {
    fn contains(&self, p: GridPoint) -> bool {
        p.x >= self.min_x && p.x <= self.max_x && p.y >= self.min_y && p.y <= self.max_y
    }

    fn expand(&self, margin: i32) -> Self {
        Self {
            min_x: self.min_x - margin,
            max_x: self.max_x + margin,
            min_y: self.min_y - margin,
            max_y: self.max_y + margin,
        }
    }
}

fn cmp_object_ids_lexical(a: &ObjectId, b: &ObjectId) -> Ordering {
    a.cmp(b)
}

fn sort_object_ids_lexical(ids: &mut [ObjectId]) {
    ids.sort_by(cmp_object_ids_lexical);
}

fn cmp_edge_routing_order(
    edge_id_a: &ObjectId,
    edge_a: &FlowEdge,
    edge_id_b: &ObjectId,
    edge_b: &FlowEdge,
) -> Ordering {
    cmp_object_ids_lexical(edge_a.from_node_id(), edge_b.from_node_id())
        .then_with(|| cmp_object_ids_lexical(edge_a.to_node_id(), edge_b.to_node_id()))
        .then_with(|| cmp_object_ids_lexical(edge_id_a, edge_id_b))
}

fn barycenter_for_layer_sort(
    node_id: &ObjectId,
    prev_positions: &BTreeMap<ObjectId, usize>,
    predecessors: &BTreeMap<ObjectId, Vec<ObjectId>>,
) -> Option<(usize, usize)> {
    predecessors
        .get(node_id)
        .map(|preds| {
            preds
                .iter()
                .filter_map(|p| prev_positions.get(p).copied())
                .fold((0usize, 0usize), |(sum, count), pos| (sum + pos, count + 1))
        })
        .and_then(|(sum, count)| (count > 0).then_some((sum, count)))
}

fn cmp_layer_nodes_by_barycenter(
    a: &ObjectId,
    b: &ObjectId,
    prev_positions: &BTreeMap<ObjectId, usize>,
    predecessors: &BTreeMap<ObjectId, Vec<ObjectId>>,
) -> Ordering {
    let bary_a = barycenter_for_layer_sort(a, prev_positions, predecessors);
    let bary_b = barycenter_for_layer_sort(b, prev_positions, predecessors);

    match (bary_a, bary_b) {
        (None, None) => cmp_object_ids_lexical(a, b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some((sum_a, count_a)), Some((sum_b, count_b))) => {
            // Compare sum_a/count_a vs sum_b/count_b without floats.
            let left = (sum_a as u128) * (count_b as u128);
            let right = (sum_b as u128) * (count_a as u128);
            left.cmp(&right).then_with(|| cmp_object_ids_lexical(a, b))
        }
    }
}

fn topo_sort_nodes(ast: &FlowchartAst) -> Result<Vec<ObjectId>, FlowchartLayoutError> {
    let mut indegree = BTreeMap::<ObjectId, usize>::new();
    let mut outgoing = BTreeMap::<ObjectId, Vec<ObjectId>>::new();

    for node_id in ast.nodes().keys() {
        indegree.insert(node_id.clone(), 0);
        outgoing.insert(node_id.clone(), Vec::new());
    }

    for (edge_id, edge) in ast.edges() {
        let from = edge.from_node_id();
        let to = edge.to_node_id();

        if !ast.nodes().contains_key(from) {
            return Err(FlowchartLayoutError::UnknownNode {
                edge_id: edge_id.clone(),
                endpoint: FlowEdgeEndpoint::From,
                node_id: from.clone(),
            });
        }
        if !ast.nodes().contains_key(to) {
            return Err(FlowchartLayoutError::UnknownNode {
                edge_id: edge_id.clone(),
                endpoint: FlowEdgeEndpoint::To,
                node_id: to.clone(),
            });
        }

        outgoing.get_mut(from).expect("node exists (validated)").push(to.clone());
        *indegree.get_mut(to).expect("node exists (validated)") += 1;
    }

    for tos in outgoing.values_mut() {
        sort_object_ids_lexical(tos);
    }

    let mut ready = BTreeSet::<ObjectId>::new();
    for (node_id, degree) in &indegree {
        if *degree == 0 {
            ready.insert(node_id.clone());
        }
    }

    let mut topo = Vec::<ObjectId>::with_capacity(indegree.len());
    while !ready.is_empty() {
        let next = ready.iter().next().cloned().expect("set not empty");
        ready.remove(&next);
        topo.push(next.clone());
        let tos = outgoing.get(&next).expect("node exists");
        for to in tos {
            let degree = indegree.get_mut(to).expect("node exists");
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                ready.insert(to.clone());
            }
        }
    }

    if topo.len() != indegree.len() {
        let nodes = indegree
            .into_iter()
            .filter_map(|(node_id, degree)| (degree > 0).then_some(node_id))
            .collect::<Vec<_>>();
        return Err(FlowchartLayoutError::CycleDetected { nodes });
    }

    Ok(topo)
}

fn assign_layers(
    topo: &[ObjectId],
    outgoing: &BTreeMap<ObjectId, Vec<ObjectId>>,
) -> BTreeMap<ObjectId, usize> {
    let mut layers = BTreeMap::<ObjectId, usize>::new();
    for node_id in topo {
        layers.insert(node_id.clone(), 0);
    }

    for from in topo {
        let from_layer = *layers.get(from).expect("node exists");
        let tos = outgoing.get(from).map(|v| v.as_slice()).unwrap_or(&[]);
        for to in tos {
            let to_layer = layers.get(to).copied().unwrap_or(0);
            layers.insert(to.clone(), to_layer.max(from_layer + 1));
        }
    }

    layers
}

fn sort_layer_by_barycenter(
    layer_nodes: &mut [ObjectId],
    prev_positions: &BTreeMap<ObjectId, usize>,
    predecessors: &BTreeMap<ObjectId, Vec<ObjectId>>,
) {
    layer_nodes.sort_by(|a, b| cmp_layer_nodes_by_barycenter(a, b, prev_positions, predecessors));
}

/// Deterministic layered layout for flowcharts (DAG-first).
///
/// Baseline:
/// - Rejects cycles.
/// - Assigns node layers via longest-path layering over a deterministic topological order.
/// - Orders nodes within each layer deterministically (with a simple barycenter sweep).
pub fn layout_flowchart(ast: &FlowchartAst) -> Result<FlowchartLayout, FlowchartLayoutError> {
    let topo = topo_sort_nodes(ast)?;

    // Rebuild adjacency + predecessors with validated nodes.
    let mut outgoing = BTreeMap::<ObjectId, Vec<ObjectId>>::new();
    let mut predecessors = BTreeMap::<ObjectId, Vec<ObjectId>>::new();
    for node_id in ast.nodes().keys() {
        outgoing.insert(node_id.clone(), Vec::new());
        predecessors.insert(node_id.clone(), Vec::new());
    }
    for edge in ast.edges().values() {
        outgoing
            .get_mut(edge.from_node_id())
            .expect("node exists (validated)")
            .push(edge.to_node_id().clone());
        predecessors
            .get_mut(edge.to_node_id())
            .expect("node exists (validated)")
            .push(edge.from_node_id().clone());
    }
    for tos in outgoing.values_mut() {
        sort_object_ids_lexical(tos);
    }
    for preds in predecessors.values_mut() {
        sort_object_ids_lexical(preds);
    }

    let node_layers = assign_layers(&topo, &outgoing);

    let max_layer = node_layers.values().copied().max().unwrap_or(0);
    let mut layers = vec![Vec::<ObjectId>::new(); max_layer + 1];
    for node_id in ast.nodes().keys() {
        let layer = *node_layers.get(node_id).unwrap_or(&0);
        layers[layer].push(node_id.clone());
    }

    // Start deterministic: ObjectId ordering within each layer.
    for layer_nodes in layers.iter_mut() {
        sort_object_ids_lexical(layer_nodes);
    }

    // One downward barycenter sweep for readability (deterministic).
    for layer_idx in 1..layers.len() {
        let prev_positions = layers[layer_idx - 1]
            .iter()
            .enumerate()
            .map(|(idx, node_id)| (node_id.clone(), idx))
            .collect::<BTreeMap<_, _>>();

        sort_layer_by_barycenter(&mut layers[layer_idx], &prev_positions, &predecessors);
    }

    let mut node_placements = BTreeMap::<ObjectId, FlowNodePlacement>::new();
    for (layer, nodes) in layers.iter().enumerate() {
        for (index_in_layer, node_id) in nodes.iter().enumerate() {
            node_placements.insert(node_id.clone(), FlowNodePlacement { layer, index_in_layer });
        }
    }

    Ok(FlowchartLayout { layers, node_placements })
}

#[derive(Debug, Clone)]
struct RoutingObstacleProjection {
    bounds: GridBounds,
    obstacles: BTreeSet<GridPoint>,
    point_owners: BTreeMap<GridPoint, BTreeSet<ObjectId>>,
    projected_by_node: BTreeMap<ObjectId, Vec<GridPoint>>,
}

fn projected_node_obstacle_points(anchor: GridPoint) -> Vec<GridPoint> {
    const HALF_WIDTH: i32 = 1;
    const HALF_HEIGHT: i32 = 1;

    let mut points = BTreeSet::<GridPoint>::new();
    for y in (anchor.y - HALF_HEIGHT)..=(anchor.y + HALF_HEIGHT) {
        for x in (anchor.x - HALF_WIDTH)..=(anchor.x + HALF_WIDTH) {
            points.insert(GridPoint::new(x, y));
        }
    }

    points.into_iter().collect()
}

fn routing_obstacle_projection(layout: &FlowchartLayout) -> RoutingObstacleProjection {
    let mut obstacles = BTreeSet::<GridPoint>::new();
    let mut point_owners = BTreeMap::<GridPoint, BTreeSet<ObjectId>>::new();
    let mut projected_by_node = BTreeMap::<ObjectId, Vec<GridPoint>>::new();

    for node_id in layout.node_placements().keys() {
        let Some(anchor) = layout.node_grid_point(node_id) else {
            continue;
        };
        let projected = projected_node_obstacle_points(anchor);
        for point in projected.iter().copied() {
            obstacles.insert(point);
            point_owners.entry(point).or_default().insert(node_id.clone());
        }
        projected_by_node.insert(node_id.clone(), projected);
    }

    let mut min_x = 0i32;
    let mut max_x = 0i32;
    let mut min_y = 0i32;
    let mut max_y = 0i32;
    for (idx, p) in obstacles.iter().enumerate() {
        if idx == 0 {
            min_x = p.x;
            max_x = p.x;
            min_y = p.y;
            max_y = p.y;
            continue;
        }
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    let bounds = GridBounds { min_x, max_x, min_y, max_y }.expand(4);

    RoutingObstacleProjection { bounds, obstacles, point_owners, projected_by_node }
}

fn edge_obstacles_with_endpoint_clearance(
    projection: &RoutingObstacleProjection,
    from_node_id: &ObjectId,
    to_node_id: &ObjectId,
) -> BTreeSet<GridPoint> {
    let mut obstacles = projection.obstacles.clone();
    let from_anchor = projection.projected_by_node.get(from_node_id).and_then(|points| {
        points.iter().copied().find(|point| (point.x % 2 == 0) && (point.y % 2 == 0))
    });
    let to_anchor = projection.projected_by_node.get(to_node_id).and_then(|points| {
        points.iter().copied().find(|point| (point.x % 2 == 0) && (point.y % 2 == 0))
    });
    let mut force_clear_points = BTreeSet::<GridPoint>::new();
    if let Some(anchor) = from_anchor {
        force_clear_points.insert(anchor);
    }
    if let Some(anchor) = to_anchor {
        force_clear_points.insert(anchor);
    }
    if let (Some(from_anchor), Some(to_anchor)) = (from_anchor, to_anchor) {
        let dx = to_anchor.x - from_anchor.x;
        let dy = to_anchor.y - from_anchor.y;
        let from_step = if dy.abs() >= dx.abs() { (0, dy.signum()) } else { (dx.signum(), 0) };
        if from_step != (0, 0) {
            force_clear_points.insert(from_anchor.offset(from_step.0, from_step.1));
        }

        let dx_back = from_anchor.x - to_anchor.x;
        let dy_back = from_anchor.y - to_anchor.y;
        let to_step = if dy_back.abs() >= dx_back.abs() {
            (0, dy_back.signum())
        } else {
            (dx_back.signum(), 0)
        };
        if to_step != (0, 0) {
            force_clear_points.insert(to_anchor.offset(to_step.0, to_step.1));
        }
    }

    if let Some(points) = projection.projected_by_node.get(from_node_id) {
        for point in points {
            let owners = projection.point_owners.get(point);
            let owned_only_by_endpoints = owners
                .map(|owners| {
                    owners.iter().all(|owner| owner == from_node_id || owner == to_node_id)
                })
                .unwrap_or(false);
            if owned_only_by_endpoints || force_clear_points.contains(point) {
                obstacles.remove(point);
            }
        }
    }
    if to_node_id != from_node_id {
        if let Some(points) = projection.projected_by_node.get(to_node_id) {
            for point in points {
                let owners = projection.point_owners.get(point);
                let owned_only_by_endpoints = owners
                    .map(|owners| {
                        owners.iter().all(|owner| owner == from_node_id || owner == to_node_id)
                    })
                    .unwrap_or(false);
                if owned_only_by_endpoints || force_clear_points.contains(point) {
                    obstacles.remove(point);
                }
            }
        }
    }

    obstacles
}

fn neighbor_deltas_towards(current: GridPoint, goal: GridPoint) -> [(i32, i32); 4] {
    let dx = goal.x - current.x;
    let dy = goal.y - current.y;

    let primary_x = if dx > 0 {
        Some((1, 0))
    } else if dx < 0 {
        Some((-1, 0))
    } else {
        None
    };

    let primary_y = if dy > 0 {
        Some((0, 1))
    } else if dy < 0 {
        Some((0, -1))
    } else {
        None
    };

    let mut out = [(0, 0); 4];
    let mut idx = 0usize;

    if let Some(delta) = primary_x {
        out[idx] = delta;
        idx += 1;
    }

    if let Some(delta) = primary_y {
        out[idx] = delta;
        idx += 1;
    }

    // Fill remaining directions deterministically:
    // prefer vertical detours before moving horizontally away from the goal.
    for delta in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
        if primary_x == Some(delta) || primary_y == Some(delta) {
            continue;
        }
        out[idx] = delta;
        idx += 1;
    }

    debug_assert_eq!(idx, 4);
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RoutingGrid {
    min_x: i32,
    min_y: i32,
    width: usize,
    height: usize,
}

impl RoutingGrid {
    fn new(bounds: GridBounds) -> Self {
        let width = (bounds.max_x - bounds.min_x + 1) as usize;
        let height = (bounds.max_y - bounds.min_y + 1) as usize;
        Self { min_x: bounds.min_x, min_y: bounds.min_y, width, height }
    }

    fn len(&self) -> usize {
        self.width.checked_mul(self.height).expect("routing grid area overflow")
    }

    fn idx_of(&self, point: GridPoint) -> Option<usize> {
        let x = point.x - self.min_x;
        let y = point.y - self.min_y;
        if x < 0 || y < 0 {
            return None;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(y * self.width + x)
    }

    fn point_of(&self, idx: usize) -> GridPoint {
        let x = (idx % self.width) as i32 + self.min_x;
        let y = (idx / self.width) as i32 + self.min_y;
        GridPoint::new(x, y)
    }
}

#[derive(Debug, Default)]
struct ShortestPathScratch {
    grid: Option<RoutingGrid>,
    obstacles: Vec<u8>,
    visit_gen: Vec<u32>,
    came_from: Vec<i32>,
    queue: Vec<GridPoint>,
    path: Vec<GridPoint>,
    queue_head: usize,
    gen: u32,
    dist_gen: Vec<u32>,
    dist_cost: Vec<u32>,
    heap: BinaryHeap<Reverse<(u32, u32, u32, u32)>>,
}

impl ShortestPathScratch {
    fn configure(&mut self, bounds: GridBounds, obstacles: &BTreeSet<GridPoint>) {
        let grid = RoutingGrid::new(bounds);
        let len = grid.len();

        if self.grid != Some(grid) {
            self.grid = Some(grid);
            self.obstacles = vec![0u8; len];
            self.visit_gen = vec![0u32; len];
            self.came_from = vec![-1i32; len];
            self.dist_gen = vec![0u32; len];
            self.dist_cost = vec![0u32; len];
        } else {
            if self.obstacles.len() != len {
                self.obstacles.resize(len, 0);
            }
            self.obstacles.fill(0);
            if self.visit_gen.len() != len {
                self.visit_gen.resize(len, 0);
            }
            if self.came_from.len() != len {
                self.came_from.resize(len, -1);
            }
            if self.dist_gen.len() != len {
                self.dist_gen.resize(len, 0);
            }
            if self.dist_cost.len() != len {
                self.dist_cost.resize(len, 0);
            }
        }

        let reserve_hint = len.min(4096);
        self.queue.reserve(reserve_hint.saturating_sub(self.queue.len()));
        self.path.reserve(reserve_hint.saturating_sub(self.path.len()));
        self.heap.reserve(reserve_hint.saturating_sub(self.heap.len()));

        let grid = self.grid.expect("configured");
        for point in obstacles.iter().copied() {
            if let Some(idx) = grid.idx_of(point) {
                self.obstacles[idx] = 1;
            }
        }
    }

    fn begin(&mut self) -> u32 {
        self.gen = self.gen.wrapping_add(1);
        if self.gen == 0 {
            self.visit_gen.fill(0);
            self.dist_gen.fill(0);
            self.gen = 1;
        }
        self.queue.clear();
        self.queue_head = 0;
        self.heap.clear();
        self.gen
    }

    fn visit(&mut self, idx: usize, gen: u32, came_from: i32) -> bool {
        if self.visit_gen[idx] == gen {
            return false;
        }

        self.visit_gen[idx] = gen;
        self.came_from[idx] = came_from;
        true
    }

    fn grid(&self) -> RoutingGrid {
        self.grid.expect("routing scratch configured")
    }
}

fn shortest_path_4dir(
    start: GridPoint,
    goal: GridPoint,
    bounds: GridBounds,
    scratch: &mut ShortestPathScratch,
) -> Option<&[GridPoint]> {
    if start == goal {
        scratch.path.clear();
        scratch.path.push(start);
        return Some(&scratch.path);
    }

    let grid = scratch.grid();
    let start_idx = grid.idx_of(start)?;
    let goal_idx = grid.idx_of(goal)?;

    let gen = scratch.begin();
    scratch.visit(start_idx, gen, -1);
    scratch.queue.push(start);

    while let Some(&current) = scratch.queue.get(scratch.queue_head) {
        scratch.queue_head += 1;
        if current == goal {
            scratch.path.clear();
            scratch.path.push(goal);
            let mut cursor_idx = goal_idx;
            while cursor_idx != start_idx {
                let prev_idx = scratch.came_from[cursor_idx] as isize;
                if prev_idx < 0 {
                    return None;
                }
                let prev_idx = prev_idx as usize;
                scratch.path.push(grid.point_of(prev_idx));
                cursor_idx = prev_idx;
            }
            scratch.path.reverse();
            return Some(&scratch.path);
        }

        let current_idx = grid.idx_of(current)?;
        for (dx, dy) in neighbor_deltas_towards(current, goal) {
            let next = current.offset(dx, dy);
            if !bounds.contains(next) {
                continue;
            }
            let Some(next_idx) = grid.idx_of(next) else {
                continue;
            };
            if next != start && next != goal && next.x % 2 == 0 && next.y % 2 == 0 {
                continue;
            }
            if next != start && next != goal && scratch.obstacles[next_idx] == 1 {
                continue;
            }
            if scratch.visit(next_idx, gen, current_idx as i32) {
                scratch.queue.push(next);
            }
        }
    }

    None
}

fn shortest_path_4dir_soft_occupancy<'a>(
    start: GridPoint,
    goal: GridPoint,
    bounds: GridBounds,
    scratch: &'a mut ShortestPathScratch,
    occupancy: &[u8],
) -> Option<&'a [GridPoint]> {
    const BEND_PENALTY: u32 = 12;
    const CONGESTION_PENALTY: u32 = 16;
    const CONGESTION_HARD_BLOCK_THRESHOLD: u8 = 9;
    const STATE_DIRS: usize = 3;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum MoveDir {
        None = 0,
        Horizontal = 1,
        Vertical = 2,
    }

    impl MoveDir {
        fn index(self) -> usize {
            self as usize
        }

        fn from_state_index(idx: usize) -> Self {
            match idx % STATE_DIRS {
                0 => Self::None,
                1 => Self::Horizontal,
                _ => Self::Vertical,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct RouteScore {
        weighted: u32,
        bends: u32,
        congestion: u32,
        length: u32,
    }

    impl RouteScore {
        const UNREACHED: Self =
            Self { weighted: u32::MAX, bends: u32::MAX, congestion: u32::MAX, length: u32::MAX };
        const ZERO: Self = Self { weighted: 0, bends: 0, congestion: 0, length: 0 };

        fn reached(self) -> bool {
            self.weighted != u32::MAX
        }

        fn ranking_key(self) -> (u32, u32, u32, u32) {
            (self.weighted, self.bends, self.congestion, self.length)
        }

        fn with_increment(self, length_inc: u32, bend_inc: u32, congestion_inc: u32) -> Self {
            Self {
                weighted: self
                    .weighted
                    .saturating_add(length_inc)
                    .saturating_add(bend_inc.saturating_mul(BEND_PENALTY))
                    .saturating_add(congestion_inc.saturating_mul(CONGESTION_PENALTY)),
                bends: self.bends.saturating_add(bend_inc),
                congestion: self.congestion.saturating_add(congestion_inc),
                length: self.length.saturating_add(length_inc),
            }
        }
    }

    #[inline(always)]
    fn state_idx(cell_idx: usize, dir: MoveDir) -> usize {
        cell_idx * STATE_DIRS + dir.index()
    }

    #[inline(always)]
    fn state_cell_idx(state_idx: usize) -> usize {
        state_idx / STATE_DIRS
    }

    #[inline(always)]
    fn manhattan(a: GridPoint, b: GridPoint) -> u32 {
        a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
    }

    #[inline(always)]
    fn is_hard_blocked(
        point: GridPoint,
        point_idx: usize,
        start: GridPoint,
        goal: GridPoint,
        obstacles: &[u8],
    ) -> bool {
        point != start && point != goal && obstacles[point_idx] == 1
    }

    fn candidate_is_better(
        candidate: RouteScore,
        candidate_parent: usize,
        candidate_midpoint: GridPoint,
        existing: RouteScore,
        existing_parent: i32,
        existing_midpoint: Option<GridPoint>,
    ) -> bool {
        if !existing.reached() {
            return true;
        }
        match candidate.ranking_key().cmp(&existing.ranking_key()) {
            Ordering::Less => true,
            Ordering::Greater => false,
            Ordering::Equal => {
                let existing_parent =
                    if existing_parent < 0 { u32::MAX } else { existing_parent as u32 };
                let candidate_parent = candidate_parent as u32;
                if candidate_parent != existing_parent {
                    return candidate_parent < existing_parent;
                }
                Some(candidate_midpoint) < existing_midpoint
            }
        }
    }

    fn reconstruct_path<'a>(
        start_state: usize,
        goal_state: usize,
        grid: RoutingGrid,
        parent_state: &[i32],
        parent_midpoint: &[Option<GridPoint>],
        scratch: &'a mut ShortestPathScratch,
    ) -> Option<&'a [GridPoint]> {
        scratch.path.clear();
        scratch.path.push(grid.point_of(state_cell_idx(goal_state)));

        let mut cursor = goal_state;
        while cursor != start_state {
            let prev = *parent_state.get(cursor)?;
            if prev < 0 {
                return None;
            }
            if let Some(midpoint) = parent_midpoint.get(cursor).copied().flatten() {
                scratch.path.push(midpoint);
            }
            let prev = prev as usize;
            scratch.path.push(grid.point_of(state_cell_idx(prev)));
            cursor = prev;
        }

        debug_assert_eq!(
            scratch.path.last().copied(),
            Some(grid.point_of(state_cell_idx(start_state)))
        );
        scratch.path.reverse();
        Some(&scratch.path)
    }

    fn neighbor_deltas2_towards(current: GridPoint, goal: GridPoint) -> [(i32, i32); 4] {
        let dx = goal.x - current.x;
        let dy = goal.y - current.y;

        let primary_x = if dx > 0 {
            Some((2, 0))
        } else if dx < 0 {
            Some((-2, 0))
        } else {
            None
        };

        let primary_y = if dy > 0 {
            Some((0, 2))
        } else if dy < 0 {
            Some((0, -2))
        } else {
            None
        };

        let mut out = [(0, 0); 4];
        let mut idx = 0usize;

        if let Some(delta) = primary_x {
            out[idx] = delta;
            idx += 1;
        }

        if let Some(delta) = primary_y {
            out[idx] = delta;
            idx += 1;
        }

        for delta in [(0, 2), (0, -2), (2, 0), (-2, 0)] {
            if primary_x == Some(delta) || primary_y == Some(delta) {
                continue;
            }
            out[idx] = delta;
            idx += 1;
        }

        debug_assert_eq!(idx, 4);
        out
    }

    if start == goal {
        scratch.path.clear();
        scratch.path.push(start);
        return Some(&scratch.path);
    }

    let grid = scratch.grid();
    let start_idx = grid.idx_of(start)?;
    let goal_idx = grid.idx_of(goal)?;
    if occupancy.len() != grid.len() {
        return None;
    }

    let state_count = grid.len().saturating_mul(STATE_DIRS);
    let mut best = vec![RouteScore::UNREACHED; state_count];
    let mut parent_state = vec![-1i32; state_count];
    let mut parent_midpoint = vec![None::<GridPoint>; state_count];
    let mut heap = BinaryHeap::<Reverse<(u32, u32, u32, u32, u32, u32)>>::new();

    let start_state = state_idx(start_idx, MoveDir::None);
    best[start_state] = RouteScore::ZERO;
    heap.push(Reverse((manhattan(start, goal), 0, 0, 0, 0, start_state as u32)));

    while let Some(Reverse((f_weighted, g_weighted, g_bends, g_congestion, g_length, state_raw))) =
        heap.pop()
    {
        let state = state_raw as usize;
        let current_score = best[state];
        if !current_score.reached() {
            continue;
        }
        let current_point = grid.point_of(state_cell_idx(state));
        let expected_f = current_score.weighted.saturating_add(manhattan(current_point, goal));
        if (f_weighted, g_weighted, g_bends, g_congestion, g_length)
            != (
                expected_f,
                current_score.weighted,
                current_score.bends,
                current_score.congestion,
                current_score.length,
            )
        {
            continue;
        }

        let current_idx = state_cell_idx(state);
        let current_dir = MoveDir::from_state_index(state);
        if current_idx == goal_idx {
            return reconstruct_path(
                start_state,
                state,
                grid,
                &parent_state,
                &parent_midpoint,
                scratch,
            );
        }

        let mut relax = |first_dir: MoveDir,
                         arrival_dir: MoveDir,
                         midpoint: GridPoint,
                         next: GridPoint,
                         next_idx: usize| {
            if !bounds.contains(midpoint) {
                return;
            }
            let Some(mid_idx) = grid.idx_of(midpoint) else {
                return;
            };
            if midpoint != start && midpoint != goal && midpoint.x % 2 == 0 && midpoint.y % 2 == 0 {
                return;
            }
            if next != start && next != goal && next.x % 2 == 0 && next.y % 2 == 0 {
                return;
            }
            if is_hard_blocked(midpoint, mid_idx, start, goal, &scratch.obstacles)
                || is_hard_blocked(next, next_idx, start, goal, &scratch.obstacles)
            {
                return;
            }
            if midpoint != start
                && midpoint != goal
                && occupancy[mid_idx] >= CONGESTION_HARD_BLOCK_THRESHOLD
            {
                return;
            }
            if next != start
                && next != goal
                && occupancy[next_idx] >= CONGESTION_HARD_BLOCK_THRESHOLD
            {
                return;
            }

            let bend_inc =
                if current_dir == MoveDir::None || current_dir == first_dir { 0 } else { 1 };
            let congestion_inc = u32::from(occupancy[mid_idx]) + u32::from(occupancy[next_idx]);
            let candidate = current_score.with_increment(2, bend_inc, congestion_inc);
            let next_state = state_idx(next_idx, arrival_dir);
            if !candidate_is_better(
                candidate,
                state,
                midpoint,
                best[next_state],
                parent_state[next_state],
                parent_midpoint[next_state],
            ) {
                return;
            }

            best[next_state] = candidate;
            parent_state[next_state] = state as i32;
            parent_midpoint[next_state] = Some(midpoint);
            heap.push(Reverse((
                candidate.weighted.saturating_add(manhattan(next, goal)),
                candidate.weighted,
                candidate.bends,
                candidate.congestion,
                candidate.length,
                next_state as u32,
            )));
        };

        if current_idx == start_idx && current_dir == MoveDir::None {
            let dx = (goal.x - current_point.x).abs();
            let dy = (goal.y - current_point.y).abs();
            if (dx == 2 && dy == 0) || (dx == 0 && dy == 2) {
                let midpoint =
                    GridPoint::new((current_point.x + goal.x) / 2, (current_point.y + goal.y) / 2);
                let travel_dir = if dx == 2 { MoveDir::Horizontal } else { MoveDir::Vertical };
                relax(travel_dir, travel_dir, midpoint, goal, goal_idx);
            }

            for (sx, sy) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
                let next = current_point.offset(sx, sy);
                if !bounds.contains(next) {
                    continue;
                }
                if (next.x & 1) == 0 || (next.y & 1) == 0 {
                    continue;
                }
                let Some(next_idx) = grid.idx_of(next) else {
                    continue;
                };

                // Vertical then horizontal.
                relax(
                    MoveDir::Vertical,
                    MoveDir::Horizontal,
                    GridPoint::new(current_point.x, next.y),
                    next,
                    next_idx,
                );
                // Horizontal then vertical.
                relax(
                    MoveDir::Horizontal,
                    MoveDir::Vertical,
                    GridPoint::new(next.x, current_point.y),
                    next,
                    next_idx,
                );
            }
            continue;
        }

        let dx_to_goal = (goal.x - current_point.x).abs();
        let dy_to_goal = (goal.y - current_point.y).abs();
        if dx_to_goal == 1 && dy_to_goal == 1 {
            // Vertical then horizontal.
            relax(
                MoveDir::Vertical,
                MoveDir::Horizontal,
                GridPoint::new(current_point.x, goal.y),
                goal,
                goal_idx,
            );
            // Horizontal then vertical.
            relax(
                MoveDir::Horizontal,
                MoveDir::Vertical,
                GridPoint::new(goal.x, current_point.y),
                goal,
                goal_idx,
            );
        }

        for (dx, dy) in neighbor_deltas2_towards(current_point, goal) {
            let next = current_point.offset(dx, dy);
            if !bounds.contains(next) {
                continue;
            }
            let Some(next_idx) = grid.idx_of(next) else {
                continue;
            };
            let midpoint = current_point.offset(dx / 2, dy / 2);
            let travel_dir = if dx == 0 { MoveDir::Vertical } else { MoveDir::Horizontal };
            relax(travel_dir, travel_dir, midpoint, next, next_idx);
        }
    }

    None
}

fn compress_to_polyline(path: &[GridPoint]) -> Vec<GridPoint> {
    match path.len() {
        0 => Vec::new(),
        1 => vec![path[0]],
        2 => vec![path[0], path[1]],
        _ => {
            let mut points = Vec::<GridPoint>::new();
            points.push(path[0]);

            let mut prev_dir = (path[1].x - path[0].x, path[1].y - path[0].y);
            for idx in 1..path.len() - 1 {
                let dir = (path[idx + 1].x - path[idx].x, path[idx + 1].y - path[idx].y);
                if dir != prev_dir {
                    points.push(path[idx]);
                    prev_dir = dir;
                }
            }

            if let Some(last) = path.last() {
                points.push(*last);
            }
            points
        }
    }
}

fn fallback_polyline(start: GridPoint, goal: GridPoint) -> Vec<GridPoint> {
    if start == goal {
        return vec![start];
    }
    if start.x == goal.x || start.y == goal.y {
        return vec![start, goal];
    }
    vec![start, GridPoint::new(goal.x, start.y), goal]
}

fn expand_polyline_points(route: &[GridPoint]) -> Vec<GridPoint> {
    if route.len() < 2 {
        return route.to_vec();
    }

    let mut points = Vec::<GridPoint>::new();
    points.push(route[0]);
    for window in route.windows(2) {
        let a = window[0];
        let b = window[1];
        let dx = (b.x - a.x).signum();
        let dy = (b.y - a.y).signum();
        let steps = (b.x - a.x).abs().max((b.y - a.y).abs());
        for step in 1..=steps {
            points.push(GridPoint::new(a.x + (dx * step), a.y + (dy * step)));
        }
    }
    points
}

fn fallback_polyline_soft_occupancy(
    start: GridPoint,
    goal: GridPoint,
    scratch: &ShortestPathScratch,
    occupancy: &[u8],
) -> Vec<GridPoint> {
    if start == goal {
        return vec![start];
    }
    if start.x == goal.x || start.y == goal.y {
        return vec![start, goal];
    }

    let min_x = start.x.min(goal.x);
    let max_x = start.x.max(goal.x);
    let min_y = start.y.min(goal.y);
    let max_y = start.y.max(goal.y);

    let candidates = vec![
        vec![start, GridPoint::new(goal.x, start.y), goal],
        vec![start, GridPoint::new(start.x, goal.y), goal],
        vec![start, GridPoint::new(start.x, min_y - 4), GridPoint::new(goal.x, min_y - 4), goal],
        vec![start, GridPoint::new(start.x, max_y + 4), GridPoint::new(goal.x, max_y + 4), goal],
        vec![start, GridPoint::new(min_x - 4, start.y), GridPoint::new(min_x - 4, goal.y), goal],
        vec![start, GridPoint::new(max_x + 4, start.y), GridPoint::new(max_x + 4, goal.y), goal],
    ];

    let grid = scratch.grid();

    let mut best_key = None::<(u32, u32, u32, u32, u32, Vec<GridPoint>)>;
    let mut best_route = fallback_polyline(start, goal);
    for route in candidates {
        let expanded = expand_polyline_points(&route);

        let mut hard_collisions = 0u32;
        let mut anchor_crossings = 0u32;
        let mut occupancy_cost = 0u32;
        for point in expanded.iter().copied() {
            let Some(idx) = grid.idx_of(point) else {
                hard_collisions = hard_collisions.saturating_add(1000);
                continue;
            };
            if point != start && point != goal && scratch.obstacles[idx] == 1 {
                hard_collisions = hard_collisions.saturating_add(1);
            }
            if point != start && point != goal && (point.x % 2 == 0) && (point.y % 2 == 0) {
                anchor_crossings = anchor_crossings.saturating_add(1);
            }
            occupancy_cost = occupancy_cost.saturating_add(u32::from(occupancy[idx]));
        }

        let bends = route.len().saturating_sub(2) as u32;
        let length = expanded.len() as u32;
        let key = (hard_collisions, anchor_crossings, occupancy_cost, bends, length, route.clone());
        if best_key.as_ref().map_or(true, |best| key < *best) {
            best_key = Some(key);
            best_route = route;
        }
    }

    best_route
}

fn edge_routing_bounds(start: GridPoint, goal: GridPoint) -> GridBounds {
    const MARGIN_MAIN: i32 = 2;
    const MARGIN_CROSS: i32 = 8;
    let dx = (goal.x - start.x).abs();
    let dy = (goal.y - start.y).abs();
    let (margin_x, margin_y) =
        if dx >= dy { (MARGIN_MAIN, MARGIN_CROSS) } else { (MARGIN_CROSS, MARGIN_MAIN) };
    GridBounds {
        min_x: start.x.min(goal.x) - margin_x,
        max_x: start.x.max(goal.x) + margin_x,
        min_y: start.y.min(goal.y) - margin_y,
        max_y: start.y.max(goal.y) + margin_y,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoutedPath {
    points: Vec<GridPoint>,
    used_fallback: bool,
}

fn route_orthogonal_with_scratch_result(
    start: GridPoint,
    goal: GridPoint,
    base_bounds: GridBounds,
    scratch: &mut ShortestPathScratch,
) -> RoutedPath {
    if let Some(path) = shortest_path_4dir(start, goal, base_bounds, scratch) {
        return RoutedPath { points: compress_to_polyline(path), used_fallback: false };
    }

    // Historically this router expanded bounds in 4-unit steps up to a max of
    // `base_bounds.expand(20)`. Keep the same maximum search window, but avoid
    // re-running the pathfinder multiple times on failures.
    let expanded = base_bounds.expand(20);
    if let Some(path) = shortest_path_4dir(start, goal, expanded, scratch) {
        return RoutedPath { points: compress_to_polyline(path), used_fallback: false };
    }

    RoutedPath { points: fallback_polyline(start, goal), used_fallback: true }
}

#[cfg(test)]
fn route_orthogonal_with_scratch(
    start: GridPoint,
    goal: GridPoint,
    base_bounds: GridBounds,
    scratch: &mut ShortestPathScratch,
) -> Vec<GridPoint> {
    route_orthogonal_with_scratch_result(start, goal, base_bounds, scratch).points
}

fn route_orthogonal_with_scratch_soft_occupancy_result(
    start: GridPoint,
    goal: GridPoint,
    base_bounds: GridBounds,
    scratch: &mut ShortestPathScratch,
    occupancy: &[u8],
) -> RoutedPath {
    let expanded = base_bounds.expand(20);
    if let Some(path) = shortest_path_4dir_soft_occupancy(start, goal, expanded, scratch, occupancy)
    {
        return RoutedPath { points: compress_to_polyline(path), used_fallback: false };
    }

    if let Some(path) =
        shortest_path_4dir_soft_occupancy(start, goal, base_bounds, scratch, occupancy)
    {
        return RoutedPath { points: compress_to_polyline(path), used_fallback: false };
    }

    RoutedPath {
        points: fallback_polyline_soft_occupancy(start, goal, scratch, occupancy),
        used_fallback: true,
    }
}

#[cfg(test)]
fn route_orthogonal(
    start: GridPoint,
    goal: GridPoint,
    base_bounds: GridBounds,
    obstacles: &BTreeSet<GridPoint>,
) -> Vec<GridPoint> {
    let max_bounds = base_bounds.expand(24);
    let mut scratch = ShortestPathScratch::default();
    scratch.configure(max_bounds, obstacles);
    route_orthogonal_with_scratch(start, goal, base_bounds, &mut scratch)
}

/// Deterministic orthogonal edge routing baseline for flowcharts.
///
/// Routes each edge as a polyline in a simple integer grid coordinate system.
/// Nodes are treated as obstacles (occupied grid points), but the start and end
/// nodes are permitted as endpoints.
pub fn route_flowchart_edges_orthogonal(
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
) -> BTreeMap<ObjectId, Vec<GridPoint>> {
    let routes_in_key_order = route_flowchart_edges_orthogonal_key_order(ast, layout);

    let mut routes = BTreeMap::<ObjectId, Vec<GridPoint>>::new();
    for ((edge_id, _edge), points) in ast.edges().iter().zip(routes_in_key_order) {
        routes.insert(edge_id.clone(), points);
    }
    routes
}

/// Routing variant that returns routes in `ast.edges()` key order.
///
/// Compared to [`route_flowchart_edges_orthogonal`], this avoids allocating a
/// `BTreeMap` and is generally faster for hot paths like rendering.
pub fn route_flowchart_edges_orthogonal_key_order(
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
) -> Vec<Vec<GridPoint>> {
    let edge_count = ast.edges().len();
    if edge_count == 0 {
        return Vec::new();
    }

    let enable_soft_occupancy = enable_soft_occupancy(ast, layout);

    let mut routes = vec![Vec::<GridPoint>::new(); edge_count];

    route_flowchart_edges_orthogonal_record(
        ast,
        layout,
        enable_soft_occupancy,
        |idx, _edge_id, points, _used_fallback| {
            routes[idx] = points;
        },
    );

    routes
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RoutingQualityDiagnostics {
    pub(crate) fallback_route_count: usize,
    pub(crate) overlap_proxy_count: usize,
    pub(crate) min_clearance_violation_count: usize,
}

#[cfg(test)]
#[derive(Debug, Default)]
struct RoutingQualityDiagnosticsAccumulator {
    fallback_route_count: usize,
    occupied_route_cells: BTreeSet<GridPoint>,
    overlap_cells: BTreeSet<GridPoint>,
    min_clearance_touch_pairs: BTreeSet<(GridPoint, GridPoint)>,
}

#[cfg(test)]
impl RoutingQualityDiagnosticsAccumulator {
    fn record_route(&mut self, route: &[GridPoint], used_fallback: bool) {
        if used_fallback {
            self.fallback_route_count = self.fallback_route_count.saturating_add(1);
        }

        let Some((start, goal)) = route.first().copied().zip(route.last().copied()) else {
            return;
        };
        let route_cells = expand_polyline_points(route)
            .into_iter()
            .filter(|point| *point != start && *point != goal)
            .collect::<BTreeSet<_>>();

        for point in route_cells.iter().copied() {
            if self.occupied_route_cells.contains(&point) {
                self.overlap_cells.insert(point);
            }
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let neighbor = point.offset(dx, dy);
                if !self.occupied_route_cells.contains(&neighbor) {
                    continue;
                }
                let pair = if point <= neighbor { (point, neighbor) } else { (neighbor, point) };
                self.min_clearance_touch_pairs.insert(pair);
            }
        }

        self.occupied_route_cells.extend(route_cells);
    }

    fn finish(self) -> RoutingQualityDiagnostics {
        RoutingQualityDiagnostics {
            fallback_route_count: self.fallback_route_count,
            overlap_proxy_count: self.overlap_cells.len(),
            min_clearance_violation_count: self.min_clearance_touch_pairs.len(),
        }
    }
}

#[cfg(test)]
pub(crate) fn route_flowchart_edges_orthogonal_with_diagnostics(
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
) -> (BTreeMap<ObjectId, Vec<GridPoint>>, RoutingQualityDiagnostics) {
    let edge_count = ast.edges().len();
    if edge_count == 0 {
        return (BTreeMap::new(), RoutingQualityDiagnostics::default());
    }

    let enable_soft_occupancy = enable_soft_occupancy(ast, layout);
    let mut routes = BTreeMap::<ObjectId, Vec<GridPoint>>::new();
    let mut diagnostics = RoutingQualityDiagnosticsAccumulator::default();

    route_flowchart_edges_orthogonal_record(
        ast,
        layout,
        enable_soft_occupancy,
        |_idx, edge_id, points, used_fallback| {
            diagnostics.record_route(&points, used_fallback);
            routes.insert(edge_id.clone(), points);
        },
    );

    (routes, diagnostics.finish())
}

fn enable_soft_occupancy(ast: &FlowchartAst, layout: &FlowchartLayout) -> bool {
    let edge_count = ast.edges().len();
    let node_count = layout.node_placements().len();
    node_count > 0 && edge_count >= 4 && (edge_count > node_count || edge_count * 2 >= node_count)
}

fn edges_in_stable_order_with_key_indices(
    ast: &FlowchartAst,
) -> Vec<(usize, &ObjectId, &crate::model::flow_ast::FlowEdge)> {
    let mut edges = ast
        .edges()
        .iter()
        .enumerate()
        .map(|(idx, (edge_id, edge))| (idx, edge_id, edge))
        .collect::<Vec<_>>();

    edges.sort_unstable_by(|(_idx_a, id_a, edge_a), (_idx_b, id_b, edge_b)| {
        cmp_edge_routing_order(id_a, edge_a, id_b, edge_b)
    });

    edges
}

fn mark_soft_occupancy(
    route: &[GridPoint],
    grid: RoutingGrid,
    occupancy: &mut [u8],
    occupied_nonzero: &mut usize,
) {
    let (Some(_start), Some(_goal)) = (route.first().copied(), route.last().copied()) else {
        return;
    };

    let mut mark_cell = |p: GridPoint, weight: u8| {
        if p.x % 2 == 0 && p.y % 2 == 0 {
            return;
        }
        let Some(idx) = grid.idx_of(p) else {
            return;
        };

        let before = occupancy[idx];
        if before >= 8 {
            return;
        }
        let after = before.saturating_add(weight).min(8);
        occupancy[idx] = after;
        if before == 0 && after > 0 {
            *occupied_nonzero = occupied_nonzero.saturating_add(1);
        }
    };

    for window in route.windows(2) {
        let a = window[0];
        let b = window[1];
        let dx = (b.x - a.x).signum();
        let dy = (b.y - a.y).signum();
        if dx != 0 && dy != 0 {
            continue;
        }
        let steps = (b.x - a.x).abs() + (b.y - a.y).abs();
        for step in 1..=steps {
            let p = a.offset(dx * step, dy * step);
            mark_cell(p, 2);
            mark_cell(p.offset(-1, 0), 1);
            mark_cell(p.offset(1, 0), 1);
            mark_cell(p.offset(0, -1), 1);
            mark_cell(p.offset(0, 1), 1);
        }
    }
}

fn route_flowchart_edges_orthogonal_record(
    ast: &FlowchartAst,
    layout: &FlowchartLayout,
    enable_soft_occupancy: bool,
    mut record: impl FnMut(usize, &ObjectId, Vec<GridPoint>, bool),
) {
    let projection = routing_obstacle_projection(layout);
    let max_bounds = projection.bounds.expand(24);

    let mut scratch = ShortestPathScratch::default();
    scratch.configure(max_bounds, &projection.obstacles);
    let grid = scratch.grid();
    let mut occupancy = vec![0u8; grid.len()];
    let mut occupied_nonzero = 0usize;

    for (key_idx, edge_id, edge) in edges_in_stable_order_with_key_indices(ast) {
        let start = layout.node_grid_point(edge.from_node_id());
        let goal = layout.node_grid_point(edge.to_node_id());

        let routed = match (start, goal) {
            (Some(start), Some(goal)) => {
                let edge_obstacles = edge_obstacles_with_endpoint_clearance(
                    &projection,
                    edge.from_node_id(),
                    edge.to_node_id(),
                );
                scratch.configure(max_bounds, &edge_obstacles);
                let bounds = edge_routing_bounds(start, goal);
                if enable_soft_occupancy && occupied_nonzero > 0 {
                    route_orthogonal_with_scratch_soft_occupancy_result(
                        start,
                        goal,
                        bounds,
                        &mut scratch,
                        &occupancy,
                    )
                } else {
                    route_orthogonal_with_scratch_result(start, goal, bounds, &mut scratch)
                }
            }
            (Some(start), None) => {
                RoutedPath { points: vec![start, start.offset(1, 0)], used_fallback: false }
            }
            (None, Some(goal)) => {
                RoutedPath { points: vec![goal.offset(-1, 0), goal], used_fallback: false }
            }
            (None, None) => RoutedPath {
                points: vec![GridPoint::new(0, 0), GridPoint::new(1, 0)],
                used_fallback: false,
            },
        };

        if enable_soft_occupancy {
            mark_soft_occupancy(&routed.points, grid, &mut occupancy, &mut occupied_nonzero);
        }
        record(key_idx, edge_id, routed.points, routed.used_fallback);
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        cmp_edge_routing_order, cmp_layer_nodes_by_barycenter, layout_flowchart,
        projected_node_obstacle_points, route_flowchart_edges_orthogonal,
        route_flowchart_edges_orthogonal_with_diagnostics, route_orthogonal,
        route_orthogonal_with_scratch, routing_obstacle_projection, shortest_path_4dir,
        shortest_path_4dir_soft_occupancy, FlowEdgeEndpoint, FlowchartLayout, FlowchartLayoutError,
        GridBounds, GridPoint, ShortestPathScratch,
    };
    use crate::model::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
    use crate::model::ids::ObjectId;

    fn oid(value: &str) -> ObjectId {
        ObjectId::new(value).expect("object id")
    }

    fn gp(x: i32, y: i32) -> GridPoint {
        GridPoint::new(x, y)
    }

    fn expanded_polyline_points(route: &[GridPoint]) -> Vec<GridPoint> {
        if route.len() < 2 {
            return route.to_vec();
        }

        let mut points = Vec::<GridPoint>::new();
        points.push(route[0]);
        for window in route.windows(2) {
            let a = window[0];
            let b = window[1];
            let dx = (b.x() - a.x()).signum();
            let dy = (b.y() - a.y()).signum();
            let steps = (b.x() - a.x()).abs().max((b.y() - a.y()).abs());
            for step in 1..=steps {
                points.push(gp(a.x() + (dx * step), a.y() + (dy * step)));
            }
        }
        points
    }

    fn turn_count(path: &[GridPoint]) -> usize {
        path.windows(3)
            .filter(|window| {
                let prev = (window[1].x() - window[0].x(), window[1].y() - window[0].y());
                let next = (window[2].x() - window[1].x(), window[2].y() - window[1].y());
                prev != next
            })
            .count()
    }

    #[test]
    fn assigns_layers_for_a_simple_dag() {
        let n_a = oid("n:a");
        let n_b = oid("n:b");
        let n_d = oid("n:d");

        let ast = crate::model::fixtures::flowchart_small_dag();

        let layout = layout_flowchart(&ast).expect("layout");

        let layers = layout
            .layers()
            .iter()
            .map(|layer| layer.iter().map(|id| id.as_str().to_owned()).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        assert_eq!(
            layers,
            vec![
                vec!["n:a".to_owned()],
                vec!["n:b".to_owned(), "n:c".to_owned()],
                vec!["n:d".to_owned()],
            ]
        );

        assert_eq!(layout.placement(&n_a).unwrap().layer(), 0);
        assert_eq!(layout.placement(&n_b).unwrap().layer(), 1);
        assert_eq!(layout.placement(&n_d).unwrap().layer(), 2);
    }

    #[test]
    fn orders_nodes_within_layer_using_barycenter_sweep() {
        let mut ast = FlowchartAst::default();
        let n_a = oid("n:a");
        let n_b = oid("n:b");
        let n_c = oid("n:c");
        let n_d = oid("n:d");
        let n_e = oid("n:e");

        ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
        ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
        ast.nodes_mut().insert(n_c.clone(), FlowNode::new("C"));
        ast.nodes_mut().insert(n_d.clone(), FlowNode::new("D"));
        ast.nodes_mut().insert(n_e.clone(), FlowNode::new("E"));

        ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));
        ast.edges_mut().insert(oid("e:ac"), FlowEdge::new(n_a.clone(), n_c.clone()));
        // These two edges would cross if layer 2 stayed in lexical order [n:d, n:e].
        ast.edges_mut().insert(oid("e:be"), FlowEdge::new(n_b.clone(), n_e.clone()));
        ast.edges_mut().insert(oid("e:cd"), FlowEdge::new(n_c.clone(), n_d.clone()));

        let layout = layout_flowchart(&ast).expect("layout");

        let layer2 = layout.layers()[2].iter().map(|id| id.as_str().to_owned()).collect::<Vec<_>>();
        assert_eq!(layer2, vec!["n:e".to_owned(), "n:d".to_owned()]);
    }

    #[test]
    fn barycenter_ties_break_by_node_id() {
        let n_prev_a = oid("n:prev:a");
        let n_prev_b = oid("n:prev:b");
        let n_d = oid("n:d");
        let n_e = oid("n:e");

        let prev_positions =
            BTreeMap::from([(n_prev_a.clone(), 0usize), (n_prev_b.clone(), 1usize)]);
        let predecessors = BTreeMap::from([
            (n_d.clone(), vec![n_prev_a.clone(), n_prev_b.clone()]),
            (n_e.clone(), vec![n_prev_a, n_prev_b]),
        ]);

        assert_eq!(
            cmp_layer_nodes_by_barycenter(&n_d, &n_e, &prev_positions, &predecessors),
            Ordering::Less
        );
        assert_eq!(
            cmp_layer_nodes_by_barycenter(&n_e, &n_d, &prev_positions, &predecessors),
            Ordering::Greater
        );
    }

    #[test]
    fn edge_order_ties_break_by_edge_id() {
        let n_a = oid("n:a");
        let n_b = oid("n:b");
        let edge = FlowEdge::new(n_a.clone(), n_b.clone());
        let edge_same = FlowEdge::new(n_a, n_b);
        let e_a = oid("e:a");
        let e_z = oid("e:z");

        assert_eq!(cmp_edge_routing_order(&e_a, &edge, &e_z, &edge_same), Ordering::Less);
        assert_eq!(cmp_edge_routing_order(&e_z, &edge_same, &e_a, &edge), Ordering::Greater);
    }

    #[test]
    fn errors_on_unknown_nodes_referenced_by_edges() {
        let mut ast = FlowchartAst::default();
        let n_a = oid("n:a");
        let n_missing = oid("n:missing");

        ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
        ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_missing.clone()));

        assert_eq!(
            layout_flowchart(&ast),
            Err(FlowchartLayoutError::UnknownNode {
                edge_id: oid("e:ab"),
                endpoint: FlowEdgeEndpoint::To,
                node_id: n_missing,
            })
        );
    }

    #[test]
    fn errors_on_cycles() {
        let mut ast = FlowchartAst::default();
        let n_a = oid("n:a");
        let n_b = oid("n:b");

        ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
        ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));

        ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));
        ast.edges_mut().insert(oid("e:ba"), FlowEdge::new(n_b.clone(), n_a.clone()));

        assert_eq!(
            layout_flowchart(&ast),
            Err(FlowchartLayoutError::CycleDetected { nodes: vec![oid("n:a"), oid("n:b")] })
        );
    }

    #[test]
    fn routes_edges_as_straight_lines_when_unobstructed() {
        let mut ast = FlowchartAst::default();
        let n_a = oid("n:a");
        let n_b = oid("n:b");

        ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
        ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
        ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a.clone(), n_b.clone()));

        let layout = layout_flowchart(&ast).expect("layout");
        let routes = route_flowchart_edges_orthogonal(&ast, &layout);
        let route = routes.get(&oid("e:ab")).expect("route");

        assert_eq!(
            route,
            &vec![layout.node_grid_point(&n_a).unwrap(), layout.node_grid_point(&n_b).unwrap()]
        );
    }

    #[test]
    fn orthogonal_router_constrains_intermediate_traversal_to_streets() {
        let start = gp(0, 0);
        let goal = gp(4, 0);
        let bounds = GridBounds { min_x: 0, max_x: 4, min_y: 0, max_y: 1 };

        let obstacles = BTreeSet::new();
        let mut scratch = ShortestPathScratch::default();
        scratch.configure(bounds, &obstacles);

        let path = shortest_path_4dir(start, goal, bounds, &mut scratch).expect("path");
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));

        for point in path.iter().skip(1).take(path.len().saturating_sub(2)) {
            assert!(
                !(point.x() % 2 == 0 && point.y() % 2 == 0),
                "intermediate point should be in a street: {point:?}"
            );
        }
    }

    #[test]
    fn routes_around_node_obstacles() {
        let n_a = oid("n:a");
        let n_b = oid("n:b");
        let n_d = oid("n:d");

        let ast = crate::model::fixtures::flowchart_obstacle_route();

        let layout = layout_flowchart(&ast).expect("layout");
        let routes = route_flowchart_edges_orthogonal(&ast, &layout);
        let route = routes.get(&oid("e:ad")).expect("route");

        let start = layout.node_grid_point(&n_a).unwrap();
        let goal = layout.node_grid_point(&n_d).unwrap();
        let obstacle = layout.node_grid_point(&n_b).unwrap();
        let obstacle_projection = projected_node_obstacle_points(obstacle);

        assert_eq!(route.first(), Some(&start));
        assert_eq!(route.last(), Some(&goal));

        for p in route.iter().skip(1).take(route.len().saturating_sub(2)) {
            assert_ne!(p, &obstacle);
            assert!(
                !obstacle_projection.contains(p),
                "route should avoid projected obstacle cell: {p:?}"
            );
        }
    }

    #[test]
    fn projected_node_obstacles_use_a_rectangle_around_anchor() {
        let projected = projected_node_obstacle_points(gp(4, 6));
        let expected = vec![
            gp(3, 5),
            gp(3, 6),
            gp(3, 7),
            gp(4, 5),
            gp(4, 6),
            gp(4, 7),
            gp(5, 5),
            gp(5, 6),
            gp(5, 7),
        ];
        assert_eq!(projected, expected);
    }

    #[test]
    fn routes_avoid_projected_non_endpoint_node_rectangles() {
        let ast = crate::model::fixtures::flowchart_obstacle_route();
        let n_b = oid("n:b");

        let layout = layout_flowchart(&ast).expect("layout");
        let routes = route_flowchart_edges_orthogonal(&ast, &layout);
        let route = routes.get(&oid("e:ad")).expect("route");

        let projection = routing_obstacle_projection(&layout);
        let blocked = projection
            .projected_by_node
            .get(&n_b)
            .expect("projected node rectangle")
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let expanded = expanded_polyline_points(route);
        let last_idx = expanded.len().saturating_sub(1);
        for (idx, point) in expanded.iter().enumerate() {
            if idx == 0 || idx == last_idx {
                continue;
            }
            assert!(
                !blocked.contains(point),
                "route entered projected rectangle of n:b at point {point:?}: {expanded:?}"
            );
        }
    }

    #[test]
    fn soft_occupancy_spreads_parallel_edges_across_detours() {
        let mut ast = FlowchartAst::default();
        let n_a = oid("n:a");
        let n_b = oid("n:b");
        let n_c = oid("n:c");

        ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
        ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
        ast.nodes_mut().insert(n_c.clone(), FlowNode::new("C"));

        // Two parallel edges (same endpoints) should be routed in stable order, and the second
        // should prefer an unoccupied detour once the first marks its route segments.
        let e_ac_first = oid("e:ac:a");
        let e_ac_second = oid("e:ac:z");
        ast.edges_mut().insert(e_ac_second.clone(), FlowEdge::new(n_a.clone(), n_c.clone()));
        ast.edges_mut().insert(e_ac_first.clone(), FlowEdge::new(n_a.clone(), n_c.clone()));

        // Add additional edges so `enable_soft_occupancy` is triggered (dense enough).
        ast.edges_mut().insert(oid("e:ba"), FlowEdge::new(n_b.clone(), n_a.clone()));
        ast.edges_mut().insert(oid("e:bc"), FlowEdge::new(n_b.clone(), n_c.clone()));

        // Manual layout: A (layer 0) -> B (layer 1) -> C (layer 2), single row.
        let layout = FlowchartLayout {
            layers: vec![vec![n_a.clone()], vec![n_b.clone()], vec![n_c.clone()]],
            node_placements: BTreeMap::from([
                (n_a.clone(), super::FlowNodePlacement { layer: 0, index_in_layer: 0 }),
                (n_b.clone(), super::FlowNodePlacement { layer: 1, index_in_layer: 0 }),
                (n_c.clone(), super::FlowNodePlacement { layer: 2, index_in_layer: 0 }),
            ]),
        };

        let routes = route_flowchart_edges_orthogonal(&ast, &layout);
        let first = routes.get(&e_ac_first).expect("first route");
        let second = routes.get(&e_ac_second).expect("second route");

        assert!(
            first.iter().any(|p| p.y() != 0),
            "expected first edge to detour off the primary row: {first:?}"
        );
        assert!(
            second.iter().any(|p| p.y() != 0),
            "expected second edge to detour off the primary row: {second:?}"
        );
        assert_ne!(first, second);
    }

    #[test]
    fn weighted_soft_occupancy_prefers_low_bend_monotonic_routes() {
        let start = gp(0, 0);
        let goal = gp(8, 4);
        let bounds = GridBounds { min_x: -2, max_x: 10, min_y: -2, max_y: 6 };
        let obstacles = BTreeSet::<GridPoint>::new();

        let mut scratch = ShortestPathScratch::default();
        scratch.configure(bounds.expand(24), &obstacles);
        let occupancy = vec![0u8; scratch.grid().len()];

        let path = shortest_path_4dir_soft_occupancy(start, goal, bounds, &mut scratch, &occupancy)
            .expect("path");
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
        assert!(
            turn_count(path) <= 3,
            "expected a low-bend route under readability weighting, got {path:?}",
        );
    }

    #[test]
    fn weighted_soft_occupancy_prefers_less_congested_detours() {
        let start = gp(0, 0);
        let goal = gp(6, 0);
        let bounds = GridBounds { min_x: -2, max_x: 8, min_y: -2, max_y: 2 };
        let obstacles = [gp(3, 0)].into_iter().collect::<BTreeSet<_>>();

        let mut scratch = ShortestPathScratch::default();
        scratch.configure(bounds.expand(24), &obstacles);
        let mut occupancy = vec![0u8; scratch.grid().len()];
        let grid = scratch.grid();

        // Penalize the upper detour corridor so the lower one is preferred.
        for point in [gp(1, 1), gp(2, 1), gp(3, 1), gp(4, 1), gp(5, 1)] {
            if let Some(idx) = grid.idx_of(point) {
                occupancy[idx] = 8;
            }
        }

        let path = shortest_path_4dir_soft_occupancy(start, goal, bounds, &mut scratch, &occupancy)
            .expect("path");
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
        assert!(
            path.iter().any(|p| p.y() < 0),
            "expected the route to take the uncongested lower detour: {path:?}",
        );
        assert!(
            !path.iter().any(|p| p.y() > 0),
            "route should avoid the congested upper detour: {path:?}",
        );
    }

    #[test]
    fn weighted_soft_occupancy_tie_breaks_are_deterministic() {
        let start = gp(0, 0);
        let goal = gp(6, 0);
        let bounds = GridBounds { min_x: -2, max_x: 8, min_y: -2, max_y: 2 };
        let obstacles = [gp(3, 0)].into_iter().collect::<BTreeSet<_>>();
        let mut scratch = ShortestPathScratch::default();
        scratch.configure(bounds.expand(24), &obstacles);
        let occupancy = vec![0u8; scratch.grid().len()];

        let baseline =
            shortest_path_4dir_soft_occupancy(start, goal, bounds, &mut scratch, &occupancy)
                .expect("baseline path")
                .to_vec();
        for _ in 0..32 {
            let next =
                shortest_path_4dir_soft_occupancy(start, goal, bounds, &mut scratch, &occupancy)
                    .expect("path")
                    .to_vec();
            assert_eq!(next, baseline);
        }
    }

    #[test]
    fn orthogonal_router_returns_deterministic_fallback_when_no_path_exists() {
        let start = gp(0, 0);
        let goal = gp(2, 2);

        // Block all start-adjacent points so the graph is disconnected.
        let obstacles =
            [gp(1, 0), gp(-1, 0), gp(0, 1), gp(0, -1)].into_iter().collect::<BTreeSet<_>>();

        let bounds = GridBounds { min_x: -2, max_x: 4, min_y: -2, max_y: 4 };

        let route = route_orthogonal(start, goal, bounds, &obstacles);

        assert_eq!(route, vec![start, gp(goal.x(), start.y()), goal]);
    }

    #[test]
    fn orthogonal_routing_is_deterministic_for_moderate_fixture() {
        let bounds = GridBounds { min_x: 0, max_x: 63, min_y: 0, max_y: 47 };

        let mut obstacles = BTreeSet::<GridPoint>::new();
        for x in 2..=63 {
            if x % 4 != 0 {
                continue;
            }
            for y in 1..=47 {
                if y % 11 == 0 {
                    continue;
                }
                obstacles.insert(gp(x, y));
            }
        }

        let pairs: [(GridPoint, GridPoint); 6] = [
            (gp(1, 1), gp(62, 46)),
            (gp(1, 46), gp(62, 1)),
            (gp(2, 10), gp(61, 10)),
            (gp(2, 20), gp(61, 20)),
            (gp(10, 5), gp(50, 40)),
            (gp(5, 40), gp(55, 10)),
        ];

        for (start, goal) in pairs.iter() {
            obstacles.remove(start);
            obstacles.remove(goal);
        }

        let mut scratch = ShortestPathScratch::default();
        scratch.configure(bounds.expand(24), &obstacles);
        let first = pairs
            .iter()
            .copied()
            .map(|(start, goal)| route_orthogonal_with_scratch(start, goal, bounds, &mut scratch))
            .collect::<Vec<_>>();
        let second = pairs
            .iter()
            .copied()
            .map(|(start, goal)| route_orthogonal_with_scratch(start, goal, bounds, &mut scratch))
            .collect::<Vec<_>>();
        assert_eq!(first, second);

        let mut scratch_fresh = ShortestPathScratch::default();
        scratch_fresh.configure(bounds.expand(24), &obstacles);
        let third = pairs
            .iter()
            .copied()
            .map(|(start, goal)| {
                route_orthogonal_with_scratch(start, goal, bounds, &mut scratch_fresh)
            })
            .collect::<Vec<_>>();
        assert_eq!(first, third);
    }

    #[test]
    fn flowchart_routing_with_projected_obstacles_is_deterministic() {
        let ast = crate::model::fixtures::flowchart_node_overlap_avoidance_regression();
        let layout = layout_flowchart(&ast).expect("layout");

        let baseline = route_flowchart_edges_orthogonal(&ast, &layout);
        for _ in 0..32 {
            let next = route_flowchart_edges_orthogonal(&ast, &layout);
            assert_eq!(next, baseline);
        }
    }

    #[test]
    fn flowchart_routing_quality_diagnostics_are_deterministic_for_projected_fixture() {
        let ast = crate::model::fixtures::flowchart_node_overlap_avoidance_regression();
        let layout = layout_flowchart(&ast).expect("layout");

        let (baseline_routes, baseline_diagnostics) =
            route_flowchart_edges_orthogonal_with_diagnostics(&ast, &layout);
        for _ in 0..32 {
            let (next_routes, next_diagnostics) =
                route_flowchart_edges_orthogonal_with_diagnostics(&ast, &layout);
            assert_eq!(next_routes, baseline_routes);
            assert_eq!(next_diagnostics, baseline_diagnostics);
        }

        assert_eq!(baseline_diagnostics.fallback_route_count, 0);
        assert_eq!(baseline_diagnostics.overlap_proxy_count, 0);
        assert_eq!(baseline_diagnostics.min_clearance_violation_count, 0);
    }

    #[test]
    fn does_not_panic_when_layout_is_missing_edge_endpoint_placements() {
        let mut ast = FlowchartAst::default();
        let n_a = oid("n:a");
        let n_b = oid("n:b");

        ast.nodes_mut().insert(n_a.clone(), FlowNode::new("A"));
        ast.nodes_mut().insert(n_b.clone(), FlowNode::new("B"));
        ast.edges_mut().insert(oid("e:ab"), FlowEdge::new(n_a, n_b));

        let layout = FlowchartLayout { layers: Vec::new(), node_placements: BTreeMap::new() };

        let routes = route_flowchart_edges_orthogonal(&ast, &layout);
        let route = routes.get(&oid("e:ab")).expect("route");

        assert_eq!(route, &vec![gp(0, 0), gp(1, 0)]);
    }
}
