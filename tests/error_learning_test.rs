//! Error Learning Integration Tests
//!
//! Tests for the complete error learning workflow including:
//! - Automatic error recording
//! - Root cause analysis
//! - Similar case recommendation
//! - Learning statistics

use cadagent::context::error_library::{
    ErrorLearningConfig, ErrorLearningManager, ErrorSource,
};

/// Test: Basic error learning workflow
#[test]
fn test_error_learning_workflow() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    // Record a numerical error
    let source = ErrorSource {
        tool_name: "constraint_solver".to_string(),
        operation: "solve_newton".to_string(),
        input_params: Some(r#"{"variables": 50, "iterations": 100}"#.to_string()),
        error_message: "Jacobian matrix is singular at iteration 5".to_string(),
        context: Some("Residual: 0.001, Variables: 50".to_string()),
    };

    let result = manager.record_error(source).expect("Failed to record error");

    // Verify learning result
    assert!(result.recorded, "Error should be recorded");
    assert!(result.error_id.is_some(), "Should have error ID");
    assert!(!result.recommendations.is_empty(), "Should have recommendations");

    // Verify statistics
    let stats = manager.stats();
    assert_eq!(stats.total_errors_received, 1);
    assert_eq!(stats.errors_recorded, 1);

    println!("✓ Error learning workflow test passed");
    println!("  Recorded error: {:?}", result.error_id);
    println!("  Recommendations: {}", result.recommendations.len());
}

/// Test: Duplicate error detection
#[test]
fn test_duplicate_detection() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    // Record first occurrence
    let source1 = ErrorSource {
        tool_name: "solver".to_string(),
        operation: "solve".to_string(),
        input_params: None,
        error_message: "Matrix is singular".to_string(),
        context: None,
    };
    let result1 = manager.record_error(source1).expect("Failed to record first error");
    assert!(result1.recorded);

    // Record duplicate
    let source2 = ErrorSource {
        tool_name: "solver".to_string(),
        operation: "solve".to_string(),
        input_params: None,
        error_message: "Matrix is singular".to_string(),
        context: None,
    };
    let result2 = manager.record_error(source2).expect("Failed to record duplicate");

    // Should be detected as duplicate
    assert!(!result2.recorded, "Duplicate should not be recorded as new case");
    assert!(result2.error_id.is_some(), "Should reference original error ID");

    // Verify occurrence count increased
    let error_id = result2.error_id.unwrap();
    let case = manager
        .get_errors_by_type("numerical_error")
        .into_iter()
        .find(|c| c.id == error_id)
        .expect("Error case not found");

    assert_eq!(case.occurrence_count, 2, "Occurrence count should be 2");

    println!("✓ Duplicate detection test passed");
    println!("  Original error ID: {:?}", result1.error_id);
    println!("  Occurrence count: {}", case.occurrence_count);
}

/// Test: Error classification
#[test]
fn test_error_classification() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        ..Default::default()
    };

    let manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    let test_cases = vec![
        ("Jacobian matrix is singular", "numerical_error"),
        ("Over-constrained system with conflicts", "constraint_conflict"),
        ("Invalid input parameter: radius must be positive", "invalid_input"),
        ("Operation timeout after 30s", "timeout_error"),
        ("Resource not found: file.xyz", "resource_not_found"),
        ("Permission denied: /etc/config", "permission_error"),
        ("Parse error at line 42", "parse_error"),
        ("Unknown error occurred", "general_error"),
    ];

    for (error_msg, expected_type) in test_cases {
        let source = ErrorSource {
            tool_name: "test".to_string(),
            operation: "test".to_string(),
            input_params: None,
            error_message: error_msg.to_string(),
            context: None,
        };

        let classified = manager.classify_error_type(&source);
        assert_eq!(
            classified, expected_type,
            "Expected {} for '{}', got {}",
            expected_type, error_msg, classified
        );
    }

    println!("✓ Error classification test passed");
    println!("  Tested {} error patterns", test_cases.len());
}

/// Test: Similar case recommendation
#[test]
fn test_similar_case_recommendation() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        max_similar_cases: 5,
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    // Record several constraint conflict errors
    let errors = vec![
        "Parallel and perpendicular constraints on same lines",
        "Concentric circles cannot be tangent",
        "Conflicting distance constraints",
        "Over-constrained sketch with redundant dimensions",
    ];

    for error_msg in errors {
        let source = ErrorSource {
            tool_name: "constraint_verifier".to_string(),
            operation: "detect_conflicts".to_string(),
            input_params: None,
            error_message: error_msg.to_string(),
            context: None,
        };
        let _ = manager.record_error(source).expect("Failed to record error");
    }

    // Search for similar cases
    let similar = manager
        .search_similar("constraint conflict parallel perpendicular")
        .expect("Failed to search similar");

    assert!(!similar.is_empty(), "Should find similar cases");
    assert!(
        similar.len() <= 5,
        "Should respect max_similar_cases limit"
    );

    // Verify similarity scores are ordered
    for i in 1..similar.len() {
        assert!(
            similar[i - 1].similarity >= similar[i].similarity,
            "Similar cases should be ordered by similarity"
        );
    }

    println!("✓ Similar case recommendation test passed");
    println!("  Found {} similar cases", similar.len());
    if let Some(first) = similar.first() {
        println!("  Most similar: {} (score: {:.2})", first.description, first.similarity);
    }
}

/// Test: Error severity calculation
#[test]
fn test_error_severity() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    // Record an error multiple times to increase severity
    for _ in 0..10 {
        let source = ErrorSource {
            tool_name: "solver".to_string(),
            operation: "solve".to_string(),
            input_params: None,
            error_message: "Frequent numerical error".to_string(),
            context: None,
        };
        let _ = manager.record_error(source).expect("Failed to record error");
    }

    // Get frequent errors and check severity
    let frequent = manager.get_frequent_errors(1);
    assert_eq!(frequent.len(), 1);

    let severity = frequent[0].severity();
    assert_eq!(
        severity,
        cadagent::context::ErrorSeverity::High,
        "Frequent errors should have high severity"
    );

    println!("✓ Error severity test passed");
    println!("  Frequent error severity: {:?}", severity);
}

/// Test: Batch error recording
#[test]
fn test_batch_error_recording() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    let sources = vec![
        ErrorSource {
            tool_name: "solver".to_string(),
            operation: "solve".to_string(),
            input_params: None,
            error_message: "Error 1".to_string(),
            context: None,
        },
        ErrorSource {
            tool_name: "verifier".to_string(),
            operation: "verify".to_string(),
            input_params: None,
            error_message: "Error 2".to_string(),
            context: None,
        },
        ErrorSource {
            tool_name: "parser".to_string(),
            operation: "parse".to_string(),
            input_params: None,
            error_message: "Error 3".to_string(),
            context: None,
        },
    ];

    let results = manager
        .record_errors_batch(sources)
        .expect("Failed to record batch");

    assert_eq!(results.len(), 3, "Should process all errors");
    assert!(results.iter().all(|r| r.recorded), "All errors should be recorded");

    let stats = manager.stats();
    assert_eq!(stats.total_errors_received, 3);
    assert_eq!(stats.errors_recorded, 3);

    println!("✓ Batch error recording test passed");
    println!("  Recorded {} errors in batch", results.len());
}

/// Test: Library statistics
#[test]
fn test_library_statistics() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    // Record various errors
    let error_types = vec![
        "numerical_error",
        "numerical_error",
        "constraint_conflict",
        "invalid_input",
        "numerical_error",
    ];

    for error_type in error_types {
        let source = ErrorSource {
            tool_name: "test".to_string(),
            operation: "test".to_string(),
            input_params: None,
            error_message: format!("{}: test message", error_type),
            context: None,
        };
        let _ = manager.record_error(source).expect("Failed to record error");
    }

    let lib_stats = manager.library_stats();

    assert_eq!(lib_stats.total_cases, 3, "Should have 3 unique error types");
    assert_eq!(lib_stats.total_occurrences, 5, "Should have 5 total occurrences");

    println!("✓ Library statistics test passed");
    println!("  Total cases: {}", lib_stats.total_cases);
    println!("  Total occurrences: {}", lib_stats.total_occurrences);
    println!("  Error types: {:?}", lib_stats.error_types);
}

/// Test: Confidence threshold
#[test]
fn test_confidence_threshold() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        min_confidence: 0.8, // High threshold
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    // Record error with low confidence (vague error message)
    let source = ErrorSource {
        tool_name: "test".to_string(),
        operation: "test".to_string(),
        input_params: None,
        error_message: "Unknown error occurred".to_string(),
        context: None, // No context reduces confidence
    };

    let result = manager.record_error(source).expect("Failed to record error");

    // Should be below threshold
    assert!(!result.recorded, "Low confidence error should not be recorded");
    assert!(
        result.recommendations
            .iter()
            .any(|r| r.contains("confidence")),
        "Should mention confidence threshold"
    );

    let stats = manager.stats();
    assert_eq!(stats.errors_below_threshold, 1);

    println!("✓ Confidence threshold test passed");
    println!("  Errors below threshold: {}", stats.errors_below_threshold);
}

/// Test: Root cause analysis (template-based)
#[test]
fn test_root_cause_analysis() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        llm_analysis: true,
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    let source = ErrorSource {
        tool_name: "constraint_solver".to_string(),
        operation: "solve_newton".to_string(),
        input_params: Some(r#"{"variables": 100}"#.to_string()),
        error_message: "Jacobian matrix is singular".to_string(),
        context: Some("Iteration 10, residual: 0.01".to_string()),
    };

    let result = manager.record_error(source).expect("Failed to record error");

    // Should have root cause analysis
    assert!(
        result.root_cause_analysis.is_some(),
        "Should have root cause analysis"
    );

    let analysis = result.root_cause_analysis.unwrap();
    assert!(
        analysis.contains("Root Cause Analysis"),
        "Should contain analysis header"
    );
    assert!(
        analysis.contains("numerical_error"),
        "Should identify error type"
    );
    assert!(
        analysis.contains("Recommended Investigation"),
        "Should have recommendations"
    );

    println!("✓ Root cause analysis test passed");
    println!("  Analysis length: {} chars", analysis.len());
}

/// Test: High severity errors retrieval
#[test]
fn test_high_severity_retrieval() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let config = ErrorLearningConfig {
        context_root: temp_dir.path().join("errors").to_str().unwrap().to_string(),
        ..Default::default()
    };

    let mut manager = ErrorLearningManager::with_config(config).expect("Failed to create manager");

    // Record errors with low confidence (will be high severity)
    for i in 0..3 {
        let source = ErrorSource {
            tool_name: "test".to_string(),
            operation: "test".to_string(),
            input_params: None,
            error_message: format!("Low confidence error {}", i),
            context: None,
        };
        let mut result = manager.record_error(source).expect("Failed to record error");
        
        // Manually lower confidence to trigger high severity
        if result.error_id.is_some() {
            // The error is recorded, now we need to modify it
            // This is a limitation - we can't directly modify through the manager
            // So we'll just record more occurrences instead
        }
    }

    // Record one error many times (will be high severity due to occurrence count)
    for _ in 0..10 {
        let source = ErrorSource {
            tool_name: "frequent_solver".to_string(),
            operation: "solve".to_string(),
            input_params: None,
            error_message: "Frequent error".to_string(),
            context: None,
        };
        let _ = manager.record_error(source).expect("Failed to record error");
    }

    let high_severity = manager.get_high_severity_errors();
    assert!(
        !high_severity.is_empty(),
        "Should have at least one high severity error"
    );

    println!("✓ High severity retrieval test passed");
    println!("  High severity errors: {}", high_severity.len());
}
