// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use crate::model::flow_ast::FlowchartAst;
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
        tos.sort();
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
    layer_nodes.sort_by(|a, b| {
        let bary_a = predecessors
            .get(a)
            .map(|preds| {
                preds
                    .iter()
                    .filter_map(|p| prev_positions.get(p).copied())
                    .fold((0usize, 0usize), |(sum, count), pos| (sum + pos, count + 1))
            })
            .and_then(|(sum, count)| (count > 0).then_some((sum, count)));
        let bary_b = predecessors
            .get(b)
            .map(|preds| {
                preds
                    .iter()
                    .filter_map(|p| prev_positions.get(p).copied())
                    .fold((0usize, 0usize), |(sum, count), pos| (sum + pos, count + 1))
            })
            .and_then(|(sum, count)| (count > 0).then_some((sum, count)));

        match (bary_a, bary_b) {
            (None, None) => a.cmp(b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some((sum_a, count_a)), Some((sum_b, count_b))) => {
                // Compare sum_a/count_a vs sum_b/count_b without floats.
                let left = (sum_a as u128) * (count_b as u128);
                let right = (sum_b as u128) * (count_a as u128);
                left.cmp(&right).then_with(|| a.cmp(b))
            }
        }
    });
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
        tos.sort();
    }
    for preds in predecessors.values_mut() {
        preds.sort();
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
        layer_nodes.sort();
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

fn routing_bounds_and_obstacles(layout: &FlowchartLayout) -> (GridBounds, BTreeSet<GridPoint>) {
    let obstacles = layout
        .node_placements()
        .values()
        .map(|placement| {
            GridPoint::new((placement.layer() * 2) as i32, (placement.index_in_layer() * 2) as i32)
        })
        .collect::<BTreeSet<_>>();

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

    (bounds, obstacles)
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

    fn dist(&self, idx: usize, gen: u32) -> u32 {
        if self.dist_gen[idx] == gen {
            self.dist_cost[idx]
        } else {
            u32::MAX
        }
    }

    fn set_dist(&mut self, idx: usize, gen: u32, cost: u32, came_from: i32) {
        self.dist_gen[idx] = gen;
        self.dist_cost[idx] = cost;
        self.came_from[idx] = came_from;
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
    const OCCUPANCY_PENALTY: u32 = 20;

    #[inline(always)]
    fn step_cost(idx: usize, occupancy: &[u8]) -> u32 {
        1u32 + u32::from(occupancy[idx]) * OCCUPANCY_PENALTY
    }

    fn best_diag_midpoint_and_cost(
        from: GridPoint,
        to: GridPoint,
        to_idx: usize,
        bounds: GridBounds,
        grid: RoutingGrid,
        occupancy: &[u8],
    ) -> Option<(GridPoint, u32)> {
        let candidates = [GridPoint::new(from.x, to.y), GridPoint::new(to.x, from.y)];

        let mut best: Option<(u32, GridPoint)> = None;
        for midpoint in candidates {
            if !bounds.contains(midpoint) {
                continue;
            }
            let Some(mid_idx) = grid.idx_of(midpoint) else {
                continue;
            };
            let cost = step_cost(mid_idx, occupancy) + step_cost(to_idx, occupancy);
            match best {
                None => best = Some((cost, midpoint)),
                Some((best_cost, _)) if cost < best_cost => best = Some((cost, midpoint)),
                Some((best_cost, best_mid)) if cost == best_cost && midpoint < best_mid => {
                    best = Some((cost, midpoint));
                }
                _ => {}
            }
        }

        best.map(|(cost, midpoint)| (midpoint, cost))
    }

    fn best_diag_cost(
        from: GridPoint,
        to: GridPoint,
        to_idx: usize,
        bounds: GridBounds,
        grid: RoutingGrid,
        occupancy: &[u8],
    ) -> Option<u32> {
        let (_midpoint, cost) =
            best_diag_midpoint_and_cost(from, to, to_idx, bounds, grid, occupancy)?;
        Some(cost)
    }

    #[allow(clippy::too_many_arguments)]
    fn reconstruct_path<'a>(
        start: GridPoint,
        goal: GridPoint,
        start_idx: usize,
        goal_idx: usize,
        bounds: GridBounds,
        grid: RoutingGrid,
        scratch: &'a mut ShortestPathScratch,
        occupancy: &[u8],
    ) -> Option<&'a [GridPoint]> {
        scratch.path.clear();
        scratch.path.push(goal);

        let mut cursor_idx = goal_idx;
        while cursor_idx != start_idx {
            let prev_idx = scratch.came_from[cursor_idx] as isize;
            if prev_idx < 0 {
                return None;
            }
            let prev_idx = prev_idx as usize;

            let from_idx = prev_idx;
            let to_idx = cursor_idx;
            let from = grid.point_of(from_idx);
            let to = grid.point_of(to_idx);

            let dx = (to.x - from.x).abs();
            let dy = (to.y - from.y).abs();

            if (dx == 2 && dy == 0) || (dx == 0 && dy == 2) {
                let midpoint = GridPoint::new((from.x + to.x) / 2, (from.y + to.y) / 2);
                scratch.path.push(midpoint);
                scratch.path.push(from);
                cursor_idx = prev_idx;
                continue;
            }

            if dx == 1 && dy == 1 {
                let (midpoint, _cost) =
                    best_diag_midpoint_and_cost(from, to, to_idx, bounds, grid, occupancy)?;
                scratch.path.push(midpoint);
                scratch.path.push(from);
                cursor_idx = prev_idx;
                continue;
            }

            return None;
        }

        debug_assert_eq!(scratch.path.last().copied(), Some(start));
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

    let gen = scratch.begin();
    scratch.set_dist(start_idx, gen, 0, -1);
    let h0 = start.x.abs_diff(goal.x) + start.y.abs_diff(goal.y);
    scratch.heap.push(Reverse((h0, 0u32, 0u32, start_idx as u32)));

    let mut tie_seq = 1u32;
    let mut best_goal_cost = u32::MAX;

    while let Some(Reverse((f_cost, g_cost, _tie, idx))) = scratch.heap.pop() {
        let idx = idx as usize;
        let cost = scratch.dist(idx, gen);
        if g_cost != cost {
            continue;
        }

        if best_goal_cost != u32::MAX && f_cost >= best_goal_cost {
            break;
        }

        if idx == goal_idx {
            return reconstruct_path(
                start, goal, start_idx, goal_idx, bounds, grid, scratch, occupancy,
            );
        }

        let current = grid.point_of(idx);
        if idx == start_idx {
            let dx = (goal.x - current.x).abs();
            let dy = (goal.y - current.y).abs();
            if (dx == 2 && dy == 0) || (dx == 0 && dy == 2) {
                let midpoint = GridPoint::new((current.x + goal.x) / 2, (current.y + goal.y) / 2);
                if bounds.contains(midpoint) {
                    let mid_idx = grid.idx_of(midpoint)?;
                    let edge_cost = step_cost(mid_idx, occupancy) + 1;
                    let next_cost = cost + edge_cost;
                    if next_cost < scratch.dist(goal_idx, gen) {
                        best_goal_cost = next_cost;
                        scratch.set_dist(goal_idx, gen, next_cost, idx as i32);
                        scratch.heap.push(Reverse((
                            next_cost,
                            next_cost,
                            tie_seq,
                            goal_idx as u32,
                        )));
                        tie_seq = tie_seq.wrapping_add(1);
                    }
                }
            }

            for (sx, sy) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
                let next = current.offset(sx, sy);
                if !bounds.contains(next) {
                    continue;
                }
                if (next.x & 1) == 0 || (next.y & 1) == 0 {
                    continue;
                }
                let h = next.x.abs_diff(goal.x) + next.y.abs_diff(goal.y);
                if best_goal_cost != u32::MAX && cost + 2 + h >= best_goal_cost {
                    continue;
                }
                let Some(next_idx) = grid.idx_of(next) else {
                    continue;
                };
                let Some(edge_cost) =
                    best_diag_cost(current, next, next_idx, bounds, grid, occupancy)
                else {
                    continue;
                };
                let next_cost = cost + edge_cost;
                if next_cost < scratch.dist(next_idx, gen) {
                    let f_cost = next_cost + h;
                    if best_goal_cost != u32::MAX && f_cost >= best_goal_cost {
                        continue;
                    }
                    scratch.set_dist(next_idx, gen, next_cost, idx as i32);
                    scratch.heap.push(Reverse((f_cost, next_cost, tie_seq, next_idx as u32)));
                    tie_seq = tie_seq.wrapping_add(1);
                }
            }

            continue;
        }

        let dx_to_goal = (goal.x - current.x).abs();
        let dy_to_goal = (goal.y - current.y).abs();
        if dx_to_goal == 1 && dy_to_goal == 1 {
            if let Some(edge_cost) =
                best_diag_cost(current, goal, goal_idx, bounds, grid, occupancy)
            {
                let next_cost = cost + edge_cost;
                if next_cost < scratch.dist(goal_idx, gen) {
                    best_goal_cost = next_cost;
                    scratch.set_dist(goal_idx, gen, next_cost, idx as i32);
                    scratch.heap.push(Reverse((next_cost, next_cost, tie_seq, goal_idx as u32)));
                    tie_seq = tie_seq.wrapping_add(1);
                }
            }
        }

        let idx_i = idx as isize;
        let stride = grid.width as isize;
        for (dx, dy) in neighbor_deltas2_towards(current, goal) {
            let next_x = current.x + dx;
            let next_y = current.y + dy;
            if next_x < bounds.min_x
                || next_x > bounds.max_x
                || next_y < bounds.min_y
                || next_y > bounds.max_y
            {
                continue;
            }

            let h = next_x.abs_diff(goal.x) + next_y.abs_diff(goal.y);
            if best_goal_cost != u32::MAX && cost + 2 + h >= best_goal_cost {
                continue;
            }

            let (next_offset, mid_offset) = match (dx, dy) {
                (2, 0) => (2, 1),
                (-2, 0) => (-2, -1),
                (0, 2) => (2 * stride, stride),
                (0, -2) => (-2 * stride, -stride),
                _ => continue,
            };
            let next_idx = (idx_i + next_offset) as usize;
            let mid_idx = (idx_i + mid_offset) as usize;
            debug_assert!(next_idx < grid.len());
            debug_assert!(mid_idx < grid.len());

            let edge_cost = step_cost(mid_idx, occupancy) + step_cost(next_idx, occupancy);
            let next_cost = cost + edge_cost;
            if next_cost < scratch.dist(next_idx, gen) {
                let f_cost = next_cost + h;
                if best_goal_cost != u32::MAX && f_cost >= best_goal_cost {
                    continue;
                }
                scratch.set_dist(next_idx, gen, next_cost, idx as i32);
                scratch.heap.push(Reverse((f_cost, next_cost, tie_seq, next_idx as u32)));
                tie_seq = tie_seq.wrapping_add(1);
            }
        }
    }

    if best_goal_cost == u32::MAX {
        return None;
    }

    reconstruct_path(start, goal, start_idx, goal_idx, bounds, grid, scratch, occupancy)
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

fn route_orthogonal_with_scratch(
    start: GridPoint,
    goal: GridPoint,
    base_bounds: GridBounds,
    scratch: &mut ShortestPathScratch,
) -> Vec<GridPoint> {
    if let Some(path) = shortest_path_4dir(start, goal, base_bounds, scratch) {
        return compress_to_polyline(path);
    }

    // Historically this router expanded bounds in 4-unit steps up to a max of
    // `base_bounds.expand(20)`. Keep the same maximum search window, but avoid
    // re-running the pathfinder multiple times on failures.
    let expanded = base_bounds.expand(20);
    if let Some(path) = shortest_path_4dir(start, goal, expanded, scratch) {
        return compress_to_polyline(path);
    }

    fallback_polyline(start, goal)
}

fn route_orthogonal_with_scratch_soft_occupancy(
    start: GridPoint,
    goal: GridPoint,
    base_bounds: GridBounds,
    scratch: &mut ShortestPathScratch,
    occupancy: &[u8],
) -> Vec<GridPoint> {
    if let Some(path) =
        shortest_path_4dir_soft_occupancy(start, goal, base_bounds, scratch, occupancy)
    {
        return compress_to_polyline(path);
    }

    let expanded = base_bounds.expand(20);
    if let Some(path) = shortest_path_4dir_soft_occupancy(start, goal, expanded, scratch, occupancy)
    {
        return compress_to_polyline(path);
    }

    fallback_polyline(start, goal)
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
        |idx, _edge_id, points| {
            routes[idx] = points;
        },
    );

    routes
}

fn enable_soft_occupancy(ast: &FlowchartAst, layout: &FlowchartLayout) -> bool {
    let edge_count = ast.edges().len();
    let node_count = layout.node_placements().len();
    node_count > 0 && edge_count > node_count
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
        edge_a
            .from_node_id()
            .cmp(edge_b.from_node_id())
            .then_with(|| edge_a.to_node_id().cmp(edge_b.to_node_id()))
            .then_with(|| id_a.cmp(id_b))
    });

    edges
}

fn mark_soft_occupancy(
    route: &[GridPoint],
    grid: RoutingGrid,
    occupancy: &mut [u8],
    occupied_nonzero: &mut usize,
) {
    let (Some(start), Some(goal)) = (route.first().copied(), route.last().copied()) else {
        return;
    };

    let mut mark_cell = |p: GridPoint, weight: u8| {
        if p == start || p == goal {
            return;
        }
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
    mut record: impl FnMut(usize, &ObjectId, Vec<GridPoint>),
) {
    let (bounds, obstacles) = routing_bounds_and_obstacles(layout);
    let max_bounds = bounds.expand(24);

    let mut scratch = ShortestPathScratch::default();
    scratch.configure(max_bounds, &obstacles);
    let grid = scratch.grid();
    let mut occupancy = vec![0u8; grid.len()];
    let mut occupied_nonzero = 0usize;

    for (key_idx, edge_id, edge) in edges_in_stable_order_with_key_indices(ast) {
        let start = layout.node_grid_point(edge.from_node_id());
        let goal = layout.node_grid_point(edge.to_node_id());

        let points = match (start, goal) {
            (Some(start), Some(goal)) => {
                let bounds = edge_routing_bounds(start, goal);
                if enable_soft_occupancy && occupied_nonzero > 0 {
                    route_orthogonal_with_scratch_soft_occupancy(
                        start,
                        goal,
                        bounds,
                        &mut scratch,
                        &occupancy,
                    )
                } else {
                    route_orthogonal_with_scratch(start, goal, bounds, &mut scratch)
                }
            }
            (Some(start), None) => vec![start, start.offset(1, 0)],
            (None, Some(goal)) => vec![goal.offset(-1, 0), goal],
            (None, None) => vec![GridPoint::new(0, 0), GridPoint::new(1, 0)],
        };

        if enable_soft_occupancy {
            mark_soft_occupancy(&points, grid, &mut occupancy, &mut occupied_nonzero);
        }
        record(key_idx, edge_id, points);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        layout_flowchart, route_flowchart_edges_orthogonal, route_orthogonal,
        route_orthogonal_with_scratch, shortest_path_4dir, FlowEdgeEndpoint, FlowchartLayout,
        FlowchartLayoutError, GridBounds, GridPoint, ShortestPathScratch,
    };
    use crate::model::flow_ast::{FlowEdge, FlowNode, FlowchartAst};
    use crate::model::ids::ObjectId;

    fn oid(value: &str) -> ObjectId {
        ObjectId::new(value).expect("object id")
    }

    fn gp(x: i32, y: i32) -> GridPoint {
        GridPoint::new(x, y)
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

        assert_eq!(
            route,
            &vec![
                start,
                gp(start.x() + 1, start.y()),
                gp(start.x() + 1, start.y() + 1),
                gp(goal.x(), start.y() + 1),
                goal
            ]
        );

        for p in route.iter().skip(1).take(route.len().saturating_sub(2)) {
            assert_ne!(p, &obstacle);
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
        ast.edges_mut().insert(oid("e:ac1"), FlowEdge::new(n_a.clone(), n_c.clone()));
        ast.edges_mut().insert(oid("e:ac2"), FlowEdge::new(n_a.clone(), n_c.clone()));

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
        let first = routes.get(&oid("e:ac1")).expect("first route");
        let second = routes.get(&oid("e:ac2")).expect("second route");

        assert!(
            first.iter().any(|p| p.y() == 1),
            "expected first edge to detour via y=1: {first:?}"
        );
        assert!(
            second.iter().any(|p| p.y() == -1),
            "expected second edge to detour via y=-1: {second:?}"
        );
        assert_ne!(first, second);
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
