//! 评估指标模块
//!
//! 提供几何一致性、IoU 等评估指标

pub mod consistency;
pub mod iou;

pub use consistency::*;
pub use iou::*;
