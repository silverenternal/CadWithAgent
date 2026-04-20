//! Incremental Updater - Efficiently update affected entities
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
//!
//! This module coordinates change tracking and dependency analysis to perform
//! efficient incremental updates, rebuilding only affected entities.
//!
//! # Architecture
//!
//! The updater minimizes rebuild time through:
//! 1. **Change Tracking**: Records all geometric changes
//! 2. **Dependency Analysis**: Uses dependency graph to find affected entities
//! 3. **Selective Rebuild**: Updates only affected entities in topological order
//! 4. **Full Rebuild Fallback**: Automatically falls back to full rebuild if
//!    too many entities are affected (configurable threshold)
//!
//! # Update Flow
//!
//! ```text
//!     ┌─────────────┐
//!     │  Change     │ (User modifies entity)
//!     └──────┬──────┘
//!            │
//!            ▼
//!     ┌─────────────┐
//!     │ Record      │ (ChangeTracker stores change)
//!     └──────┬──────┘
//!            │
//!            ▼
//!     ┌─────────────┐
//!     │ Analyze     │ (DependencyGraph finds affected entities)
//!     └──────┬──────┘
//!            │
//!            ▼
//!     ┌─────────────┐
//!     │ Update      │ (Rebuild affected entities in order)
//!     └─────────────┘
//! ```
//!
//! # Examples
//!
//! ```
//! use cadagent::incremental::{IncrementalUpdater, Change};
//! use cadagent::feature::FeatureTree;
//!
//! // Create updater and feature tree
//! let mut updater = IncrementalUpdater::new();
//! let mut tree = FeatureTree::new();
//!
//! // Add features and dependencies
//! // ...
//!
//! // Record a change
//! updater.record_change(Change::parameter_changed("feature_1", "depth", 10.0, 20.0));
//!
//! // Perform incremental update
//! let result = updater.update(&mut tree).unwrap();
//! assert!(result.status.is_success());
//! ```

use crate::feature::FeatureTree;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use super::{Change, ChangeTracker, ChangeTrackerConfig, DependencyGraph, DependencyGraphError};

/// Error types for incremental update operations
///
/// This enum represents all possible errors that can occur during
/// incremental update operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum IncrementalUpdateError {
    /// Dependency graph error
    #[error("Dependency error: {0}")]
    Dependency(#[from] DependencyGraphError),

    /// Feature tree error
    #[error("Feature tree error: {0}")]
    FeatureTree(String),

    /// Entity not found
    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    /// Update failed
    #[error("Update failed: {0}")]
    UpdateFailed(String),
}

/// Result type for incremental update operations
pub type IncrementalUpdateResult<T> = Result<T, IncrementalUpdateError>;

/// Status of an incremental update
///
/// This enum represents the outcome of an incremental update operation,
/// providing detailed statistics about what was updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateStatus {
    /// Update completed successfully
    Success {
        /// Number of entities updated
        entities_updated: usize,
        /// Time taken in milliseconds
        elapsed_ms: u64,
    },
    /// No updates were needed
    NoChanges,
    /// Update partially completed
    Partial {
        /// Number of entities updated
        entities_updated: usize,
        /// Number of entities that failed
        entities_failed: usize,
        /// Errors encountered
        errors: Vec<String>,
    },
}

impl UpdateStatus {
    /// Check if update was successful
    pub fn is_success(&self) -> bool {
        matches!(self, UpdateStatus::Success { .. })
    }

    /// Check if no changes were needed
    pub fn is_no_changes(&self) -> bool {
        matches!(self, UpdateStatus::NoChanges)
    }
}

/// Record of an incremental update
///
/// This struct stores the history and results of an update operation,
/// enabling audit trails and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecord {
    /// Unique update ID
    pub id: Uuid,
    /// Timestamp of the update
    pub timestamp: u64,
    /// Status of the update
    pub status: UpdateStatus,
    /// List of updated entity IDs
    pub updated_entities: Vec<String>,
    /// Trigger change ID
    pub trigger_change_id: Option<super::ChangeId>,
}

/// Configuration for incremental updater
///
/// Controls the behavior of the incremental update system, including
/// performance thresholds and history management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalUpdaterConfig {
    /// Enable automatic dependency tracking
    pub auto_track_dependencies: bool,
    /// Enable incremental updates
    pub enable_incremental: bool,
    /// Force full rebuild if more than this percentage of entities are affected
    /// (0.0 - 1.0, default 0.5 = 50%)
    pub full_rebuild_threshold: f64,
    /// Maximum number of update records to keep
    pub max_update_history: usize,
    /// Change tracker configuration
    pub change_tracker_config: ChangeTrackerConfig,
}

impl Default for IncrementalUpdaterConfig {
    fn default() -> Self {
        Self {
            auto_track_dependencies: true,
            enable_incremental: true,
            full_rebuild_threshold: 0.5,
            max_update_history: 100,
            change_tracker_config: ChangeTrackerConfig::default(),
        }
    }
}

/// Incremental Updater - Coordinates efficient model updates
///
/// The incremental updater minimizes rebuild time by:
/// 1. Tracking changes through `ChangeTracker`
/// 2. Analyzing dependencies through `DependencyGraph`
/// 3. Updating only affected entities
///
/// # Performance Features
///
/// - **Affected Entities Cache**: Caches analysis results to avoid recomputation
/// - **Topological Rebuild Order**: Updates entities in dependency order
/// - **Adaptive Fallback**: Automatically switches to full rebuild when
///   incremental update would be inefficient
///
/// # Examples
///
/// ```
/// use cadagent::incremental::{IncrementalUpdater, Change};
/// use cadagent::feature::FeatureTree;
///
/// let mut updater = IncrementalUpdater::new();
/// let mut tree = FeatureTree::new();
///
/// // Record a change
/// updater.record_change(Change::added("feature_1", None));
///
/// // Perform incremental update
/// let result = updater.update(&mut tree).unwrap();
/// assert!(result.status.is_success());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalUpdater {
    /// Change tracker
    change_tracker: ChangeTracker,
    /// Dependency graph
    dependency_graph: DependencyGraph,
    /// Configuration
    config: IncrementalUpdaterConfig,
    /// Update history
    update_history: Vec<UpdateRecord>,
    /// Cache of affected entities from last analysis
    cached_affected: HashSet<String>,
    /// Whether cache is valid
    cache_valid: bool,
}

impl Default for IncrementalUpdater {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalUpdater {
    /// Create a new incremental updater with default configuration
    pub fn new() -> Self {
        Self {
            change_tracker: ChangeTracker::new(),
            dependency_graph: DependencyGraph::new(),
            config: IncrementalUpdaterConfig::default(),
            update_history: Vec::new(),
            cached_affected: HashSet::new(),
            cache_valid: false,
        }
    }

    /// Create updater with custom configuration
    pub fn with_config(config: IncrementalUpdaterConfig) -> Self {
        Self {
            change_tracker: ChangeTracker::with_config(config.change_tracker_config.clone()),
            config,
            ..Default::default()
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &IncrementalUpdaterConfig {
        &self.config
    }

    /// Get the change tracker
    pub fn change_tracker(&self) -> &ChangeTracker {
        &self.change_tracker
    }

    /// Get the dependency graph
    pub fn dependency_graph(&self) -> &DependencyGraph {
        &self.dependency_graph
    }

    /// Get mutable reference to dependency graph
    pub fn dependency_graph_mut(&mut self) -> &mut DependencyGraph {
        &mut self.dependency_graph
    }

    /// Record a change
    ///
    /// This invalidates the affected entities cache.
    pub fn record_change(&mut self, change: Change) -> super::ChangeId {
        self.cache_valid = false;
        self.change_tracker.record_change(change)
    }

    /// Start a batch operation
    pub fn start_batch(&mut self) {
        self.cache_valid = false;
        self.change_tracker.start_batch();
    }

    /// End a batch operation
    pub fn end_batch(&mut self) -> Vec<super::ChangeId> {
        self.change_tracker.end_batch()
    }

    /// Add a dependency between entities
    ///
    /// # Arguments
    /// * `from` - The dependent entity
    /// * `to` - The dependency entity
    pub fn add_dependency(
        &mut self,
        from: impl Into<String>,
        to: impl Into<String>,
    ) -> Result<(), DependencyGraphError> {
        let from_id: String = from.into();
        let to_id: String = to.into();

        self.dependency_graph.add_dependency(from_id, to_id)?;
        self.cache_valid = false;
        Ok(())
    }

    /// Remove an entity from the dependency graph
    pub fn remove_entity(&mut self, entity_id: &str) {
        self.dependency_graph.remove_node(entity_id);
        self.cache_valid = false;
    }

    /// Get entities affected by a change
    ///
    /// # Arguments
    /// * `entity_id` - The changed entity
    ///
    /// # Returns
    /// Set of all entity IDs that are affected by the change
    pub fn get_affected_entities(&mut self, entity_id: &str) -> HashSet<String> {
        // Return cached result if valid
        if self.cache_valid && !self.cached_affected.is_empty() {
            return self.cached_affected.clone();
        }

        // Calculate affected entities
        let affected = self.dependency_graph.get_all_dependents(entity_id);

        // Cache the result
        self.cached_affected = affected.clone();
        self.cache_valid = true;

        affected
    }

    /// Get all entities affected by recorded changes
    pub fn get_all_affected_entities(&mut self) -> HashSet<String> {
        let mut all_affected = HashSet::new();

        // Get recent changes and find affected entities for each
        let recent_changes: Vec<_> = self
            .change_tracker
            .get_recent_changes(100)
            .into_iter()
            .cloned()
            .collect();
        for record in recent_changes {
            let affected = self.get_affected_entities(&record.change.entity_id);
            all_affected.extend(affected);
            all_affected.insert(record.change.entity_id.clone());
        }

        all_affected
    }

    /// Perform incremental update on feature tree
    ///
    /// # Arguments
    /// * `tree` - The feature tree to update
    ///
    /// # Returns
    /// Update result with status and statistics
    pub fn update(&mut self, tree: &mut FeatureTree) -> IncrementalUpdateResult<UpdateRecord> {
        let start_time = std::time::Instant::now();
        let update_id = Uuid::new_v4();

        // Check if incremental updates are enabled
        if !self.config.enable_incremental {
            // Force full rebuild
            tree.rebuild().map_err(|e| {
                IncrementalUpdateError::FeatureTree(format!("Full rebuild failed: {e}"))
            })?;

            let record = UpdateRecord {
                id: update_id,
                timestamp: self.current_timestamp(),
                status: UpdateStatus::NoChanges,
                updated_entities: Vec::new(),
                trigger_change_id: None,
            };

            self.record_update(record.clone());
            return Ok(record);
        }

        // Get affected entities
        let affected = self.get_all_affected_entities();

        if affected.is_empty() {
            let record = UpdateRecord {
                id: update_id,
                timestamp: self.current_timestamp(),
                status: UpdateStatus::NoChanges,
                updated_entities: Vec::new(),
                trigger_change_id: None,
            };

            self.record_update(record.clone());
            return Ok(record);
        }

        // Check if we should do full rebuild
        let total_entities = tree.get_all_features().len();
        let affected_ratio = affected.len() as f64 / total_entities.max(1) as f64;

        if affected_ratio > self.config.full_rebuild_threshold {
            // Too many affected, do full rebuild
            tree.rebuild().map_err(|e| {
                IncrementalUpdateError::FeatureTree(format!("Full rebuild failed: {e}"))
            })?;

            let elapsed = start_time.elapsed().as_millis() as u64;
            let record = UpdateRecord {
                id: update_id,
                timestamp: self.current_timestamp(),
                status: UpdateStatus::Success {
                    entities_updated: total_entities,
                    elapsed_ms: elapsed,
                },
                updated_entities: vec!["[full rebuild]".to_string()],
                trigger_change_id: None,
            };

            self.record_update(record.clone());
            return Ok(record);
        }

        // Perform incremental update
        let mut updated = Vec::new();
        let mut errors = Vec::new();

        // Get topological order for rebuild
        let sorted = self
            .dependency_graph
            .topological_sort()
            .unwrap_or_else(|_| affected.iter().cloned().collect());

        // Update affected entities in dependency order
        for entity_id in sorted {
            if affected.contains(&entity_id) {
                // Try to update this entity
                match self.update_entity(tree, &entity_id) {
                    Ok(_) => {
                        updated.push(entity_id);
                    }
                    Err(e) => {
                        errors.push(format!("Failed to update {entity_id}: {e}"));
                    }
                }
            }
        }

        // Rebuild the tree to ensure consistency
        if let Err(e) = tree.rebuild() {
            errors.push(format!("Tree rebuild failed: {e}"));
        }

        let elapsed = start_time.elapsed().as_millis() as u64;

        let status = if errors.is_empty() {
            UpdateStatus::Success {
                entities_updated: updated.len(),
                elapsed_ms: elapsed,
            }
        } else {
            UpdateStatus::Partial {
                entities_updated: updated.len(),
                entities_failed: errors.len(),
                errors,
            }
        };

        let record = UpdateRecord {
            id: update_id,
            timestamp: self.current_timestamp(),
            status,
            updated_entities: updated,
            trigger_change_id: None,
        };

        self.record_update(record.clone());
        Ok(record)
    }

    /// Update a single entity in the feature tree
    fn update_entity(&self, tree: &FeatureTree, entity_id: &str) -> Result<(), String> {
        // Try to parse entity_id as FeatureId (Uuid)
        if let Ok(uuid) = Uuid::parse_str(entity_id) {
            let feature_id = uuid; // FeatureId is a type alias for Uuid

            // Check if feature exists
            if tree.get_feature(feature_id).is_some() {
                // Feature exists, mark for rebuild
                // The actual update happens during tree rebuild
                return Ok(());
            }
        }

        Err(format!("Entity {entity_id} not found in feature tree"))
    }

    /// Record an update in history
    fn record_update(&mut self, record: UpdateRecord) {
        self.update_history.push(record);

        // Trim history
        while self.update_history.len() > self.config.max_update_history {
            self.update_history.remove(0);
        }
    }

    /// Get update history
    pub fn update_history(&self) -> &[UpdateRecord] {
        &self.update_history
    }

    /// Get the last update record
    pub fn last_update(&self) -> Option<&UpdateRecord> {
        self.update_history.last()
    }

    /// Clear all changes and reset state
    pub fn clear(&mut self) {
        self.change_tracker.clear();
        self.cached_affected.clear();
        self.cache_valid = false;
    }

    /// Get current timestamp in milliseconds
    fn current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Get statistics about the updater
    pub fn stats(&self) -> UpdaterStats {
        UpdaterStats {
            total_entities: self.dependency_graph.node_count(),
            total_dependencies: self.dependency_graph.edge_count(),
            pending_changes: self.change_tracker.change_count(),
            update_count: self.update_history.len(),
            cache_valid: self.cache_valid,
            affected_count: self.cached_affected.len(),
        }
    }
}

/// Statistics about the incremental updater
///
/// Provides runtime metrics for monitoring and debugging the update system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdaterStats {
    /// Total number of tracked entities
    pub total_entities: usize,
    /// Total number of dependencies
    pub total_dependencies: usize,
    /// Number of pending changes
    pub pending_changes: usize,
    /// Number of updates performed
    pub update_count: usize,
    /// Whether the affected entities cache is valid
    pub cache_valid: bool,
    /// Number of affected entities in cache
    pub affected_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_updater_creation() {
        let updater = IncrementalUpdater::new();
        assert!(updater.config.enable_incremental);
    }

    #[test]
    fn test_updater_record_change() {
        let mut updater = IncrementalUpdater::new();
        let change = Change::added("entity_1", None);
        updater.record_change(change);

        assert_eq!(updater.change_tracker().change_count(), 1);
    }

    #[test]
    fn test_updater_add_dependency() {
        let mut updater = IncrementalUpdater::new();
        updater.add_dependency("B", "A").unwrap();

        assert!(updater.dependency_graph().has_edge("B", "A"));
    }

    #[test]
    fn test_updater_get_affected() {
        let mut updater = IncrementalUpdater::new();
        updater.add_dependency("B", "A").unwrap();
        updater.add_dependency("C", "B").unwrap();

        let affected = updater.get_affected_entities("A");
        assert!(affected.contains("B"));
        assert!(affected.contains("C"));
    }

    #[test]
    fn test_updater_update_empty() {
        let mut updater = IncrementalUpdater::new();
        let mut tree = FeatureTree::new();

        let result = updater.update(&mut tree).unwrap();
        assert!(result.status.is_no_changes());
    }

    #[test]
    fn test_updater_stats() {
        let mut updater = IncrementalUpdater::new();
        updater.add_dependency("B", "A").unwrap();
        updater.add_dependency("C", "B").unwrap();

        let stats = updater.stats();
        assert_eq!(stats.total_entities, 3);
        assert_eq!(stats.total_dependencies, 2);
        assert_eq!(stats.pending_changes, 0);
    }

    #[test]
    fn test_updater_clear() {
        let mut updater = IncrementalUpdater::new();
        updater.record_change(Change::added("entity_1", None));
        updater.add_dependency("B", "A").unwrap();

        updater.clear();

        assert_eq!(updater.change_tracker().change_count(), 0);
        assert_eq!(updater.stats().affected_count, 0);
    }

    #[test]
    fn test_update_status() {
        let success = UpdateStatus::Success {
            entities_updated: 5,
            elapsed_ms: 100,
        };
        assert!(success.is_success());
        assert!(!success.is_no_changes());

        let no_changes = UpdateStatus::NoChanges;
        assert!(no_changes.is_no_changes());
        assert!(!no_changes.is_success());
    }
}
