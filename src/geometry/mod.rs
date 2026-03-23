//! 几何模块
//!
//! 提供基础图元定义、测量工具和变换工具

pub mod primitives;
pub mod measure;
pub mod transform;
pub mod boolean;

#[cfg(test)]
mod transform_tests;
#[cfg(test)]
mod measure_tests;
#[cfg(test)]
mod boolean_tests;

pub use primitives::*;
pub use measure::*;
pub use transform::*;
