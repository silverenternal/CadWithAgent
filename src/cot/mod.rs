//! Geo-CoT 模块
//!
//! 几何思维链生成器，用于生成 AI 训练和推理数据

pub mod generator;
pub mod qa;
pub mod templates;

pub use generator::*;
pub use qa::*;
pub use templates::*;
