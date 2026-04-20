//! Memory optimization module for `CadAgent`
//!
//! This module provides efficient memory management strategies for large-scale
//! CAD geometry processing, including:
//! - `GeometryArena`: Bump allocator for fast geometry allocation
//! - Object pools for frequently allocated geometry entities
//! - Out-of-core memory management for very large models
//!
//! # Example
//!
//! ```
//! use cadagent::memory::{GeometryArena, ObjectPool};
//!
//! // Create an arena for fast allocation
//! let mut arena = GeometryArena::new();
//! let point_id = arena.alloc(nalgebra::Point3::new(1.0, 2.0, 3.0));
//!
//! // Create a pool for reusable objects
//! let mut pool = ObjectPool::<Vec<f32>>::new(100);
//! let vec = pool.acquire();
//! ```

pub mod arena;
pub mod pool;

pub use arena::*;
pub use pool::*;
