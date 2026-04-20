//! LLM 推理核心数据类型
//!
//! 定义大模型驱动的思维链数据结构，支持：
//! - **动态推理步骤**: LLM 根据任务动态生成推理流程
//! - **工具调用**: 集成几何处理工具获取结构化数据
//! - **不确定性处理**: 显式建模和跟踪推理中的不确定性
//! - **可解释性**: 完整的思维链记录，支持审计和调试
//!
//! # 核心数据结构
//!
//! | 结构体 | 用途 |
//! |--------|------|
//! | `ReasoningTask` | 推理任务类型（房间计数、面积计算等） |
//! | `ReasoningStep` | 单个推理步骤（理解、规划、工具调用等） |
//! | `ChainOfThought` | 完整的思维链（步骤序列 + 结论） |
//! | `LlmReasoningRequest/Response` | 推理请求/响应 |
//! | `ReasoningPlan` | LLM 生成的执行计划 |
//! | `Uncertainty` | 不确定性建模 |

use serde::{Deserialize, Serialize};

/// 推理任务类型
///
/// 定义了 LLM 推理引擎支持的任务类型，每种类型对应特定的
/// 几何分析目标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningTask {
    /// 房间数量统计
    CountRooms,
    /// 面积计算
    CalculateArea,
    /// 尺寸测量
    MeasureDimension,
    /// 门窗检测
    DetectDoorsWindows,
    /// 户型分析
    AnalyzeLayout,
    /// 自定义任务
    Custom,
}

/// 推理状态
///
/// 表示推理过程当前的执行状态，用于跟踪进度和处理异步操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningState {
    /// 等待开始
    #[default]
    Pending,
    /// 思考中
    Thinking,
    /// 等待工具调用结果
    WaitingForTool,
    /// 回溯修正中
    Revising,
    /// 已完成
    Completed,
    /// 失败
    Failed,
}

/// 推理步骤（由 LLM 动态生成）
///
/// 每个步骤代表推理过程中的一个逻辑单元，包含思考内容、
/// 工具调用（如有）和观察结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// 步骤 ID
    pub id: usize,
    /// 步骤类型
    pub step_type: StepType,
    /// 思考内容（LLM 生成）
    pub thought: String,
    /// 调用的工具（如果有）
    pub tool_call: Option<ToolCallInfo>,
    /// 观察结果（工具返回或 LLM 观察）
    pub observation: Option<serde_json::Value>,
    /// 结论（可选，中间结论）
    pub conclusion: Option<String>,
}

/// 步骤类型
///
/// 定义推理步骤的类型，反映思维链中的不同认知活动。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// 理解任务
    Understand,
    /// 规划步骤
    Plan,
    /// 调用工具
    ToolUse,
    /// 分析结果
    Analyze,
    /// 验证结论
    Verify,
    /// 回溯修正
    Revise,
    /// 生成结论
    Conclude,
}

/// 工具调用信息
///
/// 记录推理过程中对几何处理工具的调用详情。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// 工具名称
    pub tool_name: String,
    /// 工具参数
    pub arguments: serde_json::Value,
    /// 调用状态
    pub status: ToolCallStatus,
}

/// 工具调用状态
///
/// 表示工具调用的执行结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    /// 等待执行
    Pending,
    /// 成功
    Success,
    /// 失败
    Failed,
}

/// 思维链（LLM 生成的完整推理过程）
///
/// 包含从任务理解到最终结论的完整推理轨迹，支持可解释的 AI 决策。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfThought {
    /// 任务描述
    pub task: String,
    /// 任务类型
    pub task_type: ReasoningTask,
    /// 推理步骤列表（动态生成）
    pub steps: Vec<ReasoningStep>,
    /// 最终答案
    pub answer: String,
    /// 置信度
    pub confidence: f64,
    /// 推理状态
    pub state: ReasoningState,
}

/// LLM 推理请求
///
/// 发送给推理引擎的任务请求，包含任务描述和上下文数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmReasoningRequest {
    /// 任务描述
    pub task: String,
    /// 任务类型
    pub task_type: ReasoningTask,
    /// 上下文数据（几何信息等）
    pub context: serde_json::Value,
    /// 是否需要详细推理过程
    pub verbose: bool,
}

/// LLM 推理响应
///
/// 推理引擎返回的完整结果，包含思维链和统计信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmReasoningResponse {
    /// 思维链
    pub chain_of_thought: ChainOfThought,
    /// 使用的工具列表
    pub tools_used: Vec<String>,
    /// 推理耗时（毫秒）
    pub latency_ms: u64,
}

/// 推理规划（LLM 生成的执行计划）
///
/// 在正式推理前，LLM 会生成一个执行计划，规划需要的步骤和工具。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPlan {
    /// 任务描述
    pub task: String,
    /// 计划步骤（按顺序执行）
    pub plan_steps: Vec<PlanStep>,
    /// 预期需要的工具
    pub required_tools: Vec<String>,
}

/// 计划步骤
///
/// 推理计划中的单个步骤，定义执行顺序和依赖关系。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// 步骤序号
    pub order: usize,
    /// 步骤描述
    pub description: String,
    /// 是否需要调用工具
    pub needs_tool: bool,
    /// 工具名称（如果需要）
    pub tool_name: Option<String>,
    /// 前置步骤（依赖）
    pub dependencies: Vec<usize>,
}

/// 推理中的假设
///
/// 在不确定性推理中，LLM 会生成多个假设并进行验证。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    /// 假设描述
    pub description: String,
    /// 置信度
    pub confidence: f64,
    /// 支持证据
    pub supporting_evidence: Vec<String>,
    /// 反面证据
    pub contradicting_evidence: Vec<String>,
    /// 是否已验证
    pub verified: bool,
    /// 验证结果
    pub verification_result: Option<bool>,
}

/// 推理中的不确定性
///
/// 显式建模推理过程中的不确定性来源和影响程度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uncertainty {
    /// 不确定性描述
    pub description: String,
    /// 不确定性来源
    pub source: UncertaintySource,
    /// 影响程度
    pub impact: ImpactLevel,
    /// 处理策略
    pub resolution_strategy: Option<String>,
}

/// 不确定性来源
///
/// 识别不确定性的具体来源，帮助选择适当的处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintySource {
    /// 图纸质量差
    PoorImageQuality,
    /// 标注模糊
    AmbiguousLabel,
    /// 基元识别不确定
    PrimitiveUncertainty,
    /// 约束冲突
    ConstraintConflict,
    /// 其他
    Other,
}

/// 影响程度
///
/// 评估不确定性对最终结论的影响程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactLevel {
    /// 低影响
    Low,
    /// 中等影响
    Medium,
    /// 高影响
    High,
    /// 关键影响
    Critical,
}
