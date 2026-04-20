//! Mock LLM Client for Testing
//!
//! This module provides a mock LLM client for testing AI features
//! without requiring a real LLM API connection.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Mock LLM client for testing
///
/// This client stores predefined responses and returns them
/// when matching prompts are received. Useful for testing AI
/// features without network access.
#[derive(Debug, Default)]
pub struct MockLLMClient {
    /// Predefined responses (prompt -> response)
    responses: Arc<Mutex<HashMap<String, String>>>,
    /// Call history for verification
    call_history: Arc<Mutex<Vec<String>>>,
    /// Default response when no match is found
    default_response: Arc<Mutex<String>>,
}

impl MockLLMClient {
    /// Create a new mock LLM client
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(HashMap::new())),
            call_history: Arc::new(Mutex::new(Vec::new())),
            default_response: Arc::new(Mutex::new("Mock response".to_string())),
        }
    }

    /// Add a predefined response
    pub async fn add_response(&self, prompt: &str, response: &str) {
        let mut responses = self.responses.lock().await;
        responses.insert(prompt.to_string(), response.to_string());
    }

    /// Set the default response for unmatched prompts
    pub async fn set_default_response(&self, response: &str) {
        let mut default = self.default_response.lock().await;
        *default = response.to_string();
    }

    /// Get the call history
    pub async fn get_call_history(&self) -> Vec<String> {
        let history = self.call_history.lock().await;
        history.clone()
    }

    /// Get the number of calls made
    pub async fn call_count(&self) -> usize {
        let history = self.call_history.lock().await;
        history.len()
    }

    /// Clear the call history
    pub async fn clear_history(&self) {
        let mut history = self.call_history.lock().await;
        history.clear();
    }

    /// Check if a specific prompt was called
    pub async fn was_called_with(&self, prompt: &str) -> bool {
        let history = self.call_history.lock().await;
        history.contains(&prompt.to_string())
    }
}

// Mock implementation that mimics the tokitai_context LLMClient trait
// Note: This is a simplified version for testing. The actual LLMClient trait
// from tokitai_context may have different method signatures.

impl MockLLMClient {
    /// Generate a response for the given prompt
    pub async fn generate(
        &self,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Record the call
        {
            let mut history = self.call_history.lock().await;
            history.push(prompt.to_string());
        }

        // Look for a matching response
        let responses = self.responses.lock().await;
        if let Some(response) = responses.get(prompt) {
            return Ok(response.clone());
        }

        // Return default response
        let default = self.default_response.lock().await;
        Ok(default.clone())
    }

    /// Generate a response with streaming
    pub async fn generate_stream(
        &self,
        prompt: &str,
    ) -> Result<MockStreamResponse, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.generate(prompt).await?;
        Ok(MockStreamResponse::new(response))
    }
}

/// Mock streaming response
pub struct MockStreamResponse {
    content: String,
    position: usize,
}

impl MockStreamResponse {
    pub fn new(content: String) -> Self {
        Self {
            content,
            position: 0,
        }
    }

    /// Get the next chunk of the response
    pub async fn next_chunk(&mut self) -> Option<String> {
        if self.position >= self.content.len() {
            return None;
        }

        // Return content in chunks of 10 characters
        let end = (self.position + 10).min(self.content.len());
        let chunk = self.content[self.position..end].to_string();
        self.position = end;
        Some(chunk)
    }

    /// Get the full content
    pub fn content(&self) -> &str {
        &self.content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_llm_client_creation() {
        let client = MockLLMClient::new();
        assert_eq!(client.call_count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_llm_client_add_response() {
        let client = MockLLMClient::new();
        client.add_response("test prompt", "test response").await;

        let response = client.generate("test prompt").await.unwrap();
        assert_eq!(response, "test response");
    }

    #[tokio::test]
    async fn test_mock_llm_client_default_response() {
        let client = MockLLMClient::new();
        client.set_default_response("default").await;

        let response = client.generate("unknown prompt").await.unwrap();
        assert_eq!(response, "default");
    }

    #[tokio::test]
    async fn test_mock_llm_client_call_history() {
        let client = MockLLMClient::new();

        client.generate("prompt 1").await.unwrap();
        client.generate("prompt 2").await.unwrap();
        client.generate("prompt 1").await.unwrap();

        assert_eq!(client.call_count().await, 3);
        assert!(client.was_called_with("prompt 1").await);
        assert!(client.was_called_with("prompt 2").await);
    }

    #[tokio::test]
    async fn test_mock_llm_client_clear_history() {
        let client = MockLLMClient::new();

        client.generate("prompt 1").await.unwrap();
        client.generate("prompt 2").await.unwrap();

        client.clear_history().await;

        assert_eq!(client.call_count().await, 0);
    }

    #[tokio::test]
    async fn test_mock_stream_response() {
        let mut stream = MockStreamResponse::new("Hello, World!".to_string());

        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next_chunk().await {
            chunks.push(chunk);
        }

        assert_eq!(chunks.join(""), "Hello, World!");
    }
}
