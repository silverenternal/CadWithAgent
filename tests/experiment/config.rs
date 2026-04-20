//! 实验配置管理模块
//!
//! 支持从 TOML 文件加载实验配置，实现灵活的参数管理。
//!
//! # 配置文件示例
//!
//! ```toml
//! # tests/experiment/config.toml
//!
//! [global]
//! verbose = true
//! output_dir = "tests/experiment/results"
//!
//! [exp1_accuracy]
//! enabled = true
//! num_samples = 1000
//! tolerance = 1e-10
//!
//! [exp2_performance]
//! enabled = true
//! num_samples = 100
//! data_sizes = [100, 500, 1000]
//! enable_baseline = true
//!
//! [exp3_vlm_reasoning]
//! enabled = false
//! num_samples = 20
//! api_endpoint = "http://localhost:8000"
//! ```

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// 是否启用详细输出
    #[serde(default = "default_verbose")]
    pub verbose: bool,
    /// 输出目录
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    /// 随机种子（用于可复现性）
    pub random_seed: Option<u64>,
    /// 超时时间（秒）
    pub timeout_secs: Option<f64>,
}

fn default_verbose() -> bool {
    false
}

fn default_output_dir() -> String {
    "tests/experiment/results".to_string()
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            output_dir: default_output_dir(),
            random_seed: None,
            timeout_secs: None,
        }
    }
}

/// 准确性实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_num_samples")]
    pub num_samples: usize,
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
}

impl Default for AccuracyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_samples: default_num_samples(),
            tolerance: default_tolerance(),
        }
    }
}

/// 性能实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_num_samples")]
    pub num_samples: usize,
    #[serde(default = "default_data_sizes")]
    pub data_sizes: Vec<usize>,
    #[serde(default = "default_false")]
    pub enable_baseline: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_samples: default_num_samples(),
            data_sizes: default_data_sizes(),
            enable_baseline: false,
        }
    }
}

fn default_data_sizes() -> Vec<usize> {
    vec![100, 500, 1000]
}

/// VLM 推理实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLMReasoningConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_num_samples_small")]
    pub num_samples: usize,
    pub api_endpoint: Option<String>,
    pub api_key: Option<String>,
    #[serde(default = "default_false")]
    pub enable_baseline: bool,
}

impl Default for VLMReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_samples: default_num_samples_small(),
            api_endpoint: None,
            api_key: None,
            enable_baseline: false,
        }
    }
}

/// 消融实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_num_samples_small")]
    pub num_samples: usize,
    #[serde(default = "default_false")]
    pub test_combinations: bool,
}

impl Default for AblationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_samples: default_num_samples_small(),
            test_combinations: false,
        }
    }
}

/// 案例研究实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseStudyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub output_dir: Option<String>,
}

impl Default for CaseStudyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output_dir: None,
        }
    }
}

/// 对比实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_num_samples_small")]
    pub num_samples: usize,
    #[serde(default = "default_true")]
    pub include_commercial: bool,
    #[serde(default = "default_true")]
    pub include_opensource: bool,
    #[serde(default = "default_true")]
    pub include_ai_tools: bool,
}

impl Default for ComparisonConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_samples: default_num_samples_small(),
            include_commercial: true,
            include_opensource: true,
            include_ai_tools: true,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_num_samples() -> usize {
    1000
}

fn default_num_samples_small() -> usize {
    50
}

fn default_tolerance() -> f64 {
    1e-10
}

/// 完整实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSuiteConfig {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub exp1_accuracy: AccuracyConfig,
    #[serde(default)]
    pub exp2_performance: PerformanceConfig,
    #[serde(default)]
    pub exp3_vlm_reasoning: VLMReasoningConfig,
    #[serde(default)]
    pub exp4_ablation: AblationConfig,
    #[serde(default)]
    pub exp5_case_studies: CaseStudyConfig,
    #[serde(default)]
    pub exp6_comparison: ComparisonConfig,
}

impl Default for ExperimentSuiteConfig {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            exp1_accuracy: AccuracyConfig {
                enabled: true,
                num_samples: default_num_samples(),
                tolerance: default_tolerance(),
            },
            exp2_performance: PerformanceConfig {
                enabled: true,
                num_samples: default_num_samples(),
                data_sizes: default_data_sizes(),
                enable_baseline: false,
            },
            exp3_vlm_reasoning: VLMReasoningConfig {
                enabled: true,
                num_samples: default_num_samples_small(),
                api_endpoint: None,
                api_key: None,
                enable_baseline: false,
            },
            exp4_ablation: AblationConfig {
                enabled: true,
                num_samples: default_num_samples_small(),
                test_combinations: false,
            },
            exp5_case_studies: CaseStudyConfig {
                enabled: true,
                output_dir: None,
            },
            exp6_comparison: ComparisonConfig {
                enabled: true,
                num_samples: default_num_samples_small(),
                include_commercial: true,
                include_opensource: true,
                include_ai_tools: true,
            },
        }
    }
}

impl ExperimentSuiteConfig {
    /// 从 TOML 文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path.as_ref())?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    /// 从字符串加载配置
    pub fn from_str(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = toml::from_str(content)?;
        Ok(config)
    }

    /// 保存配置到 TOML 文件
    pub fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path.as_ref(), content)?;
        Ok(())
    }

    /// 获取默认配置
    pub fn default_with_values() -> Self {
        Self {
            global: GlobalConfig {
                verbose: true,
                output_dir: "tests/experiment/results".to_string(),
                random_seed: Some(42),
                timeout_secs: Some(300.0),
            },
            exp1_accuracy: AccuracyConfig {
                enabled: true,
                num_samples: 1000,
                tolerance: 1e-10,
            },
            exp2_performance: PerformanceConfig {
                enabled: true,
                num_samples: 100,
                data_sizes: vec![100, 500, 1000],
                enable_baseline: true,
            },
            exp3_vlm_reasoning: VLMReasoningConfig {
                enabled: true,
                num_samples: 20,
                api_endpoint: None,
                api_key: None,
                enable_baseline: true,
            },
            exp4_ablation: AblationConfig {
                enabled: true,
                num_samples: 50,
                test_combinations: true,
            },
            exp5_case_studies: CaseStudyConfig {
                enabled: true,
                output_dir: Some("tests/experiment/results/case_studies".to_string()),
            },
            exp6_comparison: ComparisonConfig {
                enabled: true,
                num_samples: 50,
                include_commercial: true,
                include_opensource: true,
                include_ai_tools: true,
            },
        }
    }

    /// 获取启用的实验列表
    pub fn enabled_experiments(&self) -> Vec<&str> {
        let mut enabled = Vec::new();

        if self.exp1_accuracy.enabled {
            enabled.push("exp1_accuracy");
        }
        if self.exp2_performance.enabled {
            enabled.push("exp2_performance");
        }
        if self.exp3_vlm_reasoning.enabled {
            enabled.push("exp3_vlm_reasoning");
        }
        if self.exp4_ablation.enabled {
            enabled.push("exp4_ablation");
        }
        if self.exp5_case_studies.enabled {
            enabled.push("exp5_case_studies");
        }
        if self.exp6_comparison.enabled {
            enabled.push("exp6_comparison");
        }

        enabled
    }

    /// 检查实验是否启用
    pub fn is_enabled(&self, experiment: &str) -> bool {
        match experiment {
            "exp1_accuracy" => self.exp1_accuracy.enabled,
            "exp2_performance" => self.exp2_performance.enabled,
            "exp3_vlm_reasoning" => self.exp3_vlm_reasoning.enabled,
            "exp4_ablation" => self.exp4_ablation.enabled,
            "exp5_case_studies" => self.exp5_case_studies.enabled,
            "exp6_comparison" => self.exp6_comparison.enabled,
            _ => false,
        }
    }
}

/// 配置生成器
pub struct ConfigBuilder {
    config: ExperimentSuiteConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ExperimentSuiteConfig::default(),
        }
    }

    pub fn global<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut GlobalConfig),
    {
        f(&mut self.config.global);
        self
    }

    pub fn exp1<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut AccuracyConfig),
    {
        f(&mut self.config.exp1_accuracy);
        self
    }

    pub fn exp2<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut PerformanceConfig),
    {
        f(&mut self.config.exp2_performance);
        self
    }

    pub fn exp3<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut VLMReasoningConfig),
    {
        f(&mut self.config.exp3_vlm_reasoning);
        self
    }

    pub fn exp4<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut AblationConfig),
    {
        f(&mut self.config.exp4_ablation);
        self
    }

    pub fn exp5<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut CaseStudyConfig),
    {
        f(&mut self.config.exp5_case_studies);
        self
    }

    pub fn exp6<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut ComparisonConfig),
    {
        f(&mut self.config.exp6_comparison);
        self
    }

    pub fn build(self) -> ExperimentSuiteConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成默认配置文件
pub fn generate_default_config<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let config = ExperimentSuiteConfig::default_with_values();
    config.save_to(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_str() {
        let toml_str = r#"
            [global]
            verbose = true
            output_dir = "custom_results"

            [exp1_accuracy]
            enabled = true
            num_samples = 500
            tolerance = 1e-8
        "#;

        let config = ExperimentSuiteConfig::from_str(toml_str).unwrap();

        assert!(config.global.verbose);
        assert_eq!(config.global.output_dir, "custom_results");
        assert_eq!(config.exp1_accuracy.num_samples, 500);
        assert_eq!(config.exp1_accuracy.tolerance, 1e-8);
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .global(|g| {
                g.verbose = true;
                g.random_seed = Some(123);
            })
            .exp1(|e| {
                e.num_samples = 2000;
            })
            .build();

        assert!(config.global.verbose);
        assert_eq!(config.global.random_seed, Some(123));
        assert_eq!(config.exp1_accuracy.num_samples, 2000);
    }

    #[test]
    fn test_enabled_experiments() {
        let config = ExperimentSuiteConfig::default();
        let enabled = config.enabled_experiments();

        // 默认所有实验都启用
        assert_eq!(enabled.len(), 6);
    }

    #[test]
    fn test_disable_experiment() {
        let toml_str = r#"
            [exp1_accuracy]
            enabled = false
        "#;

        let config: ExperimentSuiteConfig = toml::from_str(toml_str).unwrap();

        assert!(!config.is_enabled("exp1_accuracy"));
        assert!(config.is_enabled("exp2_performance"));
    }

    #[test]
    fn test_global_config_default() {
        let config = GlobalConfig::default();
        assert!(!config.verbose);
        assert_eq!(config.output_dir, "tests/experiment/results");
        assert!(config.random_seed.is_none());
        assert!(config.timeout_secs.is_none());
    }

    #[test]
    fn test_accuracy_config_default() {
        let config = AccuracyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.num_samples, 1000);
        assert!((config.tolerance - 1e-10).abs() < 1e-20);
    }

    #[test]
    fn test_performance_config_default() {
        let config = PerformanceConfig::default();
        assert!(config.enabled);
        assert_eq!(config.num_samples, 1000);
        assert_eq!(config.data_sizes, vec![100, 500, 1000]);
        assert!(!config.enable_baseline);
    }

    #[test]
    fn test_vlm_reasoning_config_default() {
        let config = VLMReasoningConfig::default();
        assert!(config.enabled);
        assert_eq!(config.num_samples, 50);
        assert!(config.api_endpoint.is_none());
        assert!(config.api_key.is_none());
        assert!(!config.enable_baseline);
    }

    #[test]
    fn test_ablation_config_default() {
        let config = AblationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.num_samples, 50);
        assert!(!config.test_combinations);
    }

    #[test]
    fn test_case_study_config_default() {
        let config = CaseStudyConfig::default();
        assert!(config.enabled);
        assert!(config.output_dir.is_none());
    }

    #[test]
    fn test_comparison_config_default() {
        let config = ComparisonConfig::default();
        assert!(config.enabled);
        assert_eq!(config.num_samples, 50);
        assert!(config.include_commercial);
        assert!(config.include_opensource);
        assert!(config.include_ai_tools);
    }

    #[test]
    fn test_is_enabled_unknown_experiment() {
        let config = ExperimentSuiteConfig::default();
        assert!(!config.is_enabled("unknown_experiment"));
    }

    #[test]
    fn test_config_builder_chaining() {
        let config = ConfigBuilder::new()
            .global(|g| {
                g.verbose = true;
            })
            .exp1(|e| {
                e.num_samples = 100;
            })
            .exp2(|e| {
                e.num_samples = 200;
            })
            .exp3(|e| {
                e.num_samples = 300;
            })
            .exp4(|e| {
                e.num_samples = 400;
            })
            .exp5(|e| {
                e.output_dir = Some("/tmp".to_string());
            })
            .exp6(|e| {
                e.num_samples = 600;
            })
            .build();

        assert!(config.global.verbose);
        assert_eq!(config.exp1_accuracy.num_samples, 100);
        assert_eq!(config.exp2_performance.num_samples, 200);
        assert_eq!(config.exp3_vlm_reasoning.num_samples, 300);
        assert_eq!(config.exp4_ablation.num_samples, 400);
        assert_eq!(
            config.exp5_case_studies.output_dir,
            Some("/tmp".to_string())
        );
        assert_eq!(config.exp6_comparison.num_samples, 600);
    }
}
