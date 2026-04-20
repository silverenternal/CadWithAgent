//! 顶会论文实验验证
//!
//! 运行所有实验并生成报告。
//!
//! # 使用方法
//!
//! ```bash
//! # 运行单个实验
//! cargo test --test experiment -- exp1_accuracy_validation --nocapture
//!
//! # 运行所有实验
//! cargo test --test experiment -- --nocapture
//!
//! # 运行特定实验套件
//! cargo test --test experiment -- experiment_suite --nocapture
//! ```

#![allow(dead_code)]

mod experiment;

use experiment::runner::ExperimentBuilder;
use experiment::{
    AblationExperiment, AccuracyExperiment, CaseStudyExperiment, ComparisonExperiment,
    ExperimentRunner, PerformanceExperiment, VLMReasoningExperiment,
};

/// 实验 1: 几何计算准确性验证
#[test]
fn exp1_accuracy_validation() {
    let config = experiment::exp1_accuracy::AccuracyExperimentConfig {
        num_samples: 1000,
        tolerance: 1e-10,
        verbose: true,
    };

    let experiment = AccuracyExperiment::new(config);
    let result = experiment.run();

    assert!(result.overall_passed, "准确性实验应该通过");

    // 验证各项子测试
    assert!(
        result.measurement_results.length_test.passed,
        "长度测量应该通过"
    );
    assert!(
        result.measurement_results.area_test.passed,
        "面积测量应该通过"
    );
    assert!(
        result.measurement_results.perimeter_test.passed,
        "周长测量应该通过"
    );
    assert!(
        result.measurement_results.angle_test.passed,
        "角度测量应该通过"
    );

    assert!(
        result.relation_results.parallel_test.passed,
        "平行检测应该通过"
    );
    assert!(
        result.relation_results.perpendicular_test.passed,
        "垂直检测应该通过"
    );
    assert!(
        result.relation_results.collinear_test.passed,
        "共线检测应该通过"
    );
    assert!(
        result.relation_results.tangent_test.passed,
        "相切检测应该通过"
    );
    assert!(
        result.relation_results.concentric_test.passed,
        "同心检测应该通过"
    );

    assert!(
        result.transform_results.translation_test.passed,
        "平移变换应该通过"
    );
    assert!(
        result.transform_results.rotation_test.passed,
        "旋转变换应该通过"
    );
    assert!(
        result.transform_results.scale_test.passed,
        "缩放变换应该通过"
    );
    assert!(
        result.transform_results.mirror_test.passed,
        "镜像变换应该通过"
    );

    println!("\n✓ 实验 1 通过验证");
}

/// 实验 2: 性能基准测试
#[test]
fn exp2_performance_benchmark() {
    let config = experiment::exp2_performance::PerformanceExperimentConfig {
        num_samples: 100,
        data_sizes: vec![100, 500, 1000],
        verbose: true,
        enable_baseline: true,
    };

    let experiment = PerformanceExperiment::new(config);
    let result = experiment.run_detailed();

    assert!(
        result.point_query_metrics.throughput > 0.0,
        "点查询吞吐量应该大于 0"
    );
    assert!(
        result.range_query_metrics.throughput > 0.0,
        "范围查询吞吐量应该大于 0"
    );
    assert!(
        result.nearest_query_metrics.throughput > 0.0,
        "最近邻查询吞吐量应该大于 0"
    );

    // 验证可扩展性数据
    assert!(!result.scalability_results.is_empty(), "应该有可扩展性数据");

    println!("\n✓ 实验 2 通过验证");
}

/// 实验 3: VLM 推理质量对比
#[test]
fn exp3_vlm_reasoning_quality() {
    let config = experiment::exp3_vlm_reasoning::VLMReasoningExperimentConfig {
        num_samples: 20,
        enable_baseline: true,
        verbose: true,
        api_endpoint: None,
        api_key: None,
    };

    let experiment = VLMReasoningExperiment::new(config);
    let result = experiment.run_detailed();

    assert!(
        result.enhanced_metrics.final_answer_accuracy > 0.5,
        "增强方法准确率应该大于 50%"
    );
    assert!(
        result.enhanced_metrics.hallucination_rate < 0.2,
        "幻觉率应该小于 20%"
    );
    assert!(
        result.geometry_understanding_accuracy > 0.5,
        "几何理解准确率应该大于 50%"
    );
    assert!(
        result.code_generation_success_rate > 0.5,
        "代码生成成功率应该大于 50%"
    );

    println!("\n✓ 实验 3 通过验证");
}

/// 实验 4: 消融实验
#[test]
fn exp4_ablation_study() {
    let config = experiment::exp4_ablation::AblationExperimentConfig {
        num_samples: 50,
        test_combinations: true,
        verbose: true,
    };

    let experiment = AblationExperiment::new(config);
    let result = experiment.run_detailed();

    assert!(
        !result.module_contribution_ranking.is_empty(),
        "应该有模块贡献度排序"
    );
    assert!(
        result
            .full_system_metrics
            .performance
            .get("accuracy")
            .copied()
            .unwrap_or(0.0)
            > 0.9,
        "完整系统准确率应该大于 90%"
    );

    println!("\n✓ 实验 4 通过验证");
}

/// 实验 5: 真实案例研究
#[test]
fn exp5_case_studies() {
    let config = experiment::exp5_case_studies::CaseStudyExperimentConfig {
        verbose: true,
        output_dir: "tests/experiment/results/case_studies".to_string(),
    };

    let experiment = CaseStudyExperiment::new(config);
    let result = experiment.run_detailed();

    assert!(
        result.overall_statistics.total_cases >= 3,
        "应该至少有 3 个案例"
    );
    assert!(
        result.overall_statistics.completion_rate > 0.5,
        "任务完成率应该大于 50%"
    );
    assert!(
        result.overall_statistics.avg_quality_score > 0.7,
        "平均质量评分应该大于 3.5/5.0"
    );

    println!("\n✓ 实验 5 通过验证");
}

/// 实验 6: 对比实验
#[test]
fn exp6_comparison_study() {
    let config = experiment::exp6_comparison::ComparisonExperimentConfig {
        num_samples: 50,
        include_commercial: true,
        include_opensource: true,
        include_ai_tools: true,
        verbose: true,
    };

    let experiment = ComparisonExperiment::new(config);
    let result = experiment.run_detailed();

    assert!(!result.overall_ranking.is_empty(), "应该有综合排名");
    assert!(result.method_metrics.len() >= 4, "应该至少有 4 个对比方法");

    // 验证 CadAgent 排名第一
    if let Some((top_method, _)) = result.overall_ranking.first() {
        assert!(
            matches!(
                top_method,
                experiment::exp6_comparison::ComparisonMethod::CadAgent
            ),
            "CadAgent 应该排名第一"
        );
    }

    println!("\n✓ 实验 6 通过验证");
}

/// 完整实验套件
#[test]
fn experiment_suite() {
    println!("\n{}", "=".repeat(60));
    println!("运行完整实验套件");
    println!("{}", "=".repeat(60));

    // 创建实验运行器
    let config = ExperimentBuilder::new("full_suite", "完整实验套件")
        .verbose(true)
        .output_dir("tests/experiment/results".into())
        .build();

    let mut runner = ExperimentRunner::new(config);

    // 运行所有实验
    run_exp1(&mut runner);
    run_exp2(&mut runner);
    run_exp3(&mut runner);
    run_exp4(&mut runner);
    run_exp5(&mut runner);
    run_exp6(&mut runner);

    // 生成汇总报告
    let summary = runner.generate_summary();
    runner.add_report(summary.clone());

    // 保存结果
    if let Err(e) = runner.save_results() {
        eprintln!("保存实验结果失败：{}", e);
    }

    if let Err(e) = runner.save_reports() {
        eprintln!("保存实验报告失败：{}", e);
    }

    // 验证所有实验通过
    let all_passed = runner.results().iter().all(|r| r.passed);
    assert!(all_passed, "所有实验应该通过");

    println!("\n{}", "=".repeat(60));
    println!("✓ 完整实验套件通过验证");
    println!("{}", "=".repeat(60));
}

fn run_exp1(runner: &mut ExperimentRunner) {
    println!("\n运行实验 1: 几何计算准确性验证");
    let config = experiment::exp1_accuracy::AccuracyExperimentConfig {
        num_samples: 100,
        tolerance: 1e-10,
        verbose: false,
    };
    let experiment = AccuracyExperiment::new(config);
    runner.run_experiment(&experiment);
}

fn run_exp2(runner: &mut ExperimentRunner) {
    println!("\n运行实验 2: 性能基准测试");
    let config = experiment::exp2_performance::PerformanceExperimentConfig {
        num_samples: 50,
        data_sizes: vec![100, 500],
        verbose: false,
        enable_baseline: true,
    };
    let experiment = PerformanceExperiment::new(config);
    runner.run_experiment(&experiment);
}

fn run_exp3(runner: &mut ExperimentRunner) {
    println!("\n运行实验 3: VLM 推理质量对比");
    let config = experiment::exp3_vlm_reasoning::VLMReasoningExperimentConfig {
        num_samples: 10,
        enable_baseline: true,
        verbose: false,
        api_endpoint: None,
        api_key: None,
    };
    let experiment = VLMReasoningExperiment::new(config);
    runner.run_experiment(&experiment);
}

fn run_exp4(runner: &mut ExperimentRunner) {
    println!("\n运行实验 4: 消融实验");
    let config = experiment::exp4_ablation::AblationExperimentConfig {
        num_samples: 20,
        test_combinations: false,
        verbose: false,
    };
    let experiment = AblationExperiment::new(config);
    runner.run_experiment(&experiment);
}

fn run_exp5(runner: &mut ExperimentRunner) {
    println!("\n运行实验 5: 真实案例研究");
    let config = experiment::exp5_case_studies::CaseStudyExperimentConfig {
        verbose: false,
        output_dir: "tests/experiment/results/case_studies".to_string(),
    };
    let experiment = CaseStudyExperiment::new(config);
    runner.run_experiment(&experiment);
}

fn run_exp6(runner: &mut ExperimentRunner) {
    println!("\n运行实验 6: 对比实验");
    let config = experiment::exp6_comparison::ComparisonExperimentConfig {
        num_samples: 20,
        include_commercial: true,
        include_opensource: true,
        include_ai_tools: true,
        verbose: false,
    };
    let experiment = ComparisonExperiment::new(config);
    runner.run_experiment(&experiment);
}

// 生成实验报告（不运行测试，仅生成报告）
// 暂时注释，待修复类型问题
// #[test]
// #[ignore]
// fn generate_experiment_reports() {
//     println!("\n生成实验报告...");

//     let config = ExperimentBuilder::new("report_generation", "实验报告生成")
//         .verbose(true)
//         .output_dir("tests/experiment/results".into())
//         .build();

//     let mut runner = ExperimentRunner::new(config);

//     // 生成各实验报告
//     let exp1_report = generate_exp1_report();
//     let exp2_report = generate_exp2_report();
//     let exp3_report = generate_exp3_report();
//     let exp4_report = generate_exp4_report();
//     let exp5_report = generate_exp5_report();
//     let exp6_report = generate_exp6_report();

//     runner.add_report(exp1_report);
//     runner.add_report(exp2_report);
//     runner.add_report(exp3_report);
//     runner.add_report(exp4_report);
//     runner.add_report(exp5_report);
//     runner.add_report(exp6_report);

//     // 保存报告
//     if let Err(e) = runner.save_reports() {
//         eprintln!("保存报告失败：{}", e);
//     }

//     println!("\n✓ 实验报告生成完成");
// }

// fn generate_exp1_report() -> ExperimentReport {
//     let config = experiment::exp1_accuracy::AccuracyExperimentConfig::default();
//     let experiment = AccuracyExperiment::new(config);
//     experiment.run();
//     experiment.generate_report()
// }

// fn generate_exp2_report() -> ExperimentReport {
//     let config = experiment::exp2_performance::PerformanceExperimentConfig::default();
//     let experiment = PerformanceExperiment::new(config);
//     experiment.generate_report()
// }

// fn generate_exp3_report() -> ExperimentReport {
//     let config = experiment::exp3_vlm_reasoning::VLMReasoningExperimentConfig::default();
//     let experiment = VLMReasoningExperiment::new(config);
//     experiment.generate_report()
// }

// fn generate_exp4_report() -> ExperimentReport {
//     let config = experiment::exp4_ablation::AblationExperimentConfig::default();
//     let experiment = AblationExperiment::new(config);
//     experiment.generate_report()
// }

// fn generate_exp5_report() -> ExperimentReport {
//     let config = experiment::exp5_case_studies::CaseStudyExperimentConfig::default();
//     let experiment = CaseStudyExperiment::new(config);
//     experiment.generate_report()
// }

// fn generate_exp6_report() -> ExperimentReport {
//     let config = experiment::exp6_comparison::ComparisonExperimentConfig::default();
//     let experiment = ComparisonExperiment::new(config);
//     experiment.generate_report()
// }
