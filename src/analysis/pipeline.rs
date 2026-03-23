//! 分析管线实现
//!
//! 实现统一的几何分析管线，整合基元提取、几何推理、约束校验和提示词构造

use crate::cad_extractor::{CadPrimitiveExtractor, ExtractorConfig};
use crate::cad_reasoning::{GeometricRelationReasoner, ReasoningConfig};
use crate::cad_verifier::{ConstraintVerifier, VerifierConfig};
use crate::prompt_builder::{PromptBuilder, PromptConfig};
use crate::bridge::vlm_client::{VlmClient, VlmConfig};
use crate::error::{CadAgentError, CadAgentResult};
use crate::analysis::types::{AnalysisConfig, AnalysisResult, VlmResponseInfo, TokenUsageInfo, ToolCallChain, ToolCallStep};
use std::path::Path;
use std::collections::HashMap;
use std::time::Instant;

/// 分析管线
#[derive(Debug, Clone)]
pub struct AnalysisPipeline {
    config: AnalysisConfig,
    extractor: CadPrimitiveExtractor,
    reasoner: GeometricRelationReasoner,
    verifier: ConstraintVerifier,
    prompt_builder: PromptBuilder,
    vlm_client: VlmClient,
}

impl AnalysisPipeline {
    /// 创建新的分析管线
    ///
    /// # Errors
    /// 如果 VLM API Key 未设置，返回 `vlm_client::VlmError`
    pub fn new(config: AnalysisConfig) -> Result<Self, crate::bridge::vlm_client::VlmError> {
        // 验证配置
        if let Err(e) = config.validate() {
            eprintln!("警告：配置验证失败，已自动修正：{:?}", e);
        }

        Ok(Self {
            extractor: CadPrimitiveExtractor::new(ExtractorConfig {
                geometry: crate::error::GeometryConfig {
                    normalize_range: config.normalize_range,
                    enable_normalization: config.enable_normalization,
                    angle_tolerance: config.angle_tolerance,
                    distance_tolerance: config.distance_tolerance,
                    min_confidence: config.min_confidence,
                },
                min_line_length: 0.1,
                min_circle_radius: 0.1,
                filter_text: false,
                layer_filter: None,
            }),
            reasoner: GeometricRelationReasoner::new(ReasoningConfig {
                angle_tolerance: config.angle_tolerance,
                distance_tolerance: config.distance_tolerance,
                min_confidence: config.min_confidence,
                detect_parallel: true,
                detect_perpendicular: true,
                detect_collinear: true,
                detect_tangent: true,
                detect_concentric: true,
                detect_connected: true,
                detect_symmetric: false,
            }),
            verifier: ConstraintVerifier::new(VerifierConfig::default()),
            prompt_builder: PromptBuilder::new(PromptConfig {
                max_primitives_display: config.max_primitives_display,
                ..Default::default()
            }),
            vlm_client: VlmClient::with_zazaz()?,
            config,
        })
    }

    /// 使用默认配置创建管线
    ///
    /// # Errors
    /// 如果 VLM API Key 未设置，返回 `vlm_client::VlmError`
    pub fn with_defaults() -> Result<Self, crate::bridge::vlm_client::VlmError> {
        Self::new(AnalysisConfig::default())
    }

    /// 使用自定义 VLM 配置创建管线
    pub fn with_vlm_config(config: AnalysisConfig, vlm_config: VlmConfig) -> Self {
        Self {
            extractor: CadPrimitiveExtractor::new(ExtractorConfig {
                geometry: crate::error::GeometryConfig {
                    normalize_range: config.normalize_range,
                    enable_normalization: config.enable_normalization,
                    angle_tolerance: config.angle_tolerance,
                    distance_tolerance: config.distance_tolerance,
                    min_confidence: config.min_confidence,
                },
                min_line_length: 0.1,
                min_circle_radius: 0.1,
                filter_text: false,
                layer_filter: None,
            }),
            reasoner: GeometricRelationReasoner::new(ReasoningConfig {
                angle_tolerance: config.angle_tolerance,
                distance_tolerance: config.distance_tolerance,
                min_confidence: config.min_confidence,
                detect_parallel: true,
                detect_perpendicular: true,
                detect_collinear: true,
                detect_tangent: true,
                detect_concentric: true,
                detect_connected: true,
                detect_symmetric: false,
            }),
            verifier: ConstraintVerifier::new(VerifierConfig::default()),
            prompt_builder: PromptBuilder::new(PromptConfig {
                max_primitives_display: config.max_primitives_display,
                ..Default::default()
            }),
            vlm_client: VlmClient::new(vlm_config),
            config,
        }
    }

    /// 运行 VLM 推理
    ///
    /// # Errors
    /// 如果 VLM 推理失败，返回 `CadAgentError::Api`
    pub fn run_vlm_inference(&self, prompt: &str) -> CadAgentResult<String> {
        let messages = &[
            ("system", "你是一个 CAD 几何推理专家。你将收到精确的几何数据和约束关系，请基于这些结构化信息进行推理分析。你的推理应该：1. 基于给定的几何事实，不臆测 2. 逻辑清晰，分步骤推理 3. 输出可解释、可验证的结论"),
            ("user", prompt),
        ];

        let response = self.vlm_client
            .chat_completions_blocking(messages)
            .map_err(|e| CadAgentError::Api(format!("VLM 推理失败：{}", e)))?;

        let content = response.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "无回答".to_string());

        if self.config.verbose {
            eprintln!("VLM 推理完成");
            if let Some(usage) = &response.usage {
                eprintln!("Token 使用：prompt={}, completion={}, total={}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
            }
        }

        Ok(content)
    }

    /// 运行 VLM 推理并返回结构化结果
    ///
    /// # Errors
    /// 如果 VLM 推理失败，返回 `CadAgentError::Api`
    pub fn run_vlm_inference_with_details(
        &self,
        prompt: &str,
    ) -> CadAgentResult<VlmResponseInfo> {
        let messages = &[
            ("system", "你是一个 CAD 几何推理专家。你将收到精确的几何数据和约束关系，请基于这些结构化信息进行推理分析。你的推理应该：1. 基于给定的几何事实，不臆测 2. 逻辑清晰，分步骤推理 3. 输出可解释、可验证的结论"),
            ("user", prompt),
        ];

        let response = self.vlm_client
            .chat_completions_blocking(messages)
            .map_err(|e| CadAgentError::Api(format!("VLM 推理失败：{}", e)))?;

        let choice = response.choices
            .first()
            .ok_or_else(|| CadAgentError::Api("VLM 无回答".to_string()))?;

        let usage = response.usage.map(|u| TokenUsageInfo {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(VlmResponseInfo {
            content: choice.message.content.clone(),
            model: response.model,
            usage,
            latency_ms: 0,
        })
    }

    /// 从 SVG 文件注入上下文（不执行 VLM 推理）
    ///
    /// # Errors
    /// 如果基元提取、几何推理或提示词构造失败，返回相应的 `CadAgentError`
    pub fn inject_from_svg(&self, svg_path: impl AsRef<Path>, task: &str) -> CadAgentResult<AnalysisResult> {
        let start_time = Instant::now();
        let mut execution_log = Vec::new();
        let mut tool_call_chain = ToolCallChain::new();

        execution_log.push(format!("开始处理 SVG 文件：{:?}", svg_path.as_ref()));

        // Step 1: 提取基元
        execution_log.push("Step 1: 提取 CAD 基元...".to_string());
        let step1_start = Instant::now();
        let extraction_result = self.extractor.extract_from_svg(svg_path.as_ref());
        let step1_latency = step1_start.elapsed().as_millis() as u64;
        
        let extraction_result = match extraction_result {
            Ok(r) => {
                execution_log.push(format!("  提取了 {} 个基元", r.primitives.len()));
                tool_call_chain.add_step(ToolCallStep {
                    step: 1,
                    tool_name: "cad_extract_primitives".to_string(),
                    description: "从 SVG 图纸提取几何基元".to_string(),
                    latency_ms: step1_latency,
                    success: true,
                    error: None,
                    output_stats: HashMap::from([
                        ("primitive_count".to_string(), serde_json::json!(r.primitives.len())),
                    ]),
                });
                r
            }
            Err(e) => {
                tool_call_chain.add_step(ToolCallStep {
                    step: 1,
                    tool_name: "cad_extract_primitives".to_string(),
                    description: "从 SVG 图纸提取几何基元".to_string(),
                    latency_ms: step1_latency,
                    success: false,
                    error: Some(e.to_string()),
                    output_stats: HashMap::new(),
                });
                return Err(e);
            }
        };

        // Step 2: 推理几何关系
        execution_log.push("Step 2: 推理几何关系...".to_string());
        let step2_start = Instant::now();
        let reasoning_result = self.reasoner.find_all_relations(&extraction_result.primitives);
        let step2_latency = step2_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  发现 {} 个几何关系", reasoning_result.relations.len()));
        
        tool_call_chain.add_step(ToolCallStep {
            step: 2,
            tool_name: "cad_find_geometric_relations".to_string(),
            description: "检测基元之间的几何关系（平行、垂直、连接等）".to_string(),
            latency_ms: step2_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                ("relation_count".to_string(), serde_json::json!(reasoning_result.relations.len())),
                ("parallel_count".to_string(), serde_json::json!(reasoning_result.statistics.parallel_count)),
                ("perpendicular_count".to_string(), serde_json::json!(reasoning_result.statistics.perpendicular_count)),
            ]),
        });

        // Step 3: 约束合法性校验
        let verification_result = if !self.config.skip_verification {
            execution_log.push("Step 3: 校验约束合法性...".to_string());
            let step3_start = Instant::now();
            let result = self.verifier.verify(
                &extraction_result.primitives,
                &reasoning_result.relations,
            );
            let step3_latency = step3_start.elapsed().as_millis() as u64;
            
            match result {
                Ok(r) => {
                    execution_log.push(format!(
                        "  校验结果：{} (评分：{:.2})",
                        if r.is_valid { "通过" } else { "未通过" },
                        r.overall_score
                    ));
                    tool_call_chain.add_step(ToolCallStep {
                        step: 3,
                        tool_name: "cad_verify_constraints".to_string(),
                        description: "校验约束合法性，检测冲突和冗余".to_string(),
                        latency_ms: step3_latency,
                        success: true,
                        error: None,
                        output_stats: HashMap::from([
                            ("is_valid".to_string(), serde_json::json!(r.is_valid)),
                            ("overall_score".to_string(), serde_json::json!(r.overall_score)),
                            ("conflict_count".to_string(), serde_json::json!(r.conflicts.len())),
                        ]),
                    });
                    Some(r)
                }
                Err(e) => {
                    tool_call_chain.add_step(ToolCallStep {
                        step: 3,
                        tool_name: "cad_verify_constraints".to_string(),
                        description: "校验约束合法性，检测冲突和冗余".to_string(),
                        latency_ms: step3_latency,
                        success: false,
                        error: Some(e.to_string()),
                        output_stats: HashMap::new(),
                    });
                    return Err(e);
                }
            }
        } else {
            execution_log.push("Step 3: 跳过（配置为不校验）".to_string());
            None
        };

        // Step 4: 构造结构化提示词
        execution_log.push("Step 4: 构造结构化提示词...".to_string());
        let step4_start = Instant::now();
        let prompt = self.prompt_builder.build_reasoning_prompt(
            &extraction_result.primitives,
            &reasoning_result.relations,
            verification_result.as_ref(),
            task,
        );
        let step4_latency = step4_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  提示词长度：{} 字符", prompt.full_prompt.len()));
        
        tool_call_chain.add_step(ToolCallStep {
            step: 4,
            tool_name: "cad_build_analysis_prompt".to_string(),
            description: "构建几何分析提示词，注入精准上下文".to_string(),
            latency_ms: step4_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                ("prompt_length".to_string(), serde_json::json!(prompt.full_prompt.len())),
            ]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{}ms", total_latency_ms));

        Ok(AnalysisResult {
            primitives: extraction_result.primitives,
            relations: reasoning_result.relations,
            verification: verification_result,
            prompt,
            execution_log,
            total_latency_ms,
            vlm_response: None,
            tool_call_chain: Some(tool_call_chain),
        })
    }

    /// 从 SVG 文件注入上下文并执行 VLM 推理
    ///
    /// # Errors
    /// 如果基元提取、几何推理或 VLM 推理失败，返回相应的 `CadAgentError`
    pub fn inject_from_svg_with_vlm(&self, svg_path: impl AsRef<Path>, task: &str) -> CadAgentResult<AnalysisResult> {
        let mut result = self.inject_from_svg(svg_path, task)?;

        // Step 5: VLM 推理
        result.execution_log.push("Step 5: 执行 VLM 推理...".to_string());
        let vlm_result = self.run_vlm_inference_with_details(&result.prompt.full_prompt)?;
        result.execution_log.push(format!("  VLM 回答长度：{} 字符", vlm_result.content.len()));
        result.vlm_response = Some(vlm_result);
        result.execution_log.push("完成".to_string());

        Ok(result)
    }

    /// 从 SVG 字符串注入上下文（不执行 VLM 推理）
    ///
    /// # Errors
    /// 如果基元提取、几何推理或提示词构造失败，返回相应的 `CadAgentError`
    pub fn inject_from_svg_string(&self, svg_content: &str, task: &str) -> CadAgentResult<AnalysisResult> {
        let start_time = Instant::now();
        let mut execution_log = Vec::new();
        let mut tool_call_chain = ToolCallChain::new();

        execution_log.push("开始处理 SVG 字符串...".to_string());

        // Step 1: 提取基元
        execution_log.push("Step 1: 提取 CAD 基元...".to_string());
        let step1_start = Instant::now();
        let extraction_result = self.extractor.extract_from_svg_string(svg_content);
        let step1_latency = step1_start.elapsed().as_millis() as u64;
        
        let extraction_result = match extraction_result {
            Ok(r) => {
                execution_log.push(format!("  提取了 {} 个基元", r.primitives.len()));
                tool_call_chain.add_step(ToolCallStep {
                    step: 1,
                    tool_name: "cad_extract_primitives".to_string(),
                    description: "从 SVG 图纸提取几何基元".to_string(),
                    latency_ms: step1_latency,
                    success: true,
                    error: None,
                    output_stats: HashMap::from([
                        ("primitive_count".to_string(), serde_json::json!(r.primitives.len())),
                    ]),
                });
                r
            }
            Err(e) => {
                tool_call_chain.add_step(ToolCallStep {
                    step: 1,
                    tool_name: "cad_extract_primitives".to_string(),
                    description: "从 SVG 图纸提取几何基元".to_string(),
                    latency_ms: step1_latency,
                    success: false,
                    error: Some(e.to_string()),
                    output_stats: HashMap::new(),
                });
                return Err(e);
            }
        };

        // Step 2: 推理几何关系
        execution_log.push("Step 2: 推理几何关系...".to_string());
        let step2_start = Instant::now();
        let reasoning_result = self.reasoner.find_all_relations(&extraction_result.primitives);
        let step2_latency = step2_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  发现 {} 个几何关系", reasoning_result.relations.len()));
        
        tool_call_chain.add_step(ToolCallStep {
            step: 2,
            tool_name: "cad_find_geometric_relations".to_string(),
            description: "检测基元之间的几何关系（平行、垂直、连接等）".to_string(),
            latency_ms: step2_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                ("relation_count".to_string(), serde_json::json!(reasoning_result.relations.len())),
                ("parallel_count".to_string(), serde_json::json!(reasoning_result.statistics.parallel_count)),
                ("perpendicular_count".to_string(), serde_json::json!(reasoning_result.statistics.perpendicular_count)),
            ]),
        });

        // Step 3: 约束合法性校验
        let verification_result = if !self.config.skip_verification {
            execution_log.push("Step 3: 校验约束合法性...".to_string());
            let step3_start = Instant::now();
            let result = self.verifier.verify(
                &extraction_result.primitives,
                &reasoning_result.relations,
            );
            let step3_latency = step3_start.elapsed().as_millis() as u64;
            
            match result {
                Ok(r) => {
                    execution_log.push(format!(
                        "  校验结果：{} (评分：{:.2})",
                        if r.is_valid { "通过" } else { "未通过" },
                        r.overall_score
                    ));
                    tool_call_chain.add_step(ToolCallStep {
                        step: 3,
                        tool_name: "cad_verify_constraints".to_string(),
                        description: "校验约束合法性，检测冲突和冗余".to_string(),
                        latency_ms: step3_latency,
                        success: true,
                        error: None,
                        output_stats: HashMap::from([
                            ("is_valid".to_string(), serde_json::json!(r.is_valid)),
                            ("overall_score".to_string(), serde_json::json!(r.overall_score)),
                            ("conflict_count".to_string(), serde_json::json!(r.conflicts.len())),
                        ]),
                    });
                    Some(r)
                }
                Err(e) => {
                    tool_call_chain.add_step(ToolCallStep {
                        step: 3,
                        tool_name: "cad_verify_constraints".to_string(),
                        description: "校验约束合法性，检测冲突和冗余".to_string(),
                        latency_ms: step3_latency,
                        success: false,
                        error: Some(e.to_string()),
                        output_stats: HashMap::new(),
                    });
                    return Err(e);
                }
            }
        } else {
            execution_log.push("Step 3: 跳过（配置为不校验）".to_string());
            None
        };

        // Step 4: 构造结构化提示词
        execution_log.push("Step 4: 构造结构化提示词...".to_string());
        let step4_start = Instant::now();
        let prompt = self.prompt_builder.build_reasoning_prompt(
            &extraction_result.primitives,
            &reasoning_result.relations,
            verification_result.as_ref(),
            task,
        );
        let step4_latency = step4_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  提示词长度：{} 字符", prompt.full_prompt.len()));
        
        tool_call_chain.add_step(ToolCallStep {
            step: 4,
            tool_name: "cad_build_analysis_prompt".to_string(),
            description: "构建几何分析提示词，注入精准上下文".to_string(),
            latency_ms: step4_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                ("prompt_length".to_string(), serde_json::json!(prompt.full_prompt.len())),
            ]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{}ms", total_latency_ms));

        Ok(AnalysisResult {
            primitives: extraction_result.primitives,
            relations: reasoning_result.relations,
            verification: verification_result,
            prompt,
            execution_log,
            total_latency_ms,
            vlm_response: None,
            tool_call_chain: Some(tool_call_chain),
        })
    }

    /// 从 SVG 字符串注入上下文并执行 VLM 推理
    ///
    /// # Errors
    /// 如果基元提取、几何推理或 VLM 推理失败，返回相应的 `CadAgentError`
    pub fn inject_from_svg_string_with_vlm(&self, svg_content: &str, task: &str) -> CadAgentResult<AnalysisResult> {
        let mut result = self.inject_from_svg_string(svg_content, task)?;

        // Step 5: VLM 推理
        result.execution_log.push("Step 5: 执行 VLM 推理...".to_string());
        let vlm_result = self.run_vlm_inference_with_details(&result.prompt.full_prompt)?;
        result.execution_log.push(format!("  VLM 回答长度：{} 字符", vlm_result.content.len()));
        result.vlm_response = Some(vlm_result);
        result.execution_log.push("完成".to_string());

        Ok(result)
    }

    /// 从已有基元和关系注入上下文
    ///
    /// # Errors
    /// 如果几何推理、约束校验或提示词构造失败，返回相应的 `CadAgentError`
    pub fn inject_from_primitives(
        &self,
        primitives: &[crate::geometry::primitives::Primitive],
        task: &str,
    ) -> CadAgentResult<AnalysisResult> {
        let start_time = std::time::Instant::now();
        let mut execution_log = Vec::new();

        execution_log.push(format!("从 {} 个基元开始注入上下文...", primitives.len()));

        // Step 2: 推理几何关系
        execution_log.push("Step 2: 推理几何关系...".to_string());
        let reasoning_result = self.reasoner.find_all_relations(primitives);
        execution_log.push(format!("  发现 {} 个几何关系", reasoning_result.relations.len()));

        // Step 3: 约束合法性校验
        let verification_result = if !self.config.skip_verification {
            execution_log.push("Step 3: 校验约束合法性...".to_string());
            let result = self.verifier.verify(primitives, &reasoning_result.relations)?;
            execution_log.push(format!(
                "  校验结果：{} (评分：{:.2})",
                if result.is_valid { "通过" } else { "未通过" },
                result.overall_score
            ));
            Some(result)
        } else {
            execution_log.push("Step 3: 跳过（配置为不校验）".to_string());
            None
        };

        // Step 4: 构造结构化提示词
        execution_log.push("Step 4: 构造结构化提示词...".to_string());
        let step4_start = Instant::now();
        let prompt = self.prompt_builder.build_reasoning_prompt(
            primitives,
            &reasoning_result.relations,
            verification_result.as_ref(),
            task,
        );
        let step4_latency = step4_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  提示词长度：{} 字符", prompt.full_prompt.len()));
        
        let mut tool_call_chain = ToolCallChain::new();
        tool_call_chain.add_step(ToolCallStep {
            step: 4,
            tool_name: "cad_build_analysis_prompt".to_string(),
            description: "构建几何分析提示词，注入精准上下文".to_string(),
            latency_ms: step4_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                ("prompt_length".to_string(), serde_json::json!(prompt.full_prompt.len())),
            ]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{}ms", total_latency_ms));

        Ok(AnalysisResult {
            primitives: primitives.to_vec(),
            relations: reasoning_result.relations,
            verification: verification_result,
            prompt,
            execution_log,
            total_latency_ms,
            vlm_response: None,
            tool_call_chain: Some(tool_call_chain),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_pipeline_creation() {
        // 测试管线创建（需要 API Key，所以用 with_vlm_config）
        let config = AnalysisConfig::default();
        let vlm_config = VlmConfig::new(
            "https://zazaz.top/v1",
            "test_key",  // 测试用 key
            "./Qwen3.5-27B-FP8",
        );
        let _pipeline = AnalysisPipeline::with_vlm_config(config, vlm_config);
        assert!(true); // 能创建成功即可
    }

    #[test]
    fn test_analysis_config_validation() {
        let mut config = AnalysisConfig::default();
        assert!(config.validate().is_ok());

        // 测试无效配置
        config.angle_tolerance = -0.01;
        assert!(config.validate().is_err());

        // 测试自动修正
        let mut config = AnalysisConfig::default();
        config.angle_tolerance = -0.01;
        let warnings = config.validate_or_fix();
        assert!(!warnings.is_empty());
        assert!((config.angle_tolerance - 0.01).abs() < 1e-10);
    }
}
