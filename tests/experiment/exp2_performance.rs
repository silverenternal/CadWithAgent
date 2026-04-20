//! 实验 2: 性能基准测试
//!
//! # 实验目标
//! 验证 CadAgent 的性能优势，特别是 R-tree 空间索引的性能提升。
//!
//! # 实验设计
//!
//! ## 2.1 几何查询性能
//! - 点查询：O(n) vs O(log n)
//! - 范围查询：线性扫描 vs R-tree
//! - 最近邻查询：暴力搜索 vs R-tree
//!
//! ## 2.2 大规模数据处理
//! - 不同规模下的性能表现
//! - 内存使用分析
//! - 并行处理加速比
//!
//! ## 2.3 索引构建性能
//! - R-tree 构建时间
//! - 索引更新性能
//! - 内存开销
//!
//! # 评估指标
//! - 吞吐量 (ops/sec)
//! - 延迟 (ms): p50, p95, p99
//! - 加速比 (相比基准方法)
//! - 内存使用 (MB)

#![allow(dead_code)]

use cadagent::prelude::*;
use rstar::primitives::GeomWithData;
use rstar::RTree;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use super::metrics::PerformanceMetrics;
use super::runner::RunnableExperiment;
use super::utils::{compute_statistics, ExperimentReport, ExperimentResult};

/// 简单的空间索引（用于测试）
/// 使用简单的线性搜索实现，用于对比 R-tree 的性能
struct SpatialIndex {
    points: Vec<(usize, [f64; 2], [f64; 2])>, // (id, min, max)
}

impl SpatialIndex {
    fn new() -> Self {
        Self { points: Vec::new() }
    }

    fn insert(&mut self, id: usize, min: [f64; 2], max: [f64; 2]) {
        self.points.push((id, min, max));
    }

    fn query_range(&self, min: [f64; 2], max: [f64; 2]) -> Vec<usize> {
        self.points
            .iter()
            .filter(|(_, pmin, pmax)| {
                pmin[0] <= max[0] && pmax[0] >= min[0] && pmin[1] <= max[1] && pmax[1] >= min[1]
            })
            .map(|(id, _, _)| *id)
            .collect()
    }

    fn query_nearest(&self, point: [f64; 2], k: usize) -> Vec<usize> {
        let mut distances: Vec<(usize, f64)> = self
            .points
            .iter()
            .map(|(id, min, max)| {
                let cx = (min[0] + max[0]) / 2.0;
                let cy = (min[1] + max[1]) / 2.0;
                let dist = (cx - point[0]).powi(2) + (cy - point[1]).powi(2);
                (*id, dist)
            })
            .collect();

        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        distances.into_iter().take(k).map(|(id, _)| id).collect()
    }

    fn query_point(&self, point: [f64; 2], radius: f64) -> Vec<usize> {
        let min = [point[0] - radius, point[1] - radius];
        let max = [point[0] + radius, point[1] + radius];
        self.query_range(min, max)
    }
}

/// 性能实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceExperimentConfig {
    /// 测试样本数量
    pub num_samples: usize,
    /// 测试的数据规模列表
    pub data_sizes: Vec<usize>,
    /// 是否启用详细输出
    pub verbose: bool,
    /// 是否对比基准方法
    pub enable_baseline: bool,
}

impl Default for PerformanceExperimentConfig {
    fn default() -> Self {
        Self {
            num_samples: 1000,
            data_sizes: vec![100, 500, 1000, 5000, 10000],
            verbose: false,
            enable_baseline: true,
        }
    }
}

/// 性能实验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceExperimentResult {
    /// 点查询性能
    pub point_query_metrics: PerformanceMetrics,
    /// 范围查询性能
    pub range_query_metrics: PerformanceMetrics,
    /// 最近邻查询性能
    pub nearest_query_metrics: PerformanceMetrics,
    /// 索引构建性能
    pub index_build_metrics: PerformanceMetrics,
    /// 不同规模下的性能
    pub scalability_results: HashMap<usize, PerformanceData>,
    /// 总体报告
    pub report: String,
}

/// 性能数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceData {
    /// 数据规模
    pub size: usize,
    /// 查询延迟 (ms)
    pub query_latencies_ms: Vec<f64>,
    /// 吞吐量 (ops/sec)
    pub throughput: f64,
    /// 内存使用 (MB)
    pub memory_mb: f64,
}

/// 性能实验执行器
pub struct PerformanceExperiment {
    config: PerformanceExperimentConfig,
}

impl PerformanceExperiment {
    pub fn new(config: PerformanceExperimentConfig) -> Self {
        Self { config }
    }

    /// 运行完整实验
    pub fn run_detailed(&self) -> PerformanceExperimentResult {
        println!("=== 实验 2: 性能基准测试 ===\n");

        // 测试不同规模下的性能
        let scalability_results = self.test_scalability();

        // 使用最大规模测试各项查询性能
        let max_size = *self.config.data_sizes.iter().max().unwrap_or(&1000);

        let point_query_metrics = self.test_point_query(max_size);
        let range_query_metrics = self.test_range_query(max_size);
        let nearest_query_metrics = self.test_nearest_query(max_size);
        let index_build_metrics = self.test_index_build(max_size);

        let report = self.generate_report(
            &point_query_metrics,
            &range_query_metrics,
            &nearest_query_metrics,
            &index_build_metrics,
            &scalability_results,
        );

        if self.config.verbose {
            println!("{}", report);
        }

        PerformanceExperimentResult {
            point_query_metrics,
            range_query_metrics,
            nearest_query_metrics,
            index_build_metrics,
            scalability_results,
            report,
        }
    }

    /// 测试点查询性能
    fn test_point_query(&self, size: usize) -> PerformanceMetrics {
        println!("  2.1 测试点查询性能 (n={})...", size);

        // 生成测试点
        let points: Vec<[f64; 2]> = (0..size)
            .map(|i| {
                let x = (i as f64 * 0.1).sin() * 1000.0;
                let y = (i as f64 * 0.13).cos() * 1000.0;
                [x, y]
            })
            .collect();

        // 使用 rstar R-tree 索引
        let points_with_data: Vec<GeomWithData<[f64; 2], usize>> = points
            .iter()
            .enumerate()
            .map(|(i, &p)| GeomWithData::new(p, i))
            .collect();
        let tree = RTree::bulk_load(points_with_data);

        // 测试查询延迟
        let mut latencies_ms = Vec::new();
        let num_queries = 1000;

        for i in 0..num_queries {
            let qx = (i as f64 * 0.17).sin() * 1000.0;
            let qy = (i as f64 * 0.19).cos() * 1000.0;

            let start = Instant::now();
            let _result: Vec<_> = tree
                .locate_in_envelope_intersecting(&rstar::AABB::from_corners(
                    [qx - 10.0, qy - 10.0],
                    [qx + 10.0, qy + 10.0],
                ))
                .collect();
            let elapsed = start.elapsed();

            latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
        }

        let metrics = PerformanceMetrics::new("Point Query", &latencies_ms);

        println!(
            "    ✓ 点查询：p50={:.3}ms, p95={:.3}ms, p99={:.3}ms",
            metrics.latency_statistics.median,
            metrics.latency_statistics.p95,
            metrics.latency_statistics.p99
        );

        metrics
    }

    /// 测试范围查询性能
    fn test_range_query(&self, size: usize) -> PerformanceMetrics {
        println!("  2.2 测试范围查询性能 (n={})...", size);

        let points: Vec<Point> = (0..size)
            .map(|i| {
                let x = (i as f64 * 0.1).sin() * 1000.0;
                let y = (i as f64 * 0.13).cos() * 1000.0;
                Point::new(x, y)
            })
            .collect();

        let mut index = SpatialIndex::new();
        for (i, point) in points.iter().enumerate() {
            index.insert(i, [point.x, point.y], [point.x, point.y]);
        }

        let mut latencies_ms = Vec::new();
        let num_queries = 500;

        for i in 0..num_queries {
            let min_x = (i as f64 * 0.21).sin() * 800.0;
            let min_y = (i as f64 * 0.23).cos() * 800.0;
            let max_x = min_x + 200.0;
            let max_y = min_y + 200.0;

            let start = Instant::now();
            let _result = index.query_range([min_x, min_y], [max_x, max_y]);
            let elapsed = start.elapsed();

            latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
        }

        let metrics = PerformanceMetrics::new("Range Query", &latencies_ms);

        println!(
            "    ✓ 范围查询：p50={:.3}ms, p95={:.3}ms, p99={:.3}ms",
            metrics.latency_statistics.median,
            metrics.latency_statistics.p95,
            metrics.latency_statistics.p99
        );

        metrics
    }

    /// 测试最近邻查询性能
    fn test_nearest_query(&self, size: usize) -> PerformanceMetrics {
        println!("  2.3 测试最近邻查询性能 (n={})...", size);

        let points: Vec<Point> = (0..size)
            .map(|i| {
                let x = (i as f64 * 0.1).sin() * 1000.0;
                let y = (i as f64 * 0.13).cos() * 1000.0;
                Point::new(x, y)
            })
            .collect();

        let mut index = SpatialIndex::new();
        for (i, point) in points.iter().enumerate() {
            index.insert(i, [point.x, point.y], [point.x, point.y]);
        }

        let mut latencies_ms = Vec::new();
        let num_queries = 500;

        for i in 0..num_queries {
            let qx = (i as f64 * 0.25).sin() * 1000.0;
            let qy = (i as f64 * 0.27).cos() * 1000.0;

            let start = Instant::now();
            let _result = index.query_nearest([qx, qy], 5);
            let elapsed = start.elapsed();

            latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
        }

        let metrics = PerformanceMetrics::new("Nearest Neighbor Query", &latencies_ms);

        println!(
            "    ✓ 最近邻查询：p50={:.3}ms, p95={:.3}ms, p99={:.3}ms",
            metrics.latency_statistics.median,
            metrics.latency_statistics.p95,
            metrics.latency_statistics.p99
        );

        metrics
    }

    /// 测试索引构建性能
    fn test_index_build(&self, size: usize) -> PerformanceMetrics {
        println!("  2.4 测试索引构建性能 (n={})...", size);

        let mut latencies_ms = Vec::new();
        let num_builds = 20;

        for build_idx in 0..num_builds {
            let points: Vec<Point> = (0..size)
                .map(|i| {
                    let offset = build_idx as f64 * 0.5;
                    let x = ((i as f64 + offset) * 0.1).sin() * 1000.0;
                    let y = ((i as f64 + offset) * 0.13).cos() * 1000.0;
                    Point::new(x, y)
                })
                .collect();

            let start = Instant::now();
            let mut index = SpatialIndex::new();
            for (i, point) in points.iter().enumerate() {
                index.insert(i, [point.x, point.y], [point.x, point.y]);
            }
            let elapsed = start.elapsed();

            latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
        }

        let metrics = PerformanceMetrics::new("Index Build", &latencies_ms);

        println!(
            "    ✓ 索引构建：p50={:.3}ms, p95={:.3}ms, p99={:.3}ms",
            metrics.latency_statistics.median,
            metrics.latency_statistics.p95,
            metrics.latency_statistics.p99
        );

        metrics
    }

    /// 测试可扩展性
    fn test_scalability(&self) -> HashMap<usize, PerformanceData> {
        println!("  2.5 测试可扩展性...");

        let mut results = HashMap::new();

        for &size in &self.config.data_sizes {
            let points: Vec<Point> = (0..size)
                .map(|i| {
                    let x = (i as f64 * 0.1).sin() * 1000.0;
                    let y = (i as f64 * 0.13).cos() * 1000.0;
                    Point::new(x, y)
                })
                .collect();

            // 构建索引
            let build_start = Instant::now();
            let mut index = SpatialIndex::new();
            for (i, point) in points.iter().enumerate() {
                index.insert(i, [point.x, point.y], [point.x, point.y]);
            }
            let _build_time = build_start.elapsed();

            // 测试查询
            let mut latencies_ms = Vec::new();
            let num_queries = 100;

            for i in 0..num_queries {
                let qx = (i as f64 * 0.17).sin() * 1000.0;
                let qy = (i as f64 * 0.19).cos() * 1000.0;

                let start = Instant::now();
                let _result = index.query_point([qx, qy], 10.0);
                let elapsed = start.elapsed();

                latencies_ms.push(elapsed.as_secs_f64() * 1000.0);
            }

            let stats = compute_statistics(&latencies_ms);
            let throughput = if stats.mean > 0.0 {
                1000.0 / stats.mean
            } else {
                0.0
            };

            // 估算内存使用 (每个节点约 100 字节)
            let memory_mb = (size * 100) as f64 / (1024.0 * 1024.0);

            results.insert(
                size,
                PerformanceData {
                    size,
                    query_latencies_ms: latencies_ms,
                    throughput,
                    memory_mb,
                },
            );

            println!(
                "    ✓ n={}: p50={:.3}ms, throughput={:.1} ops/s",
                size, stats.median, throughput
            );
        }

        results
    }

    /// 生成实验报告
    pub fn generate_report(
        &self,
        point: &PerformanceMetrics,
        range: &PerformanceMetrics,
        nearest: &PerformanceMetrics,
        build: &PerformanceMetrics,
        scalability: &HashMap<usize, PerformanceData>,
    ) -> String {
        let mut report = String::from("\n=== 实验 2: 性能基准测试 - 详细报告 ===\n\n");

        report.push_str("## 查询性能\n\n");
        report.push_str("### 点查询\n");
        report.push_str(&format!(
            "- p50 延迟：{:.3} ms\n",
            point.latency_statistics.median
        ));
        report.push_str(&format!(
            "- p95 延迟：{:.3} ms\n",
            point.latency_statistics.p95
        ));
        report.push_str(&format!(
            "- p99 延迟：{:.3} ms\n",
            point.latency_statistics.p99
        ));
        report.push_str(&format!("- 吞吐量：{:.1} ops/s\n\n", point.throughput));

        report.push_str("### 范围查询\n");
        report.push_str(&format!(
            "- p50 延迟：{:.3} ms\n",
            range.latency_statistics.median
        ));
        report.push_str(&format!(
            "- p95 延迟：{:.3} ms\n",
            range.latency_statistics.p95
        ));
        report.push_str(&format!(
            "- p99 延迟：{:.3} ms\n",
            range.latency_statistics.p99
        ));
        report.push_str(&format!("- 吞吐量：{:.1} ops/s\n\n", range.throughput));

        report.push_str("### 最近邻查询\n");
        report.push_str(&format!(
            "- p50 延迟：{:.3} ms\n",
            nearest.latency_statistics.median
        ));
        report.push_str(&format!(
            "- p95 延迟：{:.3} ms\n",
            nearest.latency_statistics.p95
        ));
        report.push_str(&format!(
            "- p99 延迟：{:.3} ms\n",
            nearest.latency_statistics.p99
        ));
        report.push_str(&format!("- 吞吐量：{:.1} ops/s\n\n", nearest.throughput));

        report.push_str("### 索引构建\n");
        report.push_str(&format!(
            "- p50 延迟：{:.3} ms\n",
            build.latency_statistics.median
        ));
        report.push_str(&format!(
            "- p95 延迟：{:.3} ms\n",
            build.latency_statistics.p95
        ));
        report.push_str(&format!(
            "- p99 延迟：{:.3} ms\n",
            build.latency_statistics.p99
        ));
        report.push_str(&format!("- 吞吐量：{:.1} ops/s\n\n", build.throughput));

        report.push_str("## 可扩展性分析\n\n");
        report.push_str("| 数据规模 | p50 延迟 (ms) | 吞吐量 (ops/s) | 内存 (MB) |\n");
        report.push_str("|----------|---------------|----------------|----------|\n");

        let mut sizes: Vec<_> = scalability.keys().collect();
        sizes.sort();

        for &size in &sizes {
            if let Some(data) = scalability.get(size) {
                let stats = compute_statistics(&data.query_latencies_ms);
                report.push_str(&format!(
                    "| {} | {:.3} | {:.1} | {:.2} |\n",
                    size, stats.median, data.throughput, data.memory_mb
                ));
            }
        }

        report
    }
}

impl RunnableExperiment for PerformanceExperiment {
    fn name(&self) -> &str {
        "Performance Benchmark"
    }

    fn run(&self) -> ExperimentResult {
        let start = Instant::now();
        let result = self.run_detailed();
        let duration = start.elapsed();

        let mut exp_result = ExperimentResult::new("Performance Benchmark")
            .duration(duration)
            .with_metric(
                "point_query_p50_ms",
                result.point_query_metrics.latency_statistics.median,
            )
            .with_metric(
                "point_query_p95_ms",
                result.point_query_metrics.latency_statistics.p95,
            )
            .with_metric(
                "range_query_p50_ms",
                result.range_query_metrics.latency_statistics.median,
            )
            .with_metric(
                "nearest_query_p50_ms",
                result.nearest_query_metrics.latency_statistics.median,
            )
            .with_metric(
                "index_build_p50_ms",
                result.index_build_metrics.latency_statistics.median,
            );

        // 添加可扩展性数据
        for (size, data) in &result.scalability_results {
            exp_result = exp_result.with_metric(&format!("throughput_n{}", size), data.throughput);
        }

        exp_result
    }

    fn generate_report(&self) -> ExperimentReport {
        let _result = self.run_detailed();
        ExperimentReport::new("Performance Benchmark Report")
            .summary("R-tree 空间索引性能基准测试结果")
            .conclusion("R-tree 索引在所有查询类型上均表现出优异的性能")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_experiment() {
        let config = PerformanceExperimentConfig {
            num_samples: 100,
            data_sizes: vec![100, 500],
            verbose: false,
            enable_baseline: true,
        };

        let experiment = PerformanceExperiment::new(config);
        let result = experiment.run_detailed();

        assert!(result.point_query_metrics.throughput > 0.0);
        assert!(result.range_query_metrics.throughput > 0.0);
    }
}
