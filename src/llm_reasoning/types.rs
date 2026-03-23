//! LLM 推理核心数据类型
//!
//! 定义大模型驱动的思维链数据结构
//!
//! # 定位说明
//!
//! 这是**真正的思维链模块**，由 LLM 驱动推理过程：
//! - 动态生成推理步骤
//! - 支持回溯和条件分支
//! - 处理不确定性和多义性
//! - 生成可解释的推理过程

use serde::{Deserialize, Serialize};

/// 推理任务类型
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningState {
    #[default]
    Pending,
    Thinking,
    WaitingForTool,
    Revising,
    Completed,
    Failed,
}

/// 推理步骤（由 LLM 动态生成）
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Success,
    Failed,
}

/// 思维链（LLM 生成的完整推理过程）
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}
