//! 真实 VLM 推理示例
//!
//! 演示完整的"工具增强上下文注入"范式，包括真实的 VLM API 调用：
//! 1. 从 SVG 图纸提取几何基元
//! 2. 推理几何约束关系
//! 3. 校验约束合法性
//! 4. 生成结构化提示词
//! 5. 调用 VLM API 进行推理
//!
//! # 运行示例
//!
//! ```bash
//! cargo run --example vlm_inference
//! ```

use cadagent::analysis::AnalysisPipeline;
use cadagent::bridge::vlm_client::VlmConfig;
use cadagent::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 真实 VLM 推理示例 ===\n");
    println!("此示例将调用 ZazaZ API 进行真实的几何推理\n");

    // 示例 1: 简单矩形的 VLM 推理
    example_simple_rectangle_with_vlm()?;

    Ok(())
}

/// 示例：简单矩形的 VLM 推理
fn example_simple_rectangle_with_vlm() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例：简单矩形的 VLM 推理 ---\n");

    // 一个简单的矩形 SVG（表示房间）
    let svg = r#"<svg width="400" height="300" viewBox="0 0 400 300">
        <!-- 外墙 -->
        <line x1="50" y1="50" x2="350" y2="50" />
        <line x1="350" y1="50" x2="350" y2="250" />
        <line x1="350" y1="250" x2="50" y2="250" />
        <line x1="50" y1="250" x2="50" y2="50" />

        <!-- 内墙（分割成两个房间） -->
        <line x1="200" y1="50" x2="200" y2="250" />

        <!-- 门（用文本标注） -->
        <text x="195" y="150" font-size="12">DOOR</text>
    </svg>"#;

    // 创建管线（使用默认 ZazaZ 配置）
    let pipeline = AnalysisPipeline::with_defaults()
        .expect("创建管线失败，请设置 PROVIDER_ZAZAZ_API_KEY 环境变量");

    let task = "请分析这个户型图：1. 识别有几个房间 2. 描述房间布局 3. 指出门的位置";

    println!("📝 任务：{}\n", task);
    println!("⏳ 正在执行几何分析和 VLM 推理...\n");

    // 执行完整的上下文注入 + VLM 推理
    let result = pipeline.inject_from_svg_string_with_vlm(svg, task)?;

    // 输出几何分析结果
    println!("📊 几何分析结果:");
    println!("   - 提取了 {} 个基元", result.primitives.len());

    let mut line_count = 0;
    let mut text_count = 0;
    for prim in &result.primitives {
        match prim {
            Primitive::Line(_) => line_count += 1,
            Primitive::Text { .. } => text_count += 1,
            _ => {}
        }
    }
    println!("   - 线段：{} 条", line_count);
    println!("   - 文本标注：{} 个", text_count);

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
        println!(
            "   - 合法性：{}",
            if verification.is_valid {
                "✓ 通过"
            } else {
                "✗ 未通过"
            }
        );
        println!(
            "   - 总体评分：{:.1}/1.0",
            verification.overall_score * 10.0
        );
    }

    println!("\n⏱️  几何处理耗时：{}ms", result.total_latency_ms);

    // 输出 VLM 推理结果
    if let Some(ref vlm) = result.vlm_response {
        println!("\n🤖 VLM 推理结果:");
        println!("   - 模型：{}", vlm.model);
        println!("   - 推理耗时：{}ms", vlm.latency_ms);

        if let Some(ref usage) = vlm.usage {
            println!("   - Token 使用：");
            println!("     * Prompt: {} tokens", usage.prompt_tokens);
            println!("     * Completion: {} tokens", usage.completion_tokens);
            println!("     * 总计：{} tokens", usage.total_tokens);
        }

        println!("\n📝 VLM 回答:\n");
        println!(
            "   {}",
            vlm.content.split('\n').collect::<Vec<_>>().join("\n   ")
        );
    } else {
        println!("\n⚠️  未执行 VLM 推理");
    }

    println!("\n{}\n", "=".repeat(60));

    Ok(())
}

/// 示例：使用自定义 VLM 配置
fn _example_custom_vlm_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- 示例：使用自定义 VLM 配置 ---\n");

    // 创建自定义 VLM 配置
    let vlm_config = VlmConfig::new(
        "https://zazaz.top/v1",
        "sk-zaza-tr2mfIus1HU97JS2CMSgj6zPJNxPUr5jgL4s",
        "./Qwen3.5-27B-FP8",
    );

    // 使用自定义 VLM 配置创建管线
    let pipeline = AnalysisPipeline::with_vlm_config(AnalysisConfig::default(), vlm_config);

    let svg = r#"<svg width="100" height="100">
        <line x1="0" y1="0" x2="100" y2="0" />
        <line x1="0" y1="0" x2="0" y2="100" />
    </svg>"#;

    let result = pipeline.inject_from_svg_string_with_vlm(svg, "分析这个几何图形")?;

    if let Some(ref vlm) = result.vlm_response {
        println!("VLM 回答：{}", vlm.content);
    }

    Ok(())
}
