//! 分析管线和 LLM 推理协同使用示例
//!
//! 演示 analysis 与 llm_reasoning 如何配合工作：
//! - analysis: 工具增强上下文注入（确定性几何处理）
//! - llm_reasoning: LLM 驱动的思维链推理
//!
//! # 核心设计理念
//!
//! ```text
//! ┌─────────────────┐        ┌─────────────────┐
//! │  Analysis        │───────▶│  LLM Reasoning  │
//! │  (几何处理层)    │        │  (推理决策层)    │
//! │  提供精准数据    │        │  生成可解释结论  │
//! └─────────────────┘        └─────────────────┘
//! ```

use cadagent::llm_reasoning::{LlmReasoningEngine, LlmReasoningRequest, ReasoningTask};
use cadagent::prelude::*;
use serde_json::json;

fn main() {
    println!("=== 分析管线 + LLM 推理 协同示例 ===\n");

    // ==================== 示例 1: 单独使用 analysis 管线 ====================
    println!("【示例 1】单独使用 analysis 管线（确定性工具）");

    // 创建一些测试基元
    let primitives = vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
        Primitive::Line(Line::from_coords([100.0, 100.0], [0.0, 100.0])),
        Primitive::Line(Line::from_coords([0.0, 100.0], [0.0, 0.0])),
    ];

    // 使用几何推理器检测关系
    let reasoner = GeometricRelationReasoner::with_defaults();
    let relations_result = reasoner.find_all_relations(&primitives);

    println!("  基元数量：{} 个", primitives.len());
    println!(
        "  检测到的几何关系：{} 个",
        relations_result.relations.len()
    );
    println!(
        "  - 平行关系：{} 对",
        relations_result.statistics.parallel_count
    );
    println!(
        "  - 垂直关系：{} 对",
        relations_result.statistics.perpendicular_count
    );
    println!(
        "  - 连接关系：{} 对",
        relations_result.statistics.connected_count
    );
    println!();

    // ==================== 示例 2: 使用 analysis 管线 ====================
    println!("【示例 2】使用 analysis 管线（上下文注入）");

    // 注意：需要设置环境变量 PROVIDER_ZAZAZ_API_KEY
    // 这里演示不使用 VLM 的模式
    let svg_content = r#"
        <svg width="100" height="100" viewBox="0 0 100 100">
            <line x1="0" y1="0" x2="100" y2="0" />
            <line x1="100" y1="0" x2="100" y2="100" />
            <line x1="100" y1="100" x2="0" y2="100" />
            <line x1="0" y1="100" x2="0" y2="0" />
        </svg>
    "#;

    match AnalysisPipeline::with_defaults() {
        Ok(pipeline) => match pipeline.inject_from_svg_string(svg_content, "分析这个矩形") {
            Ok(result) => {
                println!("  提示词长度：{} 字符", result.prompt.full_prompt.len());
                println!("  基元数量：{} 个", result.primitives.len());
                println!("  约束数量：{} 个", result.relations.len());
                println!("  执行耗时：{} ms", result.total_latency_ms);
            }
            Err(e) => {
                println!("  注入上下文失败：{}", e);
                println!("  （可能是 API Key 未设置，这是正常的）");
            }
        },
        Err(e) => {
            println!("  创建管线失败：{}", e);
            println!("  （可能是 API Key 未设置，这是正常的）");
        }
    }
    println!();

    // ==================== 示例 3: 使用 llm_reasoning 引擎 ====================
    println!("【示例 3】使用 llm_reasoning 引擎（LLM 思维链）");

    let engine = LlmReasoningEngine::new();
    let request = LlmReasoningRequest {
        task: "这个户型有多少个房间？".to_string(),
        task_type: ReasoningTask::CountRooms,
        context: json!({
            "svg_data": svg_content,
            "instruction": "分析这个图形"
        }),
        verbose: false,
    };

    let response = engine.reason(request).unwrap();

    println!("  任务：{}", response.chain_of_thought.task);
    println!("  答案：{}", response.chain_of_thought.answer);
    println!(
        "  置信度：{:.0}%",
        response.chain_of_thought.confidence * 100.0
    );
    println!("  推理步骤：{} 步", response.chain_of_thought.steps.len());
    println!("  使用工具：{:?}", response.tools_used);
    println!();

    // ==================== 示例 4: 展示思维链详情 ====================
    println!("【示例 4】思维链详情（LLM 推理过程）");

    use cadagent::llm_reasoning::StepType;

    for step in &response.chain_of_thought.steps {
        let step_type_str = match step.step_type {
            StepType::Understand => "理解",
            StepType::Plan => "规划",
            StepType::ToolUse => "工具",
            StepType::Analyze => "分析",
            StepType::Verify => "验证",
            StepType::Revise => "修正",
            StepType::Conclude => "结论",
        };

        println!("  [步骤 {}] {}", step.id, step_type_str);
        println!("    思考：{}", truncate(&step.thought, 60));

        if let Some(ref conclusion) = step.conclusion {
            println!("    结论：{}", conclusion);
        }

        if let Some(ref tool_call) = step.tool_call {
            println!("    工具：{} ({:?})", tool_call.tool_name, tool_call.status);
        }
    }
    println!();

    // ==================== 示例 5: 使用工具注册表 ====================
    println!("【示例 5】使用工具注册表");

    let registry = ToolRegistry::default();

    // 测量长度
    let result = registry
        .call(
            "measure_length",
            json!({
                "start": [0.0, 0.0],
                "end": [3.0, 4.0]
            }),
        )
        .unwrap();
    println!("  测量长度 (0,0) 到 (3,4): {}", result);

    // 测量面积
    let result = registry
        .call(
            "measure_area",
            json!({
                "vertices": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
            }),
        )
        .unwrap();
    println!("  测量面积 10x10 正方形：{}", result);
    println!();

    // ==================== 示例 6: 架构对比 ====================
    println!("【示例 6】两个模块的对比");
    println!("  ┌─────────────────────┬──────────────────────┐");
    println!("  │ Analysis            │ LLM Reasoning        │");
    println!("  ├─────────────────────┼──────────────────────┤");
    println!("  │ 确定性算法          │ AI 思维链             │");
    println!("  │ 固定流程            │ 动态生成步骤          │");
    println!("  │ 数学计算            │ LLM 推理决策          │");
    println!("  │ 输出结构化数据      │ 输出可解释结论        │");
    println!("  │ 作为工具被调用      │ 调用其他工具          │");
    println!("  └─────────────────────┴──────────────────────┘");
    println!();

    println!("=== 示例结束 ===");
}

fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect::<String>()
}
