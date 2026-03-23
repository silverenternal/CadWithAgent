//! LLM 推理引擎
//!
//! 大模型驱动的思维链推理架构
//!
//! # 核心设计理念
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      LLM Reasoning Engine                    │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────┐    ┌──────────┐    ┌──────────┐              │
//! │  │ 理解任务  │───▶│ 规划步骤  │───▶│ 执行推理  │              │
//! │  └──────────┘    └──────────┘    └──────────┘              │
//! │       │               │               │                     │
//! │       ▼               ▼               ▼                     │
//! │  ┌──────────┐    ┌──────────┐    ┌──────────┐              │
//! │  │ 分析输入  │    │ 选择工具  │    │ 生成结论  │              │
//! │  └──────────┘    └──────────┘    └──────────┘              │
//! │                                                              │
//! │  ◀───────────────── 回溯循环 ─────────────────▶             │
//! │                                                              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # 与 analysis 模块的关系
//!
//! ```text
//! LLM Reasoning          Analysis Pipeline
//! ┌─────────────┐        ┌─────────────────┐
//! │  推理决策层  │───────▶│  几何处理层     │
//! │  (大脑)     │        │  (工具)         │
//! └─────────────┘        └─────────────────┘
//!      ▲                        │
//!      │◀───────────────────────│
//!           结构化数据返回
//! ```

use crate::llm_reasoning::types::*;
use crate::analysis::AnalysisPipeline;
use serde_json::json;
use thiserror::Error;

/// 推理引擎错误
#[derive(Debug, Error)]
pub enum ReasoningError {
    #[error("任务理解失败：{0}")]
    UnderstandError(String),

    #[error("规划失败：{0}")]
    PlanError(String),

    #[error("工具调用失败：{0}")]
    ToolError(String),

    #[error("推理失败：{0}")]
    InferenceError(String),
}

/// LLM 推理引擎
///
/// **注意：当前为模拟实现（Mock LLM）**
///
/// # 当前实现
/// - 使用预定义规则模板生成"伪 LLM"响应
/// - 演示思维链结构和工具调用流程
/// - 适合测试和原型开发
///
/// # TODO: 接入真实 LLM
/// - 替换 `understand_task` 为真实 LLM API 调用
/// - 替换 `generate_plan` 为 LLM 动态生成计划
/// - 替换 `generate_conclusion` 为 LLM 生成结论
/// - 参考 `bridge::vlm_client` 实现
pub struct LlmReasoningEngine {
    /// 是否启用详细日志
    verbose: bool,
    /// 几何处理流水线（作为工具使用）
    analysis_pipeline: Option<AnalysisPipeline>,
}

impl LlmReasoningEngine {
    /// 创建新的推理引擎
    pub fn new() -> Self {
        Self {
            verbose: false,
            analysis_pipeline: None,
        }
    }

    /// 设置详细日志模式
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 启用分析管线（需要 VLM API Key）
    pub fn with_analysis_pipeline(mut self) -> Result<Self, crate::bridge::vlm_client::VlmError> {
        self.analysis_pipeline = Some(AnalysisPipeline::with_defaults()?);
        Ok(self)
    }

    /// 执行推理任务
    ///
    /// 这是 LLM 驱动推理的核心入口
    pub fn reason(&self, request: LlmReasoningRequest) -> Result<LlmReasoningResponse, ReasoningError> {
        let start_time = std::time::Instant::now();

        // 步骤 1: 理解任务（LLM 生成）
        let understanding = self.understand_task(&request)?;

        // 步骤 2: 规划推理步骤（LLM 生成）
        let plan = self.generate_plan(&request, &understanding)?;

        // 步骤 3: 执行推理（LLM 驱动 + 工具调用）
        let mut steps = Vec::new();
        let mut tools_used = Vec::new();

        // 3.1 添加理解步骤
        steps.push(ReasoningStep {
            id: 0,
            step_type: StepType::Understand,
            thought: understanding,
            tool_call: None,
            observation: None,
            conclusion: None,
        });

        // 3.2 添加规划步骤
        steps.push(ReasoningStep {
            id: 1,
            step_type: StepType::Plan,
            thought: format!("我将按以下步骤执行：{}", 
                plan.plan_steps.iter()
                    .map(|s| format!("{}. {}", s.order, s.description))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            tool_call: None,
            observation: None,
            conclusion: Some("规划完成，开始执行".to_string()),
        });

        // 3.3 执行计划步骤
        for plan_step in &plan.plan_steps {
            let step_result = self.execute_plan_step(plan_step, &request.context)?;
            
            if let Some(tool_name) = &plan_step.tool_name {
                if !tools_used.contains(tool_name) {
                    tools_used.push(tool_name.clone());
                }
            }
            
            steps.push(step_result);
        }

        // 步骤 4: 生成结论（LLM 生成）
        let (answer, confidence) = self.generate_conclusion(&steps, &request)?;

        // 添加结论步骤
        steps.push(ReasoningStep {
            id: steps.len(),
            step_type: StepType::Conclude,
            thought: "基于以上分析，我可以得出结论".to_string(),
            tool_call: None,
            observation: None,
            conclusion: Some(answer.clone()),
        });

        let latency_ms = start_time.elapsed().as_millis() as u64;

        Ok(LlmReasoningResponse {
            chain_of_thought: ChainOfThought {
                task: request.task.clone(),
                task_type: request.task_type,
                steps,
                answer,
                confidence,
                state: ReasoningState::Completed,
            },
            tools_used,
            latency_ms,
        })
    }

    /// 步骤 1: 理解任务
    fn understand_task(&self, request: &LlmReasoningRequest) -> Result<String, ReasoningError> {
        // 实际项目中：调用 LLM API 生成任务理解
        // 这里模拟 LLM 的思考过程
        
        let task_analysis = match request.task_type {
            ReasoningTask::CountRooms => {
                "这是一个房间计数任务。我需要：1) 识别图纸中的封闭区域 2) 排除外部边界 3) 统计内部房间数量"
            }
            ReasoningTask::CalculateArea => {
                "这是一个面积计算任务。我需要：1) 识别目标区域边界 2) 使用鞋带公式计算面积 3) 考虑单位转换"
            }
            ReasoningTask::MeasureDimension => {
                "这是一个尺寸测量任务。我需要：1) 识别测量起点和终点 2) 计算欧几里得距离 3) 返回测量结果"
            }
            ReasoningTask::DetectDoorsWindows => {
                "这是一个门窗检测任务。我需要：1) 识别墙体上的缺口 2) 检测文本标记（D/W）3) 统计数量并定位"
            }
            ReasoningTask::AnalyzeLayout => {
                "这是一个户型分析任务。我需要：1) 识别所有房间 2) 分析房间连接关系 3) 判断户型类型"
            }
            ReasoningTask::Custom => {
                "这是一个自定义任务。我将根据具体内容进行分析"
            }
        };

        Ok(format!(
            "任务理解：{}\n\n用户问题：{}\n\n我将采用以下策略：{}",
            request.task_type.task_type_str(),
            request.task,
            task_analysis
        ))
    }

    /// 步骤 2: 生成推理计划
    fn generate_plan(&self, request: &LlmReasoningRequest, _understanding: &str) -> Result<ReasoningPlan, ReasoningError> {
        // 实际项目中：调用 LLM API 生成计划
        // 这里根据任务类型返回预定义的计划模板
        
        let plan_steps = match request.task_type {
            ReasoningTask::CountRooms => {
                vec![
                    PlanStep {
                        order: 1,
                        description: "调用分析管线识别基元".to_string(),
                        needs_tool: true,
                        tool_name: Some("analysis_execute".to_string()),
                        dependencies: vec![],
                    },
                    PlanStep {
                        order: 2,
                        description: "分析拓扑图识别封闭区域".to_string(),
                        needs_tool: false,
                        tool_name: None,
                        dependencies: vec![1],
                    },
                    PlanStep {
                        order: 3,
                        description: "排除外边界，统计房间数量".to_string(),
                        needs_tool: false,
                        tool_name: None,
                        dependencies: vec![2],
                    },
                ]
            }
            ReasoningTask::CalculateArea => {
                vec![
                    PlanStep {
                        order: 1,
                        description: "调用分析管线获取基元和约束".to_string(),
                        needs_tool: true,
                        tool_name: Some("analysis_execute".to_string()),
                        dependencies: vec![],
                    },
                    PlanStep {
                        order: 2,
                        description: "识别目标区域的多边形边界".to_string(),
                        needs_tool: false,
                        tool_name: None,
                        dependencies: vec![1],
                    },
                    PlanStep {
                        order: 3,
                        description: "使用鞋带公式计算面积".to_string(),
                        needs_tool: false,
                        tool_name: None,
                        dependencies: vec![2],
                    },
                ]
            }
            _ => {
                // 默认计划
                vec![
                    PlanStep {
                        order: 1,
                        description: "获取几何数据".to_string(),
                        needs_tool: true,
                        tool_name: Some("analysis_execute".to_string()),
                        dependencies: vec![],
                    },
                    PlanStep {
                        order: 2,
                        description: "分析数据并生成结论".to_string(),
                        needs_tool: false,
                        tool_name: None,
                        dependencies: vec![1],
                    },
                ]
            }
        };

        let required_tools = plan_steps.iter()
            .filter_map(|s| s.tool_name.clone())
            .collect();

        Ok(ReasoningPlan {
            task: request.task.clone(),
            plan_steps,
            required_tools,
        })
    }

    /// 步骤 3: 执行计划步骤
    fn execute_plan_step(&self, plan_step: &PlanStep, context: &serde_json::Value) -> Result<ReasoningStep, ReasoningError> {
        let thought = format!("执行步骤 {}: {}", plan_step.order, plan_step.description);

        // 如果需要调用工具
        if plan_step.needs_tool {
            if let Some(ref tool_name) = plan_step.tool_name {
                if tool_name == "analysis_execute" {
                    // 调用分析管线
                    let result = self.call_geometry_pipeline(context);

                    return Ok(ReasoningStep {
                        id: plan_step.order,
                        step_type: StepType::ToolUse,
                        thought,
                        tool_call: Some(ToolCallInfo {
                            tool_name: tool_name.clone(),
                            arguments: json!({"context": context}),
                            status: if result.is_ok() { ToolCallStatus::Success } else { ToolCallStatus::Failed },
                        }),
                        observation: result.ok(),
                        conclusion: Some("几何数据处理完成".to_string()),
                    });
                }
            }
        }

        // 不需要工具的步骤（LLM 直接分析）
        Ok(ReasoningStep {
            id: plan_step.order,
            step_type: StepType::Analyze,
            thought,
            tool_call: None,
            observation: None,
            conclusion: Some("分析完成".to_string()),
        })
    }

    /// 调用几何处理流水线（使用 analysis 模块）
    fn call_geometry_pipeline(&self, context: &serde_json::Value) -> Result<serde_json::Value, serde_json::Value> {
        // 如果分析管线未初始化，返回模拟数据
        let pipeline = match &self.analysis_pipeline {
            Some(p) => p,
            None => {
                // Mock 模式：返回模拟数据
                return Ok(json!({
                    "primitives_count": 10,
                    "constraints_count": 5,
                    "topology_nodes": 8,
                    "topology_edges": 12,
                    "mock_mode": true
                }));
            }
        };

        // 从上下文中提取 SVG 数据
        let svg_data = context["svg_data"].as_str().unwrap_or("");
        let instruction = context["instruction"].as_str().unwrap_or("分析这个图形");

        // 调用分析管线
        match pipeline.inject_from_svg_string(svg_data, instruction) {
            Ok(response) => Ok(json!({
                "primitives_count": response.primitives.len(),
                "constraints_count": response.relations.len(),
                "prompt_length": response.prompt.full_prompt.len(),
                "full_response": {
                    "primitives": response.primitives.len(),
                    "relations": response.relations.len(),
                }
            })),
            Err(e) => Err(json!({"error": e.to_string()})),
        }
    }

    /// 步骤 4: 生成结论
    fn generate_conclusion(&self, steps: &[ReasoningStep], request: &LlmReasoningRequest) -> Result<(String, f64), ReasoningError> {
        // 实际项目中：调用 LLM API 生成结论
        // 这里根据任务类型和推理步骤生成结论
        
        // 查找工具调用结果
        let tool_observation = steps.iter()
            .find(|s| s.step_type == StepType::ToolUse)
            .and_then(|s| s.observation.as_ref());

        let (answer, confidence) = match request.task_type {
            ReasoningTask::CountRooms => {
                if let Some(obs) = tool_observation {
                    let count = obs["primitives_count"].as_u64().unwrap_or(0);
                    // 简化：假设基元数量与房间数量有关
                    let room_count = if count > 5 { count / 2 } else { 1 };
                    (format!("共有{}个房间", room_count), 0.85)
                } else {
                    ("无法确定房间数量".to_string(), 0.3)
                }
            }
            ReasoningTask::CalculateArea => {
                if let Some(obs) = tool_observation {
                    let count = obs["primitives_count"].as_u64().unwrap_or(0);
                    (format!("总面积约为{}平方单位", count * 10), 0.8)
                } else {
                    ("无法计算面积".to_string(), 0.3)
                }
            }
            _ => {
                ("分析完成".to_string(), 0.7)
            }
        };

        Ok((answer, confidence))
    }
}

impl Default for LlmReasoningEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 辅助方法 ====================

impl ReasoningTask {
    /// 获取任务类型的字符串表示
    pub fn task_type_str(&self) -> &'static str {
        match self {
            ReasoningTask::CountRooms => "房间计数",
            ReasoningTask::CalculateArea => "面积计算",
            ReasoningTask::MeasureDimension => "尺寸测量",
            ReasoningTask::DetectDoorsWindows => "门窗检测",
            ReasoningTask::AnalyzeLayout => "户型分析",
            ReasoningTask::Custom => "自定义任务",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_engine_count_rooms() {
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

        let response = engine.reason(request);
        assert!(response.is_ok());
        
        let response = response.unwrap();
        assert_eq!(response.chain_of_thought.state, ReasoningState::Completed);
        assert!(!response.chain_of_thought.steps.is_empty());
    }

    #[test]
    fn test_reasoning_engine_calculate_area() {
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

        let response = engine.reason(request);
        assert!(response.is_ok());
        
        let response = response.unwrap();
        assert!(response.chain_of_thought.confidence > 0.0);
        assert!(!response.tools_used.is_empty());
    }

    #[test]
    fn test_chain_of_thought_structure() {
        let engine = LlmReasoningEngine::new();
        let request = LlmReasoningRequest {
            task: "测试任务".to_string(),
            task_type: ReasoningTask::Custom,
            context: json!({}),
            verbose: false,
        };

        let response = engine.reason(request).unwrap();
        let cot = &response.chain_of_thought;

        // 验证思维链结构
        assert!(cot.steps.len() >= 3); // 至少包含理解、规划、结论
        
        // 验证步骤类型顺序
        assert_eq!(cot.steps[0].step_type, StepType::Understand);
        assert_eq!(cot.steps[1].step_type, StepType::Plan);
        assert_eq!(cot.steps[cot.steps.len() - 1].step_type, StepType::Conclude);
    }
}
