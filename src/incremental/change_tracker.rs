//! Change Tracker - Record and manage geometric changes
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
//!
//! This module provides change tracking capabilities for incremental updates.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use uuid::Uuid;

/// Unique identifier for a change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId(pub Uuid);

impl ChangeId {
    /// Generate a new unique change ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ChangeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of change
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Entity was added
    Added,
    /// Entity was removed
    Removed,
    /// Entity geometry was modified
    GeometryModified,
    /// Entity parameter was changed
    ParameterChanged { name: String },
    /// Entity transform was modified
    TransformModified,
    /// Entity visibility changed
    VisibilityChanged,
    /// Entity material changed
    MaterialChanged,
    /// Entity suppression state changed
    SuppressionChanged,
}

/// Represents a change to an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Unique change identifier
    pub id: ChangeId,
    /// ID of the entity that changed
    pub entity_id: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Timestamp of the change (milliseconds since epoch)
    pub timestamp: u64,
    /// Optional old value (for undo support)
    pub old_value: Option<serde_json::Value>,
    /// Optional new value (for redo support)
    pub new_value: Option<serde_json::Value>,
    /// Optional description of the change
    pub description: Option<String>,
}

impl Change {
    /// Create a new change
    pub fn new(
        entity_id: impl Into<String>,
        change_type: ChangeType,
        old_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
    ) -> Self {
        // 获取时间戳（系统时间几乎不可能早于 UNIX epoch，除非时钟被篡改）
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_millis() as u64;

        Self {
            id: ChangeId::new(),
            entity_id: entity_id.into(),
            change_type,
            timestamp,
            old_value,
            new_value,
            description: None,
        }
    }

    /// Create an entity addition change
    pub fn added(entity_id: impl Into<String>, value: Option<serde_json::Value>) -> Self {
        Self::new(entity_id, ChangeType::Added, None, value)
    }

    /// Create an entity removal change
    pub fn removed(entity_id: impl Into<String>, old_value: Option<serde_json::Value>) -> Self {
        Self::new(entity_id, ChangeType::Removed, old_value, None)
    }

    /// Create a geometry modification change
    pub fn geometry_modified(
        entity_id: impl Into<String>,
        old_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            entity_id,
            ChangeType::GeometryModified,
            old_value,
            new_value,
        )
    }

    /// Create a parameter change
    pub fn parameter_changed(
        entity_id: impl Into<String>,
        param_name: impl Into<String>,
        old_value: impl Into<serde_json::Value>,
        new_value: impl Into<serde_json::Value>,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_millis() as u64;

        Self {
            id: ChangeId::new(),
            entity_id: entity_id.into(),
            change_type: ChangeType::ParameterChanged {
                name: param_name.into(),
            },
            timestamp,
            old_value: Some(old_value.into()),
            new_value: Some(new_value.into()),
            description: None,
        }
    }

    /// Create a transform modification change
    pub fn transform_modified(
        entity_id: impl Into<String>,
        old_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
    ) -> Self {
        Self::new(
            entity_id,
            ChangeType::TransformModified,
            old_value,
            new_value,
        )
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Change record for undo/redo support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    /// The change
    pub change: Change,
    /// Whether the change has been applied
    pub applied: bool,
    /// Whether the change can be undone
    pub undoable: bool,
}

impl ChangeRecord {
    /// Create a new change record
    pub fn new(change: Change) -> Self {
        Self {
            change,
            applied: true,
            undoable: true,
        }
    }
}

/// Configuration for change tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTrackerConfig {
    /// Maximum number of changes to keep in history
    pub max_history_size: usize,
    /// Enable automatic change batching
    pub enable_batching: bool,
    /// Batch timeout in milliseconds
    pub batch_timeout_ms: u64,
}

impl Default for ChangeTrackerConfig {
    fn default() -> Self {
        Self {
            max_history_size: 1000,
            enable_batching: true,
            batch_timeout_ms: 100,
        }
    }
}

/// Change Tracker - Records and manages geometric changes
///
/// The change tracker maintains a history of all changes made to entities,
/// enabling undo/redo functionality and incremental updates.
///
/// # Examples
///
/// ```
/// use cadagent::incremental::{ChangeTracker, Change};
///
/// let mut tracker = ChangeTracker::new();
///
/// // Record changes
/// tracker.record_change(Change::added("entity_1", None));
/// tracker.record_change(Change::geometry_modified("entity_2", None, None));
///
/// // Get recent changes
/// let changes = tracker.get_recent_changes(10);
/// assert_eq!(changes.len(), 2);
///
/// // Undo last change
/// if let Some(undo_change) = tracker.undo() {
///     println!("Undone: {:?}", undo_change);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeTracker {
    /// Change history
    history: VecDeque<ChangeRecord>,
    /// Redo stack (for redo after undo)
    redo_stack: VecDeque<ChangeRecord>,
    /// Configuration
    config: ChangeTrackerConfig,
    /// Pending batched changes
    pending_batch: Vec<Change>,
    /// Whether currently in a batch operation
    in_batch: bool,
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeTracker {
    /// Create a new change tracker with default configuration
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(100),
            redo_stack: VecDeque::new(),
            config: ChangeTrackerConfig::default(),
            pending_batch: Vec::new(),
            in_batch: false,
        }
    }

    /// Create a change tracker with custom configuration
    pub fn with_config(config: ChangeTrackerConfig) -> Self {
        Self {
            history: VecDeque::with_capacity(config.max_history_size.min(100)),
            redo_stack: VecDeque::new(),
            config,
            pending_batch: Vec::new(),
            in_batch: false,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ChangeTrackerConfig {
        &self.config
    }

    /// Record a change
    ///
    /// # Arguments
    /// * `change` - The change to record
    ///
    /// # Returns
    /// The change ID for reference
    pub fn record_change(&mut self, change: Change) -> ChangeId {
        let change_id = change.id;

        if self.config.enable_batching && self.in_batch {
            // Add to pending batch
            self.pending_batch.push(change);
        } else {
            // Record immediately
            let record = ChangeRecord::new(change);
            self.history.push_back(record);

            // Clear redo stack on new change
            self.redo_stack.clear();

            // Trim history if needed
            while self.history.len() > self.config.max_history_size {
                self.history.pop_front();
            }
        }

        change_id
    }

    /// Start a batch operation
    ///
    /// Changes recorded during a batch will be grouped together
    pub fn start_batch(&mut self) {
        self.in_batch = true;
        self.pending_batch.clear();
    }

    /// End a batch operation
    ///
    /// All pending changes are recorded as a single group
    pub fn end_batch(&mut self) -> Vec<ChangeId> {
        self.in_batch = false;

        let ids: Vec<ChangeId> = self
            .pending_batch
            .drain(..)
            .map(|change| {
                let id = change.id;
                let record = ChangeRecord::new(change);
                self.history.push_back(record);
                id
            })
            .collect();

        // Clear redo stack on batch commit
        self.redo_stack.clear();

        // Trim history
        while self.history.len() > self.config.max_history_size {
            self.history.pop_front();
        }

        ids
    }

    /// Cancel a batch operation
    pub fn cancel_batch(&mut self) {
        self.in_batch = false;
        self.pending_batch.clear();
    }

    /// Get the number of recorded changes
    pub fn change_count(&self) -> usize {
        self.history.len()
    }

    /// Get recent changes
    ///
    /// # Arguments
    /// * `count` - Maximum number of changes to return (most recent first)
    pub fn get_recent_changes(&self, count: usize) -> Vec<&ChangeRecord> {
        self.history.iter().rev().take(count).collect()
    }

    /// Get all changes
    pub fn get_all_changes(&self) -> &VecDeque<ChangeRecord> {
        &self.history
    }

    /// Undo the last change
    ///
    /// # Returns
    /// The undone change record, if any
    pub fn undo(&mut self) -> Option<&ChangeRecord> {
        if let Some(mut record) = self.history.pop_back() {
            if record.undoable {
                record.applied = false;
                self.redo_stack.push_back(record);
                return self.redo_stack.back();
            }
        }
        None
    }

    /// Redo the last undone change
    ///
    /// # Returns
    /// The redone change record, if any
    pub fn redo(&mut self) -> Option<&ChangeRecord> {
        if let Some(mut record) = self.redo_stack.pop_back() {
            record.applied = true;
            self.history.push_back(record);
            return self.history.back();
        }
        None
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.history.iter().rev().any(|r| r.undoable)
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clear all change history
    pub fn clear(&mut self) {
        self.history.clear();
        self.redo_stack.clear();
        self.pending_batch.clear();
    }

    /// Get changes affecting a specific entity
    ///
    /// # Arguments
    /// * `entity_id` - The entity ID to search for
    pub fn get_changes_for_entity(&self, entity_id: &str) -> Vec<&ChangeRecord> {
        self.history
            .iter()
            .filter(|r| r.change.entity_id == entity_id)
            .collect()
    }

    /// Get the latest change for an entity
    pub fn get_latest_change_for_entity(&self, entity_id: &str) -> Option<&ChangeRecord> {
        self.history
            .iter()
            .rev()
            .find(|r| r.change.entity_id == entity_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_creation() {
        let change = Change::added("test_entity", Some(serde_json::json!({"type": "test"})));
        assert_eq!(change.entity_id, "test_entity");
        assert!(matches!(change.change_type, ChangeType::Added));
    }

    #[test]
    fn test_change_parameter_changed() {
        let change = Change::parameter_changed("entity_1", "depth", 10.0, 20.0);
        assert_eq!(change.entity_id, "entity_1");
        if let ChangeType::ParameterChanged { name } = &change.change_type {
            assert_eq!(name, "depth");
        } else {
            panic!("Expected ParameterChanged variant");
        }
    }

    #[test]
    fn test_change_with_description() {
        let change = Change::added("test", None).with_description("Test description");
        assert_eq!(change.description, Some("Test description".to_string()));
    }

    #[test]
    fn test_change_tracker_new() {
        let tracker = ChangeTracker::new();
        assert_eq!(tracker.change_count(), 0);
        assert!(!tracker.can_undo());
        assert!(!tracker.can_redo());
    }

    #[test]
    fn test_change_tracker_record() {
        let mut tracker = ChangeTracker::new();
        let change = Change::added("entity_1", None);
        tracker.record_change(change);

        assert_eq!(tracker.change_count(), 1);
        assert!(tracker.can_undo());
    }

    #[test]
    fn test_change_tracker_undo_redo() {
        let mut tracker = ChangeTracker::new();

        tracker.record_change(Change::added("entity_1", None));
        tracker.record_change(Change::added("entity_2", None));

        assert_eq!(tracker.change_count(), 2);

        // Undo
        tracker.undo();
        assert_eq!(tracker.change_count(), 1);
        assert!(tracker.can_redo());

        // Redo
        tracker.redo();
        assert_eq!(tracker.change_count(), 2);
    }

    #[test]
    fn test_change_tracker_batch() {
        let mut tracker = ChangeTracker::new();

        tracker.start_batch();
        tracker.record_change(Change::added("entity_1", None));
        tracker.record_change(Change::added("entity_2", None));
        tracker.record_change(Change::added("entity_3", None));

        // Should not be recorded yet
        assert_eq!(tracker.change_count(), 0);

        let ids = tracker.end_batch();
        assert_eq!(ids.len(), 3);
        assert_eq!(tracker.change_count(), 3);
    }

    #[test]
    fn test_change_tracker_cancel_batch() {
        let mut tracker = ChangeTracker::new();

        tracker.start_batch();
        tracker.record_change(Change::added("entity_1", None));
        tracker.cancel_batch();

        assert_eq!(tracker.change_count(), 0);
    }

    #[test]
    fn test_change_tracker_get_changes_for_entity() {
        let mut tracker = ChangeTracker::new();

        tracker.record_change(Change::added("entity_1", None));
        tracker.record_change(Change::geometry_modified("entity_2", None, None));
        tracker.record_change(Change::parameter_changed("entity_1", "width", 5.0, 10.0));

        let changes = tracker.get_changes_for_entity("entity_1");
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn test_change_tracker_clear() {
        let mut tracker = ChangeTracker::new();

        tracker.record_change(Change::added("entity_1", None));
        tracker.record_change(Change::added("entity_2", None));

        tracker.clear();

        assert_eq!(tracker.change_count(), 0);
        assert!(!tracker.can_undo());
    }

    #[test]
    fn test_change_tracker_history_limit() {
        let config = ChangeTrackerConfig {
            max_history_size: 5,
            ..Default::default()
        };
        let mut tracker = ChangeTracker::with_config(config);

        // Add 10 changes
        for i in 0..10 {
            tracker.record_change(Change::added(format!("entity_{}", i), None));
        }

        // Should only keep last 5
        assert_eq!(tracker.change_count(), 5);
    }

    #[test]
    fn test_change_id_generation() {
        let id1 = ChangeId::new();
        let id2 = ChangeId::new();
        assert_ne!(id1, id2);
    }
}
