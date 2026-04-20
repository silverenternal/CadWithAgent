//! Tokioitai-Context Performance Benchmarks
//!
//! Measures key tokitai-context operations for autonomous decision-making:
//! - Branch creation time (target: <10ms, O(1))
//! - Branch checkout time (target: <10ms)
//! - Merge operation time (target: <100ms)
//! - Semantic search latency (target: <100ms)
//! - Crash recovery time (target: <10s)

use cadagent::context::{DialogStateConfig, DialogStateManager, TaskPlanner};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::tempdir;

/// Benchmark: Branch creation performance (P0-T2)
///
/// Measures O(1) branch creation time using tokitai-context's COW implementation.
fn bench_branch_creation(c: &mut Criterion) {
    let temp_dir = tempdir().unwrap();
    #[allow(unused_variables)]
    let config = DialogStateConfig {
        context_root: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut group = c.benchmark_group("branch_creation");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("create_branch_o1", |b| {
        b.iter(|| {
            // Create a fresh manager for each iteration to allow branch creation
            let temp_dir = tempdir().unwrap();
            let config = DialogStateConfig {
                context_root: temp_dir.path().to_str().unwrap().to_string(),
                ..Default::default()
            };
            let mut manager = DialogStateManager::new("bench-branch", config).unwrap();
            manager.add_user_message("Initial context").unwrap();
            let _ = black_box(manager.create_branch("bench-branch"));
            let _ = manager.checkout_branch("bench-branch");
        })
    });

    group.finish();
}

/// Benchmark: Branch checkout performance (P0-T2)
///
/// Measures branch switching time.
fn bench_branch_checkout(c: &mut Criterion) {
    let temp_dir = tempdir().unwrap();
    let config = DialogStateConfig {
        context_root: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = DialogStateManager::new("bench-checkout", config).unwrap();

    // Create multiple branches upfront
    for i in 0..5 {
        manager
            .create_branch(&format!("bench-branch-{}", i))
            .unwrap();
    }

    manager.checkout_branch("main").unwrap();

    let mut group = c.benchmark_group("branch_checkout");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("checkout_branch", |b| {
        b.iter(|| {
            // Alternate between branches to avoid caching effects
            static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let i = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 5;
            manager
                .checkout_branch(&format!("bench-branch-{}", i))
                .unwrap();
        })
    });

    group.finish();
}

/// Benchmark: Merge operation performance (P0-T3)
///
/// Measures branch merge time with different strategies.
fn bench_merge_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge_operation");
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(15));

    // Benchmark SelectiveMerge strategy
    group.bench_function("merge_selective", |b| {
        b.iter(|| {
            let temp_dir = tempdir().unwrap();
            let config = DialogStateConfig {
                context_root: temp_dir.path().to_str().unwrap().to_string(),
                ..Default::default()
            };

            let mut manager = DialogStateManager::new("bench-merge", config).unwrap();

            // Create branch with context
            manager
                .create_design_option("feature", "Feature branch")
                .unwrap();
            manager.add_user_message("Feature content").unwrap();
            manager
                .add_assistant_response("Feature response", None)
                .unwrap();

            // Return to main and merge
            manager.checkout_branch("main").unwrap();

            // Merge (may fail in benchmark environment, but measures attempt time)
            let _ = manager.merge_design_options("feature", false);
        })
    });

    // Benchmark AIAssisted strategy
    group.bench_function("merge_ai_assisted", |b| {
        b.iter(|| {
            let temp_dir = tempdir().unwrap();
            let config = DialogStateConfig {
                context_root: temp_dir.path().to_str().unwrap().to_string(),
                ..Default::default()
            };

            let mut manager = DialogStateManager::new("bench-merge-ai", config).unwrap();

            // Create branch with context
            manager
                .create_design_option("feature-ai", "Feature branch")
                .unwrap();
            manager
                .add_user_message("Feature content for AI merge")
                .unwrap();
            manager
                .add_assistant_response("Feature response", None)
                .unwrap();

            // Return to main and merge with AI
            manager.checkout_branch("main").unwrap();

            // AI-assisted merge
            let _ = manager.merge_design_options("feature-ai", true);
        })
    });

    group.finish();
}

/// Benchmark: Semantic search performance (P1-T4)
///
/// Measures cross-branch semantic search latency.
fn bench_semantic_search(c: &mut Criterion) {
    let temp_dir = tempdir().unwrap();
    let config = DialogStateConfig {
        context_root: temp_dir.path().to_str().unwrap().to_string(),
        enable_semantic_search: true,
        ..Default::default()
    };

    let mut manager = DialogStateManager::new("bench-search", config).unwrap();

    // Add diverse context across branches
    manager
        .add_user_message("CAD analysis using constraint solving")
        .unwrap();
    manager
        .add_assistant_response("Using geometric constraints for layout", None)
        .unwrap();

    // Create branches with different topics
    for topic in &["constraint", "parametric", "generative", "optimization"] {
        manager
            .create_design_option(&format!("topic-{}", topic), topic)
            .unwrap();
        manager
            .add_user_message(&format!("{} design approach details", topic))
            .unwrap();
        manager
            .add_assistant_response(&format!("{} response content", topic), None)
            .unwrap();
        manager.checkout_branch("main").unwrap();
    }

    let mut group = c.benchmark_group("semantic_search");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(10));

    // Single branch search
    group.bench_function("search_single_branch", |b| {
        b.iter(|| {
            let hits = manager.search_context(black_box("constraint")).unwrap();
            black_box(hits.len());
        })
    });

    // Cross-branch search
    group.bench_function("search_cross_branch", |b| {
        b.iter(|| {
            let hits = manager
                .cross_branch_search(black_box("design approach"))
                .unwrap();
            black_box(hits.len());
        })
    });

    // Error search
    group.bench_function("search_similar_errors", |b| {
        b.iter(|| {
            let hits = manager
                .search_similar_errors(black_box("constraint violation"), 5)
                .unwrap();
            black_box(hits.len());
        })
    });

    // Decision search
    group.bench_function("search_historical_decisions", |b| {
        b.iter(|| {
            let hits = manager
                .search_historical_decisions(black_box("design decision"), 10)
                .unwrap();
            black_box(hits.len());
        })
    });

    group.finish();
}

/// Benchmark: Layered storage performance (P0-T1)
///
/// Measures write performance across different storage layers.
fn bench_layered_storage(c: &mut Criterion) {
    let temp_dir = tempdir().unwrap();
    let config = DialogStateConfig {
        context_root: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = DialogStateManager::new("bench-layered", config).unwrap();

    let mut group = c.benchmark_group("layered_storage");
    group.sample_size(100);
    group.measurement_time(std::time::Duration::from_secs(10));

    // Transient layer (temporary thoughts)
    group.bench_function("store_transient", |b| {
        b.iter(|| {
            manager
                .add_temporary_thought(black_box("Temporary thought for benchmark"))
                .unwrap();
        })
    });

    // ShortTerm layer (recent conversation)
    group.bench_function("store_short_term", |b| {
        b.iter(|| {
            manager
                .add_user_message(black_box("User message for benchmark"))
                .unwrap();
        })
    });

    // LongTerm layer (permanent knowledge)
    group.bench_function("store_long_term", |b| {
        b.iter(|| {
            manager
                .store_long_term_knowledge(
                    black_box("benchmark_knowledge"),
                    black_box("Knowledge content for long-term storage"),
                )
                .unwrap();
        })
    });

    group.finish();
}

/// Benchmark: Task checkpoint performance (P0-T4)
///
/// Measures checkpoint creation and rollback time.
fn bench_checkpoint(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoint");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(10));

    group.bench_function("create_checkpoint", |b| {
        b.iter(|| {
            let temp_dir = tempdir().unwrap();
            let config = DialogStateConfig {
                context_root: temp_dir.path().to_str().unwrap().to_string(),
                ..Default::default()
            };

            let mut dialog_manager = DialogStateManager::new("bench-checkpoint", config).unwrap();
            let mut task_planner = TaskPlanner::new().unwrap();

            dialog_manager.add_user_message("Task context").unwrap();

            task_planner
                .create_plan("bench-plan", "Benchmark plan")
                .unwrap();
            task_planner
                .add_task_simple("task1", "Task 1", vec![])
                .unwrap();
            task_planner
                .add_task_simple("task2", "Task 2", vec!["task1"])
                .unwrap();

            let checkpoint = task_planner
                .create_checkpoint(black_box("bench-checkpoint"))
                .unwrap();
            black_box(checkpoint);
        })
    });

    group.finish();
}

/// Benchmark: Design option creation (P0-T2)
///
/// Measures end-to-end design option branching performance.
fn bench_design_option_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("design_option");
    group.sample_size(30);
    group.measurement_time(std::time::Duration::from_secs(15));

    group.bench_function("create_design_option_with_metadata", |b| {
        b.iter(|| {
            let temp_dir = tempdir().unwrap();
            let config = DialogStateConfig {
                context_root: temp_dir.path().to_str().unwrap().to_string(),
                ..Default::default()
            };

            let mut manager = DialogStateManager::new("bench-design", config).unwrap();

            // Simulate design exploration workflow
            manager.add_user_message("Explore design options").unwrap();

            let metadata = manager
                .create_design_option(
                    black_box("bench-option"),
                    black_box("Benchmark design option with detailed description"),
                )
                .unwrap();

            manager.add_user_message("Design details").unwrap();
            manager
                .add_assistant_response("Design response", None)
                .unwrap();

            black_box(metadata);
        })
    });

    group.finish();
}

/// Benchmark: Context statistics (monitoring)
fn bench_context_stats(c: &mut Criterion) {
    let temp_dir = tempdir().unwrap();
    let config = DialogStateConfig {
        context_root: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = DialogStateManager::new("bench-stats", config).unwrap();

    // Add varying amounts of context
    for i in 0..20 {
        manager.add_user_message(&format!("Message {}", i)).unwrap();
        manager
            .add_assistant_response(&format!("Response {}", i), None)
            .unwrap();
    }

    let mut group = c.benchmark_group("context_stats");
    group.sample_size(100);
    group.measurement_time(std::time::Duration::from_secs(5));

    group.bench_function("get_context_stats", |b| {
        b.iter(|| {
            let stats = manager.stats();
            black_box(stats);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_branch_creation,
    bench_branch_checkout,
    bench_merge_operation,
    bench_semantic_search,
    bench_layered_storage,
    bench_checkpoint,
    bench_design_option_creation,
    bench_context_stats,
);

criterion_main!(benches);
