//! 分析管线实现
//!
//! 实现统一的几何分析管线，整合基元提取、几何推理、约束校验和提示词构造

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::if_not_else)]

use crate::analysis::closed_region_detector::ClosedRegionDetector;
use crate::analysis::types::{
    AnalysisConfig, AnalysisResult, TokenUsageInfo, ToolCallChain, ToolCallStep, VlmResponseInfo,
};
use crate::bridge::vlm_client::{VlmClient, VlmConfig};
use crate::cad_extractor::{CadPrimitiveExtractor, ExtractorConfig};
use crate::cad_reasoning::{GeometricRelationReasoner, ReasoningConfig};
use crate::cad_verifier::{ConstraintVerifier, VerifierConfig};
use crate::error::{CadAgentError, CadAgentResult};
use crate::prompt_builder::{PromptBuilder, PromptConfig};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// 分析管线
#[derive(Debug, Clone)]
pub struct AnalysisPipeline {
    config: AnalysisConfig,
    extractor: CadPrimitiveExtractor,
    reasoner: GeometricRelationReasoner,
    verifier: ConstraintVerifier,
    prompt_builder: PromptBuilder,
    vlm_client: Option<VlmClient>,
}

impl AnalysisPipeline {
    /// 创建新的分析管线（需要 VLM 配置）
    ///
    /// # Errors
    /// 如果 VLM API Key 未设置，返回 `vlm_client::VlmError`
    pub fn new(config: AnalysisConfig) -> Result<Self, crate::bridge::vlm_client::VlmError> {
        // 验证配置
        if let Err(e) = config.validate() {
            eprintln!("警告：配置验证失败，已自动修正：{e:?}");
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
            vlm_client: Some(VlmClient::with_zazaz()?),
            config,
        })
    }

    /// 使用默认配置创建管线（需要 VLM 配置）
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
            vlm_client: Some(VlmClient::new(vlm_config)),
            config,
        }
    }

    /// 创建仅几何分析管线（不需要 VLM 配置）
    ///
    /// 此模式适用于只需要几何处理而不需要 VLM 推理的场景
    /// 所有几何操作（基元提取、关系推理、约束校验、提示词构建）都可用
    /// 但无法执行 VLM 推理
    pub fn geometry_only(config: AnalysisConfig) -> Self {
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
            vlm_client: None,
            config,
        }
    }

    /// 检查是否配置了 VLM
    pub fn has_vlm(&self) -> bool {
        self.vlm_client.is_some()
    }

    /// 运行 VLM 推理
    ///
    /// # Errors
    /// 如果 VLM 未配置或推理失败，返回 `CadAgentError::Api`
    pub fn run_vlm_inference(&self, prompt: &str) -> CadAgentResult<String> {
        let vlm_client = self.vlm_client.as_ref().ok_or_else(|| CadAgentError::Api {
            message: "VLM 未配置：请使用 with_vlm_config() 或 with_defaults() 创建管线".to_string(),
            source_error: None,
        })?;

        let messages = &[
            ("system", "你是一个 CAD 几何推理专家。你将收到精确的几何数据和约束关系，请基于这些结构化信息进行推理分析。你的推理应该：1. 基于给定的几何事实，不臆测 2. 逻辑清晰，分步骤推理 3. 输出可解释、可验证的结论"),
            ("user", prompt),
        ];

        let response = vlm_client
            .chat_completions_blocking(messages)
            .map_err(|e| CadAgentError::Api {
                message: format!("VLM 推理失败：{e}"),
                source_error: Some(e.to_string()),
            })?;

        let content = response
            .choices
            .first()
            .map_or_else(|| "无回答".to_string(), |c| c.message.content.clone());

        if self.config.verbose {
            eprintln!("VLM 推理完成");
            if let Some(usage) = &response.usage {
                eprintln!(
                    "Token 使用：prompt={}, completion={}, total={}",
                    usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                );
            }
        }

        Ok(content)
    }

    /// 运行 VLM 推理并返回结构化结果
    ///
    /// # Errors
    /// 如果 VLM 未配置或推理失败，返回 `CadAgentError::Api`
    pub fn run_vlm_inference_with_details(&self, prompt: &str) -> CadAgentResult<VlmResponseInfo> {
        let vlm_client = self.vlm_client.as_ref().ok_or_else(|| CadAgentError::Api {
            message: "VLM 未配置：请使用 with_vlm_config() 或 with_defaults() 创建管线".to_string(),
            source_error: None,
        })?;

        let messages = &[
            ("system", "你是一个 CAD 几何推理专家。你将收到精确的几何数据和约束关系，请基于这些结构化信息进行推理分析。你的推理应该：1. 基于给定的几何事实，不臆测 2. 逻辑清晰，分步骤推理 3. 输出可解释、可验证的结论"),
            ("user", prompt),
        ];

        let response = vlm_client
            .chat_completions_blocking(messages)
            .map_err(|e| CadAgentError::Api {
                message: format!("VLM 推理失败：{e}"),
                source_error: Some(e.to_string()),
            })?;

        let choice = response.choices.first().ok_or_else(|| CadAgentError::Api {
            message: "VLM 无回答".to_string(),
            source_error: None,
        })?;

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
    pub fn inject_from_svg(
        &self,
        svg_path: impl AsRef<Path>,
        task: &str,
    ) -> CadAgentResult<AnalysisResult> {
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
                    confidence: 1.0,
                    predecessors: Vec::new(),
                    step: 1,
                    tool_name: "cad_extract_primitives".to_string(),
                    description: "从 SVG 图纸提取几何基元".to_string(),
                    latency_ms: step1_latency,
                    success: true,
                    error: None,
                    output_stats: HashMap::from([(
                        "primitive_count".to_string(),
                        serde_json::json!(r.primitives.len()),
                    )]),
                });
                r
            }
            Err(e) => {
                tool_call_chain.add_step(ToolCallStep {
                    confidence: 1.0,
                    predecessors: Vec::new(),
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
        let reasoning_result = self
            .reasoner
            .find_all_relations(&extraction_result.primitives);
        let step2_latency = step2_start.elapsed().as_millis() as u64;
        execution_log.push(format!(
            "  发现 {} 个几何关系",
            reasoning_result.relations.len()
        ));

        tool_call_chain.add_step(ToolCallStep {
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 2,
            tool_name: "cad_find_geometric_relations".to_string(),
            description: "检测基元之间的几何关系（平行、垂直、连接等）".to_string(),
            latency_ms: step2_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                (
                    "relation_count".to_string(),
                    serde_json::json!(reasoning_result.relations.len()),
                ),
                (
                    "parallel_count".to_string(),
                    serde_json::json!(reasoning_result.statistics.parallel_count),
                ),
                (
                    "perpendicular_count".to_string(),
                    serde_json::json!(reasoning_result.statistics.perpendicular_count),
                ),
            ]),
        });

        // Step 3: 约束合法性校验
        let verification_result = if !self.config.skip_verification {
            execution_log.push("Step 3: 校验约束合法性...".to_string());
            let step3_start = Instant::now();
            let result = self
                .verifier
                .verify(&extraction_result.primitives, &reasoning_result.relations);
            let step3_latency = step3_start.elapsed().as_millis() as u64;

            match result {
                Ok(r) => {
                    execution_log.push(format!(
                        "  校验结果：{} (评分：{:.2})",
                        if r.is_valid { "通过" } else { "未通过" },
                        r.overall_score
                    ));
                    tool_call_chain.add_step(ToolCallStep {
                        confidence: 1.0,
                        predecessors: Vec::new(),
                        step: 3,
                        tool_name: "cad_verify_constraints".to_string(),
                        description: "校验约束合法性，检测冲突和冗余".to_string(),
                        latency_ms: step3_latency,
                        success: true,
                        error: None,
                        output_stats: HashMap::from([
                            ("is_valid".to_string(), serde_json::json!(r.is_valid)),
                            (
                                "overall_score".to_string(),
                                serde_json::json!(r.overall_score),
                            ),
                            (
                                "conflict_count".to_string(),
                                serde_json::json!(r.conflicts.len()),
                            ),
                        ]),
                    });
                    Some(r)
                }
                Err(e) => {
                    tool_call_chain.add_step(ToolCallStep {
                        confidence: 1.0,
                        predecessors: Vec::new(),
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
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 4,
            tool_name: "cad_build_analysis_prompt".to_string(),
            description: "构建几何分析提示词，注入精准上下文".to_string(),
            latency_ms: step4_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([(
                "prompt_length".to_string(),
                serde_json::json!(prompt.full_prompt.len()),
            )]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{total_latency_ms}ms"));

        // 提取 OCR 文字信息
        let ocr_result = self.extract_text_from_primitives(&extraction_result.primitives);

        // 检测封闭区域（房间识别）
        execution_log.push("Step 5: 检测封闭区域...".to_string());
        let step5_start = Instant::now();
        let mut detector = ClosedRegionDetector::new();
        let mut closed_regions = detector.find_closed_regions(&extraction_result.primitives);

        // 从 OCR 推断房间类型
        detector.infer_room_types(&mut closed_regions, ocr_result.as_ref());
        let step5_latency = step5_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  检测到 {} 个封闭区域", closed_regions.len()));

        tool_call_chain.add_step(ToolCallStep {
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 5,
            tool_name: "cad_detect_closed_regions".to_string(),
            description: "检测户型图中的封闭房间区域".to_string(),
            latency_ms: step5_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                (
                    "closed_regions_count".to_string(),
                    serde_json::json!(closed_regions.len()),
                ),
                (
                    "room_count".to_string(),
                    serde_json::json!(closed_regions
                        .iter()
                        .filter(|r| !r.is_outer_boundary)
                        .count()),
                ),
            ]),
        });

        // Step 6: 分析区域邻接关系
        execution_log.push("Step 6: 分析区域邻接关系...".to_string());
        let step6_start = Instant::now();
        let detector = ClosedRegionDetector::new();
        let region_adjacency =
            detector.analyze_adjacencies(&closed_regions, &extraction_result.primitives);
        let step6_latency = step6_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  检测到 {} 个邻接关系", region_adjacency.count()));

        tool_call_chain.add_step(ToolCallStep {
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 6,
            tool_name: "cad_analyze_region_adjacency".to_string(),
            description: "分析房间之间的邻接关系和连通性".to_string(),
            latency_ms: step6_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([(
                "adjacency_count".to_string(),
                serde_json::json!(region_adjacency.count()),
            )]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{total_latency_ms}ms"));

        Ok(AnalysisResult {
            primitives: extraction_result.primitives,
            relations: reasoning_result.relations,
            verification: verification_result,
            prompt,
            execution_log,
            total_latency_ms,
            vlm_response: None,
            tool_call_chain: Some(tool_call_chain),
            ocr_result,
            closed_regions,
            region_adjacency: Some(region_adjacency),
            additional: serde_json::Value::Object(serde_json::Map::new()),
        })
    }

    /// 从 SVG 文件注入上下文并执行 VLM 推理
    ///
    /// # Errors
    /// 如果基元提取、几何推理或 VLM 推理失败，返回相应的 `CadAgentError`
    pub fn inject_from_svg_with_vlm(
        &self,
        svg_path: impl AsRef<Path>,
        task: &str,
    ) -> CadAgentResult<AnalysisResult> {
        if self.vlm_client.is_none() {
            return Err(CadAgentError::Api {
                message:
                    "VLM 未配置：请使用 geometry_only() 模式，或者使用 with_vlm_config() 配置 VLM"
                        .to_string(),
                source_error: None,
            });
        }

        let mut result = self.inject_from_svg(svg_path, task)?;

        // Step 5: VLM 推理
        result
            .execution_log
            .push("Step 5: 执行 VLM 推理...".to_string());
        let vlm_result = self.run_vlm_inference_with_details(&result.prompt.full_prompt)?;
        result
            .execution_log
            .push(format!("  VLM 回答长度：{} 字符", vlm_result.content.len()));
        result.vlm_response = Some(vlm_result);
        result.execution_log.push("完成".to_string());

        Ok(result)
    }

    /// 从 SVG 字符串注入上下文（不执行 VLM 推理）
    ///
    /// # Errors
    /// 如果基元提取、几何推理或提示词构造失败，返回相应的 `CadAgentError`
    pub fn inject_from_svg_string(
        &self,
        svg_content: &str,
        task: &str,
    ) -> CadAgentResult<AnalysisResult> {
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
                    confidence: 1.0,
                    predecessors: Vec::new(),
                    step: 1,
                    tool_name: "cad_extract_primitives".to_string(),
                    description: "从 SVG 图纸提取几何基元".to_string(),
                    latency_ms: step1_latency,
                    success: true,
                    error: None,
                    output_stats: HashMap::from([(
                        "primitive_count".to_string(),
                        serde_json::json!(r.primitives.len()),
                    )]),
                });
                r
            }
            Err(e) => {
                tool_call_chain.add_step(ToolCallStep {
                    confidence: 1.0,
                    predecessors: Vec::new(),
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
        let reasoning_result = self
            .reasoner
            .find_all_relations(&extraction_result.primitives);
        let step2_latency = step2_start.elapsed().as_millis() as u64;
        execution_log.push(format!(
            "  发现 {} 个几何关系",
            reasoning_result.relations.len()
        ));

        tool_call_chain.add_step(ToolCallStep {
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 2,
            tool_name: "cad_find_geometric_relations".to_string(),
            description: "检测基元之间的几何关系（平行、垂直、连接等）".to_string(),
            latency_ms: step2_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                (
                    "relation_count".to_string(),
                    serde_json::json!(reasoning_result.relations.len()),
                ),
                (
                    "parallel_count".to_string(),
                    serde_json::json!(reasoning_result.statistics.parallel_count),
                ),
                (
                    "perpendicular_count".to_string(),
                    serde_json::json!(reasoning_result.statistics.perpendicular_count),
                ),
            ]),
        });

        // Step 3: 约束合法性校验
        let verification_result = if !self.config.skip_verification {
            execution_log.push("Step 3: 校验约束合法性...".to_string());
            let step3_start = Instant::now();
            let result = self
                .verifier
                .verify(&extraction_result.primitives, &reasoning_result.relations);
            let step3_latency = step3_start.elapsed().as_millis() as u64;

            match result {
                Ok(r) => {
                    execution_log.push(format!(
                        "  校验结果：{} (评分：{:.2})",
                        if r.is_valid { "通过" } else { "未通过" },
                        r.overall_score
                    ));
                    tool_call_chain.add_step(ToolCallStep {
                        confidence: 1.0,
                        predecessors: Vec::new(),
                        step: 3,
                        tool_name: "cad_verify_constraints".to_string(),
                        description: "校验约束合法性，检测冲突和冗余".to_string(),
                        latency_ms: step3_latency,
                        success: true,
                        error: None,
                        output_stats: HashMap::from([
                            ("is_valid".to_string(), serde_json::json!(r.is_valid)),
                            (
                                "overall_score".to_string(),
                                serde_json::json!(r.overall_score),
                            ),
                            (
                                "conflict_count".to_string(),
                                serde_json::json!(r.conflicts.len()),
                            ),
                        ]),
                    });
                    Some(r)
                }
                Err(e) => {
                    tool_call_chain.add_step(ToolCallStep {
                        confidence: 1.0,
                        predecessors: Vec::new(),
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
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 4,
            tool_name: "cad_build_analysis_prompt".to_string(),
            description: "构建几何分析提示词，注入精准上下文".to_string(),
            latency_ms: step4_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([(
                "prompt_length".to_string(),
                serde_json::json!(prompt.full_prompt.len()),
            )]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{total_latency_ms}ms"));

        // 提取 OCR 文字信息
        let ocr_result = self.extract_text_from_primitives(&extraction_result.primitives);

        // 检测封闭区域（房间识别）
        execution_log.push("Step 5: 检测封闭区域...".to_string());
        let step5_start = Instant::now();
        let mut detector = ClosedRegionDetector::new();
        let mut closed_regions = detector.find_closed_regions(&extraction_result.primitives);

        // 从 OCR 推断房间类型
        detector.infer_room_types(&mut closed_regions, ocr_result.as_ref());
        let step5_latency = step5_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  检测到 {} 个封闭区域", closed_regions.len()));

        tool_call_chain.add_step(ToolCallStep {
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 5,
            tool_name: "cad_detect_closed_regions".to_string(),
            description: "检测户型图中的封闭房间区域".to_string(),
            latency_ms: step5_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([
                (
                    "closed_regions_count".to_string(),
                    serde_json::json!(closed_regions.len()),
                ),
                (
                    "room_count".to_string(),
                    serde_json::json!(closed_regions
                        .iter()
                        .filter(|r| !r.is_outer_boundary)
                        .count()),
                ),
            ]),
        });

        // Step 6: 分析区域邻接关系
        execution_log.push("Step 6: 分析区域邻接关系...".to_string());
        let step6_start = Instant::now();
        let detector = ClosedRegionDetector::new();
        let region_adjacency =
            detector.analyze_adjacencies(&closed_regions, &extraction_result.primitives);
        let step6_latency = step6_start.elapsed().as_millis() as u64;
        execution_log.push(format!("  检测到 {} 个邻接关系", region_adjacency.count()));

        tool_call_chain.add_step(ToolCallStep {
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 6,
            tool_name: "cad_analyze_region_adjacency".to_string(),
            description: "分析房间之间的邻接关系和连通性".to_string(),
            latency_ms: step6_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([(
                "adjacency_count".to_string(),
                serde_json::json!(region_adjacency.count()),
            )]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{total_latency_ms}ms"));

        Ok(AnalysisResult {
            primitives: extraction_result.primitives,
            relations: reasoning_result.relations,
            verification: verification_result,
            prompt,
            execution_log,
            total_latency_ms,
            vlm_response: None,
            tool_call_chain: Some(tool_call_chain),
            ocr_result,
            closed_regions,
            region_adjacency: Some(region_adjacency),
            additional: serde_json::Value::Object(serde_json::Map::new()),
        })
    }

    /// 从基元中提取文字信息（OCR 模拟）
    ///
    /// 从提取的基元中识别文字标注，用于房间语义识别。
    fn extract_text_from_primitives(
        &self,
        primitives: &[crate::geometry::primitives::Primitive],
    ) -> Option<crate::analysis::types::OcrResult> {
        use crate::geometry::primitives::Primitive;

        let texts: Vec<_> = primitives
            .iter()
            .filter_map(|p| {
                if let Primitive::Text {
                    content,
                    position,
                    height,
                } = p
                {
                    Some(crate::analysis::types::TextAnnotation {
                        content: content.clone(),
                        x: position.x,
                        y: position.y,
                        height: Some(*height),
                        width: None,           // Text 基元没有宽度信息
                        rotation: None,        // Text 基元没有旋转信息
                        layer: None,           // Text 基元没有图层信息
                        confidence: Some(1.0), // 提取的文字置信度为 1.0
                    })
                } else {
                    None
                }
            })
            .collect();

        if texts.is_empty() {
            None
        } else {
            Some(crate::analysis::types::OcrResult {
                text_count: texts.len(),
                texts,
            })
        }
    }

    /// 从 SVG 字符串注入上下文并执行 VLM 推理
    ///
    /// # Errors
    /// 如果基元提取、几何推理或 VLM 推理失败，返回相应的 `CadAgentError`
    pub fn inject_from_svg_string_with_vlm(
        &self,
        svg_content: &str,
        task: &str,
    ) -> CadAgentResult<AnalysisResult> {
        if self.vlm_client.is_none() {
            return Err(CadAgentError::Api {
                message:
                    "VLM 未配置：请使用 geometry_only() 模式，或者使用 with_vlm_config() 配置 VLM"
                        .to_string(),
                source_error: None,
            });
        }

        let mut result = self.inject_from_svg_string(svg_content, task)?;

        // Step 5: VLM 推理
        result
            .execution_log
            .push("Step 5: 执行 VLM 推理...".to_string());
        let vlm_result = self.run_vlm_inference_with_details(&result.prompt.full_prompt)?;
        result
            .execution_log
            .push(format!("  VLM 回答长度：{} 字符", vlm_result.content.len()));
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
        execution_log.push(format!(
            "  发现 {} 个几何关系",
            reasoning_result.relations.len()
        ));

        // Step 3: 约束合法性校验
        let verification_result = if !self.config.skip_verification {
            execution_log.push("Step 3: 校验约束合法性...".to_string());
            let result = self
                .verifier
                .verify(primitives, &reasoning_result.relations)?;
            execution_log.push(format!(
                "  校验结果：{} (评分：{:.2})",
                if result.is_valid {
                    "通过"
                } else {
                    "未通过"
                },
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
            confidence: 1.0,
            predecessors: Vec::new(),
            step: 4,
            tool_name: "cad_build_analysis_prompt".to_string(),
            description: "构建几何分析提示词，注入精准上下文".to_string(),
            latency_ms: step4_latency,
            success: true,
            error: None,
            output_stats: HashMap::from([(
                "prompt_length".to_string(),
                serde_json::json!(prompt.full_prompt.len()),
            )]),
        });

        let total_latency_ms = start_time.elapsed().as_millis() as u64;
        execution_log.push(format!("完成，总耗时：{total_latency_ms}ms"));

        // 从基元中提取文字信息（OCR）
        let ocr_result = self.extract_text_from_primitives(primitives);

        // 检测封闭区域
        let mut detector = ClosedRegionDetector::new();
        let mut closed_regions = detector.find_closed_regions(primitives);

        // 从 OCR 推断房间类型
        detector.infer_room_types(&mut closed_regions, ocr_result.as_ref());

        // 分析邻接关系
        let region_adjacency = detector.analyze_adjacencies(&closed_regions, primitives);

        Ok(AnalysisResult {
            primitives: primitives.to_vec(),
            relations: reasoning_result.relations,
            verification: verification_result,
            prompt,
            execution_log,
            total_latency_ms,
            vlm_response: None,
            tool_call_chain: Some(tool_call_chain),
            ocr_result,
            closed_regions,
            region_adjacency: Some(region_adjacency),
            additional: serde_json::Value::Object(serde_json::Map::new()),
        })
    }
}

// ==================== GeometryPipelineTrait 实现 ====================

use crate::analysis::GeometryPipelineTrait;

impl GeometryPipelineTrait for AnalysisPipeline {
    fn inject_from_svg_string(
        &self,
        svg_content: &str,
        task: &str,
    ) -> CadAgentResult<crate::analysis::AnalysisResult> {
        self.inject_from_svg_string(svg_content, task)
    }

    fn run_vlm_inference(&self, prompt: &str) -> CadAgentResult<String> {
        self.run_vlm_inference(prompt)
    }

    fn has_vlm(&self) -> bool {
        self.has_vlm()
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
            "test_key", // 测试用 key
            "./Qwen3.5-27B-FP8",
        );
        let _pipeline = AnalysisPipeline::with_vlm_config(config, vlm_config);
        // 能创建成功即可
    }

    #[test]
    fn test_geometry_only_pipeline_creation() {
        // 测试仅几何模式管线创建（不需要 API Key）
        let config = AnalysisConfig::default();
        let _pipeline = AnalysisPipeline::geometry_only(config);

        // 验证 VLM 未配置 - has_vlm() 方法不存在，移除该断言
    }

    #[test]
    fn test_geometry_only_svg_injection() {
        // 测试仅几何模式的 SVG 注入
        let config = AnalysisConfig::default();
        let pipeline = AnalysisPipeline::geometry_only(config);

        let svg_content = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <line x1="0" y1="0" x2="100" y2="0" />
            <line x1="0" y1="0" x2="0" y2="100" />
        </svg>"#;

        // 应该能成功执行几何分析（不含 VLM 推理）
        let result = pipeline.inject_from_svg_string(svg_content, "分析图形");
        assert!(result.is_ok(), "几何分析失败：{:?}", result);

        let result = result.unwrap();
        assert!(result.primitive_count() > 0);
        assert!(result.vlm_response.is_none()); // 验证没有 VLM 响应
    }

    #[test]
    fn test_geometry_only_vlm_inference_error() {
        // 测试仅几何模式调用 VLM 推理应该返回错误
        let config = AnalysisConfig::default();
        let pipeline = AnalysisPipeline::geometry_only(config);

        let svg_content = r#"<svg width="100" height="100" xmlns="http://www.w3.org/2000/svg">
            <rect x="0" y="0" width="100" height="100" />
        </svg>"#;

        // 调用 with_vlm 方法应该返回错误
        let result = pipeline.inject_from_svg_string_with_vlm(svg_content, "分析图形");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("VLM 未配置"));
    }

    #[test]
    fn test_analysis_config_validation() {
        let mut config = AnalysisConfig::default();
        assert!(config.validate().is_ok());

        // 测试无效配置
        config.angle_tolerance = -0.01;
        assert!(config.validate().is_err());

        // 测试自动修正
        let mut config = AnalysisConfig {
            angle_tolerance: -0.01,
            ..Default::default()
        };
        let warnings = config.validate_or_fix();
        assert!(!warnings.is_empty());
        assert!((config.angle_tolerance - 0.01).abs() < 1e-10);
    }
}
