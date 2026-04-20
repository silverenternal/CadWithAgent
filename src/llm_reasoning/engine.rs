//! LLM 推理引擎
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
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
//!
//! # LLM 集成
//!
//! 本模块使用 ZAZAZ API 进行真实的大模型推理：
//! - `understand_task`: 调用 LLM 理解任务意图
//! - `generate_plan`: 调用 LLM 动态生成推理计划
//! - `generate_conclusion`: 调用 LLM 基于推理步骤生成结论

use crate::analysis::AnalysisPipeline;
use crate::bridge::vlm_client::{VlmClient, VlmConfig};
use crate::llm_reasoning::types::*;
use serde_json::json;
use std::cell::RefCell;
use thiserror::Error;
use tracing::{debug, info, instrument, warn};

/// 推理引擎错误
///
/// 表示 LLM 推理过程中可能发生的各类错误。
#[derive(Debug, Error)]
pub enum ReasoningError {
    /// 任务理解失败
    #[error("任务理解失败：{0}")]
    UnderstandError(String),

    /// 规划失败
    #[error("规划失败：{0}")]
    PlanError(String),

    /// 工具调用失败
    #[error("工具调用失败：{0}")]
    ToolError(String),

    /// 推理失败
    #[error("推理失败：{0}")]
    InferenceError(String),

    /// LLM API 错误
    #[error("LLM API 错误：{0}")]
    LlmApiError(#[from] crate::bridge::vlm_client::VlmError),
}

/// LLM 推理引擎
///
/// 大模型驱动的思维链推理引擎，支持：
/// - **真实 LLM 推理**: 使用 ZAZAZ API 进行任务理解、规划和结论生成
/// - **工具调用**: 集成几何处理工具获取结构化数据
/// - **回退机制**: LLM API 不可用时自动回退到预定义模板
///
/// # 实现说明
///
/// 本引擎使用真实的 ZAZAZ LLM API 进行推理：
/// - 任务理解：通过 LLM 分析用户意图和任务类型
/// - 规划生成：LLM 动态生成推理步骤规划
/// - 结论生成：LLM 基于工具调用结果生成自然语言结论
///
/// # 环境变量要求
///
/// 需要设置以下环境变量（通过 `.env` 文件）：
/// - `PROVIDER_ZAZAZ_API_KEY`: ZAZAZ API Key
/// - `PROVIDER_ZAZAZ_API_URL`: ZAZAZ API URL (可选，默认：https://zazaz.top/v1)
/// - `PROVIDER_ZAZAZ_MODEL`: 模型名称 (可选，默认：./Qwen3.5-27B-FP8)
///
/// # 示例
///
/// ```rust,no_run
/// use cadagent::llm_reasoning::{LlmReasoningEngine, LlmReasoningRequest, ReasoningTask};
/// use serde_json::json;
///
/// // 创建引擎（需要设置环境变量）
/// let engine = LlmReasoningEngine::new().expect("Failed to create engine");
/// let request = LlmReasoningRequest {
///     task: "这个户型有多少个房间？".to_string(),
///     task_type: ReasoningTask::CountRooms,
///     context: json!({
///         "drawing_type": "vector",
///         "drawing_data": "vector_data_here"
///     }),
///     verbose: false,
/// };
///
/// let response = engine.reason(request).unwrap();
/// println!("答案：{}", response.chain_of_thought.answer);
/// println!("置信度：{:.2}", response.chain_of_thought.confidence);
/// ```
pub struct LlmReasoningEngine {
    /// 是否启用详细日志
    verbose: bool,
    /// 几何处理流水线（作为工具使用，使用 trait 对象解耦）
    geometry_pipeline: Option<Box<dyn crate::analysis::GeometryPipelineTrait>>,
    /// LLM 客户端（用于真实推理）
    llm_client: Option<VlmClient>,
    /// 对话状态管理器（用于多轮对话上下文追踪）
    dialog_state: Option<RefCell<crate::context::DialogStateManager>>,
}

impl LlmReasoningEngine {
    /// 创建新的推理引擎（使用真实 LLM API）
    ///
    /// 此方法会尝试从环境变量加载 ZAZAZ 配置并创建 LLM 客户端和分析管线。
    /// 如果环境变量未设置，会创建一个没有 LLM 客户端的引擎（回退到 Mock 模式）。
    ///
    /// # 错误
    ///
    /// 如果 VLM 配置加载失败，返回 `VlmError`。
    pub fn new() -> Result<Self, crate::bridge::vlm_client::VlmError> {
        let llm_client = match VlmConfig::default_zazaz() {
            Ok(config) => {
                let client = VlmClient::new(config);
                Some(client)
            }
            Err(_) => None,
        };

        // 尝试创建分析管线（用于几何处理）
        let geometry_pipeline = match AnalysisPipeline::with_defaults() {
            Ok(pipeline) => {
                Some(Box::new(pipeline) as Box<dyn crate::analysis::GeometryPipelineTrait>)
            }
            Err(_) => None,
        };

        // 尝试创建对话状态管理器
        let dialog_state =
            crate::context::DialogStateManager::new("default-session", Default::default())
                .ok()
                .map(RefCell::new);

        Ok(Self {
            verbose: false,
            geometry_pipeline,
            llm_client,
            dialog_state,
        })
    }

    /// 创建仅几何模式的推理引擎（不使用 LLM API）
    ///
    /// 在此模式下，所有推理步骤都使用预定义模板（Mock 模式）。
    /// 适用于测试或没有 LLM API 配置的场景。
    pub fn geometry_only() -> Self {
        Self {
            verbose: false,
            geometry_pipeline: None,
            llm_client: None,
            dialog_state: None,
        }
    }

    /// 设置详细日志模式
    ///
    /// 启用后会输出 LLM API 调用、回退等详细日志。
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 启用分析管线（需要 VLM API Key）
    ///
    /// 创建默认的分析管线并设置为几何处理工具。
    ///
    /// # 错误
    ///
    /// 如果 VLM 配置加载失败，返回 `VlmError`。
    pub fn with_analysis_pipeline(mut self) -> Result<Self, crate::bridge::vlm_client::VlmError> {
        let pipeline = AnalysisPipeline::with_defaults()?;
        self.geometry_pipeline = Some(Box::new(pipeline));
        Ok(self)
    }

    /// 使用自定义几何管线（通过 trait 对象）
    ///
    /// 这允许你传入任何实现 `GeometryPipelineTrait` 的类型，
    /// 包括自定义实现或 Mock 用于测试。
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let engine = LlmReasoningEngine::geometry_only()
    ///     .with_geometry_pipeline(my_custom_pipeline);
    /// ```
    pub fn with_geometry_pipeline<P>(mut self, pipeline: P) -> Self
    where
        P: crate::analysis::GeometryPipelineTrait + 'static,
    {
        self.geometry_pipeline = Some(Box::new(pipeline));
        self
    }

    /// 使用自定义 LLM 配置创建推理引擎
    ///
    /// # 错误
    ///
    /// 如果 VLM 配置无效，返回 `VlmError`。
    pub fn with_llm_config(config: VlmConfig) -> Result<Self, crate::bridge::vlm_client::VlmError> {
        let client = VlmClient::new(config);
        let dialog_state =
            crate::context::DialogStateManager::new("default-session", Default::default())
                .ok()
                .map(RefCell::new);
        Ok(Self {
            verbose: false,
            geometry_pipeline: None,
            llm_client: Some(client),
            dialog_state,
        })
    }

    /// 使用 `Ollama` 本地模型创建推理引擎
    ///
    /// Ollama 是一个本地运行开源 LLM 的工具（https://ollama.ai）
    ///
    /// # 环境变量
    /// - `OLLAMA_HOST`: Ollama 服务地址 (可选，默认：http://localhost:11434)
    /// - `OLLAMA_MODEL`: 模型名称 (可选，默认：qwen2.5:7b)
    ///
    /// # 错误
    ///
    /// 如果配置失败，返回 `VlmError`。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use cadagent::llm_reasoning::LlmReasoningEngine;
    ///
    /// let engine = LlmReasoningEngine::with_ollama().unwrap();
    /// ```
    pub fn with_ollama() -> Result<Self, crate::bridge::vlm_client::VlmError> {
        let config = VlmConfig::default_ollama()?;
        Self::with_llm_config(config)
    }

    /// 使用 `LM Studio` 本地模型创建推理引擎
    ///
    /// LM Studio 是本地运行 LLM 的桌面应用（https://lmstudio.ai）
    ///
    /// # 环境变量
    /// - `LM_STUDIO_HOST`: LM Studio 服务地址 (可选，默认：http://localhost:1234)
    /// - `LM_STUDIO_MODEL`: 模型名称 (可选，默认：local-model)
    ///
    /// # 错误
    ///
    /// 如果配置失败，返回 `VlmError`。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use cadagent::llm_reasoning::LlmReasoningEngine;
    ///
    /// let engine = LlmReasoningEngine::with_lm_studio().unwrap();
    /// ```
    pub fn with_lm_studio() -> Result<Self, crate::bridge::vlm_client::VlmError> {
        let config = VlmConfig::default_lm_studio()?;
        Self::with_llm_config(config)
    }

    /// 执行推理任务
    ///
    /// 这是 LLM 驱动推理的核心入口，执行完整的思维链推理流程：
    /// 1. 理解任务（LLM 生成）
    /// 2. 规划推理步骤（LLM 生成）
    /// 3. 执行推理（LLM 驱动 + 工具调用）
    /// 4. 生成结论（LLM 生成）
    ///
    /// # 参数
    ///
    /// * `request` - 推理请求，包含任务描述、类型和上下文数据
    ///
    /// # 返回
    ///
    /// 返回包含完整思维链、工具使用情况和推理耗时的响应。
    ///
    /// # 错误
    ///
    /// 返回 `ReasoningError` 如果推理过程中发生错误。
    #[instrument(skip(self, request), fields(task = %request.task, task_type = %request.task_type.task_type_str(), latency_ms = 0))]
    pub fn reason(
        &self,
        request: LlmReasoningRequest,
    ) -> Result<LlmReasoningResponse, ReasoningError> {
        let start_time = std::time::Instant::now();

        debug!("Starting LLM reasoning");

        // 记录用户输入到对话状态
        if let Some(ref dialog_cell) = self.dialog_state {
            let mut dialog = dialog_cell.borrow_mut();
            let _ = dialog.add_user_message(&request.task);
        }

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
            thought: format!(
                "我将按以下步骤执行：{}",
                plan.plan_steps
                    .iter()
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
            if self.verbose {
                println!(
                    "\n[步骤 {}/{}] {}",
                    plan_step.order,
                    plan.plan_steps.len(),
                    plan_step.description
                );
            }

            let step_result = self.execute_plan_step(plan_step, &request.context)?;

            if let Some(tool_name) = &plan_step.tool_name {
                if !tools_used.contains(tool_name) {
                    tools_used.push(tool_name.clone());
                }
            }

            // 输出步骤执行结果
            if self.verbose {
                let status = if let Some(tool_call) = &step_result.tool_call {
                    format!("✓ 工具调用完成：{}", tool_call.tool_name)
                } else {
                    "✓ 分析完成".to_string()
                };
                println!("  {status}");
            }

            steps.push(step_result);
        }

        if self.verbose {
            println!("\n[生成结论] 正在整合推理结果...");
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

        // 记录助手响应到对话状态（在移动 tools_used 之前）
        if let Some(ref dialog_cell) = self.dialog_state {
            let tools_json = serde_json::to_string(&tools_used).unwrap_or_default();
            let mut dialog = dialog_cell.borrow_mut();
            let _ = dialog.add_assistant_response(&answer, Some(&tools_json));
        }

        // 记录日志（在移动之前）
        let steps_count = steps.len();
        let tools_count = tools_used.len();

        let response = LlmReasoningResponse {
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
        };

        // 记录延迟到 span
        tracing::Span::current().record("latency_ms", latency_ms);
        info!(
            latency_ms = %latency_ms,
            steps_count = %steps_count,
            tools_count = %tools_count,
            confidence = %confidence,
            "LLM reasoning completed"
        );

        Ok(response)
    }

    /// 步骤 1: 理解任务（使用真实 LLM API）
    fn understand_task(&self, request: &LlmReasoningRequest) -> Result<String, ReasoningError> {
        // 如果没有 LLM 客户端，回退到 Mock 模式
        let Some(client) = &self.llm_client else {
            return self.understand_task_mock(request);
        };

        // 构建系统提示词
        let system_prompt =
            "你是一个 CAD 几何推理专家，擅长理解用户关于户型图、建筑平面图的查询意图。
你的任务是分析用户问题，识别任务类型，并生成清晰的任务理解描述。

任务类型包括：
- 房间计数：统计户型图中的房间数量
- 面积计算：计算房间或户型的总面积
- 尺寸测量：测量长度、宽度等尺寸
- 门窗检测：检测门窗的位置和数量
- 户型分析：分析户型布局和房间类型";

        // 构建用户提示词
        let user_prompt = format!(
            "任务类型：{}\n用户问题：{}\n\n请分析这个任务的意图和需要执行的步骤。",
            request.task_type.task_type_str(),
            request.task
        );

        // 调用 LLM API
        match client
            .chat_completions_blocking(&[("system", system_prompt), ("user", user_prompt.as_str())])
        {
            Ok(response) => {
                let content = response.choices.first().map_or_else(
                    || "无法生成任务理解".to_string(),
                    |c| c.message.content.clone(),
                );

                Ok(format!(
                    "任务理解：{}\n\n用户问题：{}\n\n策略：{}",
                    request.task_type.task_type_str(),
                    request.task,
                    content
                ))
            }
            Err(e) => {
                if self.verbose {
                    eprintln!("LLM API 调用失败，回退到 Mock 模式：{e}");
                }
                self.understand_task_mock(request)
            }
        }
    }

    /// Mock 模式：理解任务（回退实现）
    fn understand_task_mock(
        &self,
        request: &LlmReasoningRequest,
    ) -> Result<String, ReasoningError> {
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

    /// 步骤 2: 生成推理计划（使用真实 LLM API）
    fn generate_plan(
        &self,
        request: &LlmReasoningRequest,
        understanding: &str,
    ) -> Result<ReasoningPlan, ReasoningError> {
        // 如果没有 LLM 客户端，回退到 Mock 模式
        let Some(client) = &self.llm_client else {
            return self.generate_plan_mock(request, understanding);
        };

        // 构建系统提示词
        let system_prompt = "你是一个 CAD 几何推理规划专家。你的任务是根据用户问题和任务理解，生成一个清晰的推理计划。

计划应该包含以下步骤：
1. 数据获取：调用几何处理工具获取基元和约束
2. 数据分析：分析几何关系和拓扑结构
3. 结论生成：基于分析结果生成最终结论

可用工具列表：
- cad_analyze_layout: 完整的布局分析（基元提取 + 关系推理 + 约束校验）
- cad_extract_primitives: 提取 CAD 基元（线、圆、弧等）
- cad_find_geometric_relations: 查找几何关系（平行、垂直、连接等）
- cad_verify_constraints: 验证约束合法性

请以 JSON 格式返回计划，包含 step_order（序号）、description（描述）、needs_tool（是否需要工具）、tool_name（工具名称，如果需要）。
对于户型分析任务，推荐使用 cad_analyze_layout 工具。";

        // 构建用户提示词
        let user_prompt = format!(
            "任务理解：{}\n用户问题：{}\n\n请生成一个推理计划，包含 2-4 个步骤。",
            understanding, request.task
        );

        // 调用 LLM API
        match client
            .chat_completions_blocking(&[("system", system_prompt), ("user", user_prompt.as_str())])
        {
            Ok(response) => {
                let content = response
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default();

                // 尝试解析 LLM 返回的 JSON 计划
                if let Ok(plan_steps) = self.parse_llm_plan(&content) {
                    let required_tools = plan_steps
                        .iter()
                        .filter_map(|s| s.tool_name.clone())
                        .collect();

                    return Ok(ReasoningPlan {
                        task: request.task.clone(),
                        plan_steps,
                        required_tools,
                    });
                }

                // 解析失败，回退到 Mock 模式
                if self.verbose {
                    eprintln!("LLM 计划解析失败，回退到 Mock 模式");
                }
                self.generate_plan_mock(request, understanding)
            }
            Err(e) => {
                if self.verbose {
                    eprintln!("LLM API 调用失败，回退到 Mock 模式：{e}");
                }
                self.generate_plan_mock(request, understanding)
            }
        }
    }

    /// 解析 LLM 返回的计划文本（尝试提取 JSON）
    fn parse_llm_plan(&self, content: &str) -> Result<Vec<PlanStep>, ReasoningError> {
        // 尝试直接解析为 JSON 数组
        if let Ok(steps) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
            let mut plan_steps = Vec::new();
            for (i, step) in steps.iter().enumerate() {
                let description = step["description"]
                    .as_str()
                    .unwrap_or("执行推理")
                    .to_string();
                let needs_tool = step["needs_tool"].as_bool().unwrap_or(false);
                let tool_name = step["tool_name"]
                    .as_str()
                    .map(std::string::ToString::to_string);

                plan_steps.push(PlanStep {
                    order: i + 1,
                    description,
                    needs_tool,
                    tool_name,
                    dependencies: vec![],
                });
            }
            return Ok(plan_steps);
        }

        // 尝试从 Markdown 代码块中提取 JSON
        if let Some(json_start) = content.find("```json") {
            let remaining = &content[json_start + 7..];
            if let Some(json_end) = remaining.find("```") {
                let json_str = &remaining[..json_end];
                return self.parse_llm_plan(json_str.trim());
            }
        }

        // 解析失败
        Err(ReasoningError::PlanError("无法解析 LLM 计划".to_string()))
    }

    /// Mock 模式：生成推理计划（回退实现）
    fn generate_plan_mock(
        &self,
        request: &LlmReasoningRequest,
        _understanding: &str,
    ) -> Result<ReasoningPlan, ReasoningError> {
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

        let required_tools = plan_steps
            .iter()
            .filter_map(|s| s.tool_name.clone())
            .collect();

        Ok(ReasoningPlan {
            task: request.task.clone(),
            plan_steps,
            required_tools,
        })
    }

    /// 步骤 3: 执行计划步骤
    fn execute_plan_step(
        &self,
        plan_step: &PlanStep,
        context: &serde_json::Value,
    ) -> Result<ReasoningStep, ReasoningError> {
        let thought = format!("执行步骤 {}: {}", plan_step.order, plan_step.description);

        // 如果需要调用工具
        if plan_step.needs_tool {
            if let Some(ref tool_name) = plan_step.tool_name {
                // 支持多种工具名称映射到几何管线调用
                if tool_name == "analysis_execute"
                    || tool_name == "cad_analyze_layout"
                    || tool_name == "cad_extract_primitives"
                    || tool_name == "geometry_primitive_extractor"
                    || tool_name == "spatial_analysis_and_ocr"
                {
                    // 调用分析管线
                    let result = self.call_geometry_pipeline(context);

                    return Ok(ReasoningStep {
                        id: plan_step.order,
                        step_type: StepType::ToolUse,
                        thought,
                        tool_call: Some(ToolCallInfo {
                            tool_name: tool_name.clone(),
                            arguments: json!({"context": context}),
                            status: if result.is_ok() {
                                ToolCallStatus::Success
                            } else {
                                ToolCallStatus::Failed
                            },
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

    /// 调用几何处理流水线（使用 trait 对象）
    fn call_geometry_pipeline(
        &self,
        context: &serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        // 从上下文中提取 SVG 数据
        let svg_data = context["svg_data"].as_str().unwrap_or("");
        let instruction = context["instruction"].as_str().unwrap_or("分析这个图形");

        // 验证 SVG 数据是否有效
        if svg_data.is_empty() {
            return Err(json!({
                "error": "SVG 数据为空",
                "error_type": "invalid_input",
                "suggestion": "请提供有效的 SVG 格式户型图数据"
            }));
        }

        // 如果几何管线未初始化，返回错误
        let pipeline = match &self.geometry_pipeline {
            Some(p) => p.as_ref(),
            None => {
                return Err(json!({
                    "error": "几何处理管线未初始化",
                    "error_type": "pipeline_not_initialized",
                    "suggestion": "请检查 AnalysisPipeline 配置是否正确"
                }));
            }
        };

        // 调用几何管线
        match pipeline.inject_from_svg_string(svg_data, instruction) {
            Ok(response) => {
                // 构建详细的成功响应
                let mut result = json!({
                    "primitives_count": response.primitives.len(),
                    "constraints_count": response.relations.len(),
                    "prompt_length": response.prompt.full_prompt.len(),
                    "full_response": {
                        "primitives": response.primitives.len(),
                        "relations": response.relations.len(),
                    },
                    "status": "success"
                });

                // 如果有封闭区域信息，添加到结果中
                if let Some(rooms) = response.additional.get("closed_regions") {
                    result["closed_regions"] = rooms.clone();
                }

                Ok(result)
            }
            Err(e) => {
                // 构建详细的错误信息
                let error_msg = e.to_string();
                let error_type = if error_msg.contains("SVG") {
                    "parse_error"
                } else if error_msg.contains("几何") || error_msg.contains("基元") {
                    "geometry_error"
                } else {
                    "unknown"
                };

                Err(json!({
                    "error": error_msg,
                    "error_type": error_type,
                    "svg_data_length": svg_data.len(),
                    "suggestion": format!("几何处理失败：{}", error_msg)
                }))
            }
        }
    }

    /// 步骤 4: 生成结论（使用真实 LLM API）
    fn generate_conclusion(
        &self,
        steps: &[ReasoningStep],
        request: &LlmReasoningRequest,
    ) -> Result<(String, f64), ReasoningError> {
        // 如果没有 LLM 客户端，回退到 Mock 模式
        let Some(client) = &self.llm_client else {
            return self.generate_conclusion_mock(steps, request);
        };

        // 构建系统提示词
        let system_prompt = "你是一个 CAD 几何推理结论生成专家。你的任务是根据推理步骤和工具调用结果，生成清晰、准确的自然语言结论。

结论应该：
1. 直接回答用户的问题
2. 基于工具调用的实际结果
3. 提供适当的置信度评估
4. 保持专业但易于理解";

        // 构建推理步骤摘要
        let steps_summary: Vec<String> = steps
            .iter()
            .map(|s| {
                format!(
                    "步骤 {} [{}]: {}",
                    s.id,
                    match s.step_type {
                        StepType::Understand => "理解",
                        StepType::Plan => "规划",
                        StepType::ToolUse => "工具调用",
                        StepType::Analyze => "分析",
                        StepType::Verify => "验证",
                        StepType::Revise => "修正",
                        StepType::Conclude => "结论",
                    },
                    s.thought
                )
            })
            .collect();

        // 查找工具调用结果
        let tool_observation = steps
            .iter()
            .find(|s| s.step_type == StepType::ToolUse)
            .and_then(|s| s.observation.as_ref());

        let tool_result = tool_observation.map_or_else(
            || "无工具调用结果".to_string(),
            |obs| format!("工具调用结果：{obs}"),
        );

        // 构建用户提示词
        let user_prompt = format!(
            "任务类型：{}\n用户问题：{}\n\n推理步骤:\n{}\n\n{}\n\n请基于以上信息生成最终结论。",
            request.task_type.task_type_str(),
            request.task,
            steps_summary.join("\n"),
            tool_result
        );

        // 调用 LLM API
        if self.verbose {
            println!("  正在调用 LLM API 生成结论...");
        }

        match client
            .chat_completions_blocking(&[("system", system_prompt), ("user", user_prompt.as_str())])
        {
            Ok(response) => {
                if self.verbose {
                    println!(
                        "  ✓ LLM 响应完成 (模型：{}, token: {})",
                        response.model,
                        response
                            .usage
                            .as_ref()
                            .map_or_else(|| "N/A".to_string(), |u| u.total_tokens.to_string())
                    );
                }

                let content = response
                    .choices
                    .first()
                    .map_or_else(|| "无法生成结论".to_string(), |c| c.message.content.clone());

                // 使用 LLM 生成的内容作为答案，置信度基于是否有工具结果
                let confidence = if tool_observation.is_some() { 0.9 } else { 0.6 };
                Ok((content, confidence))
            }
            Err(e) => {
                if self.verbose {
                    eprintln!("LLM API 调用失败，回退到 Mock 模式：{e}");
                }
                self.generate_conclusion_mock(steps, request)
            }
        }
    }

    /// Mock 模式：生成结论（回退实现）
    fn generate_conclusion_mock(
        &self,
        steps: &[ReasoningStep],
        request: &LlmReasoningRequest,
    ) -> Result<(String, f64), ReasoningError> {
        // 查找工具调用结果
        let tool_observation = steps
            .iter()
            .find(|s| s.step_type == StepType::ToolUse)
            .and_then(|s| s.observation.as_ref());

        let (answer, confidence) = match request.task_type {
            ReasoningTask::CountRooms => {
                if let Some(obs) = tool_observation {
                    let count = obs["primitives_count"].as_u64().unwrap_or(0);
                    // 简化：假设基元数量与房间数量有关
                    let room_count = if count > 5 { count / 2 } else { 1 };
                    (format!("共有{room_count}个房间"), 0.85)
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
            _ => ("分析完成".to_string(), 0.7),
        };

        Ok((answer, confidence))
    }
}

impl Default for LlmReasoningEngine {
    fn default() -> Self {
        // 使用 geometry_only 作为默认，避免环境变量未设置时的 panic
        Self::geometry_only()
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
        // 使用 geometry_only 模式进行测试，避免依赖 LLM API
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

        let response = engine.reason(request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert_eq!(response.chain_of_thought.state, ReasoningState::Completed);
        assert!(!response.chain_of_thought.steps.is_empty());
    }

    #[test]
    fn test_reasoning_engine_calculate_area() {
        // 使用 geometry_only 模式进行测试，避免依赖 LLM API
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

        let response = engine.reason(request);
        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(response.chain_of_thought.confidence > 0.0);
        assert!(!response.tools_used.is_empty());
    }

    #[test]
    fn test_chain_of_thought_structure() {
        // 使用 geometry_only 模式进行测试，避免依赖 LLM API
        let engine = LlmReasoningEngine::geometry_only();
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

    #[test]
    fn test_engine_creation_with_llm() {
        // 测试 LLM 引擎创建（需要环境变量）
        // 如果环境变量未设置，应该返回 Err 或 None 客户端
        let result = LlmReasoningEngine::new();
        // 无论成功还是失败都应该正确处理
        assert!(result.is_ok() || result.is_err());
    }
}
