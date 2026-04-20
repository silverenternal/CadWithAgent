//! 顶会论文实验验证框架
//!
//! # 实验设计概述
//!
//! 本实验框架旨在全面验证 CadAgent 的核心贡献，为顶会论文提供充分的实验数据支持。
//!
//! ## 实验列表
//!
//! | 实验编号 | 实验名称 | 验证目标 | 对应论文章节 |
//! |----------|----------|----------|--------------|
//! | Exp-1 | 几何计算准确性验证 | 验证确定性几何算法的 100% 准确性 | §5.1 Accuracy |
//! | Exp-2 | 性能基准测试 | 验证 R-tree 空间索引的性能优势 | §5.2 Performance |
//! | Exp-3 | VLM 推理质量对比 | 验证工具增强上下文注入的有效性 | §5.3 Reasoning Quality |
//! | Exp-4 | 消融实验 | 验证各模块的贡献度 | §5.4 Ablation Study |
//! | Exp-5 | 真实案例研究 | 验证实际应用场景的有效性 | §5.5 Case Studies |
//! | Exp-6 | 对比实验 | 与现有方法的全面对比 | §5.6 Comparison |
//!
//! ## 使用方式
//!
//! ```bash
//! # 运行单个实验
//! cargo test --test experiment -- exp1_accuracy_validation --nocapture
//!
//! # 运行所有实验
//! cargo test --test experiment -- --nocapture
//!
//! # 生成实验报告
//! cargo run --bin experiment_runner
//! ```
//!
//! ## 实验数据管理
//!
//! - `tests/experiment/data/`: 原始实验数据
//! - `tests/experiment/results/`: 处理后的结果
//! - `tests/experiment/fixtures/`: 测试 fixtures
//! - `tests/experiment/scripts/`: 可视化脚本

pub mod exp1_accuracy;
pub mod exp2_performance;
pub mod exp3_vlm_reasoning;
pub mod exp4_ablation;
pub mod exp5_case_studies;
pub mod exp6_comparison;

pub mod assertion_utils;
pub mod config;
pub mod cubicasa5k;
pub mod data;
pub mod error;
pub mod fixtures;
pub mod latex_tables;
pub mod metrics;
pub mod parallel;
pub mod reproducibility;
pub mod runner;
pub mod statistics;
pub mod utils;
pub mod validity_threats;
pub mod venue_configs;
pub mod visualization;

// 注意：宏在 crate 根级别导出，不需要在这里 use
// 使用方式：use cadagent::assert_within_tolerance;

// 重新导出断言工具
#[allow(unused_imports)]
pub use assertion_utils::*;

// 重新导出可视化工具
#[allow(unused_imports)]
pub use visualization::*;

// 重新导出并行工具
#[allow(unused_imports)]
pub use parallel::{
    run_experiment_suite_parallel, ParallelRunner, ParallelRunnerBuilder, ParallelRunnerConfig,
};

// 重新导出配置工具
#[allow(unused_imports)]
pub use config::{generate_default_config, ConfigBuilder, ExperimentSuiteConfig, GlobalConfig};

// 重新导出 CubiCasa5k 工具
#[allow(unused_imports)]
pub use cubicasa5k::{
    analyze_room_layout, parse_svg_elements, HouseData, RoomAnalysis, SvgCircle, SvgElements,
    SvgLine, SvgPath, SvgRect, SvgText,
};

// 重新导出 fixtures 工具
#[allow(unused_imports)]
pub use fixtures::{
    create_concentric_circles, create_large_geometry_dataset, create_parallel_lines,
    create_perpendicular_lines, create_standard_geometries, create_tangent_circle_line,
    create_test_geometries, create_test_polygon, create_test_rect, create_vlm_test_cases, data_dir,
    fixtures_dir, results_dir, DifficultyLevel, VLMTestCase,
};

// 重新导出数据加载工具
#[allow(unused_imports)]
pub use data::{
    load_accuracy_data, load_case_study_data, load_performance_data, load_vlm_data, save_data,
    AccuracyTestData, CaseStudyData, PerformanceTestData, VLMTestData,
};

// 重新导出常用类型
#[allow(unused_imports)]
pub use exp1_accuracy::AccuracyExperiment;
#[allow(unused_imports)]
pub use exp2_performance::PerformanceExperiment;
#[allow(unused_imports)]
pub use exp3_vlm_reasoning::VLMReasoningExperiment;
#[allow(unused_imports)]
pub use exp4_ablation::AblationExperiment;
#[allow(unused_imports)]
pub use exp5_case_studies::CaseStudyExperiment;
#[allow(unused_imports)]
pub use exp6_comparison::ComparisonExperiment;

#[allow(unused_imports)]
pub use latex_tables::{ExperimentTables, LatexTable, TableBuilder, TableStyle};
#[allow(unused_imports)]
pub use metrics::{AccuracyMetrics, PerformanceMetrics, QualityMetrics};
#[allow(unused_imports)]
pub use reproducibility::{EnvironmentInfo, ExperimentRun, ReproducibilityConfig, SeedManager};
pub use runner::ExperimentRunner;
#[allow(unused_imports)]
pub use statistics::{anova, effect_size, power_analysis, t_test};
#[allow(unused_imports)]
pub use utils::{ExperimentConfig, ExperimentReport, ExperimentResult};
#[allow(unused_imports)]
pub use validity_threats::{
    threat_templates, ThreatAnalyzer, ThreatCategory, ThreatSeverity, ValidityThreat,
};
#[allow(unused_imports)]
pub use venue_configs::{ExperimentConfigGenerator, VenueConfig, VenueType};
