//! 增量更新和依赖图性能基准测试
//!
//! 测试 dependency_graph 克隆优化和增量更新的性能

use cadagent::incremental::dependency_graph::DependencyGraph;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// 创建线性依赖链
fn create_linear_chain(size: usize) -> DependencyGraph {
    let mut graph = DependencyGraph::new();
    for i in 0..size {
        let node = format!("node_{}", i);
        graph.add_node(node.clone());
        if i > 0 {
            let _ = graph.add_dependency(format!("node_{}", i), format!("node_{}", i - 1));
        }
    }
    graph
}

/// 创建树状依赖结构
fn create_tree_dependency(size: usize) -> DependencyGraph {
    let mut graph = DependencyGraph::new();

    // 创建根节点
    graph.add_node("root".to_string());

    // 逐层创建子节点
    let mut current_level = vec!["root".to_string()];
    let mut node_count = 1;

    while node_count < size {
        let mut next_level = Vec::new();
        for parent in &current_level {
            if node_count >= size {
                break;
            }
            for _i in 0..3 {
                // 每个节点 3 个子节点
                let child = format!("node_{}", node_count);
                graph.add_node(child.clone());
                let _ = graph.add_dependency(child.clone(), parent.clone());
                next_level.push(child);
                node_count += 1;
                if node_count >= size {
                    break;
                }
            }
        }
        current_level = next_level;
    }

    graph
}

/// 创建 DAG（有向无环图）
fn create_dag(size: usize) -> DependencyGraph {
    let mut graph = DependencyGraph::new();

    for i in 0..size {
        let node = format!("node_{}", i);
        graph.add_node(node.clone());

        // 每个节点依赖于前面的 1-3 个节点
        let deps = std::cmp::min(i, 3);
        for j in 1..=deps {
            let _ = graph.add_dependency(node.clone(), format!("node_{}", i - j));
        }
    }

    graph
}

fn bench_topological_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("topological_sort");

    for size in [10, 50, 100, 200].iter() {
        let graph = create_linear_chain(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &graph, |b, graph| {
            b.iter(|| graph.topological_sort())
        });
    }
    group.finish();
}

fn bench_dependency_addition(c: &mut Criterion) {
    let mut group = c.benchmark_group("dependency_addition");

    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut graph = DependencyGraph::new();
                for i in 0..size {
                    let node = format!("node_{}", i);
                    graph.add_node(node.clone());
                    if i > 0 {
                        let _ = black_box(graph.add_dependency(node, format!("node_{}", i - 1)));
                    }
                }
            })
        });
    }
    group.finish();
}

fn bench_tree_propagation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_propagation");

    for size in [20, 50, 100].iter() {
        let graph = create_tree_dependency(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &graph, |b, graph| {
            b.iter(|| {
                // 模拟修改根节点
                black_box(graph.get_dependents("root"));
            })
        });
    }
    group.finish();
}

fn bench_dag_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_traversal");

    for size in [50, 100, 200].iter() {
        let graph = create_dag(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &graph, |b, graph| {
            b.iter(|| {
                // 遍历所有节点的依赖
                for node in graph.nodes() {
                    black_box(graph.get_dependencies(node));
                    black_box(graph.get_dependents(node));
                }
            })
        });
    }
    group.finish();
}

fn bench_clone_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("clone_optimization");

    // 测试减少克隆后的性能
    let graph = create_dag(100);

    group.bench_function("get_dependencies_no_clone", |b| {
        b.iter(|| {
            for i in 0..50 {
                let node = format!("node_{}", i);
                black_box(graph.get_dependencies(&node));
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_topological_sort,
    bench_dependency_addition,
    bench_tree_propagation,
    bench_dag_traversal,
    bench_clone_optimization,
);

criterion_main!(benches);
