//! NURBS 几何处理性能基准测试
//!
//! 测试 NURBS tessellation 和 thread-local buffer pool 的优化效果

use cadagent::geometry::nurbs::{NurbsCurve, NurbsSurface, Point3D};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// 创建测试 NURBS 曲线（二次）
fn create_quadratic_nurbs_curve(control_points: usize) -> NurbsCurve {
    let points: Vec<Point3D> = (0..control_points)
        .map(|i| {
            let x = i as f64 * 10.0;
            let y = (i as f64 * 0.5).sin() * 50.0;
            Point3D::new(x, y, 0.0)
        })
        .collect();

    let weights = vec![1.0; control_points];
    // 对于 open uniform knot vector: 节点数 = 控制点数 + 阶数
    // 对于二次曲线 (order=2), 需要 control_points + 2 个节点
    let knots: Vec<f64> = (0..=control_points + 1).map(|i| i as f64).collect();

    NurbsCurve::new(points, weights, knots, 2).unwrap()
}

/// 创建测试 NURBS 曲面（简单）
fn create_simple_nurbs_surface(control_points_u: usize, control_points_v: usize) -> NurbsSurface {
    let points: Vec<Vec<Point3D>> = (0..control_points_u)
        .map(|u| {
            (0..control_points_v)
                .map(|v| {
                    let x = u as f64 * 10.0;
                    let y = v as f64 * 10.0;
                    let z = ((u as f64 * 0.3).sin() + (v as f64 * 0.3).cos()) * 20.0;
                    Point3D::new(x, y, z)
                })
                .collect()
        })
        .collect();

    let weights: Vec<Vec<f64>> = (0..control_points_u)
        .map(|_| (0..control_points_v).map(|_| 1.0).collect())
        .collect();
    let knots_u: Vec<f64> = (0..=control_points_u + 1).map(|i| i as f64).collect();
    let knots_v: Vec<f64> = (0..=control_points_v + 1).map(|i| i as f64).collect();

    NurbsSurface::new(points, weights, knots_u, knots_v, 2, 2).unwrap()
}

fn bench_curve_tessellation(c: &mut Criterion) {
    let mut group = c.benchmark_group("curve_tessellation");

    for control_points in [5, 10, 20, 50].iter() {
        let curve = create_quadratic_nurbs_curve(*control_points);

        group.bench_with_input(
            BenchmarkId::from_parameter(control_points),
            &curve,
            |b, curve| b.iter(|| curve.tessellate(black_box(0.1))),
        );
    }
    group.finish();
}

fn bench_surface_tessellation(c: &mut Criterion) {
    let mut group = c.benchmark_group("surface_tessellation");

    for (u, v) in [(5, 5), (10, 10), (20, 20)].iter() {
        let surface = create_simple_nurbs_surface(*u, *v);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", u, v)),
            &surface,
            |b, surface| b.iter(|| surface.tessellate(black_box(0.5))),
        );
    }
    group.finish();
}

fn bench_thread_local_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_local_buffer_pool");

    // 测试 thread-local buffer pool 的效果
    // 对比多次调用的性能（缓存复用）
    let curve = create_quadratic_nurbs_curve(20);

    // 第一次调用（需要分配 buffer）
    group.bench_function("first_tessellation", |b| {
        let curve = create_quadratic_nurbs_curve(20);
        b.iter(|| curve.tessellate(black_box(0.1)))
    });

    // 后续调用（复用 thread-local buffer）
    group.bench_function("reused_tessellation", |b| {
        b.iter(|| curve.tessellate(black_box(0.1)))
    });

    group.finish();
}

fn bench_adaptive_tessellation(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_tessellation");

    let curve = create_quadratic_nurbs_curve(20);

    // 测试不同精度要求
    for tolerance in [1.0, 0.5, 0.1, 0.05, 0.01].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(tolerance),
            tolerance,
            |b, tol| b.iter(|| curve.tessellate(black_box(*tol))),
        );
    }
    group.finish();
}

fn bench_parallel_tessellation(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_tessellation");

    // 对比普通、并行和自适应离散化
    for num_points in [100, 500, 1000, 5000].iter() {
        let curve = create_quadratic_nurbs_curve(50);

        group.bench_with_input(
            BenchmarkId::new("sequential", num_points),
            num_points,
            |b, n| {
                b.iter(|| {
                    let _ = curve.tessellate(1.0 / (*n as f64));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel", num_points),
            num_points,
            |b, n| {
                b.iter(|| {
                    let _ = curve.tessellate_parallel(black_box(*n));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("adaptive", num_points),
            num_points,
            |b, n| {
                b.iter(|| {
                    let _ =
                        curve.tessellate_adaptive(black_box(0.01), black_box(10), black_box(*n));
                })
            },
        );
    }
    group.finish();
}

fn bench_surface_parallel_tessellation(c: &mut Criterion) {
    let mut group = c.benchmark_group("surface_parallel_tessellation");

    for (u, v) in [(10, 10), (20, 20), (30, 30)].iter() {
        let surface = create_simple_nurbs_surface(*u, *v);
        let (u_val, v_val) = (*u, *v);

        group.bench_with_input(
            BenchmarkId::new("sequential", format!("{}x{}", u, v)),
            &(u, v),
            |b, _input| {
                b.iter(|| {
                    let _ = surface.tessellate(black_box(1.0));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parallel", format!("{}x{}", u, v)),
            &(u, v),
            |b, _input| {
                b.iter(|| {
                    let _ = surface.tessellate_parallel(black_box(u_val), black_box(v_val));
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_curve_tessellation,
    bench_surface_tessellation,
    bench_thread_local_buffer_pool,
    bench_adaptive_tessellation,
    bench_parallel_tessellation,
    bench_surface_parallel_tessellation,
);

criterion_main!(benches);
