//! 实验工具模块
//!
//! 提供实验配置、结果记录和报告生成的通用工具。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// 实验名称
    pub name: String,
    /// 实验描述
    pub description: String,
    /// 是否启用详细输出
    pub verbose: bool,
    /// 输出目录
    pub output_dir: PathBuf,
    /// 实验参数
    pub parameters: HashMap<String, String>,
}

impl ExperimentConfig {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            verbose: false,
            output_dir: PathBuf::from("tests/experiment/results"),
            parameters: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: &str, value: &str) -> Self {
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
}

/// 实验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// 实验名称
    pub experiment_name: String,
    /// 实验是否通过
    pub passed: bool,
    /// 实验耗时
    pub duration_secs: f64,
    /// 实验指标
    pub metrics: HashMap<String, f64>,
    /// 实验数据
    pub data: HashMap<String, String>,
    /// 错误信息
    pub errors: Vec<String>,
    /// 时间戳
    pub timestamp: String,
}

impl ExperimentResult {
    pub fn new(experiment_name: &str) -> Self {
        Self {
            experiment_name: experiment_name.to_string(),
            passed: true,
            duration_secs: 0.0,
            metrics: HashMap::new(),
            data: HashMap::new(),
            errors: Vec::new(),
            timestamp: chrono_lite_timestamp(),
        }
    }

    pub fn with_metric(mut self, key: &str, value: f64) -> Self {
        self.metrics.insert(key.to_string(), value);
        self
    }

    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self.passed = false;
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration_secs = duration.as_secs_f64();
        self
    }

    /// 保存结果到 JSON 文件
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, self)?;
        writer.flush()?;
        Ok(())
    }
}

/// 实验报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentReport {
    /// 报告标题
    pub title: String,
    /// 报告摘要
    pub summary: String,
    /// 实验结果列表
    pub results: Vec<ExperimentResult>,
    /// 总体结论
    pub conclusion: String,
    /// 生成时间
    pub generated_at: String,
    /// 图表数据
    pub charts: Vec<ChartData>,
}

impl ExperimentReport {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            summary: String::new(),
            results: Vec::new(),
            conclusion: String::new(),
            generated_at: chrono_lite_timestamp(),
            charts: Vec::new(),
        }
    }

    pub fn add_result(mut self, result: ExperimentResult) -> Self {
        self.results.push(result);
        self
    }

    pub fn summary(mut self, summary: &str) -> Self {
        self.summary = summary.to_string();
        self
    }

    pub fn conclusion(mut self, conclusion: &str) -> Self {
        self.conclusion = conclusion.to_string();
        self
    }

    pub fn add_chart(mut self, chart: ChartData) -> Self {
        self.charts.push(chart);
        self
    }

    /// 保存报告到 JSON 文件
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, self)?;
        writer.flush()?;
        Ok(())
    }

    /// 保存报告到 Markdown 文件
    pub fn save_as_markdown(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut content = String::new();

        content.push_str(&format!("# {}\n\n", self.title));
        content.push_str(&format!("**生成时间**: {}\n\n", self.generated_at));
        content.push_str("## 摘要\n\n");
        content.push_str(&format!("{}\n\n", self.summary));
        content.push_str("## 实验结果\n\n");

        for (i, result) in self.results.iter().enumerate() {
            content.push_str(&format!("### {}. {}\n\n", i + 1, result.experiment_name));
            content.push_str(&format!(
                "**状态**: {}\n",
                if result.passed {
                    "✓ 通过"
                } else {
                    "✗ 失败"
                }
            ));
            content.push_str(&format!("**耗时**: {:.2}s\n\n", result.duration_secs));

            if !result.metrics.is_empty() {
                content.push_str("**指标**:\n");
                for (key, value) in &result.metrics {
                    content.push_str(&format!("- {}: {:.4}\n", key, value));
                }
                content.push('\n');
            }

            if !result.errors.is_empty() {
                content.push_str("**错误**:\n");
                for error in &result.errors {
                    content.push_str(&format!("- {}\n", error));
                }
                content.push('\n');
            }
        }

        content.push_str("## 结论\n\n");
        content.push_str(&format!("{}\n", self.conclusion));

        fs::write(path, content)
    }
}

/// 图表数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    /// 图表类型
    pub chart_type: ChartType,
    /// 图表标题
    pub title: String,
    /// X 轴标签
    pub x_label: String,
    /// Y 轴标签
    pub y_label: String,
    /// 数据系列
    pub series: Vec<DataSeries>,
}

impl ChartData {
    pub fn new(chart_type: ChartType, title: &str) -> Self {
        Self {
            chart_type,
            title: title.to_string(),
            x_label: String::new(),
            y_label: String::new(),
            series: Vec::new(),
        }
    }

    pub fn x_label(mut self, label: &str) -> Self {
        self.x_label = label.to_string();
        self
    }

    pub fn y_label(mut self, label: &str) -> Self {
        self.y_label = label.to_string();
        self
    }

    pub fn add_series(mut self, name: &str, data: Vec<f64>) -> Self {
        self.series.push(DataSeries {
            name: name.to_string(),
            data,
        });
        self
    }
}

/// 图表类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartType {
    /// 柱状图
    Bar,
    /// 折线图
    Line,
    /// 散点图
    Scatter,
    /// 箱线图
    Box,
}

/// 数据系列
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSeries {
    /// 系列名称
    pub name: String,
    /// 数据点
    pub data: Vec<f64>,
}

/// 实验计时器
pub struct ExperimentTimer {
    start: Instant,
}

impl ExperimentTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

/// 简单的 timestamp 生成（不依赖 chrono crate）
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    // Simple conversion to human-readable format
    // This is a simplified version - for production use, consider using chrono
    format!("Unix timestamp: {}", secs)
}

/// 创建实验输出目录
pub fn ensure_output_dir(output_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(output_dir)
}

/// 计算统计指标
pub fn compute_statistics(values: &[f64]) -> Statistics {
    if values.is_empty() {
        return Statistics::default();
    }

    let n = values.len() as f64;
    let sum: f64 = values.iter().sum();
    let mean = sum / n;

    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let min = sorted.first().copied().unwrap_or(0.0);
    let max = sorted.last().copied().unwrap_or(0.0);
    let median = sorted[sorted.len() / 2];

    let p95_idx = (sorted.len() as f64 * 0.95) as usize;
    let p99_idx = (sorted.len() as f64 * 0.99) as usize;
    let p95 = sorted[p95_idx.min(sorted.len() - 1)];
    let p99 = sorted[p99_idx.min(sorted.len() - 1)];

    Statistics {
        count: values.len(),
        mean,
        std_dev,
        min,
        max,
        median,
        p95,
        p99,
    }
}

/// 统计指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Statistics {
    /// 样本数量
    pub count: usize,
    /// 平均值
    pub mean: f64,
    /// 标准差
    pub std_dev: f64,
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 中位数
    pub median: f64,
    /// 95 百分位数
    pub p95: f64,
    /// 99 百分位数
    pub p99: f64,
}

impl Statistics {
    pub fn to_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert("count".to_string(), self.count as f64);
        map.insert("mean".to_string(), self.mean);
        map.insert("std_dev".to_string(), self.std_dev);
        map.insert("min".to_string(), self.min);
        map.insert("max".to_string(), self.max);
        map.insert("median".to_string(), self.median);
        map.insert("p95".to_string(), self.p95);
        map.insert("p99".to_string(), self.p99);
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_experiment_result() {
        let result = ExperimentResult::new("test_exp")
            .with_metric("accuracy", 0.95)
            .with_metric("latency", 10.5);

        assert_eq!(result.experiment_name, "test_exp");
        assert!(result.passed);
        assert_eq!(result.metrics.get("accuracy"), Some(&0.95));
    }

    #[test]
    fn test_statistics() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = compute_statistics(&values);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.median, 3.0);
    }
}
