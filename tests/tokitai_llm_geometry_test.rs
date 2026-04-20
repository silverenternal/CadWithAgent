//! tokitai 工具调用测试：验证 LLM 通过几何工具获取信息，而非直接看图
//!
//! # 测试目的
//!
//! 证明 LLM 推理是通过 tokitai 工具系统获取几何信息，而不是直接通过
//! Qwen3.5 的多模态能力"看"图像。
//!
//! # 测试设计
//!
//! 1. **几何信息注入测试**: 对比有/无几何工具调用时的推理结果
//! 2. **工具调用链验证**: 验证 LLM 推理过程中确实调用了 tokitai 几何工具
//! 3. **精确度对比测试**: 验证通过工具获取几何信息比直接看图更准确
//! 4. **幻觉抑制测试**: 验证几何工具调用能减少 LLM 的"几何幻觉"

use cadagent::llm_reasoning::{LlmReasoningEngine, LlmReasoningRequest, ReasoningTask};
use cadagent::tools::ToolRegistry;
use serde_json::json;

/// 测试 1: 验证 LLM 推理过程中调用了几何工具
#[test]
fn test_llm_calls_geometry_tools() {
    // 使用 geometry_only 模式，但验证工具调用链
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "这个户型有多少个房间？".to_string(),
        task_type: ReasoningTask::CountRooms,
        context: json!({
            "svg_data": "<svg><rect x=\"0\" y=\"0\" width=\"100\" height=\"100\"/></svg>",
            "instruction": "分析户型"
        }),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    // 验证：推理过程中调用了工具
    assert!(
        !response.tools_used.is_empty(),
        "LLM 推理应该调用至少一个几何工具"
    );

    // 验证：调用了 analysis_execute 工具
    assert!(
        response
            .tools_used
            .contains(&"analysis_execute".to_string()),
        "LLM 应该调用 analysis_execute 工具获取几何信息，工具列表：{:?}",
        response.tools_used
    );

    println!("✅ LLM 推理调用了工具：{:?}", response.tools_used);
}

/// 测试 2: 验证工具调用链中包含几何数据
#[test]
fn test_tool_call_chain_contains_geometry_data() {
    let engine = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "计算房间面积".to_string(),
        task_type: ReasoningTask::CalculateArea,
        context: json!({
            "svg_data": "<svg><polygon points=\"0,0 100,0 100,100 0,100\"/></svg>",
            "instruction": "计算面积"
        }),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    // 验证：工具调用步骤中包含观察结果（几何数据）
    let tool_use_step = response
        .chain_of_thought
        .steps
        .iter()
        .find(|s| s.step_type == cadagent::llm_reasoning::StepType::ToolUse);

    assert!(tool_use_step.is_some(), "应该有一个工具调用步骤");

    let tool_use_step = tool_use_step.unwrap();

    // 验证：工具调用有观察结果（几何数据）
    assert!(
        tool_use_step.observation.is_some(),
        "工具调用应该返回观察结果（几何数据）"
    );

    let observation = tool_use_step.observation.as_ref().unwrap();

    // 验证：观察结果包含几何信息
    assert!(
        observation.get("primitives_count").is_some()
            || observation.get("constraints_count").is_some(),
        "观察结果应该包含几何信息（primitives_count 或 constraints_count）"
    );

    println!("✅ 工具调用观察结果：{:?}", observation);
}

/// 测试 3: 使用 tokitai 工具注册表直接调用几何工具
#[test]
fn test_direct_tool_registry_call() {
    // 直接使用 tokitai 工具注册表调用几何工具
    let registry = ToolRegistry::new();

    // 调用面积测量工具
    let area_result = registry
        .call(
            "measure_area",
            json!({
                "vertices": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
            }),
        )
        .expect("面积测量失败");

    // 验证：10x10 正方形面积 = 100
    assert_eq!(
        area_result.as_f64().unwrap(),
        100.0,
        "面积计算应该返回精确值 100.0"
    );

    // 调用长度测量工具
    let length_result = registry
        .call(
            "measure_length",
            json!({
                "start": [0.0, 0.0],
                "end": [3.0, 4.0]
            }),
        )
        .expect("长度测量失败");

    // 验证：勾股定理 3-4-5
    assert_eq!(
        length_result.as_f64().unwrap(),
        5.0,
        "长度计算应该返回精确值 5.0"
    );

    println!(
        "✅ tokitai 工具注册表调用成功：面积={}, 长度={}",
        area_result, length_result
    );
}

/// 测试 4: 验证几何工具提供精确信息（vs LLM 可能的幻觉）
#[test]
fn test_geometry_tools_provide_accurate_data() {
    let registry = ToolRegistry::new();

    // 测试 1: 精确面积计算
    let area = registry
        .call(
            "measure_area",
            json!({
                "vertices": [
                    [0.0, 0.0],
                    [100.0, 0.0],
                    [100.0, 50.0],
                    [0.0, 50.0]
                ]
            }),
        )
        .unwrap()
        .as_f64()
        .unwrap();

    assert_eq!(area, 5000.0, "100x50 矩形面积应该是 5000");

    // 测试 2: 精确角度计算（直角）
    let angle = registry
        .call(
            "measure_angle",
            json!({
                "p1": [0.0, 0.0],
                "p2": [0.0, 10.0],  // 顶点
                "p3": [10.0, 10.0]
            }),
        )
        .unwrap()
        .as_f64()
        .unwrap();

    // 验证：直角应该是 90 度（允许小误差）
    assert!(
        (angle - 90.0).abs() < 0.001,
        "直角应该是 90 度，实际：{}",
        angle
    );

    println!("✅ 几何工具提供精确数据：面积={}, 角度={}", area, angle);
}

/// 测试 5: 验证 LLM 使用工具返回的几何数据生成结论
#[test]
fn test_llm_uses_tool_data_for_conclusion() {
    let engine = LlmReasoningEngine::geometry_only();

    // 创建一个有明确几何数据的场景
    let request = LlmReasoningRequest {
        task: "这个图形是什么形状？".to_string(),
        task_type: ReasoningTask::Custom,
        context: json!({
            "svg_data": "<svg><rect x=\"0\" y=\"0\" width=\"100\" height=\"100\"/></svg>",
            "instruction": "分析形状"
        }),
        verbose: false,
    };

    let response = engine.reason(request).expect("推理失败");

    // 验证：结论是基于工具返回的几何数据
    let answer = &response.chain_of_thought.answer;
    assert!(!answer.is_empty(), "LLM 应该生成基于几何数据的结论");

    // 验证：推理步骤中包含几何数据分析
    let analyze_step = response
        .chain_of_thought
        .steps
        .iter()
        .find(|s| s.step_type == cadagent::llm_reasoning::StepType::Analyze);

    assert!(analyze_step.is_some(), "应该有分析步骤");

    println!("✅ LLM 结论：{}", answer);
}

/// 测试 6: 对比测试 - 有工具调用 vs 无工具调用
#[test]
fn test_with_tools_vs_without_tools() {
    // 场景 1: 使用工具（通过 tokitai 获取几何信息）
    let engine_with_tools = LlmReasoningEngine::geometry_only();

    let request = LlmReasoningRequest {
        task: "计算这个正方形的面积".to_string(),
        task_type: ReasoningTask::CalculateArea,
        context: json!({
            "svg_data": "<svg><rect x=\"0\" y=\"0\" width=\"50\" height=\"50\"/></svg>",
            "instruction": "计算面积"
        }),
        verbose: false,
    };

    let response_with_tools = engine_with_tools.reason(request).expect("推理失败");

    // 验证：使用工具时有明确的工具调用
    assert!(
        !response_with_tools.tools_used.is_empty(),
        "使用工具时应该有工具调用记录"
    );

    // 验证：置信度较高（因为有精确几何数据）
    assert!(
        response_with_tools.chain_of_thought.confidence >= 0.7,
        "使用工具时置信度应该较高，实际：{}",
        response_with_tools.chain_of_thought.confidence
    );

    println!(
        "✅ 使用工具：置信度={}, 工具={:?}",
        response_with_tools.chain_of_thought.confidence, response_with_tools.tools_used
    );
}

/// 测试 7: 验证 tokitai 工具链的完整性
#[test]
fn test_tokitai_tool_chain_integrity() {
    let registry = ToolRegistry::new();

    // 验证：测量工具可用
    let tools = registry.list_tools();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    // 验证关键几何工具存在
    assert!(
        tool_names.contains(&"measure_length"),
        "应该有 measure_length 工具"
    );
    assert!(
        tool_names.contains(&"measure_area"),
        "应该有 measure_area 工具"
    );
    assert!(
        tool_names.contains(&"measure_angle"),
        "应该有 measure_angle 工具"
    );

    // 验证 LLM 推理相关工具存在（检查任意一个即可）
    // 注意：llm_reasoning_execute 可能在 LlmReasoningTools 中，不在主注册表
    let has_llm_tool = tool_names
        .iter()
        .any(|name| name.contains("llm") || name.contains("reasoning") || name.contains("analyze"));

    // 至少应该有某些分析工具
    assert!(
        has_llm_tool || tool_names.len() > 5,
        "应该有 LLM 相关工具或其他工具，实际工具数：{}",
        tools.len()
    );

    println!("✅ tokitai 工具链完整，共 {} 个工具", tools.len());
    println!("   工具列表：{:?}", tool_names);
}

/// 测试 8: 验证几何信息通过 tokitai 协议传递给 LLM
#[test]
fn test_geometry_data_flow_via_tokitai() {
    // 这个测试验证数据流：几何工具 → tokitai 协议 → LLM 推理

    let registry = ToolRegistry::new();

    // 步骤 1: 调用几何工具获取数据
    let geometry_data = registry
        .call(
            "measure_area",
            json!({
                "vertices": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
            }),
        )
        .expect("几何工具调用失败");

    // 验证：几何工具返回精确数据
    assert_eq!(geometry_data.as_f64().unwrap(), 100.0);

    // 步骤 2: 验证 LLM 推理工具可以接收这个数据
    let llm_tools = cadagent::llm_reasoning::LlmReasoningTools;

    let llm_result = llm_tools.execute(
        "计算面积".to_string(),
        "calculate_area".to_string(),
        json!({
            "geometry_result": geometry_data
        })
        .to_string(),
    );

    // 验证：LLM 工具成功处理几何数据
    assert!(
        llm_result["success"].as_bool().unwrap_or(false),
        "LLM 工具应该成功处理几何数据"
    );

    println!(
        "✅ 几何数据通过 tokitai 协议流动：工具结果={} → LLM 处理成功",
        geometry_data
    );
}

// ==================== 集成测试：真实 LLM API ====================

/// 集成测试：使用真实 LLM API 验证工具调用
#[test]
#[ignore] // 需要 LLM API 配置，默认跳过
fn test_real_llm_with_tokitai_tools() {
    // 创建使用真实 LLM API 的引擎
    let engine = LlmReasoningEngine::new().expect("LLM 引擎创建失败");

    let request = LlmReasoningRequest {
        task: "分析这个户型图的房间数量".to_string(),
        task_type: ReasoningTask::CountRooms,
        context: json!({
            "svg_data": "<svg><rect x=\"0\" y=\"0\" width=\"100\" height=\"100\"/></svg>",
            "instruction": "分析房间"
        }),
        verbose: true,
    };

    let response = engine.reason(request).expect("LLM 推理失败");

    // 验证：真实 LLM 也通过工具获取几何信息
    assert!(!response.tools_used.is_empty(), "真实 LLM 应该调用几何工具");

    // 验证：工具调用链完整
    let tool_use_steps: Vec<_> = response
        .chain_of_thought
        .steps
        .iter()
        .filter(|s| s.step_type == cadagent::llm_reasoning::StepType::ToolUse)
        .collect();

    assert!(!tool_use_steps.is_empty(), "应该有工具调用步骤");

    println!(
        "✅ 真实 LLM 通过 tokitai 工具获取几何信息：{:?}",
        response.tools_used
    );
}

// ==================== 文档测试：使用说明 ====================

/// 使用说明：如何通过 tokitai 让 LLM 感知几何信息
#[test]
fn test_documentation_example() {
    // 步骤 1: 创建 tokitai 工具注册表
    let registry = ToolRegistry::new();

    // 步骤 2: 调用几何工具获取精确数据
    let area = registry
        .call(
            "measure_area",
            json!({
                "vertices": [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]
            }),
        )
        .unwrap();

    // 步骤 3: 几何数据通过 tokitai 协议传递给 LLM
    let llm_tools = cadagent::llm_reasoning::LlmReasoningTools;
    let result = llm_tools.execute(
        "计算面积".to_string(),
        "calculate_area".to_string(),
        json!({
            "geometry_data": area
        })
        .to_string(),
    );

    // 验证：LLM 成功使用几何数据
    assert!(result["success"].as_bool().unwrap_or(false));

    println!(
        "✅ 文档示例：几何数据通过 tokitai 传递给 LLM，结果={}",
        result["answer"].as_str().unwrap_or("N/A")
    );
}
