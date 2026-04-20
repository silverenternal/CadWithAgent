//! Task Planning Module
//!
//! DAG-based task planning and execution tracking.

mod executor;
mod plan;
mod types;

pub use executor::*;
pub use plan::*;
pub use types::*;

use crate::error::{CadAgentError, CadAgentResult};

/// Configuration for TaskPlanner
#[derive(Debug, Clone)]
pub struct TaskPlannerConfig {
    /// Context root directory
    pub context_root: String,
    /// Enable automatic retry on failure
    pub enable_auto_retry: bool,
    /// Default maximum retries per task
    pub default_max_retries: u32,
}

impl Default for TaskPlannerConfig {
    fn default() -> Self {
        Self {
            context_root: "./.cad_context/tasks".to_string(),
            enable_auto_retry: true,
            default_max_retries: 3,
        }
    }
}

/// Task Planner
///
/// Manages task planning, execution tracking, and dependency management.
/// Combines TaskPlan creation with TaskExecutor for full planning workflow.
pub struct TaskPlanner {
    /// Task executor
    executor: TaskExecutor,
}

impl TaskPlanner {
    /// Create a new TaskPlanner with default configuration
    pub fn new() -> CadAgentResult<Self> {
        Self::with_config(TaskPlannerConfig::default())
    }

    /// Create a new TaskPlanner with custom configuration
    pub fn with_config(config: TaskPlannerConfig) -> CadAgentResult<Self> {
        let executor_config = TaskExecutorConfig {
            enable_auto_retry: config.enable_auto_retry,
            default_max_retries: config.default_max_retries,
        };

        let executor = TaskExecutor::new(executor_config)?;

        Ok(Self { executor })
    }

    /// Create a new task plan
    pub fn create_plan(&mut self, name: &str, description: &str) -> CadAgentResult<&TaskPlan> {
        let plan = TaskPlan::new(name, description);
        self.executor.set_plan(plan);
        Ok(self.executor.get_plan().unwrap())
    }

    /// Add a task to the current plan
    pub fn add_task(&mut self, task: TaskNode) -> CadAgentResult<&TaskNode> {
        let plan = self
            .executor
            .get_plan_mut()
            .ok_or_else(|| CadAgentError::internal("No active plan".to_string()))?;

        plan.add_task(task);
        Ok(plan.tasks.last().unwrap())
    }

    /// Add a task with builder pattern
    pub fn add_task_simple(
        &mut self,
        name: &str,
        description: &str,
        dependencies: Vec<&str>,
    ) -> CadAgentResult<&TaskNode> {
        let task = TaskNode::new(name, description).with_dependencies(dependencies);
        self.add_task(task)
    }

    /// Mark a task as completed
    pub fn complete_task(&mut self, task_id: &str, result: Option<&str>) -> CadAgentResult<()> {
        let plan = self
            .executor
            .get_plan_mut()
            .ok_or_else(|| CadAgentError::internal("No active plan".to_string()))?;

        let task = plan
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| CadAgentError::internal(format!("Task not found: {}", task_id)))?;

        task.status = TaskStatus::Completed;
        task.result = result.map(|s| s.to_string());

        Ok(())
    }

    /// Execute the current plan
    pub fn execute<F>(&mut self, executor: F) -> CadAgentResult<TaskPlanStats>
    where
        F: Fn(&TaskNode) -> CadAgentResult<String> + Send + Sync,
    {
        self.executor.execute(executor)
    }

    /// Get the current plan
    pub fn get_current_plan(&self) -> Option<&TaskPlan> {
        self.executor.get_plan()
    }

    /// Get the current plan mutably
    pub fn get_current_plan_mut(&mut self) -> Option<&mut TaskPlan> {
        self.executor.get_plan_mut()
    }

    /// Get plan statistics
    pub fn get_plan_stats(&self) -> Option<TaskPlanStats> {
        self.executor.get_plan_stats()
    }

    /// Approve the current plan for execution
    pub fn approve_plan(&mut self) -> CadAgentResult<()> {
        self.executor.approve_plan()
    }

    /// Cancel the current plan
    pub fn cancel_plan(&mut self) -> CadAgentResult<()> {
        self.executor.cancel_plan()
    }

    /// Clear the current plan
    pub fn clear_plan(&mut self) {
        self.executor.clear_plan();
    }

    /// Create a checkpoint
    pub fn create_checkpoint(&mut self, checkpoint_name: &str) -> CadAgentResult<String> {
        self.executor.create_checkpoint(checkpoint_name)
    }

    /// Rollback to a checkpoint
    pub fn rollback_to_checkpoint(&mut self, checkpoint_hash: &str) -> CadAgentResult<bool> {
        self.executor.rollback_to_checkpoint(checkpoint_hash)
    }

    /// Retry a failed task
    pub fn retry_from_checkpoint(&mut self, task_id: &str) -> CadAgentResult<bool> {
        self.executor.retry_task(task_id)
    }
}

impl Default for TaskPlanner {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_creation() {
        let planner = TaskPlanner::new().unwrap();
        assert!(planner.get_current_plan().is_none());
    }

    #[test]
    fn test_create_plan() {
        let mut planner = TaskPlanner::new().unwrap();
        let plan = planner
            .create_plan("Test Plan", "Test Description")
            .unwrap();
        assert_eq!(plan.name, "Test Plan");
    }

    #[test]
    fn test_add_task() {
        let mut planner = TaskPlanner::new().unwrap();
        planner.create_plan("Test", "Desc").unwrap();

        let task = planner.add_task_simple("task1", "Task 1", vec![]).unwrap();
        assert_eq!(task.name, "task1");
    }

    #[test]
    fn test_complete_task() {
        let mut planner = TaskPlanner::new().unwrap();
        planner.create_plan("Test", "Desc").unwrap();
        let task = planner.add_task_simple("task1", "Task 1", vec![]).unwrap();
        let task_id = task.id.clone();

        assert!(planner.complete_task(&task_id, Some("Done")).is_ok());
    }

    #[test]
    fn test_approve_and_execute() {
        let mut planner = TaskPlanner::new().unwrap();
        planner.create_plan("Test", "Desc").unwrap();
        planner.add_task_simple("task1", "Task 1", vec![]).unwrap();
        planner.approve_plan().unwrap();

        let stats = planner.execute(|_task| Ok("Success".to_string())).unwrap();
        assert_eq!(stats.completed_count, 1);
    }
}
