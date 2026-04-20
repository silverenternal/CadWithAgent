//! Task Plan Types
//!
//! Defines task plan data structures.

use super::types::{TaskNode, TaskStatus};
use crate::context::utils::current_timestamp;
use serde::{Deserialize, Serialize};

/// Task plan status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    /// Plan is being drafted
    Draft,
    /// Plan is approved and ready to execute
    Approved,
    /// Plan is currently executing
    Executing,
    /// Plan execution paused
    Paused,
    /// Plan completed successfully
    Completed,
    /// Plan failed
    Failed,
    /// Plan cancelled
    Cancelled,
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanStatus::Draft => write!(f, "Draft"),
            PlanStatus::Approved => write!(f, "Approved"),
            PlanStatus::Executing => write!(f, "Executing"),
            PlanStatus::Paused => write!(f, "Paused"),
            PlanStatus::Completed => write!(f, "Completed"),
            PlanStatus::Failed => write!(f, "Failed"),
            PlanStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Task plan statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TaskPlanStats {
    /// Total number of tasks
    pub total_tasks: usize,
    /// Number of pending tasks
    pub pending_count: usize,
    /// Number of in-progress tasks
    pub in_progress_count: usize,
    /// Number of completed tasks
    pub completed_count: usize,
    /// Number of failed tasks
    pub failed_count: usize,
    /// Number of skipped tasks
    pub skipped_count: usize,
    /// Number of cancelled tasks
    pub cancelled_count: usize,
    /// Completion rate (0.0-1.0)
    pub completion_rate: f32,
}

impl std::fmt::Display for TaskPlanStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Task Plan Statistics:")?;
        writeln!(f, "  Total: {}", self.total_tasks)?;
        writeln!(
            f,
            "  Completed: {} ({:.1}%)",
            self.completed_count,
            self.completion_rate * 100.0
        )?;
        writeln!(f, "  In Progress: {}", self.in_progress_count)?;
        writeln!(f, "  Pending: {}", self.pending_count)?;
        writeln!(f, "  Failed: {}", self.failed_count)?;
        writeln!(f, "  Skipped: {}", self.skipped_count)
    }
}

/// Task plan representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Plan ID
    pub id: String,
    /// Plan name
    pub name: String,
    /// Plan description
    pub description: String,
    /// Tasks in the plan
    pub tasks: Vec<TaskNode>,
    /// Plan status
    pub status: PlanStatus,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
}

impl TaskPlan {
    /// Create a new task plan
    pub fn new(name: &str, description: &str) -> Self {
        let now = current_timestamp();

        Self {
            id: crate::context::utils::generate_id(),
            name: name.to_string(),
            description: description.to_string(),
            tasks: Vec::new(),
            status: PlanStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a task to the plan
    pub fn add_task(&mut self, task: TaskNode) {
        self.tasks.push(task);
        self.updated_at = current_timestamp();
    }

    /// Get a task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&TaskNode> {
        self.tasks.iter().find(|t| t.id == task_id)
    }

    /// Get a mutable task by ID
    pub fn get_task_mut(&mut self, task_id: &str) -> Option<&mut TaskNode> {
        self.tasks.iter_mut().find(|t| t.id == task_id)
    }

    /// Get next ready task
    pub fn get_next_ready_task(&self) -> Option<&TaskNode> {
        let task_map: HashMap<String, &TaskNode> =
            self.tasks.iter().map(|t| (t.id.clone(), t)).collect();

        self.tasks
            .iter()
            .filter(|t| t.is_ready(&task_map))
            .max_by_key(|t| t.priority)
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> TaskPlanStats {
        let mut stats = TaskPlanStats {
            total_tasks: self.tasks.len(),
            ..Default::default()
        };

        for task in &self.tasks {
            match task.status {
                TaskStatus::Pending => stats.pending_count += 1,
                TaskStatus::InProgress => stats.in_progress_count += 1,
                TaskStatus::Completed => stats.completed_count += 1,
                TaskStatus::Failed => stats.failed_count += 1,
                TaskStatus::Skipped => stats.skipped_count += 1,
                TaskStatus::Cancelled => stats.cancelled_count += 1,
            }
        }

        stats.completion_rate = if stats.total_tasks > 0 {
            stats.completed_count as f32 / stats.total_tasks as f32
        } else {
            0.0
        };

        stats
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_plan_creation() {
        let plan = TaskPlan::new("Test Plan", "Test Description");
        assert_eq!(plan.name, "Test Plan");
        assert_eq!(plan.status, PlanStatus::Draft);
        assert!(plan.tasks.is_empty());
    }

    #[test]
    fn test_add_task() {
        let mut plan = TaskPlan::new("Test", "Desc");
        let task = TaskNode::new("task1", "Task 1");
        plan.add_task(task);
        assert_eq!(plan.tasks.len(), 1);
    }

    #[test]
    fn test_get_task() {
        let mut plan = TaskPlan::new("Test", "Desc");
        let task = TaskNode::new("task1", "Task 1");
        let task_id = task.id.clone();
        plan.add_task(task);

        assert!(plan.get_task(&task_id).is_some());
        assert!(plan.get_task("nonexistent").is_none());
    }

    #[test]
    fn test_task_plan_stats() {
        let mut plan = TaskPlan::new("Test", "Desc");

        let mut task1 = TaskNode::new("task1", "Task 1");
        task1.status = TaskStatus::Completed;
        plan.add_task(task1);

        let task2 = TaskNode::new("task2", "Task 2");
        plan.add_task(task2);

        let stats = plan.get_stats();
        assert_eq!(stats.total_tasks, 2);
        assert_eq!(stats.completed_count, 1);
        assert_eq!(stats.pending_count, 1);
    }
}
