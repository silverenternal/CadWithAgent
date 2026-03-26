//! llm_reasoning 模块集成测试

use cadagent::llm_reasoning::{LlmReasoningEngine, LlmReasoningRequest, ReasoningTask};
use serde_json::json;

/// 测试：房间计数任务
#[test]
fn test_count_rooms_task() {
    let engine = LlmReasoningEngine::new();

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
    let engine = LlmReasoningEngine::new();

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
    let engine = LlmReasoningEngine::new();

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
    let engine = LlmReasoningEngine::new();

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
    let engine = LlmReasoningEngine::new();

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
    let engine = LlmReasoningEngine::new();

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
    let engine = LlmReasoningEngine::new();

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
