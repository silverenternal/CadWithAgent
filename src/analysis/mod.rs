//! 统一的几何分析管线模块
//!
//! 整合 CAD 基元提取、几何推理、约束校验和提示词构造，
//! 实现完整的"工具增强上下文注入"范式。
//!
//! # 模块定位
//!
//! 本模块是**确定性几何分析工具**，提供：
//! - 标准化的几何分析流程
//! - 结构化的几何数据输出
//! - 可被 LLM 调用的 tokitai 工具接口
//!
//! # 处理流程
//!
//! ```text
//! 输入 CAD 图纸 → 基元提取 → 几何关系推理 → 约束校验 → 结构化提示词 → VLM → 推理思维链
//! ```
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::analysis::{AnalysisPipeline, AnalysisConfig};
//!
//! let config = AnalysisConfig::default();
//! let pipeline = AnalysisPipeline::new(config).unwrap();
//!
//! // 从 SVG 字符串注入上下文
//! let svg = r#"<svg>...</svg>"#;
//! let result = pipeline.inject_from_svg_string(svg, "分析这个图形").unwrap();
//!
//! println!("提示词长度：{}", result.prompt.full_prompt.len());
//! println!("工具调用链：{:?}", result.tool_call_chain);
//! ```

pub mod pipeline;
pub mod types;
pub mod tools;

pub use pipeline::*;
pub use types::*;
pub use tools::*;
