//! Task Types
//!
//! Defines task-related data structures for the task planner.

use crate::context::utils::generate_id;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task status representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is waiting to be executed
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed with error
    Failed,
    /// Task was skipped (dependency failed)
    Skipped,
    /// Task was cancelled by user
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "Pending"),
            TaskStatus::InProgress => write!(f, "InProgress"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed => write!(f, "Failed"),
            TaskStatus::Skipped => write!(f, "Skipped"),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Task node in the planning DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    /// Unique task ID
    pub id: String,
    /// Task name
    pub name: String,
    /// Task description
    pub description: String,
    /// Current task status
    pub status: TaskStatus,
    /// IDs of dependency tasks that must complete first
    pub dependencies: Vec<String>,
    /// Task result or error message
    pub result: Option<String>,
    /// Tool call chain used for this task
    pub tool_chain: Option<String>,
    /// Execution error (if failed)
    pub error: Option<String>,
    /// Priority (higher = more urgent)
    pub priority: u32,
    /// Estimated execution time in seconds
    pub estimated_time_secs: Option<u32>,
    /// Actual execution time in seconds
    pub actual_time_secs: Option<u32>,
    /// Retry count (for failed tasks)
    pub retry_count: u32,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl TaskNode {
    /// Create a new task node
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: generate_id(),
            name: name.to_string(),
            description: description.to_string(),
            status: TaskStatus::Pending,
            dependencies: Vec::new(),
            result: None,
            tool_chain: None,
            error: None,
            priority: 0,
            estimated_time_secs: None,
            actual_time_secs: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// Set task dependencies
    pub fn with_dependencies(mut self, deps: Vec<&str>) -> Self {
        self.dependencies = deps.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set estimated execution time
    pub fn with_estimated_time(mut self, secs: u32) -> Self {
        self.estimated_time_secs = Some(secs);
        self
    }

    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Mark task as in progress
    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
    }

    /// Mark task as completed
    pub fn complete(&mut self, result: &str, tool_chain: Option<&str>) {
        self.status = TaskStatus::Completed;
        self.result = Some(result.to_string());
        self.tool_chain = tool_chain.map(|s| s.to_string());
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: &str) {
        self.status = TaskStatus::Failed;
        self.error = Some(error.to_string());
        self.retry_count += 1;
    }

    /// Mark task as skipped
    pub fn skip(&mut self) {
        self.status = TaskStatus::Skipped;
    }

    /// Check if task can be executed (all dependencies completed)
    pub fn is_ready(&self, task_map: &HashMap<String, &TaskNode>) -> bool {
        if self.status != TaskStatus::Pending {
            return false;
        }

        self.dependencies.iter().all(|dep_id| {
            task_map
                .get(dep_id)
                .map(|dep| dep.status == TaskStatus::Completed)
                .unwrap_or(false)
        })
    }

    /// Check if task should be skipped (any dependency failed)
    pub fn should_skip(&self, task_map: &HashMap<String, &TaskNode>) -> bool {
        if self.status != TaskStatus::Pending {
            return false;
        }

        self.dependencies.iter().any(|dep_id| {
            task_map
                .get(dep_id)
                .map(|dep| matches!(dep.status, TaskStatus::Failed | TaskStatus::Skipped))
                .unwrap_or(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_node_creation() {
        let task = TaskNode::new("test", "Test task");
        assert_eq!(task.name, "test");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(task.dependencies.is_empty());
    }

    #[test]
    fn test_task_with_dependencies() {
        let task = TaskNode::new("test", "Test task").with_dependencies(vec!["dep1", "dep2"]);
        assert_eq!(task.dependencies.len(), 2);
    }

    #[test]
    fn test_task_status_transitions() {
        let mut task = TaskNode::new("test", "Test task");

        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);

        task.complete("Done", None);
        assert_eq!(task.status, TaskStatus::Completed);
        assert_eq!(task.result, Some("Done".to_string()));
    }

    #[test]
    fn test_task_failure_and_retry() {
        let mut task = TaskNode::new("test", "Test task");

        task.fail("Error occurred");
        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.retry_count, 1);
    }

    #[test]
    fn test_task_is_ready() {
        let mut task_map = HashMap::new();

        let mut dep1 = TaskNode::new("dep1", "Dependency 1");
        dep1.status = TaskStatus::Completed;
        task_map.insert("dep1".to_string(), &dep1);

        let task = TaskNode::new("test", "Test task").with_dependencies(vec!["dep1"]);

        assert!(task.is_ready(&task_map));
    }

    #[test]
    fn test_task_is_not_ready() {
        let mut task_map = HashMap::new();

        let mut dep1 = TaskNode::new("dep1", "Dependency 1");
        dep1.status = TaskStatus::Pending;
        task_map.insert("dep1".to_string(), &dep1);

        let task = TaskNode::new("test", "Test task").with_dependencies(vec!["dep1"]);

        assert!(!task.is_ready(&task_map));
    }
}
