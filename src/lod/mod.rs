//! Level of Detail (LOD) System
//!
//! This module provides performance optimization for large-scale CAD models through:
//!
//! - **LOD Management**: Distance-based automatic LOD switching
//! - **Mesh Simplification**: Vertex clustering and edge collapse algorithms
//! - **Adaptive Tessellation**: Resolution adjustment based on viewing distance
//!
//! # Architecture
//!
//! ```text
//! LOD System Flow:
//!
//!     ┌──────────────────┐
//!     │  Original Mesh   │ (High precision, 10K+ faces)
//!     └────────┬─────────┘
//!              │
//!              ▼
//!     ┌──────────────────┐
//!     │  LOD Manager     │ (Calculates distance, selects LOD level)
//!     └────────┬─────────┘
//!              │
//!       ┌───────┴───────┬───────────┐
//!       ▼               ▼           ▼
//! ┌──────────┐   ┌──────────┐ ┌──────────┐
//! │ LOD High │   │ LOD Med  │ │ LOD Low  │
//! │ 100%     │   │ 50%      │ │ 10%      │
//! └──────────┘   └──────────┘ └──────────┘
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use cadagent::lod::{LodManager, LodLevel, MeshSimplifier};
//! use cadagent::geometry::nurbs::Mesh;
//!
//! // Create LOD manager with custom distance thresholds
//! let mut lod_manager = LodManager::new();
//! lod_manager.set_lod_distances(10.0, 50.0, 100.0);
//!
//! // Get appropriate LOD level based on camera distance
//! let lod_level = lod_manager.get_lod_level(75.0);
//! assert_eq!(lod_level, LodLevel::Medium);
//!
//! // Simplify mesh according to LOD level
//! let simplifier = MeshSimplifier::new();
//! let original_mesh = Mesh::new(); // Your mesh here
//! let simplified = simplifier.simplify(&original_mesh, lod_level);
//! ```

pub mod lod_manager;
pub mod simplification;

pub use lod_manager::*;
pub use simplification::*;
