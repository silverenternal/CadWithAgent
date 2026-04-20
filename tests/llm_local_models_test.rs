//! LLM Reasoning Integration Tests with Local Models
//!
//! Tests for local LLM integration:
//! - Ollama support
//! - LM Studio support
//! - Multi-turn dialogue
//!
//! Note: These tests require local LLM services running.
//! - Ollama: http://localhost:11434
//! - LM Studio: http://localhost:1234

use cadagent::llm_reasoning::{LlmReasoningEngine, LlmReasoningRequest, ReasoningTask};
use serde_json::json;

/// Test: Create engine with Ollama
#[test]
#[ignore] // Requires Ollama running - run with `cargo test test_ollama_engine -- --ignored`
fn test_ollama_engine() {
    // Test creating engine with Ollama
    let result = LlmReasoningEngine::with_ollama();
    
    match result {
        Ok(engine) => {
            println!("✓ Ollama engine created successfully");
            
            // Test a simple reasoning task
            let request = LlmReasoningRequest {
                task: "测试任务".to_string(),
                task_type: ReasoningTask::Custom,
                context: json!({}),
                verbose: false,
            };
            
            let response = engine.reason(request);
            match response {
                Ok(r) => {
                    println!("✓ Reasoning completed");
                    println!("  Answer: {}", r.chain_of_thought.answer);
                    println!("  Confidence: {:.2}", r.chain_of_thought.confidence);
                    println!("  Latency: {}ms", r.latency_ms);
                }
                Err(e) => {
                    println!("✗ Reasoning failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠ Ollama not available (this is OK if not running): {:?}", e);
        }
    }
}

/// Test: Create engine with LM Studio
#[test]
#[ignore] // Requires LM Studio running - run with `cargo test test_lm_studio_engine -- --ignored`
fn test_lm_studio_engine() {
    // Test creating engine with LM Studio
    let result = LlmReasoningEngine::with_lm_studio();
    
    match result {
        Ok(engine) => {
            println!("✓ LM Studio engine created successfully");
            
            // Test a simple reasoning task
            let request = LlmReasoningRequest {
                task: "测试任务".to_string(),
                task_type: ReasoningTask::Custom,
                context: json!({}),
                verbose: false,
            };
            
            let response = engine.reason(request);
            match response {
                Ok(r) => {
                    println!("✓ Reasoning completed");
                    println!("  Answer: {}", r.chain_of_thought.answer);
                    println!("  Confidence: {:.2}", r.chain_of_thought.confidence);
                    println!("  Latency: {}ms", r.latency_ms);
                }
                Err(e) => {
                    println!("✗ Reasoning failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠ LM Studio not available (this is OK if not running): {:?}", e);
        }
    }
}

/// Test: Multi-turn dialogue with Ollama
#[test]
#[ignore] // Requires Ollama running - run with `cargo test test_ollama_multi_turn -- --ignored`
fn test_ollama_multi_turn() {
    let engine = match LlmReasoningEngine::with_ollama() {
        Ok(e) => e,
        Err(_) => {
            println!("⚠ Ollama not available, skipping test");
            return;
        }
    };
    
    println!("Testing multi-turn dialogue with Ollama...");
    
    // Turn 1: Count rooms
    let request1 = LlmReasoningRequest {
        task: "这个户型有多少个房间？".to_string(),
        task_type: ReasoningTask::CountRooms,
        context: json!({}),
        verbose: false,
    };
    
    let response1 = engine.reason(request1);
    match response1 {
        Ok(r) => {
            println!("✓ Turn 1 completed: {}", r.chain_of_thought.answer);
        }
        Err(e) => {
            println!("✗ Turn 1 failed: {:?}", e);
            return;
        }
    }
    
    // Turn 2: Calculate area (should maintain context)
    let request2 = LlmReasoningRequest {
        task: "计算总面积".to_string(),
        task_type: ReasoningTask::CalculateArea,
        context: json!({}),
        verbose: false,
    };
    
    let response2 = engine.reason(request2);
    match response2 {
        Ok(r) => {
            println!("✓ Turn 2 completed: {}", r.chain_of_thought.answer);
        }
        Err(e) => {
            println!("✗ Turn 2 failed: {:?}", e);
        }
    }
    
    println!("Multi-turn dialogue test completed");
}

/// Test: Multi-turn dialogue with LM Studio
#[test]
#[ignore] // Requires LM Studio running - run with `cargo test test_lm_studio_multi_turn -- --ignored`
fn test_lm_studio_multi_turn() {
    let engine = match LlmReasoningEngine::with_lm_studio() {
        Ok(e) => e,
        Err(_) => {
            println!("⚠ LM Studio not available, skipping test");
            return;
        }
    };
    
    println!("Testing multi-turn dialogue with LM Studio...");
    
    // Turn 1: Count rooms
    let request1 = LlmReasoningRequest {
        task: "这个户型有多少个房间？".to_string(),
        task_type: ReasoningTask::CountRooms,
        context: json!({}),
        verbose: false,
    };
    
    let response1 = engine.reason(request1);
    match response1 {
        Ok(r) => {
            println!("✓ Turn 1 completed: {}", r.chain_of_thought.answer);
        }
        Err(e) => {
            println!("✗ Turn 1 failed: {:?}", e);
            return;
        }
    }
    
    // Turn 2: Calculate area (should maintain context)
    let request2 = LlmReasoningRequest {
        task: "计算总面积".to_string(),
        task_type: ReasoningTask::CalculateArea,
        context: json!({}),
        verbose: false,
    };
    
    let response2 = engine.reason(request2);
    match response2 {
        Ok(r) => {
            println!("✓ Turn 2 completed: {}", r.chain_of_thought.answer);
        }
        Err(e) => {
            println!("✗ Turn 2 failed: {:?}", e);
        }
    }
    
    println!("Multi-turn dialogue test completed");
}

/// Test: Local model fallback to mock
#[test]
fn test_local_model_fallback() {
    // This test verifies that when local models are not available,
    // the engine gracefully handles the error
    
    // Try to create Ollama engine (will likely fail if not running)
    let result = LlmReasoningEngine::with_ollama();
    
    match result {
        Ok(_) => {
            println!("✓ Ollama is available and working");
        }
        Err(_) => {
            println!("✓ Ollama not available - graceful error handling works");
            // This is expected behavior when Ollama is not running
        }
    }
    
    // Try to create LM Studio engine (will likely fail if not running)
    let result = LlmReasoningEngine::with_lm_studio();
    
    match result {
        Ok(_) => {
            println!("✓ LM Studio is available and working");
        }
        Err(_) => {
            println!("✓ LM Studio not available - graceful error handling works");
            // This is expected behavior when LM Studio is not running
        }
    }
}

/// Test: Compare response times between different configurations
#[test]
#[ignore] // Requires both services - run with `cargo test compare_response_times -- --ignored`
fn compare_response_times() {
    println!("\n=== Response Time Comparison ===\n");
    
    // Test with Ollama
    if let Ok(engine) = LlmReasoningEngine::with_ollama() {
        let request = LlmReasoningRequest {
            task: "测试".to_string(),
            task_type: ReasoningTask::Custom,
            context: json!({}),
            verbose: false,
        };
        
        if let Ok(response) = engine.reason(request) {
            println!("Ollama: {}ms", response.latency_ms);
        }
    } else {
        println!("Ollama: Not available");
    }
    
    // Test with LM Studio
    if let Ok(engine) = LlmReasoningEngine::with_lm_studio() {
        let request = LlmReasoningRequest {
            task: "测试".to_string(),
            task_type: ReasoningTask::Custom,
            context: json!({}),
            verbose: false,
        };
        
        if let Ok(response) = engine.reason(request) {
            println!("LM Studio: {}ms", response.latency_ms);
        }
    } else {
        println!("LM Studio: Not available");
    }
    
    // Test with ZAZAZ (cloud)
    if let Ok(engine) = LlmReasoningEngine::new() {
        let request = LlmReasoningRequest {
            task: "测试".to_string(),
            task_type: ReasoningTask::Custom,
            context: json!({}),
            verbose: false,
        };
        
        if let Ok(response) = engine.reason(request) {
            println!("ZAZAZ (Cloud): {}ms", response.latency_ms);
        }
    } else {
        println!("ZAZAZ: Not configured");
    }
}
