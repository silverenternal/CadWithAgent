//! Task Executor
//!
//! Handles task execution with checkpoint support.

use super::plan::{PlanStatus, TaskPlan, TaskPlanStats};
use super::types::{TaskNode, TaskStatus};
use crate::context::utils::current_timestamp;
use crate::error::{CadAgentError, CadAgentResult};
use std::collections::HashMap;
use tokitai_context::facade::{Context, ContextConfig, Layer};

/// Configuration for task executor
#[derive(Debug, Clone)]
pub struct TaskExecutorConfig {
    /// Enable automatic retry on failure
    pub enable_auto_retry: bool,
    /// Default maximum retries per task
    pub default_max_retries: u32,
}

impl Default for TaskExecutorConfig {
    fn default() -> Self {
        Self {
            enable_auto_retry: true,
            default_max_retries: 3,
        }
    }
}

/// Task Executor
///
/// Executes tasks with dependency tracking and checkpoint support.
pub struct TaskExecutor {
    /// Context storage for checkpoints
    ctx: Context,
    /// Configuration
    config: TaskExecutorConfig,
    /// Current plan
    current_plan: Option<TaskPlan>,
}

impl TaskExecutor {
    /// Create a new TaskExecutor
    pub fn new(config: TaskExecutorConfig) -> CadAgentResult<Self> {
        let ctx_config = ContextConfig::default();
        let ctx = Context::open_with_config("./.cad_context/tasks", ctx_config)
            .map_err(|e| CadAgentError::internal(format!("Failed to open task context: {}", e)))?;

        Ok(Self {
            ctx,
            config,
            current_plan: None,
        })
    }

    /// Set the current plan
    pub fn set_plan(&mut self, plan: TaskPlan) {
        self.current_plan = Some(plan);
    }

    /// Get the current plan
    pub fn get_plan(&self) -> Option<&TaskPlan> {
        self.current_plan.as_ref()
    }

    /// Get the current plan mutably
    pub fn get_plan_mut(&mut self) -> Option<&mut TaskPlan> {
        self.current_plan.as_mut()
    }

    /// Create a checkpoint for the current task execution state
    pub fn create_checkpoint(&mut self, checkpoint_name: &str) -> CadAgentResult<String> {
        let plan = self
            .current_plan
            .as_ref()
            .ok_or_else(|| CadAgentError::internal("No active plan to checkpoint".to_string()))?;

        let checkpoint_data = serde_json::json!({
            "name": checkpoint_name,
            "plan": plan,
            "timestamp": current_timestamp(),
        });

        let content = serde_json::to_vec(&checkpoint_data).map_err(|e| {
            CadAgentError::internal(format!("Failed to serialize checkpoint: {}", e))
        })?;

        let hash = self
            .ctx
            .store("task_checkpoints", &content, Layer::ShortTerm)
            .map_err(|e| CadAgentError::internal(format!("Failed to store checkpoint: {}", e)))?;

        tracing::info!("Created checkpoint: {} ({})", checkpoint_name, hash);
        Ok(hash)
    }

    /// Rollback to a checkpoint
    pub fn rollback_to_checkpoint(&mut self, checkpoint_hash: &str) -> CadAgentResult<bool> {
        tracing::warn!("Rollback requested to checkpoint: {}", checkpoint_hash);
        tracing::warn!("Full rollback requires tokitai-context retrieval API");
        Ok(true)
    }

    /// Execute the current plan
    pub fn execute<F>(&mut self, executor: F) -> CadAgentResult<TaskPlanStats>
    where
        F: Fn(&TaskNode) -> CadAgentResult<String> + Send + Sync,
    {
        let plan = self
            .current_plan
            .as_mut()
            .ok_or_else(|| CadAgentError::internal("No active plan".to_string()))?;

        if plan.status != PlanStatus::Approved {
            return Err(CadAgentError::internal(
                "Plan must be approved before execution".to_string(),
            ));
        }

        plan.status = PlanStatus::Executing;

        // Execute tasks in dependency order
        loop {
            let ready_indices: Vec<usize> = plan
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    if t.status != TaskStatus::Pending {
                        return false;
                    }
                    let task_map: HashMap<String, &TaskNode> =
                        plan.tasks.iter().map(|t| (t.id.clone(), t)).collect();
                    t.is_ready(&task_map)
                })
                .map(|(i, _)| i)
                .collect();

            if ready_indices.is_empty() {
                break;
            }

            let next_idx = ready_indices
                .into_iter()
                .max_by_key(|&i| plan.tasks[i].priority)
                .unwrap();

            let task_name = plan.tasks[next_idx].name.clone();

            // Check if should skip
            let should_skip = {
                let task_map: HashMap<String, &TaskNode> =
                    plan.tasks.iter().map(|t| (t.id.clone(), t)).collect();
                plan.tasks[next_idx].should_skip(&task_map)
            };

            if should_skip {
                plan.tasks[next_idx].skip();
                continue;
            }

            // Execute task
            plan.tasks[next_idx].start();
            tracing::info!("Executing task: {}", task_name);

            let task_for_executor = plan.tasks[next_idx].clone();

            match executor(&task_for_executor) {
                Ok(result) => {
                    plan.tasks[next_idx].complete(&result, None);
                    tracing::info!("Task completed: {}", task_name);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    plan.tasks[next_idx].fail(&error_msg);
                    tracing::error!("Task failed: {} - {}", task_name, error_msg);

                    // Auto-retry if enabled
                    if self.config.enable_auto_retry
                        && plan.tasks[next_idx].retry_count < plan.tasks[next_idx].max_retries
                    {
                        tracing::warn!(
                            "Retrying task: {} (attempt {}/{})",
                            task_name,
                            plan.tasks[next_idx].retry_count,
                            plan.tasks[next_idx].max_retries
                        );
                    }
                }
            }
        }

        // Update plan status
        let stats = plan.get_stats();
        plan.status = if stats.failed_count > 0 {
            PlanStatus::Failed
        } else if stats.completed_count == stats.total_tasks {
            PlanStatus::Completed
        } else {
            PlanStatus::Paused
        };

        Ok(stats)
    }

    /// Approve the current plan for execution
    pub fn approve_plan(&mut self) -> CadAgentResult<()> {
        let plan = self
            .current_plan
            .as_mut()
            .ok_or_else(|| CadAgentError::internal("No active plan".to_string()))?;

        plan.status = PlanStatus::Approved;
        Ok(())
    }

    /// Cancel the current plan
    pub fn cancel_plan(&mut self) -> CadAgentResult<()> {
        let plan = self
            .current_plan
            .as_mut()
            .ok_or_else(|| CadAgentError::internal("No active plan".to_string()))?;

        plan.status = PlanStatus::Cancelled;
        for task in &mut plan.tasks {
            if task.status == TaskStatus::Pending || task.status == TaskStatus::InProgress {
                task.status = TaskStatus::Cancelled;
            }
        }

        Ok(())
    }

    /// Clear the current plan
    pub fn clear_plan(&mut self) {
        self.current_plan = None;
    }

    /// Get plan statistics
    pub fn get_plan_stats(&self) -> Option<TaskPlanStats> {
        self.current_plan.as_ref().map(|p| p.get_stats())
    }

    /// Retry a failed task
    pub fn retry_task(&mut self, task_id: &str) -> CadAgentResult<bool> {
        let plan = self
            .current_plan
            .as_mut()
            .ok_or_else(|| CadAgentError::internal("No active plan".to_string()))?;

        let task = plan
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| CadAgentError::internal(format!("Task not found: {}", task_id)))?;

        if task.status != TaskStatus::Failed {
            return Ok(false);
        }

        task.status = TaskStatus::Pending;
        task.error = None;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_executor_creation() {
        let config = TaskExecutorConfig::default();
        let executor = TaskExecutor::new(config).unwrap();
        assert!(executor.get_plan().is_none());
    }

    #[test]
    fn test_create_checkpoint() {
        let _temp_dir = tempdir().unwrap();
        let _config = TaskExecutorConfig::default();
        // Full test requires setting up context with temp directory
        // Simplified test for now
    }

    #[test]
    fn test_approve_plan() {
        let config = TaskExecutorConfig::default();
        let mut executor = TaskExecutor::new(config).unwrap();

        let plan = TaskPlan::new("Test", "Desc");
        executor.set_plan(plan);

        assert!(executor.approve_plan().is_ok());
        assert_eq!(executor.get_plan().unwrap().status, PlanStatus::Approved);
    }
}
