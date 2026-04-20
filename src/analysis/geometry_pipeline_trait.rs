//! 几何管线 trait
#![allow(clippy::cast_precision_loss)]
//!
//! 定义几何分析管线的统一接口，用于解耦 `llm_reasoning` 和具体实现
//!
//! # 设计目标
//!
//! - **解耦**: `llm_reasoning` 模块只依赖此 trait，不依赖具体实现
//! - **可测试**: 可以创建 Mock 实现用于单元测试
//! - **可扩展**: 未来可以替换不同的几何处理实现
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::analysis::{GeometryPipelineTrait, AnalysisPipeline, AnalysisConfig};
//!
//! // 使用真实实现
//! let pipeline = AnalysisPipeline::geometry_only(AnalysisConfig::default());
//! let result = pipeline.inject_from_svg_string("<svg>...</svg>", "分析图形").unwrap();
//!
//! // 在 llm_reasoning 中使用 trait
//! fn process_with_pipeline<P: GeometryPipelineTrait>(
//!     pipeline: &P,
//!     svg: &str,
//! ) -> cadagent::error::CadAgentResult<()> {
//!     let result = pipeline.inject_from_svg_string(svg, "分析")?;
//!     Ok(())
//! }
//! ```

use crate::error::CadAgentResult;
use serde_json::Value;

/// 几何分析管线 trait
///
/// 定义了 LLM 推理引擎需要调用的几何处理接口
pub trait GeometryPipelineTrait {
    /// 从 SVG 字符串注入上下文（不执行 VLM 推理）
    ///
    /// # Parameters
    /// * `svg_content` - SVG 内容
    /// * `task` - 任务描述
    ///
    /// # Returns
    /// 返回分析结果，包含基元、关系、提示词等
    fn inject_from_svg_string(
        &self,
        svg_content: &str,
        task: &str,
    ) -> CadAgentResult<crate::analysis::AnalysisResult>;

    /// 运行 VLM 推理
    ///
    /// # Parameters
    /// * `prompt` - 结构化提示词
    ///
    /// # Returns
    /// 返回 VLM 的回答内容
    fn run_vlm_inference(&self, prompt: &str) -> CadAgentResult<String>;

    /// 检查是否配置了 VLM
    fn has_vlm(&self) -> bool;

    /// 获取基元数量（便捷方法）
    fn get_primitive_count(&self, svg_content: &str, task: &str) -> CadAgentResult<usize> {
        let result = self.inject_from_svg_string(svg_content, task)?;
        Ok(result.primitive_count())
    }

    /// 获取几何数据摘要（用于 LLM 推理上下文）
    ///
    /// # Returns
    /// 返回 JSON 格式的几何数据摘要
    fn get_geometry_summary(&self, svg_content: &str, task: &str) -> CadAgentResult<Value> {
        let result = self.inject_from_svg_string(svg_content, task)?;
        Ok(serde_json::json!({
            "primitives_count": result.primitives.len(),
            "relations_count": result.relations.len(),
            "has_verification": result.verification.is_some(),
            "prompt_length": result.prompt.full_prompt.len(),
        }))
    }
}

/// Mock 几何管线实现
///
/// 用于单元测试，返回预定义的模拟数据
///
/// # 使用示例
///
/// ```rust,no_run
/// use cadagent::analysis::MockGeometryPipeline;
///
/// let mock = MockGeometryPipeline::new();
/// let result = mock.inject_from_svg_string("<svg>...</svg>", "测试").unwrap();
/// assert_eq!(result.primitive_count(), 10);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MockGeometryPipeline {
    /// 模拟的基元数量
    pub mock_primitive_count: usize,
    /// 模拟的关系数量
    pub mock_relation_count: usize,
    /// 是否模拟 VLM 可用
    pub mock_vlm_available: bool,
}

impl MockGeometryPipeline {
    /// 创建新的 Mock 管线
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置模拟的基元数量
    pub fn with_primitive_count(mut self, count: usize) -> Self {
        self.mock_primitive_count = count;
        self
    }

    /// 设置模拟的关系数量
    pub fn with_relation_count(mut self, count: usize) -> Self {
        self.mock_relation_count = count;
        self
    }

    /// 设置 VLM 是否可用
    pub fn with_vlm(mut self, available: bool) -> Self {
        self.mock_vlm_available = available;
        self
    }
}

impl GeometryPipelineTrait for MockGeometryPipeline {
    fn inject_from_svg_string(
        &self,
        _svg_content: &str,
        _task: &str,
    ) -> CadAgentResult<crate::analysis::AnalysisResult> {
        use crate::analysis::AnalysisResult;
        use crate::cad_reasoning::GeometricRelation;
        use crate::geometry::primitives::Primitive;
        use crate::geometry::{Line, Point};
        use crate::prompt_builder::StructuredPrompt;

        // 创建模拟的基元
        let primitives: Vec<Primitive> = (0..self.mock_primitive_count)
            .map(|i| {
                Primitive::Line(Line {
                    start: Point {
                        x: i as f64,
                        y: 0.0,
                    },
                    end: Point {
                        x: i as f64 + 1.0,
                        y: 1.0,
                    },
                })
            })
            .collect();

        // 创建模拟的关系（使用 Parallel 变体）
        let relations: Vec<GeometricRelation> = (0..self.mock_relation_count)
            .map(|i| GeometricRelation::Parallel {
                line1_id: i,
                line2_id: i + 1,
                angle_diff: 0.0,
                confidence: 1.0,
            })
            .collect();

        Ok(AnalysisResult {
            primitives,
            relations,
            verification: None,
            prompt: StructuredPrompt {
                full_prompt: format!("Mock prompt for {} primitives", self.mock_primitive_count),
                system_prompt: "System".to_string(),
                user_prompt: "User".to_string(),
                metadata: crate::prompt_builder::PromptMetadata {
                    primitive_count: self.mock_primitive_count,
                    constraint_count: self.mock_relation_count,
                    prompt_length: 100,
                    template: crate::prompt_builder::PromptTemplate::Analysis,
                    injected_context: Vec::new(),
                },
            },
            execution_log: vec!["Mock execution".to_string()],
            total_latency_ms: 10,
            vlm_response: None,
            tool_call_chain: None,
            ocr_result: None,
            closed_regions: Vec::new(),
            region_adjacency: None,
            additional: serde_json::Value::Object(serde_json::Map::new()),
        })
    }

    fn run_vlm_inference(&self, _prompt: &str) -> CadAgentResult<String> {
        if self.mock_vlm_available {
            Ok("Mock VLM response".to_string())
        } else {
            Err(crate::error::CadAgentError::Api {
                message: "VLM not configured".to_string(),
                source_error: None,
            })
        }
    }

    fn has_vlm(&self) -> bool {
        self.mock_vlm_available
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{AnalysisConfig, AnalysisPipeline};

    #[test]
    fn test_analysis_pipeline_implements_trait() {
        // 验证 AnalysisPipeline 实现 GeometryPipelineTrait
        let pipeline = AnalysisPipeline::geometry_only(AnalysisConfig::default());

        // 调用 trait 方法
        let svg = r#"<svg width="100" height="100">
            <line x1="0" y1="0" x2="100" y2="100" />
        </svg>"#;

        let result = pipeline.inject_from_svg_string(svg, "测试");
        assert!(result.is_ok());

        let result = result.unwrap();
        // primitive_count 返回 usize，永远 >= 0，这里只是验证方法能正常调用
        let _ = result.primitive_count();
    }

    #[test]
    fn test_has_vlm() {
        let pipeline = AnalysisPipeline::geometry_only(AnalysisConfig::default());
        assert!(!pipeline.has_vlm());
    }

    #[test]
    fn test_get_geometry_summary() {
        let pipeline = AnalysisPipeline::geometry_only(AnalysisConfig::default());

        let svg = r#"<svg width="100" height="100">
            <line x1="0" y1="0" x2="100" y2="100" />
        </svg>"#;

        let summary = pipeline.get_geometry_summary(svg, "测试").unwrap();

        assert!(summary.get("primitives_count").is_some());
        assert!(summary.get("relations_count").is_some());
        assert!(summary.get("has_verification").is_some());
        assert!(summary.get("prompt_length").is_some());
    }

    #[test]
    fn test_mock_geometry_pipeline() {
        let mock = MockGeometryPipeline::new()
            .with_primitive_count(10)
            .with_relation_count(5);

        let result = mock
            .inject_from_svg_string("<svg>test</svg>", "测试")
            .unwrap();

        assert_eq!(result.primitive_count(), 10);
        assert_eq!(result.relation_count(), 5);
        assert!(!mock.has_vlm());
    }

    #[test]
    fn test_mock_geometry_pipeline_with_vlm() {
        let mock = MockGeometryPipeline::new().with_vlm(true);

        let response = mock.run_vlm_inference("test prompt").unwrap();
        assert_eq!(response, "Mock VLM response");
        assert!(mock.has_vlm());
    }

    #[test]
    fn test_mock_geometry_pipeline_vlm_not_available() {
        let mock = MockGeometryPipeline::new().with_vlm(false);

        let result = mock.run_vlm_inference("test prompt");
        assert!(result.is_err());
    }
}
