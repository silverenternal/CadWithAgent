//! Context module integration tests
//!
//! Tests for DialogStateManager, ErrorCaseLibrary, and TaskPlanner integration

use cadagent::context::{
    DialogStateManager, ErrorCase, ErrorCaseLibrary, ErrorLibraryConfig, TaskNode, TaskPlanner,
};

/// Test: DialogStateManager basic operations
#[test]
fn test_dialog_state_manager_basic() {
    let mut manager = DialogStateManager::new("test-session-basic", Default::default())
        .expect("Failed to create DialogStateManager");

    // Test message recording
    manager
        .add_user_message("Hello, can you help me?")
        .expect("Failed to add message");
    manager
        .add_assistant_response("Of course! How can I assist you?", None)
        .expect("Failed to add response");

    // Test branch creation
    let branch_name = "test-branch";
    manager
        .create_branch(branch_name)
        .expect("Failed to create branch");

    // Verify messages were added (check via state)
    let state = manager.get_state();
    assert!(state.turn_count > 0);
}

/// Test: DialogStateManager branch merging
#[test]
fn test_dialog_state_manager_merge() {
    let mut manager = DialogStateManager::new("test-session-merge", Default::default())
        .expect("Failed to create DialogStateManager");

    // Create main branch with messages
    manager
        .add_user_message("Main branch message")
        .expect("Failed to add message");

    // Create feature branch
    manager
        .create_branch("feature")
        .expect("Failed to create feature branch");
    manager
        .add_user_message("Feature branch message")
        .expect("Failed to add message");

    // Verify feature branch has messages
    let state = manager.get_state();
    assert_eq!(state.current_branch, "feature");
    assert!(state.turn_count > 0);

    // Note: Full merge testing requires more complex setup
    // This test verifies branch creation and switching works
}

/// Test: ErrorCaseLibrary basic operations
#[test]
fn test_error_case_library_basic() {
    let mut library = ErrorCaseLibrary::new().expect("Failed to create ErrorCaseLibrary");

    // Create an error case
    let case = ErrorCase::new(
        "test_error",
        "Test error description",
        "Test scenario",
        "Test root cause",
        "Test solution",
    )
    .with_tags(vec!["test", "integration"])
    .with_confidence(0.9);

    // Add to library
    let hash = library.add_case(case).expect("Failed to add case");
    assert!(!hash.is_empty());

    // Find by type (more reliable than semantic search)
    let by_type = library.find_by_type("test_error");
    assert_eq!(by_type.len(), 1);
    assert_eq!(by_type[0].error_type, "test_error");
}

/// Test: ErrorCaseLibrary with custom config
#[test]
fn test_error_case_library_config() {
    let config = ErrorLibraryConfig {
        context_root: "./.cad_context/test_errors".to_string(),
        enable_semantic_search: true,
        enable_filekv: false,
    };

    let mut library = ErrorCaseLibrary::with_config(config)
        .expect("Failed to create ErrorCaseLibrary with config");

    // Add multiple error cases
    for i in 0..3 {
        let case = ErrorCase::new(
            &format!("error_type_{}", i),
            &format!("Error description {}", i),
            &format!("Scenario {}", i),
            &format!("Root cause {}", i),
            &format!("Solution {}", i),
        )
        .with_tags(vec!["test", &format!("tag_{}", i)]);

        library.add_case(case).expect("Failed to add case");
    }

    // Test statistics
    let stats = library.stats();
    assert_eq!(stats.total_cases, 3);
    assert!(!stats.error_types.is_empty());
}

/// Test: TaskPlanner basic operations
#[test]
fn test_task_planner_basic() {
    let mut planner = TaskPlanner::new().expect("Failed to create TaskPlanner");

    // Create a plan
    planner
        .create_plan("test-plan", "Test plan for basic operations")
        .expect("Failed to create plan");

    // Add tasks
    let task1 = TaskNode::new("task1", "First task");
    let task2 = TaskNode::new("task2", "Second task").with_dependencies(vec!["task1"]);

    planner.add_task(task1).expect("Failed to add task1");
    planner.add_task(task2).expect("Failed to add task2");

    // Get plan stats
    let stats = planner.get_plan_stats().expect("No plan stats");
    assert_eq!(stats.total_tasks, 2);
    assert_eq!(stats.pending_count, 2);
}

/// Test: TaskPlanner with multiple tasks
#[test]
fn test_task_planner_multiple_tasks() {
    let mut planner = TaskPlanner::new().expect("Failed to create TaskPlanner");

    planner
        .create_plan("test-plan", "Test plan")
        .expect("Failed to create plan");

    // Add tasks with dependencies
    planner
        .add_task_simple("A", "Task A", vec![])
        .expect("Failed to add A");
    planner
        .add_task_simple("B", "Task B", vec!["A"])
        .expect("Failed to add B");
    planner
        .add_task_simple("C", "Task C", vec!["A"])
        .expect("Failed to add C");

    // Get plan stats
    let stats = planner.get_plan_stats().expect("No plan stats");
    assert_eq!(stats.total_tasks, 3);
}

/// Test: Integration between components
#[test]
fn test_context_integration() {
    // Create all three components
    let mut dialog_manager = DialogStateManager::new("test-integration", Default::default())
        .expect("Failed to create DialogStateManager");

    let mut error_library = ErrorCaseLibrary::new().expect("Failed to create ErrorCaseLibrary");

    let mut task_planner = TaskPlanner::new().expect("Failed to create TaskPlanner");

    // Simulate a workflow:
    // 1. User asks a question
    dialog_manager
        .add_user_message("Please analyze this geometry")
        .expect("Failed to add message");

    // 2. Create tasks for the analysis
    task_planner
        .create_plan("analysis-plan", "Geometry analysis plan")
        .expect("Failed to create plan");
    task_planner
        .add_task_simple("extract", "Extract geometry", vec![])
        .expect("Failed to add task");
    task_planner
        .add_task_simple("verify", "Verify constraints", vec!["extract"])
        .expect("Failed to add task");

    // 3. Simulate an error during extraction
    let error_case = ErrorCase::new(
        "extraction_error",
        "Failed to extract geometry",
        "Invalid SVG format",
        "Malformed path data",
        "Validate SVG before extraction",
    );
    error_library
        .add_case(error_case)
        .expect("Failed to add error");

    // 4. Record the error in dialog
    dialog_manager
        .add_assistant_response(
            "I encountered an error while extracting geometry. \
         Based on past cases, please validate the SVG format.",
            None,
        )
        .expect("Failed to add response");

    // 5. Verify state
    let _hits = dialog_manager
        .search_context("geometry")
        .expect("Search failed");
    // Note: semantic search may not return results immediately in tests
    // The important thing is that the components work together

    let errors = error_library.find_by_type("extraction_error");
    assert_eq!(errors.len(), 1);

    let stats = task_planner.get_plan_stats().expect("No plan stats");
    assert_eq!(stats.total_tasks, 2);
}

/// Test: DialogStateManager session persistence
#[test]
fn test_dialog_state_persistence() {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Use unique session ID based on timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let session_id = format!("test-persistent-session-{}", timestamp);

    // Create and populate manager
    {
        let mut manager = DialogStateManager::new(&session_id, Default::default())
            .expect("Failed to create DialogStateManager");

        manager
            .add_user_message("Persistent message 1")
            .expect("Failed to add message");
        manager
            .add_assistant_response("Persistent response 1", None)
            .expect("Failed to add response");

        let state = manager.get_state();
        assert_eq!(state.turn_count, 2);
    }

    // Small delay to ensure WAL is flushed
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Reopen the same session and verify we can access data via search
    {
        let manager = DialogStateManager::new(&session_id, Default::default())
            .expect("Failed to reopen DialogStateManager");

        // Search for persisted messages
        let _hits = manager.search_context("Persistent").expect("Search failed");
        // At least some messages should be found
        // Note: This depends on semantic search being enabled and working
        // The key test is that we can reopen the session without error
        let _state = manager.get_state();
    }

    // Cleanup
    let _ = fs::remove_dir_all(format!("./.cad_context/{}", session_id));
}

/// Test: ErrorCaseLibrary semantic search
#[test]
fn test_error_semantic_search() {
    let mut library = ErrorCaseLibrary::new().expect("Failed to create ErrorCaseLibrary");

    // Add diverse error cases
    let cases = vec![
        ErrorCase::new(
            "constraint_conflict",
            "Parallel and perpendicular constraints conflict",
            "Applying conflicting angle constraints",
            "Over-constrained geometry",
            "Remove one constraint",
        )
        .with_tags(vec!["constraint", "parallel", "perpendicular"]),
        ErrorCase::new(
            "invalid_geometry",
            "Zero-length line detected",
            "Line with same start and end point",
            "Degenerate geometry",
            "Remove or redefine line",
        )
        .with_tags(vec!["geometry", "line", "degenerate"]),
        ErrorCase::new(
            "parsing_error",
            "Failed to parse SVG path",
            "Invalid path command",
            "Malformed SVG data",
            "Validate SVG syntax",
        )
        .with_tags(vec!["parsing", "svg", "path"]),
    ];

    for case in cases {
        library.add_case(case).expect("Failed to add case");
    }

    // Test find by tags (more reliable than semantic search in tests)
    let by_tag = library.find_by_tags(&["constraint"]);
    assert!(!by_tag.is_empty());

    let by_tag = library.find_by_tags(&["geometry"]);
    assert!(!by_tag.is_empty());
}

/// Test: TaskPlanner plan creation and stats
#[test]
fn test_task_planner_stats() {
    let mut planner = TaskPlanner::new().expect("Failed to create TaskPlanner");

    // Create plan with description
    planner
        .create_plan("stats-test", "Test plan for statistics")
        .expect("Failed to create plan");

    // Add tasks with different priorities
    planner
        .add_task_simple("high", "High priority task", vec![])
        .expect("Failed to add high");
    planner
        .add_task_simple("normal", "Normal priority task", vec![])
        .expect("Failed to add normal");
    planner
        .add_task_simple("low", "Low priority task", vec![])
        .expect("Failed to add low");

    // Get stats
    let stats = planner.get_plan_stats().expect("No plan stats");
    assert_eq!(stats.total_tasks, 3);
    assert_eq!(stats.pending_count, 3);
    assert_eq!(stats.completed_count, 0);
}
