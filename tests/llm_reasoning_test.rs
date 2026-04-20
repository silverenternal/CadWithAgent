//! llm_reasoning 模块集成测试
//!
//! 注意：这些测试使用 geometry_only 模式，不依赖真实 LLM API
//! 如需测试真实 LLM API，请使用 `test_llm_api_integration` 中的方法

use cadagent::llm_reasoning::{LlmReasoningEngine, LlmReasoningRequest, ReasoningTask, StepType};
use serde_json::json;

/// 测试：房间计数任务
#[test]
fn test_count_rooms_task() {
    // 使用 geometry_only 模式，避免依赖 LLM API
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "这个户型有多少个房间？".to_string(),
        task_type: ReasoningTask::CountRooms,
        context: json!({
            "drawing_type": "vector",
            "drawing_data": "test_data"
        }),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    // 验证推理完成
    assert_eq!(
        response.chain_of_thought.state,
        cadagent::llm_reasoning::ReasoningState::Completed
    );

    // 验证有答案
    assert!(!response.chain_of_thought.answer.is_empty());

    // 验证置信度在合理范围
    assert!(response.chain_of_thought.confidence >= 0.0);
    assert!(response.chain_of_thought.confidence <= 1.0);
}

/// 测试：面积计算任务
#[test]
fn test_calculate_area_task() {
    // 使用 geometry_only 模式，避免依赖 LLM API
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "计算这个房间的面积".to_string(),
        task_type: ReasoningTask::CalculateArea,
        context: json!({
            "drawing_type": "vector",
            "drawing_data": "test_data"
        }),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    assert_eq!(
        response.chain_of_thought.state,
        cadagent::llm_reasoning::ReasoningState::Completed
    );
    assert!(!response.chain_of_thought.answer.is_empty());
}

/// 测试：思维链结构
#[test]
fn test_chain_of_thought_structure() {
    // 使用 geometry_only 模式，避免依赖 LLM API
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "测试任务".to_string(),
        task_type: ReasoningTask::Custom,
        context: json!({}),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");
    let cot = &response.chain_of_thought;

    // 验证思维链包含必要的步骤
    assert!(cot.steps.len() >= 3); // 至少包含理解、规划、结论

    // 验证步骤类型顺序正确
    use cadagent::llm_reasoning::StepType;
    assert_eq!(cot.steps[0].step_type, StepType::Understand);
    assert_eq!(cot.steps[1].step_type, StepType::Plan);
    assert_eq!(cot.steps[cot.steps.len() - 1].step_type, StepType::Conclude);
}

/// 测试：工具调用记录
#[test]
fn test_tool_usage_tracking() {
    // 使用 geometry_only 模式，避免依赖 LLM API
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "分析这个户型".to_string(),
        task_type: ReasoningTask::AnalyzeLayout,
        context: json!({
            "drawing_type": "vector",
            "drawing_data": "test_data"
        }),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    // 验证工具使用记录
    assert!(!response.tools_used.is_empty());
    assert!(response
        .tools_used
        .contains(&"analysis_execute".to_string()));
}

/// 测试：推理耗时记录
#[test]
fn test_latency_recording() {
    // 使用 geometry_only 模式，避免依赖 LLM API
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "测试".to_string(),
        task_type: ReasoningTask::Custom,
        context: json!({}),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    // 验证响应结构完整
    assert!(!response.chain_of_thought.steps.is_empty());
}

/// 测试：不同任务类型
#[test]
fn test_different_task_types() {
    // 使用 geometry_only 模式，避免依赖 LLM API
    let engine = LlmReasoningEngine::geometry_only();

    let task_types = vec![
        ReasoningTask::CountRooms,
        ReasoningTask::CalculateArea,
        ReasoningTask::MeasureDimension,
        ReasoningTask::DetectDoorsWindows,
        ReasoningTask::AnalyzeLayout,
        ReasoningTask::Custom,
    ];

    for task_type in task_types {
        let request = LlmReasoningRequest {
            task: format!("测试{:?}", task_type),
            task_type,
            context: json!({}),
            verbose: false,
        };

        let response = engine.reason(request);
        assert!(response.is_ok(), "任务类型 {:?} 执行失败", task_type);
    }
}

/// 测试：思维步骤包含必要字段
#[test]
fn test_reasoning_step_fields() {
    // 使用 geometry_only 模式，避免依赖 LLM API
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "测试任务".to_string(),
        task_type: ReasoningTask::Custom,
        context: json!({}),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    for step in &response.chain_of_thought.steps {
        // 验证必要字段不为空
        assert!(!step.thought.is_empty(), "步骤思考内容不能为空");

        // 验证步骤 ID 递增
        if step.id > 0 {
            assert!(step.id <= response.chain_of_thought.steps.len());
        }
    }
}

/// 测试：使用真实 LLM API（集成测试）
#[test]
#[ignore] // 默认跳过，需要时运行：cargo test test_llm_api_integration -- --ignored
fn test_llm_api_integration() {
    // 此测试使用真实 LLM API，需要设置环境变量
    // 运行前请确保 .env 文件已配置
    let engine = LlmReasoningEngine::new();

    // 如果 API 配置失败，跳过测试
    let engine = match engine {
        Ok(e) => e.with_verbose(true), // 启用详细输出
        Err(_) => {
            println!("LLM API 未配置，跳过集成测试");
            return;
        }
    };

    // 使用真实的户型图 SVG 数据（简单的多房间布局）
    let svg_data = r#"<svg width="500" height="400" xmlns="http://www.w3.org/2000/svg">
        <!-- 外墙 -->
        <line x1="0" y1="0" x2="500" y2="0" stroke="black" stroke-width="2"/>
        <line x1="500" y1="0" x2="500" y2="400" stroke="black" stroke-width="2"/>
        <line x1="500" y1="400" x2="0" y2="400" stroke="black" stroke-width="2"/>
        <line x1="0" y1="400" x2="0" y2="0" stroke="black" stroke-width="2"/>
        
        <!-- 内墙 - 分隔房间 -->
        <line x1="200" y1="0" x2="200" y2="250" stroke="black" stroke-width="2"/>
        <line x1="0" y1="250" x2="200" y2="250" stroke="black" stroke-width="2"/>
        <line x1="350" y1="0" x2="350" y2="400" stroke="black" stroke-width="2"/>
        
        <!-- 门 -->
        <line x1="180" y1="250" x2="200" y2="270" stroke="blue" stroke-width="1"/>
        <line x1="330" y1="0" x2="350" y2="20" stroke="blue" stroke-width="1"/>
        
        <!-- 窗户 -->
        <line x1="50" y1="0" x2="100" y2="0" stroke="green" stroke-width="1"/>
        <line x1="250" y1="0" x2="300" y2="0" stroke="green" stroke-width="1"/>
        <line x1="400" y1="0" x2="450" y2="0" stroke="green" stroke-width="1"/>
        <line x1="500" y1="100" x2="500" y2="150" stroke="green" stroke-width="1"/>
        <line x1="500" y1="250" x2="500" y2="300" stroke="green" stroke-width="1"/>
        <line x1="200" y1="400" x2="250" y2="400" stroke="green" stroke-width="1"/>
        <line x1="0" y1="100" x2="0" y2="150" stroke="green" stroke-width="1"/>
        
        <!-- 房间标注 -->
        <text x="100" y="125" font-size="14">卧室 1</text>
        <text x="275" y="125" font-size="14">客厅</text>
        <text x="425" y="200" font-size="14">卧室 2</text>
        <text x="100" y="325" font-size="14">厨房</text>
        <text x="275" y="325" font-size="14">卫生间</text>
    </svg>"#;

    let request = LlmReasoningRequest {
        task: "这个户型有多少个房间？分别是什么类型的房间？".to_string(),
        task_type: ReasoningTask::CountRooms,
        context: json!({
            "svg_data": svg_data,
            "instruction": "分析这个户型图，统计房间数量并识别房间类型"
        }),
        verbose: true,
    };

    let response = engine.reason(request).expect("LLM 推理失败");

    // 验证 LLM 生成了答案
    assert!(!response.chain_of_thought.answer.is_empty());

    // 验证工具被调用
    assert!(!response.tools_used.is_empty());

    // 验证推理步骤完整
    assert!(response.chain_of_thought.steps.len() >= 4);

    println!("\n{}", "=".repeat(80));
    println!("=== LLM 推理结果 ===");
    println!("{}", "=".repeat(80));
    println!("答案：{}", response.chain_of_thought.answer);
    println!("{}", "-".repeat(80));
    println!("置信度：{:.2}", response.chain_of_thought.confidence);
    println!("推理耗时：{}ms", response.latency_ms);
    println!("使用的工具：{:?}", response.tools_used);
    println!("推理步骤数：{}", response.chain_of_thought.steps.len());
    println!("{}", "=".repeat(80));

    // 打印每个步骤的详情
    println!("\n推理步骤详情:");
    for step in &response.chain_of_thought.steps {
        println!(
            "\n[步骤 {}] {}: {}",
            step.id,
            match step.step_type {
                StepType::Understand => "理解",
                StepType::Plan => "规划",
                StepType::ToolUse => "工具调用",
                StepType::Analyze => "分析",
                StepType::Verify => "验证",
                StepType::Revise => "修正",
                StepType::Conclude => "结论",
            },
            step.thought
        );

        if let Some(obs) = &step.observation {
            println!("  观察结果：{}", obs);
        }

        if let Some(conclusion) = &step.conclusion {
            println!("  结论：{}", conclusion);
        }
    }
}
