//! 约束求解器性能基准测试
//!
//! 测试 Jacobian 计算并行化效果

use cadagent::geometry::constraint::{Constraint, ConstraintSolver, ConstraintSystem};
use cadagent::geometry::Point;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// 创建简单的距离约束系统
fn create_chain_system(num_points: usize) -> ConstraintSystem {
    let mut system = ConstraintSystem::new();

    // 添加点链
    let mut prev_point = None;
    for i in 0..num_points {
        let point_id = system.add_point(Point::new(i as f64 * 10.0, 0.0));

        // 添加固定长度约束（与前一个点）
        if let Some(prev_id) = prev_point {
            system.add_constraint(Constraint::FixLength {
                line_start: prev_id,
                line_end: point_id,
                length: 10.0,
            });
        }

        prev_point = Some(point_id);
    }

    // 固定第一个点
    if let Some(first_id) = system.entities.keys().next().copied() {
        system.add_constraint(Constraint::FixPoint { point_id: first_id });
    }

    system
}

fn bench_constraint_solving(c: &mut Criterion) {
    let mut group = c.benchmark_group("constraint_solving");

    for num_points in [3, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_points),
            num_points,
            |b, &n| {
                b.iter(|| {
                    let mut system = create_chain_system(n);
                    let solver = ConstraintSolver::new();
                    let _ = solver.solve(black_box(&mut system));
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_constraint_solving,);

criterion_main!(benches);
