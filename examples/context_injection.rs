//! 工具增强上下文注入管线示例
//!
//! 演示完整的"工具增强上下文注入"范式：
//! 1. 从 SVG 图纸提取几何基元
//! 2. 推理几何约束关系
//! 3. 校验约束合法性
//! 4. 生成结构化提示词注入 VLM
//!
//! # 运行示例
//!
//! ```bash
//! cargo run --example context_injection
//! ```

use cadagent::prelude::*;
use cadagent::analysis::{AnalysisPipeline, AnalysisTools};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 工具增强上下文注入管线示例 ===\n");

    // 示例 1: 从 SVG 字符串注入上下文
    example_svg_string()?;

    // 示例 2: 从已有基元注入上下文
    example_from_primitives()?;

    // 示例 3: 使用 tokitai 工具
    example_tool_usage()?;

    Ok(())
}

/// 示例 1: 从 SVG 字符串注入上下文
fn example_svg_string() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例 1: 从 SVG 字符串注入上下文 ---\n");

    // 一个简单的矩形 SVG
    let svg = r#"<svg width="200" height="200" viewBox="0 0 200 200">
        <!-- 外墙 -->
        <line x1="20" y1="20" x2="180" y2="20" />
        <line x1="180" y1="20" x2="180" y2="180" />
        <line x1="180" y1="180" x2="20" y2="180" />
        <line x1="20" y1="180" x2="20" y2="20" />

        <!-- 内墙 -->
        <line x1="100" y1="20" x2="100" y2="180" />
        <line x1="20" y1="100" x2="100" y2="100" />

        <!-- 圆（可能表示柱子） -->
        <circle cx="50" cy="50" r="5" />
        <circle cx="150" cy="50" r="5" />
        <circle cx="50" cy="150" r="5" />
        <circle cx="150" cy="150" r="5" />
    </svg>"#;

    // 创建管线
    let pipeline = AnalysisPipeline::with_defaults().expect("创建管线失败，请设置 PROVIDER_ZAZAZ_API_KEY 环境变量");

    // 执行上下文注入
    let task = "请分析这个户型图，识别所有房间并计算面积";
    let result = pipeline.inject_from_svg_string(svg, task)?;

    // 输出结果
    println!("📊 基元统计:");
    println!("   - 提取了 {} 个基元", result.primitives.len());

    let mut line_count = 0;
    let mut circle_count = 0;
    for prim in &result.primitives {
        match prim {
            Primitive::Line(_) => line_count += 1,
            Primitive::Circle(_) => circle_count += 1,
            _ => {}
        }
    }
    println!("   - 线段：{} 条", line_count);
    println!("   - 圆：{} 个", circle_count);

    println!("\n🔗 几何关系:");
    println!("   - 发现 {} 个约束关系", result.relations.len());

    let mut parallel_count = 0;
    let mut perpendicular_count = 0;
    let mut connected_count = 0;
    for rel in &result.relations {
        match rel {
            GeometricRelation::Parallel { .. } => parallel_count += 1,
            GeometricRelation::Perpendicular { .. } => perpendicular_count += 1,
            GeometricRelation::Connected { .. } => connected_count += 1,
            _ => {}
        }
    }
    println!("   - 平行：{} 对", parallel_count);
    println!("   - 垂直：{} 对", perpendicular_count);
    println!("   - 连接：{} 对", connected_count);

    println!("\n✅ 约束校验:");
    if let Some(ref verification) = result.verification {
        println!("   - 合法性：{}", if verification.is_valid { "✓ 通过" } else { "✗ 未通过" });
        println!("   - 总体评分：{:.1}/1.0", verification.overall_score * 10.0);
        if !verification.conflicts.is_empty() {
            println!("   - ⚠ 发现 {} 个冲突", verification.conflicts.len());
        }
    } else {
        println!("   - 已跳过");
    }

    println!("\n📝 生成的提示词:");
    println!("   - 提示词长度：{} 字符", result.prompt.full_prompt.len());
    println!("   - 注入的上下文：{:?}", result.prompt.metadata.injected_context);

    // 显示提示词前 500 字符
    let prompt_preview = if result.prompt.full_prompt.len() > 500 {
        &result.prompt.full_prompt[..500]
    } else {
        &result.prompt.full_prompt
    };
    println!("\n   预览:\n   {}\n   ...", prompt_preview.replace('\n', "\n   "));

    println!("\n⏱️  总耗时：{}ms", result.total_latency_ms);
    println!("\n{}\n", "=".repeat(50));

    Ok(())
}

/// 示例 2: 从已有基元注入上下文
fn example_from_primitives() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例 2: 从已有基元注入上下文 ---\n");

    // 创建一个简单的矩形基元列表
    let primitives = vec![
        // 矩形边框
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
        Primitive::Line(Line::from_coords([100.0, 100.0], [0.0, 100.0])),
        Primitive::Line(Line::from_coords([0.0, 100.0], [0.0, 0.0])),

        // 对角线（表示斜撑）
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 100.0])),

        // 中心圆
        Primitive::Circle(Circle::from_coords([50.0, 50.0], 10.0)),
    ];

    // 创建管线
    let pipeline = AnalysisPipeline::with_defaults().expect("创建管线失败，请设置 PROVIDER_ZAZAZ_API_KEY 环境变量");

    // 执行上下文注入
    let task = "分析这个几何图形的结构特征";
    let result = pipeline.inject_from_primitives(&primitives, task)?;

    println!("📊 输入基元：{} 个", result.primitives.len());
    println!("🔗 推理关系：{} 个", result.relations.len());

    // 显示检测到的关系
    for rel in &result.relations {
        match rel {
            GeometricRelation::Parallel { line1_id, line2_id, .. } => {
                println!("   - line_{} ∥ line_{}", line1_id, line2_id);
            }
            GeometricRelation::Perpendicular { line1_id, line2_id, .. } => {
                println!("   - line_{} ⊥ line_{}", line1_id, line2_id);
            }
            GeometricRelation::Connected { primitive1_id, primitive2_id, .. } => {
                println!("   - primitive_{} 连接到 primitive_{}", primitive1_id, primitive2_id);
            }
            _ => {}
        }
    }

    println!("\n📝 提示词已生成，可送入 VLM 模型进行推理");
    println!("{}\n", "=".repeat(50));

    Ok(())
}

/// 示例 3: 使用 tokitai 工具
fn example_tool_usage() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例 3: 使用 tokitai 工具 ---\n");

    let svg = r#"<svg width="100" height="100">
        <line x1="0" y1="0" x2="100" y2="0" />
        <line x1="0" y1="0" x2="0" y2="100" />
    </svg>"#;

    let tools = AnalysisTools::default();

    // 调用分析工具（使用存在的 tool 方法）
    let result = tools.analyze_layout(
        svg.to_string(),
        "分析这个图形".to_string(),
        None,
    );

    println!("🔧 工具调用结果:");
    println!("   - 成功：{}", result["success"].as_bool().unwrap_or(false));
    println!("   - 基元数量：{}", result["primitive_count"]);

    // 获取管线信息
    let info = tools.get_analysis_info();
    println!("\n📋 分析管线信息:");
    println!("   - 名称：{}", info["name"]);
    println!("   - 描述：{}", info["description"]);

    if let Some(steps) = info["steps"].as_array() {
        println!("\n   处理步骤:");
        for step in steps {
            println!("   {}. {} - {}",
                step["id"],
                step["name"],
                step["description"]
            );
        }
    }

    println!("\n{}\n", "=".repeat(50));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pipeline() {
        let svg = r#"<svg width="100" height="100">
            <line x1="0" y1="0" x2="100" y2="0" />
            <line x1="0" y1="0" x2="0" y2="100" />
            <line x1="100" y1="0" x2="100" y2="100" />
            <line x1="0" y1="100" x2="100" y2="100" />
        </svg>"#;

        let pipeline = AnalysisPipeline::with_defaults().expect("创建管线失败，请设置 PROVIDER_ZAZAZ_API_KEY 环境变量");
        let result = pipeline.inject_from_svg_string(svg, "分析这个图形").unwrap();

        assert!(!result.primitives.is_empty());
        assert!(result.relations.len() > 0);
        assert!(result.verification.is_some());
        assert!(!result.prompt.full_prompt.is_empty());
    }

    #[test]
    fn test_pipeline_without_verification() {
        let mut config = AnalysisConfig::default();
        config.skip_verification = true;

        let pipeline = AnalysisPipeline::new(config).expect("创建管线失败，请设置 PROVIDER_ZAZAZ_API_KEY 环境变量");

        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
        ];

        let result = pipeline.inject_from_primitives(&primitives, "测试").unwrap();

        assert!(result.verification.is_none());
    }
}
