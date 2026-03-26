//! CAD 几何关系推理测试
//!
//! 测试 GeometricRelationReasoner 的各种几何关系检测功能

use cadagent::cad_reasoning::{GeometricRelation, GeometricRelationReasoner, ReasoningConfig};
use cadagent::prelude::*;

// ==================== 平行关系检测测试 ====================

#[test]
fn test_detect_parallel_lines() {
    let primitives = vec![
        // 两条平行的垂直线
        Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 100.0])),
        Primitive::Line(Line::from_coords([10.0, 0.0], [10.0, 100.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert!(result.statistics.parallel_count >= 1);
}

#[test]
fn test_detect_parallel_horizontal_lines() {
    let primitives = vec![
        // 两条平行的水平线
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([0.0, 10.0], [100.0, 10.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert!(result.statistics.parallel_count >= 1);
}

#[test]
fn test_detect_non_parallel_lines() {
    let primitives = vec![
        // 两条不平行的线
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 100.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert_eq!(result.statistics.parallel_count, 0);
}

// ==================== 垂直关系检测测试 ====================

#[test]
fn test_detect_perpendicular_lines() {
    let primitives = vec![
        // 两条垂直的线（形成 L 形）
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert!(result.statistics.perpendicular_count >= 1);
}

#[test]
fn test_detect_rectangle_perpendicular() {
    let primitives = vec![
        // 矩形的四条边
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 50.0])),
        Primitive::Line(Line::from_coords([100.0, 50.0], [0.0, 50.0])),
        Primitive::Line(Line::from_coords([0.0, 50.0], [0.0, 0.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    // 矩形应该有 4 个垂直关系（相邻边）
    assert!(result.statistics.perpendicular_count >= 4);
}

// ==================== 连接关系检测测试 ====================

#[test]
fn test_detect_connected_lines() {
    let primitives = vec![
        // 两条连接的线（共享端点）
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert!(result.statistics.connected_count >= 1);
}

#[test]
fn test_detect_disconnected_lines() {
    let primitives = vec![
        // 两条不连接的线
        Primitive::Line(Line::from_coords([0.0, 0.0], [10.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert_eq!(result.statistics.connected_count, 0);
}

// ==================== 同心圆检测测试 ====================

#[test]
fn test_detect_concentric_circles() {
    let primitives = vec![
        // 两个同心圆
        Primitive::Circle(Circle::from_coords([50.0, 50.0], 10.0)),
        Primitive::Circle(Circle::from_coords([50.0, 50.0], 20.0)),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert!(result.statistics.concentric_count >= 1);
}

#[test]
fn test_detect_non_concentric_circles() {
    let primitives = vec![
        // 两个不同心的圆
        Primitive::Circle(Circle::from_coords([0.0, 0.0], 10.0)),
        Primitive::Circle(Circle::from_coords([100.0, 100.0], 20.0)),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert_eq!(result.statistics.concentric_count, 0);
}

// ==================== 等距关系检测测试 ====================

#[test]
fn test_detect_equal_length_lines() {
    let primitives = vec![
        // 两条等长的线
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([0.0, 10.0], [100.0, 10.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert!(result.statistics.equal_distance_count >= 1);
}

#[test]
fn test_detect_different_length_lines() {
    let primitives = vec![
        // 两条不等长的线
        Primitive::Line(Line::from_coords([0.0, 0.0], [50.0, 0.0])),
        Primitive::Line(Line::from_coords([0.0, 10.0], [100.0, 10.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    assert_eq!(result.statistics.equal_distance_count, 0);
}

// ==================== 包含关系检测测试 ====================

#[test]
fn test_detect_point_on_line() {
    let primitives = vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Point(Point::new(50.0, 0.0)),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    // 应该检测到点在线上的关系
    assert!(result.statistics.contains_count >= 1);
}

#[test]
fn test_detect_point_in_polygon() {
    let primitives = vec![
        Primitive::Polygon(Polygon::from_coords(vec![
            [0.0, 0.0],
            [100.0, 0.0],
            [100.0, 100.0],
            [0.0, 100.0],
        ])),
        Primitive::Point(Point::new(50.0, 50.0)),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    // 应该检测到点在多边形内的关系
    assert!(result.statistics.contains_count >= 1);
}

// ==================== 矩形综合测试 ====================

#[test]
fn test_rectangle_relations() {
    let primitives = vec![
        // 矩形的四条边
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 50.0])),
        Primitive::Line(Line::from_coords([100.0, 50.0], [0.0, 50.0])),
        Primitive::Line(Line::from_coords([0.0, 50.0], [0.0, 0.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    // 矩形应该有：
    // - 2 对平行边
    // - 4 个垂直关系（相邻边）
    // - 4 个连接关系
    // - 2 对等长边
    assert!(result.statistics.parallel_count >= 2);
    assert!(result.statistics.perpendicular_count >= 4);
    assert!(result.statistics.connected_count >= 4);
    assert!(result.statistics.equal_distance_count >= 2);
}

// ==================== 配置验证测试 ====================

#[test]
fn test_reasoning_config_validation() {
    // 测试有效配置
    let config = ReasoningConfig::default();
    assert!(config.validate().is_ok());

    // 测试无效角度容差
    let invalid_config = ReasoningConfig {
        angle_tolerance: -0.01,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // 测试 validate_or_fix 自动修正
    let mut config_to_fix = ReasoningConfig {
        angle_tolerance: -0.01,
        ..Default::default()
    };
    let warnings = config_to_fix.validate_or_fix();
    assert!(!warnings.is_empty());
    assert!((config_to_fix.angle_tolerance - 0.01).abs() < 1e-10);
}

// ==================== 性能测试（小数据量） ====================

#[test]
fn test_performance_with_10_primitives() {
    // 创建 10 条线
    let primitives: Vec<Primitive> = (0..10)
        .map(|i| {
            Primitive::Line(Line::from_coords(
                [i as f64 * 10.0, 0.0],
                [i as f64 * 10.0, 100.0],
            ))
        })
        .collect();

    let reasoner = GeometricRelationReasoner::with_defaults();

    let start = std::time::Instant::now();
    let result = reasoner.find_all_relations(&primitives);
    let elapsed = start.elapsed();

    println!("10 个基元的关系检测耗时：{:?}", elapsed);
    println!(
        "平行关系：{}, 垂直关系：{}, 连接关系：{}",
        result.statistics.parallel_count,
        result.statistics.perpendicular_count,
        result.statistics.connected_count
    );

    // 应该在很短时间内完成（100ms 内）
    assert!(elapsed.as_millis() < 100);
}

// ==================== 几何关系类型测试 ====================

#[test]
fn test_geometric_relation_serialization() {
    let relation = GeometricRelation::Parallel {
        line1_id: 0,
        line2_id: 1,
        angle_diff: 0.001,
        confidence: 0.95,
    };

    let json = serde_json::to_string(&relation).unwrap();
    assert!(json.contains("parallel"));

    let deserialized: GeometricRelation = serde_json::from_str(&json).unwrap();
    match deserialized {
        GeometricRelation::Parallel {
            line1_id, line2_id, ..
        } => {
            assert_eq!(line1_id, 0);
            assert_eq!(line2_id, 1);
        }
        _ => panic!("Deserialized to wrong relation type"),
    }
}
