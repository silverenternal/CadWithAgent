//! 分析管线数据类型
//!
//! 定义统一的几何分析管线使用的数据结构

use crate::geometry::primitives::Primitive;
use crate::cad_reasoning::GeometricRelation;
use crate::cad_verifier::VerificationResult;
use crate::prompt_builder::StructuredPrompt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// VLM 响应信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmResponseInfo {
    /// 模型返回的内容
    pub content: String,
    /// 使用的模型名称
    pub model: String,
    /// Token 使用统计
    pub usage: Option<TokenUsageInfo>,
    /// 延迟（毫秒）
    pub latency_ms: u64,
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageInfo {
    /// 提示词 token 数
    pub prompt_tokens: u32,
    /// 完成 token 数
    pub completion_tokens: u32,
    /// 总 token 数
    pub total_tokens: u32,
}

/// 工具调用链中的单个步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStep {
    /// 步骤序号
    pub step: usize,
    /// 工具名称
    pub tool_name: String,
    /// 工具描述
    pub description: String,
    /// 执行耗时（毫秒）
    pub latency_ms: u64,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 输出统计信息
    pub output_stats: HashMap<String, serde_json::Value>,
}

/// 工具调用链追踪
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallChain {
    /// 调用步骤列表
    pub steps: Vec<ToolCallStep>,
    /// 总耗时（毫秒）
    pub total_latency_ms: u64,
    /// 是否全部成功
    pub all_success: bool,
}

impl ToolCallChain {
    /// 创建新的调用链
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加步骤
    pub fn add_step(&mut self, step: ToolCallStep) {
        self.all_success = self.all_success && step.success;
        self.total_latency_ms += step.latency_ms;
        self.steps.push(step);
    }

    /// 转换为 JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "steps": self.steps,
            "total_latency_ms": self.total_latency_ms,
            "all_success": self.all_success
        })
    }
}

/// 分析管线配置
///
/// 统一配置各子模块参数，减少重复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// 是否启用坐标归一化
    pub enable_normalization: bool,
    /// 归一化范围 [min, max]
    pub normalize_range: [f64; 2],
    /// 角度容差（弧度）
    pub angle_tolerance: f64,
    /// 距离容差
    pub distance_tolerance: f64,
    /// 最小置信度阈值
    pub min_confidence: f64,
    /// 是否跳过校验步骤
    pub skip_verification: bool,
    /// 是否包含详细日志
    pub verbose: bool,
    /// 最大基元显示数量
    pub max_primitives_display: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            enable_normalization: true,
            normalize_range: [0.0, 100.0],
            angle_tolerance: 0.01, // ~0.57 度
            distance_tolerance: 0.01,
            min_confidence: 0.8,
            skip_verification: false,
            verbose: false,
            max_primitives_display: 50,
        }
    }
}

impl AnalysisConfig {
    /// 验证配置参数的合理性
    pub fn validate(&self) -> crate::error::CadAgentResult<()> {
        use crate::error::CadAgentError;

        // 验证归一化范围
        if self.normalize_range[0] >= self.normalize_range[1] {
            return Err(CadAgentError::Config(format!(
                "归一化范围无效：[{}, {}]。最小值必须小于最大值。建议值：[0.0, 100.0]",
                self.normalize_range[0],
                self.normalize_range[1]
            )));
        }

        // 验证角度容差
        if self.angle_tolerance <= 0.0 {
            return Err(CadAgentError::Config(format!(
                "角度容差必须为正数，当前值：{}。建议值：0.01（约 0.57 度）",
                self.angle_tolerance
            )));
        }
        if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            return Err(CadAgentError::Config(format!(
                "角度容差过大（{} 弧度 ≈ {:.2} 度），最大允许 90 度（π/2）。建议值：0.01",
                self.angle_tolerance,
                self.angle_tolerance.to_degrees()
            )));
        }

        // 验证距离容差
        if self.distance_tolerance < 0.0 {
            return Err(CadAgentError::Config(format!(
                "距离容差必须为非负数，当前值：{}",
                self.distance_tolerance
            )));
        }

        // 验证置信度
        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            return Err(CadAgentError::Config(format!(
                "最小置信度必须在 0 到 1 之间，当前值：{}",
                self.min_confidence
            )));
        }

        Ok(())
    }

    /// 验证并自动修正不合理的配置
    pub fn validate_or_fix(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let default = AnalysisConfig::default();

        if self.normalize_range[0] >= self.normalize_range[1] {
            warnings.push(format!(
                "归一化范围 [{}, {}] 无效，已修正为默认值 [{}, {}]",
                self.normalize_range[0],
                self.normalize_range[1],
                default.normalize_range[0],
                default.normalize_range[1]
            ));
            self.normalize_range = default.normalize_range;
        }

        if self.angle_tolerance <= 0.0 || self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            warnings.push(format!(
                "角度容差 {} 无效，已修正为默认值 {}",
                self.angle_tolerance,
                default.angle_tolerance
            ));
            self.angle_tolerance = default.angle_tolerance;
        }

        if self.distance_tolerance < 0.0 {
            warnings.push(format!(
                "距离容差 {} 无效，已修正为默认值 {}",
                self.distance_tolerance,
                default.distance_tolerance
            ));
            self.distance_tolerance = default.distance_tolerance;
        }

        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            warnings.push(format!(
                "最小置信度 {} 无效，已修正为默认值 {}",
                self.min_confidence,
                default.min_confidence
            ));
            self.min_confidence = default.min_confidence;
        }

        warnings
    }
}

/// 分析管线结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// 提取的基元
    pub primitives: Vec<Primitive>,
    /// 推理的约束关系
    pub relations: Vec<GeometricRelation>,
    /// 校验结果（如果启用）
    pub verification: Option<VerificationResult>,
    /// 生成的结构化提示词
    pub prompt: StructuredPrompt,
    /// 管线执行日志
    pub execution_log: Vec<String>,
    /// 总耗时（毫秒）
    pub total_latency_ms: u64,
    /// VLM 推理结果（如果执行了 VLM 推理）
    pub vlm_response: Option<VlmResponseInfo>,
    /// 工具调用链追踪（用于可解释性和实验分析）
    pub tool_call_chain: Option<ToolCallChain>,
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisResult {
    /// 创建新的分析结果
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            relations: Vec::new(),
            verification: None,
            prompt: StructuredPrompt {
                full_prompt: String::new(),
                system_prompt: String::new(),
                user_prompt: String::new(),
                metadata: crate::prompt_builder::PromptMetadata {
                    primitive_count: 0,
                    constraint_count: 0,
                    prompt_length: 0,
                    template: crate::prompt_builder::PromptTemplate::Analysis,
                    injected_context: Vec::new(),
                },
            },
            execution_log: Vec::new(),
            total_latency_ms: 0,
            vlm_response: None,
            tool_call_chain: None,
        }
    }

    /// 获取基元数量
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// 获取关系数量
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    /// 获取工具调用链的 JSON 表示
    pub fn tool_chain_json(&self) -> serde_json::Value {
        self.tool_call_chain
            .as_ref()
            .map(|c| c.to_json())
            .unwrap_or_else(|| serde_json::json!(null))
    }
}
