//! 几何推理模块性能基准测试
//!
//! 测试 R-tree 空间索引优化的效果

use cadagent::cad_reasoning::{GeometricRelationReasoner, ReasoningConfig};
use cadagent::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// 创建密集排列的线段（测试空间索引）
fn create_dense_grid(size: usize) -> Vec<Primitive> {
    let mut primitives = Vec::new();

    // 垂直线
    for i in 0..=size {
        let x = i as f64 * 10.0;
        primitives.push(Primitive::Line(Line::from_coords(
            [x, 0.0],
            [x, size as f64 * 10.0],
        )));
    }

    // 水平线
    for i in 0..=size {
        let y = i as f64 * 10.0;
        primitives.push(Primitive::Line(Line::from_coords(
            [0.0, y],
            [size as f64 * 10.0, y],
        )));
    }

    primitives
}

/// 创建随机线段（更真实的场景）
fn create_random_lines(count: usize, seed: u64) -> Vec<Primitive> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut primitives = Vec::new();

    for i in 0..count {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        i.hash(&mut hasher);
        let hash = hasher.finish();

        let x1 = ((hash % 1000) as f64) / 10.0;
        let y1 = (((hash >> 10) % 1000) as f64) / 10.0;
        let x2 = (((hash >> 20) % 1000) as f64) / 10.0;
        let y2 = (((hash >> 30) % 1000) as f64) / 10.0;

        primitives.push(Primitive::Line(Line::from_coords([x1, y1], [x2, y2])));
    }

    primitives
}

/// 创建房间布局（真实场景模拟）
fn create_room_layout() -> Vec<Primitive> {
    let mut primitives = Vec::new();

    // 外墙
    primitives.extend(create_rectangle(0.0, 0.0, 500.0, 400.0));

    // 内墙（房间分隔）
    primitives.push(Primitive::Line(Line::from_coords(
        [200.0, 0.0],
        [200.0, 250.0],
    )));
    primitives.push(Primitive::Line(Line::from_coords(
        [0.0, 250.0],
        [500.0, 250.0],
    )));
    primitives.push(Primitive::Line(Line::from_coords(
        [300.0, 250.0],
        [300.0, 400.0],
    )));

    // 门窗（用小线段表示）
    primitives.push(Primitive::Line(Line::from_coords(
        [100.0, 0.0],
        [180.0, 0.0],
    ))); // 门
    primitives.push(Primitive::Line(Line::from_coords(
        [350.0, 0.0],
        [450.0, 0.0],
    ))); // 窗

    primitives
}

fn create_rectangle(x: f64, y: f64, width: f64, height: f64) -> Vec<Primitive> {
    vec![
        Primitive::Line(Line::from_coords([x, y], [x + width, y])),
        Primitive::Line(Line::from_coords([x + width, y], [x + width, y + height])),
        Primitive::Line(Line::from_coords([x + width, y + height], [x, y + height])),
        Primitive::Line(Line::from_coords([x, y + height], [x, y])),
    ]
}

fn bench_grid_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("grid_detection");

    for size in [5, 10, 15, 20].iter() {
        let primitives = create_dense_grid(*size);
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            &primitives,
            |b, primitives| b.iter(|| reasoner.find_all_relations(black_box(primitives))),
        );
    }
    group.finish();
}

fn bench_random_lines_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_lines_detection");

    for count in [50, 100, 200, 500].iter() {
        let primitives = create_random_lines(*count, 42);
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &primitives,
            |b, primitives| b.iter(|| reasoner.find_all_relations(black_box(primitives))),
        );
    }
    group.finish();
}

fn bench_room_layout_analysis(c: &mut Criterion) {
    let primitives = create_room_layout();
    let reasoner = GeometricRelationReasoner::with_defaults();

    c.bench_function("room_layout_analysis", |b| {
        b.iter(|| reasoner.find_all_relations(black_box(&primitives)))
    });
}

fn bench_rtree_threshold(c: &mut Criterion) {
    let mut group = c.benchmark_group("rtree_threshold");

    // 测试 R-tree 阈值附近的性能变化
    for count in [45, 48, 50, 52, 55, 60].iter() {
        let primitives = create_random_lines(*count, 123);
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &primitives,
            |b, primitives| b.iter(|| reasoner.find_all_relations(black_box(primitives))),
        );
    }
    group.finish();
}

fn bench_all_relations_types(c: &mut Criterion) {
    let primitives = create_room_layout();
    let mut config = ReasoningConfig::default();

    let mut group = c.benchmark_group("all_relation_types");

    // 测试启用不同关系检测的性能
    config.detect_parallel = true;
    config.detect_perpendicular = true;
    config.detect_collinear = true;
    config.detect_tangent = false;
    config.detect_concentric = false;
    config.detect_connected = true;
    config.detect_symmetric = false;

    let reasoner = GeometricRelationReasoner::new(config);

    group.bench_function("standard_detection", |b| {
        b.iter(|| reasoner.find_all_relations(black_box(&primitives)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_grid_detection,
    bench_random_lines_detection,
    bench_room_layout_analysis,
    bench_rtree_threshold,
    bench_all_relations_types,
);

criterion_main!(benches);
