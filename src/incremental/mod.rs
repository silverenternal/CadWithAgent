//! Incremental Update System
//!
//! This module provides dependency tracking and incremental update capabilities for
//! efficient large-scale model modifications. Features include:
//!
//! - **Change Tracking**: Record and manage geometric changes
//! - **Dependency Graph**: Track entity dependencies for impact analysis
//! - **Incremental Rebuild**: Update only affected entities instead of full rebuild
//!
//! # Architecture
//!
//! ```text
//! Incremental Update Flow:
//!
//!     ┌──────────────┐
//!     │  Change      │ (User modifies entity)
//!     └──────┬───────┘
//!            │
//!            ▼
//!     ┌──────────────┐
//!     │ ChangeTracker│ (Records change, identifies affected entities)
//!     └──────┬───────┘
//!            │
//!            ▼
//!     ┌──────────────┐
//!     │DependencyGraph│ (Traverses dependencies, finds all impacted entities)
//!     └──────┬───────┘
//!            │
//!            ▼
//!     ┌──────────────┐
//!     │IncrementalUpdate│ (Updates only affected entities)
//!     └──────────────┘
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use cadagent::incremental::{ChangeTracker, DependencyGraph, Change, ChangeType};
//! use cadagent::feature::FeatureId;
//!
//! // Create change tracker
//! let mut tracker = ChangeTracker::new();
//!
//! // Record a change
//! let feature_id = FeatureId::new_v4();
//! tracker.record_change(Change::parameter_changed(feature_id, "depth", 10.0, 20.0));
//!
//! // Build dependency graph
//! let mut graph = DependencyGraph::new();
//! graph.add_dependency(feature_id, FeatureId::new_v4());
//!
//! // Get affected entities
//! let affected = tracker.get_affected_entities(&graph, feature_id);
//! ```

pub mod change_tracker;
pub mod dependency_graph;
pub mod updater;

pub use change_tracker::*;
pub use dependency_graph::*;
pub use updater::*;
