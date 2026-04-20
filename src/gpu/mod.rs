//! GPU acceleration module for `CadAgent`
//!
//! This module provides GPU-accelerated compute and rendering capabilities using wgpu.
//! It includes:
//! - Compute pipelines for parallel geometry operations
//! - Rendering pipelines for high-performance visualization
//! - Buffer management for efficient GPU memory usage
//! - NURBS curve and surface evaluation on GPU
//!
//! # Example
//!
//! ```no_run
//! use cadagent::gpu::{GpuContext, ComputePipeline, Renderer};
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let ctx = GpuContext::new().await?;
//!     let compute = ComputePipeline::new(&ctx);
//!     let renderer = Renderer::new(&ctx);
//!     Ok(())
//! }
//! ```

pub mod buffers;
pub mod compute;
pub mod nurbs;
pub mod renderer;

pub use buffers::*;
pub use compute::*;
pub use nurbs::*;
pub use renderer::*;
