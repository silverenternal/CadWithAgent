//! 实验指标模块
//!
//! 定义各类实验的评估指标。

#![allow(dead_code)]

use super::utils::Statistics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 准确性指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyMetrics {
    /// 测试名称
    pub test_name: String,
    /// 总测试用例数
    pub total_cases: usize,
    /// 通过的测试用例数
    pub passed_cases: usize,
    /// 准确率 (0-1)
    pub accuracy: f64,
    /// 最大绝对误差
    pub max_absolute_error: f64,
    /// 最大相对误差
    pub max_relative_error: f64,
    /// 平均绝对误差
    pub avg_absolute_error: f64,
    /// 平均相对误差
    pub avg_relative_error: f64,
    /// 误差统计
    pub error_statistics: Option<Statistics>,
}

impl AccuracyMetrics {
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            total_cases: 0,
            passed_cases: 0,
            accuracy: 0.0,
            max_absolute_error: 0.0,
            max_relative_error: 0.0,
            avg_absolute_error: 0.0,
            avg_relative_error: 0.0,
            error_statistics: None,
        }
    }

    pub fn from_errors(
        test_name: &str,
        total_cases: usize,
        passed_cases: usize,
        absolute_errors: &[f64],
        relative_errors: &[f64],
    ) -> Self {
        let accuracy = if total_cases > 0 {
            passed_cases as f64 / total_cases as f64
        } else {
            0.0
        };

        let max_absolute_error = absolute_errors.iter().cloned().fold(0.0_f64, f64::max);
        let max_relative_error = relative_errors.iter().cloned().fold(0.0_f64, f64::max);

        let avg_absolute_error = if !absolute_errors.is_empty() {
            absolute_errors.iter().sum::<f64>() / absolute_errors.len() as f64
        } else {
            0.0
        };

        let avg_relative_error = if !relative_errors.is_empty() {
            relative_errors.iter().sum::<f64>() / relative_errors.len() as f64
        } else {
            0.0
        };

        let error_statistics = Some(super::utils::compute_statistics(absolute_errors));

        Self {
            test_name: test_name.to_string(),
            total_cases,
            passed_cases,
            accuracy,
            max_absolute_error,
            max_relative_error,
            avg_absolute_error,
            avg_relative_error,
            error_statistics,
        }
    }

    pub fn to_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert("accuracy".to_string(), self.accuracy);
        map.insert("max_absolute_error".to_string(), self.max_absolute_error);
        map.insert("max_relative_error".to_string(), self.max_relative_error);
        map.insert("avg_absolute_error".to_string(), self.avg_absolute_error);
        map.insert("avg_relative_error".to_string(), self.avg_relative_error);
        if let Some(stats) = &self.error_statistics {
            for (key, value) in stats.to_map() {
                map.insert(format!("error_{}", key), value);
            }
        }
        map
    }
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 测试名称
    pub test_name: String,
    /// 吞吐量 (ops/sec)
    pub throughput: f64,
    /// 平均延迟 (ms)
    pub avg_latency_ms: f64,
    /// 延迟统计
    pub latency_statistics: Statistics,
    /// 内存使用 (MB)
    pub memory_usage_mb: Option<f64>,
    /// CPU 使用率 (%)
    pub cpu_usage_percent: Option<f64>,
    /// 加速比 (相比基准)
    pub speedup: Option<f64>,
}

impl PerformanceMetrics {
    pub fn new(test_name: &str, latencies_ms: &[f64]) -> Self {
        let latency_statistics = super::utils::compute_statistics(latencies_ms);

        let throughput = if latency_statistics.mean > 0.0 {
            1000.0 / latency_statistics.mean // ops per second
        } else {
            0.0
        };

        Self {
            test_name: test_name.to_string(),
            throughput,
            avg_latency_ms: latency_statistics.mean,
            latency_statistics,
            memory_usage_mb: None,
            cpu_usage_percent: None,
            speedup: None,
        }
    }

    pub fn with_memory(mut self, memory_mb: f64) -> Self {
        self.memory_usage_mb = Some(memory_mb);
        self
    }

    pub fn with_cpu(mut self, cpu_percent: f64) -> Self {
        self.cpu_usage_percent = Some(cpu_percent);
        self
    }

    pub fn with_speedup(mut self, speedup: f64) -> Self {
        self.speedup = Some(speedup);
        self
    }

    pub fn to_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        map.insert("throughput".to_string(), self.throughput);
        map.insert("avg_latency_ms".to_string(), self.avg_latency_ms);
        map.insert("p50_latency_ms".to_string(), self.latency_statistics.median);
        map.insert("p95_latency_ms".to_string(), self.latency_statistics.p95);
        map.insert("p99_latency_ms".to_string(), self.latency_statistics.p99);
        map.insert("min_latency_ms".to_string(), self.latency_statistics.min);
        map.insert("max_latency_ms".to_string(), self.latency_statistics.max);
        map.insert(
            "std_dev_latency_ms".to_string(),
            self.latency_statistics.std_dev,
        );

        if let Some(mem) = self.memory_usage_mb {
            map.insert("memory_usage_mb".to_string(), mem);
        }
        if let Some(cpu) = self.cpu_usage_percent {
            map.insert("cpu_usage_percent".to_string(), cpu);
        }
        if let Some(speedup) = self.speedup {
            map.insert("speedup".to_string(), speedup);
        }
        map
    }
}

/// 质量指标 (用于 VLM 推理质量评估)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// 测试名称
    pub test_name: String,
    /// BLEU 分数
    pub bleu_score: Option<f64>,
    /// ROUGE-L 分数
    pub rouge_l_score: Option<f64>,
    /// 语义相似度 (0-1)
    pub semantic_similarity: Option<f64>,
    /// 人工评分 (1-5)
    pub human_rating: Option<f64>,
    /// 推理步骤正确率
    pub reasoning_step_accuracy: f64,
    /// 最终答案正确率
    pub final_answer_accuracy: f64,
    /// 幻觉率
    pub hallucination_rate: f64,
    /// 响应时间 (秒)
    pub response_time_secs: f64,
}

impl QualityMetrics {
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            bleu_score: None,
            rouge_l_score: None,
            semantic_similarity: None,
            human_rating: None,
            reasoning_step_accuracy: 0.0,
            final_answer_accuracy: 0.0,
            hallucination_rate: 0.0,
            response_time_secs: 0.0,
        }
    }

    pub fn bleu(mut self, score: f64) -> Self {
        self.bleu_score = Some(score);
        self
    }

    pub fn rouge_l(mut self, score: f64) -> Self {
        self.rouge_l_score = Some(score);
        self
    }

    pub fn semantic_similarity(mut self, score: f64) -> Self {
        self.semantic_similarity = Some(score);
        self
    }

    pub fn human_rating(mut self, rating: f64) -> Self {
        self.human_rating = Some(rating);
        self
    }

    pub fn reasoning_accuracy(mut self, accuracy: f64) -> Self {
        self.reasoning_step_accuracy = accuracy;
        self
    }

    pub fn final_answer_accuracy(mut self, accuracy: f64) -> Self {
        self.final_answer_accuracy = accuracy;
        self
    }

    pub fn hallucination_rate(mut self, rate: f64) -> Self {
        self.hallucination_rate = rate;
        self
    }

    pub fn response_time(mut self, secs: f64) -> Self {
        self.response_time_secs = secs;
        self
    }

    pub fn to_map(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        if let Some(bleu) = self.bleu_score {
            map.insert("bleu_score".to_string(), bleu);
        }
        if let Some(rouge) = self.rouge_l_score {
            map.insert("rouge_l_score".to_string(), rouge);
        }
        if let Some(sim) = self.semantic_similarity {
            map.insert("semantic_similarity".to_string(), sim);
        }
        if let Some(rating) = self.human_rating {
            map.insert("human_rating".to_string(), rating);
        }
        map.insert(
            "reasoning_step_accuracy".to_string(),
            self.reasoning_step_accuracy,
        );
        map.insert(
            "final_answer_accuracy".to_string(),
            self.final_answer_accuracy,
        );
        map.insert("hallucination_rate".to_string(), self.hallucination_rate);
        map.insert("response_time_secs".to_string(), self.response_time_secs);
        map
    }
}

/// 消融实验指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationMetrics {
    /// 配置名称
    pub config_name: String,
    /// 是否包含某模块
    pub module_enabled: HashMap<String, bool>,
    /// 性能指标
    pub performance: HashMap<String, f64>,
    /// 相对基准的下降百分比
    pub degradation_percent: HashMap<String, f64>,
}

impl AblationMetrics {
    pub fn new(config_name: &str) -> Self {
        Self {
            config_name: config_name.to_string(),
            module_enabled: HashMap::new(),
            performance: HashMap::new(),
            degradation_percent: HashMap::new(),
        }
    }

    pub fn with_module(mut self, name: &str, enabled: bool) -> Self {
        self.module_enabled.insert(name.to_string(), enabled);
        self
    }

    pub fn with_performance(mut self, metric: &str, value: f64) -> Self {
        self.performance.insert(metric.to_string(), value);
        self
    }

    pub fn with_degradation(mut self, metric: &str, percent: f64) -> Self {
        self.degradation_percent.insert(metric.to_string(), percent);
        self
    }
}

/// 对比实验指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetrics {
    /// 方法名称
    pub method_name: String,
    /// 各项指标
    pub metrics: HashMap<String, f64>,
    /// 排名
    pub rank: HashMap<String, usize>,
    /// 是否显著优于基准 (p-value < 0.05)
    pub significantly_better: HashMap<String, bool>,
}

impl ComparisonMetrics {
    pub fn new(method_name: &str) -> Self {
        Self {
            method_name: method_name.to_string(),
            metrics: HashMap::new(),
            rank: HashMap::new(),
            significantly_better: HashMap::new(),
        }
    }

    pub fn with_metric(mut self, name: &str, value: f64) -> Self {
        self.metrics.insert(name.to_string(), value);
        self
    }

    pub fn with_rank(mut self, metric: &str, rank: usize) -> Self {
        self.rank.insert(metric.to_string(), rank);
        self
    }

    pub fn with_significance(mut self, metric: &str, significant: bool) -> Self {
        self.significantly_better
            .insert(metric.to_string(), significant);
        self
    }
}

/// 案例研究指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseStudyMetrics {
    /// 案例名称
    pub case_name: String,
    /// 案例描述
    pub description: String,
    /// 输入复杂度
    pub input_complexity: InputComplexity,
    /// 输出质量评分
    pub output_quality: f64,
    /// 用户满意度 (1-5)
    pub user_satisfaction: Option<f64>,
    /// 关键成功因素
    pub success_factors: Vec<String>,
    /// 遇到的挑战
    pub challenges: Vec<String>,
    /// 经验教训
    pub lessons_learned: Vec<String>,
}

impl CaseStudyMetrics {
    pub fn new(case_name: &str, description: &str) -> Self {
        Self {
            case_name: case_name.to_string(),
            description: description.to_string(),
            output_quality: 0.0,
            user_satisfaction: None,
            input_complexity: InputComplexity::default(),
            success_factors: Vec::new(),
            challenges: Vec::new(),
            lessons_learned: Vec::new(),
        }
    }

    pub fn input_complexity(mut self, complexity: InputComplexity) -> Self {
        self.input_complexity = complexity;
        self
    }

    pub fn output_quality(mut self, quality: f64) -> Self {
        self.output_quality = quality;
        self
    }

    pub fn user_satisfaction(mut self, satisfaction: f64) -> Self {
        self.user_satisfaction = Some(satisfaction);
        self
    }

    pub fn add_success_factor(mut self, factor: &str) -> Self {
        self.success_factors.push(factor.to_string());
        self
    }

    pub fn add_challenge(mut self, challenge: &str) -> Self {
        self.challenges.push(challenge.to_string());
        self
    }

    pub fn add_lesson(mut self, lesson: &str) -> Self {
        self.lessons_learned.push(lesson.to_string());
        self
    }
}

/// 输入复杂度指标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InputComplexity {
    /// 几何元素数量
    pub num_elements: usize,
    /// 约束数量
    pub num_constraints: usize,
    /// 嵌套深度
    pub nesting_depth: usize,
    /// 特殊几何特征数量
    pub num_special_features: usize,
}

impl InputComplexity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn elements(mut self, count: usize) -> Self {
        self.num_elements = count;
        self
    }

    pub fn constraints(mut self, count: usize) -> Self {
        self.num_constraints = count;
        self
    }

    pub fn depth(mut self, depth: usize) -> Self {
        self.nesting_depth = depth;
        self
    }

    pub fn special_features(mut self, count: usize) -> Self {
        self.num_special_features = count;
        self
    }

    /// 计算综合复杂度分数 (0-100)
    pub fn complexity_score(&self) -> f64 {
        let element_score = (self.num_elements as f64 / 100.0).min(1.0);
        let constraint_score = (self.num_constraints as f64 / 50.0).min(1.0);
        let depth_score = (self.nesting_depth as f64 / 10.0).min(1.0);
        let feature_score = (self.num_special_features as f64 / 20.0).min(1.0);

        (element_score * 0.3 + constraint_score * 0.3 + depth_score * 0.2 + feature_score * 0.2)
            * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accuracy_metrics() {
        let absolute_errors = vec![0.001, 0.002, 0.0015, 0.003, 0.001];
        let relative_errors = vec![0.01, 0.02, 0.015, 0.03, 0.01];

        let metrics =
            AccuracyMetrics::from_errors("test", 5, 5, &absolute_errors, &relative_errors);

        assert_eq!(metrics.accuracy, 1.0);
        assert!((metrics.max_absolute_error - 0.003).abs() < 1e-10);
    }

    #[test]
    fn test_performance_metrics() {
        let latencies = vec![10.0, 15.0, 12.0, 18.0, 11.0];
        let metrics = PerformanceMetrics::new("benchmark", &latencies);

        assert!(metrics.throughput > 0.0);
        assert!(metrics.avg_latency_ms > 10.0);
    }

    #[test]
    fn test_input_complexity() {
        let complexity = InputComplexity::new()
            .elements(50)
            .constraints(25)
            .depth(5)
            .special_features(10);

        let score = complexity.complexity_score();
        assert!(score > 0.0 && score <= 100.0);
    }
}
