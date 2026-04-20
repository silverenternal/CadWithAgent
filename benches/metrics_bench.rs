//! 评估指标性能基准测试
//!
//! 测试 MetricEvaluator 在房间检测、尺寸提取、冲突检测等任务上的性能
//!
//! # 测试内容
//!
//! - 房间检测评估 (F1/IoU 计算)
//! - 尺寸提取评估 (带误差容忍)
//! - 冲突检测评估 (大规模)
//! - 综合评估 (多任务并行)

use cadagent::metrics::evaluator::{
    ConflictDetection, DimensionExtraction, MetricEvaluator, RoomDetection,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// 创建房间检测测试数据
fn create_room_detections(count: usize, room_types: &[&str]) -> Vec<RoomDetection> {
    let mut detections = Vec::with_capacity(count);

    for i in 0..count {
        let room_type = room_types[i % room_types.len()].to_string();
        let offset = (i as f64 * 100.0) % 1000.0;

        detections.push(RoomDetection {
            id: i,
            room_type,
            bbox: [offset, offset, offset + 100.0, offset + 100.0],
            area: 10000.0,
        });
    }

    detections
}

/// 创建尺寸提取测试数据
fn create_dimension_extractions(count: usize) -> Vec<DimensionExtraction> {
    let mut extractions = Vec::with_capacity(count);

    for i in 0..count {
        let dim_type = if i % 2 == 0 { "length" } else { "width" };
        extractions.push(DimensionExtraction {
            primitive_id: i,
            dimension_type: dim_type.to_string(),
            value: 100.0 + (i as f64 * 0.5),
            unit: "mm".to_string(),
        });
    }

    extractions
}

/// 创建冲突检测测试数据
fn create_conflict_detections(count: usize) -> Vec<ConflictDetection> {
    let mut detections = Vec::with_capacity(count);

    for i in 0..count {
        detections.push(ConflictDetection {
            conflict_id: i,
            primitive_ids: vec![i * 2, i * 2 + 1],
            conflict_type: "parallel_perpendicular".to_string(),
        });
    }

    detections
}

// ============ 房间检测基准测试 ============

fn bench_room_detection_small(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let ground_truth = create_room_detections(50, &["living_room", "bedroom", "kitchen"]);
    let predictions = create_room_detections(50, &["living_room", "bedroom", "kitchen"]);

    c.bench_function("room_detection/50_rooms", |b| {
        b.iter(|| {
            evaluator.evaluate_room_detection(black_box(&predictions), black_box(&ground_truth))
        })
    });
}

fn bench_room_detection_medium(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let ground_truth =
        create_room_detections(200, &["living_room", "bedroom", "kitchen", "bathroom"]);
    let predictions =
        create_room_detections(200, &["living_room", "bedroom", "kitchen", "bathroom"]);

    c.bench_function("room_detection/200_rooms", |b| {
        b.iter(|| {
            evaluator.evaluate_room_detection(black_box(&predictions), black_box(&ground_truth))
        })
    });
}

fn bench_room_detection_large(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let ground_truth = create_room_detections(
        1000,
        &["living_room", "bedroom", "kitchen", "bathroom", "garage"],
    );
    let predictions = create_room_detections(
        1000,
        &["living_room", "bedroom", "kitchen", "bathroom", "garage"],
    );

    c.bench_function("room_detection/1000_rooms", |b| {
        b.iter(|| {
            evaluator.evaluate_room_detection(black_box(&predictions), black_box(&ground_truth))
        })
    });
}

fn bench_room_detection_with_mismatch(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let ground_truth = create_room_detections(100, &["living_room", "bedroom", "kitchen"]);
    // 预测包含一些错误类型
    let predictions = create_room_detections(100, &["living_room", "bedroom", "office"]);

    c.bench_function("room_detection/100_rooms_with_mismatch", |b| {
        b.iter(|| {
            evaluator.evaluate_room_detection(black_box(&predictions), black_box(&ground_truth))
        })
    });
}

// ============ 尺寸提取基准测试 ============

fn bench_dimension_extraction_small(c: &mut Criterion) {
    let evaluator = MetricEvaluator::with_thresholds(0.5, 0.01);
    let ground_truth = create_dimension_extractions(100);
    let predictions = create_dimension_extractions(100);

    c.bench_function("dimension_extraction/100_dims", |b| {
        b.iter(|| {
            evaluator
                .evaluate_dimension_extraction(black_box(&predictions), black_box(&ground_truth))
        })
    });
}

fn bench_dimension_extraction_medium(c: &mut Criterion) {
    let evaluator = MetricEvaluator::with_thresholds(0.5, 0.01);
    let ground_truth = create_dimension_extractions(500);
    let predictions = create_dimension_extractions(500);

    c.bench_function("dimension_extraction/500_dims", |b| {
        b.iter(|| {
            evaluator
                .evaluate_dimension_extraction(black_box(&predictions), black_box(&ground_truth))
        })
    });
}

fn bench_dimension_extraction_large(c: &mut Criterion) {
    let evaluator = MetricEvaluator::with_thresholds(0.5, 0.01);
    let ground_truth = create_dimension_extractions(2000);
    let predictions = create_dimension_extractions(2000);

    c.bench_function("dimension_extraction/2000_dims", |b| {
        b.iter(|| {
            evaluator
                .evaluate_dimension_extraction(black_box(&predictions), black_box(&ground_truth))
        })
    });
}

// ============ 冲突检测基准测试 ============

fn bench_conflict_detection_small(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let ground_truth = create_conflict_detections(50);
    let predictions = create_conflict_detections(50);

    c.bench_function("conflict_detection/50_conflicts", |b| {
        b.iter(|| {
            evaluator.evaluate_conflict_detection(
                black_box(&predictions),
                black_box(&ground_truth),
                black_box(1000),
            )
        })
    });
}

fn bench_conflict_detection_medium(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let ground_truth = create_conflict_detections(200);
    let predictions = create_conflict_detections(200);

    c.bench_function("conflict_detection/200_conflicts", |b| {
        b.iter(|| {
            evaluator.evaluate_conflict_detection(
                black_box(&predictions),
                black_box(&ground_truth),
                black_box(5000),
            )
        })
    });
}

fn bench_conflict_detection_large(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let ground_truth = create_conflict_detections(1000);
    let predictions = create_conflict_detections(1000);

    c.bench_function("conflict_detection/1000_conflicts", |b| {
        b.iter(|| {
            evaluator.evaluate_conflict_detection(
                black_box(&predictions),
                black_box(&ground_truth),
                black_box(20000),
            )
        })
    });
}

// ============ 综合评估基准测试 ============

fn bench_comprehensive_evaluation(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();

    let room_gt = create_room_detections(100, &["living_room", "bedroom", "kitchen"]);
    let room_pred = create_room_detections(100, &["living_room", "bedroom", "kitchen"]);

    let dim_gt = create_dimension_extractions(200);
    let dim_pred = create_dimension_extractions(200);

    let conflict_gt = create_conflict_detections(50);
    let conflict_pred = create_conflict_detections(50);

    c.bench_function("comprehensive/100_rooms_200_dims_50_conflicts", |b| {
        b.iter(|| {
            evaluator.run_comprehensive_evaluation(
                black_box(&room_pred),
                black_box(&room_gt),
                black_box(&dim_pred),
                black_box(&dim_gt),
                black_box(&conflict_pred),
                black_box(&conflict_gt),
                black_box(10000),
            )
        })
    });
}

// ============ IoU 计算基准测试 ============
// 注意：compute_bbox_iou 和 compute_average_iou 是私有方法，不直接 benchmark
// 而是通过 evaluate_room_detection 公开 API 来测试 IoU 计算性能

// ============ 参数化基准测试 ============

fn bench_room_detection_scaling(c: &mut Criterion) {
    let evaluator = MetricEvaluator::new();
    let mut group = c.benchmark_group("room_detection_scaling");

    for &size in &[10, 50, 100, 200, 500] {
        let _ground_truth = create_room_detections(size, &["living_room", "bedroom", "kitchen"]);
        let _predictions = create_room_detections(size, &["living_room", "bedroom", "kitchen"]);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let gt = create_room_detections(size, &["living_room", "bedroom", "kitchen"]);
            let pred = create_room_detections(size, &["living_room", "bedroom", "kitchen"]);
            b.iter(|| evaluator.evaluate_room_detection(black_box(&pred), black_box(&gt)))
        });
    }

    group.finish();
}

fn bench_dimension_extraction_scaling(c: &mut Criterion) {
    let evaluator = MetricEvaluator::with_thresholds(0.5, 0.01);
    let mut group = c.benchmark_group("dimension_extraction_scaling");

    for &size in &[100, 500, 1000, 2000, 5000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let gt = create_dimension_extractions(size);
            let pred = create_dimension_extractions(size);
            b.iter(|| evaluator.evaluate_dimension_extraction(black_box(&pred), black_box(&gt)))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    // 房间检测
    bench_room_detection_small,
    bench_room_detection_medium,
    bench_room_detection_large,
    bench_room_detection_with_mismatch,
    bench_room_detection_scaling,
    // 尺寸提取
    bench_dimension_extraction_small,
    bench_dimension_extraction_medium,
    bench_dimension_extraction_large,
    bench_dimension_extraction_scaling,
    // 冲突检测
    bench_conflict_detection_small,
    bench_conflict_detection_medium,
    bench_conflict_detection_large,
    // 综合评估
    bench_comprehensive_evaluation,
);

criterion_main!(benches);
