//! 会议/期刊配置模板
//!
//! 提供不同顶会/期刊的实验配置模板，包括格式要求、评估指标、统计要求等。
//!
//! # 支持的 venue
//!
//! - SIGGRAPH / SIGGRAPH Asia
//! - CHI (Human Factors in Computing Systems)
//! - UIST (User Interface Software and Technology)
//! - CVPR (Computer Vision and Pattern Recognition)
//! - ICCV (International Conference on Computer Vision)
//! - IEEE TVCG (Transactions on Visualization and Computer Graphics)
//! - ACM TOG (Transactions on Graphics)
//! - C&C (Creativity and Cognition)

#![allow(dead_code, clippy::upper_case_acronyms)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Venue 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VenueType {
    Siggraph,
    SiggraphAsia,
    CHI,
    UIST,
    CVPR,
    ICCV,
    IEEETVCG,
    ACMTOG,
    CreativityAndCognition,
    Custom,
}

impl VenueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VenueType::Siggraph => "SIGGRAPH",
            VenueType::SiggraphAsia => "SIGGRAPH Asia",
            VenueType::CHI => "CHI",
            VenueType::UIST => "UIST",
            VenueType::CVPR => "CVPR",
            VenueType::ICCV => "ICCV",
            VenueType::IEEETVCG => "IEEE TVCG",
            VenueType::ACMTOG => "ACM TOG",
            VenueType::CreativityAndCognition => "Creativity & Cognition",
            VenueType::Custom => "Custom",
        }
    }
}

/// Venue 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueConfig {
    /// Venue 名称
    pub name: String,
    /// Venue 类型
    pub venue_type: VenueType,
    /// 页面限制
    pub page_limit: PageLimit,
    /// 格式要求
    pub format_requirements: FormatRequirements,
    /// 统计要求
    pub statistical_requirements: StatisticalRequirements,
    /// 推荐实验列表
    pub recommended_experiments: Vec<String>,
    /// 特殊要求说明
    pub special_notes: String,
}

impl VenueConfig {
    /// 创建 SIGGRAPH 配置
    pub fn siggraph() -> Self {
        Self {
            name: "SIGGRAPH".to_string(),
            venue_type: VenueType::Siggraph,
            page_limit: PageLimit {
                max_pages: 12,
                unlimited_references: true,
                additional_pages_fee: true,
            },
            format_requirements: FormatRequirements {
                template: "ACM SIGGRAPH".to_string(),
                citation_style: "ACM".to_string(),
                figure_format: "PDF, PNG (300+ DPI)".to_string(),
                color_policy: "Color allowed".to_string(),
            },
            statistical_requirements: StatisticalRequirements {
                require_effect_sizes: true,
                require_confidence_intervals: true,
                require_power_analysis: false,
                p_value_threshold: 0.05,
                require_multiple_testing_correction: true,
            },
            recommended_experiments: vec![
                "Geometric Accuracy Validation".to_string(),
                "Performance Benchmarks".to_string(),
                "Comparison with State-of-the-Art".to_string(),
                "User Study (if applicable)".to_string(),
                "Case Studies".to_string(),
            ],
            special_notes: "Emphasize technical novelty and visual results. Include high-quality renderings and comparisons.".to_string(),
        }
    }

    /// 创建 CHI 配置
    pub fn chi() -> Self {
        Self {
            name: "CHI".to_string(),
            venue_type: VenueType::CHI,
            page_limit: PageLimit {
                max_pages: 12,
                unlimited_references: false,
                additional_pages_fee: true,
            },
            format_requirements: FormatRequirements {
                template: "ACM CHI".to_string(),
                citation_style: "ACM".to_string(),
                figure_format: "PDF, PNG (300+ DPI)".to_string(),
                color_policy: "Color allowed".to_string(),
            },
            statistical_requirements: StatisticalRequirements {
                require_effect_sizes: true,
                require_confidence_intervals: true,
                require_power_analysis: true,
                p_value_threshold: 0.05,
                require_multiple_testing_correction: true,
            },
            recommended_experiments: vec![
                "User Study with Statistical Analysis".to_string(),
                "Task Completion Time & Accuracy".to_string(),
                "Subjective Measures (SUS, NASA-TLX, etc.)".to_string(),
                "Qualitative Analysis".to_string(),
                "System Performance Metrics".to_string(),
            ],
            special_notes: "Strong emphasis on human factors contribution. Include participant demographics, procedure details, and ethical considerations.".to_string(),
        }
    }

    /// 创建 UIST 配置
    pub fn uist() -> Self {
        Self {
            name: "UIST".to_string(),
            venue_type: VenueType::UIST,
            page_limit: PageLimit {
                max_pages: 12,
                unlimited_references: false,
                additional_pages_fee: true,
            },
            format_requirements: FormatRequirements {
                template: "ACM UIST".to_string(),
                citation_style: "ACM".to_string(),
                figure_format: "PDF, PNG (300+ DPI)".to_string(),
                color_policy: "Color allowed".to_string(),
            },
            statistical_requirements: StatisticalRequirements {
                require_effect_sizes: true,
                require_confidence_intervals: false,
                require_power_analysis: false,
                p_value_threshold: 0.05,
                require_multiple_testing_correction: true,
            },
            recommended_experiments: vec![
                "Technical System Evaluation".to_string(),
                "User Study (if applicable)".to_string(),
                "Performance Benchmarks".to_string(),
                "Comparison with Existing Tools".to_string(),
            ],
            special_notes: "Focus on novel UI technology and implementation. Include system architecture and technical details.".to_string(),
        }
    }

    /// 创建 CVPR 配置
    pub fn cvpr() -> Self {
        Self {
            name: "CVPR".to_string(),
            venue_type: VenueType::CVPR,
            page_limit: PageLimit {
                max_pages: 9,
                unlimited_references: true,
                additional_pages_fee: false,
            },
            format_requirements: FormatRequirements {
                template: "IEEE CVPR".to_string(),
                citation_style: "IEEE".to_string(),
                figure_format: "PDF, PNG (300+ DPI)".to_string(),
                color_policy: "Color allowed".to_string(),
            },
            statistical_requirements: StatisticalRequirements {
                require_effect_sizes: false,
                require_confidence_intervals: false,
                require_power_analysis: false,
                p_value_threshold: 0.05,
                require_multiple_testing_correction: false,
            },
            recommended_experiments: vec![
                "Benchmark Dataset Evaluation".to_string(),
                "Comparison with SOTA Methods".to_string(),
                "Ablation Study".to_string(),
                "Qualitative Results".to_string(),
                "Runtime Analysis".to_string(),
            ],
            special_notes: "Emphasize computer vision novelty. Include comprehensive comparisons on standard benchmarks.".to_string(),
        }
    }

    /// 创建 IEEE TVCG 配置
    pub fn ieee_tvcg() -> Self {
        Self {
            name: "IEEE TVCG".to_string(),
            venue_type: VenueType::IEEETVCG,
            page_limit: PageLimit {
                max_pages: 14,
                unlimited_references: false,
                additional_pages_fee: true,
            },
            format_requirements: FormatRequirements {
                template: "IEEE TVCG".to_string(),
                citation_style: "IEEE".to_string(),
                figure_format: "PDF, PNG, TIFF (300+ DPI)".to_string(),
                color_policy: "Color allowed (online), B&W optional (print)".to_string(),
            },
            statistical_requirements: StatisticalRequirements {
                require_effect_sizes: true,
                require_confidence_intervals: true,
                require_power_analysis: true,
                p_value_threshold: 0.05,
                require_multiple_testing_correction: true,
            },
            recommended_experiments: vec![
                "Comprehensive Quantitative Evaluation".to_string(),
                "Statistical Analysis with Effect Sizes".to_string(),
                "User Study (if applicable)".to_string(),
                "Performance Benchmarks".to_string(),
                "Comparison with Multiple Baselines".to_string(),
            ],
            special_notes: "Journal-length paper with comprehensive evaluation. Include detailed methodology and thorough analysis.".to_string(),
        }
    }

    /// 获取配置
    pub fn get_config(venue_type: VenueType) -> Self {
        match venue_type {
            VenueType::Siggraph => Self::siggraph(),
            VenueType::SiggraphAsia => Self::siggraph(), // Similar to SIGGRAPH
            VenueType::CHI => Self::chi(),
            VenueType::UIST => Self::uist(),
            VenueType::CVPR => Self::cvpr(),
            VenueType::ICCV => Self::cvpr(), // Similar to CVPR
            VenueType::IEEETVCG => Self::ieee_tvcg(),
            VenueType::ACMTOG => Self::siggraph(), // Similar to SIGGRAPH
            VenueType::CreativityAndCognition => Self::chi(), // Similar to CHI
            VenueType::Custom => Self::default(),
        }
    }

    /// 生成实验清单
    pub fn generate_experiment_checklist(&self) -> Vec<ExperimentChecklistItem> {
        let mut items = Vec::new();

        // 基础要求
        items.push(ExperimentChecklistItem {
            category: "General".to_string(),
            requirement: "Clear research questions stated".to_string(),
            required: true,
            notes: String::new(),
        });

        items.push(ExperimentChecklistItem {
            category: "General".to_string(),
            requirement: format!("Page limit: {} pages", self.page_limit.max_pages),
            required: true,
            notes: String::new(),
        });

        // 统计要求
        if self.statistical_requirements.require_effect_sizes {
            items.push(ExperimentChecklistItem {
                category: "Statistics".to_string(),
                requirement: "Report effect sizes (Cohen's d, η², etc.)".to_string(),
                required: true,
                notes: String::new(),
            });
        }

        if self.statistical_requirements.require_confidence_intervals {
            items.push(ExperimentChecklistItem {
                category: "Statistics".to_string(),
                requirement: "Include 95% confidence intervals".to_string(),
                required: true,
                notes: String::new(),
            });
        }

        if self.statistical_requirements.require_power_analysis {
            items.push(ExperimentChecklistItem {
                category: "Statistics".to_string(),
                requirement: "Report power analysis for sample size justification".to_string(),
                required: true,
                notes: String::new(),
            });
        }

        if self
            .statistical_requirements
            .require_multiple_testing_correction
        {
            items.push(ExperimentChecklistItem {
                category: "Statistics".to_string(),
                requirement: "Apply multiple testing correction (Bonferroni, etc.)".to_string(),
                required: true,
                notes: String::new(),
            });
        }

        // 推荐实验
        for exp in &self.recommended_experiments {
            items.push(ExperimentChecklistItem {
                category: "Experiments".to_string(),
                requirement: exp.clone(),
                required: false,
                notes: "Recommended".to_string(),
            });
        }

        items
    }

    /// 保存配置
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }
}

impl Default for VenueConfig {
    fn default() -> Self {
        Self::siggraph()
    }
}

/// 页面限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLimit {
    /// 最大页数
    pub max_pages: usize,
    /// 参考文献是否不计页数
    pub unlimited_references: bool,
    /// 超页是否收费
    pub additional_pages_fee: bool,
}

/// 格式要求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatRequirements {
    /// 模板名称
    pub template: String,
    /// 引用格式
    pub citation_style: String,
    /// 图片格式要求
    pub figure_format: String,
    /// 颜色政策
    pub color_policy: String,
}

/// 统计要求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalRequirements {
    /// 是否需要效应量
    pub require_effect_sizes: bool,
    /// 是否需要置信区间
    pub require_confidence_intervals: bool,
    /// 是否需要功效分析
    pub require_power_analysis: bool,
    /// p 值阈值
    pub p_value_threshold: f64,
    /// 是否需要多重检验校正
    pub require_multiple_testing_correction: bool,
}

/// 实验清单项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentChecklistItem {
    /// 类别
    pub category: String,
    /// 要求描述
    pub requirement: String,
    /// 是否必需
    pub required: bool,
    /// 备注
    pub notes: String,
}

/// 实验配置生成器
#[allow(dead_code)]
pub struct ExperimentConfigGenerator;

#[allow(dead_code)]
impl ExperimentConfigGenerator {
    /// 为指定 venue 生成配置
    pub fn generate(venue_type: VenueType) -> VenueConfig {
        VenueConfig::get_config(venue_type)
    }

    /// 生成所有 venue 的配置
    pub fn generate_all() -> HashMap<String, VenueConfig> {
        let mut configs = HashMap::new();

        configs.insert("siggraph".to_string(), VenueConfig::siggraph());
        configs.insert("chi".to_string(), VenueConfig::chi());
        configs.insert("uist".to_string(), VenueConfig::uist());
        configs.insert("cvpr".to_string(), VenueConfig::cvpr());
        configs.insert("ieee_tvcg".to_string(), VenueConfig::ieee_tvcg());

        configs
    }

    /// 生成配置对比表格 (LaTeX)
    pub fn generate_comparison_table() -> String {
        let configs = Self::generate_all();
        let mut latex = String::new();

        latex.push_str("\\begin{table}[t]\n");
        latex.push_str("\\centering\n");
        latex.push_str("\\caption{Comparison of Venue Requirements}\n");
        latex.push_str("\\label{tab:venue_comparison}\n");
        latex.push_str("\\begin{tabular}{lccccc}\n");
        latex.push_str("\\toprule\n");
        latex.push_str("\\textbf{Venue} & \\textbf{Pages} & \\textbf{Effect Sizes} & \\textbf{CI} & \\textbf{Power} & \\textbf{p-value} \\\\\n");
        latex.push_str("\\midrule\n");

        for config in configs.values() {
            latex.push_str(&format!(
                "{} & {} & {} & {} & {} & {:.2} \\\\\n",
                config.name,
                config.page_limit.max_pages,
                if config.statistical_requirements.require_effect_sizes {
                    "✓"
                } else {
                    "✗"
                },
                if config.statistical_requirements.require_confidence_intervals {
                    "✓"
                } else {
                    "✗"
                },
                if config.statistical_requirements.require_power_analysis {
                    "✓"
                } else {
                    "✗"
                },
                config.statistical_requirements.p_value_threshold,
            ));
        }

        latex.push_str("\\bottomrule\n");
        latex.push_str("\\end{tabular}\n");
        latex.push_str("\\end{table}\n");

        latex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_venue_config_creation() {
        let siggraph = VenueConfig::siggraph();
        assert_eq!(siggraph.name, "SIGGRAPH");
        assert_eq!(siggraph.page_limit.max_pages, 12);
        assert!(siggraph.statistical_requirements.require_effect_sizes);
    }

    #[test]
    fn test_chi_config() {
        let chi = VenueConfig::chi();
        assert_eq!(chi.name, "CHI");
        assert!(chi.statistical_requirements.require_power_analysis);
    }

    #[test]
    fn test_cvpr_config() {
        let cvpr = VenueConfig::cvpr();
        assert_eq!(cvpr.name, "CVPR");
        assert_eq!(cvpr.page_limit.max_pages, 9);
        assert!(!cvpr.statistical_requirements.require_effect_sizes);
    }

    #[test]
    fn test_checklist_generation() {
        let config = VenueConfig::chi();
        let checklist = config.generate_experiment_checklist();

        assert!(!checklist.is_empty());
        assert!(checklist
            .iter()
            .any(|i| i.requirement.contains("power analysis")));
    }
}
