//! ZazaZ Client AI Methods Integration Tests
//!
//! Tests for AI-powered features:
//! - AI-assisted merging
//! - Conflict resolution
//! - Branch purpose inference
//! - Branch summarization
//! - Merge risk assessment
//!
//! Note: These tests require ZAZAZ_API_KEY environment variable.
//! Run with: `cargo test zazaz -- --ignored`

use cadagent::bridge::zaza_client::{ZazaClient, ZazaConfig};

/// Helper to create a test client
/// Returns None if API key is not configured
fn create_test_client() -> Option<ZazaClient> {
    match ZazaClient::from_env() {
        Ok(client) => {
            if client.is_configured() {
                Some(client)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

/// Test: AI-assisted merge
#[tokio::test]
#[ignore] // Requires API key - run with `cargo test test_assist_merge -- --ignored`
async fn test_assist_merge() {
    let client = match create_test_client() {
        Some(c) => c,
        None => {
            println!("ZAZAZ_API_KEY not configured, skipping test");
            return;
        }
    };

    let result = client
        .assist_merge(
            "feature/new-constraint-solver",
            "main",
            "Conflicting changes in constraint.rs: \
             source branch modified Newton-Raphson solver, \
             target branch updated Levenberg-Marquardt parameters",
        )
        .await;

    assert!(result.is_ok(), "assist_merge failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.contains("conflict") || response.contains("resolution") || response.contains("merge"),
        "Response should mention conflict or resolution"
    );

    println!("\n=== AI-Assisted Merge Response ===");
    println!("{}", response);
}

/// Test: Conflict resolution
#[tokio::test]
#[ignore] // Requires API key - run with `cargo test test_resolve_conflict -- --ignored`
async fn test_resolve_conflict() {
    let client = match create_test_client() {
        Some(c) => c,
        None => {
            println!("ZAZAZ_API_KEY not configured, skipping test");
            return;
        }
    };

    let entities = vec![
        "Constraint::Distance".to_string(),
        "Constraint::Angle".to_string(),
        "Geometry::Line".to_string(),
    ];

    let constraints = vec![
        "Distance must be positive".to_string(),
        "Angle must be between 0 and 180 degrees".to_string(),
        "Line endpoints must be distinct".to_string(),
    ];

    let result = client
        .resolve_conflict(
            "Type mismatch in constraint application",
            &entities,
            &constraints,
        )
        .await;

    assert!(result.is_ok(), "resolve_conflict failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.contains("cause") || response.contains("solution") || response.contains("resolution"),
        "Response should mention cause or solution"
    );

    println!("\n=== Conflict Resolution Response ===");
    println!("{}", response);
}

/// Test: Branch purpose inference
#[tokio::test]
#[ignore] // Requires API key - run with `cargo test test_infer_branch_purpose -- --ignored`
async fn test_infer_branch_purpose() {
    let client = match create_test_client() {
        Some(c) => c,
        None => {
            println!("ZAZAZ_API_KEY not configured, skipping test");
            return;
        }
    };

    let recent_changes = vec![
        "Added Newton-Raphson solver for nonlinear constraints".to_string(),
        "Implemented Jacobian computation with sparse matrix optimization".to_string(),
        "Added tests for tangency constraints".to_string(),
    ];

    let dialog_summary = "User working on improving constraint solver performance \
                          for complex geometries with nonlinear constraints";

    let result = client
        .infer_branch_purpose("feature/nonlinear-solver", &recent_changes, dialog_summary)
        .await;

    assert!(result.is_ok(), "infer_branch_purpose failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.contains("constraint") || response.contains("solver") || response.contains("nonlinear"),
        "Response should mention constraint, solver, or nonlinear"
    );

    println!("\n=== Branch Purpose Inference ===");
    println!("{}", response);
}

/// Test: Branch summarization
#[tokio::test]
#[ignore] // Requires API key - run with `cargo test test_summarize_branch -- --ignored`
async fn test_summarize_branch() {
    let client = match create_test_client() {
        Some(c) => c,
        None => {
            println!("ZAZAZ_API_KEY not configured, skipping test");
            return;
        }
    };

    let content = r#"
    This branch implements major improvements to the constraint solver:
    
    1. Added Newton-Raphson method for nonlinear equation solving
    2. Implemented sparse Jacobian matrix computation
    3. Reduced memory usage by 40% through optimized data structures
    4. Added comprehensive test suite with 50+ new tests
    5. Updated documentation with performance benchmarks
    
    The changes improve solver performance by 3x for complex constraints.
    "#;

    let result = client
        .summarize_branch("feature/solver-optimization", content)
        .await;

    assert!(result.is_ok(), "summarize_branch failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.len() < 500,
        "Summary should be concise (less than 500 chars)"
    );

    println!("\n=== Branch Summary ===");
    println!("{}", response);
}

/// Test: Merge risk assessment
#[tokio::test]
#[ignore] // Requires API key - run with `cargo test test_assess_merge_risk -- --ignored`
async fn test_assess_merge_risk() {
    let client = match create_test_client() {
        Some(c) => c,
        None => {
            println!("ZAZAZ_API_KEY not configured, skipping test");
            return;
        }
    };

    let diff_summary = r#"
    Changes in feature/new-constraint-solver:
    - Modified: src/geometry/constraint.rs (added Newton-Raphson solver)
    - Modified: src/geometry/jacobian.rs (optimized sparse matrix)
    - Added: src/geometry/nonlinear.rs (new file)
    - Modified: tests/constraint_tests.rs (50 new tests)
    
    Changes in main:
    - Modified: src/geometry/constraint.rs (bug fixes in LM solver)
    - Modified: Cargo.toml (dependency updates)
    "#;

    let result = client
        .assess_merge_risk("feature/new-constraint-solver", "main", diff_summary)
        .await;

    assert!(result.is_ok(), "assess_merge_risk failed: {:?}", result);
    let response = result.unwrap();
    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.contains("Risk") || response.contains("conflict") || response.contains("merge"),
        "Response should mention risk, conflict, or merge"
    );

    println!("\n=== Merge Risk Assessment ===");
    println!("{}", response);
}

/// Test: ZazaClient configuration from environment
#[test]
fn test_zaza_config_from_env() {
    // This test verifies that configuration loading works correctly
    // It doesn't require an API key
    
    let config = ZazaConfig::default();
    
    // Verify default values
    assert_eq!(config.timeout_ms, 60000);
    assert_eq!(config.max_tokens, 2048);
    assert!((0.0..=1.0).contains(&config.temperature));
    
    // Verify endpoint defaults
    if std::env::var("PROVIDER_ZAZAZ_API_URL").is_err() {
        assert_eq!(config.endpoint, "https://zazaz.top/v1");
    }
    
    // Verify model default
    if std::env::var("PROVIDER_ZAZAZ_MODEL").is_err() {
        assert_eq!(config.model, "./Qwen3.5-27B-FP8");
    }
}

/// Test: Client creation without API key
#[test]
fn test_client_without_api_key() {
    // Create config without API key
    let config = ZazaConfig {
        api_key: None,
        ..Default::default()
    };
    
    let client = ZazaClient::with_config(config).expect("Failed to create client");
    assert!(!client.is_configured());
}

/// Test: Error handling for unconfigured client
#[tokio::test]
async fn test_unconfigured_client_error() {
    let config = ZazaConfig {
        api_key: None,
        ..Default::default()
    };
    
    let client = ZazaClient::with_config(config).expect("Failed to create client");
    
    // Should return ApiKeyNotConfigured error
    let result = client.generate("test prompt").await;
    assert!(matches!(result, Err(cadagent::bridge::zaza_client::ZazaError::ApiKeyNotConfigured)));
}
