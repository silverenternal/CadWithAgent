//! CadAgent - CAD 几何处理工具链
//!
//! 基于 Rust tokitai 库的标准化管线，将基础几何算法封装成工具供 AI 模型调用
//!
//! # 架构层次
//!
//! - **Tool Layer**: MCP 风格的工具封装
//! - **Engine Layer**: 核心几何计算引擎
//! - **Data Layer**: Geo-CoT 数据格式与持久化
//!
//! # 快速开始
//!
//! ```rust,no_run
//! use cadagent::prelude::*;
//!
//! // 创建分析管线
//! let config = cadagent::analysis::AnalysisConfig::default();
//! let pipeline = cadagent::analysis::AnalysisPipeline::with_defaults().unwrap();
//!
//! // 从 SVG 字符串注入上下文
//! let svg = r#"<svg width="100" height="100">
//!     <line x1="0" y1="0" x2="100" y2="100" />
//! </svg>"#;
//! let result = pipeline.inject_from_svg_string(svg, "分析这个图形").unwrap();
//!
//! println!("基元数量：{}", result.primitive_count());
//! println!("工具调用链：{:?}", result.tool_chain_json());
//! ```
//!
//! # 核心模块
//!
//! - [`parser`]: SVG/DXF 文件解析
//! - [`geometry`][]: 几何图元与测量工具
//! - [`topology`][]: 拓扑分析（回路、房间、门窗检测）
//! - [`cot`]: Geo-CoT 思维链生成
//! - [`export`]: DXF/JSON 导出
//! - [`metrics`][]: 几何一致性评估
//! - [`analysis`][]: **统一的几何分析管线**（推荐使用）
//!   - 整合基元提取、几何推理、约束校验和提示词构造
//!   - 提供 tokitai 工具接口
//!   - 支持工具调用链追踪
//! - [`cad_extractor`]: CAD 基元提取工具（底层模块）
//! - [`cad_reasoning`][]: 几何关系推理工具（底层模块）
//! - [`cad_verifier`][]: 约束合法性校验工具（底层模块）
//! - [`prompt_builder`][]: 结构化提示词构造器（底层模块）
//! - [`llm_reasoning`]: LLM 推理引擎（使用 analysis 模块）

pub mod parser;
pub mod geometry;
pub mod topology;
pub mod cot;
pub mod export;
pub mod metrics;
pub mod bridge;
pub mod tools;
// 核心功能模块
pub mod cad_extractor;
pub mod cad_reasoning;
pub mod cad_verifier;
pub mod prompt_builder;
// 统一分析管线（推荐使用）
pub mod analysis;
// LLM 推理
pub mod llm_reasoning;
pub mod error;
// 配置验证
pub mod config;

/// 预导出模块，方便快速导入
pub mod prelude {
    pub use crate::geometry::primitives::{Primitive, Point, Line, Polygon, Circle, Rect, Room, Door, Window};
    pub use crate::geometry::measure::GeometryMeasurer;
    pub use crate::geometry::transform::GeometryTransform;
    pub use crate::tools::registry::{ToolRegistry, ToolResult, ToolError};
    pub use crate::cot::generator::GeoCotGenerator;
    pub use crate::export::dxf::DxfExporter;
    pub use crate::parser::svg::SvgParser;
    pub use crate::error::{CadAgentError, CadAgentResult, GeometryToleranceConfig, GeometryConfig};

    // CAD 基元提取工具
    pub use crate::cad_extractor::{
        CadPrimitiveExtractor, ExtractorConfig, PrimitiveExtractionResult,
        PrimitiveStatistics, CoordinateInfo,
    };

    // CAD 几何关系推理工具
    pub use crate::cad_reasoning::{
        GeometricRelationReasoner, ReasoningConfig, ReasoningResult,
        GeometricRelation, RelationStatistics,
    };

    // CAD 约束合法性校验工具
    pub use crate::cad_verifier::{
        ConstraintVerifier, VerifierConfig, VerificationResult,
        Conflict, GeometryIssue, FixSuggestion,
    };

    // 结构化提示词构造器
    pub use crate::prompt_builder::{
        PromptBuilder, PromptConfig, StructuredPrompt, PromptTemplate,
    };

    // 统一分析管线（推荐使用）- 整合了基元提取、几何推理、约束校验和提示词构造
    pub use crate::analysis::{
        AnalysisPipeline, AnalysisConfig, AnalysisResult,
        ToolCallChain, ToolCallStep,
    };

    // 分析工具（tokitai 工具封装）
    pub use crate::analysis::tools::{
        AnalysisTools, SpatialAnalysisResult, ConstraintVerificationResult, GeoCotData,
    };

    pub use serde_json::json;
    pub use serde_json::Value;
}

/// 库版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 库名称
pub const NAME: &str = env!("CARGO_PKG_NAME");
