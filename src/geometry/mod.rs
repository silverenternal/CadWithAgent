//! 几何模块
//!
//! 提供基础图元定义、测量工具、变换工具、缓存层、NURBS 几何和约束求解器

pub mod boolean;
pub mod constraint;
pub mod constraint3d;
pub mod constraint_sparse;
pub mod generic_solver; // 新增：通用约束求解器 trait 和共享实现
pub mod geometry_cache;
pub mod geometry_error;
pub mod measure;
pub mod numerics;
pub mod nurbs;
pub mod parametric; // 参数化编辑模块
pub mod primitives;
pub mod simd;
pub mod soa;
pub mod transform;

#[cfg(test)]
mod measure_tests;
#[cfg(test)]
mod prop_tests;
#[cfg(test)]
mod transform_tests;

pub use constraint::*;
pub use constraint3d::*;
pub use constraint_sparse::*;
pub use generic_solver::*; // 导出通用求解器
pub use geometry_cache::*;
pub use geometry_error::*;
pub use measure::*;
pub use numerics::*;
pub use nurbs::*;
pub use parametric::*; // 导出参数化编辑
pub use primitives::*;
pub use soa::*;
pub use transform::*;

// 导出高级几何算法
pub use measure::{
    closest_point_on_segment, convex_hull, line_intersection, point_in_polygon,
    point_to_segment_distance, polygon_centroid, vector_angle, LineIntersection,
};
