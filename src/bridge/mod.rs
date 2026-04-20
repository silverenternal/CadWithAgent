//! 桥接模块
//!
//! 提供与 VLM（视觉语言模型）的输入输出桥接

pub mod serializer;
pub mod vlm_client;
pub mod zaza_client;

pub use serializer::*;
pub use vlm_client::*;
pub use zaza_client::*;
