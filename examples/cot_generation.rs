//! Geo-CoT 生成示例
//!
//! 演示如何生成几何思维链数据

use cadagent::prelude::*;
use cadagent::cot::generator::GeoCotGenerator;
use cadagent::cot::qa::QaGenerator;
use cadagent::export::json::JsonExporter;

fn main() -> anyhow::Result<()> {
    println!("=== Geo-CoT 生成示例 ===\n");

    // 创建一个示例户型图
    let floor_plan = vec![
        // 外墙轮廓
        Primitive::Polygon(Polygon::from_coords(vec![
            [0.0, 0.0],
            [800.0, 0.0],
            [800.0, 600.0],
            [0.0, 600.0],
        ])),
        
        // 房间 1: 客厅
        Primitive::Polygon(Polygon::from_coords(vec![
            [50.0, 50.0],
            [400.0, 50.0],
            [400.0, 350.0],
            [50.0, 350.0],
        ])),
        
        // 房间 2: 卧室
        Primitive::Polygon(Polygon::from_coords(vec![
            [400.0, 50.0],
            [750.0, 50.0],
            [750.0, 350.0],
            [400.0, 350.0],
        ])),
        
        // 房间 3: 厨房
        Primitive::Polygon(Polygon::from_coords(vec![
            [50.0, 350.0],
            [400.0, 350.0],
            [400.0, 550.0],
            [50.0, 550.0],
        ])),
        
        // 房间 4: 卫生间
        Primitive::Polygon(Polygon::from_coords(vec![
            [400.0, 350.0],
            [750.0, 350.0],
            [750.0, 550.0],
            [400.0, 550.0],
        ])),
        
        // 门标记
        Primitive::Text {
            content: "门".to_string(),
            position: Point::new(225.0, 350.0),
            height: 20.0,
        },
        Primitive::Text {
            content: "门".to_string(),
            position: Point::new(575.0, 350.0),
            height: 20.0,
        },
    ];

    println!("户型图包含 {} 个图元\n", floor_plan.len());

    // 1. 生成不同任务的 Geo-CoT 数据
    println!("1. 生成不同任务的 Geo-CoT 数据\n");
    
    let generator = GeoCotGenerator::new();
    
    let tasks = vec![
        "计算所有房间的面积",
        "检测房间的数量和位置",
        "分析户型图的结构",
        "计算建筑的总宽度和高度",
        "找出最大的房间",
    ];

    for task in &tasks {
        println!("任务：{}", task);
        let cot_data = generator.generate(&floor_plan, task);
        
        println!("  <thinking>");
        println!("    {}", cot_data.perception.replace('\n', "\n    "));
        println!("    {}", cot_data.reasoning.replace('\n', "\n    "));
        println!("  </thinking>");
        println!("  答案：{}\n", cot_data.answer);
    }

    // 2. 生成 QA 数据集
    println!("2. 生成 QA 数据集\n");
    
    let qa_generator = QaGenerator::new();
    let qa_pairs = qa_generator.generate_all(&floor_plan);
    
    println!("生成了 {} 个问答对:\n", qa_pairs.len());
    
    for (i, qa) in qa_pairs.iter().enumerate() {
        println!("Q{}: {}", i + 1, qa.question);
        println!("   类型：{}", qa.question_type);
        if let Some(thinking) = &qa.thinking {
            println!("   <thinking>{}</thinking>", thinking);
        }
        println!("   A: {}\n", qa.answer);
    }

    // 3. 导出训练数据
    println!("3. 导出训练数据\n");
    
    // 导出 CoT 数据
    for (i, task) in tasks.iter().enumerate() {
        let cot_data = generator.generate(&floor_plan, task);
        let output_path = format!("/tmp/cot_data_{}.json", i);
        
        JsonExporter::export_with_cot(
            &floor_plan,
            &cot_data.thinking,
            &cot_data.answer,
            &output_path,
        )?;
        println!("   已保存：{}", output_path);
    }

    // 导出 QA 数据
    let qa_output_path = "/tmp/qa_dataset.json";
    let qa_content = serde_json::to_string_pretty(&qa_pairs)?;
    std::fs::write(qa_output_path, qa_content)?;
    println!("   已保存：{}", qa_output_path);

    // 4. 生成多轮对话数据
    println!("\n4. 生成多轮对话数据\n");
    
    let conversation = vec![
        ("用户".to_string(), "这个户型图有多少个房间？".to_string()),
        ("助手".to_string(), generator.generate(&floor_plan, "检测房间数量").answer),
        ("用户".to_string(), "最大的房间是哪个？".to_string()),
        ("助手".to_string(), generator.generate(&floor_plan, "找出最大的房间").answer),
        ("用户".to_string(), "所有房间的总面积是多少？".to_string()),
        ("助手".to_string(), generator.generate(&floor_plan, "计算总面积").answer),
    ];

    println!("多轮对话:");
    for (role, content) in &conversation {
        println!("  {}: {}...", role, content.chars().take(60).collect::<String>());
    }

    // 保存对话数据
    let conversation_data = conversation.iter().map(|(role, content)| {
        serde_json::json!({
            "role": role,
            "content": content
        })
    }).collect::<Vec<_>>();
    
    let conv_output_path = "/tmp/conversation_data.json";
    let conv_content = serde_json::to_string_pretty(&conversation_data)?;
    std::fs::write(conv_output_path, conv_content)?;
    println!("\n   已保存：{}", conv_output_path);

    println!("\n=== Geo-CoT 生成完成 ===");
    
    Ok(())
}
