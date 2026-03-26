//! 几何模块性能基准测试
//!
//! 测试关键几何操作的性能

use cadagent::cad_reasoning::GeometricRelationReasoner;
use cadagent::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// 创建测试基元（矩形）
fn create_rectangle(x: f64, y: f64, width: f64, height: f64) -> Vec<Primitive> {
    vec![
        Primitive::Line(Line::from_coords([x, y], [x + width, y])),
        Primitive::Line(Line::from_coords([x + width, y], [x + width, y + height])),
        Primitive::Line(Line::from_coords([x + width, y + height], [x, y + height])),
        Primitive::Line(Line::from_coords([x, y + height], [x, y])),
    ]
}

/// 创建多个矩形（模拟房间）
fn create_multiple_rectangles(count: usize) -> Vec<Primitive> {
    let mut primitives = Vec::new();
    for i in 0..count {
        let x = (i % 5) as f64 * 150.0;
        let y = (i / 5) as f64 * 150.0;
        primitives.extend(create_rectangle(x, y, 100.0, 100.0));
    }
    primitives
}

/// 创建平行线组
fn create_parallel_lines(count: usize) -> Vec<Primitive> {
    (0..count)
        .map(|i| {
            Primitive::Line(Line::from_coords(
                [i as f64 * 10.0, 0.0],
                [i as f64 * 10.0, 100.0],
            ))
        })
        .collect()
}

/// 创建垂直线组
fn create_perpendicular_lines(count: usize) -> Vec<Primitive> {
    let mut lines = Vec::new();
    // 垂直线
    for i in 0..count / 2 {
        lines.push(Primitive::Line(Line::from_coords(
            [i as f64 * 10.0, 0.0],
            [i as f64 * 10.0, 100.0],
        )));
    }
    // 水平线
    for i in 0..count / 2 {
        lines.push(Primitive::Line(Line::from_coords(
            [0.0, i as f64 * 10.0],
            [100.0, i as f64 * 10.0],
        )));
    }
    lines
}

fn bench_measure_length(c: &mut Criterion) {
    let measurer = GeometryMeasurer;
    let start = [0.0, 0.0];
    let end = [3.0, 4.0];

    c.bench_function("measure_length", |b| {
        b.iter(|| measurer.measure_length(black_box(start), black_box(end)))
    });
}

fn bench_measure_area(c: &mut Criterion) {
    let measurer = GeometryMeasurer;
    let vertices = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

    c.bench_function("measure_area", |b| {
        b.iter(|| measurer.measure_area(black_box(vertices.clone())))
    });
}

fn bench_detect_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect_parallel");

    for size in [10, 50, 100, 200].iter() {
        let lines = create_parallel_lines(*size);
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(BenchmarkId::from_parameter(size), &lines, |b, lines| {
            b.iter(|| reasoner.find_all_relations(black_box(lines)))
        });
    }
    group.finish();
}

fn bench_detect_perpendicular(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect_perpendicular");

    for size in [10, 50, 100, 200].iter() {
        let lines = create_perpendicular_lines(*size);
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(BenchmarkId::from_parameter(size), &lines, |b, lines| {
            b.iter(|| reasoner.find_all_relations(black_box(lines)))
        });
    }
    group.finish();
}

fn bench_detect_connected(c: &mut Criterion) {
    let mut group = c.benchmark_group("detect_connected");

    for size in [10, 50, 100, 200].iter() {
        let primitives = create_multiple_rectangles(*size / 4); // 每个矩形 4 条线
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &primitives,
            |b, primitives| b.iter(|| reasoner.find_all_relations(black_box(primitives))),
        );
    }
    group.finish();
}

fn bench_rtree_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("rtree_vs_naive");

    // 测试 R-tree 优化的效果（50+ 基元时启用）
    for size in [40, 50, 60, 100].iter() {
        let primitives = create_multiple_rectangles(*size / 4);

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &primitives,
            |b, primitives| {
                let reasoner = GeometricRelationReasoner::with_defaults();
                b.iter(|| reasoner.find_all_relations(black_box(primitives)))
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_measure_length,
    bench_measure_area,
    bench_detect_parallel,
    bench_detect_perpendicular,
    bench_detect_connected,
    bench_rtree_optimization,
);

criterion_main!(benches);
