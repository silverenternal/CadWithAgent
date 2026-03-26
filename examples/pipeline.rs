//! CadAgent 完整管线示例
//!
//! 演示从 SVG 解析到 DXF 导出的完整流程

use cadagent::cot::generator::GeoCotGenerator;
use cadagent::cot::qa::QaGenerator;
use cadagent::export::{dxf::DxfExporter, json::JsonExporter};
use cadagent::metrics::ConsistencyChecker;
use cadagent::parser::svg::SvgParser;

fn main() -> anyhow::Result<()> {
    println!("=== CadAgent 完整管线示例 ===\n");

    // 创建一个示例 SVG 内容（简单户型图）
    let svg_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="800" height="600" xmlns="http://www.w3.org/2000/svg">
  <!-- 房间 1: 客厅 -->
  <polygon points="50,50 400,50 400,350 50,350" fill="none" stroke="black"/>

  <!-- 房间 2: 卧室 -->
  <polygon points="400,50 750,50 750,350 400,350" fill="none" stroke="black"/>

  <!-- 房间 3: 厨房 -->
  <polygon points="50,350 400,350 400,550 50,550" fill="none" stroke="black"/>

  <!-- 门标记 -->
  <text x="225" y="350" font-size="20">门</text>
  <text x="575" y="350" font-size="20">门</text>
</svg>"#;

    // 保存 SVG 到临时文件
    let svg_path = std::env::temp_dir().join("floor_plan.svg");
    std::fs::write(&svg_path, svg_content)?;
    println!("1. 创建示例 SVG 文件：{}", svg_path.display());

    // 步骤 1: 解析 SVG
    println!("\n2. 解析 SVG 文件");
    let svg_result = SvgParser::parse(&svg_path)?;
    println!("   解析到 {} 个图元", svg_result.primitives.len());
    println!(
        "   SVG 尺寸：{} x {}",
        svg_result.metadata.width, svg_result.metadata.height
    );

    // 步骤 2: 导出为结构化 JSON
    println!("\n3. 导出为结构化 JSON");
    let json_path = std::env::temp_dir().join("floor_plan.json");
    let json_result = JsonExporter::export(&svg_result.primitives, &json_path)?;
    println!(
        "   已保存：{} ({} 个图元)",
        json_path.display(),
        json_result.entity_count
    );

    // 步骤 3: 生成 Geo-CoT 数据
    println!("\n4. 生成 Geo-CoT 数据");
    let generator = GeoCotGenerator::new();
    let cot_data = generator.generate(&svg_result.primitives, "分析户型图的房间结构");

    println!("   任务：{}", cot_data.task);
    println!(
        "   感知摘要：{}...",
        cot_data.perception.chars().take(50).collect::<String>()
    );
    println!(
        "   推理摘要：{}...",
        cot_data.reasoning.chars().take(50).collect::<String>()
    );
    println!("   答案：{}", cot_data.answer);

    // 保存 CoT 数据
    let cot_json_path = std::env::temp_dir().join("floor_plan_cot.json");
    JsonExporter::export_with_cot(
        &svg_result.primitives,
        &cot_data.thinking,
        &cot_data.answer,
        &cot_json_path,
    )?;
    println!("   CoT 数据已保存：{}", cot_json_path.display());

    // 步骤 4: 生成 QA 数据集
    println!("\n5. 生成 QA 数据集");
    let qa_generator = QaGenerator::new();
    let qa_pairs = qa_generator.generate_all(&svg_result.primitives);
    println!("   生成了 {} 个问答对", qa_pairs.len());

    for (i, qa) in qa_pairs.iter().take(3).enumerate() {
        println!("   Q{}: {}", i + 1, qa.question);
        println!(
            "      A: {}",
            qa.answer.chars().take(60).collect::<String>()
        );
    }

    // 保存 QA 数据
    let qa_json_path = std::env::temp_dir().join("floor_plan_qa.json");
    let qa_content = serde_json::to_string_pretty(&qa_pairs)?;
    std::fs::write(&qa_json_path, qa_content)?;
    println!("   QA 数据已保存：{}", qa_json_path.display());

    // 步骤 5: 一致性检查
    println!("\n6. 几何一致性检查");
    let checker = ConsistencyChecker::new();
    let consistency_result = checker.check_all(&svg_result.primitives);

    println!("   一致性得分：{:.2}", consistency_result.score);
    println!(
        "   检查结果：{}",
        if consistency_result.passed {
            "通过 ✓"
        } else {
            "失败 ✗"
        }
    );

    for check in &consistency_result.checks {
        println!(
            "   - {}: {}",
            check.name,
            if check.passed { "✓" } else { "✗" }
        );
    }

    // 步骤 6: 导出 DXF
    println!("\n7. 导出 DXF 文件");
    let dxf_path = std::env::temp_dir().join("floor_plan.dxf");
    let dxf_result = DxfExporter::export(&svg_result.primitives, &dxf_path)?;
    println!(
        "   已保存：{} ({} 个图元)",
        dxf_path.display(),
        dxf_result.entity_count
    );

    println!("\n=== 管线完成 ===");
    println!("\n生成的文件:");
    println!("  - SVG: {}", svg_path.display());
    println!("  - JSON: {}", json_path.display());
    println!("  - CoT JSON: {}", cot_json_path.display());
    println!("  - QA JSON: {}", qa_json_path.display());
    println!("  - DXF: {}", dxf_path.display());

    Ok(())
}
