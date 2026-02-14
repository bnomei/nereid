// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};

use nereid::model::{Diagram, DiagramAst, DiagramId, ObjectId, SequenceMessageKind};
use nereid::ops::{apply_ops, ApplyResult, FlowOp, Op, SeqOp};

mod fixtures;
mod profiler;

// Benchmark identity (keep stable):
// - Group name in this file: `ops.apply`
// - Case IDs (the string after the `/`) must remain stable across refactors so
//   results stay comparable over time (e.g. `small`, `medium_dense`, `large_long_labels`).
// - If implementations move/deduplicate, update the wiring but do not rename
//   group or case IDs.
fn checksum_apply_result(result: &ApplyResult) -> u64 {
    let mut acc = 0u64;
    acc = acc.wrapping_mul(131).wrapping_add(result.new_rev);
    acc = acc.wrapping_mul(131).wrapping_add(result.applied as u64);
    acc = acc
        .wrapping_mul(131)
        .wrapping_add(result.delta.added.len() as u64);
    acc = acc
        .wrapping_mul(131)
        .wrapping_add(result.delta.updated.len() as u64);
    acc = acc
        .wrapping_mul(131)
        .wrapping_add(result.delta.removed.len() as u64);
    acc
}

fn seq_add_message_ops(participants: &[ObjectId], order_key_base: i64, count: usize) -> Vec<Op> {
    assert!(
        participants.len() >= 2,
        "sequence fixture must contain >= 2 participants"
    );

    let mut ops = Vec::with_capacity(count);
    for idx in 0..count {
        let from = participants[idx % participants.len()].clone();
        let to = participants[(idx + 1) % participants.len()].clone();
        let message_id = ObjectId::new(format!("bench_msg_{idx:06}")).expect("message id");
        let kind = match idx % 3 {
            0 => SequenceMessageKind::Sync,
            1 => SequenceMessageKind::Async,
            _ => SequenceMessageKind::Return,
        };
        let text = format!("bench_msg_{idx:06}");
        let order_key = order_key_base.saturating_add((idx as i64).saturating_mul(1000));
        ops.push(Op::Seq(SeqOp::AddMessage {
            message_id,
            from_participant_id: from,
            to_participant_id: to,
            kind,
            arrow: None,
            text,
            order_key,
        }));
    }
    ops
}

fn flow_add_edge_ops(nodes: &[ObjectId], count: usize) -> Vec<Op> {
    assert!(nodes.len() >= 2, "flow fixture must contain >= 2 nodes");

    let mut ops = Vec::with_capacity(count);
    for idx in 0..count {
        let from_index = (idx.wrapping_mul(7)) % nodes.len();
        let mut to_index = (idx.wrapping_mul(7).wrapping_add(3)) % nodes.len();
        if to_index == from_index {
            to_index = (to_index + 1) % nodes.len();
        }

        let from = nodes[from_index].clone();
        let to = nodes[to_index].clone();

        let edge_id = ObjectId::new(format!("bench_edge_{idx:06}")).expect("edge id");
        ops.push(Op::Flow(FlowOp::AddEdge {
            edge_id,
            from_node_id: from,
            to_node_id: to,
            label: None,
            connector: None,
            style: None,
        }));
    }
    ops
}

fn benches_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("ops.apply");

    // Sequence ops (baseline: deterministic medium sequence fixture).
    let seq_ast = fixtures::seq::fixture(fixtures::seq::Case::Medium);
    let seq_participants = seq_ast.participants().keys().cloned().collect::<Vec<_>>();
    let seq_order_key_base = seq_ast
        .messages()
        .iter()
        .map(|m| m.order_key())
        .max()
        .unwrap_or(0)
        .saturating_add(1000);
    let seq_template = Diagram::new(
        DiagramId::new("bench:ops:seq").expect("diagram id"),
        "bench_seq",
        DiagramAst::Sequence(seq_ast),
    );

    let seq_ops_single = seq_add_message_ops(&seq_participants, seq_order_key_base, 1);
    let seq_ops_batch_10 = seq_add_message_ops(&seq_participants, seq_order_key_base, 10);
    let seq_ops_batch_200 = seq_add_message_ops(&seq_participants, seq_order_key_base, 200);

    group.throughput(Throughput::Elements(seq_ops_single.len() as u64));
    group.bench_function("seq_single", {
        let template = seq_template.clone();
        move |b| {
            b.iter_batched(
                || template.clone(),
                |mut diagram| {
                    let base_rev = diagram.rev();
                    let result = apply_ops(&mut diagram, base_rev, black_box(&seq_ops_single))
                        .expect("apply_ops");
                    black_box(checksum_apply_result(&result))
                },
                BatchSize::SmallInput,
            )
        }
    });

    group.throughput(Throughput::Elements(seq_ops_batch_10.len() as u64));
    group.bench_function("seq_batch_10", {
        let template = seq_template.clone();
        move |b| {
            b.iter_batched(
                || template.clone(),
                |mut diagram| {
                    let base_rev = diagram.rev();
                    let result = apply_ops(&mut diagram, base_rev, black_box(&seq_ops_batch_10))
                        .expect("apply_ops");
                    black_box(checksum_apply_result(&result))
                },
                BatchSize::SmallInput,
            )
        }
    });

    group.throughput(Throughput::Elements(seq_ops_batch_200.len() as u64));
    group.bench_function("seq_batch_200", {
        let template = seq_template.clone();
        move |b| {
            b.iter_batched(
                || template.clone(),
                |mut diagram| {
                    let base_rev = diagram.rev();
                    let result = apply_ops(&mut diagram, base_rev, black_box(&seq_ops_batch_200))
                        .expect("apply_ops");
                    black_box(checksum_apply_result(&result))
                },
                BatchSize::SmallInput,
            )
        }
    });

    // Flow ops (baseline: deterministic medium_dense flow fixture).
    let flow_ast = fixtures::flow::fixture(fixtures::flow::Case::MediumDense);
    let flow_nodes = flow_ast.nodes().keys().cloned().collect::<Vec<_>>();
    let flow_template = Diagram::new(
        DiagramId::new("bench:ops:flow").expect("diagram id"),
        "bench_flow",
        DiagramAst::Flowchart(flow_ast),
    );

    let flow_ops_single = flow_add_edge_ops(&flow_nodes, 1);
    let flow_ops_batch_10 = flow_add_edge_ops(&flow_nodes, 10);
    let flow_ops_batch_200 = flow_add_edge_ops(&flow_nodes, 200);

    group.throughput(Throughput::Elements(flow_ops_single.len() as u64));
    group.bench_function("flow_single", {
        let template = flow_template.clone();
        move |b| {
            b.iter_batched(
                || template.clone(),
                |mut diagram| {
                    let base_rev = diagram.rev();
                    let result = apply_ops(&mut diagram, base_rev, black_box(&flow_ops_single))
                        .expect("apply_ops");
                    black_box(checksum_apply_result(&result))
                },
                BatchSize::SmallInput,
            )
        }
    });

    group.throughput(Throughput::Elements(flow_ops_batch_10.len() as u64));
    group.bench_function("flow_batch_10", {
        let template = flow_template.clone();
        move |b| {
            b.iter_batched(
                || template.clone(),
                |mut diagram| {
                    let base_rev = diagram.rev();
                    let result = apply_ops(&mut diagram, base_rev, black_box(&flow_ops_batch_10))
                        .expect("apply_ops");
                    black_box(checksum_apply_result(&result))
                },
                BatchSize::SmallInput,
            )
        }
    });

    group.throughput(Throughput::Elements(flow_ops_batch_200.len() as u64));
    group.bench_function("flow_batch_200", {
        let template = flow_template.clone();
        move |b| {
            b.iter_batched(
                || template.clone(),
                |mut diagram| {
                    let base_rev = diagram.rev();
                    let result = apply_ops(&mut diagram, base_rev, black_box(&flow_ops_batch_200))
                        .expect("apply_ops");
                    black_box(checksum_apply_result(&result))
                },
                BatchSize::SmallInput,
            )
        }
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_ops
}
criterion_main!(benches);
