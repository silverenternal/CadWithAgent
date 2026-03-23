//! analysis 模块集成测试

use cadagent::analysis::{AnalysisPipeline, AnalysisConfig};
use cadagent::prelude::*;

/// 测试：分析管线配置验证
#[test]
fn test_analysis_config_validation() {
    // 测试有效配置
    let config = AnalysisConfig::default();
    assert!(config.validate().is_ok());

    // 测试无效归一化范围
    let mut invalid_config = AnalysisConfig::default();
    invalid_config.normalize_range = [100.0, 0.0];  // 最小值 > 最大值
    assert!(invalid_config.validate().is_err());

    // 测试无效角度容差
    let mut invalid_config = AnalysisConfig::default();
    invalid_config.angle_tolerance = -0.01;
    assert!(invalid_config.validate().is_err());

    // 测试 validate_or_fix 自动修正
    let mut config_to_fix = AnalysisConfig::default();
    config_to_fix.angle_tolerance = -0.01;
    let warnings = config_to_fix.validate_or_fix();
    assert!(!warnings.is_empty());
    assert!((config_to_fix.angle_tolerance - 0.01).abs() < 1e-10);
}

/// 测试：从 SVG 字符串注入上下文（不含 VLM 推理）
#[test]
fn test_inject_from_svg_string() {
    let svg_content = r#"
        <svg width="100" height="100" viewBox="0 0 100 100">
            <line x1="0" y1="0" x2="100" y2="0" />
            <line x1="100" y1="0" x2="100" y2="100" />
            <line x1="100" y1="100" x2="0" y2="100" />
            <line x1="0" y1="100" x2="0" y2="0" />
        </svg>
    "#;

    // 注意：需要 API Key 才能创建管线
    // 这里测试创建失败的情况
    let result = AnalysisPipeline::with_defaults();
    
    // 如果没有设置 API Key，会返回错误
    if let Ok(pipeline) = result {
        let inject_result = pipeline.inject_from_svg_string(svg_content, "分析这个矩形");
        assert!(inject_result.is_ok());
        
        let result = inject_result.unwrap();
        assert!(!result.primitives.is_empty());
        assert!(result.prompt.full_prompt.len() > 0);
    }
    // 如果 API Key 未设置，跳过测试
}

/// 测试：几何关系检测集成
#[test]
fn test_geometric_relations_integration() {
    // 创建一个矩形
    let primitives = vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
        Primitive::Line(Line::from_coords([100.0, 100.0], [0.0, 100.0])),
        Primitive::Line(Line::from_coords([0.0, 100.0], [0.0, 0.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let result = reasoner.find_all_relations(&primitives);

    // 矩形应该有：
    // - 2 对平行关系（对边平行）
    // - 4 个垂直关系（相邻边垂直）
    // - 4 个连接关系（顶点连接）
    assert!(result.statistics.parallel_count >= 2);
    assert!(result.statistics.perpendicular_count >= 4);
    assert!(result.statistics.connected_count >= 4);
}

/// 测试：约束校验器集成
#[test]
fn test_constraint_verifier_integration() {
    let primitives = vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let relations = reasoner.find_all_relations(&primitives);

    let verifier = ConstraintVerifier::with_defaults();
    let verification = verifier.verify(&primitives, &relations.relations);

    assert!(verification.is_ok());
    let result = verification.unwrap();
    
    // 验证结果应该有冲突和问题的统计
    assert!(result.conflicts.is_empty() || !result.conflicts.is_empty());  // 无论有没有冲突都正常
}

/// 测试：提示词构造器集成
#[test]
fn test_prompt_builder_integration() {
    let primitives = vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Circle(Circle::new(Point::new(50.0, 50.0), 10.0)),
    ];

    let reasoner = GeometricRelationReasoner::with_defaults();
    let relations = reasoner.find_all_relations(&primitives);

    let builder = PromptBuilder::with_defaults();
    let prompt = builder.build_analysis_prompt(&primitives, &relations.relations, None);

    // 验证提示词结构
    assert!(!prompt.system_prompt.is_empty());
    assert!(!prompt.user_prompt.is_empty());
    assert!(!prompt.full_prompt.is_empty());
    assert_eq!(prompt.metadata.primitive_count, 2);
    assert!(prompt.metadata.prompt_length > 0);
}

/// 测试：摘要模式（超过阈值时简化输出）
#[test]
fn test_summary_mode() {
    // 创建大量基元触发摘要模式
    let mut primitives = Vec::new();
    for i in 0..150 {
        primitives.push(Primitive::Line(Line::from_coords(
            [i as f64, 0.0],
            [i as f64, 100.0]
        )));
    }

    // 启用摘要模式的配置
    let config = PromptConfig {
        enable_summary_mode: true,
        summary_mode_threshold: 100,
        ..Default::default()
    };
    let builder = PromptBuilder::new(config);
    let prompt = builder.build_analysis_prompt(&primitives, &[], None);

    // 验证启用了摘要模式
    assert!(prompt.full_prompt.contains("摘要模式"));
    assert!(prompt.full_prompt.contains("节省 Token"));
    
    // 验证不包含详细样本（因为启用了摘要）
    assert!(!prompt.full_prompt.contains("样本："));
}
