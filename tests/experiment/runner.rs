//! 实验运行器模块
//!
//! 提供统一的实验执行和报告生成框架。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::utils::{ensure_output_dir, ExperimentConfig, ExperimentReport, ExperimentResult};

/// 实验运行器 trait
pub trait RunnableExperiment {
    /// 实验名称
    fn name(&self) -> &str;

    /// 运行实验
    fn run(&self) -> ExperimentResult;

    /// 生成详细报告
    fn generate_report(&self) -> ExperimentReport;
}

/// 实验运行器
pub struct ExperimentRunner {
    config: ExperimentConfig,
    results: Vec<ExperimentResult>,
    reports: Vec<ExperimentReport>,
}

impl ExperimentRunner {
    pub fn new(config: ExperimentConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            reports: Vec::new(),
        }
    }

    /// 运行单个实验
    pub fn run_experiment<E: RunnableExperiment>(&mut self, experiment: &E) {
        println!("\n{}", "=".repeat(60));
        println!("运行实验：{}", experiment.name());
        println!("{}", "=".repeat(60));

        let start = Instant::now();
        let result = experiment.run();
        let duration = start.elapsed();

        println!("实验完成：耗时 {:.2}s", duration.as_secs_f64());

        self.results.push(result);
    }

    /// 添加报告
    pub fn add_report(&mut self, report: ExperimentReport) {
        self.reports.push(report);
    }

    /// 保存所有结果
    pub fn save_results(&self) -> std::io::Result<()> {
        ensure_output_dir(&self.config.output_dir)?;

        for result in &self.results {
            let filename = format!(
                "{}_result.json",
                result.experiment_name.to_lowercase().replace(" ", "_")
            );
            let path = self.config.output_dir.join(filename);
            result.save_to(&path)?;
            println!("结果已保存：{:?}", path);
        }

        Ok(())
    }

    /// 保存所有报告
    pub fn save_reports(&self) -> std::io::Result<()> {
        ensure_output_dir(&self.config.output_dir)?;

        for report in &self.reports {
            let filename = format!(
                "{}_report.json",
                report.title.to_lowercase().replace(" ", "_")
            );
            let path = self.config.output_dir.join(filename);
            report.save_to(&path)?;

            let md_filename = format!(
                "{}_report.md",
                report.title.to_lowercase().replace(" ", "_")
            );
            let md_path = self.config.output_dir.join(md_filename);
            report.save_as_markdown(&md_path)?;

            println!("报告已保存：{:?} {:?}", path, md_path);
        }

        Ok(())
    }

    /// 生成汇总报告
    pub fn generate_summary(&self) -> ExperimentReport {
        let mut report = ExperimentReport::new("实验汇总报告");

        let mut summary = String::from("本汇总报告包含以下实验结果:\n\n");
        for result in &self.results {
            summary.push_str(&format!(
                "- **{}**: {} (耗时 {:.2}s)\n",
                result.experiment_name,
                if result.passed {
                    "✓ 通过"
                } else {
                    "✗ 失败"
                },
                result.duration_secs
            ));
        }

        report = report.summary(&summary);

        let all_passed = self.results.iter().all(|r| r.passed);
        let conclusion = if all_passed {
            "所有实验均通过验证，CadAgent 的核心功能符合预期。".to_string()
        } else {
            let failed_count = self.results.iter().filter(|r| !r.passed).count();
            format!("{} 个实验失败，请检查错误信息。", failed_count)
        };

        report = report.conclusion(&conclusion);

        for result in &self.results.clone() {
            report = report.add_result(result.clone());
        }

        report
    }

    /// 获取所有结果
    pub fn results(&self) -> &[ExperimentResult] {
        &self.results
    }

    /// 获取所有报告
    pub fn reports(&self) -> &[ExperimentReport] {
        &self.reports
    }
}

/// 实验结果汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSummary {
    /// 总实验数
    pub total_experiments: usize,
    /// 通过的实验数
    pub passed_experiments: usize,
    /// 失败的实验数
    pub failed_experiments: usize,
    /// 总耗时 (秒)
    pub total_duration_secs: f64,
    /// 各实验结果
    pub results: Vec<ExperimentResult>,
    /// 生成时间
    pub generated_at: String,
}

impl ExperimentSummary {
    pub fn from_results(results: Vec<ExperimentResult>) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let total_duration: f64 = results.iter().map(|r| r.duration_secs).sum();

        Self {
            total_experiments: total,
            passed_experiments: passed,
            failed_experiments: failed,
            total_duration_secs: total_duration,
            generated_at: results
                .first()
                .map(|r| r.timestamp.clone())
                .unwrap_or_else(|| String::from("N/A")),
            results,
        }
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total_experiments == 0 {
            0.0
        } else {
            self.passed_experiments as f64 / self.total_experiments as f64
        }
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, self)?;
        writer.flush()?;
        Ok(())
    }
}

/// 实验配置构建器
pub struct ExperimentBuilder {
    name: String,
    description: String,
    parameters: HashMap<String, String>,
    verbose: bool,
    output_dir: PathBuf,
}

impl ExperimentBuilder {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters: HashMap::new(),
            verbose: false,
            output_dir: PathBuf::from("tests/experiment/results"),
        }
    }

    pub fn param(mut self, key: &str, value: &str) -> Self {
        self.parameters.insert(key.to_string(), value.to_string());
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = dir;
        self
    }

    pub fn build(self) -> ExperimentConfig {
        ExperimentConfig {
            name: self.name,
            description: self.description,
            verbose: self.verbose,
            output_dir: self.output_dir,
            parameters: self.parameters,
        }
    }
}

/// 批量运行实验
pub fn run_all_experiments<E, F>(experiments: Vec<(&str, F)>) -> ExperimentSummary
where
    E: RunnableExperiment,
    F: Fn() -> E,
{
    let config = ExperimentConfig::new("batch", "批量实验运行");
    let mut runner = ExperimentRunner::new(config);

    for (name, factory) in experiments {
        let experiment = factory();
        println!("\n运行实验：{}", name);
        let result = experiment.run();
        runner.results.push(result);
    }

    ExperimentSummary::from_results(runner.results.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockExperiment {
        name: String,
        should_pass: bool,
    }

    impl MockExperiment {
        fn new(name: &str, should_pass: bool) -> Self {
            Self {
                name: name.to_string(),
                should_pass,
            }
        }
    }

    impl RunnableExperiment for MockExperiment {
        fn name(&self) -> &str {
            &self.name
        }

        fn run(&self) -> ExperimentResult {
            ExperimentResult::new(&self.name).with_metric("mock_metric", 1.0)
        }

        fn generate_report(&self) -> ExperimentReport {
            ExperimentReport::new(&self.name)
        }
    }

    #[test]
    fn test_experiment_runner() {
        let config = ExperimentConfig::new("test", "Test runner");
        let mut runner = ExperimentRunner::new(config);

        let exp1 = MockExperiment::new("test1", true);
        let exp2 = MockExperiment::new("test2", true);

        runner.run_experiment(&exp1);
        runner.run_experiment(&exp2);

        assert_eq!(runner.results.len(), 2);
    }

    #[test]
    fn test_experiment_summary() {
        let results = vec![
            ExperimentResult::new("test1").with_metric("value", 1.0),
            ExperimentResult::new("test2").with_metric("value", 2.0),
        ];

        let summary = ExperimentSummary::from_results(results);

        assert_eq!(summary.total_experiments, 2);
        assert_eq!(summary.pass_rate(), 1.0);
    }
}
