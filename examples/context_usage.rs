//! Context Module Comprehensive Usage Example
//!
//! This example demonstrates the full capabilities of the context module:
//! - DialogStateManager: Multi-turn conversation with branching
//! - ErrorCaseLibrary: Persistent error storage and semantic search
//! - TaskPlanner: DAG-based task planning with dependency management
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example context_usage
//! ```

use cadagent::context::{DialogStateManager, ErrorCase, ErrorCaseLibrary, TaskPlanner};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Context Module Comprehensive Usage Example ===\n");

    // Example 1: Dialog State Management
    example_dialog_management()?;

    // Example 2: Error Case Library
    example_error_library()?;

    // Example 3: Task Planning
    example_task_planning()?;

    // Example 4: Integrated Workflow
    example_integrated_workflow()?;

    Ok(())
}

/// Example 1: Dialog State Management
fn example_dialog_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 1: Dialog State Management ---\n");

    // Create a dialog state manager
    let mut dialog = DialogStateManager::new("example-session-1", Default::default())?;

    // Record a multi-turn conversation
    println!("📝 Recording conversation...");
    dialog.add_user_message("I want to analyze this floor plan")?;
    dialog.add_assistant_response(
        "I'll help you analyze the floor plan. Let me extract the geometry first.",
        Some(r#"{"tool": "extract_geometry", "params": {...}}"#),
    )?;
    dialog.add_user_message("Can you also count the rooms?")?;
    dialog.add_assistant_response(
        "Sure! I found 5 rooms: 3 bedrooms, 1 living room, and 1 kitchen.",
        None,
    )?;

    // Show dialog state
    let state = dialog.get_state();
    println!("\n📊 Dialog State:");
    println!("   - Session: {}", state.dialog_id);
    println!("   - Current Branch: {}", state.current_branch);
    println!("   - Turn Count: {}", state.turn_count);
    if let Some(ref task) = state.current_task {
        println!("   - Current Task: {}", task);
    }

    // Create a branch for alternative analysis
    println!("\n🌿 Creating alternative analysis branch...");
    dialog.create_branch("alternative-analysis")?;
    dialog.add_user_message("Let's try a different approach")?;

    let state = dialog.get_state();
    println!("   - Switched to branch: {}", state.current_branch);

    // Search context
    println!("\n🔍 Searching for 'geometry'...");
    let hits = dialog.search_context("geometry")?;
    println!("   - Found {} relevant messages", hits.len());

    println!("\n{}\n", "=".repeat(50));
    Ok(())
}

/// Example 2: Error Case Library
fn example_error_library() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 2: Error Case Library ---\n");

    // Create an error case library
    let mut library = ErrorCaseLibrary::new()?;

    // Add error cases from typical CAD analysis scenarios
    println!("📚 Adding error cases...");

    let errors = vec![
        ErrorCase::new(
            "constraint_conflict",
            "Parallel and perpendicular constraints on same lines",
            "User applied both parallel and perpendicular constraints",
            "Over-constrained geometry with conflicting requirements",
            "Remove one constraint based on design intent",
        )
        .with_tags(vec!["constraint", "conflict", "geometry"])
        .with_tools(vec!["ConstraintVerifier::detect_conflicts"]),
        ErrorCase::new(
            "invalid_geometry",
            "Zero-length line detected in floor plan",
            "Line with identical start and end coordinates",
            "Degenerate geometry causing numerical instability",
            "Remove the zero-length line or adjust endpoints",
        )
        .with_tags(vec!["geometry", "line", "degenerate"])
        .with_tools(vec!["ConstraintVerifier::check_geometry_validity"]),
        ErrorCase::new(
            "parsing_error",
            "Failed to parse SVG path data",
            "Invalid path command in SVG string",
            "Malformed SVG syntax or unsupported command",
            "Validate SVG syntax before parsing",
        )
        .with_tags(vec!["parsing", "svg", "syntax"])
        .with_tools(vec!["AnalysisPipeline::extract_from_svg"]),
        ErrorCase::new(
            "semantic_search_failure",
            "Semantic search returned no results",
            "Query too specific or index not built",
            "Insufficient training data or index corruption",
            "Rebuild semantic index or broaden search query",
        )
        .with_tags(vec!["search", "semantic", "index"])
        .with_tools(vec!["ErrorCaseLibrary::search_similar"]),
    ];

    for error in errors {
        let hash = library.add_case(error)?;
        println!("   - Added error case: {}", &hash[..8]);
    }

    // Search for similar errors
    println!("\n🔍 Searching for 'constraint conflict'...");
    let hits = library.search_similar("constraint conflict")?;
    println!("   - Found {} similar cases", hits.len());
    for (i, hit) in hits.iter().take(3).enumerate() {
        println!("      {}. Score: {:.2}", i + 1, hit.score);
    }

    // Find by type
    println!("\n📋 Finding errors by type 'invalid_geometry'...");
    let by_type = library.find_by_type("invalid_geometry");
    println!("   - Found {} cases", by_type.len());

    // Find by tags
    println!("\n🏷️  Finding errors with tag 'svg'...");
    let by_tag = library.find_by_tags(&["svg"]);
    println!("   - Found {} cases", by_tag.len());

    // Get statistics
    println!("\n📊 Error Library Statistics:");
    let stats = library.stats();
    println!("   - Total Cases: {}", stats.total_cases);
    println!("   - Total Occurrences: {}", stats.total_occurrences);
    println!("   - Error Types: {:?}", stats.error_types);

    println!("\n{}\n", "=".repeat(50));
    Ok(())
}

/// Example 3: Task Planning
fn example_task_planning() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 3: Task Planning ---\n");

    // Create a task planner
    let mut planner = TaskPlanner::new()?;

    // Create a plan for CAD analysis
    println!("📋 Creating CAD analysis plan...");
    planner.create_plan("cad-analysis", "Complete CAD floor plan analysis pipeline")?;

    // Add tasks with dependencies
    println!("➕ Adding tasks with dependencies...");

    planner.add_task_simple("load_svg", "Load and validate SVG file", vec![])?;

    planner.add_task_simple(
        "extract_primitives",
        "Extract geometric primitives from SVG",
        vec!["load_svg"],
    )?;

    planner.add_task_simple(
        "infer_constraints",
        "Infer geometric constraints",
        vec!["extract_primitives"],
    )?;

    planner.add_task_simple(
        "verify_constraints",
        "Verify constraint consistency",
        vec!["infer_constraints"],
    )?;

    planner.add_task_simple(
        "detect_rooms",
        "Detect room boundaries",
        vec!["verify_constraints"],
    )?;

    planner.add_task_simple(
        "calculate_areas",
        "Calculate room areas",
        vec!["detect_rooms"],
    )?;

    planner.add_task_simple(
        "generate_report",
        "Generate analysis report",
        vec!["calculate_areas", "verify_constraints"],
    )?;

    // Show plan statistics
    let stats = planner.get_plan_stats().unwrap();
    println!("\n📊 Plan Statistics:");
    println!("   - Total Tasks: {}", stats.total_tasks);
    println!("   - Pending: {}", stats.pending_count);
    println!("   - In Progress: {}", stats.in_progress_count);
    println!("   - Completed: {}", stats.completed_count);

    // Execute tasks in order
    println!("\n▶️  Executing tasks...");
    let mut executed = Vec::new();

    loop {
        // Get next ready task from current plan
        let task = planner
            .get_current_plan()
            .and_then(|plan| plan.get_next_ready_task())
            .cloned();

        match task {
            Some(task) => {
                println!("   → Executing: {} ({})", task.name, task.description);
                executed.push(task.name.clone());
                planner.complete_task(&task.id, Some("Task completed successfully"))?;
            }
            None => break,
        }
    }

    println!("\n✅ Execution Order:");
    for (i, task_id) in executed.iter().enumerate() {
        println!("   {}. {}", i + 1, task_id);
    }

    // Show final stats
    let final_stats = planner.get_plan_stats().unwrap();
    println!("\n📊 Final Statistics:");
    println!(
        "   - Completed: {}/{}",
        final_stats.completed_count, final_stats.total_tasks
    );

    println!("\n{}\n", "=".repeat(50));
    Ok(())
}

/// Example 4: Integrated Workflow
fn example_integrated_workflow() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Example 4: Integrated Workflow ---\n");

    // Create all three components
    let mut dialog = DialogStateManager::new("workflow-session", Default::default())?;
    let mut error_library = ErrorCaseLibrary::new()?;
    let mut planner = TaskPlanner::new()?;

    println!("🚀 Starting integrated CAD analysis workflow...\n");

    // Step 1: User request
    dialog.add_user_message("Please analyze this floor plan SVG")?;
    println!("1️⃣  User request recorded");

    // Step 2: Create analysis plan
    planner.create_plan("floor-plan-analysis", "Analyze floor plan and detect rooms")?;
    planner.add_task_simple("extract", "Extract geometry", vec![])?;
    planner.add_task_simple("verify", "Verify constraints", vec!["extract"])?;
    planner.add_task_simple("detect", "Detect rooms", vec!["verify"])?;
    println!("2️⃣  Analysis plan created with 3 tasks");

    // Step 3: Simulate error during extraction
    println!("3️⃣  Simulating error during extraction...");
    let error_case = ErrorCase::new(
        "extraction_error",
        "Failed to extract geometry from SVG",
        "Invalid path data in SVG string",
        "Malformed SVG path command",
        "Validate SVG syntax and retry extraction",
    )
    .with_tags(vec!["extraction", "svg", "parsing"]);
    error_library.add_case(error_case)?;

    // Step 4: Record error in dialog
    dialog.add_assistant_response(
        "I encountered an error while extracting geometry. \
         Based on past cases, this is likely due to invalid SVG path data. \
         Let me validate the syntax and retry.",
        None,
    )?;
    println!("4️⃣  Error recorded and response generated");

    // Step 5: Search for similar errors
    let hits = error_library.search_similar("SVG extraction error")?;
    println!("5️⃣  Found {} similar error cases", hits.len());

    // Step 6: Continue workflow
    dialog.add_user_message("Great, please proceed with the fix")?;
    dialog.add_assistant_response(
        "I've validated the SVG and successfully extracted the geometry. \
         Found 4 rooms with a total area of 120 square meters.",
        None,
    )?;
    println!("6️⃣  Workflow completed successfully");

    // Final state summary
    println!("\n📊 Workflow Summary:");
    let dialog_state = dialog.get_state();
    println!("   - Dialog Turns: {}", dialog_state.turn_count);

    let error_stats = error_library.stats();
    println!("   - Error Cases: {}", error_stats.total_cases);

    let plan_stats = planner.get_plan_stats().unwrap();
    println!("   - Planned Tasks: {}", plan_stats.total_tasks);

    println!("\n✨ Integrated workflow demonstration complete!\n");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_examples() {
        // Run all examples as tests
        example_dialog_management().unwrap();
        example_error_library().unwrap();
        example_task_planning().unwrap();
        example_integrated_workflow().unwrap();
    }
}
