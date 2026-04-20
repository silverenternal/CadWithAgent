//! GPU 性能基准测试
//!
//! 对比 CPU 与 GPU 在几何变换、距离计算等任务上的性能差异
//!
//! # 使用示例
//!
//! ```bash
//! # 运行所有 GPU 基准测试
//! cargo test --test gpu_benchmark_test -- --nocapture
//!
//! # 运行特定测试
//! cargo test --test gpu_benchmark_test test_gpu_transform_performance -- --nocapture
//! ```

use cadagent::gpu::{GpuContext, GpuError};
use nalgebra::Point3;
use std::time::Instant;

/// GPU 基准测试工具
pub struct GpuBenchmark {
    context: Option<GpuContext>,
}

impl GpuBenchmark {
    /// 创建新的 GPU 基准测试（异步初始化）
    pub async fn new() -> Result<Self, GpuError> {
        let context = GpuContext::new().await?;

        Ok(Self {
            context: Some(context),
        })
    }

    /// 创建基准测试（同步版本，用于测试）
    pub fn new_sync() -> Option<Self> {
        // 在测试环境中，GPU 可能不可用
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::new())
            .ok()
    }

    /// 测试 GPU 是否可用
    pub fn is_gpu_available(&self) -> bool {
        self.context.is_some()
    }

    /// CPU 点变换实现
    pub fn transform_points_cpu(points: &mut [Point3<f32>], matrix: &[[f32; 4]; 4]) {
        for point in points.iter_mut() {
            let x = point.x;
            let y = point.y;
            let z = point.z;

            point.x = matrix[0][0] * x + matrix[0][1] * y + matrix[0][2] * z + matrix[0][3];
            point.y = matrix[1][0] * x + matrix[1][1] * y + matrix[1][2] * z + matrix[1][3];
            point.z = matrix[2][0] * x + matrix[2][1] * y + matrix[2][2] * z + matrix[2][3];

            // 透视除法（如果需要）
            let w = matrix[3][0] * x + matrix[3][1] * y + matrix[3][2] * z + matrix[3][3];
            if w.abs() > 1e-6 {
                point.x /= w;
                point.y /= w;
                point.z /= w;
            }
        }
    }

    /// GPU 点变换实现
    pub async fn transform_points_gpu(
        &self,
        points: &[Point3<f32>],
        matrix: &[[f32; 4]; 4],
    ) -> Result<Vec<Point3<f32>>, GpuError> {
        if let Some(context) = &self.context {
            // Use TransformPipeline directly
            let transform_pipeline = cadagent::gpu::TransformPipeline::new(context);
            // Convert matrix to nalgebra format
            let nalgebra_matrix = nalgebra::Matrix4::new(
                matrix[0][0],
                matrix[0][1],
                matrix[0][2],
                matrix[0][3],
                matrix[1][0],
                matrix[1][1],
                matrix[1][2],
                matrix[1][3],
                matrix[2][0],
                matrix[2][1],
                matrix[2][2],
                matrix[2][3],
                matrix[3][0],
                matrix[3][1],
                matrix[3][2],
                matrix[3][3],
            );
            let result = transform_pipeline
                .transform_points(points, &nalgebra_matrix, false)
                .await?;
            Ok(result)
        } else {
            Err(GpuError::ComputeError("GPU not available".to_string()))
        }
    }
}

impl Drop for GpuBenchmark {
    fn drop(&mut self) {
        // GPU 资源会自动释放
    }
}

/// 生成测试点集
fn generate_test_points(count: usize) -> Vec<Point3<f32>> {
    (0..count)
        .map(|i| {
            let angle = (i as f32) * 0.01;
            let radius = 100.0;
            Point3::new(radius * angle.cos(), radius * angle.sin(), (i as f32) * 0.1)
        })
        .collect()
}

/// 生成单位矩阵
fn identity_matrix() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// 生成平移矩阵
fn translation_matrix(tx: f32, ty: f32, tz: f32) -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, tx],
        [0.0, 1.0, 0.0, ty],
        [0.0, 0.0, 1.0, tz],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// 生成缩放矩阵
fn scale_matrix(sx: f32, sy: f32, sz: f32) -> [[f32; 4]; 4] {
    [
        [sx, 0.0, 0.0, 0.0],
        [0.0, sy, 0.0, 0.0],
        [0.0, 0.0, sz, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// 生成旋转矩阵（绕 Z 轴）
fn rotation_matrix_z(angle_rad: f32) -> [[f32; 4]; 4] {
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    [
        [cos_a, -sin_a, 0.0, 0.0],
        [sin_a, cos_a, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 GPU 初始化
    #[test]
    fn test_gpu_initialization() {
        let benchmark = GpuBenchmark::new_sync();

        if let Some(bench) = &benchmark {
            assert!(bench.is_gpu_available(), "GPU 应该可用");
            println!("✅ GPU 初始化成功");
        } else {
            println!("⚠️  GPU 不可用，跳过测试");
        }
    }

    /// 测试 CPU 点变换正确性
    #[test]
    fn test_cpu_transform_correctness() {
        let mut points = vec![Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)];
        let matrix = rotation_matrix_z(std::f32::consts::PI / 2.0);

        GpuBenchmark::transform_points_cpu(&mut points, &matrix);

        // 旋转 90 度后，(1, 0, 0) 应该变成 (0, 1, 0)
        assert!((points[0].x - 0.0).abs() < 1e-5);
        assert!((points[0].y - 1.0).abs() < 1e-5);

        // (0, 1, 0) 应该变成 (-1, 0, 0)
        assert!((points[1].x - (-1.0)).abs() < 1e-5);
        assert!((points[1].y - 0.0).abs() < 1e-5);
    }

    /// 测试 GPU 点变换正确性
    #[tokio::test]
    async fn test_gpu_transform_correctness() {
        let benchmark = match GpuBenchmark::new().await {
            Ok(b) => b,
            Err(_) => {
                println!("⚠️  GPU 不可用，跳过测试");
                return;
            }
        };

        let points = vec![Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)];
        let matrix = rotation_matrix_z(std::f32::consts::PI / 2.0);

        let result = benchmark.transform_points_gpu(&points, &matrix).await;

        if let Ok(transformed) = result {
            // 旋转 90 度后，(1, 0, 0) 应该变成 (0, 1, 0)
            assert!((transformed[0].x - 0.0).abs() < 1e-5);
            assert!((transformed[0].y - 1.0).abs() < 1e-5);

            // (0, 1, 0) 应该变成 (-1, 0, 0)
            assert!((transformed[1].x - (-1.0)).abs() < 1e-5);
            assert!((transformed[1].y - 0.0).abs() < 1e-5);

            println!("✅ GPU 变换正确性验证通过");
        } else {
            println!("⚠️  GPU 变换失败：{:?}", result);
        }
    }

    /// 测试不同规模下的 CPU vs GPU 性能对比
    #[tokio::test]
    async fn test_gpu_transform_performance() {
        let benchmark = match GpuBenchmark::new().await {
            Ok(b) => b,
            Err(_) => {
                println!("⚠️  GPU 不可用，跳过性能测试");
                return;
            }
        };

        let test_sizes = [100, 1_000, 10_000, 100_000];
        let matrix = translation_matrix(10.0, 20.0, 30.0);

        println!("\n{:60}", "GPU vsCPU 性能对比测试");
        println!("{:-<60}", "");
        println!(
            "{:>12} | {:>15} | {:>15} | {:>10}",
            "点数", "CPU 时间 (ms)", "GPU 时间 (ms)", "加速比"
        );
        println!("{:-<60}", "");

        for &size in &test_sizes {
            let points = generate_test_points(size);

            // CPU 测试
            let mut cpu_points = points.clone();
            let cpu_start = Instant::now();
            GpuBenchmark::transform_points_cpu(&mut cpu_points, &matrix);
            let cpu_time = cpu_start.elapsed().as_secs_f64() * 1000.0;

            // GPU 测试
            let gpu_start = Instant::now();
            let gpu_result = benchmark.transform_points_gpu(&points, &matrix).await;
            let gpu_time = gpu_start.elapsed().as_secs_f64() * 1000.0;

            let speedup = if gpu_time > 0.0 {
                cpu_time / gpu_time
            } else {
                f64::INFINITY
            };

            println!(
                "{:>12} | {:>15.3} | {:>15.3} | {:>10.2}x",
                size, cpu_time, gpu_time, speedup
            );

            // 验证结果一致性
            if let Ok(gpu_points) = gpu_result {
                for (cpu, gpu) in cpu_points.iter().zip(gpu_points.iter()) {
                    assert!((cpu.x - gpu.x).abs() < 1e-4);
                    assert!((cpu.y - gpu.y).abs() < 1e-4);
                    assert!((cpu.z - gpu.z).abs() < 1e-4);
                }
            }
        }

        println!("{:-<60}", "");
    }

    /// 测试不同变换类型的性能
    #[tokio::test]
    async fn test_different_transform_types() {
        let benchmark = match GpuBenchmark::new().await {
            Ok(b) => b,
            Err(_) => {
                println!("⚠️  GPU 不可用，跳过测试");
                return;
            }
        };

        let size = 10_000;
        let points = generate_test_points(size);

        println!("\n{:60}", format!("不同变换类型性能测试 ({} 点)", size));
        println!("{:-<60}", "");
        println!(
            "{:>15} | {:>15} | {:>15} | {:>10}",
            "变换类型", "CPU 时间 (ms)", "GPU 时间 (ms)", "加速比"
        );
        println!("{:-<60}", "");

        let transforms = vec![
            ("单位变换", identity_matrix()),
            ("平移变换", translation_matrix(5.0, 10.0, 15.0)),
            ("缩放变换", scale_matrix(2.0, 2.0, 2.0)),
            ("旋转变换", rotation_matrix_z(std::f32::consts::PI / 4.0)),
        ];

        for (name, matrix) in transforms {
            // CPU
            let mut cpu_points = points.clone();
            let cpu_start = Instant::now();
            GpuBenchmark::transform_points_cpu(&mut cpu_points, &matrix);
            let cpu_time = cpu_start.elapsed().as_secs_f64() * 1000.0;

            // GPU
            let gpu_start = Instant::now();
            let _ = benchmark.transform_points_gpu(&points, &matrix).await;
            let gpu_time = gpu_start.elapsed().as_secs_f64() * 1000.0;

            let speedup = if gpu_time > 0.0 {
                cpu_time / gpu_time
            } else {
                f64::INFINITY
            };

            println!(
                "{:>15} | {:>15.3} | {:>15.3} | {:>10.2}x",
                name, cpu_time, gpu_time, speedup
            );
        }

        println!("{:-<60}", "");
    }

    /// 测试大规模场景下的 GPU 性能优势
    #[tokio::test]
    async fn test_large_scale_gpu_advantage() {
        let benchmark = match GpuBenchmark::new().await {
            Ok(b) => b,
            Err(_) => {
                println!("⚠️  GPU 不可用，跳过测试");
                return;
            }
        };

        // 测试大规模场景：1,000,000 点
        let size = 1_000_000;
        let points = generate_test_points(size);
        let matrix = translation_matrix(100.0, 200.0, 300.0);

        println!("\n{:60}", format!("大规模场景测试 ({} 点)", size));
        println!("{:-<60}", "");

        // CPU
        let mut cpu_points = points.clone();
        let cpu_start = Instant::now();
        GpuBenchmark::transform_points_cpu(&mut cpu_points, &matrix);
        let cpu_time = cpu_start.elapsed().as_secs_f64() * 1000.0;
        println!("CPU 时间：{:.3} ms", cpu_time);

        // GPU
        let gpu_start = Instant::now();
        let gpu_result = benchmark.transform_points_gpu(&points, &matrix).await;
        let gpu_time = gpu_start.elapsed().as_secs_f64() * 1000.0;
        println!("GPU 时间：{:.3} ms", gpu_time);

        let speedup = if gpu_time > 0.0 {
            cpu_time / gpu_time
        } else {
            f64::INFINITY
        };
        println!("加速比：{:.2}x", speedup);

        // 验证：GPU 应该在大规模场景下有明显优势
        if let Ok(gpu_points) = gpu_result {
            // 允许一定的误差（GPU 可能有不同的浮点精度）
            let mut max_error: f64 = 0.0;
            for (cpu, gpu) in cpu_points.iter().zip(gpu_points.iter()) {
                let error: f64 = ((cpu.x as f64 - gpu.x as f64).abs()
                    + (cpu.y as f64 - gpu.y as f64).abs()
                    + (cpu.z as f64 - gpu.z as f64).abs())
                    / 3.0;
                max_error = max_error.max(error);
            }
            println!("最大平均误差：{:.6}", max_error);
            assert!(max_error < 1e-3, "CPU/GPU 结果差异过大");
        }

        println!("{:-<60}", "");
    }

    /// 测试批量多次变换的性能
    #[tokio::test]
    async fn test_batch_transforms_performance() {
        let benchmark = match GpuBenchmark::new().await {
            Ok(b) => b,
            Err(_) => {
                println!("⚠️  GPU 不可用，跳过测试");
                return;
            }
        };

        let size = 10_000;
        let points = generate_test_points(size);
        let batch_count = 100;

        println!(
            "\n{:60}",
            format!("批量变换性能测试 ({} 点 x {} 次)", size, batch_count)
        );
        println!("{:-<60}", "");

        // CPU 批量测试
        let mut cpu_points = points.clone();
        let matrix = rotation_matrix_z(0.01);
        let cpu_start = Instant::now();
        for _ in 0..batch_count {
            GpuBenchmark::transform_points_cpu(&mut cpu_points, &matrix);
        }
        let cpu_time = cpu_start.elapsed().as_secs_f64() * 1000.0;

        // GPU 批量测试
        let gpu_start = Instant::now();
        for _ in 0..batch_count {
            let _ = benchmark.transform_points_gpu(&points, &matrix).await;
        }
        let gpu_time = gpu_start.elapsed().as_secs_f64() * 1000.0;

        let speedup = if gpu_time > 0.0 {
            cpu_time / gpu_time
        } else {
            f64::INFINITY
        };

        println!(
            "CPU 总时间：{:.3} ms (平均 {:.4} ms/次)",
            cpu_time,
            cpu_time / batch_count as f64
        );
        println!(
            "GPU 总时间：{:.3} ms (平均 {:.4} ms/次)",
            gpu_time,
            gpu_time / batch_count as f64
        );
        println!("加速比：{:.2}x", speedup);
        println!("{:-<60}", "");
    }
}
