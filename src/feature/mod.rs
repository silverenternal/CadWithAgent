//! Feature Tree Module
//!
//! This module provides parametric modeling capabilities through a feature tree
//! data structure. It enables:
//!
//! - **Modeling History**: Track all modeling operations in a tree structure
//! - **Dependency Tracking**: Automatic tracking of feature dependencies
//! - **Undo/Redo**: Full undo/redo support with history management
//! - **Parametric Editing**: Modify feature parameters and rebuild
//! - **Feature Suppression**: Temporarily disable features without deletion
//!
//! # Architecture
//!
//! ```text
//! Feature Tree Structure:
//!
//!     ┌─────────────┐
//!     │   Sketch    │ (Root feature - no dependencies)
//!     └──────┬──────┘
//!            │
//!            ▼
//!     ┌─────────────┐
//!     │   Extrude   │ (Depends on Sketch)
//!     └──────┬──────┘
//!            │
//!            ▼
//!     ┌─────────────┐
//!     │   Fillet    │ (Depends on Extrude)
//!     └─────────────┘
//! ```
//!
//! # Examples
//!
//! ```
//! use cadagent::feature::{FeatureTree, Feature, Sketch, ExtrudeDirection};
//! use cadagent::geometry::Point;
//!
//! // Create a feature tree
//! let mut tree = FeatureTree::new();
//!
//! // Create a sketch
//! let mut sketch = Sketch::new();
//! let pt1 = sketch.add_point(Point::new(0.0, 0.0));
//! let pt2 = sketch.add_point(Point::new(1.0, 0.0));
//! sketch.add_line(pt1, pt2);
//!
//! // Add sketch to tree
//! let sketch_id = tree.add_feature(Feature::Sketch(sketch)).unwrap();
//!
//! // Add extrude feature
//! let extrude_id = tree.add_feature(Feature::Extrude {
//!     sketch_id,
//!     depth: 10.0,
//!     direction: ExtrudeDirection::Positive,
//!     taper_angle: None,
//! }).unwrap();
//!
//! // Rebuild the tree
//! let result = tree.rebuild().unwrap();
//! assert!(result.is_success());
//! ```

#![allow(clippy::module_inception)]

pub mod feature;
pub mod feature_tree;

pub use feature::*;
pub use feature_tree::*;
