//! `CadAgent` - CAD 几何处理工具链
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

// 加载环境变量
#[cfg(not(test))]
pub fn init_env() {
    dotenvy::dotenv().ok();
}

#[cfg(test)]
pub fn init_env() {
    dotenvy::dotenv().ok();
}

pub mod bridge;
pub mod cot;
pub mod export;
pub mod geometry;
pub mod metrics;
pub mod parser;
pub mod tools;
pub mod topology;
// 核心功能模块
pub mod cad_extractor;
pub mod cad_reasoning;
pub mod cad_verifier;
pub mod prompt_builder;
// 统一分析管线（推荐使用）
pub mod analysis;
// LLM 推理
pub mod error;
pub mod llm_reasoning;
// 上下文管理模块 (基于 tokitai-context)
pub mod context;
// 配置验证
pub mod config;
// Feature tree for parametric modeling
pub mod feature;
// Level of Detail system for large model performance
pub mod lod;
// Incremental update system for dependency tracking
pub mod incremental;
// GPU acceleration for compute and rendering
pub mod gpu;
// Memory optimization for large-scale geometry
pub mod memory;
// Web API server
pub mod web_server;

/// 预导出模块，方便快速导入
pub mod prelude {
    pub use crate::cot::generator::GeoCotGenerator;
    pub use crate::error::{
        CadAgentError, CadAgentResult, GeometryConfig, GeometryToleranceConfig,
    };
    pub use crate::export::dxf::DxfExporter;
    pub use crate::geometry::constraint::{
        Constraint, ConstraintId, ConstraintSolver, ConstraintStatus, ConstraintSystem, Entity,
        EntityId, EntityType, SolverConfig, SolverError,
    };
    pub use crate::geometry::measure::GeometryMeasurer;
    pub use crate::geometry::nurbs::{Mesh, NurbsCurve, NurbsSurface, Point3D};
    pub use crate::geometry::primitives::{
        Circle, Door, Line, Point, Polygon, Primitive, Rect, Room, Window,
    };
    pub use crate::geometry::transform::GeometryTransform;
    pub use crate::parser::{iges::IgesParser, step::StepParser, svg::SvgParser};
    pub use crate::tools::registry::{ToolError, ToolRegistry, ToolResult};

    // CAD 基元提取工具
    pub use crate::cad_extractor::{
        CadPrimitiveExtractor, CoordinateInfo, ExtractorConfig, PrimitiveExtractionResult,
        PrimitiveStatistics,
    };

    // CAD 几何关系推理工具
    pub use crate::cad_reasoning::{
        GeometricRelation, GeometricRelationReasoner, ReasoningConfig, ReasoningResult,
        RelationStatistics,
    };

    // CAD 约束合法性校验工具
    pub use crate::cad_verifier::{
        Conflict, ConstraintVerifier, FixSuggestion, GeometryIssue, VerificationResult,
        VerifierConfig,
    };

    // 结构化提示词构造器
    pub use crate::prompt_builder::{
        PromptBuilder, PromptConfig, PromptTemplate, StructuredPrompt,
    };

    // 统一分析管线（推荐使用）- 整合了基元提取、几何推理、约束校验和提示词构造
    pub use crate::analysis::{
        AnalysisConfig, AnalysisPipeline, AnalysisResult, ToolCallChain, ToolCallStep,
    };

    // 分析工具（tokitai 工具封装）
    pub use crate::analysis::tools::{
        AnalysisTools, ConstraintVerificationResult, GeoCotData, SpatialAnalysisResult,
    };

    // Context management (NEW - based on tokitai-context)
    pub use crate::context::{
        // Re-exported from tokitai-context
        Context,
        ContextConfig,
        ContextItem,
        DialogMessage,
        DialogStateConfig,
        // Dialog state
        DialogStateManager,
        ErrorCase,
        // Error library
        ErrorCaseLibrary,
        ErrorLibraryConfig,
        ErrorLibraryStats,
        ErrorSeverity,
        Layer,
        ParallelContextManager,
        ParallelContextManagerConfig,
        PlanStatus,
        SearchHit,
        TaskNode,
        TaskPlan,
        TaskPlanStats,
        // Task planner
        TaskPlanner,
        TaskPlannerConfig,
        TaskStatus,
    };

    // DialogState is in dialog_state module
    pub use crate::context::dialog_state::DialogState;

    // Feature tree for parametric modeling
    pub use crate::feature::{
        BooleanOp, ExtrudeDirection, Feature, FeatureId, FeatureNode, FeatureState, FeatureTree,
        FeatureTreeError, FeatureTreeResult, HistoryEntry, PatternType, RebuildResult, RevolveAxis,
        Sketch, SketchEntity, SketchEntityId, SketchError, SketchPlane,
    };

    // LOD system for large model performance
    pub use crate::lod::{
        LodConfig, LodLevel, LodManager, MeshSimplifier, SimplificationConfig,
        SimplificationStrategy,
    };

    // Incremental update system
    pub use crate::incremental::{
        Change, ChangeTracker, ChangeType, DependencyGraph, IncrementalUpdater,
    };

    // GPU acceleration
    pub use crate::gpu::{
        Camera, ComputeParams, ComputePipeline, GeometryCompute, GpuBuffer, GpuBufferBuilder,
        GpuContext, GpuError, IndexBuffer, RenderUniforms, Renderer, RendererError, UniformBuffer,
        Vertex, VertexBuffer, Viewport,
    };

    // Memory optimization
    pub use crate::memory::{
        ArenaHandle, ArenaId, ArenaStats, BufferPool, GeometryArena, GeometryPool, MultiArena,
        ObjectPool, PointPool, PoolStats, SharedPool, TypedArena, VectorPool,
    };

    pub use serde_json::json;
    pub use serde_json::Value;
}

/// 库版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 库名称
pub const NAME: &str = env!("CARGO_PKG_NAME");
