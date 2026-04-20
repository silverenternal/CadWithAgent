//! 约束校验器性能基准测试
//!
//! 测试冲突检测算法优化效果

use cadagent::cad_reasoning::GeometricRelation;
use cadagent::cad_verifier::ConstraintVerifier;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// 创建测试关系（包含冲突）
fn create_relations_with_conflicts(count: usize) -> Vec<GeometricRelation> {
    let mut relations = Vec::with_capacity(count);

    for i in 0..count / 2 {
        let line1 = i * 2;
        let line2 = i * 2 + 1;

        // 添加平行关系
        relations.push(GeometricRelation::Parallel {
            line1_id: line1,
            line2_id: line2,
            angle_diff: 0.001,
            confidence: 0.99,
        });

        // 添加垂直关系（制造冲突）
        relations.push(GeometricRelation::Perpendicular {
            line1_id: line1,
            line2_id: line2,
            angle_diff: 0.002,
            confidence: 0.98,
        });
    }

    relations
}

/// 创建测试关系（无冲突）
fn create_relations_no_conflicts(count: usize) -> Vec<GeometricRelation> {
    let mut relations = Vec::with_capacity(count);

    for i in 0..count {
        let line1 = i;
        let line2 = (i + 1) % count;

        // 只添加平行关系，无冲突
        relations.push(GeometricRelation::Parallel {
            line1_id: line1,
            line2_id: line2,
            angle_diff: 0.001,
            confidence: 0.99,
        });
    }

    relations
}

/// 创建包含同心/相切冲突的关系
fn create_circle_conflicts(count: usize) -> Vec<GeometricRelation> {
    let mut relations = Vec::with_capacity(count);

    for i in 0..count / 2 {
        let circle1 = i * 2;
        let circle2 = i * 2 + 1;

        // 添加同心关系
        relations.push(GeometricRelation::Concentric {
            circle1_id: circle1,
            circle2_id: circle2,
            center_distance: 0.0,
            confidence: 0.99,
        });

        // 添加相切关系（制造冲突）
        relations.push(GeometricRelation::TangentCircleCircle {
            circle1_id: circle1,
            circle2_id: circle2,
            distance: 0.0,
            confidence: 0.98,
        });
    }

    relations
}

fn bench_detect_conflicts_small(c: &mut Criterion) {
    let verifier = ConstraintVerifier::with_defaults();
    let relations = create_relations_with_conflicts(100);

    c.bench_function("detect_conflicts/100_relations_50_conflicts", |b| {
        b.iter(|| verifier.detect_conflicts_test(black_box(&relations)))
    });
}

fn bench_detect_conflicts_medium(c: &mut Criterion) {
    let verifier = ConstraintVerifier::with_defaults();
    let relations = create_relations_with_conflicts(500);

    c.bench_function("detect_conflicts/500_relations_250_conflicts", |b| {
        b.iter(|| verifier.detect_conflicts_test(black_box(&relations)))
    });
}

fn bench_detect_conflicts_large(c: &mut Criterion) {
    let verifier = ConstraintVerifier::with_defaults();
    let relations = create_relations_with_conflicts(1000);

    c.bench_function("detect_conflicts/1000_relations_500_conflicts", |b| {
        b.iter(|| verifier.detect_conflicts_test(black_box(&relations)))
    });
}

fn bench_detect_conflicts_no_conflicts(c: &mut Criterion) {
    let verifier = ConstraintVerifier::with_defaults();
    let relations = create_relations_no_conflicts(1000);

    c.bench_function("detect_conflicts/1000_relations_no_conflicts", |b| {
        b.iter(|| verifier.detect_conflicts_test(black_box(&relations)))
    });
}

fn bench_detect_circle_conflicts(c: &mut Criterion) {
    let verifier = ConstraintVerifier::with_defaults();
    let relations = create_circle_conflicts(500);

    c.bench_function("detect_conflicts/500_circle_conflicts", |b| {
        b.iter(|| verifier.detect_conflicts_test(black_box(&relations)))
    });
}

fn bench_detect_conflicts_scaling(c: &mut Criterion) {
    let verifier = ConstraintVerifier::with_defaults();
    let mut group = c.benchmark_group("conflict_detection_scaling");

    for &size in &[100, 200, 500, 1000, 2000] {
        let relations = create_relations_with_conflicts(size);
        group.bench_function(format!("{size}_relations"), |b| {
            b.iter(|| verifier.detect_conflicts_test(black_box(&relations)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_detect_conflicts_small,
    bench_detect_conflicts_medium,
    bench_detect_conflicts_large,
    bench_detect_conflicts_no_conflicts,
    bench_detect_circle_conflicts,
    bench_detect_conflicts_scaling,
);

criterion_main!(benches);
