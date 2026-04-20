//! 并行实验运行器模块
//!
//! 使用 rayon 并行执行多个实验，加速实验套件运行。
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::experiment::parallel::{ParallelRunner, ExperimentTask};
//!
//! let mut runner = ParallelRunner::new(4); // 4 个线程
//!
//! runner.add_task(ExperimentTask::new("exp1", || {
//!     let config = AccuracyExperimentConfig::default();
//!     let experiment = AccuracyExperiment::new(config);
//!     experiment.run()
//! }));
//!
//! let results = runner.run_all();
//! ```

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::utils::ExperimentResult;

/// 实验任务
pub struct ExperimentTask<F, T>
where
    F: FnOnce() -> T + Send,
    T: Send + 'static,
{
    name: String,
    task: F,
    _marker: std::marker::PhantomData<T>,
}

impl<F, T> ExperimentTask<F, T>
where
    F: FnOnce() -> T + Send,
    T: Send + 'static,
{
    pub fn new(name: &str, task: F) -> Self {
        Self {
            name: name.to_string(),
            task,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// 并行运行器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelRunnerConfig {
    /// 线程池大小
    pub num_threads: usize,
    /// 是否显示进度
    pub show_progress: bool,
    /// 超时时间（秒）
    pub timeout_secs: Option<f64>,
}

impl Default for ParallelRunnerConfig {
    fn default() -> Self {
        Self {
            num_threads: num_cpus::get(),
            show_progress: true,
            timeout_secs: None,
        }
    }
}

impl ParallelRunnerConfig {
    pub fn new(num_threads: usize) -> Self {
        Self {
            num_threads,
            ..Default::default()
        }
    }

    pub fn progress(mut self, show: bool) -> Self {
        self.show_progress = show;
        self
    }

    pub fn timeout(mut self, secs: f64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
}

/// 并行运行器
pub struct ParallelRunner {
    config: ParallelRunnerConfig,
}

impl ParallelRunner {
    pub fn new(num_threads: usize) -> Self {
        Self {
            config: ParallelRunnerConfig::new(num_threads),
        }
    }

    pub fn with_config(config: ParallelRunnerConfig) -> Self {
        Self { config }
    }

    /// 并行运行多个实验
    ///
    /// # 参数
    /// - `tasks`: 实验任务向量
    ///
    /// # 返回
    /// 实验结果向量
    pub fn run_all<F, T>(&self, tasks: Vec<ExperimentTask<F, T>>) -> Vec<ExperimentResult>
    where
        F: FnOnce() -> T + Send,
        T: Into<ExperimentResult> + Send + 'static,
    {
        let num_tasks = tasks.len();

        if self.config.show_progress {
            println!(
                "🚀 启动并行运行器：{} 个任务，{} 个线程",
                num_tasks, self.config.num_threads
            );
        }

        let start = Instant::now();

        // 使用 rayon 并行执行
        let results: Vec<ExperimentResult> = tasks
            .into_par_iter()
            .map_with(
                (self.config.show_progress, Arc::new(Mutex::new(0usize))),
                |(show_progress, counter), task| {
                    let task_name = task.name().to_string();
                    let task_start = Instant::now();

                    // 执行任务
                    let result = (task.task)().into();

                    let elapsed = task_start.elapsed();

                    if *show_progress {
                        let mut count = counter.lock().unwrap();
                        *count += 1;
                        println!(
                            "  ✓ {} 完成：耗时 {:.2}s ({}/{})",
                            task_name,
                            elapsed.as_secs_f64(),
                            *count,
                            num_tasks
                        );
                    }

                    result
                },
            )
            .collect();

        let total_elapsed = start.elapsed();

        if self.config.show_progress {
            println!(
                "📊 所有任务完成：总耗时 {:.2}s，平均 {:.2}s/任务",
                total_elapsed.as_secs_f64(),
                total_elapsed.as_secs_f64() / num_tasks.max(1) as f64
            );
        }

        results
    }
}

/// 实验结果包装器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResultWrapper {
    pub name: String,
    pub result: ExperimentResult,
}

impl From<ExperimentResultWrapper> for ExperimentResult {
    fn from(wrapper: ExperimentResultWrapper) -> Self {
        let mut result = wrapper.result;
        result.experiment_name = wrapper.name;
        result
    }
}

/// 并行运行器构建器
pub struct ParallelRunnerBuilder {
    config: ParallelRunnerConfig,
}

impl ParallelRunnerBuilder {
    pub fn new() -> Self {
        Self {
            config: ParallelRunnerConfig::default(),
        }
    }

    pub fn threads(mut self, num: usize) -> Self {
        self.config.num_threads = num;
        self
    }

    pub fn progress(mut self, show: bool) -> Self {
        self.config.show_progress = show;
        self
    }

    pub fn timeout(mut self, secs: f64) -> Self {
        self.config.timeout_secs = Some(secs);
        self
    }

    pub fn build(self) -> ParallelRunner {
        ParallelRunner::with_config(self.config)
    }
}

impl Default for ParallelRunnerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 运行实验套件（并行版本）
///
/// # 使用示例
///
/// ```rust,ignore
/// let results = run_experiment_suite_parallel(vec![
///     ("exp1", || AccuracyExperiment::new(config).run()),
///     ("exp2", || PerformanceExperiment::new(config).run()),
///     ("exp3", || VLMReasoningExperiment::new(config).run()),
/// ]);
/// ```
pub fn run_experiment_suite_parallel<F, T>(tasks: Vec<(&str, F)>) -> Vec<ExperimentResult>
where
    F: FnOnce() -> T + Send,
    T: Into<ExperimentResult> + Send + 'static,
{
    let runner = ParallelRunnerBuilder::new().progress(true).build();

    let experiment_tasks: Vec<ExperimentTask<F, T>> = tasks
        .into_iter()
        .map(|(name, task)| ExperimentTask::new(name, task))
        .collect();

    runner.run_all(experiment_tasks)
}

/// 计算并行加速比
pub fn compute_speedup(sequential_time: f64, parallel_time: f64) -> f64 {
    if parallel_time > 0.0 {
        sequential_time / parallel_time
    } else {
        f64::INFINITY
    }
}

/// 计算并行效率
pub fn compute_parallel_efficiency(speedup: f64, num_threads: usize) -> f64 {
    if num_threads > 0 {
        speedup / num_threads as f64
    } else {
        0.0
    }
}

/// 并行执行统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionStats {
    /// 任务总数
    pub total_tasks: usize,
    /// 串行总耗时（秒）
    pub sequential_time_secs: f64,
    /// 并行总耗时（秒）
    pub parallel_time_secs: f64,
    /// 线程数
    pub num_threads: usize,
    /// 加速比
    pub speedup: f64,
    /// 并行效率
    pub efficiency: f64,
}

impl ParallelExecutionStats {
    pub fn new(
        total_tasks: usize,
        sequential_time_secs: f64,
        parallel_time_secs: f64,
        num_threads: usize,
    ) -> Self {
        let speedup = compute_speedup(sequential_time_secs, parallel_time_secs);
        let efficiency = compute_parallel_efficiency(speedup, num_threads);

        Self {
            total_tasks,
            sequential_time_secs,
            parallel_time_secs,
            num_threads,
            speedup,
            efficiency,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "并行执行统计：{} 个任务，加速比 {:.2}x，效率 {:.2}%",
            self.total_tasks,
            self.speedup,
            self.efficiency * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_experiment_task_creation() {
        let task = ExperimentTask::new(
            "test_task",
            Box::new(|| ExperimentResult::new("test_task").with_metric("value", 1.0)),
        );

        assert_eq!(task.name(), "test_task");
    }

    #[test]
    fn test_parallel_runner() {
        let runner = ParallelRunner::new(2);

        // 使用 Box 来统一不同类型的闭包
        let tasks: Vec<
            ExperimentTask<Box<dyn FnOnce() -> ExperimentResult + Send>, ExperimentResult>,
        > = vec![
            ExperimentTask::new(
                "task1",
                Box::new(|| {
                    thread::sleep(Duration::from_millis(100));
                    ExperimentResult::new("task1").with_metric("value", 1.0)
                }),
            ),
            ExperimentTask::new(
                "task2",
                Box::new(|| {
                    thread::sleep(Duration::from_millis(100));
                    ExperimentResult::new("task2").with_metric("value", 2.0)
                }),
            ),
            ExperimentTask::new(
                "task3",
                Box::new(|| {
                    thread::sleep(Duration::from_millis(100));
                    ExperimentResult::new("task3").with_metric("value", 3.0)
                }),
            ),
        ];

        let results = runner.run_all(tasks);

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_parallel_runner_config_default() {
        let config = ParallelRunnerConfig::default();
        assert_eq!(config.num_threads, num_cpus::get());
        assert!(config.show_progress);
        assert!(config.timeout_secs.is_none());
    }

    #[test]
    fn test_parallel_runner_config_custom() {
        let config = ParallelRunnerConfig {
            num_threads: 8,
            show_progress: false,
            timeout_secs: Some(60.0),
        };

        assert_eq!(config.num_threads, 8);
        assert!(!config.show_progress);
        assert_eq!(config.timeout_secs, Some(60.0));
    }

    #[test]
    fn test_parallel_runner_builder() {
        let runner = ParallelRunnerBuilder::new()
            .threads(8)
            .progress(false)
            .timeout(120.0)
            .build();

        assert_eq!(runner.config.num_threads, 8);
        assert!(!runner.config.show_progress);
        assert_eq!(runner.config.timeout_secs, Some(120.0));
    }

    #[test]
    fn test_parallel_runner_builder_default() {
        let runner = ParallelRunnerBuilder::default().build();
        let default_config = ParallelRunnerConfig::default();

        assert_eq!(runner.config.num_threads, default_config.num_threads);
        assert_eq!(runner.config.show_progress, default_config.show_progress);
    }

    #[test]
    fn test_speedup_calculation() {
        let sequential = 10.0;
        let parallel = 2.5;
        let speedup = compute_speedup(sequential, parallel);

        assert!((speedup - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_speedup_zero_parallel_time() {
        let speedup = compute_speedup(10.0, 0.0);
        assert!(speedup.is_infinite());
    }

    #[test]
    fn test_efficiency_calculation() {
        let speedup = 3.0;
        let num_threads = 4;
        let efficiency = compute_parallel_efficiency(speedup, num_threads);

        assert!((efficiency - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_efficiency_zero_threads() {
        let efficiency = compute_parallel_efficiency(3.0, 0);
        assert_eq!(efficiency, 0.0);
    }

    #[test]
    fn test_parallel_execution_stats() {
        let stats = ParallelExecutionStats::new(10, 100.0, 25.0, 4);

        assert_eq!(stats.total_tasks, 10);
        assert!((stats.sequential_time_secs - 100.0).abs() < 0.01);
        assert!((stats.parallel_time_secs - 25.0).abs() < 0.01);
        assert_eq!(stats.num_threads, 4);
        assert!((stats.speedup - 4.0).abs() < 0.01);
        assert!((stats.efficiency - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_parallel_runner_single_task() {
        let runner = ParallelRunner::new(2);
        let tasks: Vec<
            ExperimentTask<Box<dyn FnOnce() -> ExperimentResult + Send>, ExperimentResult>,
        > = vec![ExperimentTask::new(
            "single",
            Box::new(|| ExperimentResult::new("single").with_metric("value", 1.0)),
        )];

        let results = runner.run_all(tasks);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].experiment_name, "single");
    }

    #[test]
    fn test_parallel_runner_many_tasks() {
        let runner = ParallelRunner::new(4);
        let tasks: Vec<
            ExperimentTask<Box<dyn FnOnce() -> ExperimentResult + Send>, ExperimentResult>,
        > = vec![
            ExperimentTask::new(
                "task_0",
                Box::new(|| ExperimentResult::new("task_0").with_metric("value", 0.0)),
            ),
            ExperimentTask::new(
                "task_1",
                Box::new(|| ExperimentResult::new("task_1").with_metric("value", 1.0)),
            ),
            ExperimentTask::new(
                "task_2",
                Box::new(|| ExperimentResult::new("task_2").with_metric("value", 2.0)),
            ),
            ExperimentTask::new(
                "task_3",
                Box::new(|| ExperimentResult::new("task_3").with_metric("value", 3.0)),
            ),
            ExperimentTask::new(
                "task_4",
                Box::new(|| ExperimentResult::new("task_4").with_metric("value", 4.0)),
            ),
            ExperimentTask::new(
                "task_5",
                Box::new(|| ExperimentResult::new("task_5").with_metric("value", 5.0)),
            ),
            ExperimentTask::new(
                "task_6",
                Box::new(|| ExperimentResult::new("task_6").with_metric("value", 6.0)),
            ),
            ExperimentTask::new(
                "task_7",
                Box::new(|| ExperimentResult::new("task_7").with_metric("value", 7.0)),
            ),
            ExperimentTask::new(
                "task_8",
                Box::new(|| ExperimentResult::new("task_8").with_metric("value", 8.0)),
            ),
            ExperimentTask::new(
                "task_9",
                Box::new(|| ExperimentResult::new("task_9").with_metric("value", 9.0)),
            ),
        ];

        let results = runner.run_all(tasks);
        assert_eq!(results.len(), 10);
    }
}
