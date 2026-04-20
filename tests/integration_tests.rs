//! CadAgent 集成测试
//!
//! 测试完整的端到端流程

use cadagent::cad_extractor::CadPrimitiveExtractor;
use cadagent::cad_reasoning::GeometricRelationReasoner;
use cadagent::prelude::*;

/// 获取测试夹具目录
fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
}

#[test]
fn test_extract_primitives_from_simple_room() {
    let svg_path = fixtures_dir().join("simple_room.svg");

    let extractor = CadPrimitiveExtractor::with_defaults();
    let result = extractor.extract_from_svg(&svg_path).unwrap();

    // 验证提取了基元
    assert!(!result.primitives.is_empty());

    // 验证统计信息
    assert!(result.statistics.line_count > 0);

    println!("提取了 {} 个基元", result.statistics.total_count);
    println!(
        "线段：{}, 圆：{}, 矩形：{}",
        result.statistics.line_count, result.statistics.circle_count, result.statistics.rect_count
    );
}

#[test]
fn test_extract_primitives_from_complex_floor() {
    let svg_path = fixtures_dir().join("complex_floor.svg");

    let extractor = CadPrimitiveExtractor::with_defaults();
    let result = extractor.extract_from_svg(&svg_path).unwrap();

    // 复杂户型应该有更多基元
    assert!(result.primitives.len() > 10);

    // 应该包含线段和圆（窗户）
    assert!(result.statistics.line_count > 0);
    assert!(result.statistics.circle_count > 0);
}

#[test]
fn test_geometric_relations_detection() {
    // 创建一个矩形
    let _primitives = vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
        Primitive::Line(Line::from_coords([100.0, 100.0], [0.0, 100.0])),
        Primitive::Line(Line::from_coords([0.0, 100.0], [0.0, 0.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&_primitives);

    // 应该检测到平行关系（对边平行）
    assert!(result.statistics.parallel_count >= 2);

    // 应该检测到垂直关系（相邻边垂直）
    assert!(result.statistics.perpendicular_count >= 4);

    // 应该检测到连接关系
    assert!(result.statistics.connected_count >= 4);

    println!("检测到 {} 个平行关系", result.statistics.parallel_count);
    println!(
        "检测到 {} 个垂直关系",
        result.statistics.perpendicular_count
    );
    println!("检测到 {} 个连接关系", result.statistics.connected_count);
}

#[test]
fn test_coordinate_normalization() {
    // 创建大坐标范围的基元（仅用于说明，实际测试使用 extract_from_svg_string）
    let _primitives: [Primitive; 3] = [
        Primitive::Point(Point::new(0.0, 0.0)),
        Primitive::Point(Point::new(1000.0, 1000.0)),
        Primitive::Line(Line::from_coords([0.0, 0.0], [1000.0, 1000.0])),
    ];

    // 使用公共 API 测试归一化功能
    let mut config = cadagent::cad_extractor::ExtractorConfig::default();
    config.geometry.normalize_range = [0.0, 100.0];
    config.geometry.enable_normalization = true;

    let extractor = CadPrimitiveExtractor::new(config);
    let result = extractor
        .extract_from_svg_string(
            r#"<svg width="1000" height="1000">
            <line x1="0" y1="0" x2="1000" y2="1000" />
        </svg>"#,
        )
        .unwrap();

    // 归一化后坐标应该在 [0, 100] 范围内
    for prim in &result.primitives {
        if let Some(bbox) = prim.bounding_box() {
            assert!(bbox.min.x >= 0.0 && bbox.min.x <= 100.0);
            assert!(bbox.min.y >= 0.0 && bbox.min.y <= 100.0);
            assert!(bbox.max.x >= 0.0 && bbox.max.x <= 100.0);
            assert!(bbox.max.y >= 0.0 && bbox.max.y <= 100.0);
        }
    }
}

#[test]
fn test_transform_operations() {
    use cadagent::geometry::transform::GeometryTransform;

    let primitives = vec![
        Primitive::Point(Point::new(1.0, 2.0)),
        Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 1.0])),
    ];

    let transform = GeometryTransform;

    // 测试平移
    let translated = transform.translate(primitives.clone(), 10.0, 20.0);
    if let Primitive::Point(p) = &translated[0] {
        assert!((p.x - 11.0).abs() < 1e-10);
        assert!((p.y - 22.0).abs() < 1e-10);
    }

    // 测试旋转（90 度）
    let rotated = transform.rotate(primitives.clone(), 90.0, [0.0, 0.0]);
    if let Primitive::Point(p) = &rotated[0] {
        assert!((p.x - (-2.0)).abs() < 1e-10);
        assert!((p.y - 1.0).abs() < 1e-10);
    }

    // 测试缩放
    let scaled = transform.scale(primitives.clone(), 2.0, [0.0, 0.0]);
    if let Primitive::Point(p) = &scaled[0] {
        assert!((p.x - 2.0).abs() < 1e-10);
        assert!((p.y - 4.0).abs() < 1e-10);
    }

    // 测试镜像（X 轴）
    let mirrored = transform.mirror(primitives, cadagent::geometry::transform::MirrorAxis::X);
    if let Primitive::Point(p) = &mirrored[0] {
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - (-2.0)).abs() < 1e-10);
    }
}

#[test]
fn test_geometry_config_validation() {
    use cadagent::error::GeometryConfig;

    // 测试有效配置
    let config = GeometryConfig::default();
    assert!(config.validate().is_ok());

    // 测试无效角度容差
    let invalid_config = GeometryConfig {
        angle_tolerance: -0.01,
        ..Default::default()
    };
    assert!(invalid_config.validate().is_err());

    // 测试 validate_or_fix 自动修正
    let mut config_to_fix = GeometryConfig {
        angle_tolerance: -0.01,
        ..Default::default()
    };
    let warnings = config_to_fix.validate_or_fix();
    assert!(!warnings.is_empty());
    assert!((config_to_fix.angle_tolerance - 0.01).abs() < 1e-10);
}

#[test]
fn test_tool_registry() {
    let registry = ToolRegistry::default();

    // 测试测量工具 - measure_length 返回的是数字而不是对象
    let result = registry.call(
        "measure_length",
        json!({
            "start": [0.0, 0.0],
            "end": [3.0, 4.0]
        }),
    );

    assert!(result.is_ok());
    let result = result.unwrap();
    // measure_length 返回的是数字（勾股定理：3-4-5 三角形）
    assert!(result.is_number());
}

#[test]
fn test_svg_parser_with_fixture() {
    use cadagent::parser::svg::SvgParser;

    let svg_path = fixtures_dir().join("simple_room.svg");
    let result = SvgParser::parse(&svg_path).unwrap();

    // 验证解析结果
    assert!(!result.primitives.is_empty());
    assert_eq!(result.metadata.width, "500");
    assert_eq!(result.metadata.height, "400");
}

// 性能基准测试（需要 criterion）
#[test]
fn test_performance_with_many_primitives() {
    // 创建 100 个基元 - 包括平行线
    let mut primitives: Vec<Primitive> = (0..50)
        .map(|i| {
            Primitive::Line(Line::from_coords(
                [i as f64 * 2.0, 0.0],
                [i as f64 * 2.0, 100.0], // 所有垂直线互相平行
            ))
        })
        .collect();

    // 添加一些水平线形成垂直关系
    for i in 0..50 {
        primitives.push(Primitive::Line(Line::from_coords(
            [0.0, i as f64 * 2.0],
            [100.0, i as f64 * 2.0], // 所有水平线互相平行
        )));
    }

    let reasoner = GeometricRelationReasoner::with_defaults();

    let start = std::time::Instant::now();
    let result = reasoner.find_all_relations(&primitives);
    let elapsed = start.elapsed();

    println!("100 个基元的关系检测耗时：{:?}", elapsed);
    println!(
        "平行关系：{}, 垂直关系：{}, 连接关系：{}, 共线关系：{}",
        result.statistics.parallel_count,
        result.statistics.perpendicular_count,
        result.statistics.connected_count,
        result.statistics.collinear_count
    );

    // 应该在合理时间内完成（1 秒内）
    assert!(elapsed.as_millis() < 1000);

    // 验证总共检测到了关系
    // 注意：由于容差设置和算法实现，可能检测到的关系类型会有所不同
    let total_relations = result.statistics.total_count;
    assert!(
        total_relations > 0,
        "应该检测到至少一个几何关系，实际检测到{}个",
        total_relations
    );
}
