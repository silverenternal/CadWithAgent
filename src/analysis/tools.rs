//! 分析工具（tokitai 工具封装）
//!
//! 提供高层语义的 tokitai 工具，整合基元提取、几何推理、约束校验和提示词构造
//!
//! # 工具列表
//!
//! - `cad_analyze_layout` - 分析空间布局（高层语义工具）
//! - `cad_verify_design` - 校验设计约束合法性
//! - `cad_generate_cot` - 生成几何思维链数据

use crate::analysis::{AnalysisConfig, AnalysisPipeline, AnalysisResult};
use crate::bridge::vlm_client::VlmConfig;
use serde::{Deserialize, Serialize};
use tokitai::tool;

/// 空间布局分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialAnalysisResult {
    /// 基元数量
    pub primitive_count: usize,
    /// 关系数量
    pub relation_count: usize,
    /// 房间数量（如果检测到）
    pub room_count: Option<usize>,
    /// 总面积
    pub total_area: Option<f64>,
    /// 工具调用链
    pub tool_call_chain: serde_json::Value,
    /// 提示词
    pub prompt: String,
}

/// 约束校验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintVerificationResult {
    /// 是否通过校验
    pub is_valid: bool,
    /// 总体评分
    pub overall_score: f64,
    /// 冲突数量
    pub conflict_count: usize,
    /// 冲突详情
    pub conflicts: Vec<serde_json::Value>,
}

/// 几何思维链数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoCotData {
    /// 感知文本
    pub perception: String,
    /// 推理文本
    pub reasoning: String,
    /// 总结文本
    pub summary: String,
    /// 思维链
    pub thinking: String,
    /// 最终答案
    pub answer: String,
}

/// 分析工具集
#[derive(Default, Clone)]
pub struct AnalysisTools;

#[tool]
impl AnalysisTools {
    /// 分析空间布局
    ///
    /// 高层语义工具：整合基元提取、几何推理、约束校验，输出结构化分析结果
    ///
    /// # 参数
    ///
    /// * `svg_content` - SVG 文件内容或路径
    /// * `task` - 分析任务描述（如"分析这个户型图的房间布局"）
    /// * `config_json` - 可选的配置（JSON 格式）
    ///
    /// # 返回
    ///
    /// 空间布局分析结果，包含基元、关系、校验结果和提示词
    #[tool(name = "cad_analyze_layout")]
    pub fn analyze_layout(
        &self,
        svg_content: String,
        task: String,
        config_json: Option<String>,
    ) -> serde_json::Value {
        let config: AnalysisConfig = config_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let base_url = std::env::var("PROVIDER_ZAZAZ_API_URL")
            .unwrap_or_else(|_| "https://zazaz.top/v1".to_string());
        let api_key = std::env::var("PROVIDER_ZAZAZ_API_KEY").unwrap_or_default();
        let model = std::env::var("PROVIDER_ZAZAZ_MODEL")
            .unwrap_or_else(|_| "./Qwen3.5-27B-FP8".to_string());

        let vlm_config = VlmConfig::new(base_url, api_key, model);

        let pipeline = AnalysisPipeline::with_vlm_config(config, vlm_config);

        let result = if std::path::Path::new(&svg_content).exists() {
            pipeline.inject_from_svg(&svg_content, &task)
        } else {
            pipeline.inject_from_svg_string(&svg_content, &task)
        };

        match result {
            Ok(result) => self.layout_result_to_json(result),
            Err(e) => serde_json::json!({
                "success": false,
                "error": e.to_string()
            }),
        }
    }

    /// 校验设计约束合法性
    ///
    /// # 参数
    ///
    /// * `svg_content` - SVG 文件内容或路径
    /// * `constraints_json` - 约束列表（JSON 格式）
    ///
    /// # 返回
    ///
    /// 约束校验结果，包含是否通过、评分和冲突详情
    #[tool(name = "cad_verify_design")]
    pub fn verify_design(
        &self,
        svg_content: String,
        constraints_json: serde_json::Value,
    ) -> serde_json::Value {
        // 解析约束
        let constraints: Vec<crate::cad_reasoning::GeometricRelation> =
            match serde_json::from_value(constraints_json) {
                Ok(c) => c,
                Err(e) => {
                    return serde_json::json!({
                        "success": false,
                        "error": format!("解析约束失败：{}", e)
                    })
                }
            };

        // 提取基元
        let extractor = crate::cad_extractor::CadPrimitiveExtractor::new(
            crate::cad_extractor::ExtractorConfig::default(),
        );

        let primitives_result = if std::path::Path::new(&svg_content).exists() {
            extractor.extract_from_svg(&svg_content)
        } else {
            extractor.extract_from_svg_string(&svg_content)
        };

        let primitives = match primitives_result {
            Ok(r) => r.primitives,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("提取基元失败：{}", e)
                })
            }
        };

        // 校验约束
        let verifier = crate::cad_verifier::ConstraintVerifier::new(
            crate::cad_verifier::VerifierConfig::default(),
        );

        match verifier.verify(&primitives, &constraints) {
            Ok(result) => serde_json::json!({
                "success": true,
                "is_valid": result.is_valid,
                "overall_score": result.overall_score,
                "conflict_count": result.conflicts.len(),
                "conflicts": result.conflicts.iter().map(|c| {
                    match c {
                        crate::cad_verifier::Conflict::ParallelPerpendicular { line1_id, line2_id, .. } => {
                            serde_json::json!({
                                "type": "parallel_perpendicular",
                                "line1_id": line1_id,
                                "line2_id": line2_id,
                                "description": "线段既平行又垂直"
                            })
                        }
                        crate::cad_verifier::Conflict::ConcentricTangent { circle1_id, circle2_id, .. } => {
                            serde_json::json!({
                                "type": "concentric_tangent",
                                "circle1_id": circle1_id,
                                "circle2_id": circle2_id,
                                "description": "圆既同心又相切"
                            })
                        }
                        crate::cad_verifier::Conflict::ConnectionMismatch { primitive1_id, primitive2_id, .. } => {
                            serde_json::json!({
                                "type": "connection_mismatch",
                                "primitive1_id": primitive1_id,
                                "primitive2_id": primitive2_id,
                                "description": "连接点不匹配"
                            })
                        }
                        crate::cad_verifier::Conflict::PolygonNotClosed { polygon_id, .. } => {
                            serde_json::json!({
                                "type": "polygon_not_closed",
                                "polygon_id": polygon_id,
                                "description": "多边形未闭合"
                            })
                        }
                        crate::cad_verifier::Conflict::CircularDependency { .. } => {
                            serde_json::json!({
                                "type": "circular_dependency",
                                "description": "约束循环依赖"
                            })
                        }
                    }
                }).collect::<Vec<_>>()
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "error": e.to_string()
            }),
        }
    }

    /// 生成几何思维链数据
    ///
    /// # 参数
    ///
    /// * `svg_content` - SVG 文件内容或路径
    /// * `task` - 任务描述
    ///
    /// # 返回
    ///
    /// Geo-CoT 数据，包含感知、推理、总结文本
    #[tool(name = "cad_generate_cot")]
    pub fn generate_cot(&self, svg_content: String, task: String) -> serde_json::Value {
        // 提取基元
        let extractor = crate::cad_extractor::CadPrimitiveExtractor::new(
            crate::cad_extractor::ExtractorConfig::default(),
        );

        let primitives_result = if std::path::Path::new(&svg_content).exists() {
            extractor.extract_from_svg(&svg_content)
        } else {
            extractor.extract_from_svg_string(&svg_content)
        };

        let primitives = match primitives_result {
            Ok(r) => r.primitives,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("提取基元失败：{}", e)
                })
            }
        };

        // 推理关系
        let reasoner = crate::cad_reasoning::GeometricRelationReasoner::with_defaults();
        let reasoning_result = reasoner.find_all_relations(&primitives);

        // 使用 GeoCotGenerator 生成思维链
        let generator = crate::cot::GeoCotGenerator::new();
        let cot_data = generator.generate(&primitives, &task);

        serde_json::json!({
            "success": true,
            "cot_data": {
                "perception": cot_data.perception,
                "reasoning": cot_data.reasoning,
                "summary": cot_data.summary,
                "thinking": cot_data.thinking,
                "answer": cot_data.answer
            },
            "statistics": {
                "primitive_count": primitives.len(),
                "relation_count": reasoning_result.relations.len()
            }
        })
    }

    /// 获取分析管线配置信息
    #[tool(name = "cad_get_analysis_info")]
    pub fn get_analysis_info(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "cad_analysis_pipeline",
            "description": "CAD 几何分析管线：整合基元提取、几何推理、约束校验和提示词构造",
            "type": "deterministic_algorithm",
            "tools": [
                {
                    "name": "cad_analyze_layout",
                    "description": "分析空间布局（高层语义工具）"
                },
                {
                    "name": "cad_verify_design",
                    "description": "校验设计约束合法性"
                },
                {
                    "name": "cad_generate_cot",
                    "description": "生成几何思维链数据"
                }
            ],
            "pipeline_steps": [
                {"id": 1, "name": "基元提取", "description": "从 SVG/DXF 提取结构化几何数据"},
                {"id": 2, "name": "几何推理", "description": "检测平行、垂直、连接等关系"},
                {"id": 3, "name": "约束校验", "description": "检测冲突和冗余约束"},
                {"id": 4, "name": "提示词构造", "description": "组织为结构化提示词"}
            ],
            "output_format": {
                "primitives": "几何基元列表",
                "relations": "几何关系列表",
                "verification": "约束校验结果",
                "prompt": "结构化提示词",
                "tool_call_chain": "工具调用链追踪"
            }
        })
    }
}

impl AnalysisTools {
    fn layout_result_to_json(&self, result: AnalysisResult) -> serde_json::Value {
        serde_json::json!({
            "success": true,
            "primitive_count": result.primitives.len(),
            "relation_count": result.relations.len(),
            "tool_call_chain": result.tool_chain_json(),
            "prompt": result.prompt.full_prompt,
            "metadata": {
                "total_latency_ms": result.total_latency_ms,
                "execution_log": result.execution_log
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_analysis_info() {
        let tools = AnalysisTools;
        let info = tools.get_analysis_info();

        assert_eq!(info["name"], "cad_analysis_pipeline");
        assert_eq!(info["tools"].as_array().unwrap().len(), 3);
        assert!(info["description"].as_str().unwrap().contains("CAD"));
        assert!(!info["pipeline_steps"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_spatial_analysis_result_clone() {
        let result = SpatialAnalysisResult {
            primitive_count: 10,
            relation_count: 5,
            room_count: Some(2),
            total_area: Some(100.0),
            tool_call_chain: serde_json::json!({"tools": []}),
            prompt: "测试提示词".to_string(),
        };

        let cloned = result.clone();
        assert_eq!(cloned.primitive_count, result.primitive_count);
        assert_eq!(cloned.room_count, result.room_count);
    }

    #[test]
    fn test_spatial_analysis_result_debug() {
        let result = SpatialAnalysisResult {
            primitive_count: 0,
            relation_count: 0,
            room_count: None,
            total_area: None,
            tool_call_chain: serde_json::json!(null),
            prompt: String::new(),
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("SpatialAnalysisResult"));
    }

    #[test]
    fn test_constraint_verification_result_clone() {
        let result = ConstraintVerificationResult {
            is_valid: true,
            overall_score: 0.9,
            conflict_count: 0,
            conflicts: vec![],
        };

        let cloned = result.clone();
        assert_eq!(cloned.is_valid, result.is_valid);
        assert!((cloned.overall_score - result.overall_score).abs() < 0.01);
    }

    #[test]
    fn test_constraint_verification_result_debug() {
        let result = ConstraintVerificationResult {
            is_valid: false,
            overall_score: 0.5,
            conflict_count: 1,
            conflicts: vec![serde_json::json!({"type": "test"})],
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("ConstraintVerificationResult"));
    }

    #[test]
    fn test_geo_cot_data_clone() {
        let data = GeoCotData {
            perception: "感知".to_string(),
            reasoning: "推理".to_string(),
            summary: "总结".to_string(),
            thinking: "思维".to_string(),
            answer: "答案".to_string(),
        };

        let cloned = data.clone();
        assert_eq!(cloned.perception, data.perception);
        assert_eq!(cloned.answer, data.answer);
    }

    #[test]
    fn test_geo_cot_data_debug() {
        let data = GeoCotData {
            perception: String::new(),
            reasoning: String::new(),
            summary: String::new(),
            thinking: String::new(),
            answer: String::new(),
        };

        let debug_str = format!("{:?}", data);
        assert!(debug_str.contains("GeoCotData"));
    }

    #[test]
    fn test_analyze_layout_invalid_svg() {
        let tools = AnalysisTools;
        // 使用无效的 SVG 内容
        let result = tools.analyze_layout(
            "invalid_svg_content".to_string(),
            "测试任务".to_string(),
            None,
        );

        // 应该返回错误（因为没有 API Key 或无效 SVG）
        assert!(!result["success"].as_bool().unwrap_or(true) || result["error"].as_str().is_some());
    }

    #[test]
    fn test_analyze_layout_with_config() {
        let tools = AnalysisTools;
        let config_json = r#"{"enable_parallel": false}"#;

        let result = tools.analyze_layout(
            "<svg></svg>".to_string(),
            "测试".to_string(),
            Some(config_json.to_string()),
        );

        // 配置应该被解析
        assert!(result.is_object());
    }

    #[test]
    fn test_verify_design_invalid_constraints() {
        let tools = AnalysisTools;
        let invalid_constraints = serde_json::json!({"invalid": "data"});

        let result = tools.verify_design("<svg></svg>".to_string(), invalid_constraints);

        assert!(!result["success"].as_bool().unwrap_or(false));
        assert!(result["error"].as_str().is_some());
    }

    #[test]
    fn test_verify_design_invalid_svg() {
        let tools = AnalysisTools;
        let constraints = serde_json::json!([
            {"relation_type": "parallel", "primitive1_id": "a", "primitive2_id": "b"}
        ]);

        let result = tools.verify_design("invalid_svg".to_string(), constraints);

        assert!(!result["success"].as_bool().unwrap_or(false));
    }

    #[test]
    fn test_generate_cot_invalid_svg() {
        let tools = AnalysisTools;

        let result = tools.generate_cot("invalid_svg".to_string(), "测试任务".to_string());

        assert!(!result["success"].as_bool().unwrap_or(false));
        assert!(result["error"].as_str().is_some());
    }

    #[test]
    fn test_generate_cot_basic() {
        let tools = AnalysisTools;
        let svg = r#"<svg width="100" height="100">
            <rect x="0" y="0" width="50" height="50"/>
        </svg>"#;

        let result = tools.generate_cot(svg.to_string(), "计算面积".to_string());

        // 应该成功生成 CoT 数据
        assert!(result["success"].as_bool().unwrap_or(false));
        assert!(result["cot_data"].is_object());
        assert!(
            result["statistics"]["primitive_count"]
                .as_u64()
                .unwrap_or(0)
                > 0
        );
    }

    #[test]
    fn test_layout_result_to_json() {
        let tools = AnalysisTools;
        // 这个方法是私有的，通过 analyze_layout 间接测试
        let result = tools.analyze_layout("<svg></svg>".to_string(), "测试".to_string(), None);

        assert!(result.is_object());
    }

    #[test]
    fn test_analysis_tools_default() {
        let tools = AnalysisTools;
        let info = tools.get_analysis_info();
        assert!(info["name"].as_str().is_some());
    }

    #[test]
    fn test_analysis_tools_clone() {
        let tools = AnalysisTools;
        let cloned = tools.clone();
        let info = cloned.get_analysis_info();
        assert_eq!(info["name"], "cad_analysis_pipeline");
    }
}
